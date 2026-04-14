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
                    "name": "Klefki",
                    "species": "Klefki",
                    "ability": "No Ability",
                    "moves": [
                        "Crafty Shield",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Sylveon",
                    "species": "Sylveon",
                    "ability": "No Ability",
                    "moves": [
                        "Thunder Wave",
                        "Perish Song"
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
fn crafty_shield_protects_side_against_status_moves() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;move 0,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Klefki,player-1,1|name:Crafty Shield",
            "singleturn|mon:Klefki,player-1,1|move:Crafty Shield",
            "move|mon:Klefki,player-2,1|name:Tackle|target:Sylveon,player-1,2",
            "split|side:0",
            "damage|mon:Sylveon,player-1,2|health:259/300",
            "damage|mon:Sylveon,player-1,2|health:87/100",
            "move|mon:Sylveon,player-2,2|name:Thunder Wave|noanim",
            "activate|mon:Klefki,player-1,1|move:Crafty Shield",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn crafty_shield_does_not_protect_against_all_targeting_moves() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Klefki,player-1,1|name:Crafty Shield",
            "singleturn|mon:Klefki,player-1,1|move:Crafty Shield",
            "move|mon:Sylveon,player-2,2|name:Perish Song|spread:Klefki,player-2,1;Sylveon,player-2,2;Klefki,player-1,1;Sylveon,player-1,2",
            "fieldactivate|move:Perish Song",
            "start|mon:Klefki,player-2,1|move:Perish Song|perish:3",
            "start|mon:Klefki,player-1,1|move:Perish Song|perish:3",
            "start|mon:Sylveon,player-2,2|move:Perish Song|perish:3",
            "start|mon:Sylveon,player-1,2|move:Perish Song|perish:3",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
