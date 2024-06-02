use ahash::HashMapExt;

use crate::{
    common::{
        Error,
        FastHashMap,
    },
    effect::fxlang::{
        BattleEvent,
        Callbacks,
        ParsedProgram,
        Program,
    },
};

/// Parsed version of [`Callbacks`][`crate::effect::fxlang::Callbacks`].
pub struct ParsedCallbacks {
    callbacks: FastHashMap<BattleEvent, ParsedProgram>,
}

impl ParsedCallbacks {
    fn parse_and_save(
        &mut self,
        event: BattleEvent,
        program: &Option<Program>,
    ) -> Result<(), Error> {
        if let Some(program) = program {
            self.callbacks.insert(event, ParsedProgram::from(program)?);
        }
        Ok(())
    }

    /// Parses a set of input [`Callbacks`] to [`ParsedCallbacks`].
    pub fn from(callbacks: &Callbacks) -> Result<Self, Error> {
        let mut parsed = Self {
            callbacks: FastHashMap::new(),
        };
        parsed.parse_and_save(BattleEvent::BasePower, &callbacks.on_base_power)?;
        parsed.parse_and_save(BattleEvent::Duration, &callbacks.on_duration)?;
        parsed.parse_and_save(BattleEvent::UseMove, &callbacks.on_use_move)?;
        parsed.parse_and_save(BattleEvent::UseMoveMessage, &callbacks.on_use_move_message)?;
        Ok(parsed)
    }

    /// Returns the [`ParsedProgram`] for the given event.
    pub fn event(&self, event: BattleEvent) -> Option<&ParsedProgram> {
        self.callbacks.get(&event)
    }
}
