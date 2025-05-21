use std::str::FromStr;

use anyhow::{
    Error,
    Result,
};
use battler::{
    DataStoreByName,
    Fraction,
    Gender,
    MonBattleData,
    MonSummaryData,
    Nature,
    StatTable,
};
use battler_calc::{
    simulate::MoveSimulatorInput,
    state::{
        Field,
        Mon,
        Move,
        Side,
    },
};
use battler_client::{
    state::{
        BattleState,
        MonBattleAppearanceReference,
    },
    state_util::{
        field_conditions,
        field_terrain,
        field_weather,
        mon_ability,
        mon_boosts,
        mon_conditions,
        mon_health,
        mon_item,
        mon_level,
        mon_or_else,
        mon_species,
        mon_status,
        mon_types,
        side_conditions,
        side_for_mon,
    },
};

/// An assumption on some part of data generated for the move simulator.
#[derive(Debug, Default, Clone)]
pub enum Assumption<T> {
    #[default]
    None,
    Minimum,
    Maximum,
    Custom(T),
}

/// Assumptions for data required by the move simulator.
#[derive(Debug, Default, Clone)]
pub struct Assumptions {
    pub ivs: Assumption<StatTable>,
    pub evs: Assumption<StatTable>,
    pub nature: Option<Nature>,
}

impl Assumptions {
    pub fn get_ivs(&self) -> Option<StatTable> {
        match &self.ivs {
            Assumption::None => None,
            Assumption::Minimum => Some(StatTable::default()),
            Assumption::Maximum => Some(StatTable {
                hp: 31,
                atk: 31,
                def: 31,
                spa: 31,
                spd: 31,
                spe: 31,
            }),
            Assumption::Custom(ivs) => Some(ivs.clone()),
        }
    }

    pub fn get_evs(&self) -> Option<StatTable> {
        match &self.ivs {
            Assumption::None => None,
            Assumption::Minimum => Some(StatTable::default()),
            Assumption::Maximum => Some(StatTable {
                hp: 255,
                atk: 255,
                def: 255,
                spa: 255,
                spd: 255,
                spe: 255,
            }),
            Assumption::Custom(ivs) => Some(ivs.clone()),
        }
    }
}

/// An input Mon.
///
/// If a Mon is owned by the user, then more data about it is available directly from the battle
/// engine. Otherwise, we only know data from the discovered battle state.
#[derive(Debug)]
pub enum MonInput<'m> {
    Owned {
        summary: &'m MonSummaryData,
        battle_data: &'m MonBattleData,
        reference: MonBattleAppearanceReference,
    },
    Unowned(&'m MonBattleAppearanceReference),
}

impl AsRef<MonBattleAppearanceReference> for MonInput<'_> {
    fn as_ref(&self) -> &MonBattleAppearanceReference {
        match self {
            Self::Owned { reference, .. } => reference,
            Self::Unowned(reference) => reference,
        }
    }
}

/// Constructs input for the move simulator from the given battle state.
pub fn move_simulator_input_from_battle_state<'d>(
    data: &'d dyn DataStoreByName,
    state: &BattleState,
    attacker: MonInput,
    defender: MonInput,
    mov: Move,
    assumptions: &Assumptions,
) -> Result<MoveSimulatorInput<'d>> {
    Ok(MoveSimulatorInput {
        data,
        field: create_field(state, attacker.as_ref(), defender.as_ref())?,
        attacker: create_mon(data, state, attacker, assumptions)?,
        defender: create_mon(data, state, defender, assumptions)?,
        mov,
    })
}

fn create_field(
    state: &BattleState,
    attacker: &MonBattleAppearanceReference,
    defender: &MonBattleAppearanceReference,
) -> Result<Field> {
    Ok(Field {
        battle_type: state.battle_type.clone(),
        weather: field_weather(state).map(|s| s.to_owned()),
        terrain: field_terrain(state).map(|s| s.to_owned()),
        environment: state.field.environment.clone(),
        conditions: field_conditions(state).map(|s| s.to_owned()).collect(),
        attacker_side: create_side(state, side_for_mon(state, attacker)?)?,
        defender_side: create_side(state, side_for_mon(state, defender)?)?,
    })
}

fn create_side(state: &BattleState, side: usize) -> Result<Side> {
    Ok(Side {
        conditions: side_conditions(state, side)?
            .map(|s| s.to_owned())
            .collect(),
    })
}

fn create_mon(
    data: &dyn DataStoreByName,
    state: &BattleState,
    mon: MonInput,
    assumptions: &Assumptions,
) -> Result<Mon> {
    match mon {
        MonInput::Owned {
            summary,
            battle_data,
            reference,
        } => create_mon_from_battle_data(state, &reference, &summary, &battle_data),
        MonInput::Unowned(reference) => {
            create_mon_from_reference(data, state, &reference, assumptions)
        }
    }
}

fn create_mon_from_reference(
    data: &dyn DataStoreByName,
    state: &BattleState,
    mon: &MonBattleAppearanceReference,
    assumptions: &Assumptions,
) -> Result<Mon> {
    Ok(Mon {
        name: mon_species(state, mon)?.to_owned(),
        level: mon_level(state, mon)?.ok_or_else(|| Error::msg("level is required"))?,
        health: mon_health(state, mon)?.map(|(a, b)| Fraction::new(a, b)),
        ability: mon_ability(state, mon)?.map(|s| s.to_owned()),
        item: mon_item(state, mon)?.map(|s| s.to_owned()),
        gender: Gender::from_str(&mon_or_else(state, mon)?.physical_appearance.gender).ok(),
        nature: assumptions.nature,
        ivs: assumptions.get_ivs(),
        evs: assumptions.get_evs(),
        boosts: mon_boosts(state, mon)?,
        status: mon_status(state, mon)?.map(|s| s.to_owned()),
        types: mon_types(state, mon, data)?,
        conditions: mon_conditions(state, mon)?.map(|s| s.to_owned()).collect(),
        hidden_power_type: None,
    })
}

fn create_mon_from_battle_data(
    state: &BattleState,
    mon: &MonBattleAppearanceReference,
    summary: &MonSummaryData,
    battle_data: &MonBattleData,
) -> Result<Mon> {
    Ok(Mon {
        name: battle_data.species.clone(),
        level: summary.level as u64,
        health: Some(Fraction::new(
            battle_data.hp as u64,
            battle_data.max_hp as u64,
        )),
        ability: Some(battle_data.ability.clone()),
        item: battle_data.item.clone(),
        gender: Some(summary.gender),
        nature: Some(summary.nature),
        ivs: Some(summary.ivs.clone()),
        evs: Some(summary.evs.clone()),
        boosts: battle_data.boosts.clone(),
        status: battle_data.status.clone(),
        types: battle_data.types.clone(),
        conditions: mon_conditions(state, mon)?.map(|s| s.to_owned()).collect(),
        hidden_power_type: Some(battle_data.hidden_power_type),
    })
}
