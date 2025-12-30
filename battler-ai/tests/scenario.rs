use std::{
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
use battler::{
    CoreBattleEngineOptions,
    CoreBattleOptions,
    DataStoreByName,
};
use battler_ai::{
    AiContext,
    BattlerAi,
    run_battler_ai_client,
};
use battler_client::{
    BattleClientEvent,
    BattlerClient,
};
use battler_service::{
    Battle,
    BattleServiceOptions,
    BattlerService,
};
use battler_service_client::battler_service_client_over_direct_service;
use battler_test_utils::static_local_data_store;
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
        })
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
        BattlerClient::new(
            self.battle.uuid,
            player.into(),
            Arc::new(battler_service_client_over_direct_service(
                self.service.clone(),
            )),
        )
        .await
    }
}

impl Scenario<'static> {
    pub async fn run_ai<S, A>(&self, player: S, ai: A) -> Result<JoinHandle<Result<()>>>
    where
        S: Into<String>,
        A: BattlerAi + 'static,
    {
        Ok(tokio::spawn(run_battler_ai_client(
            self.data_store,
            self.client(player).await?,
            Box::new(ai),
        )))
    }
}
