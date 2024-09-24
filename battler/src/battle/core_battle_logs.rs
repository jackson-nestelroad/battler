use itertools::Itertools;

use crate::{
    battle::{
        ApplyingEffectContext,
        Boost,
        Context,
        CoreBattle,
        Mon,
        MonContext,
        MonHandle,
        PlayerContext,
        SideEffectContext,
    },
    common::{
        Error,
        Id,
    },
    effect::{
        fxlang,
        Effect,
        EffectHandle,
    },
    log::Event,
    log_event,
    mons::{
        Stat,
        Type,
    },
};

/// Information for logging an activated effect.
#[derive(Default)]
pub struct EffectActivationContext {
    pub effect_flag_name: Option<String>,
    pub effect: Option<EffectHandle>,
    pub side: Option<usize>,
    pub slot: Option<usize>,
    pub player: Option<usize>,
    pub target: Option<MonHandle>,
    pub ignore_active_move_source_effect: bool,
    pub ignore_source_effect_equal_to_effect: bool,
    pub source_effect: Option<EffectHandle>,
    pub source: Option<MonHandle>,
    pub additional: Vec<String>,
}

pub fn effect_activation(
    context: &mut Context,
    header: String,
    activation_context: EffectActivationContext,
) -> Result<(), Error> {
    let event = effect_activation_internal(context, header, activation_context)?;
    context.battle_mut().log(event);
    Ok(())
}

fn effect_activation_internal(
    context: &mut Context,
    header: String,
    activation_context: EffectActivationContext,
) -> Result<Event, Error> {
    let mut event = log_event!(header);

    if let Some(side) = activation_context.side {
        event.set("side", side);
    }

    if let Some(slot) = activation_context.slot {
        event.set("slot", slot);
    }

    if let Some(player) = activation_context.player {
        event.set("player", &context.player_context(player)?.player().id);
    }

    if let Some(target) = activation_context.target {
        event.set("mon", Mon::position_details(&context.mon_context(target)?)?);
    }

    if let Some(effect) = &activation_context.effect {
        let effect = CoreBattle::get_effect_by_handle(context, effect)?;
        match activation_context.effect_flag_name {
            Some(effect_flag_name) => {
                event.set(effect_flag_name, effect.full_name());
            }
            None => match effect {
                Effect::ActiveMove(active_move, _) => {
                    event.set("move", &active_move.data.name);
                }
                Effect::MoveCondition(condition) | Effect::InactiveMove(condition) => {
                    event.set("move", &condition.data.name);
                }
                Effect::Ability(ability) | Effect::AbilityCondition(ability) => {
                    event.set("ability", &ability.data.name);
                }
                Effect::Item(item) | Effect::ItemCondition(item) => {
                    event.set("item", &item.data.name);
                }
                Effect::Condition(condition) => {
                    event.set(
                        condition.non_empty_condition_type_name(),
                        &condition.data.name,
                    );
                }
                Effect::NonExistent(_) => (),
            },
        }
    }

    for additional in activation_context.additional {
        match additional.split_once(':') {
            Some((first, second)) => event.set(first, second),
            None => event.add_flag(additional),
        }
    }

    let mut ignore_source = false;
    if let Some(source_effect) = activation_context.source_effect.as_ref() {
        let ignore_source_effect = (activation_context.ignore_active_move_source_effect
            && source_effect.is_active_move())
            || (activation_context.ignore_source_effect_equal_to_effect
                && activation_context
                    .effect
                    .as_ref()
                    .is_some_and(|effect| effect == source_effect));
        ignore_source = ignore_source_effect;
        if !ignore_source_effect {
            let effect = CoreBattle::get_effect_by_handle(context, &source_effect)?;
            event.set("from", effect.full_name());
        }
    }

    if activation_context.target != activation_context.source && !ignore_source {
        if let Some(source) = activation_context.source {
            event.set("of", Mon::position_details(&context.mon_context(source)?)?);
        }
    }

    Ok(event)
}

fn full_mon_details(context: &mut MonContext, header: &str) -> Result<(), Error> {
    let private_event = log_event!(header, Mon::private_active_details(context)?);
    let public_event = log_event!(header, Mon::public_active_details(context)?);
    let side = context.mon().side;
    context
        .battle_mut()
        .log_private_public(side, private_event, public_event);
    Ok(())
}

pub fn switch(context: &mut MonContext, is_drag: bool) -> Result<(), Error> {
    let title = if is_drag {
        "drag"
    } else if context.player().player_type.wild() {
        "appear"
    } else {
        "switch"
    };
    full_mon_details(context, title)
}

