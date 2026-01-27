use anyhow::Result;
use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    teams::TeamData,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn team() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Reuniclus",
                    "species": "Reuniclus",
                    "ability": "Magic Guard",
                    "moves": [
                        "Heal Pulse",
                        "Shadow Ball"
                    ],
                    "nature": "Modest",
                    "level": 50,
                    "evs": {
                        "hp": 252,
                        "spa": 252
                    }
                },
                {
                    "name": "Clawitzer",
                    "species": "Clawitzer",
                    "ability": "Mega Launcher",
                    "moves": [
                        "Heal Pulse",
                        "Dark Pulse"
                    ],
                    "nature": "Modest",
                    "level": 50,
                    "evs": {
                        "hp": 252,
                        "spa": 252
                    }
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_pass_allowed(true)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn heal_pulse_heals_half_max_hp() {
    let mut battle = make_battle(0, team(), team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Player 1 attacks Player 2 with Shadow Ball.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1, 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Player 1 uses Heal Pulse on Player 2.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0, 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Verify logs.
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Reuniclus,player-1,1|name:Shadow Ball|target:Reuniclus,player-2,1",
            "supereffective|mon:Reuniclus,player-2,1",
            "split|side:1",
            "damage|mon:Reuniclus,player-2,1|health:65/201",
            "damage|mon:Reuniclus,player-2,1|health:33/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Reuniclus,player-1,1|name:Heal Pulse|target:Reuniclus,player-2,1",
            "split|side:1",
            "heal|mon:Reuniclus,player-2,1|health:165/201",
            "heal|mon:Reuniclus,player-2,1|health:83/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mega_launcher_boosts_heal_pulse() {
    let mut battle = make_battle(0, team(), team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Player 1 switches to Clawitzer.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    // Turn 2: Player 1 attacks Player 2 with Dark Pulse.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1, 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 3: Player 1 uses Heal Pulse.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0, 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Verify logs.
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            "switch|player:player-1|position:1|name:Clawitzer|health:162/162|species:Clawitzer|level:50|gender:U",
            "switch|player:player-1|position:1|name:Clawitzer|health:100/100|species:Clawitzer|level:50|gender:U",
            "split|side:1",
            "switch|player:player-2|position:1|name:Clawitzer|health:162/162|species:Clawitzer|level:50|gender:U",
            "switch|player:player-2|position:1|name:Clawitzer|health:100/100|species:Clawitzer|level:50|gender:U",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Clawitzer,player-1,1|name:Dark Pulse|target:Clawitzer,player-2,1",
            "split|side:1",
            "damage|mon:Clawitzer,player-2,1|health:67/162",
            "damage|mon:Clawitzer,player-2,1|health:42/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Clawitzer,player-1,1|name:Heal Pulse|target:Clawitzer,player-2,1",
            "split|side:1",
            "heal|mon:Clawitzer,player-2,1|health:162/162",
            "heal|mon:Clawitzer,player-2,1|health:100/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn heal_pulse_fails_at_full_hp() {
    let mut battle = make_battle(0, team(), team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Player 1 uses Heal Pulse on Player 2 (Full HP).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0, 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Reuniclus,player-1,1|name:Heal Pulse|noanim",
            "fail|mon:Reuniclus,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
