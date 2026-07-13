use std::{
    collections::HashMap,
    env,
    fs::File,
    io::BufReader,
    sync::Arc,
};

use ahash::HashSet;
use anyhow::{
    Context,
    Error,
    Result,
};
use async_trait::async_trait;
use battler::{
    CoreBattleEngineOptions,
    CoreBattleOptions,
    DataStoreByName,
};
use battler_ai::{
    AiContext,
    BattlerAi,
};
use battler_client::{
    BattleClientEvent,
    BattlerClient,
};
use battler_service::{
    Battle,
    BattlePreview,
    BattleServiceOptions,
    BattlerService,
    PlayerValidation,
};
use battler_service_client::BattlerServiceClient;
use battler_test_utils::static_local_data_store;
use futures_util::lock::Mutex;
use serde::{
    Deserialize,
    Serialize,
};
use tokio::task::JoinHandle;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScenarioInputData {
    options: CoreBattleOptions,
    engine_options: CoreBattleEngineOptions,
    service_options: BattleServiceOptions,
    choices: Vec<(String, String)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScenarioExpectedResultData {
    player: String,
    choice: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScenarioData {
    input: ScenarioInputData,
    expected: ScenarioExpectedResultData,
}

pub struct Scenario<'d> {
    data_store: &'d dyn DataStoreByName,
    service: Arc<BattlerService<'d>>,
    battle: Battle,
    expected_result: ScenarioExpectedResultData,
    options: Option<CoreBattleOptions>,
    choices: Arc<futures_util::lock::Mutex<Vec<(String, String)>>>,
    explanations: Arc<
        futures_util::lock::Mutex<
            HashMap<String, Arc<futures_util::lock::Mutex<Vec<(String, String)>>>>,
        >,
    >,
    error_on_exceeded_attempts: bool,
}

impl Scenario<'static> {
    pub async fn from_file(file: &str) -> Result<Self> {
        let file = File::open(file)?;
        Self::new(
            serde_json::from_reader(BufReader::new(file))?,
            static_local_data_store(),
        )
        .await
    }

    pub async fn from_scenarios_dir(path: &str) -> Result<Self> {
        Self::from_file(&format!(
            "{}/battler-ai/tests/scenarios/{path}",
            env::var("CRATE_ROOT")?
        ))
        .await
    }
}

impl<'d> Scenario<'d> {
    pub async fn new(data: ScenarioData, data_store: &'d dyn DataStoreByName) -> Result<Self> {
        let options = data.input.options.clone();
        let service = Arc::new(BattlerService::new(data_store));
        let battle = service
            .create(
                data.input.options,
                data.input.engine_options,
                data.input.service_options,
            )
            .await?;
        service.start(battle.uuid).await?;
        for (player, choice) in data.input.choices {
            service.make_choice(battle.uuid, &player, &choice).await?;
        }
        Ok(Self {
            data_store,
            service,
            battle,
            expected_result: data.expected,
            options: Some(options),
            choices: Arc::new(futures_util::lock::Mutex::new(Vec::new())),
            explanations: Arc::new(futures_util::lock::Mutex::new(HashMap::new())),
            error_on_exceeded_attempts: false,
        })
    }

    pub async fn from_options(
        options: CoreBattleOptions,
        data_store: &'d dyn DataStoreByName,
    ) -> Result<Self> {
        let options_clone = options.clone();
        let service = Arc::new(BattlerService::new(data_store));
        let battle = service
            .create(
                options,
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
            )
            .await?;
        service.start(battle.uuid).await?;
        Ok(Self {
            data_store,
            service,
            battle,
            expected_result: ScenarioExpectedResultData {
                player: "player-1".to_string(),
                choice: None,
            },
            options: Some(options_clone),
            choices: Arc::new(futures_util::lock::Mutex::new(Vec::new())),
            explanations: Arc::new(futures_util::lock::Mutex::new(HashMap::new())),
            error_on_exceeded_attempts: false,
        })
    }

    /// Configures whether spawned AI clients should return an error when maximum choice attempts
    /// are exceeded.
    pub fn with_error_on_exceeded_attempts(mut self, value: bool) -> Self {
        self.error_on_exceeded_attempts = value;
        self
    }

