use itertools::Itertools;

use crate::{
    battle::{
        ActiveMoveContext,
        ActiveTargetContext,
        ApplyingEffectContext,
        Context,
        Mon,
        MonContext,
        MonHandle,
    },
    common::Error,
    effect::EffectType,
    log_event,
};

pub fn switch(context: &mut MonContext) -> Result<(), Error> {
    let event = log_event!("switch", Mon::active_details(context)?);
    context.battle_mut().log(event);
    Ok(())
}

pub fn cant(context: &mut MonContext, reason: &str, do_what: &str) -> Result<(), Error> {
    let event = log_event!(
        "cant",
        ("mon", Mon::position_details(context)?),
        ("reason", reason),
        ("what", do_what),
    );
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
    context.battle_mut().log(event);
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
            Mon::position_details(&mut context.mon_context(target)?)?
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

pub fn immune(context: &mut MonContext) -> Result<(), Error> {
    let event = log_event!("immune", ("mon", Mon::position_details(context)?));
    context.battle_mut().log(event);
    Ok(())
}

fn move_event_on_target(context: &mut ActiveTargetContext, event: &str) -> Result<(), Error> {
    let user_details = Mon::position_details(context.as_mon_context())?;
    let mut event = log_event!(event, ("mon", user_details));
    event.extend(&(
        "target",
        Mon::position_details(&context.target_mon_context()?)?,
    ));
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

pub fn damage(context: &mut ApplyingEffectContext) -> Result<(), Error> {
    // TODO: Handle other special cases where the damage log should have more information.
    let mut event = log_event!(
        "damage",
        ("mon", Mon::position_details(&context.target_context()?)?)
    );
    let effect_type = context.effect().effect_type();
    if effect_type != EffectType::Move {
        event.set("from", context.effect().full_name());
        let target_handle = context.target_handle();
        if let Some(source_context) = context.source_context()? {
            if source_context.mon_handle() != target_handle || effect_type == EffectType::Ability {
                event.set("source", Mon::position_details(&source_context)?);
            }
        }
    }

    context.battle_mut().log(event);
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
