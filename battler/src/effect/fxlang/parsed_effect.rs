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
            parsed.parse_and_save(BattleEvent::AccuracyExempt, callbacks)?;
            parsed.parse_and_save(BattleEvent::AddPseudoWeather, callbacks)?;
            parsed.parse_and_save(BattleEvent::AddVolatile, callbacks)?;
            parsed.parse_and_save(BattleEvent::AfterHit, callbacks)?;
            parsed.parse_and_save(BattleEvent::AfterMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::AfterMoveSecondaryEffects, callbacks)?;
            parsed.parse_and_save(BattleEvent::AfterSetStatus, callbacks)?;
            parsed.parse_and_save(BattleEvent::AfterSubstituteDamage, callbacks)?;
            parsed.parse_and_save(BattleEvent::AllySetStatus, callbacks)?;
            parsed.parse_and_save(BattleEvent::AnyDamage, callbacks)?;
            parsed.parse_and_save(BattleEvent::AnyExit, callbacks)?;
            parsed.parse_and_save(BattleEvent::AnyPrepareHit, callbacks)?;
            parsed.parse_and_save(BattleEvent::AnySetStatus, callbacks)?;
            parsed.parse_and_save(BattleEvent::AnyTryMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::Attract, callbacks)?;
            parsed.parse_and_save(BattleEvent::BasePower, callbacks)?;
            parsed.parse_and_save(BattleEvent::BeforeMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::BeforeSwitchOut, callbacks)?;
            parsed.parse_and_save(BattleEvent::BeforeTurn, callbacks)?;
            parsed.parse_and_save(BattleEvent::ClearTerrain, callbacks)?;
            parsed.parse_and_save(BattleEvent::ClearWeather, callbacks)?;
            parsed.parse_and_save(BattleEvent::ChargeMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::CopyVolatile, callbacks)?;
            parsed.parse_and_save(BattleEvent::CriticalHit, callbacks)?;
            parsed.parse_and_save(BattleEvent::CureStatus, callbacks)?;
            parsed.parse_and_save(BattleEvent::Damage, callbacks)?;
            parsed.parse_and_save(BattleEvent::DamageReceived, callbacks)?;
            parsed.parse_and_save(BattleEvent::DamagingHit, callbacks)?;
            parsed.parse_and_save(BattleEvent::DeductPp, callbacks)?;
            parsed.parse_and_save(BattleEvent::DisableMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::DragOut, callbacks)?;
            parsed.parse_and_save(BattleEvent::Duration, callbacks)?;
            parsed.parse_and_save(BattleEvent::Eat, callbacks)?;
            parsed.parse_and_save(BattleEvent::Effectiveness, callbacks)?;
            parsed.parse_and_save(BattleEvent::End, callbacks)?;
            parsed.parse_and_save(BattleEvent::EntryHazard, callbacks)?;
            parsed.parse_and_save(BattleEvent::Exit, callbacks)?;
            parsed.parse_and_save(BattleEvent::Faint, callbacks)?;
            parsed.parse_and_save(BattleEvent::FieldEnd, callbacks)?;
            parsed.parse_and_save(BattleEvent::FieldRestart, callbacks)?;
            parsed.parse_and_save(BattleEvent::FieldResidual, callbacks)?;
            parsed.parse_and_save(BattleEvent::FieldStart, callbacks)?;
            parsed.parse_and_save(BattleEvent::Flinch, callbacks)?;
            parsed.parse_and_save(BattleEvent::FoeBeforeMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::FoeDisableMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::FoeRedirectTarget, callbacks)?;
            parsed.parse_and_save(BattleEvent::Hit, callbacks)?;
            parsed.parse_and_save(BattleEvent::HitField, callbacks)?;
            parsed.parse_and_save(BattleEvent::HitSide, callbacks)?;
            parsed.parse_and_save(BattleEvent::Immunity, callbacks)?;
            parsed.parse_and_save(BattleEvent::Invulnerability, callbacks)?;
            parsed.parse_and_save(BattleEvent::IsAsleep, callbacks)?;
            parsed.parse_and_save(BattleEvent::IsBehindSubstitute, callbacks)?;
            parsed.parse_and_save(BattleEvent::IsContactProof, callbacks)?;
            parsed.parse_and_save(BattleEvent::IsGrounded, callbacks)?;
            parsed.parse_and_save(BattleEvent::IsImmuneToEntryHazards, callbacks)?;
            parsed.parse_and_save(BattleEvent::IsRaining, callbacks)?;
            parsed.parse_and_save(BattleEvent::IsSemiInvulnerable, callbacks)?;
            parsed.parse_and_save(BattleEvent::IsSkyDropped, callbacks)?;
            parsed.parse_and_save(BattleEvent::IsSnowing, callbacks)?;
            parsed.parse_and_save(BattleEvent::IsSoundproof, callbacks)?;
            parsed.parse_and_save(BattleEvent::IsSunny, callbacks)?;
            parsed.parse_and_save(BattleEvent::LockMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifyAccuracy, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifyAtk, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifyBoosts, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifyCritChance, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifyCritRatio, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifyDamage, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifyDef, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifyExperience, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifySpA, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifySpD, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifySpe, callbacks)?;
            parsed.parse_and_save(BattleEvent::ModifyType, callbacks)?;
            parsed.parse_and_save(BattleEvent::MoveAborted, callbacks)?;
            parsed.parse_and_save(BattleEvent::MoveBasePower, callbacks)?;
            parsed.parse_and_save(BattleEvent::MoveDamage, callbacks)?;
            parsed.parse_and_save(BattleEvent::MoveFailed, callbacks)?;
            parsed.parse_and_save(BattleEvent::NegateImmunity, callbacks)?;
            parsed.parse_and_save(BattleEvent::OverrideMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::PrepareHit, callbacks)?;
            parsed.parse_and_save(BattleEvent::PriorityChargeMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::RedirectTarget, callbacks)?;
            parsed.parse_and_save(BattleEvent::Residual, callbacks)?;
            parsed.parse_and_save(BattleEvent::Restart, callbacks)?;
            parsed.parse_and_save(BattleEvent::SetLastMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::SetStatus, callbacks)?;
            parsed.parse_and_save(BattleEvent::SetTerrain, callbacks)?;
            parsed.parse_and_save(BattleEvent::SetWeather, callbacks)?;
            parsed.parse_and_save(BattleEvent::SideConditionStart, callbacks)?;
            parsed.parse_and_save(BattleEvent::SideEnd, callbacks)?;
            parsed.parse_and_save(BattleEvent::SideRestart, callbacks)?;
            parsed.parse_and_save(BattleEvent::SideResidual, callbacks)?;
            parsed.parse_and_save(BattleEvent::SideStart, callbacks)?;
            parsed.parse_and_save(BattleEvent::SideTryHitSide, callbacks)?;
            parsed.parse_and_save(BattleEvent::SlotEnd, callbacks)?;
            parsed.parse_and_save(BattleEvent::SlotRestart, callbacks)?;
            parsed.parse_and_save(BattleEvent::SlotStart, callbacks)?;
            parsed.parse_and_save(BattleEvent::SourceAccuracyExempt, callbacks)?;
            parsed.parse_and_save(BattleEvent::SourceBasePower, callbacks)?;
            parsed.parse_and_save(BattleEvent::SourceInvulnerability, callbacks)?;
            parsed.parse_and_save(BattleEvent::SourceModifyDamage, callbacks)?;
            parsed.parse_and_save(BattleEvent::SourceWeatherModifyDamage, callbacks)?;
            parsed.parse_and_save(BattleEvent::Start, callbacks)?;
            parsed.parse_and_save(BattleEvent::StallMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::SuppressFieldTerrain, callbacks)?;
            parsed.parse_and_save(BattleEvent::SuppressFieldWeather, callbacks)?;
            parsed.parse_and_save(BattleEvent::SuppressMonItem, callbacks)?;
            parsed.parse_and_save(BattleEvent::SuppressMonTerrain, callbacks)?;
            parsed.parse_and_save(BattleEvent::SuppressMonWeather, callbacks)?;
            parsed.parse_and_save(BattleEvent::SwitchIn, callbacks)?;
            parsed.parse_and_save(BattleEvent::TakeItem, callbacks)?;
            parsed.parse_and_save(BattleEvent::TrapMon, callbacks)?;
            parsed.parse_and_save(BattleEvent::TryBoost, callbacks)?;
            parsed.parse_and_save(BattleEvent::TryHit, callbacks)?;
            parsed.parse_and_save(BattleEvent::TryHitField, callbacks)?;
            parsed.parse_and_save(BattleEvent::TryHitSide, callbacks)?;
            parsed.parse_and_save(BattleEvent::TryImmunity, callbacks)?;
            parsed.parse_and_save(BattleEvent::TryMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::TryPrimaryHit, callbacks)?;
            parsed.parse_and_save(BattleEvent::TryUseMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::Types, callbacks)?;
            parsed.parse_and_save(BattleEvent::Update, callbacks)?;
            parsed.parse_and_save(BattleEvent::UseMove, callbacks)?;
            parsed.parse_and_save(BattleEvent::UseMoveMessage, callbacks)?;
            parsed.parse_and_save(BattleEvent::Weather, callbacks)?;
            parsed.parse_and_save(BattleEvent::WeatherModifyDamage, callbacks)?;
        }
        Ok(parsed)
    }

    /// Returns the [`ParsedProgram`] for the given event.
    pub fn event(&self, event: BattleEvent) -> Option<&ParsedProgram> {
        self.callbacks.get(&event)
    }
}
