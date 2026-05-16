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
    assert_logs_since_start_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Wishiwashi",
                    "species": "Wishiwashi",
                    "ability": "Schooling",
                    "moves": [
                        "Recover",
                        "Thunderbolt"
                    ],
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn schooling_transforms_wishiwashi_based_on_hp() {
    let mut team_1 = team().unwrap();
    team_1.members[0].persistent_battle_data.hp = Some(50);
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Wishiwashi"],
            ["switch", "player-1", "Wishiwashi"],
            "split|side:1",
            ["switch", "player-2", "Wishiwashi"],
            ["switch", "player-2", "Wishiwashi"],
            "formechange|mon:Wishiwashi,player-2,1|species:Wishiwashi-School|from:ability:Schooling",
            "turn|turn:1",
            "continue",
            "move|mon:Wishiwashi,player-1,1|name:Recover|target:Wishiwashi,player-1,1",
            "split|side:0",
            "heal|mon:Wishiwashi,player-1,1|health:150/200",
            "heal|mon:Wishiwashi,player-1,1|health:75/100",
            "formechange|mon:Wishiwashi,player-1,1|species:Wishiwashi-School|from:ability:Schooling",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Wishiwashi,player-1,1|name:Thunderbolt|target:Wishiwashi,player-2,1",
            "supereffective|mon:Wishiwashi,player-2,1",
            "split|side:1",
            "damage|mon:Wishiwashi,player-2,1|health:46/200",
            "damage|mon:Wishiwashi,player-2,1|health:23/100",
            "formechange|mon:Wishiwashi,player-2,1|species:Wishiwashi|from:ability:Schooling",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
