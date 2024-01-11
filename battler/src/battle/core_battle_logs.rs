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
        Mon::position_details(context)?,
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
    let mut event = log_event!("move", Mon::position_details(context)?, ("name", move_name));
    if let Some(target) = target {
        event.extend(&Mon::position_details(
            &context.as_battle_context_mut().mon_context(target)?,
        )?);
    }
    context.battle_mut().log(event);
    Ok(())
}

pub fn fail(context: &mut MonContext) -> Result<(), Error> {
    let event = log_event!("fail", Mon::position_details(context)?);
    context.battle_mut().log(event);
    Ok(())
}

pub fn immune(context: &mut MonContext) -> Result<(), Error> {
    let event = log_event!("immune", Mon::position_details(context)?);
    context.battle_mut().log(event);
    Ok(())
}

fn move_event_on_target(context: &mut ActiveTargetContext, event: &str) -> Result<(), Error> {
    let user_details = Mon::position_details(context.as_mon_context())?;
    let mut event = log_event!(event, user_details);
    let target_context = context.target_mon_context()?;
    let target_details = Mon::position_details(&target_context)?;
    event.extend(&target_details);
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

pub fn damage(context: &mut ApplyingEffectContext) -> Result<(), Error> {
    todo!("damage log unimplemented")
}

pub fn hit_count(context: &mut ActiveMoveContext, hits: u8) -> Result<(), Error> {
    let event = log_event!("hitcount", ("hits", hits));
    context.battle_mut().log(event);
    Ok(())
}

pub fn ohko(context: &mut Context) -> Result<(), Error> {
    let event = log_event!("ohko");
    context.battle_mut().log(event);
    Ok(())
}
