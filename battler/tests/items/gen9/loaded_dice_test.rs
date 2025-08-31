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
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn maushold() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Maushold",
                    "species": "Maushold",
                    "ability": "No Ability",
                    "item": "Loaded Dice",
                    "moves": [
                        "Triple Kick",
                        "Fury Attack",
                        "Population Bomb",
                        "Recover"
                    ],
                    "nature": "Hardy",
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
) -> Result<PublicCoreBattle<'_>> {
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
fn loaded_dice_removes_multiaccuracy() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, maushold().unwrap(), maushold().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Maushold,player-1,1|name:Triple Kick|target:Maushold,player-2,1",
            "supereffective|mon:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:124/134",
            "damage|mon:Maushold,player-2,1|health:93/100",
            "animatemove|mon:Maushold,player-1,1|name:Triple Kick|target:Maushold,player-2,1",
            "supereffective|mon:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:102/134",
            "damage|mon:Maushold,player-2,1|health:77/100",
            "animatemove|mon:Maushold,player-1,1|name:Triple Kick|target:Maushold,player-2,1",
            "supereffective|mon:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:74/134",
            "damage|mon:Maushold,player-2,1|health:56/100",
            "hitcount|hits:3",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn loaded_dice_makes_multihit_move_hit_at_least_four_times() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, maushold().unwrap(), maushold().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Maushold,player-1,1|name:Fury Attack|target:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:122/134",
            "damage|mon:Maushold,player-2,1|health:92/100",
            "animatemove|mon:Maushold,player-1,1|name:Fury Attack|target:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:110/134",
            "damage|mon:Maushold,player-2,1|health:83/100",
            "animatemove|mon:Maushold,player-1,1|name:Fury Attack|target:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:98/134",
            "damage|mon:Maushold,player-2,1|health:74/100",
            "animatemove|mon:Maushold,player-1,1|name:Fury Attack|target:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:86/134",
            "damage|mon:Maushold,player-2,1|health:65/100",
            "hitcount|hits:4",
            "move|mon:Maushold,player-2,1|name:Recover|target:Maushold,player-2,1",
            "split|side:1",
            "heal|mon:Maushold,player-2,1|health:134/134",
            "heal|mon:Maushold,player-2,1|health:100/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Maushold,player-1,1|name:Fury Attack|target:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:122/134",
            "damage|mon:Maushold,player-2,1|health:92/100",
            "animatemove|mon:Maushold,player-1,1|name:Fury Attack|target:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:110/134",
            "damage|mon:Maushold,player-2,1|health:83/100",
            "animatemove|mon:Maushold,player-1,1|name:Fury Attack|target:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:98/134",
            "damage|mon:Maushold,player-2,1|health:74/100",
            "animatemove|mon:Maushold,player-1,1|name:Fury Attack|target:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:88/134",
            "damage|mon:Maushold,player-2,1|health:66/100",
            "hitcount|hits:4",
            "move|mon:Maushold,player-2,1|name:Recover|target:Maushold,player-2,1",
            "split|side:1",
            "heal|mon:Maushold,player-2,1|health:134/134",
            "heal|mon:Maushold,player-2,1|health:100/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Maushold,player-1,1|name:Population Bomb|target:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:121/134",
            "damage|mon:Maushold,player-2,1|health:91/100",
            "animatemove|mon:Maushold,player-1,1|name:Population Bomb|target:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:108/134",
            "damage|mon:Maushold,player-2,1|health:81/100",
            "animatemove|mon:Maushold,player-1,1|name:Population Bomb|target:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:95/134",
            "damage|mon:Maushold,player-2,1|health:71/100",
            "animatemove|mon:Maushold,player-1,1|name:Population Bomb|target:Maushold,player-2,1",
            "split|side:1",
            "damage|mon:Maushold,player-2,1|health:80/134",
            "damage|mon:Maushold,player-2,1|health:60/100",
            "hitcount|hits:4",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