pub fn species_change(context: &mut MonContext) -> Result<(), Error> {
    full_mon_details(context, "specieschange")
}

pub fn forme_change(context: &mut ApplyingEffectContext) -> Result<(), Error> {
    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "formechange".to_owned(),
        activation,
    )
}

pub fn cant(
    context: &mut MonContext,
    effect: EffectHandle,
    source: Option<MonHandle>,
) -> Result<(), Error> {
    let activation = EffectActivationContext {
        effect_flag_name: Some("reason".to_owned()),
        effect: Some(effect),
        target: Some(context.mon_handle()),
        source,
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "cant".to_owned(),
        activation,
    )
}

pub fn fail(
    context: &mut MonContext,
    what: Option<EffectHandle>,
    from: Option<EffectHandle>,
) -> Result<(), Error> {
    let activation = EffectActivationContext {
        effect_flag_name: Some("what".to_owned()),
        effect: what,
        target: Some(context.mon_handle()),
        ignore_active_move_source_effect: true,
        source_effect: from,
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "fail".to_owned(),
        activation,
    )
}

pub fn fail_unboost(
    context: &mut MonContext,
    boosts: &[Boost],
    from: Option<EffectHandle>,
) -> Result<(), Error> {
    let mut activation = EffectActivationContext {
        effect_flag_name: Some("what".to_owned()),
        effect: Some(EffectHandle::NonExistent(Id::from_known("unboost"))),
        target: Some(context.mon_handle()),
        ignore_active_move_source_effect: true,
        source_effect: from,
        ..Default::default()
    };

    if !boosts.is_empty() {
        let boosts = boosts
            .iter()
            .map(|boost| boost.to_string().to_lowercase())
            .join(",");
        activation.additional.push(format!("boosts:{boosts}"));
    }
    effect_activation(
        context.as_battle_context_mut(),
        "fail".to_owned(),
        activation,
    )
}

pub fn fail_heal(context: &mut MonContext) -> Result<(), Error> {
    let activation = EffectActivationContext {
        effect_flag_name: Some("what".to_owned()),
        effect: Some(EffectHandle::NonExistent(Id::from_known("heal"))),
        target: Some(context.mon_handle()),
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "fail".to_owned(),
        activation,
    )
}

pub fn immune(context: &mut MonContext, effect: Option<EffectHandle>) -> Result<(), Error> {
    let activation = EffectActivationContext {
        target: Some(context.mon_handle()),
        ignore_active_move_source_effect: true,
        source_effect: effect,
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "immune".to_owned(),
        activation,
    )
}

fn move_event_on_target(context: &mut MonContext, event: &str) -> Result<(), Error> {
    let activation = EffectActivationContext {
        target: Some(context.mon_handle()),
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        event.to_owned(),
        activation,
    )
}

pub fn fail_target(context: &mut MonContext) -> Result<(), Error> {
    move_event_on_target(context, "fail")
}

pub fn miss(context: &mut MonContext) -> Result<(), Error> {
    move_event_on_target(context, "miss")
}

pub fn super_effective(context: &mut MonContext) -> Result<(), Error> {
    move_event_on_target(context, "supereffective")
}

pub fn resisted(context: &mut MonContext) -> Result<(), Error> {
    move_event_on_target(context, "resisted")
}

pub fn critical_hit(context: &mut MonContext) -> Result<(), Error> {
    move_event_on_target(context, "crit")
}

pub fn ohko(context: &mut MonContext) -> Result<(), Error> {
    move_event_on_target(context, "ohko")
}

pub fn faint(context: &mut MonContext) -> Result<(), Error> {
    move_event_on_target(context, "faint")
}

pub fn damage(
    context: &mut MonContext,
    effect: Option<EffectHandle>,
    source: Option<MonHandle>,
) -> Result<(), Error> {
    let activation = EffectActivationContext {
        target: Some(context.mon_handle()),
        ignore_active_move_source_effect: true,
        source_effect: effect,
        source,
        ..Default::default()
    };
    let mut private_event = effect_activation_internal(
        context.as_battle_context_mut(),
        "damage".to_owned(),
        activation,
    )?;
    let mut public_event = private_event.clone();

    private_event.set("health", Mon::secret_health(context));
    public_event.set("health", Mon::public_health(context));

    let side = context.mon().side;
    context
        .battle_mut()
        .log_private_public(side, private_event, public_event);
    Ok(())
}

