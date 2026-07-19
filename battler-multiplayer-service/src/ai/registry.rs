use ahash::{
    HashMap,
    HashSet,
};
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
        log::trace!(
            "Dropping AI player registry task handle {}",
            self.ai_player_handle.id()
        );
        self.abort();
    }
}

/// A collection of [`AiPlayer`]s.
#[derive(Debug, Default)]
pub struct AiPlayerRegistry<'d> {
    players: HashMap<String, AiPlayerRegistryTaskHandle<'d>>,
    player_ids: HashSet<String>,
}

impl<'d> AiPlayerRegistry<'d> {
    /// Creates a new AI player.
    pub async fn create_ai_player(
        &mut self,
        id: String,
        options: AiPlayerOptions,
        modules: AiPlayerModules<'d>,
    ) -> Result<()> {
        for player in &options.players {
            self.player_ids.insert(player.clone());
        }
        let handle = AiPlayer::new(id.clone(), options, modules).start().await?;
        let error_rx = handle.error_rx();
        let join_handle = tokio::spawn(AiPlayerRegistry::watch_ai_player(id.clone(), error_rx));
        let handle = AiPlayerRegistryTaskHandle {
            join_handle,
            ai_player_handle: handle,
        };
        self.players.insert(id, handle);
        Ok(())
    }

    /// Checks if a player ID is a registered AI player.
    pub fn is_ai_player(&self, player_id: &str) -> bool {
        self.player_ids.contains(player_id)
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
