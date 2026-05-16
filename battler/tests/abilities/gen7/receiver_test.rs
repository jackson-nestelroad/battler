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
                    "name": "Passimian",
                    "species": "Passimian",
                    "ability": "Receiver",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Oranguru",
                    "species": "Oranguru",
                    "ability": "No Ability",
                    "moves": [
                        "Memento"
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
fn receiver_takes_ally_ability_on_faint() {
    let mut team_1 = team().unwrap();
    team_1.members[1].ability = "Speed Boost".to_owned();
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oranguru,player-1,2|name:Memento|target:Passimian,player-2,1",
            "unboost|mon:Passimian,player-2,1|stat:atk|by:2",
            "unboost|mon:Passimian,player-2,1|stat:spa|by:2",
            "faint|mon:Oranguru,player-1,2",
            "abilityend|mon:Passimian,player-1,1|ability:Receiver|from:ability:Receiver",
            "ability|mon:Passimian,player-1,1|ability:Speed Boost|from:ability:Receiver",
            "boost|mon:Passimian,player-1,1|stat:spe|by:1|from:ability:Speed Boost",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn receiver_does_not_activate_on_certain_abilities() {
    let mut team_1 = team().unwrap();
    team_1.members[1].ability = "Comatose".to_owned();
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oranguru,player-1,2|name:Memento|target:Passimian,player-2,1",
            "unboost|mon:Passimian,player-2,1|stat:atk|by:2",
            "unboost|mon:Passimian,player-2,1|stat:spa|by:2",
            "faint|mon:Oranguru,player-1,2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
