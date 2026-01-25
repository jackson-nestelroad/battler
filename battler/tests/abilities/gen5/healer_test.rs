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

fn alomomola() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Alomomola",
                    "species": "Alomomola",
                    "ability": "Healer",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn darkrai() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Darkrai",
                    "species": "Darkrai",
                    "ability": "No Ability",
                    "moves": [
                        "Dark Void"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn triples_team(team: TeamData) -> TeamData {
    let mut team = team.clone();
    team.members.push(team.members[0].clone());
    team.members.push(team.members[0].clone());
    team
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Triples)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn healer_has_chance_to_cure_ally_status_conditions() {
    let mut battle = make_battle(
        937982581307,
        triples_team(alomomola().unwrap()),
        triples_team(darkrai().unwrap()),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;pass;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Darkrai,player-2,2|name:Dark Void|spread:Alomomola,player-1,1;Alomomola,player-1,3",
            "miss|mon:Alomomola,player-1,2",
            "status|mon:Alomomola,player-1,1|status:Sleep",
            "status|mon:Alomomola,player-1,3|status:Sleep",
            "activate|mon:Alomomola,player-1,2|ability:Healer",
            "curestatus|mon:Alomomola,player-1,1|status:Sleep|from:ability:Healer|of:Alomomola,player-1,2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
