use ahash::HashMap;
use anyhow::Result;
use battler_data::Id;
use uuid::Uuid;

use crate::{
    battle::{
        Context,
        EffectContext,
    },
    effect::{
        AppliedEffectHandle,
        AppliedEffectLocation,
        EffectHandle,
    },
    error::WrapOptionError,
};

/// Object for managing applied effects in battle and how they link to one another.
///
/// A linked effect ends when another effect ends. Linking always goes both ways.
pub struct LinkedEffectsManager {
    effects: HashMap<Uuid, AppliedEffectHandle>,
}

impl LinkedEffectsManager {
    /// Creates a new linked effects manager.
    pub fn new() -> Self {
        Self {
            effects: HashMap::default(),
        }
    }

    fn get_linked_id(context: &mut Context, effect: &AppliedEffectHandle) -> Result<Option<Uuid>> {
        let connector = match effect.effect_state_connector() {
            Some(connector) => connector,
            None => return Ok(None),
        };
        if !connector.exists(context)? {
            return Ok(None);
        }
        if let Some(uuid) = connector.get_mut(context)?.linked_id() {
            return Ok(Some(uuid));
        }
        let uuid = Uuid::new_v4();
        connector.get_mut(context)?.set_linked_id(uuid);
        context
            .battle_mut()
            .linked_effects_manager
            .effects
            .insert(uuid, effect.to_owned());
        Ok(Some(uuid))
    }

    /// Links one effect to another.
    pub fn link(
        context: &mut Context,
        from: &AppliedEffectHandle,
        to: &AppliedEffectHandle,
    ) -> Result<bool> {
        let from_uuid = match Self::get_linked_id(context, from)? {
            Some(uuid) => uuid,
            None => return Ok(false),
        };
        let to_uuid = match Self::get_linked_id(context, to)? {
            Some(uuid) => uuid,
            None => return Ok(false),
        };
        from.effect_state_connector()
            .wrap_expectation("expected applied effect to have effect state after getting link id")?
            .get_mut(context)?
            .add_link(to_uuid);
        to.effect_state_connector()
            .wrap_expectation("expected applied effect to have effect state after getting link id")?
            .get_mut(context)?
            .add_link(from_uuid);
        Ok(true)
    }

    /// Removes an effect by ID, effectively ending all linked effects.
    pub fn remove_by_id(
        context: &mut EffectContext,
        id: &Id,
        location: AppliedEffectLocation,
    ) -> Result<()> {
        let effect_handle = context.battle_mut().get_effect_handle_by_id(id)?.clone();
        Self::remove(context, effect_handle, location)
    }

    /// Removes an effect, effectively ending all linked effects.
    pub fn remove(
        context: &mut EffectContext,
        effect: EffectHandle,
        location: AppliedEffectLocation,
    ) -> Result<()> {
        let effect = AppliedEffectHandle::new(effect, location);
        let connector = match effect.effect_state_connector() {
            Some(connector) => connector,
            None => return Ok(()),
        };
        if !connector.exists(context.as_battle_context_mut())? {
            return Ok(());
        }

        if let Some(linked_id) = connector
            .get_mut(context.as_battle_context_mut())?
            .linked_id()
        {
            context
                .battle_mut()
                .linked_effects_manager
                .effects
                .remove(&linked_id);
        }

        let mut context = context.forward_effect_context(effect.effect_handle.clone())?;
        for link in connector
            .get_mut(context.as_battle_context_mut())?
            .linked_to()
            .iter()
            .cloned()
            .collect::<Vec<_>>()
        {
            if let Some(effect) = context
                .battle_mut()
                .linked_effects_manager
                .effects
                .remove(&link)
            {
                effect.end(&mut context)?;
            }
        }

        Ok(())
    }
}
