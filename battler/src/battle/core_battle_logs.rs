use anyhow::Result;
use battler_data::{
    Boost,
    Id,
    Stat,
    Type,
};
use itertools::Itertools;

use crate::{
    battle::{
        ApplyingEffectContext,
        Context,
        CoreBattle,
        Mon,
        MonContext,
        MonHandle,
        PlayerContext,
        SideEffectContext,
    },
    battle_log_entry,
    effect::{
        Effect,
        EffectHandle,
        fxlang,
    },
    general_error,
    log::UncommittedBattleLogEntry,
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
) -> Result<()> {
    let event = effect_activation_internal(context, header, activation_context)?;
    context.battle_mut().log(event);
    Ok(())
}

fn add_effect_to_log_entry(
    context: &mut Context,
    entry: &mut UncommittedBattleLogEntry,
    effect_handle: &EffectHandle,
    effect_flag_name: Option<&str>,
) -> Result<()> {
    let effect = CoreBattle::get_effect_by_handle(context, effect_handle)?;
    match effect_flag_name {
        Some(effect_flag_name) => {
            entry.set(effect_flag_name, effect.full_name());
        }
        None => match effect {
            Effect::ActiveMove(active_move, _) => {
                entry.set("move", &active_move.data.name);
            }
            Effect::MoveCondition(condition) | Effect::InactiveMove(condition) => {
                entry.set("move", &condition.data.name);
            }
            Effect::Ability(ability) | Effect::AbilityCondition(ability) => {
                entry.set("ability", &ability.data.name);
            }
            Effect::Item(item) | Effect::ItemCondition(item) => {
                entry.set("item", &item.data.name);
            }
            Effect::Condition(condition) => {
                entry.set(
                    condition.non_empty_condition_type_name(),
                    &condition.data.name,
                );
            }
            Effect::Clause(clause) => {
                entry.set("clause", &clause.data.name);
            }
            Effect::Species(species) => {
                entry.set("species", &species.data.name);
            }
            Effect::NonExistent(_) => (),
        },
    }
    Ok(())
}

fn effect_activation_internal(
    context: &mut Context,
    header: String,
    activation_context: EffectActivationContext,
) -> Result<UncommittedBattleLogEntry> {
    let mut event = battle_log_entry!(header);

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
        add_effect_to_log_entry(
            context,
            &mut event,
            effect,
            activation_context
                .effect_flag_name
                .as_ref()
                .map(|s| s.as_str()),
        )?;
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
            add_effect_to_log_entry(context, &mut event, &source_effect, Some("from"))?
        }
    }

    if activation_context.target != activation_context.source && !ignore_source {
        if let Some(source) = activation_context.source {
            event.set("of", Mon::position_details(&context.mon_context(source)?)?);
        }
    }

    Ok(event)
}

fn full_mon_details(context: &mut MonContext, header: &str) -> Result<()> {
    let private_event = battle_log_entry!(header, Mon::private_active_details(context)?);
    let public_event = battle_log_entry!(header, Mon::public_active_details(context)?);
    let side = context.mon().side;
    context
        .battle_mut()
        .log_private_public(side, private_event, public_event);
    Ok(())
}

pub fn switch(context: &mut MonContext, is_drag: bool) -> Result<()> {
    let title = if is_drag {
        "drag"
    } else if context.player().player_type.wild() {
        "appear"
    } else {
        "switch"
    };
    full_mon_details(context, title)
}

pub fn switch_out(context: &mut MonContext) -> Result<()> {
    let event = battle_log_entry!("switchout", ("mon", Mon::position_details(context)?));
    context.battle_mut().log(event);
    Ok(())
}

pub fn species_change(context: &mut MonContext) -> Result<()> {
    full_mon_details(context, "specieschange")
}

pub fn replace(context: &mut MonContext) -> Result<()> {
    full_mon_details(context, "replace")
}

fn forme_change_internal(context: &mut ApplyingEffectContext, header: String) -> Result<()> {
    let species = context
        .battle()
        .dex
        .species
        .get_by_id(&context.target().species)?
        .data
        .name
        .clone();
    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("species:{species}")],
        ..Default::default()
    };
    effect_activation(context.as_battle_context_mut(), header, activation)
}

pub fn forme_change(context: &mut ApplyingEffectContext) -> Result<()> {
    forme_change_internal(context, "formechange".to_owned())
}

