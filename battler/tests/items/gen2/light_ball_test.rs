use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    Error,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn pikachu() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "Static",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn raichu() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Raichu",
                    "species": "Raichu",
                    "ability": "Static",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn light_ball_boosts_pikachu_attack_stats() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = pikachu().unwrap();
    team.members[0].item = Some("Light Ball".to_owned());
    let mut battle = make_battle(&data, 0, team, pikachu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:49/95",
            "damage|mon:Pikachu,player-2,1|health:52/100",
            "move|mon:Pikachu,player-2,1|name:Tackle|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:73/95",
            "damage|mon:Pikachu,player-1,1|health:77/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn light_ball_works_with_alternative_pikachu_formes() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = pikachu().unwrap();
    team.members[0].species = "Pikachu-Hoenn-Cap".to_owned();
    team.members[0].item = Some("Light Ball".to_owned());
    let mut battle = make_battle(&data, 0, team, pikachu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:49/95",
            "damage|mon:Pikachu,player-2,1|health:52/100",
            "move|mon:Pikachu,player-2,1|name:Tackle|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:73/95",
            "damage|mon:Pikachu,player-1,1|health:77/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn light_ball_does_not_work_on_non_pikachu() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = raichu().unwrap();
    team.members[0].item = Some("Light Ball".to_owned());
    let mut battle = make_battle(&data, 0, team, raichu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Raichu,player-1,1|name:Tackle|target:Raichu,player-2,1",
            "split|side:1",
            "damage|mon:Raichu,player-2,1|health:92/120",
            "damage|mon:Raichu,player-2,1|health:77/100",
            "move|mon:Raichu,player-2,1|name:Tackle|target:Raichu,player-1,1",
            "split|side:0",
            "damage|mon:Raichu,player-1,1|health:94/120",
            "damage|mon:Raichu,player-1,1|health:79/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
