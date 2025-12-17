use std::sync::Arc;

use anyhow::{
    Context,
    Error,
    Result,
};
use async_trait::async_trait;
use battler::{
    CoreBattleOptions,
    PlayerBattleData,
    Request,
    TeamData,
};
use battler_service::{
    Battle,
    BattlePreview,
    BattleServiceOptions,
    LogEntry,
    PlayerValidation,
};
use battler_wamp_values::Integer;
use battler_wamprat::{
    peer::CallOptions,
    subscription::TypedPatternMatchedSubscription,
};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::BattlerServiceClient;

/// Implementation of [`BattlerServiceClient`] that uses the
/// [`battler_service_schema::BattlerServiceConsumer`] for managing battles remotely via a WAMP
/// router.
pub struct SimpleWampBattlerServiceClient<S> {
    consumer: Arc<battler_service_schema::BattlerServiceConsumer<S>>,
}

impl<S> SimpleWampBattlerServiceClient<S> {
    /// Creates a new client around a WAMP service consumer.
    pub fn new(consumer: Arc<battler_service_schema::BattlerServiceConsumer<S>>) -> Self {
        Self { consumer }
    }
}

fn uuid_for_uri(uuid: &Uuid) -> String {
    uuid.simple()
        .encode_lower(&mut Uuid::encode_buffer())
        .to_owned()
}

fn export_battle(battle: battler_service_schema::Battle) -> Result<Battle> {
    serde_json::from_str(&battle.battle_json).context(Error::msg("invalid battle"))
}

fn export_battle_preview(battle: battler_service_schema::BattlePreview) -> Result<BattlePreview> {
    serde_json::from_str(&battle.battle_json).context(Error::msg("invalid battle preview"))
}

