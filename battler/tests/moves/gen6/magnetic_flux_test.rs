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
                    "name": "Magearna",
                    "species": "Magearna",
                    "ability": "No Ability",
                    "moves": [
                        "Magnetic Flux"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Klink",
                    "species": "Klink",
                    "ability": "Plus",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Klink",
                    "species": "Klink",
                    "ability": "Minus",
                    "moves": [
                        "Magnetic Flux"
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
fn magnetic_flux_boosts_stats_of_user_and_allies_with_plus_or_minus() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0;pass;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;pass;move 0"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Magearna,player-1,1|name:Magnetic Flux|spread:Klink,player-1,2;Klink,player-1,3",
            "boost|mon:Klink,player-1,2|stat:def|by:1",
            "boost|mon:Klink,player-1,2|stat:spd|by:1",
            "boost|mon:Klink,player-1,3|stat:def|by:1",
            "boost|mon:Klink,player-1,3|stat:spd|by:1",
            "move|mon:Klink,player-2,3|name:Magnetic Flux|spread:Klink,player-2,2;Klink,player-2,3",
            "boost|mon:Klink,player-2,2|stat:def|by:1",
            "boost|mon:Klink,player-2,2|stat:spd|by:1",
            "boost|mon:Klink,player-2,3|stat:def|by:1",
            "boost|mon:Klink,player-2,3|stat:spd|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
