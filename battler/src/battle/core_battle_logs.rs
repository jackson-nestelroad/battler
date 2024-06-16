use itertools::Itertools;

use crate::{
    battle::{
        ActiveMoveContext,
        ActiveTargetContext,
        ApplyingEffectContext,
        Boost,
        Context,
        Mon,
        MonContext,
        MonHandle,
    },
    common::Error,
    effect::{
        fxlang,
        EffectHandle,
        EffectType,
    },
    log_event,
};

pub fn switch(context: &mut MonContext) -> Result<(), Error> {
    let event = log_event!("switch", Mon::active_details(context)?);
    context.battle_mut().log(event);
    Ok(())
}

pub fn cant(context: &mut MonContext, reason: &str, do_what: Option<&str>) -> Result<(), Error> {
    let mut event = log_event!(
        "cant",
        ("mon", Mon::position_details(context)?),
        ("reason", reason),
    );
    if let Some(do_what) = do_what {
        event.set("what", do_what);
    }
    context.battle_mut().log(event);
    Ok(())
}

pub fn hint(context: &mut Context, hint: &str) -> Result<(), Error> {
    let event = log_event!("hint", ("details", hint));
    context.battle_mut().log(event);
    Ok(())
}

pub fn use_move(
    context: &mut MonContext,
    move_name: &str,
    target: Option<MonHandle>,
) -> Result<(), Error> {
    let mut event = log_event!(
        "move",
        ("mon", Mon::position_details(context)?),
        ("name", move_name)
    );
    if let Some(target) = target {
        event.extend(&(
            "target",
            Mon::position_details(&context.as_battle_context_mut().mon_context(target)?)?,
        ));
    }
    context.battle_mut().log_move(event);
    Ok(())
}

pub fn last_move_had_no_target(context: &mut Context) {
    context.battle_mut().add_attribute_to_last_move("notarget");
}

pub fn do_not_animate_last_move(context: &mut Context) {
    context.battle_mut().add_attribute_to_last_move("noanim");
}

pub fn last_move_spread_targets<I>(context: &mut Context, targets: I) -> Result<(), Error>
where
    I: IntoIterator<Item = MonHandle>,
{
    let mut target_positions = Vec::new();
    for target in targets {
        target_positions.push(format!(
            "{}",
            Mon::position_details(&context.mon_context(target)?)?
        ));
    }
    context
        .battle_mut()
        .add_attribute_value_to_last_move("spread", target_positions.into_iter().join(";"));
    Ok(())
}

pub fn fail(context: &mut MonContext) -> Result<(), Error> {
    let event = log_event!("fail", ("mon", Mon::position_details(context)?));
    context.battle_mut().log(event);
    Ok(())
}

pub fn fail_heal(context: &mut MonContext) -> Result<(), Error> {
    let event = log_event!(
        "fail",
        ("mon", Mon::position_details(context)?),
        ("what", "heal")
    );
    context.battle_mut().log(event);
    Ok(())
}

pub fn immune(context: &mut MonContext) -> Result<(), Error> {
    let event = log_event!("immune", ("mon", Mon::position_details(context)?));
    context.battle_mut().log(event);
    Ok(())
}

fn move_event_on_target(context: &mut ActiveTargetContext, event: &str) -> Result<(), Error> {
    let event = log_event!(
        event,
        (
            "mon",
            Mon::position_details(&context.target_mon_context()?)?
        )
    );
    context.battle_mut().log(event);
    Ok(())
}

pub fn fail_target(context: &mut ActiveTargetContext) -> Result<(), Error> {
    move_event_on_target(context, "fail")
}

pub fn miss(context: &mut ActiveTargetContext) -> Result<(), Error> {
    move_event_on_target(context, "miss")
}

pub fn super_effective(context: &mut ActiveTargetContext) -> Result<(), Error> {
    move_event_on_target(context, "supereffective")
}

pub fn resisted(context: &mut ActiveTargetContext) -> Result<(), Error> {
    move_event_on_target(context, "resisted")
}

pub fn critical_hit(context: &mut ActiveTargetContext) -> Result<(), Error> {
    move_event_on_target(context, "crit")
}

pub fn ohko(context: &mut ActiveTargetContext) -> Result<(), Error> {
    move_event_on_target(context, "ohko")
}

