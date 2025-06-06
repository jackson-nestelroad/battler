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

fn wobbuffet() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Wobbuffet",
                    "species": "Wobbuffet",
                    "ability": "No Ability",
                    "moves": [
                        "Substitute",
                        "Tackle",
                        "Agility",
                        "Cotton Spore",
                        "Crunch",
                        "Poison Powder"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn shedinja() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Shedinja",
                    "species": "Shedinja",
                    "ability": "No Ability",
                    "moves": [
                        "Substitute"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
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
fn substitute_avoids_hit_effects_until_broken() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_battle(&data, 1434343, wobbuffet().unwrap(), wobbuffet().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Wobbuffet,player-1,1|name:Substitute|target:Wobbuffet,player-1,1",
            "start|mon:Wobbuffet,player-1,1|move:Substitute",
            "split|side:0",
            "damage|mon:Wobbuffet,player-1,1|health:188/250",
            "damage|mon:Wobbuffet,player-1,1|health:76/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Wobbuffet,player-1,1|name:Tackle|target:Wobbuffet,player-2,1",
            "split|side:1",
            "damage|mon:Wobbuffet,player-2,1|health:240/250",
            "damage|mon:Wobbuffet,player-2,1|health:96/100",
            "move|mon:Wobbuffet,player-2,1|name:Tackle|target:Wobbuffet,player-1,1",
            "activate|mon:Wobbuffet,player-1,1|move:Substitute|damage",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Wobbuffet,player-1,1|name:Agility|target:Wobbuffet,player-1,1",
            "boost|mon:Wobbuffet,player-1,1|stat:spe|by:2",
            "move|mon:Wobbuffet,player-2,1|name:Cotton Spore",
            "activate|mon:Wobbuffet,player-1,1|move:Substitute|damage",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Wobbuffet,player-1,1|name:Crunch|target:Wobbuffet,player-2,1",
            "supereffective|mon:Wobbuffet,player-2,1",
            "split|side:1",
            "damage|mon:Wobbuffet,player-2,1|health:196/250",
            "damage|mon:Wobbuffet,player-2,1|health:79/100",
            "move|mon:Wobbuffet,player-2,1|name:Crunch|target:Wobbuffet,player-1,1",
            "supereffective|mon:Wobbuffet,player-1,1",
            "activate|mon:Wobbuffet,player-1,1|move:Substitute|damage",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Wobbuffet,player-2,1|name:Poison Powder|target:Wobbuffet,player-1,1",
            "activate|mon:Wobbuffet,player-1,1|move:Substitute|damage",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Wobbuffet,player-2,1|name:Crunch|target:Wobbuffet,player-1,1",
            "supereffective|mon:Wobbuffet,player-1,1",
            "crit|mon:Wobbuffet,player-1,1",
            "end|mon:Wobbuffet,player-1,1|move:Substitute",
            "residual",
            "turn|turn:7",
            ["time"],
            "move|mon:Wobbuffet,player-1,1|name:Crunch|target:Wobbuffet,player-2,1",
            "supereffective|mon:Wobbuffet,player-2,1",
            "split|side:1",
            "damage|mon:Wobbuffet,player-2,1|health:156/250",
            "damage|mon:Wobbuffet,player-2,1|health:63/100",
            "move|mon:Wobbuffet,player-2,1|name:Crunch|target:Wobbuffet,player-1,1",
            "supereffective|mon:Wobbuffet,player-1,1",
            "split|side:0",
            "damage|mon:Wobbuffet,player-1,1|health:144/250",
            "damage|mon:Wobbuffet,player-1,1|health:58/100",
            "residual",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn shedinja_cant_substitute() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, shedinja().unwrap(), shedinja().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Shedinja,player-1,1|name:Substitute|target:Shedinja,player-1,1",
            "fail|mon:Shedinja,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
