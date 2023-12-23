use crate::{
    battle::{
        ActiveMoveContext,
        Battle,
        Mon,
        MonContext,
        MonHandle,
    },
    battle_event,
    common::Error,
};

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

pub fn miss(context: &mut ActiveMoveContext, target: MonHandle) -> Result<(), Error> {
    let user_details = Mon::position_details(context.as_mon_context())?;
    let mut event = battle_event!("miss", user_details);
    let target_context = context.target_mon_context(target)?;
    let target_details = Mon::position_details(&target_context)?;
    event.push(&target_details);
    context.battle_mut().log(event);
    Ok(())
}
