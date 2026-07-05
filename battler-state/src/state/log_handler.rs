use alloc::{
    borrow::ToOwned,
    collections::BTreeMap,
    format,
    string::String,
    vec::Vec,
};

use anyhow::{
    Context,
    Error,
    Result,
};
use hashbrown::{
    HashMap,
    HashSet,
};

use crate::{
    Ambiguity,
    BattlePhase,
    BattleState,
    ConditionData,
    EffectName,
    Log,
    LogEntry,
    Mon,
    MonBattleAppearanceFromSwitchIn,
    MonBattleAppearanceReference,
    MonBattleAppearanceWithRecovery,
    MonName,
    MonNameList,
    MonPhysicalAppearance,
    Player,
    Side,
    ui,
};

pub(crate) fn alter_battle_state_from_log(
    state: &mut BattleState,
    log: &Log,
    up_to_turn: usize,
) -> Result<()> {
    let last_turn_in_state = state.turn.saturating_sub(1);
    for turn in last_turn_in_state..=up_to_turn {
        alter_battle_state_for_turn(state, log, turn, state.last_log_index)?;
    }
    state.turn = up_to_turn;
    state.last_log_index = log.len().saturating_sub(1);
    Ok(())
}

fn alter_battle_state_for_turn(
    state: &mut BattleState,
    log: &Log,
    turn: usize,
    min_index: usize,
) -> Result<()> {
    state.turn = turn.try_into().context("failed to convert turn number")?;

    let mut ui_log = Vec::default();
    for entry in log.entries_for_turn(turn, Some(min_index)) {
        alter_battle_state_for_entry(state, &mut ui_log, entry)?;
    }

    if turn + 1 > state.ui_log.len() {
        state.ui_log.resize_with(turn + 1, Vec::default);
    }
    // SAFETY: Resized above.
    *state.ui_log.get_mut(turn).unwrap() = ui_log;

    Ok(())
}

fn mon_name_from_log_entry(entry: &LogEntry) -> Result<MonName> {
    let name = entry
        .value("name")
        .ok_or_else(|| Error::msg("missing name"))?;
    let player = entry
        .value("player")
        .ok_or_else(|| Error::msg("missing player"))?;
    let position = entry.value::<usize>("position").map(|position| position);
    Ok(MonName {
        name,
        player,
        position,
    })
}

fn health_from_log_entry(entry: &LogEntry) -> Result<(u64, u64)> {
    entry
        .value_ref("health")
        .map(|health| match health.split_once('/') {
            Some((a, b)) => Ok((a.parse()?, b.parse()?)),
            None => Ok((health.parse()?, 1)),
        })
        .transpose()
        .map(|health| health.unwrap_or((0, 1)))
}

fn mon_appearance_from_log_entry(
    entry: &LogEntry,
) -> Result<(MonPhysicalAppearance, MonBattleAppearanceFromSwitchIn)> {
    let name: String = entry.value("name").unwrap_or_default();
    let species: String = entry.value("species").unwrap_or_default();
    let level: u64 = entry.value("level").unwrap_or_default();
    let gender = entry.value("gender").unwrap_or_default();
    let shiny = entry.value_ref("shiny").is_some();
    let health = health_from_log_entry(entry)?;
    let status: String = entry.value("status").unwrap_or_default();
    let terastallization: String = entry.value("tera").unwrap_or_default();
    Ok((
        MonPhysicalAppearance {
            name,
            species,
            gender,
            shiny,
        },
        MonBattleAppearanceFromSwitchIn {
            level,
            health,
            status,
            terastallization,
        },
    ))
}

fn mon_name_to_mon_for_ui_log(state: &mut BattleState, mon: &MonName) -> Result<ui::Mon> {
    match &mon.position {
        Some(position) => {
            let side = state.field.side_for_player(&mon.player)?;
            Ok(ui::Mon::Active(ui::FieldPosition {
                side,
                position: *position - 1,
            }))
        }
        None => Ok(ui::Mon::Inactive(ui::MonReference {
            player: mon.player.clone(),
            name: mon.name.clone(),
        })),
    }
}

fn effect_from_log_entry(entry: &LogEntry, effect_value_name: Option<&str>) -> Result<EffectName> {
    match effect_value_name {
        Some(name) => entry.value_or_else(name),
        None => {
            let check_effect_name = |entry: &LogEntry, name: &str| {
                entry.value::<String>(name).map(|value| EffectName {
                    effect_type: Some(name.to_owned()),
                    name: value,
                })
            };
            check_effect_name(entry, "move")
                .or_else(|| check_effect_name(entry, "ability"))
                .or_else(|| check_effect_name(entry, "item"))
                .or_else(|| check_effect_name(entry, "condition"))
                .or_else(|| check_effect_name(entry, "volatile"))
                .or_else(|| check_effect_name(entry, "status"))
                .or_else(|| check_effect_name(entry, "type"))
                .or_else(|| check_effect_name(entry, "weather"))
                .or_else(|| check_effect_name(entry, "clause"))
                .or_else(|| check_effect_name(entry, "species"))
                .ok_or_else(|| Error::msg("missing effect"))
        }
    }
}

