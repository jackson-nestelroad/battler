use std::sync::Arc;

use ahash::HashSet;
use anyhow::{
    Error,
    Result,
};
use battler::{
    DataStoreByName,
    Request,
};
use battler_client::{
    BattleClientEvent,
    BattlerClient,
};

use crate::{
    AiContext,
    BattlerAi,
    choice::MakeChoiceFailure,
};

struct BattlerAiClient<'data, 'battle> {
    data: &'data dyn DataStoreByName,
    client: Arc<BattlerClient<'battle>>,
    ai: Box<dyn BattlerAi>,
}

impl<'data, 'battle> BattlerAiClient<'data, 'battle> {
    fn new(
        data: &'data dyn DataStoreByName,
        client: Arc<BattlerClient<'battle>>,
        ai: Box<dyn BattlerAi>,
    ) -> Self {
        Self { data, client, ai }
    }

    async fn run(mut self) -> Result<()> {
        self.handle_battle_events().await
    }

    async fn handle_battle_events(&mut self) -> Result<()> {
        let mut battle_event_rx = self.client.battle_event_rx();
        loop {
            tokio::select! {
                changed = battle_event_rx.changed() => {
                    changed?;
                    if self.handle_battle_event(&battle_event_rx.borrow_and_update()).await? {
                        return Ok(());
                    }
                }
            }
        }
    }

    async fn handle_battle_event(&mut self, event: &BattleClientEvent) -> Result<bool> {
        match event {
            BattleClientEvent::Request(request) => match request {
                Some(request) => {
                    self.make_choice(request).await?;
                    Ok(false)
                }
                None => Ok(false),
            },
            BattleClientEvent::End => Ok(true),
            BattleClientEvent::Error(err) => {
                return Err(Error::msg(format!("battle client failed: {err}")));
            }
        }
    }

    async fn make_choice(&mut self, request: &Request) -> Result<()> {
        const MAX_ATTEMPTS: u64 = 5;
        let mut ai_context = self.ai_context().await?;
        for _ in 0..MAX_ATTEMPTS {
            let choice = self.ai.make_choice(&ai_context, request).await?;
            match self.client.make_choice(&choice).await {
                Ok(()) => return Ok(()),
                Err(err) => {
                    ai_context.make_choice_failures.push(MakeChoiceFailure {
                        choice,
                        reason: err.to_string(),
                    });
                    // TODO: Need to parse for choice_failures.
                    //
                    // Let's put choice input serialization and deserialization into the types
                    // crate, so it can be used by everyone.
                }
            }
        }

        // If we continually fail to make a move, just forfeit the battle.
        self.client.make_choice("forfeit").await
    }

    async fn ai_context(&self) -> Result<AiContext<'data>> {
        let player_data = self.client.player_data().await?;
        let state = self.client.state().await;
        Ok(AiContext {
            data: self.data,
            state,
            player_data,
            choice_failures: HashSet::default(),
            make_choice_failures: Vec::default(),
        })
    }
}

/// Runs a battler AI implementation for the given client.
pub async fn run_battler_ai_client<'data, 'battle>(
    data: &'data dyn DataStoreByName,
    client: Arc<BattlerClient<'battle>>,
    ai: Box<dyn BattlerAi>,
) -> Result<()> {
    let client = BattlerAiClient::new(data, client, ai);
    client.run().await
}
