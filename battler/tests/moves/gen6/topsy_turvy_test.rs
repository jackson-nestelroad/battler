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
                    "name": "Malamar",
                    "species": "Malamar",
                    "ability": "No Ability",
                    "moves": [
                        "Topsy-Turvy",
                        "Swords Dance",
                        "Scary Face",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Serperior",
                    "species": "Serperior",
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
fn topsy_turvy_inverts_boosts() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Malamar,player-1,1|name:Topsy-Turvy|noanim",
            "fail|mon:Malamar,player-1,1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Malamar,player-1,1|name:Scary Face|target:Malamar,player-2,1",
            "unboost|mon:Malamar,player-2,1|stat:spe|by:2",
            "move|mon:Malamar,player-2,1|name:Swords Dance|target:Malamar,player-2,1",
            "boost|mon:Malamar,player-2,1|stat:atk|by:2",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Malamar,player-1,1|name:Topsy-Turvy|target:Malamar,player-2,1",
            "invertboosts|mon:Malamar,player-2,1|from:move:Topsy-Turvy|of:Malamar,player-1,1",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Malamar,player-2,1|name:Tackle|target:Malamar,player-1,1",
            "split|side:0",
            "damage|mon:Malamar,player-1,1|health:264/282",
            "damage|mon:Malamar,player-1,1|health:94/100",
            "move|mon:Malamar,player-1,1|name:Tackle|target:Malamar,player-2,1",
            "split|side:1",
            "damage|mon:Malamar,player-2,1|health:249/282",
            "damage|mon:Malamar,player-2,1|health:89/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
