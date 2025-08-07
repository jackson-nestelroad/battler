use anyhow::Result;
use battler_data::Id;

use crate::{
    battle::{
        Context,
        MonHandle,
        MoveHandle,
    },
    effect::{
        AppliedEffectLocation,
        fxlang,
    },
};
/// [`EffectStateConnector`][`crate::effect::fxlang::EffectStateConnector`] implementation for an
/// active move.
#[derive(Debug, Clone)]
pub struct ActiveMoveEffectStateConnector {
    active_move: MoveHandle,
}

impl ActiveMoveEffectStateConnector {
    pub fn new(active_move: MoveHandle) -> Self {
        Self { active_move }
    }
}

impl fxlang::EffectStateConnector for ActiveMoveEffectStateConnector {
    fn exists(&self, context: &mut Context) -> Result<bool> {
        Ok(context.active_move(self.active_move).is_ok())
    }

    fn get_mut<'a>(&self, context: &'a mut Context) -> Result<Option<&'a mut fxlang::EffectState>> {
        Ok(Some(
            &mut context.active_move_mut(self.active_move)?.effect_state,
        ))
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::None
    }

    fn make_dynamic(&self) -> fxlang::DynamicEffectStateConnector {
        fxlang::DynamicEffectStateConnector::new(self.clone())
    }
}

/// [`EffectStateConnector`][`crate::effect::fxlang::EffectStateConnector`] implementation for an
/// ability on a Mon.
#[derive(Debug, Clone)]
pub struct MonAbilityEffectStateConnector {
    mon: MonHandle,
}

impl MonAbilityEffectStateConnector {
    pub fn new(mon: MonHandle) -> Self {
        Self { mon }
    }
}

impl fxlang::EffectStateConnector for MonAbilityEffectStateConnector {
    fn exists(&self, _: &mut Context) -> Result<bool> {
        Ok(true)
    }

    fn get_mut<'a>(&self, context: &'a mut Context) -> Result<Option<&'a mut fxlang::EffectState>> {
        Ok(Some(&mut context.mon_mut(self.mon)?.ability.effect_state))
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::MonAbility(self.mon)
    }

    fn make_dynamic(&self) -> fxlang::DynamicEffectStateConnector {
        fxlang::DynamicEffectStateConnector::new(self.clone())
    }
}

/// [`EffectStateConnector`][`crate::effect::fxlang::EffectStateConnector`] implementation for an
/// item on a Mon.
#[derive(Debug, Clone)]
pub struct MonItemEffectStateConnector {
    mon: MonHandle,
}

impl MonItemEffectStateConnector {
    pub fn new(mon: MonHandle) -> Self {
        Self { mon }
    }
}

impl fxlang::EffectStateConnector for MonItemEffectStateConnector {
    fn exists(&self, context: &mut Context) -> Result<bool> {
        Ok(context.mon(self.mon)?.item.is_some())
    }

    fn get_mut<'a>(&self, context: &'a mut Context) -> Result<Option<&'a mut fxlang::EffectState>> {
        Ok(context
            .mon_mut(self.mon)?
            .item
            .as_mut()
            .map(|item| &mut item.effect_state))
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::MonItem(self.mon)
    }

    fn make_dynamic(&self) -> fxlang::DynamicEffectStateConnector {
        fxlang::DynamicEffectStateConnector::new(self.clone())
    }
}

/// [`EffectStateConnector`][`crate::effect::fxlang::EffectStateConnector`] implementation for a
/// status on a Mon.
#[derive(Debug, Clone)]
pub struct MonStatusEffectStateConnector {
    mon: MonHandle,
}

impl MonStatusEffectStateConnector {
    pub fn new(mon: MonHandle) -> Self {
        Self { mon }
    }
}

impl fxlang::EffectStateConnector for MonStatusEffectStateConnector {
    fn exists(&self, context: &mut Context) -> Result<bool> {
        Ok(context.mon(self.mon)?.status.is_some())
    }

    fn get_mut<'a>(&self, context: &'a mut Context) -> Result<Option<&'a mut fxlang::EffectState>> {
        Ok(Some(&mut context.mon_mut(self.mon)?.status_state))
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::MonStatus(self.mon)
    }

    fn make_dynamic(&self) -> fxlang::DynamicEffectStateConnector {
        fxlang::DynamicEffectStateConnector::new(self.clone())
    }
}

/// [`EffectStateConnector`][`crate::effect::fxlang::EffectStateConnector`] implementation for a
/// volatile status on a Mon.
#[derive(Debug, Clone)]
pub struct MonVolatileStatusEffectStateConnector {
    mon: MonHandle,
    volatile: Id,
}

impl MonVolatileStatusEffectStateConnector {
    pub fn new(mon: MonHandle, volatile: Id) -> Self {
        Self { mon, volatile }
    }
}

impl fxlang::EffectStateConnector for MonVolatileStatusEffectStateConnector {
    fn exists(&self, context: &mut Context) -> Result<bool> {
        Ok(context
            .mon(self.mon)?
            .volatiles
            .contains_key(&self.volatile))
    }

    fn get_mut<'a>(&self, context: &'a mut Context) -> Result<Option<&'a mut fxlang::EffectState>> {
        Ok(context.mon_mut(self.mon)?.volatiles.get_mut(&self.volatile))
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::MonVolatile(self.mon)
    }

    fn make_dynamic(&self) -> fxlang::DynamicEffectStateConnector {
        fxlang::DynamicEffectStateConnector::new(self.clone())
    }
}

