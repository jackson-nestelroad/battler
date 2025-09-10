use anyhow::Result;

use crate::{
    battle::{
        EffectContext,
        MonHandle,
        core_battle_actions,
    },
    effect::{
        EffectHandle,
        MonAbilityEffectStateConnector,
        MonItemEffectStateConnector,
        MonStatusEffectStateConnector,
        MonTerastallizationEffectStateConnector,
        MonVolatileStatusEffectStateConnector,
        PseudoWeatherEffectStateConnector,
        SideConditionEffectStateConnector,
        SlotConditionEffectStateConnector,
        TerrainEffectStateConnector,
        WeatherEffectStateConnector,
        fxlang::{
            self,
            EffectStateConnector,
        },
    },
    error::WrapOptionError,
};
/// The location of an applied effect, which is an effect that is active in a battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppliedEffectLocation {
    None,
    Mon(MonHandle),
    MonAbility(MonHandle),
    MonInactiveMove(MonHandle),
    MonItem(MonHandle),
    MonPseudoWeather(MonHandle),
    MonSideCondition(usize, MonHandle),
    MonSlotCondition(usize, usize, MonHandle),
    MonStatus(MonHandle),
    MonTerastallization(MonHandle),
    MonTerrain(MonHandle),
    MonType(MonHandle),
    MonVolatile(MonHandle),
    MonWeather(MonHandle),
    PseudoWeather,
    SideCondition(usize),
    SlotCondition(usize, usize),
    Terrain,
    Weather,
}

impl AppliedEffectLocation {
    /// The [`AppliedEffectLocation`] for running the residual event, which should only decrease the
    /// effect's counter a single time.
    pub fn for_residual(&self) -> Self {
        match self {
            Self::MonPseudoWeather(_) => Self::PseudoWeather,
            Self::MonSideCondition(side, _) => Self::SideCondition(*side),
            Self::MonSlotCondition(side, slot, _) => Self::SlotCondition(*side, *slot),
            Self::MonTerrain(_) => Self::Terrain,
            Self::MonWeather(_) => Self::Weather,
            _ => *self,
        }
    }

    /// The associated [`MonHandle`] that the callback originates from.
    pub fn mon_handle(&self) -> Option<MonHandle> {
        match self {
            Self::Mon(mon)
            | Self::MonAbility(mon)
            | Self::MonItem(mon)
            | Self::MonPseudoWeather(mon)
            | Self::MonSideCondition(_, mon)
            | Self::MonSlotCondition(_, _, mon)
            | Self::MonStatus(mon)
            | Self::MonTerastallization(mon)
            | Self::MonTerrain(mon)
            | Self::MonType(mon)
            | Self::MonVolatile(mon)
            | Self::MonWeather(mon) => Some(*mon),
            _ => None,
        }
    }
}

/// Handle to an applied effect, which is active in some part of a battle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppliedEffectHandle {
    pub effect_handle: EffectHandle,
    pub location: AppliedEffectLocation,
}

impl AppliedEffectHandle {
    /// Creates a new applied effect handle.
    pub fn new(effect_handle: EffectHandle, location: AppliedEffectLocation) -> Self {
        Self {
            effect_handle,
            location,
        }
    }

