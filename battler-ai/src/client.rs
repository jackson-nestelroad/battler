use std::{
    sync::Arc,
    usize,
};

use ahash::HashSet;
use anyhow::{
    Error,
    Result,
};
use battler::{
    DataStoreByName,
    Request,
};
use battler_choice::choices_from_string;
use battler_client::{
    BattleClientEvent,
    BattlerClient,
};

use crate::{
    AiContext,
    BattlerAi,
    choice::{
        ChoiceFailure,
        MakeChoiceFailure,
    },
};

/// A wrapper around a [`BattlerClient`] that uses a [`BattlerAi`] implementation to make decisions.
pub struct BattlerAiClient<'data, 'battle> {
    data: &'data dyn DataStoreByName,
    client: Arc<BattlerClient<'battle>>,
    ai: Box<dyn BattlerAi>,
}

impl<'data, 'battle> BattlerAiClient<'data, 'battle> {
    /// Creates a new [`BattlerAiClient`].
    pub fn new(
        data: &'data dyn DataStoreByName,
        client: Arc<BattlerClient<'battle>>,
        ai: Box<dyn BattlerAi>,
    ) -> Self {
        Self { data, client, ai }
    }

    /// Runs the client.
    pub async fn run(mut self) -> Result<()> {
        self.handle_battle_events(usize::MAX).await
    }

    /// Runs the client for a given number of requests.
    pub async fn run_for_requests(mut self, requests: usize) -> Result<()> {
        self.handle_battle_events(requests).await
    }

    async fn handle_battle_events(&mut self, mut requests: usize) -> Result<()> {
        let mut battle_event_rx = self.client.battle_event_rx();
        loop {
            if requests == 0 {
                return Ok(());
            }
            tokio::select! {
                changed = battle_event_rx.changed() => {
                    changed?;
                    if self.handle_battle_event(&battle_event_rx.borrow_and_update(), &mut requests).await? {
                        return Ok(());
                    }
                }
            }
        }
    }

    async fn handle_battle_event(
        &mut self,
        event: &BattleClientEvent,
        requests: &mut usize,
    ) -> Result<bool> {
        match event {
            BattleClientEvent::Request(request) => match request {
                Some(request) => {
                    self.make_choice(request).await?;
                    *requests -= 1;
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
                        choice: choice.clone(),
                        reason: err.to_string(),
                    });
                    // Parse choice failure if possible.
                    if let Ok(choices) = choices_from_string(choice)
                        && let Ok(choice_failure) = ChoiceFailure::new(err, &choices)
                    {
                        ai_context.choice_failures.insert(choice_failure);
                    }
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
