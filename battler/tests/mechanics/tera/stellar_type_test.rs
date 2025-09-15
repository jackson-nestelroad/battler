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

fn pikachu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Thunderbolt",
                        "Water Gun",
                        "Flamethrower",
                        "Tera Blast"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "tera_type": "Stellar"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn eevee() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Eevee",
                    "species": "Eevee",
                    "ability": "No Ability",
                    "moves": [
                        "Recover",
                        "Earthquake"
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
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_terastallization(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn stellar_type_uses_original_type_defensively() {
    let mut battle = make_battle(
        0,
        pikachu().unwrap(),
        eevee().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Pikachu,player-1,1|type:Stellar",
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:90/115",
            "damage|mon:Eevee,player-2,1|health:79/100",
            "move|mon:Eevee,player-2,1|name:Earthquake",
            "supereffective|mon:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:0",
            "damage|mon:Pikachu,player-1,1|health:0",
            "faint|mon:Pikachu,player-1,1",
            "reverttera|mon:Pikachu,player-1,1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stellar_type_boosts_moves_once_per_type() {
    let mut battle = make_battle(
        0,
        pikachu().unwrap(),
        eevee().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Pikachu,player-1,1|type:Stellar",
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:90/115",
            "damage|mon:Eevee,player-2,1|health:79/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:69/115",
            "damage|mon:Eevee,player-2,1|health:60/100",
            "move|mon:Eevee,player-2,1|name:Recover|target:Eevee,player-2,1",
            "split|side:1",
            "heal|mon:Eevee,player-2,1|health:115/115",
            "heal|mon:Eevee,player-2,1|health:100/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Thunderbolt|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:49/115",
            "damage|mon:Eevee,player-2,1|health:43/100",
            "move|mon:Eevee,player-2,1|name:Recover|target:Eevee,player-2,1",
            "split|side:1",
            "heal|mon:Eevee,player-2,1|health:107/115",
            "heal|mon:Eevee,player-2,1|health:94/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Thunderbolt|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:58/115",
            "damage|mon:Eevee,player-2,1|health:51/100",
            "move|mon:Eevee,player-2,1|name:Recover|target:Eevee,player-2,1",
            "split|side:1",
            "heal|mon:Eevee,player-2,1|health:115/115",
            "heal|mon:Eevee,player-2,1|health:100/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Water Gun|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:97/115",
            "damage|mon:Eevee,player-2,1|health:85/100",
            "move|mon:Eevee,player-2,1|name:Recover|target:Eevee,player-2,1",
            "split|side:1",
            "heal|mon:Eevee,player-2,1|health:115/115",
            "heal|mon:Eevee,player-2,1|health:100/100",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Water Gun|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:100/115",
            "damage|mon:Eevee,player-2,1|health:87/100",
            "move|mon:Eevee,player-2,1|name:Recover|target:Eevee,player-2,1",
            "split|side:1",
            "heal|mon:Eevee,player-2,1|health:115/115",
            "heal|mon:Eevee,player-2,1|health:100/100",
            "residual",
            "turn|turn:7",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Flamethrower|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:75/115",
            "damage|mon:Eevee,player-2,1|health:66/100",
            "residual",
            "turn|turn:8",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Flamethrower|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:42/115",
            "damage|mon:Eevee,player-2,1|health:37/100",
            "residual",
            "turn|turn:9"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stellar_type_is_super_effective_on_terastallized_targets() {
    let mut battle = make_battle(
        0,
        pikachu().unwrap(),
        eevee().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,tera"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Pikachu,player-1,1|type:Stellar",
            "tera|mon:Eevee,player-2,1|type:Normal",
            "move|mon:Pikachu,player-1,1|name:Tera Blast|target:Eevee,player-2,1",
            "supereffective|mon:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:0",
            "damage|mon:Eevee,player-2,1|health:0",
            "unboost|mon:Pikachu,player-1,1|stat:atk|by:1",
            "unboost|mon:Pikachu,player-1,1|stat:spa|by:1",
            "faint|mon:Eevee,player-2,1",
            "reverttera|mon:Eevee,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