fn effect_data_from_log_entry(state: &mut BattleState, entry: &LogEntry) -> Result<ui::EffectData> {
    let effect = effect_from_log_entry(entry, None).ok();
    let side = entry.value("side");
    let slot = entry.value("slot");
    let player = entry.value("player");
    let target = entry
        .value::<MonName>("mon")
        .map(|mon| mon_name_to_mon_for_ui_log(state, &mon))
        .transpose()?;
    let source = entry
        .value::<MonName>("of")
        .map(|mon| mon_name_to_mon_for_ui_log(state, &mon))
        .transpose()?;
    let source_effect = effect_from_log_entry(entry, Some("from")).ok();

    // Additional data that may be useful to the user interface for specific effects.
    let effect_type = effect
        .as_ref()
        .map(|effect| effect.effect_type.clone())
        .flatten();
    let additional = entry
        .values()
        .filter(|(key, _)| match *key {
            "from" | "mon" | "of" | "player" | "side" | "slot" => false,
            key => effect_type
                .as_ref()
                .is_none_or(|effect_type| key != effect_type),
        })
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();

    Ok(ui::EffectData {
        effect: effect.map(|effect| effect.into()),
        side,
        slot,
        player,
        target,
        source,
        source_effect: source_effect.map(|effect| effect.into()),
        additional,
    })
}

fn mons_by_mon_name(
    state: &mut BattleState,
    mon: &MonName,
) -> Result<Vec<MonBattleAppearanceReference>> {
    match mon.position {
        Some(position) => {
            let side = state.field.side_for_player(&mon.player)?;
            let index = position
                .checked_sub(1)
                .ok_or_else(|| Error::msg("position must be greater than 0"))?;
            Ok(state
                .field
                .active_mon_reference_by_position(side, index)?
                .map(|mon| Vec::from_iter([mon]))
                .unwrap_or_default())
        }
        None => state
            .field
            .mons_by_name(&mon.player, &mon.name, Some(false)),
    }
}

fn mons_by_mon_name_require_one(
    state: &mut BattleState,
    mon: &MonName,
) -> Result<MonBattleAppearanceReference> {
    mons_by_mon_name(state, mon).and_then(|mut mons| {
        if mons.is_empty() {
            Err(Error::msg("no mons found"))
        } else if mons.len() != 1 {
            Err(Error::msg("more than one mon found"))
        } else {
            // SAFETY: mons has exactly 1 element.
            Ok(mons.pop().unwrap())
        }
    })
}

fn apply_for_each_mon_reference<F>(state: &mut BattleState, mon: &MonName, f: F) -> Result<()>
where
    F: Fn(&mut BattleState, MonBattleAppearanceReference, Ambiguity) -> Result<()>,
{
    let mons = mons_by_mon_name(state, mon)?;

    let ambiguity = if mons.len() == 1 {
        Ambiguity::Precise
    } else {
        Ambiguity::Ambiguous
    };

    for mon in mons {
        f(state, mon, ambiguity)?;
    }

    Ok(())
}

fn apply_for_each_mon_battle_appearance<F>(
    state: &mut BattleState,
    mon: &MonName,
    f: F,
) -> Result<()>
where
    F: Fn(&mut MonBattleAppearanceWithRecovery, Ambiguity),
{
    apply_for_each_mon_reference(state, mon, |state, mon, ambiguity| {
        let mon = state
            .field
            .mon_battle_appearance_with_recovery_mut_by_reference_or_else(&mon)?;
        f(mon, ambiguity);
        Ok(())
    })
}

fn apply_for_each_mon<F>(state: &mut BattleState, mon: &MonName, f: F) -> Result<()>
where
    F: Fn(&mut Mon, Ambiguity),
{
    apply_for_each_mon_reference(state, mon, |state, mon, ambiguity| {
        let mon = state.field.mon_mut_by_reference_or_else(&mon)?;
        f(mon, ambiguity);
        Ok(())
    })
}

fn record_activated_ability_for_each_mon(
    state: &mut BattleState,
    mon: &MonName,
    ability: String,
) -> Result<()> {
    apply_for_each_mon_reference(state, mon, |state, reference, ambiguity| {
        // If we have a volatile ability, then any ability we see is also volatile.
        //
        // Otherwise, an ability activation reveals the Mon's true ability.
        let mon = state.field.mon_mut_by_reference_or_else(&reference)?;
        if let Some(volatile_ability) = &mut mon.volatile_data.ability {
            *volatile_ability = ability.clone();
        } else {
            let mon_battle_appearance = state
                .field
                .mon_battle_appearance_with_recovery_mut_by_reference_or_else(&reference)?;
            mon_battle_appearance.record_ability(ability.clone().into(), ambiguity);
        }
        Ok(())
    })
}

fn record_effect_from_mon(
    state: &mut BattleState,
    effect: &ui::Effect,
    mon: &MonName,
) -> Result<()> {
    match effect.effect_type.as_ref().map(|s| s.as_str()) {
        Some("ability") => {
            record_activated_ability_for_each_mon(state, &mon, effect.name.clone())?;
        }
        Some("item") => {
            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                // If we know that the Mon does not have an item, then this effect is presumably
                // after the item ended.
                if let Some(item) = mon.primary().item.known()
                    && item.is_empty()
                {
                    return;
                }

                mon.record_item(effect.name.clone().into(), ambiguity);
            })?;
        }
        _ => (),
    }
    Ok(())
}

