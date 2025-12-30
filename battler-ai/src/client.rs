use std::usize;

use ahash::HashSet;
use anyhow::Result;
use battler::{
    DataStoreByName,
    Request,
};
use battler_choice::choices_from_string;
use battler_client::{
    BattleEndedError,
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
    client: BattlerClient<'battle>,
    ai: Box<dyn BattlerAi>,
}

impl<'data, 'battle> BattlerAiClient<'data, 'battle> {
    /// Creates a new [`BattlerAiClient`].
    pub fn new(
        data: &'data dyn DataStoreByName,
        client: BattlerClient<'battle>,
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
            match BattlerClient::wait_for_request(&mut battle_event_rx).await {
                Ok(request) => {
                    if let Err(err) = self.make_choice(&request).await {
                        log::error!(
                            "AI client {} in battle {} failed: {err:?}",
                            self.client.player(),
                            self.client.battle()
                        );
                        return Err(err);
                    }
                    requests -= 1;
                }
                Err(err) => {
                    return err
                        .downcast::<BattleEndedError>()
                        .map(|_| ())
                        .map_err(|err| err.context("battle client failed"));
                }
            }
        }
    }

    async fn make_choice(&mut self, request: &Request) -> Result<()> {
        const MAX_ATTEMPTS: u64 = 5;
        let mut ai_context = self.ai_context().await?;
        for _ in 0..MAX_ATTEMPTS {
            let choice = self.ai.make_choice(&ai_context, request).await?;
            match self.client.make_choice(&choice).await {
                Ok(()) => {
                    log::info!(
                        "AI client {} in battle {} succeeded at making choice: {choice}",
                        self.client.player(),
                        self.client.battle()
                    );
                    return Ok(());
                }
                Err(err) => {
                    log::error!(
                        "AI client {} in battle {} ({:?}) made a bad choice: {choice}: {err:?}",
                        self.client.player(),
                        self.client.battle(),
                        self.ai
                    );
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
        log::warn!(
            "AI client {} in battle {} ({:?}) exceeded {MAX_ATTEMPTS} attempts for request {request:?}, forfeiting",
            self.client.player(),
            self.client.battle(),
            self.ai
        );
        self.client.make_choice("forfeit").await
    }

    async fn ai_context(&self) -> Result<AiContext<'data>> {
        let player_data = self.client.player_data().await?;
        let state = self.client.state().await;
        Ok(AiContext {
            data: self.data,
            battle: self.client.battle(),
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
    client: BattlerClient<'battle>,
    ai: Box<dyn BattlerAi>,
) -> Result<()> {
    let client = BattlerAiClient::new(data, client, ai);
    client.run().await
}
