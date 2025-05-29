mod mon;

use anyhow::{
    Error,
    Result,
};
use battler::{
    DataStoreByName,
    Fraction,
    Nature,
    StatTable,
};
use battler_calc::{
    simulate::{
        MoveSimulatorInput,
        MoveSimulatorInputFlags,
    },
    state::{
        self,
        *,
    },
};
use battler_client::{
    state::BattleState,
    state_util::{
        field_conditions,
        field_terrain,
        field_weather,
        side_conditions,
    },
};
pub use mon::{
    Mon,
    MonReference,
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

/// Constructs input for the move simulator from the given battle state.
pub fn move_simulator_input_from_battle_state<'d>(
    data: &'d dyn DataStoreByName,
    state: &BattleState,
    attacker: MonReference,
    defender: MonReference,
    mov: Move,
    assumptions: &Assumptions,
    mut flags: MoveSimulatorInputFlags,
) -> Result<MoveSimulatorInput<'d>> {
    let attacker = Mon::new(attacker, state, data);
    let defender = Mon::new(defender, state, data);
    flags.attacking_self = attacker.is_same(&defender)?;
    Ok(MoveSimulatorInput {
        data,
        field: create_field(state, attacker.side()?, defender.side()?)?,
        attacker: create_mon(attacker, assumptions)?,
        defender: create_mon(defender, assumptions)?,
        mov,
        flags,
    })
}

fn create_field(state: &BattleState, attacker_side: usize, defender_side: usize) -> Result<Field> {
    Ok(Field {
        battle_type: state.battle_type.clone(),
        weather: field_weather(state).map(|s| s.to_owned()),
        terrain: field_terrain(state).map(|s| s.to_owned()),
        environment: state.field.environment.clone(),
        conditions: field_conditions(state).map(|s| s.to_owned()).collect(),
        attacker_side: create_side(state, attacker_side)?,
        defender_side: create_side(state, defender_side)?,
    })
}

fn create_side(state: &BattleState, side: usize) -> Result<Side> {
    Ok(Side {
        conditions: side_conditions(state, side)?
            .map(|s| s.to_owned())
            .collect(),
    })
}

fn create_mon(mon: Mon, assumptions: &Assumptions) -> Result<state::Mon> {
    Ok(state::Mon {
        name: mon.name()?.to_owned(),
        level: mon
            .level()?
            .ok_or_else(|| Error::msg("level is required"))?,
        health: mon.health()?.map(|(a, b)| Fraction::new(a, b)),
        ability: mon.ability()?.map(|s| s.to_owned()),
        item: mon.item()?.map(|s| s.to_owned()),
        gender: Some(mon.gender()?),
        nature: mon.nature(),
        ivs: mon.ivs().or_else(|| assumptions.get_ivs()),
        evs: mon.evs().or_else(|| assumptions.get_evs()),
        boosts: mon.boosts()?,
        status: mon.status()?.map(|s| s.to_owned()),
        types: mon.types()?,
        conditions: mon
            .conditions()?
            .into_iter()
            .map(|s| s.to_owned())
            .collect(),
        hidden_power_type: mon.hidden_power_type()?,
    })
}
