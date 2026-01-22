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
                    "moves": ["Splash"],
                    "level": 100,
                    "evs": { "hp": 252, "spd": 252 }
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
                    "moves": ["Splash"],
                    "level": 100,
                    "evs": { "hp": 252, "spd": 252 }
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

    // Turn 1: Switch Player 1 to Chandelure.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Hex on healthy target.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 3: Thunder Wave.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 4: Hex on paralyzed target.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Mew has 373 HP.
    // Hex 1: 186 damage. Remaining: 187.
    // Hex 2 (doubled): ~366 damage. Remaining: -179 -> 0.
    // If not doubled: 186 damage. Remaining: 1.
    // So if it faints, we know it dealt > 187 damage.

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Chandelure,player-1,1|name:Hex|target:Mew,player-2,1",
            "supereffective|mon:Mew,player-2,1",
            "split|side:1",
            ["damage", "mon:Mew,player-2,1", "health:187/373"],
            ["damage", "mon:Mew,player-2,1", "health:51/100"],
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Chandelure,player-1,1|name:Thunder Wave|target:Mew,player-2,1",
            "status|mon:Mew,player-2,1|status:Paralysis",
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

    // Turn 1: Switch Player 1 to Chandelure.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Hex on Mew.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 3: Player 2 switches to Comatose Mew.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));

    // Turn 4: Hex on Comatose Mew.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Normal Mew HP 373 -> 187 (186 dmg).
    // Comatose Mew HP 373 -> 7 (366 dmg).
    // If not doubled, it would be 187.
    // We check exact logs.

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Chandelure,player-1,1|name:Hex|target:Mew,player-2,1",
            "supereffective|mon:Mew,player-2,1",
            "split|side:1",
            ["damage", "mon:Mew,player-2,1", "health:187/373"],
            ["damage", "mon:Mew,player-2,1", "health:51/100"],
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:1",
            ["switch", "player:player-2", "position:1", "name:Mew", "health:373/373"],
            ["switch", "player:player-2", "position:1", "name:Mew", "health:100/100"],
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Chandelure,player-1,1|name:Hex|target:Mew,player-2,1",
            "supereffective|mon:Mew,player-2,1",
            "split|side:1",
            ["damage", "mon:Mew,player-2,1", "health:7/373"],
            ["damage", "mon:Mew,player-2,1", "health:2/100"],
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