fn modify_state_from_effect(
    state: &mut BattleState,
    entry: &LogEntry,
    effect_data: &ui::EffectData,
) -> Result<()> {
    if let Some(source_effect) = &effect_data.source_effect {
        if let Some(source) = entry.value::<MonName>("of") {
            record_effect_from_mon(state, source_effect, &source)?;
        } else if let Some(target) = entry.value::<MonName>("mon") {
            record_effect_from_mon(state, source_effect, &target)?;
        }
    }

    match entry.title() {
        "ability" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect_data.effect {
                record_activated_ability_for_each_mon(state, &mon, effect.name.clone())?;
            }
        }
        "abilityend" => {
            let mon = entry.value_or_else("mon")?;

            // We get to see the ability as it ends.
            if let Some(effect) = &effect_data.effect {
                record_activated_ability_for_each_mon(state, &mon, effect.name.clone())?;
            }

            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.record_ability(String::default());
            })?;
        }
        "activate" => match (&effect_data.effect, entry.value::<MonName>("mon")) {
            (Some(effect), Some(mon)) => record_effect_from_mon(state, effect, &mon)?,
            _ => (),
        },
        "catch" | "faint" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon(state, &mon, |mon, _| {
                mon.faint();
            })?;

            let side = state.field.side_for_player(&mon.player)?;
            apply_for_each_mon_reference(state, &mon, |state, mon, _| {
                state.field.side_mut_or_else(side)?.switch_out(&mon, false)
            })?;
        }
        "clearboosts" | "clearallboosts" => {
            if entry.title() == "clearallboosts" {
                for mon in state.field.active_mons().collect::<Vec<_>>() {
                    let mon = state.field.mon_mut_by_reference_or_else(&mon)?;
                    mon.volatile_data.stat_boosts.clear();
                }
            } else {
                let mon = entry.value_or_else("mon")?;
                apply_for_each_mon(state, &mon, |mon, _| {
                    mon.volatile_data.stat_boosts.clear();
                })?;
            }
        }
        "clearnegativeboosts" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon(state, &mon, |mon, _| {
                for stat in mon
                    .volatile_data
                    .stat_boosts
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                {
                    if let alloc::collections::btree_map::Entry::Occupied(entry) =
                        mon.volatile_data.stat_boosts.entry(stat)
                        && *entry.get() < 0
                    {
                        entry.remove_entry();
                    }
                }
            })?;
        }
        "clearpositiveboosts" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon(state, &mon, |mon, _| {
                for stat in mon
                    .volatile_data
                    .stat_boosts
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                {
                    if let alloc::collections::btree_map::Entry::Occupied(entry) =
                        mon.volatile_data.stat_boosts.entry(stat)
                        && *entry.get() > 0
                    {
                        entry.remove_entry();
                    }
                }
            })?;
        }
        "invertboosts" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon(state, &mon, |mon, _| {
                for value in mon.volatile_data.stat_boosts.values_mut() {
                    *value = -*value;
                }
            })?;
        }
        "clearweather" => {
            state.field.weather = None;
        }
        "copyboosts" => {
            let mon = entry.value_or_else("mon")?;
            let source = entry.value_or_else("of")?;
            let source = mons_by_mon_name_require_one(state, &source)?;
            let boosts = state
                .field
                .mon_by_reference_or_else(&source)?
                .volatile_data
                .stat_boosts
                .clone();

            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.stat_boosts = boosts.clone();
            })?;
        }
        "curestatus" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_status(String::default().into(), ambiguity);
            })?;
        }
        "damage" | "heal" | "sethp" => {
            let health = health_from_log_entry(&entry)?;
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_health(health.into(), ambiguity);
            })?;
        }
        "revive" => {
            let health = health_from_log_entry(&entry)?;
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon(state, &mon, |mon, _| {
                mon.revive();
            })?;
            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_health(health.into(), ambiguity);
            })?;
        }
        "dynamax" => {
            let mon = entry.value_or_else("mon")?;
            let turn = state.turn;
            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.record_condition(
                    "Dynamax".to_owned(),
                    ConditionData {
                        since_turn: turn,
                        data: effect_data.additional.clone(),
                    },
                );
            })?;
        }
        "end" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect_data.effect {
                apply_for_each_mon(state, &mon, |mon, _| {
                    mon.volatile_data.remove_condition(&effect.name);
                })?;

                record_effect_from_mon(state, &effect, &mon)?;
            }
        }
        "fieldend" => {
            if let Some(effect) = &effect_data.effect {
                state.field.conditions.remove(&effect.name);
            }
        }
        "fieldstart" => {
            if let Some(effect) = &effect_data.effect {
                state.field.conditions.insert(
                    effect.name.clone(),
                    ConditionData {
                        since_turn: state.turn,
                        data: effect_data.additional.clone(),
                    },
                );
            }
        }
        "formechange" | "gigantamax" | "revertgigantamax" | "mega" | "revertmega" | "primal"
        | "revertprimal" | "ultra" | "revertultra" => {
            let mon = entry.value_or_else("mon")?;
            let species: String = entry.value_or_else("species")?;
            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.record_forme_change(species.clone());
            })?;
        }
        "item" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect_data.effect {
                apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                    mon.record_item(effect.name.clone().into(), ambiguity);
                })?;
            }
        }
        "itemend" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_item(String::default().into(), ambiguity);
            })?;
        }
        "prepare" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect_data.effect {
                let turn = state.turn;
                apply_for_each_mon(state, &mon, |mon, _| {
                    mon.volatile_data.record_condition(
                        effect.name.clone(),
                        ConditionData {
                            since_turn: turn,
                            data: effect_data.additional.clone(),
                        },
                    );
                })?;
            }
        }
        "revertdynamax" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.remove_condition("Dynamax");
            })?;
        }
        "reverttera" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_terastallization(String::default().into(), ambiguity);
            })?;
        }
        "addedtype" => {
            let mon = entry.value_or_else("mon")?;
            let typ: String = entry.value_or_else("type")?;
            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.added_type = Some(typ.clone());
            })?;
        }
        "resettypechange" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.types.clear();
                mon.volatile_data.added_type = None;
            })?;
        }
        "sideend" => {
            let side = entry.value_or_else("side")?;
            let side = state.field.side_mut_or_else(side)?;
            if let Some(effect) = &effect_data.effect {
                side.conditions.remove(&effect.name);
            }
        }
        "sidestart" => {
            let side = entry.value_or_else("side")?;
            let side = state.field.side_mut_or_else(side)?;
            if let Some(effect) = &effect_data.effect {
                side.conditions.insert(
                    effect.name.clone(),
                    ConditionData {
                        since_turn: state.turn,
                        data: effect_data.additional.clone(),
                    },
                );
            }
        }
        "singlemove" | "singleturn" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect_data.effect {
                let turn = state.turn;
                let mut data = effect_data.additional.clone();
                data.insert(entry.title().to_owned(), "".to_owned());
                apply_for_each_mon(state, &mon, |mon, _| {
                    mon.volatile_data.record_condition(
                        effect.name.clone(),
                        ConditionData {
                            since_turn: turn,
                            data: data.clone(),
                        },
                    );
                })?;
            }
        }
        "specieschange" => {
            let (physical_appearance, _) = mon_appearance_from_log_entry(entry)?;
            let mon = mon_name_from_log_entry(entry)?;
            apply_for_each_mon(state, &mon, |mon, ambiguity| {
                mon.physical_appearance.species = match ambiguity {
                    Ambiguity::Ambiguous => String::default(),
                    Ambiguity::Precise => physical_appearance.species.clone(),
                }
            })?
        }
        "status" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect_data.effect {
                apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                    mon.record_status(effect.name.clone().into(), ambiguity);
                })?;
            }
        }
        "start" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect_data.effect {
                let turn = state.turn;
                apply_for_each_mon(state, &mon, |mon, _| {
                    mon.volatile_data.record_condition(
                        effect.name.clone(),
                        ConditionData {
                            since_turn: turn,
                            data: effect_data.additional.clone(),
                        },
                    );
                })?;

                record_effect_from_mon(state, &effect, &mon)?;
            }
        }
        "swapboosts" => {
            let mon_name: MonName = entry.value_or_else("mon")?;
            let mon = mons_by_mon_name_require_one(state, &mon_name)?;
            let source_name = entry.value_or_else("of")?;
            let source = mons_by_mon_name_require_one(state, &source_name)?;
            let stats = entry
                .value::<String>("stats")
                .map(|stats| stats.split(',').map(|s| s.to_owned()).collect::<Vec<_>>());
            let mon_boosts = state
                .field
                .mon_by_reference_or_else(&mon)?
                .volatile_data
                .stat_boosts
                .clone();
            let source_boosts = state
                .field
                .mon_by_reference_or_else(&source)?
                .volatile_data
                .stat_boosts
                .clone();

            let mut swap_boosts = |name: &MonName, boosts: BTreeMap<String, i64>| -> Result<()> {
                apply_for_each_mon(state, &name, |mon, _| match &stats {
                    Some(stats) => {
                        for stat in stats {
                            mon.volatile_data.stat_boosts.insert(
                                stat.clone(),
                                boosts.get(stat).cloned().unwrap_or_default(),
                            );
                        }
                    }
                    None => mon.volatile_data.stat_boosts = boosts.clone(),
                })
            };

            swap_boosts(&mon_name, source_boosts)?;
            swap_boosts(&source_name, mon_boosts)?;
        }
        "tera" => {
            let mon = entry.value_or_else("mon")?;
            let typ: String = entry.value_or_else("type")?;
            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_terastallization(typ.clone().into(), ambiguity);
            })?;
        }
        "transform" => {
            let mon = entry.value_or_else("mon")?;
            let species = entry.value_or_else("species")?;
            let into = entry.value_or_else("into")?;
            let target_reference = mons_by_mon_name_require_one(state, &into)
                .context("transform attempted to transform into an ambiguous target")?;

            let target = state.field.mon_by_reference_or_else(&target_reference)?;

            let target_volatile = &target.volatile_data;
            let mut target_appearance = target.physical_appearance.clone();
            target_appearance.species = species;

            let target_battle_appearance = state
                .field
                .mon_battle_appearance_with_recovery_by_reference_or_else(&target_reference)?;

            // Copy over volatile data that we know is transformed.
            let ability = target_volatile
                .ability
                .as_ref()
                .or_else(|| target_battle_appearance.primary().ability.known())
                .cloned();
            let types = if target_volatile.types != Vec::from_iter(["None"]) {
                Some(target_volatile.types.clone())
            } else {
                None
            };
            let stat_boosts = target_volatile.stat_boosts.clone();

            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data
                    .record_transformation(target_appearance.clone(), target_reference.clone());

                if let Some(ability) = &ability {
                    mon.volatile_data.record_ability(ability.clone());
                }
                if let Some(types) = &types {
                    mon.volatile_data.types = types.clone();
                }
                mon.volatile_data.added_type = None;
                mon.volatile_data.stat_boosts = stat_boosts.clone();
            })?;
        }
        "typechange" => {
            let mon = entry.value_or_else("mon")?;
            let types: String = entry.value_or_else("types")?;
            let types = types.split('/').map(|s| s.to_owned()).collect::<Vec<_>>();
            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.record_types(types.clone());
                mon.volatile_data.added_type = None;
            })?;
        }
        "weather" => {
            if let Some(effect) = &effect_data.effect {
                state.field.weather = Some(effect.name.clone());
            }
        }
        _ => (),
    }
    Ok(())
}

