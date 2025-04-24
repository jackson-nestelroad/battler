use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn make_battle(
    data: &dyn DataStore,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(0)
        .with_team_validation(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn drain_moves_heal_a_percent_of_damage_dealt() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team_1: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "No Ability",
                    "moves": [
                        "Mega Drain",
                        "Giga Drain"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 100
                }
            ]
        }"#,
    )
    .unwrap();
    let team_2: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Machamp",
                    "species": "Machamp",
                    "ability": "No Ability",
                    "moves": [
                        "Vital Throw"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 100
                }
            ]
        }"#,
    )
    .unwrap();

    let mut battle = make_battle(&data, team_1, team_2).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Mega Drain|target:Machamp,player-2,1",
            "split|side:1",
            "damage|mon:Machamp,player-2,1|health:232/290",
            "damage|mon:Machamp,player-2,1|health:80/100",
            "move|mon:Machamp,player-2,1|name:Vital Throw|target:Venusaur,player-1,1",
            "resisted|mon:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:201/270",
            "damage|mon:Venusaur,player-1,1|health:75/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Venusaur,player-1,1|name:Giga Drain|target:Machamp,player-2,1",
            "split|side:1",
            "damage|mon:Machamp,player-2,1|health:126/290",
            "damage|mon:Machamp,player-2,1|health:44/100",
            "split|side:0",
            "heal|mon:Venusaur,player-1,1|from:Drain|of:Machamp,player-2,1|health:254/270",
            "heal|mon:Venusaur,player-1,1|from:Drain|of:Machamp,player-2,1|health:95/100",
            "move|mon:Machamp,player-2,1|name:Vital Throw|target:Venusaur,player-1,1",
            "resisted|mon:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:185/270",
            "damage|mon:Venusaur,player-1,1|health:69/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
