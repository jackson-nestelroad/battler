use anyhow::Result;
use battler::Nature;
use battler_calc::{
    simulate::{
        MoveSimulatorInput,
        MoveSimulatorInputFlags,
    },
    state::Move,
};
use battler_calc_client_util::{
    Assumption,
    Assumptions,
    Mon,
    move_simulator_input_from_battle_state,
};

pub fn assumptions() -> Assumptions {
    Assumptions {
        ivs: Assumption::Maximum,
        evs: Assumption::Minimum,
        nature: Some(Nature::Hardy),
    }
}

fn default_move() -> Move {
    Move {
        name: "Pound".to_owned(),
        ..Default::default()
    }
}

fn default_defender() -> battler_calc::state::Mon {
    battler_calc::state::Mon {
        name: "Bulbasaur".to_owned(),
        ..Default::default()
    }
}

pub fn move_simulator_input_for_non_moves<'m>(
    attacker: &'m Mon,
    defender: &'m Mon,
) -> Result<MoveSimulatorInput<'m>> {
    move_simulator_input_from_battle_state(
        attacker.data(),
        attacker.state(),
        attacker.reference().clone(),
        defender.reference().clone(),
        default_move(),
        &assumptions(),
        MoveSimulatorInputFlags {
            indirect: true,
            ..Default::default()
        },
    )
}

pub fn move_simulator_input_for_non_moves_no_defender<'m>(
    attacker: &'m Mon,
) -> Result<MoveSimulatorInput<'m>> {
    let mut input = move_simulator_input_from_battle_state(
        attacker.data(),
        attacker.state(),
        attacker.reference().clone(),
        attacker.reference().clone(),
        default_move(),
        &assumptions(),
        MoveSimulatorInputFlags {
            indirect: true,
            ..Default::default()
        },
    )?;
    input.defender = default_defender();
    Ok(input)
}