pub fn damage(
    context: &mut MonContext,
    source: Option<MonHandle>,
    effect: Option<&EffectHandle>,
) -> Result<(), Error> {
    // TODO: Handle other special cases where the damage log should have more information.
    let mut event = log_event!("damage", ("mon", Mon::position_details(context)?));
    if let Some(effect) = effect {
        if !effect.is_move() {
            let effect_context = context.as_battle_context_mut().effect_context(effect)?;
            event.set("from", effect_context.effect().full_name());

            if let Some(source) = source {
                if source != context.mon_handle() || effect.is_ability() {
                    event.set(
                        "source",
                        Mon::position_details(
                            &context.as_battle_context_mut().mon_context(source)?,
                        )?,
                    );
                }
            }
        }
    }

    let mut private_event = event;
    let mut public_event = private_event.clone();
    private_event.set("health", Mon::secret_health(context));
    public_event.set("health", Mon::public_health(context));

    let side = context.mon().side;
    context
        .battle_mut()
        .log_private_public(side, private_event, public_event);
    Ok(())
}

pub fn heal(
    context: &mut MonContext,
    source: Option<MonHandle>,
    effect: Option<&EffectHandle>,
) -> Result<(), Error> {
    let mut event = log_event!("heal", ("mon", Mon::position_details(context)?));
    if let Some(effect) = effect {
        let effect_context = context.as_battle_context_mut().effect_context(effect)?;
        if effect_context.effect().effect_type() != EffectType::Move {
            event.set("from", effect_context.effect().full_name());
            if let Some(source) = source {
                if source != context.mon_handle() {
                    event.set(
                        "of",
                        Mon::position_details(
                            &context.as_battle_context_mut().mon_context(source)?,
                        )?,
                    );
                }
            }
        }
    }

    // TODO: Let conditions add their own context. For example, "Wish" would probably want to log
    // who originally granted the Wish.

    let mut private_event = event;
    let mut public_event = private_event.clone();
    private_event.set("health", Mon::secret_health(context));
    public_event.set("health", Mon::public_health(context));

    let side = context.mon().side;
    context
        .battle_mut()
        .log_private_public(side, private_event, public_event);
    Ok(())
}

pub fn hit_count(context: &mut ActiveMoveContext, hits: u8) -> Result<(), Error> {
    let event = log_event!("hitcount", ("hits", hits));
    context.battle_mut().log(event);
    Ok(())
}

pub fn faint(context: &mut MonContext) -> Result<(), Error> {
    let event = log_event!("faint", ("mon", Mon::position_details(context)?));
    context.battle_mut().log(event);
    Ok(())
}

pub fn boost(
    context: &mut MonContext,
    boost: Boost,
    delta: i8,
    original_delta: i8,
) -> Result<(), Error> {
    let (delta, message) = if original_delta >= 0 {
        (delta as u8, "boost")
    } else {
        (-delta as u8, "unboost")
    };

    let event = log_event!(
        message,
        ("mon", Mon::position_details(context)?),
        ("stat", boost),
        ("by", delta)
    );
    context.battle_mut().log(event);
    Ok(())
}

pub fn debug_event_failure(
    context: &mut Context,
    event: fxlang::BattleEvent,
    effect_name: &str,
    error: &str,
) {
    let log_event = log_event!(
        "debug",
        ("event", event),
        ("effect", effect_name),
        ("error", error)
    );
    context.battle_mut().log(log_event);
}

pub fn debug_full_event_failure(context: &mut Context, event: fxlang::BattleEvent, error: &str) {
    let log_event = log_event!("debug", ("event", event), ("error", error));
    context.battle_mut().log(log_event);
}

pub fn cure_status(
    context: &mut ApplyingEffectContext,
    status: &str,
    include_effect: bool,
) -> Result<(), Error> {
    let mut event = log_event!(
        "curestatus",
        (
            "mon",
            Mon::position_details(&mut context.target_context()?)?
        ),
        ("status", status)
    );
    if include_effect {
        event.set("from", context.effect().name());
    }
    context.battle_mut().log(event);
    Ok(())
}

pub fn add_volatile(context: &mut ApplyingEffectContext, volatile: &str) -> Result<(), Error> {
    if !context.battle().engine_options.log_volatile_statuses {
        return Ok(());
    }
    let event = log_event!(
        "addvolatile",
        (
            "mon",
            Mon::position_details(&mut context.target_context()?)?
        ),
        ("volatile", volatile),
        ("from", context.effect().name())
    );
    context.battle_mut().log(event);
    Ok(())
}

pub fn remove_volatile(context: &mut ApplyingEffectContext, volatile: &str) -> Result<(), Error> {
    if !context.battle().engine_options.log_volatile_statuses {
        return Ok(());
    }
    let event = log_event!(
        "removevolatile",
        (
            "mon",
            Mon::position_details(&mut context.target_context()?)?
        ),
        ("volatile", volatile),
        ("from", context.effect().name())
    );
    context.battle_mut().log(event);
    Ok(())
}
