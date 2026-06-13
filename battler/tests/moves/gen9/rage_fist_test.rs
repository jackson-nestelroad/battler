use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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
                    "name": "Primeape",
                    "species": "Primeape",
                    "ability": "No Ability",
                    "item": "Leppa Berry",
                    "moves": [
                        "Rage Fist",
                        "Recover",
                        "Tackle",
                        "Double Kick",
                        "Transform"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Mankey",
                    "species": "Mankey",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn rage_first_increases_power_for_each_hit() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Primeape,player-1,1|name:Rage Fist|target:Primeape,player-2,1",
            "split|side:1",
            "damage|mon:Primeape,player-2,1|health:166/240",
            "damage|mon:Primeape,player-2,1|health:70/100",
            "move|mon:Primeape,player-2,1|name:Recover|target:Primeape,player-2,1",
            "split|side:1",
            "heal|mon:Primeape,player-2,1|health:240/240",
            "heal|mon:Primeape,player-2,1|health:100/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Primeape,player-2,1|name:Tackle|target:Primeape,player-1,1",
            "split|side:0",
            "damage|mon:Primeape,player-1,1|health:181/240",
            "damage|mon:Primeape,player-1,1|health:76/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Primeape,player-1,1|name:Rage Fist|target:Primeape,player-2,1",
            "split|side:1",
            "damage|mon:Primeape,player-2,1|health:94/240",
            "damage|mon:Primeape,player-2,1|health:40/100",
            "move|mon:Primeape,player-2,1|name:Recover|target:Primeape,player-2,1",
            "split|side:1",
            "heal|mon:Primeape,player-2,1|health:214/240",
            "heal|mon:Primeape,player-2,1|health:90/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Primeape,player-2,1|name:Double Kick|target:Primeape,player-1,1",
            "split|side:0",
            "damage|mon:Primeape,player-1,1|health:114/240",
            "damage|mon:Primeape,player-1,1|health:48/100",
            "animatemove|mon:Primeape,player-2,1|name:Double Kick|target:Primeape,player-1,1",
            "split|side:0",
            "damage|mon:Primeape,player-1,1|health:47/240",
            "damage|mon:Primeape,player-1,1|health:20/100",
            "hitcount|hits:2",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Primeape,player-2,1|name:Recover|target:Primeape,player-2,1",
            "split|side:1",
            "heal|mon:Primeape,player-2,1|health:240/240",
            "heal|mon:Primeape,player-2,1|health:100/100",
            "residual",
            "turn|turn:6",
            "continue",
            "move|mon:Primeape,player-1,1|name:Rage Fist|target:Primeape,player-2,1",
            "split|side:1",
            "damage|mon:Primeape,player-2,1|health:22/240",
            "damage|mon:Primeape,player-2,1|health:10/100",
            "move|mon:Primeape,player-2,1|name:Recover|target:Primeape,player-2,1",
            "split|side:1",
            "heal|mon:Primeape,player-2,1|health:142/240",
            "heal|mon:Primeape,player-2,1|health:60/100",
            "residual",
            "turn|turn:7",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Mankey"],
            ["switch", "player-1", "Mankey"],
            "move|mon:Primeape,player-2,1|name:Recover|target:Primeape,player-2,1",
            "split|side:1",
            "heal|mon:Primeape,player-2,1|health:240/240",
            "heal|mon:Primeape,player-2,1|health:100/100",
            "itemend|mon:Primeape,player-2,1|item:Leppa Berry|eat",
            "restorepp|mon:Primeape,player-2,1|move:Recover|by:5|from:item:Leppa Berry",
            "residual",
            "turn|turn:8",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Primeape"],
            ["switch", "player-1", "Primeape"],
            "residual",
            "turn|turn:9",
            "continue",
            "move|mon:Primeape,player-1,1|name:Rage Fist|target:Primeape,player-2,1",
            "split|side:1",
            "damage|mon:Primeape,player-2,1|health:22/240",
            "damage|mon:Primeape,player-2,1|health:10/100",
            "move|mon:Primeape,player-2,1|name:Recover|target:Primeape,player-2,1",
            "split|side:1",
            "heal|mon:Primeape,player-2,1|health:142/240",
            "heal|mon:Primeape,player-2,1|health:60/100",
            "residual",
            "turn|turn:10",
            "continue",
            "move|mon:Primeape,player-1,1|name:Recover|target:Primeape,player-1,1",
            "split|side:0",
            "heal|mon:Primeape,player-1,1|health:167/240",
            "heal|mon:Primeape,player-1,1|health:70/100",
            "move|mon:Primeape,player-2,1|name:Transform|target:Primeape,player-1,1",
            "transform|mon:Primeape,player-2,1|into:Primeape,player-1,1|species:Primeape",
            "residual",
            "turn|turn:11",
            "continue",
            "move|mon:Primeape,player-1,1|name:Recover|target:Primeape,player-1,1",
            "split|side:0",
            "heal|mon:Primeape,player-1,1|health:240/240",
            "heal|mon:Primeape,player-1,1|health:100/100",
            "move|mon:Primeape,player-2,1|name:Rage Fist|target:Primeape,player-1,1",
            "split|side:0",
            "damage|mon:Primeape,player-1,1|health:22/240",
            "damage|mon:Primeape,player-1,1|health:10/100",
            "residual",
            "turn|turn:12"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
