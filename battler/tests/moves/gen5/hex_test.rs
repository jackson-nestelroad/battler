use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    static_local_data_store,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Mew",
                    "species": "Mew",
                    "ability": "No Ability",
                    "moves": ["Recover"],
                    "level": 100
                },
                {
                    "name": "Chandelure",
                    "species": "Chandelure",
                    "ability": "No Ability",
                    "moves": ["Hex", "Thunder Wave"],
                    "level": 100
                },
                {
                    "name": "Mew",
                    "species": "Mew",
                    "ability": "Comatose",
                    "moves": [],
                    "level": 100
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(seed: u64) -> TestBattleBuilder {
    TestBattleBuilder::new()
        .with_seed(seed)
        .with_battle_type(BattleType::Singles)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
}

#[test]
fn hex_doubles_power_on_status() {
    let mut battle = make_battle(0)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team())
        .with_team("player-2", team())
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Switch to Chandelure.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Hex on healthy target.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Recover.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // Thunder Wave.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Hex on paralyzed target.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Chandelure,player-1,1|name:Hex|target:Mew,player-2,1",
            "supereffective|mon:Mew,player-2,1",
            "split|side:1",
            ["damage", "mon:Mew,player-2,1", "health:70/310"],
            ["damage", "mon:Mew,player-2,1", "health:23/100"],
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Mew,player-2,1|name:Recover|target:Mew,player-2,1",
            "split|side:1",
            ["heal", "mon:Mew,player-2,1", "health:225/310"],
            ["heal", "mon:Mew,player-2,1", "health:73/100"],
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Chandelure,player-1,1|name:Thunder Wave|target:Mew,player-2,1",
            "status|mon:Mew,player-2,1|status:Paralysis",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Chandelure,player-1,1|name:Hex|target:Mew,player-2,1",
            "supereffective|mon:Mew,player-2,1",
            "split|side:1",
            ["damage", "mon:Mew,player-2,1", "health:0"],
            ["damage", "mon:Mew,player-2,1", "health:0"],
            "faint|mon:Mew,player-2,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}

#[test]
fn hex_doubles_power_on_comatose() {
    let mut battle = make_battle(0)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team())
        .with_team("player-2", team())
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Switch to Chandelure.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Hex on Mew.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Switch to Comatose Mew.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));

    // Hex on Comatose Mew.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Chandelure,player-1,1|name:Hex|target:Mew,player-2,1",
            "supereffective|mon:Mew,player-2,1",
            "split|side:1",
            ["damage", "mon:Mew,player-2,1", "health:70/310"],
            ["damage", "mon:Mew,player-2,1", "health:23/100"],
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:1",
            ["switch", "player:player-2", "position:1", "name:Mew", "health:310/310"],
            ["switch", "player:player-2", "position:1", "name:Mew", "health:100/100"],
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Chandelure,player-1,1|name:Hex|target:Mew,player-2,1",
            "supereffective|mon:Mew,player-2,1",
            "split|side:1",
            ["damage", "mon:Mew,player-2,1", "health:0"],
            ["damage", "mon:Mew,player-2,1", "health:0"],
            "faint|mon:Mew,player-2,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
