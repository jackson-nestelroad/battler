use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Skarmory",
                    "species": "Skarmory",
                    "ability": "No Ability",
                    "moves": [
                        "Spikes"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Houndoom",
                    "species": "Houndoom",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Kingdra",
                    "species": "Kingdra",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Donphan",
                    "species": "Donphan",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Delibird",
                    "species": "Delibird",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Octillery",
                    "species": "Octillery",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Heavy-Duty Boots"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn spikes_damages_opposing_side_on_switch_in() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;switch 3"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 1;switch 4"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;switch 3"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Skarmory,player-1,1|name:Spikes",
            "sidestart|side:1|move:Spikes|count:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Donphan"],
            ["switch", "player-2", "Donphan"],
            "split|side:1",
            ["switch", "player-2", "Kingdra"],
            ["switch", "player-2", "Kingdra"],
            "split|side:1",
            "damage|mon:Kingdra,player-2,1|from:move:Spikes|health:119/135",
            "damage|mon:Kingdra,player-2,1|from:move:Spikes|health:89/100",
            "split|side:1",
            "damage|mon:Donphan,player-2,2|from:move:Spikes|health:132/150",
            "damage|mon:Donphan,player-2,2|from:move:Spikes|health:88/100",
            "move|mon:Skarmory,player-1,1|name:Spikes",
            "sidestart|side:1|move:Spikes|count:2",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Houndoom"],
            ["switch", "player-2", "Houndoom"],
            "split|side:1",
            ["switch", "player-2", "Delibird"],
            ["switch", "player-2", "Delibird"],
            "split|side:1",
            "damage|mon:Houndoom,player-2,1|from:move:Spikes|health:113/135",
            "damage|mon:Houndoom,player-2,1|from:move:Spikes|health:84/100",
            "move|mon:Skarmory,player-1,1|name:Spikes",
            "sidestart|side:1|move:Spikes|count:3",
            "residual",
            "turn|turn:4",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Kingdra"],
            ["switch", "player-2", "Kingdra"],
            "split|side:1",
            ["switch", "player-2", "Donphan"],
            ["switch", "player-2", "Donphan"],
            "split|side:1",
            "damage|mon:Kingdra,player-2,1|from:move:Spikes|health:86/135",
            "damage|mon:Kingdra,player-2,1|from:move:Spikes|health:64/100",
            "split|side:1",
            "damage|mon:Donphan,player-2,2|from:move:Spikes|health:95/150",
            "damage|mon:Donphan,player-2,2|from:move:Spikes|health:64/100",
            "move|mon:Skarmory,player-1,1|name:Spikes|noanim",
            "fail|mon:Skarmory,player-1,1",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flying_types_avoid_spikes() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 4;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Skarmory,player-1,1|name:Spikes",
            "sidestart|side:1|move:Spikes|count:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Delibird"],
            ["switch", "player-2", "Delibird"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn heavy_duty_boots_avoid_spikes() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 5;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Skarmory,player-1,1|name:Spikes",
            "sidestart|side:1|move:Spikes|count:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Octillery"],
            ["switch", "player-2", "Octillery"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
