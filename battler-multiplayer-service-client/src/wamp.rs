use std::sync::Arc;

use anyhow::{
    Context,
    Error,
    Result,
};
use async_trait::async_trait;
use battler_multiplayer_service::{
    BattlerMultiplayerServiceClient,
    ProposedBattle,
    ProposedBattleOptions,
    ProposedBattleResponse,
    ProposedBattleUpdate,
};
use battler_wamp_values::Integer;
use battler_wamprat::{
    peer::CallOptions,
    subscription::TypedPatternMatchedSubscription,
};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Implementation of [`BattlerMultiplayerServiceClient`] that uses the
/// [`battler_multiplayer_service_schema::BattlerMultiplayerServiceConsumer`] for managing proposed
/// battles remotely via a WAMP router.
pub struct WampBattlerMultiplayerServiceClient<S> {
    consumer: Arc<battler_multiplayer_service_schema::BattlerMultiplayerServiceConsumer<S>>,
}

impl<S> WampBattlerMultiplayerServiceClient<S> {
    /// Creates a new client around a WAMP service consumer.
    pub fn new(
        consumer: Arc<battler_multiplayer_service_schema::BattlerMultiplayerServiceConsumer<S>>,
    ) -> Self {
        Self { consumer }
    }
}

fn uuid_for_uri(uuid: &Uuid) -> String {
    uuid.simple()
        .encode_lower(&mut Uuid::encode_buffer())
        .to_owned()
}

fn export_proposed_battle(
    proposed_battle: battler_multiplayer_service_schema::ProposedBattle,
) -> Result<ProposedBattle> {
    serde_json::from_str(&proposed_battle.proposed_battle_json)
        .context(Error::msg("invalid proposed battle"))
}

fn export_proposed_battle_update(
    proposed_battle_update: battler_multiplayer_service_schema::ProposedBattleUpdate,
) -> Result<ProposedBattleUpdate> {
    serde_json::from_str(&proposed_battle_update.proposed_battle_update_json)
        .context(Error::msg("invalid proposed battle"))
}

#[async_trait]
impl<S> BattlerMultiplayerServiceClient for WampBattlerMultiplayerServiceClient<S>
where
    S: Send + 'static,
{
    async fn propose_battle(&self, options: ProposedBattleOptions) -> Result<ProposedBattle> {
        let output = self
            .consumer
            .propose_battle(
                battler_multiplayer_service_schema::ProposeBattleInput(
                    battler_multiplayer_service_schema::ProposeBattleInputArgs {
                        proposed_battle_options_json: serde_json::to_string(&options)?,
                    },
                ),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        export_proposed_battle(output.0)
    }

    async fn proposed_battles_for_player(
        &self,
        player: &str,
        count: usize,
        offset: usize,
    ) -> Result<Vec<ProposedBattle>> {
        let output = self
            .consumer
            .proposed_battles_for_player(
                battler_multiplayer_service_schema::ProposedBattlesForPlayerInput(
                    battler_multiplayer_service_schema::ProposedBattlesForPlayerInputArgs {
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
            .proposed_battles
            .into_iter()
            .map(|proposed_battle| export_proposed_battle(proposed_battle))
            .collect()
    }

    async fn respond_to_proposed_battle(
        &self,
        proposed_battle: Uuid,
        player: &str,
        response: ProposedBattleResponse,
    ) -> Result<()> {
        self.consumer
            .respond_to_proposed_battle(
                battler_multiplayer_service_schema::RespondToProposedBattlePattern(uuid_for_uri(
                    &proposed_battle,
                )),
                battler_multiplayer_service_schema::RespondToProposedBattleInput(
                    battler_multiplayer_service_schema::RespondToProposedBattleInputArgs {
                        player: player.to_owned(),
                        proposed_battle_response_json: serde_json::to_string(&response)?,
                    },
                ),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        todo!()
    }

    async fn proposed_battle_updates(
        &self,
        player: &str,
    ) -> Result<broadcast::Receiver<ProposedBattleUpdate>> {
        let (update_tx, update_rx) = broadcast::channel(16);
        let pattern = battler_multiplayer_service_schema::ProposedBattleUpdatesPattern {
            player: player.to_owned(),
        };

        struct Subscription<S> {
            update_tx: broadcast::Sender<ProposedBattleUpdate>,
            consumer: Arc<battler_multiplayer_service_schema::BattlerMultiplayerServiceConsumer<S>>,
            pattern: battler_multiplayer_service_schema::ProposedBattleUpdatesPattern,
        }

        impl<S> battler_multiplayer_service_schema::ProposedBattleUpdatesSubscription for Subscription<S> where
            S: Send + 'static
        {
        }
        impl<S> TypedPatternMatchedSubscription for Subscription<S>
        where
            S: Send + 'static,
        {
            type Pattern = battler_multiplayer_service_schema::ProposedBattleUpdatesPattern;
            type Event = battler_multiplayer_service_schema::ProposedBattleUpdateEvent;

            async fn handle_event(&self, event: Self::Event, _: Self::Pattern) {
                if self.update_tx.receiver_count() == 0 {
                    self.consumer
                        .unsubscribe_proposed_battle_updates(&self.pattern)
                        .await
                        .ok();
                    return;
                }
                let update = match export_proposed_battle_update(event.0) {
                    Ok(update) => update,
                    Err(_) => return,
                };
                self.update_tx.send(update).ok();
            }
        }

        self.consumer
            .subscribe_proposed_battle_updates(
                &pattern,
                Subscription {
                    update_tx,
                    consumer: self.consumer.clone(),
                    pattern: pattern.clone(),
                },
            )
            .await?;
        Ok(update_rx)
    }
}
