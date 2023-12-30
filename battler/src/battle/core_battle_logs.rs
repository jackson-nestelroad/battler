use crate::{
    battle::{
        ActiveTargetContext,
        Context,
        Mon,
        MonContext,
    },
    battle_event,
    common::Error,
};

pub fn hint(context: &mut Context, hint: &str) -> Result<(), Error> {
    context.battle_mut().log(battle_event!("hint", hint));
    Ok(())
}

pub fn fail(context: &mut MonContext) -> Result<(), Error> {
    let event = battle_event!("fail", Mon::position_details(context)?);
    context.battle_mut().log(event);
    Ok(())
}

pub fn immune(context: &mut MonContext) -> Result<(), Error> {
    let event = battle_event!("immune", Mon::position_details(context)?);
    context.battle_mut().log(event);
    Ok(())
}

fn move_event_on_target(context: &mut ActiveTargetContext, event: &str) -> Result<(), Error> {
    let user_details = Mon::position_details(context.as_mon_context())?;
    let mut event = battle_event!(event, user_details);
    let target_context = context.target_mon_context()?;
    let target_details = Mon::position_details(&target_context)?;
    event.push(&target_details);
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
