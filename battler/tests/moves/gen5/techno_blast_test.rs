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
    static_local_data_store,
};

fn team_1() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Genesect",
                    "species": "Genesect",
                    "ability": "No Ability",
                    "moves": [
                        "Techno Blast"
                    ],
                    "nature": "Hardy",
                    "level": 20
                },
                {
                    "name": "Genesect-Douse",
                    "species": "Genesect",
                    "ability": "No Ability",
                    "moves": [
                        "Techno Blast"
                    ],
                    "nature": "Hardy",
                    "level": 20,
                    "item": "Douse Drive"
                },
                {
                    "name": "Genesect-Shock",
                    "species": "Genesect",
                    "ability": "No Ability",
                    "moves": [
                        "Techno Blast"
                    ],
                    "nature": "Hardy",
                    "level": 20,
                    "item": "Shock Drive"
                },
                {
                    "name": "Genesect-Burn",
                    "species": "Genesect",
                    "ability": "No Ability",
                    "moves": [
                        "Techno Blast"
                    ],
                    "nature": "Hardy",
                    "level": 20,
                    "item": "Burn Drive"
                },
                {
                    "name": "Genesect-Chill",
                    "species": "Genesect",
                    "ability": "No Ability",
                    "moves": [
                        "Techno Blast"
                    ],
                    "nature": "Hardy",
                    "level": 20,
                    "item": "Chill Drive"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn team_2() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Dusclops",
                    "species": "Dusclops",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Geodude",
                    "species": "Geodude",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Paras",
                    "species": "Paras",
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
        .with_base_damage_randomization(battler::CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn techno_blast_drive_types_test() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Normal vs Dusclops (Ghost). Immune.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Switch to Douse Drive Genesect. Switch to Geodude.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    // Turn 3: Water vs Geodude (Rock/Ground). Super effective (4x).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 4: Switch to Shock Drive Genesect.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 5: Electric vs Geodude (Rock/Ground). Immune.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 6: Switch to Burn Drive Genesect. Switch to Paras.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));

    // Turn 7: Fire vs Paras (Bug/Grass). Super effective (4x).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 8: Switch to Chill Drive Genesect.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 9: Ice vs Paras (Bug/Grass). Super effective (2x).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Genesect,player-1,1|name:Techno Blast|noanim",
            "immune|mon:Dusclops,player-2,1",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:1",
            ["switch", "player:player-2", "name:Geodude"],
            ["switch", "player:player-2", "name:Geodude"],
            "split|side:0",
            ["switch", "player:player-1", "name:Genesect-Douse"],
            ["switch", "player:player-1", "name:Genesect-Douse"],
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Genesect-Douse,player-1,1|name:Techno Blast|target:Geodude,player-2,1",
            "supereffective|mon:Geodude,player-2,1",
            "split|side:1",
            "damage|mon:Geodude,player-2,1|health:106/190",
            "damage|mon:Geodude,player-2,1|health:56/100",
            "residual",
            "turn|turn:4",
            "continue",
            "split|side:0",
            ["switch", "player:player-1", "name:Genesect-Shock"],
            ["switch", "player:player-1", "name:Genesect-Shock"],
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Genesect-Shock,player-1,1|name:Techno Blast|noanim",
            "immune|mon:Geodude,player-2,1",
            "residual",
            "turn|turn:6",
            "continue",
            "split|side:1",
            ["switch", "player:player-2", "name:Paras"],
            ["switch", "player:player-2", "name:Paras"],
            "split|side:0",
            ["switch", "player:player-1", "name:Genesect-Burn"],
            ["switch", "player:player-1", "name:Genesect-Burn"],
            "residual",
            "turn|turn:7",
            "continue",
            "move|mon:Genesect-Burn,player-1,1|name:Techno Blast|target:Paras,player-2,1",
            "supereffective|mon:Paras,player-2,1",
            "split|side:1",
            "damage|mon:Paras,player-2,1|health:128/180",
            "damage|mon:Paras,player-2,1|health:72/100",
            "residual",
            "turn|turn:8",
            "continue",
            "split|side:0",
            ["switch", "player:player-1", "name:Genesect-Chill"],
            ["switch", "player:player-1", "name:Genesect-Chill"],
            "residual",
            "turn|turn:9",
            "continue",
            "move|mon:Genesect-Chill,player-1,1|name:Techno Blast|target:Paras,player-2,1",
            "supereffective|mon:Paras,player-2,1",
            "split|side:1",
            "damage|mon:Paras,player-2,1|health:102/180",
            "damage|mon:Paras,player-2,1|health:57/100",
            "residual",
            "turn|turn:10"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