/// [`EffectStateConnector`][`crate::effect::fxlang::EffectStateConnector`] implementation for a
/// condition on a side.
#[derive(Debug, Clone)]
pub struct SideConditionEffectStateConnector {
    side: usize,
    condition: Id,
}

impl SideConditionEffectStateConnector {
    pub fn new(side: usize, condition: Id) -> Self {
        Self { side, condition }
    }
}

impl fxlang::EffectStateConnector for SideConditionEffectStateConnector {
    fn exists(&self, context: &mut Context) -> Result<bool> {
        Ok(context
            .side_context(self.side)?
            .side()
            .conditions
            .contains_key(&self.condition))
    }

    fn get_mut<'a>(&self, context: &'a mut Context) -> Result<Option<&'a mut fxlang::EffectState>> {
        Ok(context
            .battle_mut()
            .side_mut(self.side)?
            .conditions
            .get_mut(&self.condition))
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::SideCondition(self.side)
    }

    fn make_dynamic(&self) -> fxlang::DynamicEffectStateConnector {
        fxlang::DynamicEffectStateConnector::new(self.clone())
    }
}

/// [`EffectStateConnector`][`crate::effect::fxlang::EffectStateConnector`] implementation for a
/// condition on a slot.
#[derive(Debug, Clone)]
pub struct SlotConditionEffectStateConnector {
    side: usize,
    slot: usize,
    condition: Id,
}

impl SlotConditionEffectStateConnector {
    pub fn new(side: usize, slot: usize, condition: Id) -> Self {
        Self {
            side,
            slot,
            condition,
        }
    }
}

impl fxlang::EffectStateConnector for SlotConditionEffectStateConnector {
    fn exists(&self, context: &mut Context) -> Result<bool> {
        Ok(context
            .side_context(self.side)?
            .side()
            .slot_conditions
            .get(&self.slot)
            .is_some_and(|conditions| conditions.contains_key(&self.condition)))
    }

    fn get_mut<'a>(&self, context: &'a mut Context) -> Result<Option<&'a mut fxlang::EffectState>> {
        match context
            .battle_mut()
            .side_mut(self.side)?
            .slot_conditions
            .get_mut(&self.slot)
        {
            Some(conditions) => Ok(conditions.get_mut(&self.condition)),
            None => Ok(None),
        }
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::SlotCondition(self.side, self.slot)
    }

    fn make_dynamic(&self) -> fxlang::DynamicEffectStateConnector {
        fxlang::DynamicEffectStateConnector::new(self.clone())
    }
}

/// [`EffectStateConnector`][`crate::effect::fxlang::EffectStateConnector`] implementation for the
/// terrain on the field.
#[derive(Debug, Clone)]
pub struct TerrainEffectStateConnector {}

impl TerrainEffectStateConnector {
    pub fn new() -> Self {
        Self {}
    }
}

impl fxlang::EffectStateConnector for TerrainEffectStateConnector {
    fn exists(&self, context: &mut Context) -> Result<bool> {
        Ok(context.battle().field.terrain.is_some())
    }

    fn get_mut<'a>(&self, context: &'a mut Context) -> Result<Option<&'a mut fxlang::EffectState>> {
        Ok(Some(&mut context.battle_mut().field.terrain_state))
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::Terrain
    }

    fn make_dynamic(&self) -> fxlang::DynamicEffectStateConnector {
        fxlang::DynamicEffectStateConnector::new(self.clone())
    }
}

/// [`EffectStateConnector`][`crate::effect::fxlang::EffectStateConnector`] implementation for
/// weather on the field.
#[derive(Debug, Clone)]
pub struct WeatherEffectStateConnector {}

impl WeatherEffectStateConnector {
    pub fn new() -> Self {
        Self {}
    }
}

impl fxlang::EffectStateConnector for WeatherEffectStateConnector {
    fn exists(&self, context: &mut Context) -> Result<bool> {
        Ok(context.battle().field.weather.is_some())
    }

    fn get_mut<'a>(&self, context: &'a mut Context) -> Result<Option<&'a mut fxlang::EffectState>> {
        Ok(Some(&mut context.battle_mut().field.weather_state))
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::Weather
    }

    fn make_dynamic(&self) -> fxlang::DynamicEffectStateConnector {
        fxlang::DynamicEffectStateConnector::new(self.clone())
    }
}

/// [`EffectStateConnector`][`crate::effect::fxlang::EffectStateConnector`] implementation for a
/// pseudo-weather on the field.
#[derive(Debug, Clone)]
pub struct PseudoWeatherEffectStateConnector {
    pseudo_weather: Id,
}

impl PseudoWeatherEffectStateConnector {
    pub fn new(pseudo_weather: Id) -> Self {
        Self { pseudo_weather }
    }
}

impl fxlang::EffectStateConnector for PseudoWeatherEffectStateConnector {
    fn exists(&self, context: &mut Context) -> Result<bool> {
        Ok(context
            .battle()
            .field
            .pseudo_weathers
            .contains_key(&self.pseudo_weather))
    }

    fn get_mut<'a>(&self, context: &'a mut Context) -> Result<Option<&'a mut fxlang::EffectState>> {
        Ok(context
            .battle_mut()
            .field
            .pseudo_weathers
            .get_mut(&self.pseudo_weather))
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::PseudoWeather
    }

    fn make_dynamic(&self) -> fxlang::DynamicEffectStateConnector {
        fxlang::DynamicEffectStateConnector::new(self.clone())
    }
}
