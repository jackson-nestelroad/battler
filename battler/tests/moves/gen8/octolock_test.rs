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
                    "name": "Grapploct",
                    "species": "Grapploct",
                    "ability": "No Ability",
                    "moves": [
                        "Octolock"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Grapploct",
                    "species": "Grapploct",
                    "ability": "No Ability",
                    "moves": [
                        "Octolock"
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
fn octolock_traps_tsarget_and_lowers_defenses_each_turn() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: Grapploct is trapped", "{err:?}")
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Grapploct,player-1,1|name:Octolock|target:Grapploct,player-2,1",
            "activate|mon:Grapploct,player-2,1|condition:Trapped",
            "start|mon:Grapploct,player-2,1|move:Octolock",
            "unboost|mon:Grapploct,player-2,1|stat:def|by:1|from:move:Octolock",
            "unboost|mon:Grapploct,player-2,1|stat:spd|by:1|from:move:Octolock",
            "residual",
            "turn|turn:2",
            "continue",
            "unboost|mon:Grapploct,player-2,1|stat:def|by:1|from:move:Octolock",
            "unboost|mon:Grapploct,player-2,1|stat:spd|by:1|from:move:Octolock",
            "residual",
            "turn|turn:3",
            "continue",
            "end|mon:Grapploct,player-2,1|move:Octolock|silent",
            "split|side:0",
            "switch|player:player-1|position:1|name:Grapploct|health:270/270|species:Grapploct|level:100|gender:U",
            "switch|player:player-1|position:1|name:Grapploct|health:100/100|species:Grapploct|level:100|gender:U",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
