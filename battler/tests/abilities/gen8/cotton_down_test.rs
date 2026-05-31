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
                    "name": "Eldegoss",
                    "species": "Eldegoss",
                    "ability": "Cotton Down",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Eldegoss",
                    "species": "Eldegoss",
                    "ability": "Cotton Down",
                    "moves": [
                        "Tackle"
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
        .with_battle_type(BattleType::Doubles)
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
fn cotton_down_decreases_speed_of_all_other_mons_when_hit() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eldegoss,player-1,1|name:Tackle|target:Eldegoss,player-2,1",
            "split|side:1",
            "damage|mon:Eldegoss,player-2,1|health:210/230",
            "damage|mon:Eldegoss,player-2,1|health:92/100",
            "activate|mon:Eldegoss,player-2,1|ability:Cotton Down",
            "unboost|mon:Eldegoss,player-1,1|stat:spe|by:1|from:ability:Cotton Down|of:Eldegoss,player-2,1",
            "unboost|mon:Eldegoss,player-1,2|stat:spe|by:1|from:ability:Cotton Down|of:Eldegoss,player-2,1",
            "unboost|mon:Eldegoss,player-2,2|stat:spe|by:1|from:ability:Cotton Down|of:Eldegoss,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