#[async_trait]
impl<S> BattlerServiceClient for SimpleWampBattlerServiceClient<S>
where
    S: Send + 'static,
{
    async fn battle(&self, battle: Uuid) -> Result<Battle> {
        let output = self
            .consumer
            .battle(
                battler_service_schema::BattlePattern(uuid_for_uri(&battle)),
                battler_service_schema::BattleInput,
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        export_battle(output.0)
    }

    async fn create(
        &self,
        options: CoreBattleOptions,
        service_options: BattleServiceOptions,
    ) -> Result<Battle> {
        let battle = self
            .consumer
            .create(
                battler_service_schema::CreateInput(battler_service_schema::CreateInputArgs {
                    options_json: serde_json::to_string(&options)
                        .context(Error::msg("failed to serialize battle options"))?,
                    service_options_json: serde_json::to_string(&service_options)
                        .context(Error::msg("failed to serialize battle service options"))?,
                }),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        export_battle(battle.0)
    }

    async fn update_team(&self, battle: Uuid, player: &str, team: TeamData) -> Result<()> {
        self.consumer
            .update_team(
                battler_service_schema::UpdateTeamPattern(uuid_for_uri(&battle)),
                battler_service_schema::UpdateTeamInput(
                    battler_service_schema::UpdateTeamInputArgs {
                        player: player.to_owned(),
                        team_data_json: serde_json::to_string(&team)
                            .context(Error::msg("failed to serialize team data"))?,
                    },
                ),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(())
    }

    async fn validate_player(&self, battle: Uuid, player: &str) -> Result<PlayerValidation> {
        let output = self
            .consumer
            .validate_player(
                battler_service_schema::ValidatePlayerPattern(uuid_for_uri(&battle)),
                battler_service_schema::ValidatePlayerInput(
                    battler_service_schema::ValidatePlayerInputArgs {
                        player: player.to_owned(),
                    },
                ),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(PlayerValidation {
            problems: output.0.problems,
        })
    }

    async fn start(&self, battle: Uuid) -> Result<()> {
        self.consumer
            .start(
                battler_service_schema::StartPattern(uuid_for_uri(&battle)),
                battler_service_schema::StartInput,
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(())
    }

    async fn player_data(&self, battle: Uuid, player: &str) -> Result<PlayerBattleData> {
        let output = self
            .consumer
            .player_data(
                battler_service_schema::PlayerDataPattern(uuid_for_uri(&battle)),
                battler_service_schema::PlayerDataInput(
                    battler_service_schema::PlayerDataInputArgs {
                        player: player.to_owned(),
                    },
                ),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        serde_json::from_str(&output.0.player_data_json).context(Error::msg("invalid player data"))
    }

    async fn request(&self, battle: Uuid, player: &str) -> Result<Option<Request>> {
        let output = self
            .consumer
            .request(
                battler_service_schema::RequestPattern(uuid_for_uri(&battle)),
                battler_service_schema::RequestInput(battler_service_schema::RequestInputArgs {
                    player: player.to_owned(),
                }),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        match output.0.request_json {
            Some(request_json) => {
                serde_json::from_str(&request_json).context(Error::msg("invalid request"))
            }
            None => Ok(None),
        }
    }

    async fn make_choice(&self, battle: Uuid, player: &str, choice: &str) -> Result<()> {
        self.consumer
            .make_choice(
                battler_service_schema::MakeChoicePattern(uuid_for_uri(&battle)),
                battler_service_schema::MakeChoiceInput(
                    battler_service_schema::MakeChoiceInputArgs {
                        player: player.to_owned(),
                        choice: choice.to_owned(),
                    },
                ),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(())
    }

    async fn full_log(&self, battle: Uuid, side: Option<usize>) -> Result<Vec<String>> {
        let output = self
            .consumer
            .full_log(
                battler_service_schema::FullLogPattern(uuid_for_uri(&battle)),
                battler_service_schema::FullLogInput(battler_service_schema::FullLogInputArgs {
                    side: side.map(|side| side as Integer),
                }),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(output.0.log)
    }

    async fn last_log_entry(
        &self,
        battle: Uuid,
        side: Option<usize>,
    ) -> Result<Option<(usize, String)>> {
        let output = self
            .consumer
            .last_log_entry(
                battler_service_schema::LastLogEntryPattern(uuid_for_uri(&battle)),
                battler_service_schema::LastLogEntryInput(
                    battler_service_schema::LastLogEntryInputArgs {
                        side: side.map(|side| side as Integer),
                    },
                ),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(output
            .0
            .map(|log_entry| (log_entry.index as usize, log_entry.content)))
    }

    async fn subscribe(
        &self,
        battle: Uuid,
        side: Option<usize>,
    ) -> Result<broadcast::Receiver<LogEntry>> {
        let (entry_tx, entry_rx) = broadcast::channel(128);
        let pattern = battler_service_schema::LogPattern(
            uuid_for_uri(&battle),
            side.map(|side| battler_service_schema::LogSelector::Side(side))
                .unwrap_or_default(),
        );

        struct Subscription<S> {
            entry_tx: broadcast::Sender<LogEntry>,
            consumer: Arc<battler_service_schema::BattlerServiceConsumer<S>>,
            pattern: battler_service_schema::LogPattern,
        }

        impl<S> battler_service_schema::LogSubscription for Subscription<S> where S: Send + 'static {}
        impl<S> TypedPatternMatchedSubscription for Subscription<S>
        where
            S: Send + 'static,
        {
            type Pattern = battler_service_schema::LogPattern;
            type Event = battler_service_schema::LogEvent;

            async fn handle_event(&self, event: Self::Event, _: Self::Pattern) {
                if self.entry_tx.receiver_count() == 0 {
                    self.consumer.unsubscribe_log(&self.pattern).await.ok();
                    return;
                }
                self.entry_tx
                    .send(LogEntry {
                        index: event.0.index as usize,
                        content: event.0.content,
                    })
                    .ok();
            }
        }

        self.consumer
            .subscribe_log(
                &pattern,
                Subscription {
                    entry_tx,
                    consumer: self.consumer.clone(),
                    pattern: pattern.clone(),
                },
            )
            .await?;
        Ok(entry_rx)
    }

    async fn delete(&self, battle: Uuid) -> Result<()> {
        self.consumer
            .delete(
                battler_service_schema::DeletePattern(uuid_for_uri(&battle)),
                battler_service_schema::DeleteInput,
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(())
    }

    async fn battles(&self, count: usize, offset: usize) -> Result<Vec<BattlePreview>> {
        let output = self
            .consumer
            .battles(
                battler_service_schema::BattlesInput(battler_service_schema::BattlesInputArgs {
                    count: count as Integer,
                    offset: offset as Integer,
                }),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        output
            .0
            .battles
            .into_iter()
            .map(|battle| export_battle_preview(battle))
            .collect()
    }

    async fn battles_for_player(
        &self,
        player: &str,
        count: usize,
        offset: usize,
    ) -> Result<Vec<BattlePreview>> {
        let output = self
            .consumer
            .battles_for_player(
                battler_service_schema::BattlesForPlayerInput(
                    battler_service_schema::BattlesForPlayerInputArgs {
                        player: player.to_owned(),
                        count: count as Integer,
                        offset: offset as Integer,
                    },
                ),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        output
            .0
            .battles
            .into_iter()
            .map(|battle| export_battle_preview(battle))
            .collect()
    }
}
