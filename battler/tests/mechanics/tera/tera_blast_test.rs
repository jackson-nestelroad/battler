use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    Type,
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
                        "Tera Blast"
                    ],
                    "nature": "Hardy",
                    "level": 50
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
fn tera_blast_changes_to_tera_type() {
    let mut team = pikachu().unwrap();
    team.members[0].tera_type = Some(Type::Fighting);
    let mut battle = make_battle(0, team, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Tera Blast|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:86/115",
            "damage|mon:Eevee,player-2,1|health:75/100",
            "move|mon:Eevee,player-2,1|name:Recover|target:Eevee,player-2,1",
            "split|side:1",
            "heal|mon:Eevee,player-2,1|health:115/115",
            "heal|mon:Eevee,player-2,1|health:100/100",
            "residual",
            "turn|turn:2",
            "continue",
            "tera|mon:Pikachu,player-1,1|type:Fighting",
            "move|mon:Pikachu,player-1,1|name:Tera Blast|target:Eevee,player-2,1",
            "supereffective|mon:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:0",
            "damage|mon:Eevee,player-2,1|health:0",
            "faint|mon:Eevee,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn tera_blast_changes_to_stellar_type() {
    let mut team = pikachu().unwrap();
    team.members[0].tera_type = Some(Type::Stellar);
    let mut battle = make_battle(0, team, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Pikachu,player-1,1|type:Stellar",
            "move|mon:Pikachu,player-1,1|name:Tera Blast|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:55/115",
            "damage|mon:Eevee,player-2,1|health:48/100",
            "unboost|mon:Pikachu,player-1,1|stat:atk|by:1",
            "unboost|mon:Pikachu,player-1,1|stat:spa|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
