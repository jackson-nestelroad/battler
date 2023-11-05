use crate::{
    battle::{
        Battle,
        Mon,
        MonContext,
    },
    battle_event,
    battler_error,
    common::{
        Error,
        WrapResultError,
    },
};

/// Switches a Mon into the given position.
pub fn switch_in(context: &mut MonContext, position: usize) -> Result<(), Error> {
    if context.mon_mut().active {
        context
            .battle_mut()
            .hint("A switch failed because the Mon trying to switch in is already in.");
        return Ok(());
    }

    let active_len = context.player().active.len();
    if position >= active_len {
        return Err(battler_error!(
            "Invalid switch position {position} / {active_len}"
        ));
    }

    let prev = context
        .player()
        .active
        .get(position)
        .cloned()
        .wrap_error_with_format(format_args!(
            "expected {position} to be a valid index to active Mons"
        ))?;
    if let Some(mon) = prev {
        let mon = context.battle().mon_mut(mon)?;
        mon.switch_out();
    }
    Mon::switch_in(context, position);
    context.player_mut().active[position] = Some(context.mon_handle());

    let event = battle_event!("switch", Mon::active_details(context)?);
    context.battle_mut().log(event);

    Ok(())
}