    async fn ai_context<'a, S>(
        &'a self,
        player: S,
        client: &BattlerClient<'a>,
    ) -> Result<AiContext<'a>>
    where
        S: AsRef<str>,
    {
        let player_data = self
            .service
            .player_data(self.battle.uuid, player.as_ref())
            .await?;
        let state = client.state().await;
        Ok(AiContext {
            data: self.data_store,
            battle: client.battle(),
            state,
            player_data,
            choice_failures: HashSet::default(),
            make_choice_failures: Vec::default(),
        })
    }

    pub async fn validate_expected_result<A>(&self, ai: &mut A) -> Result<()>
    where
        A: BattlerAi,
    {
        let player = &self.expected_result.player;
        let client = self.client(player).await?;

        // Wait for the client to present a request to us.
        let mut battle_event_rx = client.battle_event_rx();
        let request = battle_event_rx
            .wait_for(|event| match event {
                BattleClientEvent::Request(Some(_)) => true,
                _ => false,
            })
            .await?;
        let request = match &*request {
            BattleClientEvent::Request(Some(request)) => request,
            _ => {
                return Err(Error::msg(
                    "request event unexpectedly did not match after waiting",
                ));
            }
        };

        let ai_context = self.ai_context(player, &client).await?;

        let choice = ai.make_choice(&ai_context, request).await?;
        self.service
            .make_choice(self.battle.uuid, player, &choice)
            .await
            .with_context(|| choice.clone())?;
        if let Some(expected) = &self.expected_result.choice
            && choice != *expected
        {
            return Err(Error::msg(format!(
                "ai generated unexpected choice: expected {expected}, got {choice}"
            )));
        }
        Ok(())
    }

    pub async fn client<S>(&self, player: S) -> Result<BattlerClient<'d>>
    where
        S: Into<String>,
    {
        let player_name = player.into();
        let direct_client =
            battler_service_client::DirectBattlerServiceClient::new(self.service.clone());
        let recording_client = ChoiceRecordingServiceClient {
            inner: direct_client,
            choices: self.choices.clone(),
        };
        let boxed_client: Box<dyn BattlerServiceClient + 'd> = Box::new(recording_client);
        BattlerClient::new(self.battle.uuid, player_name, Arc::new(boxed_client)).await
    }

    /// Registers a shared explanations vector for a player so it can be captured in the final debug
    /// log.
    pub async fn record_explanations(
        &self,
        player: &str,
        explanations: Arc<futures_util::lock::Mutex<Vec<(String, String)>>>,
    ) {
        self.explanations
            .lock()
            .await
            .insert(player.to_string(), explanations);
    }
}

impl Scenario<'static> {
    pub async fn run_ai<S, A>(&self, player: S, ai: A) -> Result<JoinHandle<Result<()>>>
    where
        S: Into<String>,
        A: BattlerAi + 'static,
    {
        let data_store = self.data_store;
        let client = self.client(player).await?;
        let error_on_exceeded_attempts = self.error_on_exceeded_attempts;
        Ok(tokio::spawn(async move {
            let client = battler_ai::BattlerAiClient::new(data_store, client, Box::new(ai))
                .with_error_on_exceeded_attempts(error_on_exceeded_attempts);
            client.run().await
        }))
    }

    pub async fn run_ai_for_requests<S, A>(
        &self,
        player: S,
        ai: A,
        requests: usize,
    ) -> Result<JoinHandle<Result<()>>>
    where
        S: Into<String>,
        A: BattlerAi + 'static,
    {
        let data_store = self.data_store;
        let client = self.client(player).await?;
        let error_on_exceeded_attempts = self.error_on_exceeded_attempts;
        Ok(tokio::spawn(async move {
            let client = battler_ai::BattlerAiClient::new(data_store, client, Box::new(ai))
                .with_error_on_exceeded_attempts(error_on_exceeded_attempts);
            client.run_for_requests(requests).await
        }))
    }
}

struct ChoiceRecordingServiceClient<'a> {
    inner: battler_service_client::DirectBattlerServiceClient<'a>,
    choices: Arc<Mutex<Vec<(String, String)>>>,
}

