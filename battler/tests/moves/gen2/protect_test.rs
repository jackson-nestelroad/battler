use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
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

fn pikachu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Protect",
                        "Quick Attack"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn aggron() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Aggron",
                    "species": "Aggron",
                    "ability": "No Ability",
                    "moves": [
                        "Protect",
                        "Outrage"
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
) -> Result<PublicCoreBattle> {
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
fn protect_gives_user_invulnerability_with_decreasing_chance() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, pikachu().unwrap(), pikachu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Protect|noanim",
            "fail|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Protect|target:Pikachu,player-1,1",
            "singleturn|mon:Pikachu,player-1,1|move:Protect",
            "move|mon:Pikachu,player-2,1|name:Quick Attack|target:Pikachu,player-1,1",
            "activate|move:Protect",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Protect|target:Pikachu,player-1,1",
            "singleturn|mon:Pikachu,player-1,1|move:Protect",
            "move|mon:Pikachu,player-2,1|name:Quick Attack|target:Pikachu,player-1,1",
            "activate|move:Protect",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Protect|noanim",
            "fail|mon:Pikachu,player-1,1",
            "move|mon:Pikachu,player-2,1|name:Quick Attack|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:70/95",
            "damage|mon:Pikachu,player-1,1|health:74/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn protect_cancels_outrage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, aggron().unwrap(), aggron().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Aggron,player-2,1|name:Outrage|target:Aggron,player-1,1",
            "resisted|mon:Aggron,player-1,1",
            "split|side:0",
            "damage|mon:Aggron,player-1,1|health:114/130",
            "damage|mon:Aggron,player-1,1|health:88/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Aggron,player-1,1|name:Protect|target:Aggron,player-1,1",
            "singleturn|mon:Aggron,player-1,1|move:Protect",
            "move|mon:Aggron,player-2,1|name:Outrage|target:Aggron,player-1,1",
            "activate|move:Protect",
            "start|mon:Aggron,player-2,1|condition:Confusion|fatigue",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
