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
                    "name": "Grumpig",
                    "species": "Grumpig",
                    "ability": "Magic Bounce",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Seviper",
                    "species": "Seviper",
                    "ability": "No Ability",
                    "moves": [
                        "Will-O-Wisp",
                        "Spikes"
                    ],
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
fn magic_bounce_reflects_status_moves() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Seviper,player-2,2|name:Will-O-Wisp|noanim",
            "activate|mon:Grumpig,player-1,1|ability:Magic Bounce",
            "move|mon:Grumpig,player-1,1|name:Will-O-Wisp|target:Seviper,player-2,2|from:ability:Magic Bounce",
            "status|mon:Seviper,player-2,2|status:Burn",
            "split|side:1",
            "damage|mon:Seviper,player-2,2|from:status:Burn|health:125/133",
            "damage|mon:Seviper,player-2,2|from:status:Burn|health:94/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Seviper,player-2,2|name:Spikes|noanim",
            "activate|mon:Grumpig,player-1,1|ability:Magic Bounce",
            "move|mon:Grumpig,player-1,1|name:Spikes|from:ability:Magic Bounce",
            "sidestart|side:1|move:Spikes|count:1",
            "split|side:1",
            "damage|mon:Seviper,player-2,2|from:status:Burn|health:117/133",
            "damage|mon:Seviper,player-2,2|from:status:Burn|health:88/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn magic_bounce_cannot_reflect_reflected_move() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Seviper,player-2,2|name:Spikes|noanim",
            "activate|mon:Grumpig,player-1,1|ability:Magic Bounce",
            "move|mon:Grumpig,player-1,1|name:Spikes|from:ability:Magic Bounce",
            "sidestart|side:1|move:Spikes|count:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