pub fn mega_evolution(context: &mut ApplyingEffectContext) -> Result<()> {
    forme_change_internal(context, "mega".to_owned())
}
pub fn revert_mega_evolution(context: &mut ApplyingEffectContext) -> Result<()> {
    forme_change_internal(context, "revertmega".to_owned())
}

pub fn cant(
    context: &mut MonContext,
    effect: EffectHandle,
    source: Option<MonHandle>,
) -> Result<()> {
    let activation = EffectActivationContext {
        target: Some(context.mon_handle()),
        source_effect: Some(effect),
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
) -> Result<()> {
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
) -> Result<()> {
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

pub fn fail_heal(context: &mut MonContext) -> Result<()> {
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

pub fn immune(context: &mut MonContext, effect: Option<EffectHandle>) -> Result<()> {
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

fn move_event_on_target(context: &mut MonContext, event: &str) -> Result<()> {
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

pub fn fail_target(context: &mut MonContext) -> Result<()> {
    move_event_on_target(context, "fail")
}

pub fn miss(context: &mut MonContext) -> Result<()> {
    move_event_on_target(context, "miss")
}

pub fn super_effective(context: &mut MonContext) -> Result<()> {
    move_event_on_target(context, "supereffective")
}

pub fn resisted(context: &mut MonContext) -> Result<()> {
    move_event_on_target(context, "resisted")
}

pub fn critical_hit(context: &mut MonContext) -> Result<()> {
    move_event_on_target(context, "crit")
}

pub fn ohko(context: &mut MonContext) -> Result<()> {
    move_event_on_target(context, "ohko")
}

pub fn faint(context: &mut MonContext) -> Result<()> {
    move_event_on_target(context, "faint")
}

pub fn damage(
    context: &mut MonContext,
    effect: Option<EffectHandle>,
    source: Option<MonHandle>,
) -> Result<()> {
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

pub fn heal(context: &mut ApplyingEffectContext) -> Result<()> {
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
) -> Result<()> {
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
) -> Result<()> {
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
) -> Result<()> {
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

pub fn add_volatile(context: &mut ApplyingEffectContext, volatile: &Id) -> Result<()> {
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

pub fn remove_volatile(context: &mut ApplyingEffectContext, volatile: &Id) -> Result<()> {
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

pub fn add_side_condition(context: &mut SideEffectContext, condition: &Id) -> Result<()> {
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

pub fn remove_side_condition(context: &mut SideEffectContext, condition: &Id) -> Result<()> {
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
) -> Result<()> {
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
) -> Result<()> {
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
) -> Result<()> {
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

pub fn transform(context: &mut ApplyingEffectContext, target: MonHandle) -> Result<()> {
    let species = context
        .battle()
        .dex
        .species
        .get_by_id(&context.target().species)?
        .data
        .name
        .clone();
    let into = Mon::position_details(&context.as_battle_context_mut().mon_context(target)?)?;
    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        ignore_active_move_source_effect: true,
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional: vec![format!("into:{into}"), format!("species:{species}")],
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "transform".to_owned(),
        activation,
    )
}

pub fn ability(context: &mut ApplyingEffectContext) -> Result<()> {
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

pub fn ability_end(context: &mut ApplyingEffectContext) -> Result<()> {
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
        "abilityend".to_owned(),
        activation,
    )
}

pub fn item(context: &mut ApplyingEffectContext) -> Result<()> {
    let item = match &context.target().item {
        Some(item) => item.id.clone(),
        None => return Err(general_error("target has no item")),
    };
    let item = context
        .battle()
        .dex
        .items
        .get_by_id(&item)?
        .data
        .name
        .clone();

    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
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
    context: &mut ApplyingEffectContext,
    no_source: bool,
    silent: bool,
    eat: bool,
) -> Result<()> {
    let item = match &context.target().item {
        Some(item) => item.id.clone(),
        None => return Err(general_error("target has no item")),
    };
    let item = context
        .battle()
        .dex
        .items
        .get_by_id(&item)?
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
        target: Some(context.target_handle()),
        source_effect: if no_source {
            None
        } else {
            Some(context.effect_handle().clone())
        },
        source: if no_source {
            None
        } else {
            context.source_handle()
        },
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
    from: Option<&EffectHandle>,
    animate_only: bool,
) -> Result<()> {
    let title = if animate_only { "animatemove" } else { "move" };
    let mut event = battle_log_entry!(
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
    if !animate_only {
        if let Some(from) = from {
            add_effect_to_log_entry(
                context.as_battle_context_mut(),
                &mut event,
                &from,
                Some("from"),
            )?
        }
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

pub fn last_move_spread_targets<I>(context: &mut Context, targets: I) -> Result<()>
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

pub fn hit_count(context: &mut Context, hits: u8) -> Result<()> {
    let event = battle_log_entry!("hitcount", ("hits", hits));
    context.battle_mut().log(event);
    Ok(())
}

pub fn boost(
    context: &mut ApplyingEffectContext,
    boost: Boost,
    delta: i8,
    original_delta: i8,
) -> Result<()> {
    let (delta, message) = if original_delta >= 0 {
        (delta as u8, "boost")
    } else {
        (-delta as u8, "unboost")
    };

    let mut additional = vec![format!("stat:{boost}"), format!("by:{delta}")];

    if original_delta >= 12 {
        additional.push("max".to_owned());
    } else if original_delta <= -12 {
        additional.push("min".to_owned());
    }

    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        ignore_active_move_source_effect: true,
        additional,
        ..Default::default()
    };

    effect_activation(
        context.as_battle_context_mut(),
        message.to_owned(),
        activation,
    )
}

pub fn swap_boosts(context: &mut ApplyingEffectContext, boosts: &[Boost]) -> Result<()> {
    let mut additional = Vec::default();
    if !boosts.is_empty() {
        additional.push(format!("stats:{}", boosts.iter().join(",")));
    }
    let activation = EffectActivationContext {
        target: Some(context.target_handle()),
        source_effect: Some(context.effect_handle().clone()),
        source: context.source_handle(),
        additional,
        ..Default::default()
    };
    effect_activation(
        context.as_battle_context_mut(),
        "swapboosts".to_owned(),
        activation,
    )
}

pub fn debug_event_failure(
    context: &mut Context,
    event: fxlang::BattleEvent,
    effect_name: &str,
    error: &str,
) {
    let log_event = battle_log_entry!(
        "debug",
        ("event", event),
        ("effect", effect_name),
        ("error", error)
    );
    context.battle_mut().log(log_event);
}

pub fn debug_full_event_failure(context: &mut Context, event: fxlang::BattleEvent, error: &str) {
    let log_event = battle_log_entry!("debug", ("event", event), ("error", error));
    context.battle_mut().log(log_event);
}

pub fn experience(context: &mut MonContext, exp: u32) -> Result<()> {
    let event = battle_log_entry!(
        "exp",
        ("mon", Mon::position_details(context)?),
        ("exp", exp)
    );
    context.battle_mut().log(event);
    Ok(())
}

pub fn level_up(context: &mut MonContext) -> Result<()> {
    let event = battle_log_entry!(
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

pub fn cannot_escape(context: &mut PlayerContext) -> Result<()> {
    let event = battle_log_entry!("cannotescape", ("player", &context.player().id));
    context.battle_mut().log(event);
    Ok(())
}

pub fn escaped(context: &mut PlayerContext) -> Result<()> {
    let event = battle_log_entry!("escaped", ("player", &context.player().id));
    context.battle_mut().log(event);
    Ok(())
}

pub fn forfeited(context: &mut PlayerContext) -> Result<()> {
    let event = battle_log_entry!("forfeited", ("player", &context.player().id));
    context.battle_mut().log(event);
    Ok(())
}

pub fn use_item(context: &mut PlayerContext, item: &Id, target: Option<MonHandle>) -> Result<()> {
    let item = context.battle().dex.items.get_by_id(item)?;
    let mut event = battle_log_entry!(
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
) -> Result<()> {
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

pub fn deduct_pp(context: &mut ApplyingEffectContext, move_id: &Id, delta: u8) -> Result<()> {
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

pub fn restore_pp(context: &mut ApplyingEffectContext, move_id: &Id, delta: u8) -> Result<()> {
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

pub fn set_pp(context: &mut ApplyingEffectContext, move_id: &Id, pp: u8) -> Result<()> {
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

pub fn clear_negative_boosts(context: &mut ApplyingEffectContext) -> Result<()> {
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

pub fn uncatchable(context: &mut PlayerContext, target: MonHandle, wild: bool) -> Result<()> {
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
) -> Result<()> {
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
) -> Result<()> {
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