fn alter_battle_state_for_entry(
    state: &mut BattleState,
    ui_log: &mut Vec<ui::UiLogEntry>,
    entry: &LogEntry,
) -> Result<()> {
    let title = entry.title().strip_prefix("-").unwrap_or(entry.title());
    match title {
        "ability"
        | "abilityend"
        | "activate"
        | "addedtype"
        | "block"
        | "cant"
        | "catch"
        | "catchfailed"
        | "clearboosts"
        | "clearallboosts"
        | "clearnegativeboosts"
        | "clearpositiveboosts"
        | "clearweather"
        | "copyboosts"
        | "curestatus"
        | "crit"
        | "damage"
        | "deductpp"
        | "dynamax"
        | "end"
        | "fail"
        | "faint"
        | "fieldactivate"
        | "fieldend"
        | "fieldstart"
        | "formechange"
        | "gigantamax"
        | "heal"
        | "hitcount"
        | "immune"
        | "invertboosts"
        | "item"
        | "itemend"
        | "mega"
        | "miss"
        | "ohko"
        | "prepare"
        | "primal"
        | "protectweaken"
        | "resettypechange"
        | "resisted"
        | "restorepp"
        | "revertdynamax"
        | "revertgigantamax"
        | "revertmega"
        | "revertprimal"
        | "reverttera"
        | "revertultra"
        | "revive"
        | "sethp"
        | "setpp"
        | "sidestart"
        | "sideend"
        | "singlemove"
        | "singleturn"
        | "specieschange"
        | "status"
        | "start"
        | "supereffective"
        | "swapboosts"
        | "tera"
        | "transform"
        | "typechange"
        | "ultra"
        | "uncatchable"
        | "weather" => {
            let effect = effect_data_from_log_entry(state, entry)?;
            modify_state_from_effect(state, entry, &effect)?;

            // Generate UI log for the effect. Some effects may have special logs.
            match entry.title() {
                "catch" => {
                    ui_log.push(ui::UiLogEntry::Caught { effect });
                }
                "damage" | "heal" | "sethp" => {
                    let health = health_from_log_entry(entry)?;
                    ui_log.push(match entry.title() {
                        "damage" => ui::UiLogEntry::Damage { health, effect },
                        "heal" => ui::UiLogEntry::Heal { health, effect },
                        "sethp" => ui::UiLogEntry::SetHealth { health, effect },
                        _ => unreachable!(),
                    });
                }
                "faint" => {
                    ui_log.push(ui::UiLogEntry::Faint { effect });
                }
                "formechange" | "gigantamax" | "mega" | "revertgigantamax" | "revertmega"
                | "specieschange" | "transform" | "primal" | "revertprimal" | "ultra"
                | "revertultra" => {
                    let species = entry.value_or_else("species")?;
                    ui_log.push(ui::UiLogEntry::UpdateAppearance {
                        title: entry.title().to_owned(),
                        species,
                        effect,
                    });
                }
                "revive" => {
                    ui_log.push(ui::UiLogEntry::Revive { effect });
                }
                _ => {
                    ui_log.push(ui::UiLogEntry::Effect {
                        title: entry.title().to_owned(),
                        effect,
                    });
                }
            }
        }
        "battlestart" => {
            state.phase = BattlePhase::Battle;
        }
        "boost" | "unboost" => {
            let mon: MonName = entry.value_or_else("mon")?;

            let stat: String = entry.value_or_else("stat")?;
            let by: i64 = entry.value_or_else("by")?;
            let by = if entry.title() == "unboost" { -by } else { by };

            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.record_stat_boost(stat.clone(), by);
            })?;

            ui_log.push(ui::UiLogEntry::StatBoost {
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                stat,
                by,
            });
        }
        "cannotescape" => {
            let player = entry.value_or_else("player")?;
            ui_log.push(ui::UiLogEntry::CannotEscape { player });
        }
        "continue" => (),
        "debug" | "fxlang_debug" => ui_log.push(ui::UiLogEntry::Debug {
            title: entry.title().to_owned(),
            values: entry
                .values()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
        }),
        "didnotlearnmove" => {
            let mon = entry.value_or_else("mon")?;
            let move_name = entry.value_or_else("move")?;
            ui_log.push(ui::UiLogEntry::MoveUpdate {
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                move_name,
                learned: false,
                forgot: None,
            });
        }
        "escaped" | "forfeited" => {
            let player: String = entry.value_or_else("player")?;
            let side_index = state.field.side_for_player(&player)?;

            state.field.player_mut_or_else(&player)?.left_battle = true;

            // All Mons belonging to the player leave immediately.
            let active_mons = state
                .field
                .active_mons_on_side(side_index)
                .filter(|(_, reference)| reference.player == player)
                .collect::<Vec<_>>();

            let side = state.field.side_mut_or_else(side_index)?;
            for (_, mon) in &active_mons {
                side.switch_out(&mon, true)?;
            }

            ui_log.push(ui::UiLogEntry::Leave {
                title: entry.title().to_owned(),
                player: player.clone(),
                positions: active_mons
                    .into_iter()
                    .map(|(i, _)| ui::FieldPosition {
                        side: side_index,
                        position: i,
                    })
                    .collect(),
            });
        }
        "exp" => {
            let mon = entry.value_or_else("mon")?;
            let exp = entry.value_or_else("exp")?;
            ui_log.push(ui::UiLogEntry::Experience {
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                exp,
            })
        }
        "info" => {
            if let Some(battle_type) = entry.value::<String>("battletype") {
                state.battle_type = battle_type.to_lowercase();
                state.field.max_side_length = match state.battle_type.as_str() {
                    "doubles" => 2,
                    "triples" => 3,
                    _ => 1,
                };
            }
            if let Some(rule) = entry.value::<String>("rule") {
                state.field.rules.push(rule.to_owned());
            }
            if let Some(environment) = entry.value::<String>("environment") {
                state.field.environment = Some(environment);
            }
            if let Some(time) = entry.value::<String>("time") {
                state.field.time = Some(time);
            }
        }
        "learnedmove" => {
            let mon = entry.value_or_else("mon")?;
            let move_name: String = entry.value_or_else("move")?;
            let forgot = entry.value::<String>("forgot");

            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_move(move_name.clone(), ambiguity);

                if let Some(forgot) = &forgot {
                    mon.forget_move(forgot.clone(), ambiguity);
                }
            })?;

            ui_log.push(ui::UiLogEntry::MoveUpdate {
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                move_name,
                learned: true,
                forgot,
            });
        }
        "levelup" => {
            let mon = entry.value_or_else("mon")?;
            let level: u64 = entry.value_or_else("level")?;

            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_level(level.into(), ambiguity);
            })?;

            let mut stats = HashMap::default();

            let mut add_stat_to_map_if_present = |name: &str| {
                if let Some(stat) = entry.value(name) {
                    stats.insert(name.to_owned(), stat);
                }
            };
            add_stat_to_map_if_present("hp");
            add_stat_to_map_if_present("atk");
            add_stat_to_map_if_present("def");
            add_stat_to_map_if_present("spa");
            add_stat_to_map_if_present("spd");
            add_stat_to_map_if_present("spe");

            ui_log.push(ui::UiLogEntry::LevelUp {
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                level,
                stats,
            });
        }
        "maxsidelength" => {
            state.field.max_side_length = entry.value_or_else("length")?;
        }
        "mon" => {
            let (physical_appearance, battle_appearance) = mon_appearance_from_log_entry(entry)?;
            let player: String = entry.value_or_else("player")?;
            let player = state.field.player_mut_or_else(&player)?;
            player
                .mons
                .push(Mon::new(physical_appearance, [(&battle_appearance).into()]));
        }
        "move" | "animatemove" => {
            let mon: MonName = entry.value_or_else("mon")?;
            let name: String = entry.value_or_else("name")?;
            let used_directly = entry.title() == "move";
            let target: Option<MonName> = entry.value("target");
            let spread: Option<MonNameList> = entry.value("spread");
            let from: Option<EffectName> = entry.value("from");
            let animate = entry.value_ref("noanim").is_none();
            let animate_only = entry.title() == "animatemove";

            if used_directly && from.is_none() && name != "Struggle" {
                apply_for_each_mon_reference(state, &mon, |state, mon_reference, ambiguity| {
                    let mon = state.field.mon_mut_by_reference_or_else(&mon_reference)?;

                    if mon.volatile_data.conditions.contains_key(&name)
                        || mon.volatile_data.moves.contains(&name)
                    {
                        return Ok(());
                    }

                    if let Some(condition) = mon.volatile_data.conditions.get("Mimic")
                        && let Some(mimic) = condition.data.get("mimic")
                        && mimic == &name
                    {
                        return Ok(());
                    }

                    if let Some((_, transformation_battle_appearance)) =
                        mon.volatile_data.transformed.clone()
                    {
                        let mon = state.field.mon_mut_by_reference_or_else(&mon_reference)?;
                        mon.volatile_data.record_move(name.clone());

                        // Record the move on the transformation source.
                        if let Ok(mon) = state
                            .field
                            .mon_battle_appearance_with_recovery_mut_by_reference_or_else(
                                &transformation_battle_appearance,
                            )
                        {
                            mon.record_move(name.clone(), ambiguity);
                        }
                    } else {
                        let mon = state
                            .field
                            .mon_battle_appearance_with_recovery_mut_by_reference_or_else(
                                &mon_reference,
                            )?;
                        mon.record_move(name.clone(), ambiguity);
                    }

                    // If we get here, record the move in history.
                    state
                        .field
                        .mon_battle_appearance_with_recovery_mut_by_reference_or_else(
                            &mon_reference,
                        )?
                        .record_used_move(name.clone());

                    Ok(())
                })?;
            }

            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.remove_condition(&name);

                for name in mon
                    .volatile_data
                    .conditions
                    .iter()
                    .filter(|(_, condition)| condition.data.contains_key("singlemove"))
                    .map(|(name, _)| name)
                    .cloned()
                    .collect::<Vec<_>>()
                {
                    mon.volatile_data.remove_condition(&name);
                }
            })?;

            ui_log.push(ui::UiLogEntry::Move {
                name,
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                target: if let Some(spread) = spread {
                    Some(ui::MoveTarget::Spread(
                        spread
                            .0
                            .into_iter()
                            .map(|mon| mon_name_to_mon_for_ui_log(state, &mon))
                            .collect::<Result<HashSet<_>>>()?,
                    ))
                } else if let Some(mon) = target {
                    Some(ui::MoveTarget::Single(mon_name_to_mon_for_ui_log(
                        state, &mon,
                    )?))
                } else {
                    None
                },
                animate,
                animate_only,
            })
        }
        "player" => {
            let id: String = entry.value_or_else("id")?;
            let name = entry.value_or_else("name")?;
            let side: usize = entry.value_or_else("side")?;
            let position = entry.value_or_else::<usize>("position")?;
            let side = state.field.side_mut_or_else(side)?;
            side.players.insert(
                id.clone(),
                Player {
                    name,
                    id,
                    position,
                    ..Default::default()
                },
            );
        }
        "residual" => (),
        "side" => {
            let id: usize = entry.value_or_else("id")?;
            let name = entry.value_or_else("name")?;
            if id + 1 > state.field.sides.len() {
                state.field.sides.resize_with(id + 1, Side::default);
            }
            // SAFETY: Resized above.
            let side = state.field.side_mut_or_else(id).unwrap();
            side.id = id;
            side.name = name;
        }
        "switch" | "drag" | "appear" | "replace" => {
            let (physical_appearance, battle_appearance) = mon_appearance_from_log_entry(entry)?;
            let player: String = entry.value_or_else("player")?;
            let position = entry
                .value_or_else::<usize>("position")?
                .checked_sub(1)
                .ok_or_else(|| Error::msg("position must be greater than 0"))?;

            let side_index = state.field.side_for_player(&player)?;
            let side = state.field.side_mut_or_else(side_index)?;

            if position + 1 > side.active.len() {
                side.active.resize_with(position + 1, Option::default);
            }

            // SAFETY: Resized above.
            let previous = side.active.get_mut(position).cloned().unwrap();

            let replace = entry.title() == "replace";
            let mut current_appearance = None;

            // First, handle illusion recovery.
            if let Some(previous) = &previous {
                // If applicable, handle illusion recovery first.

                if replace {
                    // When an illusion breaks, we recover the old Mon before switching it out, and
                    // move the current appearance to the replacement Mon.
                    current_appearance = Some(
                        side.mon_battle_appearance_with_recovery_mut_by_reference_or_else(
                            &previous,
                        )?
                        .recover(),
                    );
                }

                // Mark that the previous Mon is inactive if we are replacing an illusion, or if the
                // Mon fainted.
                //
                // Ordinarily, we want the previous Mon to still be considered active when switching
                // in the new Mon, so that it is clear that the new and previous Mons are distinct.
                // However, in the case of illusion replacement, the Mon that the illusion took the
                // appearance of was *never* active, so we want that Mon to be a candidate for
                // merging.
                //
                // The same reasoning applies if the previous Mon fainted; if this Mon is the same
                // as the fainted one, then it should not really be fainted (handled later) and we
                // should be able to use the Mon we thought fainted.
                //
                // There is technically an edge case here: if an illusion user creates an illusion
                // of a Mon that looks exactly identical (by physical appearance) to it, then when
                // the illusion breaks, the active Mon will not really change. However, this case is
                // acceptable because there is ambiguity *anyway*. To avoid this edge case, we would
                // need to track switching for non-overlap with a separate field somewhere.
                side.switch_out(
                    &previous,
                    replace || side.mon_by_reference_or_else(previous)?.fainted,
                )?;

                // If the replaced Mon ends up empty, we can remove that battle appearance.
                let previous_mon_appearance =
                    side.mon_battle_appearance_with_recovery_mut_by_reference_or_else(&previous)?;
                if previous_mon_appearance.primary().is_empty() {
                    side.mon_mut_by_reference_or_else(&previous)?
                        .remove_battle_appearance(previous.battle_appearance_index);
                }
            }

            // Third, switch in the new Mon.
            //
            // This may result in some merging.
            let mon = side.switch_in(&player, &physical_appearance, &battle_appearance, replace)?;
            let mon_index = mon.mon_index;

            // Fourth, merge in the current appearance from prior to the illusion breaking, if
            // applicable.
            if let Some(current_appearance) = current_appearance {
                side.mon_battle_appearance_with_recovery_mut_by_reference_or_else(&mon)?
                    .record_all(current_appearance);
            }

            // Finally, set the active position to the new Mon.
            //
            // SAFETY: Resized above.
            *side.active.get_mut(position).unwrap() = Some(mon.clone());

            ui_log.push(ui::UiLogEntry::Switch {
                title: entry.title().to_owned(),
                player,
                mon: mon_index,
                into_position: ui::FieldPosition {
                    side: side_index,
                    position,
                },
            });
        }
        "switchout" => {
            // The switch out log is purely visual.
            let mon = entry.value_or_else("mon")?;
            ui_log.push(ui::UiLogEntry::SwitchOut {
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
            });
        }
        "teampreviewstart" => {
            state.phase = BattlePhase::PreTeamPreview;
        }
        "teampreview" => {
            let pick = entry.value_or_else("pick")?;
            state.phase = BattlePhase::TeamPreview(pick);
        }
        "teamsize" => {
            let player: String = entry.value_or_else("player")?;
            let size = entry.value_or_else("size")?;
            let player = state.field.player_mut_or_else(&player)?;
            player.team_size = size;

            // TODO: We could try to remember Mons from team preview and match them up as they
            // appear.
            player.mons.clear();
        }
        "tie" => {
            state.phase = BattlePhase::Finished;
            ui_log.push(ui::UiLogEntry::Tie);
        }
        "time" => (),
        "turn" => (),
        "turnlimit" => {
            ui_log.push(ui::UiLogEntry::TurnLimit);
        }
        "useitem" => {
            let player = entry.value_or_else("player")?;
            let item = entry.value_or_else("name")?;
            let target = entry.value("target");
            ui_log.push(ui::UiLogEntry::UseItem {
                player,
                item,
                target: target
                    .map(|target| mon_name_to_mon_for_ui_log(state, &target))
                    .transpose()?,
            });
        }
        "win" => {
            state.phase = BattlePhase::Finished;
            let side = entry.value_or_else("side")?;
            state.winning_side = Some(side);
            ui_log.push(ui::UiLogEntry::Win { side });
        }
        "catchrate" => {
            ui_log.push(ui::UiLogEntry::Debug {
                title: entry.title().to_owned(),
                values: entry
                    .values()
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect(),
            });
        }
        "swap" => {
            let mon_name = entry.value_or_else::<MonName>("mon")?;
            let side_index = state.field.side_for_player(&mon_name.player)?;
            let side = state.field.side_mut_or_else(side_index)?;
            let pos_new: usize = entry.value_or_else::<usize>("position")? - 1;
            let pos_old = side.active.iter().position(|active_mon| {
                active_mon.as_ref().is_some_and(|ref_mon| {
                    if let Ok(m) = side.mon_by_reference_or_else(ref_mon) {
                        m.physical_appearance.name == mon_name.name
                    } else {
                        false
                    }
                })
            });
            if let Some(pos_old) = pos_old {
                if pos_new < side.active.len() {
                    side.active.swap(pos_old, pos_new);
                }
            }
        }
        "swapplayer" => {
            let player_id: String = entry.value_or_else("player")?;
            let position: usize = entry.value_or_else("position")?;
            let side_index = state.field.side_for_player(&player_id)?;
            let side = state.field.side_mut_or_else(side_index)?;
            if let Some(player) = side.players.get_mut(&player_id) {
                player.position = position;
            }
        }
        "swapsideconditions" => {
            let side_idx: usize = entry.value_or_else("side")?;
            let with_idx: usize = entry.value_or_else("with")?;
            if side_idx < state.field.sides.len() && with_idx < state.field.sides.len() {
                let cond1 = state.field.sides[side_idx].conditions.clone();
                let cond2 = state.field.sides[with_idx].conditions.clone();
                state.field.sides[side_idx].conditions = cond2;
                state.field.sides[with_idx].conditions = cond1;
            }
        }
        "swapsidecondition" => {
            let side_idx: usize = entry.value_or_else("side")?;
            let source_idx: usize = entry.value_or_else("source")?;
            let condition: String = entry.value_or_else("condition")?;
            if side_idx < state.field.sides.len() && source_idx < state.field.sides.len() {
                let cond_source = state.field.sides[source_idx].conditions.remove(&condition);
                let cond_target = state.field.sides[side_idx].conditions.remove(&condition);
                if let Some(c) = cond_source {
                    state.field.sides[side_idx]
                        .conditions
                        .insert(condition.clone(), c);
                }
                if let Some(c) = cond_target {
                    state.field.sides[source_idx]
                        .conditions
                        .insert(condition, c);
                }
            }
        }
        "waiting" => {
            let mon: MonName = entry.value_or_else("mon")?;
            let on: MonName = entry.value_or_else("on")?;
            ui_log.push(ui::UiLogEntry::Waiting {
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                on: mon_name_to_mon_for_ui_log(state, &on)?,
            });
        }
        "addvolatile"
        | "removevolatile"
        | "addsidecondition"
        | "removesidecondition"
        | "addslotcondition"
        | "removeslotcondition"
        | "addpseudoweather"
        | "removepseudoweather" => {
            // Debug only logs, ignore in state tracking.
        }
        title @ _ => {
            let orig_title = entry.title();
            if orig_title.starts_with("-") && orig_title.contains(":") {
                let (source, title) = orig_title
                    .split_once(":")
                    .ok_or_else(|| Error::msg("extension log had no title following a colon"))?;
                ui_log.push(ui::UiLogEntry::Extension {
                    source: source.to_owned(),
                    title: title.to_owned(),
                    values: entry
                        .values()
                        .map(|(k, v)| (k.to_owned(), v.to_owned()))
                        .collect(),
                });
            } else {
                return Err(Error::msg(format!("unsupported log: {title}")));
            }
        }
    }
    Ok(())
}