    /// Creates a dynamic connector for retrieving the effect state of the applied effect.
    pub fn effect_state_connector(&self) -> Option<fxlang::DynamicEffectStateConnector> {
        match self.location {
            AppliedEffectLocation::None => None,
            AppliedEffectLocation::Mon(_) => None,
            AppliedEffectLocation::MonAbility(mon) => {
                Some(MonAbilityEffectStateConnector::new(mon).make_dynamic())
            }
            AppliedEffectLocation::MonInactiveMove(_) => None,
            AppliedEffectLocation::MonItem(mon) => {
                Some(MonItemEffectStateConnector::new(mon).make_dynamic())
            }
            AppliedEffectLocation::MonStatus(mon) => {
                Some(MonStatusEffectStateConnector::new(mon).make_dynamic())
            }
            AppliedEffectLocation::MonTerastallization(mon) => {
                Some(MonTerastallizationEffectStateConnector::new(mon).make_dynamic())
            }
            AppliedEffectLocation::MonType(_) => None,
            AppliedEffectLocation::MonVolatile(mon) => self.effect_handle.try_id().map(|id| {
                MonVolatileStatusEffectStateConnector::new(mon, id.clone()).make_dynamic()
            }),
            AppliedEffectLocation::PseudoWeather | AppliedEffectLocation::MonPseudoWeather(_) => {
                self.effect_handle
                    .try_id()
                    .map(|id| PseudoWeatherEffectStateConnector::new(id.clone()).make_dynamic())
            }
            AppliedEffectLocation::SideCondition(side)
            | AppliedEffectLocation::MonSideCondition(side, _) => self
                .effect_handle
                .try_id()
                .map(|id| SideConditionEffectStateConnector::new(side, id.clone()).make_dynamic()),
            AppliedEffectLocation::SlotCondition(side, slot)
            | AppliedEffectLocation::MonSlotCondition(side, slot, _) => {
                self.effect_handle.try_id().map(|id| {
                    SlotConditionEffectStateConnector::new(side, slot, id.clone()).make_dynamic()
                })
            }
            AppliedEffectLocation::Terrain | AppliedEffectLocation::MonTerrain(_) => {
                Some(TerrainEffectStateConnector::new().make_dynamic())
            }
            AppliedEffectLocation::Weather | AppliedEffectLocation::MonWeather(_) => {
                Some(WeatherEffectStateConnector::new().make_dynamic())
            }
        }
    }

    /// Ends the applied effect.
    pub fn end(&self, context: &mut EffectContext) -> Result<bool> {
        match self.location {
            AppliedEffectLocation::None
            | AppliedEffectLocation::Mon(_)
            | AppliedEffectLocation::MonAbility(_)
            | AppliedEffectLocation::MonInactiveMove(_)
            | AppliedEffectLocation::MonItem(_)
            | AppliedEffectLocation::MonType(_) => Ok(false),
            AppliedEffectLocation::MonStatus(mon) => {
                let mut context = context.applying_effect_context(None, mon)?;
                core_battle_actions::clear_status(&mut context)
            }
            AppliedEffectLocation::MonTerastallization(mon) => {
                let mut context = context.applying_effect_context(None, mon)?;
                core_battle_actions::end_terastallization(&mut context).map(|()| true)
            }
            AppliedEffectLocation::MonVolatile(mon) => {
                let mut context = context.applying_effect_context(None, mon)?;
                core_battle_actions::remove_volatile(
                    &mut context,
                    self.effect_handle
                        .try_id()
                        .wrap_expectation("expected volatile to have an id")?,
                    false,
                )
            }
            AppliedEffectLocation::PseudoWeather | AppliedEffectLocation::MonPseudoWeather(_) => {
                core_battle_actions::remove_pseudo_weather(
                    &mut context.field_effect_context(None)?,
                    self.effect_handle
                        .try_id()
                        .wrap_expectation("expected pseudo-weather to have an id")?,
                )
            }
            AppliedEffectLocation::SideCondition(side)
            | AppliedEffectLocation::MonSideCondition(side, _) => {
                core_battle_actions::remove_side_condition(
                    &mut context.side_effect_context(side, None)?,
                    self.effect_handle
                        .try_id()
                        .wrap_expectation("expected side condition to have an id")?,
                )
            }
            AppliedEffectLocation::SlotCondition(side, slot)
            | AppliedEffectLocation::MonSlotCondition(side, slot, _) => {
                core_battle_actions::remove_slot_condition(
                    &mut context.side_effect_context(side, None)?,
                    slot,
                    self.effect_handle
                        .try_id()
                        .wrap_expectation("expected side condition to have an id")?,
                )
            }
            AppliedEffectLocation::Terrain | AppliedEffectLocation::MonTerrain(_) => {
                core_battle_actions::clear_terrain(&mut context.field_effect_context(None)?)
            }
            AppliedEffectLocation::Weather | AppliedEffectLocation::MonWeather(_) => {
                core_battle_actions::clear_weather(&mut context.field_effect_context(None)?)
            }
        }
    }
}
