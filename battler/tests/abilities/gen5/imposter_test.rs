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

fn ditto() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ditto",
                    "species": "Ditto",
                    "ability": "Imposter",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn reshiram() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Reshiram",
                    "species": "Reshiram",
                    "ability": "No Ability",
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
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn imposter_transforms_on_switch_in() {
    let mut battle = make_battle(0, ditto().unwrap(), reshiram().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            "switch|player:player-1|position:1|name:Ditto|health:108/108|species:Ditto|level:50|gender:U",
            "switch|player:player-1|position:1|name:Ditto|health:100/100|species:Ditto|level:50|gender:U",
            "split|side:1",
            "switch|player:player-2|position:1|name:Reshiram|health:160/160|species:Reshiram|level:50|gender:U",
            "switch|player:player-2|position:1|name:Reshiram|health:100/100|species:Reshiram|level:50|gender:U",
            "transform|mon:Ditto,player-1,1|into:Reshiram,player-2,1|species:Reshiram|from:ability:Imposter",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