pub fn heal(context: &mut ApplyingEffectContext) -> Result<(), Error> {
    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        ignore_active_move_source_effect: true,
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        ..Default::default()
    };
    let mut private_event = effect_activation_internal(
        context.as_battle_context_mut(),
        "heal".to_owned(),
        activation,
    )?;
    let mut public_event = private_event.clone();

    let context = &mut context.target_context()?;
    private_event.set("health", Mon::secret_health(&context));
    public_event.set("health", Mon::public_health(&context));

    let side = context.mon().side;
    context
        .battle_mut()
        .log_private_public(side, private_event, public_event);
    Ok(())
}

pub fn set_hp(
    context: &mut MonContext,
    effect: Option<EffectHandle>,
    source: Option<MonHandle>,
) -> Result<(), Error> {
    let activation = EffectActivationContext {
        target: Some(context.mon_handle()),
        ignore_active_move_source_effect: true,
        source_effect: effect,
        source,
        ..Default::default()
    };
    let mut private_event = effect_activation_internal(
        context.as_battle_context_mut(),
        "sethp".to_owned(),
        activation,
    )?;
    let mut public_event = private_event.clone();

    private_event.set("health", Mon::secret_health(context));
    public_event.set("health", Mon::public_health(context));

    let side = context.mon().side;
    context
        .battle_mut()
        .log_private_public(side, private_event, public_event);
    Ok(())
}

pub fn revive(
    context: &mut MonContext,
    effect: Option<EffectHandle>,
    source: Option<MonHandle>,
) -> Result<(), Error> {
    let activation = EffectActivationContext {
        target: Some(context.mon_handle()),
        ignore_active_move_source_effect: true,
        source_effect: effect,
        source,
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "revive".to_owned(),
        activation,
    )
}

pub fn cure_status(
    context: &mut ApplyingEffectContext,
    status: &Id,
    log_effect: bool,
) -> Result<(), Error> {
    let status = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &status)?
        .name()
        .to_owned();
    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        ignore_active_move_source_effect: true,
        source_effect: if log_effect {
            Some(context.effect_handle().clone())
        } else {
            None
        },
        source: context.source_handle(),
        additional: vec![format!("status:{status}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "curestatus".to_owned(),
        activation,
    )
}

pub fn add_volatile(context: &mut ApplyingEffectContext, volatile: &Id) -> Result<(), Error> {
    if !context.battle().engine_options.log_volatile_statuses {
        return Ok(());
    }
    let volatile = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &volatile)?
        .name()
        .to_owned();
    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("volatile:{volatile}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "addvolatile".to_owned(),
        activation,
    )
}

pub fn remove_volatile(context: &mut ApplyingEffectContext, volatile: &Id) -> Result<(), Error> {
    if !context.battle().engine_options.log_volatile_statuses {
        return Ok(());
    }
    let volatile = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &volatile)?
        .name()
        .to_owned();
    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("volatile:{volatile}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "removevolatile".to_owned(),
        activation,
    )
}

pub fn add_side_condition(context: &mut SideEffectContext, condition: &Id) -> Result<(), Error> {
    if !context.battle().engine_options.log_side_conditions {
        return Ok(());
    }
    let condition = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &condition)?
        .name()
        .to_owned();
    let activation = EffectActivationContext {
        side: Some(context.side().index),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("condition:{condition}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "addsidecondition".to_owned(),
        activation,
    )
}

pub fn remove_side_condition(context: &mut SideEffectContext, condition: &Id) -> Result<(), Error> {
    if !context.battle().engine_options.log_side_conditions {
        return Ok(());
    }
    let condition = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &condition)?
        .name()
        .to_owned();
    let activation = EffectActivationContext {
        side: Some(context.side().index),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("condition:{condition}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "removesidecondition".to_owned(),
        activation,
    )
}

pub fn add_slot_condition(
    context: &mut SideEffectContext,
    slot: usize,
    condition: &Id,
) -> Result<(), Error> {
    if !context.battle().engine_options.log_slot_conditions {
        return Ok(());
    }
    let condition = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &condition)?
        .name()
        .to_owned();
    let activation = EffectActivationContext {
        side: Some(context.side().index),
        slot: Some(slot),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("condition:{condition}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "addslotcondition".to_owned(),
        activation,
    )
}

