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

fn eevee() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Eevee",
                    "species": "Eevee",
                    "ability": "No Ability",
                    "moves": [
                        "Trump Card",
                        "Recover",
                        "Mirror Move"
                    ],
                    "nature": "Hardy",
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn trump_card_power_increases_with_less_pp() {
    let mut battle = make_battle(12345, eevee().unwrap(), eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eevee,player-1,1|name:Trump Card|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:94/115",
            "damage|mon:Eevee,player-2,1|health:82/100",
            "move|mon:Eevee,player-2,1|name:Recover|target:Eevee,player-2,1",
            "split|side:1",
            "heal|mon:Eevee,player-2,1|health:115/115",
            "heal|mon:Eevee,player-2,1|health:100/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Eevee,player-1,1|name:Trump Card|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:90/115",
            "damage|mon:Eevee,player-2,1|health:79/100",
            "move|mon:Eevee,player-2,1|name:Recover|target:Eevee,player-2,1",
            "split|side:1",
            "heal|mon:Eevee,player-2,1|health:115/115",
            "heal|mon:Eevee,player-2,1|health:100/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Eevee,player-1,1|name:Trump Card|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:85/115",
            "damage|mon:Eevee,player-2,1|health:74/100",
            "move|mon:Eevee,player-2,1|name:Recover|target:Eevee,player-2,1",
            "split|side:1",
            "heal|mon:Eevee,player-2,1|health:115/115",
            "heal|mon:Eevee,player-2,1|health:100/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Eevee,player-1,1|name:Trump Card|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:75/115",
            "damage|mon:Eevee,player-2,1|health:66/100",
            "move|mon:Eevee,player-2,1|name:Recover|target:Eevee,player-2,1",
            "split|side:1",
            "heal|mon:Eevee,player-2,1|health:115/115",
            "heal|mon:Eevee,player-2,1|health:100/100",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Eevee,player-1,1|name:Trump Card|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:19/115",
            "damage|mon:Eevee,player-2,1|health:17/100",
            "move|mon:Eevee,player-2,1|name:Recover|target:Eevee,player-2,1",
            "split|side:1",
            "heal|mon:Eevee,player-2,1|health:77/115",
            "heal|mon:Eevee,player-2,1|health:67/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn trump_card_power_based_on_move_source() {
    let mut battle = make_battle(12345, eevee().unwrap(), eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eevee,player-2,1|name:Mirror Move|target:Eevee,player-1,1",
            "move|mon:Eevee,player-2,1|name:Trump Card|target:Eevee,player-1,1|from:move:Mirror Move",
            "split|side:0",
            "damage|mon:Eevee,player-1,1|health:94/115",
            "damage|mon:Eevee,player-1,1|health:82/100",
            "residual",
            "turn|turn:7",
            "continue",
            "move|mon:Eevee,player-2,1|name:Mirror Move|target:Eevee,player-1,1",
            "move|mon:Eevee,player-2,1|name:Trump Card|target:Eevee,player-1,1|from:move:Mirror Move",
            "split|side:0",
            "damage|mon:Eevee,player-1,1|health:73/115",
            "damage|mon:Eevee,player-1,1|health:64/100",
            "residual",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 6, &expected_logs);
}
