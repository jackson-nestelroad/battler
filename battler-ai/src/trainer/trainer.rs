use anyhow::{
    Error,
    Result,
};
use battler::{
    LearnMoveRequest,
    Request,
    SwitchRequest,
    TeamPreviewRequest,
    TurnRequest,
};
use battler_prng::PseudoRandomNumberGenerator;

use crate::{
    AiContext,
    BattlerAi,
};

#[allow(unused)]
pub struct Trainer {
    prng: Box<dyn PseudoRandomNumberGenerator>,
}

impl BattlerAi for Trainer {
    fn make_choice(&mut self, context: AiContext, request: Request) -> Result<String> {
        match request {
            Request::TeamPreview(request) => self.team_preview(context, request),
            Request::Turn(request) => self.turn(context, request),
            Request::Switch(request) => self.switch(context, request),
            Request::LearnMove(request) => self.learn_move(context, request),
        }
    }
}

#[allow(unused)]
impl Trainer {
    pub fn new(prng: Box<dyn PseudoRandomNumberGenerator>) -> Self {
        Self { prng }
    }

    fn team_preview(&mut self, context: AiContext, request: TeamPreviewRequest) -> Result<String> {
        return Err(Error::msg("team preview is not implemented"));
    }

    fn turn(&mut self, context: AiContext, request: TurnRequest) -> Result<String> {
        return Err(Error::msg("turn is not implemented"));
    }

    fn switch(&mut self, context: AiContext, request: SwitchRequest) -> Result<String> {
        return Err(Error::msg("switch is not implemented"));
    }

    fn learn_move(&mut self, context: AiContext, request: LearnMoveRequest) -> Result<String> {
        return Err(Error::msg("learn move is not implemented"));
    }
}
