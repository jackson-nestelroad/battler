use ahash::HashMapExt;

use crate::{
    common::{
        Error,
        FastHashMap,
        WrapResultError,
    },
    effect::fxlang::{
        BattleEvent,
        Callbacks,
        ParsedProgram,
    },
};

/// Parsed version of [`Callbacks`][`crate::effect::fxlang::Callbacks`].
pub struct ParsedCallbacks {
    callbacks: FastHashMap<BattleEvent, ParsedProgram>,
}

impl ParsedCallbacks {
    fn parse_and_save(&mut self, event: BattleEvent, callbacks: &Callbacks) -> Result<(), Error> {
        if let Some(program) = callbacks
            .event(event)
            .map(|callback| callback.program())
            .flatten()
        {
            self.callbacks.insert(
                event,
                ParsedProgram::from(program)
                    .wrap_error_with_format(format_args!("error parsing {event} callback"))?,
            );
        }
        Ok(())
    }

    /// Parses a set of input [`Callbacks`] to [`ParsedCallbacks`].
    pub fn from(callbacks: Option<&Callbacks>) -> Result<Self, Error> {
        let mut parsed = Self {
            callbacks: FastHashMap::new(),
        };
        if let Some(callbacks) = callbacks {
            parsed.parse_and_save(BattleEvent::AfterMoveSecondaryEffects, callbacks)?;
            parsed.parse_and_save(BattleEvent::AfterSetStatus, callbacks)?;
            parsed.parse_and_save(BattleEvent::BasePower, callbacks)?;
            parsed.parse_and_save(BattleEvent::BeforeMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::DamagingHit, callbacks)?;
            parsed.parse_and_save(BattleEvent::Duration, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifyAtk, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifyDamage, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifySpA, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifySpD, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifySpe, callbacks)?;
            parsed.parse_and_save(BattleEvent::Residual, callbacks)?;
            parsed.parse_and_save(BattleEvent::Start, callbacks)?;
            parsed.parse_and_save(BattleEvent::SwitchIn, callbacks)?;
            parsed.parse_and_save(BattleEvent::TryUseMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::UseMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::UseMoveMessage, callbacks)?;
        }
        Ok(parsed)
    }

    /// Returns the [`ParsedProgram`] for the given event.
    pub fn event(&self, event: BattleEvent) -> Option<&ParsedProgram> {
        self.callbacks.get(&event)
    }
}
