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

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Eternatus",
                    "species": "Eternatus-Eternamax",
                    "ability": "No Ability",
                    "moves": [
                        "Eternabeam",
                        "Protect",
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "level": 100,
                    "tera_type": "Fairy"
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
        .with_terastallization(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn eternabeam_requires_recharge() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eternatus,player-1,1|name:Eternabeam|target:Eternatus,player-2,1",
            "supereffective|mon:Eternatus,player-2,1",
            "split|side:1",
            "damage|mon:Eternatus,player-2,1|health:422/620",
            "damage|mon:Eternatus,player-2,1|health:69/100",
            "activate|mon:Eternatus,player-1,1|condition:Must Recharge",
            "residual",
            "turn|turn:2",
            "continue",
            "cant|mon:Eternatus,player-1,1|from:Must Recharge",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn eternabeam_does_not_require_recharge_due_to_missing() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eternatus,player-2,1|name:Protect|target:Eternatus,player-2,1",
            "singleturn|mon:Eternatus,player-2,1|move:Protect",
            "move|mon:Eternatus,player-1,1|name:Eternabeam|noanim",
            "activate|mon:Eternatus,player-2,1|move:Protect",
            "residual",
            "turn|turn:2",
            "continue",
            "tera|mon:Eternatus,player-2,1|type:Fairy",
            "move|mon:Eternatus,player-1,1|name:Eternabeam|noanim",
            "immune|mon:Eternatus,player-2,1",
            "move|mon:Eternatus,player-2,1|name:Splash|target:Eternatus,player-2,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Eternatus,player-1,1|name:Eternabeam|noanim",
            "immune|mon:Eternatus,player-2,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
