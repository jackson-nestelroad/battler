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
                    "name": "Chesnaught",
                    "species": "Chesnaught",
                    "ability": "No Ability",
                    "moves": [
                        "Rototiller"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Delphox",
                    "species": "Delphox",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Gogoat",
                    "species": "Gogoat",
                    "ability": "No Ability",
                    "moves": [
                        "Magnet Rise"
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
        .with_battle_type(BattleType::Triples)
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
fn rototiller_boosts_attack_of_grounded_grass_types() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0;pass;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;pass;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gogoat,player-1,3|name:Magnet Rise|target:Gogoat,player-1,3",
            "start|mon:Gogoat,player-1,3|move:Magnet Rise",
            "move|mon:Chesnaught,player-1,1|name:Rototiller|spread:Chesnaught,player-1,1;Chesnaught,player-2,1;Gogoat,player-2,3",
            "immune|mon:Gogoat,player-1,3",
            "boost|mon:Chesnaught,player-1,1|stat:atk|by:1",
            "boost|mon:Chesnaught,player-1,1|stat:spa|by:1",
            "boost|mon:Chesnaught,player-2,1|stat:atk|by:1",
            "boost|mon:Chesnaught,player-2,1|stat:spa|by:1",
            "boost|mon:Gogoat,player-2,3|stat:atk|by:1",
            "boost|mon:Gogoat,player-2,3|stat:spa|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
