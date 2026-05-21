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
                    "name": "Inteleon",
                    "species": "Inteleon",
                    "ability": "No Ability",
                    "moves": [
                        "Snipe Shot",
                        "Ally Switch"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Cinderace",
                    "species": "Cinderace",
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
        .with_battle_type(BattleType::Doubles)
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
fn snipe_shot_tracks_target() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Inteleon,player-1,1|name:Ally Switch|target:Inteleon,player-1,1",
            "swap|mon:Inteleon,player-1,1|position:2|from:move:Ally Switch",
            "move|mon:Inteleon,player-2,1|name:Snipe Shot|target:Cinderace,player-1,1",
            "supereffective|mon:Cinderace,player-1,1",
            "split|side:0",
            "damage|mon:Cinderace,player-1,1|health:0",
            "damage|mon:Cinderace,player-1,1|health:0",
            "faint|mon:Cinderace,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
