use crate::{
    battle::{
        Context,
        MonHandle,
        MoveHandle,
    },
    common::{
        Error,
        Id,
    },
    effect::fxlang,
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
    fn exists(&self, context: &mut Context) -> Result<bool, Error> {
        Ok(context.active_move(self.active_move).is_ok())
    }

    fn get_mut<'a>(
        &self,
        context: &'a mut Context,
    ) -> Result<Option<&'a mut fxlang::EffectState>, Error> {
        Ok(Some(
            &mut context.active_move_mut(self.active_move)?.effect_state,
        ))
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
    fn exists(&self, _: &mut Context) -> Result<bool, Error> {
        Ok(true)
    }

    fn get_mut<'a>(
        &self,
        context: &'a mut Context,
    ) -> Result<Option<&'a mut fxlang::EffectState>, Error> {
        Ok(Some(&mut context.mon_mut(self.mon)?.ability.effect_state))
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
    fn exists(&self, context: &mut Context) -> Result<bool, Error> {
        Ok(context.mon(self.mon)?.item.is_some())
    }

    fn get_mut<'a>(
        &self,
        context: &'a mut Context,
    ) -> Result<Option<&'a mut fxlang::EffectState>, Error> {
        Ok(context
            .mon_mut(self.mon)?
            .item
            .as_mut()
            .map(|item| &mut item.effect_state))
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
    fn exists(&self, _: &mut Context) -> Result<bool, Error> {
        Ok(true)
    }

    fn get_mut<'a>(
        &self,
        context: &'a mut Context,
    ) -> Result<Option<&'a mut fxlang::EffectState>, Error> {
        Ok(Some(&mut context.mon_mut(self.mon)?.status_state))
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
    fn exists(&self, context: &mut Context) -> Result<bool, Error> {
        Ok(context
            .mon(self.mon)?
            .volatiles
            .contains_key(&self.volatile))
    }

    fn get_mut<'a>(
        &self,
        context: &'a mut Context,
    ) -> Result<Option<&'a mut fxlang::EffectState>, Error> {
        Ok(context.mon_mut(self.mon)?.volatiles.get_mut(&self.volatile))
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
    fn exists(&self, context: &mut Context) -> Result<bool, Error> {
        Ok(context
            .side_context(self.side)?
            .side()
            .conditions
            .contains_key(&self.condition))
    }

    fn get_mut<'a>(
        &self,
        context: &'a mut Context,
    ) -> Result<Option<&'a mut fxlang::EffectState>, Error> {
        Ok(context
            .battle_mut()
            .side_mut(self.side)?
            .conditions
            .get_mut(&self.condition))
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
    fn exists(&self, _: &mut Context) -> Result<bool, Error> {
        Ok(true)
    }

    fn get_mut<'a>(
        &self,
        context: &'a mut Context,
    ) -> Result<Option<&'a mut fxlang::EffectState>, Error> {
        Ok(Some(&mut context.battle_mut().field.weather_state))
    }

    fn make_dynamic(&self) -> fxlang::DynamicEffectStateConnector {
        fxlang::DynamicEffectStateConnector::new(self.clone())
    }
}
