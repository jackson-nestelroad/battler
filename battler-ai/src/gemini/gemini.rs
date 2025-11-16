use anyhow::{
    Error,
    Result,
};
use async_trait::async_trait;
use battler::{
    PlayerBattleData,
    Request,
};
use battler_client::state::BattleState;
use serde::{
    Deserialize,
    Serialize,
};
use tokio::process::Command;

use crate::{
    AiContext,
    BattlerAi,
    choice::MakeChoiceFailure,
};

#[derive(Serialize)]
struct Input<'a> {
    player_data: &'a PlayerBattleData,
    battle_state: &'a BattleState,
    request_data: &'a Request,
    failed_actions: &'a Vec<MakeChoiceFailure>,
}

#[derive(Deserialize)]
struct Output {
    actions: String,
    #[allow(unused)]
    explanation: String,
}

pub struct Gemini {}

#[async_trait]
impl BattlerAi for Gemini {
    async fn make_choice<'a>(
        &mut self,
        context: &AiContext<'a>,
        request: &Request,
    ) -> Result<String> {
        // After so many attempts, just give up.
        if context.make_choice_failures.len() > 5 {
            return Ok("forfeit".to_owned());
        }

        let executable = env!("GEMINI_PYTHON_EXECUTABLE");
        let input = Input {
            player_data: &context.player_data,
            battle_state: &context.state,
            request_data: &request,
            failed_actions: &context.make_choice_failures,
        };
        let mut cmd = Command::new(executable);
        cmd.arg("--use_cache=false")
            .arg(format!("--input='{}'", serde_json::to_string(&input)?));
        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(Error::msg(format!(
                "Gemini executable failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let result = String::from_utf8_lossy(&output.stdout);
        let output: Output = serde_json::from_str(&result)?;
        Ok(output.actions)
    }
}
