use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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

fn cresselia() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Cresselia",
                    "species": "Cresselia",
                    "ability": "No Ability",
                    "moves": [
                        "Power Swap",
                        "Growth",
                        "Tackle",
                        "Surf",
                        "Heart Swap"
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn power_swap_swaps_atk_and_spa_boosts() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, cresselia().unwrap(), cresselia().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cresselia,player-1,1|name:Tackle|target:Cresselia,player-2,1",
            "split|side:1",
            "damage|mon:Cresselia,player-2,1|health:167/180",
            "damage|mon:Cresselia,player-2,1|health:93/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Cresselia,player-1,1|name:Surf",
            "split|side:1",
            "damage|mon:Cresselia,player-2,1|health:140/180",
            "damage|mon:Cresselia,player-2,1|health:78/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Cresselia,player-1,1|name:Growth|target:Cresselia,player-1,1",
            "boost|mon:Cresselia,player-1,1|stat:atk|by:1",
            "boost|mon:Cresselia,player-1,1|stat:spa|by:1",
            "move|mon:Cresselia,player-2,1|name:Power Swap|target:Cresselia,player-1,1",
            "swapboosts|mon:Cresselia,player-1,1|stats:atk,spa|from:move:Power Swap|of:Cresselia,player-2,1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Cresselia,player-2,1|name:Tackle|target:Cresselia,player-1,1",
            "split|side:0",
            "damage|mon:Cresselia,player-1,1|health:161/180",
            "damage|mon:Cresselia,player-1,1|health:90/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Cresselia,player-2,1|name:Surf",
            "split|side:0",
            "damage|mon:Cresselia,player-1,1|health:121/180",
            "damage|mon:Cresselia,player-1,1|health:68/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn heart_swap_swaps_all_boosts() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, cresselia().unwrap(), cresselia().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cresselia,player-1,1|name:Tackle|target:Cresselia,player-2,1",
            "split|side:1",
            "damage|mon:Cresselia,player-2,1|health:167/180",
            "damage|mon:Cresselia,player-2,1|health:93/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Cresselia,player-1,1|name:Surf",
            "split|side:1",
            "damage|mon:Cresselia,player-2,1|health:140/180",
            "damage|mon:Cresselia,player-2,1|health:78/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Cresselia,player-1,1|name:Growth|target:Cresselia,player-1,1",
            "boost|mon:Cresselia,player-1,1|stat:atk|by:1",
            "boost|mon:Cresselia,player-1,1|stat:spa|by:1",
            "move|mon:Cresselia,player-2,1|name:Heart Swap|target:Cresselia,player-1,1",
            "swapboosts|mon:Cresselia,player-1,1|from:move:Heart Swap|of:Cresselia,player-2,1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Cresselia,player-2,1|name:Tackle|target:Cresselia,player-1,1",
            "split|side:0",
            "damage|mon:Cresselia,player-1,1|health:161/180",
            "damage|mon:Cresselia,player-1,1|health:90/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Cresselia,player-2,1|name:Surf",
            "split|side:0",
            "damage|mon:Cresselia,player-1,1|health:121/180",
            "damage|mon:Cresselia,player-1,1|health:68/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