pub fn remove_slot_condition(
    context: &mut SideEffectContext,
    slot: usize,
    condition: &Id,
) -> Result<(), Error> {
    if !context.battle().engine_options.log_slot_conditions {
        return Ok(());
    }
    let condition = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &condition)?
        .name()
        .to_owned();
    let activation = EffectActivationContext {
        side: Some(context.side().index),
        slot: Some(slot),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("condition:{condition}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "removeslotcondition".to_owned(),
        activation,
    )
}

pub fn type_change(
    context: &mut MonContext,
    types: &[Type],
    effect: Option<EffectHandle>,
    source: Option<MonHandle>,
) -> Result<(), Error> {
    let types = types.iter().map(|typ| typ.to_string()).join("/");
    let activation = EffectActivationContext {
        target: Some(context.mon_handle()),
        ignore_active_move_source_effect: true,
        source_effect: effect,
        source,
        additional: vec![format!("types:{types}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "typechange".to_owned(),
        activation,
    )
}

pub fn transform(context: &mut ApplyingEffectContext, target: MonHandle) -> Result<(), Error> {
    let into = Mon::position_details(&context.as_battle_context_mut().mon_context(target)?)?;
    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        ignore_active_move_source_effect: true,
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("into:{into}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "transform".to_owned(),
        activation,
    )
}

pub fn ability(context: &mut ApplyingEffectContext) -> Result<(), Error> {
    let ability = context
        .battle()
        .dex
        .abilities
        .get_by_id(&context.target().ability.id)?
        .data
        .name
        .clone();
    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("ability:{ability}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "ability".to_owned(),
        activation,
    )
}

pub fn end_ability(context: &mut ApplyingEffectContext) -> Result<(), Error> {
    let ability = context
        .battle()
        .dex
        .abilities
        .get_by_id(&context.target().ability.id)?
        .data
        .name
        .clone();
    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("ability:{ability}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "endability".to_owned(),
        activation,
    )
}

pub fn item(
    context: &mut MonContext,
    item: &Id,
    effect: Option<EffectHandle>,
    source: Option<MonHandle>,
) -> Result<(), Error> {
    let item = context
        .battle()
        .dex
        .items
        .get_by_id(item)?
        .data
        .name
        .clone();

    let activation = EffectActivationContext {
        target: Some(context.mon_handle()),
        source_effect: effect,
        source,
        additional: vec![format!("item:{item}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "item".to_owned(),
        activation,
    )
}

pub fn item_end(
    context: &mut MonContext,
    item: &Id,
    effect: Option<EffectHandle>,
    source: Option<MonHandle>,
    silent: bool,
    eat: bool,
) -> Result<(), Error> {
    let item = context
        .battle()
        .dex
        .items
        .get_by_id(item)?
        .data
        .name
        .clone();
    let mut additional = vec![format!("item:{item}")];
    if silent {
        additional.push("silent".to_owned());
    }
    if eat {
        additional.push("eat".to_owned());
    }
    let activation = EffectActivationContext {
        target: Some(context.mon_handle()),
        source_effect: effect,
        source,
        additional,
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "itemend".to_owned(),
        activation,
    )
}

pub fn use_move(
    context: &mut MonContext,
    move_name: &str,
    target: Option<MonHandle>,
    animate_only: bool,
) -> Result<(), Error> {
    let title = if animate_only { "animatemove" } else { "move" };
    let mut event = log_event!(
        title,
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
            Mon::position_details_or_previous(&context.mon_context(target)?)?
        ));
    }
    context
        .battle_mut()
        .add_attribute_value_to_last_move("spread", target_positions.into_iter().join(";"));
    Ok(())
}

pub fn hit_count(context: &mut Context, hits: u8) -> Result<(), Error> {
    let event = log_event!("hitcount", ("hits", hits));
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

pub fn experience(context: &mut MonContext, exp: u32) -> Result<(), Error> {
    let event = log_event!(
        "exp",
        ("mon", Mon::position_details(context)?),
        ("exp", exp)
    );
    context.battle_mut().log(event);
    Ok(())
}

pub fn level_up(context: &mut MonContext) -> Result<(), Error> {
    let event = log_event!(
        "levelup",
        ("mon", Mon::position_details(context)?),
        ("level", context.mon().level),
        ("hp", context.mon().base_max_hp),
        ("atk", context.mon().stats.get(Stat::Atk)),
        ("def", context.mon().stats.get(Stat::Def)),
        ("spa", context.mon().stats.get(Stat::SpAtk)),
        ("spd", context.mon().stats.get(Stat::SpDef)),
        ("spe", context.mon().stats.get(Stat::Spe)),
    );
    context.battle_mut().log(event);
    Ok(())
}

pub fn cannot_escape(context: &mut PlayerContext) -> Result<(), Error> {
    let event = log_event!("cannotescape", ("player", &context.player().id));
    context.battle_mut().log(event);
    Ok(())
}

pub fn escaped(context: &mut PlayerContext) -> Result<(), Error> {
    let event = log_event!("escaped", ("player", &context.player().id));
    context.battle_mut().log(event);
    Ok(())
}

pub fn forfeited(context: &mut PlayerContext) -> Result<(), Error> {
    let event = log_event!("forfeited", ("player", &context.player().id));
    context.battle_mut().log(event);
    Ok(())
}

pub fn use_item(
    context: &mut PlayerContext,
    item: &Id,
    target: Option<MonHandle>,
) -> Result<(), Error> {
    let item = context.battle().dex.items.get_by_id(item)?;
    let mut event = log_event!(
        "useitem",
        ("player", &context.player().id),
        ("name", item.data.name.clone())
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

pub fn fail_use_item(
    context: &mut PlayerContext,
    item: &Id,
    from: Option<EffectHandle>,
) -> Result<(), Error> {
    let activation = EffectActivationContext {
        effect_flag_name: Some("what".to_owned()),
        effect: Some(EffectHandle::Item(item.clone())),
        player: Some(context.player().index),
        source_effect: from,
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "fail".to_owned(),
        activation,
    )
}

pub fn deduct_pp(
    context: &mut ApplyingEffectContext,
    move_id: &Id,
    delta: u8,
) -> Result<(), Error> {
    let activation = EffectActivationContext {
        effect: Some(EffectHandle::InactiveMove(move_id.clone())),
        target: Some(context.target_handle()),
        ignore_active_move_source_effect: true,
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("by:{delta}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "deductpp".to_owned(),
        activation,
    )
}

pub fn restore_pp(
    context: &mut ApplyingEffectContext,
    move_id: &Id,
    delta: u8,
) -> Result<(), Error> {
    let activation = EffectActivationContext {
        effect: Some(EffectHandle::InactiveMove(move_id.clone())),
        target: Some(context.target_handle()),
        ignore_active_move_source_effect: true,
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("by:{delta}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "restorepp".to_owned(),
        activation,
    )
}

pub fn set_pp(context: &mut ApplyingEffectContext, move_id: &Id, pp: u8) -> Result<(), Error> {
    let activation = EffectActivationContext {
        effect: Some(EffectHandle::InactiveMove(move_id.clone())),
        target: Some(context.target_handle()),
        ignore_active_move_source_effect: true,
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("to:{pp}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "setpp".to_owned(),
        activation,
    )
}

pub fn clear_negative_boosts(context: &mut ApplyingEffectContext) -> Result<(), Error> {
    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        ignore_active_move_source_effect: true,
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "clearnegativeboosts".to_owned(),
        activation,
    )
}

pub fn uncatchable(
    context: &mut PlayerContext,
    target: MonHandle,
    wild: bool,
) -> Result<(), Error> {
    let mut additional = Vec::new();
    if !wild {
        additional.push("thief".to_owned());
    }
    let activation = EffectActivationContext {
        player: Some(context.player().index),
        target: Some(target),
        additional,
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "uncatchable".to_owned(),
        activation,
    )
}

pub fn catch_failed(
    context: &mut PlayerContext,
    target: MonHandle,
    item: &Id,
    shakes: u8,
    critical: bool,
) -> Result<(), Error> {
    let mut additional = vec![format!("shakes:{shakes}")];
    if critical {
        additional.push("critical".to_owned());
    }
    let activation = EffectActivationContext {
        player: Some(context.player().index),
        effect: Some(EffectHandle::Item(item.clone())),
        target: Some(target),
        additional,
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "catchfailed".to_owned(),
        activation,
    )
}

pub fn catch(
    context: &mut PlayerContext,
    target: MonHandle,
    item: &Id,
    shakes: u8,
    critical: bool,
) -> Result<(), Error> {
    let mut additional = vec![format!("shakes:{shakes}")];
    if critical {
        additional.push("critical".to_owned());
    }
    let activation = EffectActivationContext {
        player: Some(context.player().index),
        effect: Some(EffectHandle::Item(item.clone())),
        target: Some(target),
        additional,
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "catch".to_owned(),
        activation,
    )
}