#[async_trait]
impl<'a> BattlerServiceClient for ChoiceRecordingServiceClient<'a> {
    async fn battle(&self, battle: uuid::Uuid) -> Result<Battle> {
        self.inner.battle(battle).await
    }
    async fn create(
        &self,
        options: CoreBattleOptions,
        service_options: BattleServiceOptions,
    ) -> Result<Battle> {
        self.inner.create(options, service_options).await
    }
    async fn update_team(
        &self,
        battle: uuid::Uuid,
        player: &str,
        team: battler::TeamData,
    ) -> Result<()> {
        self.inner.update_team(battle, player, team).await
    }
    async fn validate_player(&self, battle: uuid::Uuid, player: &str) -> Result<PlayerValidation> {
        self.inner.validate_player(battle, player).await
    }
    async fn start(&self, battle: uuid::Uuid) -> Result<()> {
        self.inner.start(battle).await
    }
    async fn player_data(
        &self,
        battle: uuid::Uuid,
        player: &str,
    ) -> Result<battler::PlayerBattleData> {
        self.inner.player_data(battle, player).await
    }
    async fn request(&self, battle: uuid::Uuid, player: &str) -> Result<Option<battler::Request>> {
        self.inner.request(battle, player).await
    }
    async fn make_choice(&self, battle: uuid::Uuid, player: &str, choice: &str) -> Result<()> {
        self.choices
            .lock()
            .await
            .push((player.to_string(), choice.to_string()));
        self.inner.make_choice(battle, player, choice).await
    }
    async fn full_log(&self, battle: uuid::Uuid, side: Option<usize>) -> Result<Vec<String>> {
        self.inner.full_log(battle, side).await
    }
    async fn last_log_entry(
        &self,
        battle: uuid::Uuid,
        side: Option<usize>,
    ) -> Result<Option<(usize, String)>> {
        self.inner.last_log_entry(battle, side).await
    }
    async fn subscribe(
        &self,
        battle: uuid::Uuid,
        side: Option<usize>,
    ) -> Result<tokio::sync::broadcast::Receiver<battler_service::LogEntry>> {
        self.inner.subscribe(battle, side).await
    }
    async fn delete(&self, battle: uuid::Uuid) -> Result<()> {
        self.inner.delete(battle).await
    }
    async fn battles(&self, count: usize, offset: usize) -> Result<Vec<BattlePreview>> {
        self.inner.battles(count, offset).await
    }
    async fn battles_for_player(
        &self,
        player: &str,
        count: usize,
        offset: usize,
    ) -> Result<Vec<BattlePreview>> {
        self.inner.battles_for_player(player, count, offset).await
    }
}

impl<'d> Drop for Scenario<'d> {
    fn drop(&mut self) {
        let service = self.service.clone();
        let battle_uuid = self.battle.uuid;
        let options = self.options.clone();
        let choices = self.choices.clone();
        let explanations_map = self.explanations.clone();

        let test_name = std::thread::current()
            .name()
            .unwrap_or("unknown_test")
            .to_string();

        futures::executor::block_on(async move {
            let choices = choices.lock().await.clone();
            let explanations = explanations_map.lock().await.clone();
            let logs = service
                .full_log(battle_uuid, None)
                .await
                .unwrap_or_default();

            let mut resolved_explanations = HashMap::new();
            for (player, explanations) in explanations {
                resolved_explanations.insert(player, explanations.lock().await.clone());
            }

            let debug_data = serde_json::json!({
                "test_name": test_name,
                "seed": options.as_ref().and_then(|o| o.seed),
                "options": options,
                "choices": choices,
                "explanations": resolved_explanations,
                "logs": logs,
            });

            let manifest_dir =
                std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
            let debug_dir = std::path::Path::new(&manifest_dir)
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join(".battler-debug-test-runs");

            if let Err(e) = std::fs::create_dir_all(&debug_dir) {
                log::error!("Failed to create .battler-debug-test-runs directory: {e}");
                return;
            }

            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let seed_str = options
                .as_ref()
                .and_then(|o| o.seed)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "none".to_string());
            let slug = test_name.replace("::", "_");
            let file_name = format!("{slug}_{seed_str}_{timestamp}.json");
            let file_path = debug_dir.join(file_name);

            if let Ok(file) = std::fs::File::create(&file_path) {
                if let Err(err) = serde_json::to_writer_pretty(file, &debug_data) {
                    log::error!("Failed to write debug JSON: {err}");
                } else {
                    log::info!("Wrote debug JSON to {}", file_path.display());
                }
            }
        });
    }
}
