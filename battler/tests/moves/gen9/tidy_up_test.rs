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
                    "name": "Maushold",
                    "species": "Maushold",
                    "ability": "No Ability",
                    "moves": [
                        "Substitute",
                        "Spikes",
                        "Stealth Rock",
                        "Sticky Web",
                        "Tidy Up"
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
fn tidy_up_clears_substitute_and_entry_hazards() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Maushold,player-1,1|name:Tidy Up|target:Maushold,player-1,1",
            "boost|mon:Maushold,player-1,1|stat:atk|by:1",
            "boost|mon:Maushold,player-1,1|stat:spe|by:1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Maushold,player-1,1|name:Substitute|target:Maushold,player-1,1",
            "start|mon:Maushold,player-1,1|move:Substitute",
            "split|side:0",
            "damage|mon:Maushold,player-1,1|health:194/258",
            "damage|mon:Maushold,player-1,1|health:76/100",
            "move|mon:Maushold,player-2,1|name:Spikes",
            "sidestart|side:0|move:Spikes|count:1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Maushold,player-1,1|name:Stealth Rock",
            "sidestart|side:1|move:Stealth Rock",
            "move|mon:Maushold,player-2,1|name:Sticky Web",
            "sidestart|side:0|move:Sticky Web",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Maushold,player-1,1|name:Tidy Up|target:Maushold,player-1,1",
            "boost|mon:Maushold,player-1,1|stat:atk|by:1",
            "boost|mon:Maushold,player-1,1|stat:spe|by:1",
            "end|mon:Maushold,player-1,1|move:Substitute",
            "sideend|side:0|move:Spikes",
            "sideend|side:0|move:Sticky Web",
            "sideend|side:1|move:Stealth Rock",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
