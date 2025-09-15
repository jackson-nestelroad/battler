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

fn ninetales() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ninetales",
                    "species": "Ninetales",
                    "ability": "Flash Fire",
                    "moves": [
                        "Flamethrower"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Ninetales",
                    "species": "Ninetales",
                    "ability": "Flash Fire",
                    "moves": [
                        "Flamethrower"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn blastoise() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "Torrent",
                    "moves": [
                        "Ember",
                        "Gastro Acid"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "Neutralizing Gas",
                    "moves": [],
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
fn flash_fire_boosts_attack_after_hit_by_fire_move() {
    let mut battle = make_battle(0, ninetales().unwrap(), blastoise().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ninetales,player-1,1|name:Flamethrower|target:Blastoise,player-2,1",
            "resisted|mon:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:115/139",
            "damage|mon:Blastoise,player-2,1|health:83/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Blastoise,player-2,1|name:Ember|target:Ninetales,player-1,1",
            "start|mon:Ninetales,player-1,1|ability:Flash Fire",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Ninetales,player-1,1|name:Flamethrower|target:Blastoise,player-2,1",
            "resisted|mon:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:79/139",
            "damage|mon:Blastoise,player-2,1|health:57/100",
            "move|mon:Blastoise,player-2,1|name:Ember|target:Ninetales,player-1,1",
            "immune|mon:Ninetales,player-1,1|from:ability:Flash Fire",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flash_fire_gets_suppressed_and_reactivates() {
    let mut battle = make_battle(0, ninetales().unwrap(), blastoise().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blastoise,player-2,1|name:Ember|target:Ninetales,player-1,1",
            "start|mon:Ninetales,player-1,1|ability:Flash Fire",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:1|name:Blastoise|health:139/139|species:Blastoise|level:50|gender:U",
            "switch|player:player-2|position:1|name:Blastoise|health:100/100|species:Blastoise|level:50|gender:U",
            "ability|mon:Blastoise,player-2,1|ability:Neutralizing Gas",
            "end|mon:Ninetales,player-1,1|ability:Flash Fire|silent",
            "move|mon:Ninetales,player-1,1|name:Flamethrower|target:Blastoise,player-2,1",
            "resisted|mon:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:115/139",
            "damage|mon:Blastoise,player-2,1|health:83/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "end|mon:Blastoise,player-2,1|ability:Neutralizing Gas",
            "start|mon:Ninetales,player-1,1|ability:Flash Fire|silent",
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "move|mon:Ninetales,player-1,1|name:Flamethrower|target:Blastoise,player-2,1",
            "resisted|mon:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:103/139",
            "damage|mon:Blastoise,player-2,1|health:75/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Blastoise,player-2,1|name:Gastro Acid|target:Ninetales,player-1,1",
            "abilityend|mon:Ninetales,player-1,1|ability:Flash Fire|from:move:Gastro Acid|of:Blastoise,player-2,1",
            "end|mon:Ninetales,player-1,1|ability:Flash Fire|silent",
            "residual",
            "turn|turn:5",
            ["time"],
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "residual",
            "turn|turn:6",
            ["time"],
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "residual",
            "turn|turn:7",
            ["time"],
            "move|mon:Ninetales,player-1,1|name:Flamethrower|target:Blastoise,player-2,1",
            "resisted|mon:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:79/139",
            "damage|mon:Blastoise,player-2,1|health:57/100",
            "residual",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
