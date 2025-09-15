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
    get_controlled_rng_for_battle,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gyarados",
                    "species": "Gyarados",
                    "ability": "No Ability",
                    "moves": [
                        "Rage",
                        "Tackle",
                        "Fury Attack"
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

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn rage_increases_attack_on_hit() {
    let mut battle = make_battle(111111, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 17)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 17)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gyarados,player-1,1|name:Rage|target:Gyarados,player-2,1",
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|health:142/155",
            "damage|mon:Gyarados,player-2,1|health:92/100",
            "move|mon:Gyarados,player-2,1|name:Tackle|target:Gyarados,player-1,1",
            "split|side:0",
            "damage|mon:Gyarados,player-1,1|health:126/155",
            "damage|mon:Gyarados,player-1,1|health:82/100",
            "boost|mon:Gyarados,player-1,1|stat:atk|by:1|from:move:Rage",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Gyarados,player-2,1|name:Tackle|target:Gyarados,player-1,1",
            "split|side:0",
            "damage|mon:Gyarados,player-1,1|health:102/155",
            "damage|mon:Gyarados,player-1,1|health:66/100",
            "boost|mon:Gyarados,player-1,1|stat:atk|by:1|from:move:Rage",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Gyarados,player-2,1|name:Fury Attack|noanim",
            "miss|mon:Gyarados,player-1,1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Gyarados,player-2,1|name:Fury Attack|target:Gyarados,player-1,1",
            "split|side:0",
            "damage|mon:Gyarados,player-1,1|health:92/155",
            "damage|mon:Gyarados,player-1,1|health:60/100",
            "boost|mon:Gyarados,player-1,1|stat:atk|by:1|from:move:Rage",
            "animatemove|mon:Gyarados,player-2,1|name:Fury Attack|target:Gyarados,player-1,1",
            "split|side:0",
            "damage|mon:Gyarados,player-1,1|health:82/155",
            "damage|mon:Gyarados,player-1,1|health:53/100",
            "boost|mon:Gyarados,player-1,1|stat:atk|by:1|from:move:Rage",
            "animatemove|mon:Gyarados,player-2,1|name:Fury Attack|target:Gyarados,player-1,1",
            "split|side:0",
            "damage|mon:Gyarados,player-1,1|health:71/155",
            "damage|mon:Gyarados,player-1,1|health:46/100",
            "boost|mon:Gyarados,player-1,1|stat:atk|by:1|from:move:Rage",
            "animatemove|mon:Gyarados,player-2,1|name:Fury Attack|target:Gyarados,player-1,1",
            "split|side:0",
            "damage|mon:Gyarados,player-1,1|health:61/155",
            "damage|mon:Gyarados,player-1,1|health:40/100",
            "boost|mon:Gyarados,player-1,1|stat:atk|by:1|from:move:Rage",
            "animatemove|mon:Gyarados,player-2,1|name:Fury Attack|target:Gyarados,player-1,1",
            "split|side:0",
            "damage|mon:Gyarados,player-1,1|health:51/155",
            "damage|mon:Gyarados,player-1,1|health:33/100",
            "boost|mon:Gyarados,player-1,1|stat:atk|by:0|from:move:Rage",
            "hitcount|hits:5",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Gyarados,player-1,1|name:Rage|target:Gyarados,player-2,1",
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|health:92/155",
            "damage|mon:Gyarados,player-2,1|health:60/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
