use ahash::HashMap;
use anyhow::Result;
use tokio::{
    sync::broadcast,
    task::JoinHandle,
};

use crate::{
    AiPlayerOptions,
    ai::ai_player::{
        AiPlayer,
        AiPlayerHandle,
        AiPlayerModules,
    },
};

#[derive(Debug)]
struct AiPlayerRegistryTaskHandle<'d> {
    join_handle: JoinHandle<()>,
    ai_player_handle: AiPlayerHandle<'d>,
}

impl<'d> AiPlayerRegistryTaskHandle<'d> {
    fn abort(&self) {
        self.ai_player_handle.cancel().ok();
        self.join_handle.abort();
    }
}

impl Drop for AiPlayerRegistryTaskHandle<'_> {
    fn drop(&mut self) {
        self.abort();
    }
}

/// A collection of [`AiPlayer`]s.
#[derive(Debug, Default)]
pub struct AiPlayerRegistry<'d> {
    players: HashMap<String, AiPlayerRegistryTaskHandle<'d>>,
}

impl<'d> AiPlayerRegistry<'d> {
    /// Creates a new AI player.
    pub async fn create_ai_player(
        &mut self,
        id: String,
        options: AiPlayerOptions,
        modules: AiPlayerModules<'d>,
    ) -> Result<()> {
        let handle = AiPlayer::new(options, modules).start().await?;
        let error_rx = handle.error_rx();
        let join_handle = tokio::spawn(AiPlayerRegistry::watch_ai_player(id.clone(), error_rx));
        let handle = AiPlayerRegistryTaskHandle {
            join_handle,
            ai_player_handle: handle,
        };
        self.players.insert(id, handle);
        Ok(())
    }

    async fn watch_ai_player(id: String, mut error_rx: broadcast::Receiver<String>) {
        loop {
            tokio::select! {
                err = error_rx.recv() => {
                    let err = match err {
                        Ok(err) => err,
                        Err(_) => break,
                    };
                    log::error!("AI player {id} error: {err}");
                }
            }
        }
    }
}
