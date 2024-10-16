use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    error::{
        Error,
        WrapResultError,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Espeon",
                    "species": "Espeon",
                    "ability": "No Ability",
                    "moves": [
                        "Baton Pass",
                        "Growth",
                        "Agility",
                        "Focus Energy",
                        "Substitute",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "gender": "F"
                },
                {
                    "name": "Umbreon",
                    "species": "Umbreon",
                    "ability": "No Ability",
                    "moves": [
                        "Baton Pass",
                        "Mud Slap",
                        "Perish Song",
                        "Tackle",
                        "Pursuit"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "gender": "M"
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
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
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
fn baton_pass_switches_user_out_and_passes_volatiles() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Espeon,player-2,1|name:Baton Pass|target:Espeon,player-2,1",
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:1|name:Umbreon|health:155/155|species:Umbreon|level:50|gender:M",
            "switch|player:player-2|position:1|name:Umbreon|health:100/100|species:Umbreon|level:50|gender:M",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Espeon,player-1,1|name:Growth|target:Espeon,player-1,1",
            "boost|mon:Espeon,player-1,1|stat:atk|by:1",
            "boost|mon:Espeon,player-1,1|stat:spa|by:1",
            "move|mon:Umbreon,player-2,1|name:Mud-Slap|target:Espeon,player-1,1",
            "split|side:0",
            "damage|mon:Espeon,player-1,1|health:119/125",
            "damage|mon:Espeon,player-1,1|health:96/100",
            "unboost|mon:Espeon,player-1,1|stat:acc|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Espeon,player-1,1|name:Agility|target:Espeon,player-1,1",
            "boost|mon:Espeon,player-1,1|stat:spe|by:2",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Espeon,player-1,1|name:Focus Energy|target:Espeon,player-1,1",
            "start|mon:Espeon,player-1,1|move:Focus Energy",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Espeon,player-1,1|name:Substitute|target:Espeon,player-1,1",
            "start|mon:Espeon,player-1,1|move:Substitute",
            "split|side:0",
            "damage|mon:Espeon,player-1,1|health:88/125",
            "damage|mon:Espeon,player-1,1|health:71/100",
            "move|mon:Umbreon,player-2,1|name:Perish Song",
            "fieldactivate|move:Perish Song",
            "start|mon:Espeon,player-1,1|move:Perish Song|perish:3",
            "start|mon:Umbreon,player-2,1|move:Perish Song|perish:3",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Espeon,player-1,1|name:Baton Pass|target:Espeon,player-1,1",
            ["time"],
            "split|side:0",
            "switch|player:player-1|position:1|name:Umbreon|health:155/155|species:Umbreon|level:50|gender:M",
            "switch|player:player-1|position:1|name:Umbreon|health:100/100|species:Umbreon|level:50|gender:M",
            "start|mon:Umbreon,player-1,1|move:Perish Song|perish:2",
            "start|mon:Umbreon,player-2,1|move:Perish Song|perish:2",
            "residual",
            "turn|turn:7",
            ["time"],
            "move|mon:Umbreon,player-1,1|name:Tackle|noanim",
            "miss|mon:Umbreon,player-2,1",
            "move|mon:Umbreon,player-2,1|name:Tackle|target:Umbreon,player-1,1",
            "activate|mon:Umbreon,player-1,1|move:Substitute|damage",
            "start|mon:Umbreon,player-1,1|move:Perish Song|perish:1",
            "start|mon:Umbreon,player-2,1|move:Perish Song|perish:1",
            "residual",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn baton_pass_does_not_activate_pursuit() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            "switch|player:player-2|position:1|name:Umbreon|health:155/155|species:Umbreon|level:50|gender:M",
            "switch|player:player-2|position:1|name:Umbreon|health:100/100|species:Umbreon|level:50|gender:M",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Espeon,player-1,1|name:Baton Pass|target:Espeon,player-1,1",
            ["time"],
            "split|side:0",
            "switch|player:player-1|position:1|name:Umbreon|health:155/155|species:Umbreon|level:50|gender:M",
            "switch|player:player-1|position:1|name:Umbreon|health:100/100|species:Umbreon|level:50|gender:M",
            "move|mon:Umbreon,player-2,1|name:Pursuit|target:Umbreon,player-1,1",
            "resisted|mon:Umbreon,player-1,1",
            "split|side:0",
            "damage|mon:Umbreon,player-1,1|health:147/155",
            "damage|mon:Umbreon,player-1,1|health:95/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
