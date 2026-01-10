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

fn staraptor() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Staraptor",
                    "species": "Staraptor",
                    "ability": "No Ability",
                    "item": "Metronome",
                    "moves": [
                        "Tackle",
                        "Pound",
                        "Fly",
                        "Recover"
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
fn metronome_increases_damage_of_consecutive_moves() {
    let mut battle = make_battle(0, staraptor().unwrap(), staraptor().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Staraptor,player-1,1|name:Tackle|target:Staraptor,player-2,1",
            "split|side:1",
            "damage|mon:Staraptor,player-2,1|health:99/145",
            "damage|mon:Staraptor,player-2,1|health:69/100",
            "move|mon:Staraptor,player-2,1|name:Recover|target:Staraptor,player-2,1",
            "split|side:1",
            "heal|mon:Staraptor,player-2,1|health:145/145",
            "heal|mon:Staraptor,player-2,1|health:100/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Staraptor,player-1,1|name:Tackle|target:Staraptor,player-2,1",
            "split|side:1",
            "damage|mon:Staraptor,player-2,1|health:90/145",
            "damage|mon:Staraptor,player-2,1|health:63/100",
            "move|mon:Staraptor,player-2,1|name:Recover|target:Staraptor,player-2,1",
            "split|side:1",
            "heal|mon:Staraptor,player-2,1|health:145/145",
            "heal|mon:Staraptor,player-2,1|health:100/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Staraptor,player-1,1|name:Tackle|target:Staraptor,player-2,1",
            "split|side:1",
            "damage|mon:Staraptor,player-2,1|health:81/145",
            "damage|mon:Staraptor,player-2,1|health:56/100",
            "move|mon:Staraptor,player-2,1|name:Recover|target:Staraptor,player-2,1",
            "split|side:1",
            "heal|mon:Staraptor,player-2,1|health:145/145",
            "heal|mon:Staraptor,player-2,1|health:100/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Staraptor,player-1,1|name:Pound|target:Staraptor,player-2,1",
            "split|side:1",
            "damage|mon:Staraptor,player-2,1|health:99/145",
            "damage|mon:Staraptor,player-2,1|health:69/100",
            "move|mon:Staraptor,player-2,1|name:Recover|target:Staraptor,player-2,1",
            "split|side:1",
            "heal|mon:Staraptor,player-2,1|health:145/145",
            "heal|mon:Staraptor,player-2,1|health:100/100",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Staraptor,player-1,1|name:Tackle|target:Staraptor,player-2,1",
            "split|side:1",
            "damage|mon:Staraptor,player-2,1|health:99/145",
            "damage|mon:Staraptor,player-2,1|health:69/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn metronome_increases_damage_of_consecutive_two_turn_moves() {
    let mut battle = make_battle(0, staraptor().unwrap(), staraptor().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Staraptor,player-1,1|name:Fly|noanim",
            "prepare|mon:Staraptor,player-1,1|move:Fly",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Staraptor,player-1,1|name:Fly|target:Staraptor,player-2,1",
            "split|side:1",
            "damage|mon:Staraptor,player-2,1|health:23/145",
            "damage|mon:Staraptor,player-2,1|health:16/100",
            "move|mon:Staraptor,player-2,1|name:Recover|target:Staraptor,player-2,1",
            "split|side:1",
            "heal|mon:Staraptor,player-2,1|health:96/145",
            "heal|mon:Staraptor,player-2,1|health:67/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Staraptor,player-1,1|name:Fly|noanim",
            "prepare|mon:Staraptor,player-1,1|move:Fly",
            "move|mon:Staraptor,player-2,1|name:Recover|target:Staraptor,player-2,1",
            "split|side:1",
            "heal|mon:Staraptor,player-2,1|health:145/145",
            "heal|mon:Staraptor,player-2,1|health:100/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Staraptor,player-1,1|name:Fly|target:Staraptor,player-2,1",
            "split|side:1",
            "damage|mon:Staraptor,player-2,1|health:0",
            "damage|mon:Staraptor,player-2,1|health:0",
            "faint|mon:Staraptor,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
