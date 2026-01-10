use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Roserade",
                    "species": "Roserade",
                    "ability": "No Ability",
                    "moves": [
                        "Toxic Spikes"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Torterra",
                    "species": "Torterra",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Infernape",
                    "species": "Infernape",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Vespiquen",
                    "species": "Vespiquen",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Empoleon",
                    "species": "Empoleon",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Drapion",
                    "species": "Drapion",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Rampardos",
                    "species": "Rampardos",
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

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
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
        .build(static_local_data_store())
}

#[test]
fn toxic_spikes_poison_opposing_side_on_switch_in() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;switch 2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Roserade,player-1,1|name:Toxic Spikes",
            "sidestart|side:1|move:Toxic Spikes|count:1",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Infernape"],
            ["switch", "player-2", "Infernape"],
            "status|mon:Infernape,player-2,2|status:Poison|from:move:Toxic Spikes",
            "move|mon:Roserade,player-1,1|name:Toxic Spikes",
            "sidestart|side:1|move:Toxic Spikes|count:2",
            "split|side:1",
            "damage|mon:Infernape,player-2,2|from:status:Poison|health:119/136",
            "damage|mon:Infernape,player-2,2|from:status:Poison|health:88/100",
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Torterra"],
            ["switch", "player-2", "Torterra"],
            "status|mon:Torterra,player-2,1|status:Bad Poison|from:move:Toxic Spikes",
            "move|mon:Roserade,player-1,1|name:Toxic Spikes|noanim",
            "fail|mon:Roserade,player-1,1",
            "split|side:1",
            "damage|mon:Infernape,player-2,2|from:status:Poison|health:102/136",
            "damage|mon:Infernape,player-2,2|from:status:Poison|health:75/100",
            "split|side:1",
            "damage|mon:Torterra,player-2,1|from:status:Bad Poison|health:146/155",
            "damage|mon:Torterra,player-2,1|from:status:Bad Poison|health:95/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flying_types_avoid_toxic_spikes() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 3;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Roserade,player-1,1|name:Toxic Spikes",
            "sidestart|side:1|move:Toxic Spikes|count:1",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Vespiquen"],
            ["switch", "player-2", "Vespiquen"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn steel_types_are_immune_to_toxic_spikes() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
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
            "move|mon:Roserade,player-1,1|name:Toxic Spikes",
            "sidestart|side:1|move:Toxic Spikes|count:1",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Empoleon"],
            ["switch", "player-2", "Empoleon"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn poison_type_absorbs_toxic_spikes() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 5;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Roserade,player-1,1|name:Toxic Spikes",
            "sidestart|side:1|move:Toxic Spikes|count:1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Roserade,player-1,1|name:Toxic Spikes",
            "sidestart|side:1|move:Toxic Spikes|count:2",
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Drapion"],
            ["switch", "player-2", "Drapion"],
            "sideend|side:1|move:Toxic Spikes|of:Drapion,player-2,1",
            "residual",
            "turn|turn:4",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Infernape"],
            ["switch", "player-2", "Infernape"],
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn heavy_duty_boots_avoid_toxic_spikes() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 6;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Roserade,player-1,1|name:Toxic Spikes",
            "sidestart|side:1|move:Toxic Spikes|count:1",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Rampardos"],
            ["switch", "player-2", "Rampardos"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
