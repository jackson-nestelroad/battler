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
                    "moves": ["Recover", "Splash"],
                    "level": 100,
                    "evs": { "hp": 252, "spd": 252 }
                },
                {
                    "name": "Chandelure",
                    "species": "Chandelure",
                    "ability": "No Ability",
                    "moves": ["Hex", "Thunder Wave", "Recover"],
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

fn get_hp(battle: &mut battler::PublicCoreBattle, player_id: &str) -> u16 {
    let data = battle.player_data(player_id).unwrap();
    for mon in &data.mons {
        if mon.active {
            return mon.hp;
        }
    }
    panic!("No active mon found for player {player_id}. Full data: {:?}", data);
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

    let hp_start = 373;
    let hp_after_hex_1 = get_hp(&mut battle, "player-2");
    let damage_1 = hp_start - hp_after_hex_1;

    // Turn 3: Recover on target.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // Turn 4: Thunder Wave.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 5: Hex on paralyzed target.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let hp_after_hex_2 = get_hp(&mut battle, "player-2");
    let damage_2 = hp_start - hp_after_hex_2;

    // Base power doubles from 65 to 130.
    // Due to the +2 constant in the damage formula, damage won't exactly double.
    assert!(damage_2 > damage_1 + 100);

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
            "move|mon:Mew,player-2,1|name:Recover|target:Mew,player-2,1",
            "split|side:1",
            ["heal", "mon:Mew,player-2,1", "health:373/373"],
            ["heal", "mon:Mew,player-2,1", "health:100/100"],
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
            ["damage", "mon:Mew,player-2,1", "health:7/373"],
            ["damage", "mon:Mew,player-2,1", "health:2/100"],
            "residual",
            "turn|turn:6"
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

    let hp_start = 373;
    let hp_after_hex_1 = get_hp(&mut battle, "player-2");
    let damage_1 = hp_start - hp_after_hex_1;

    // Turn 3: Player 2 switches to Comatose Mew.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));

    // Turn 4: Hex on Comatose Mew.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let hp_after_hex_2 = get_hp(&mut battle, "player-2");
    let damage_2 = hp_start - hp_after_hex_2;

    // Comatose makes the Mon essentially always asleep, so Hex should double.
    assert!(damage_2 > damage_1 + 100);

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