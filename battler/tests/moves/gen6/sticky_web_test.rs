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
                    "name": "Slurpuff",
                    "species": "Slurpuff",
                    "ability": "No Ability",
                    "moves": [
                        "Sticky Web",
                        "Rapid Spin"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Talonflame",
                    "species": "Talonflame",
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
fn sticky_web_reduces_speed_on_switch_in() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Slurpuff,player-1,1|name:Sticky Web",
            "sidestart|side:1|move:Sticky Web",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Talonflame"],
            ["switch", "player-2", "Talonflame"],
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Slurpuff"],
            ["switch", "player-2", "Slurpuff"],
            "activate|mon:Slurpuff,player-2,1|move:Sticky Web",
            "unboost|mon:Slurpuff,player-2,1|stat:spe|by:1|from:move:Sticky Web",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Slurpuff,player-2,1|name:Rapid Spin|target:Slurpuff,player-1,1",
            "split|side:0",
            "damage|mon:Slurpuff,player-1,1|health:235/274",
            "damage|mon:Slurpuff,player-1,1|health:86/100",
            "sideend|side:1|move:Sticky Web",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
