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

fn weezing() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Weezing",
                    "species": "Weezing",
                    "ability": "No Ability",
                    "item": "Black Sludge",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn staraptor() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Staraptor",
                    "species": "Staraptor",
                    "ability": "No Ability",
                    "item": "Black Sludge",
                    "moves": [
                        "Aerial Ace"
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
fn black_sludge_damages_and_heals_based_on_poison_type() {
    let mut battle = make_battle(0, weezing().unwrap(), staraptor().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Staraptor,player-2,1|name:Aerial Ace|target:Weezing,player-1,1",
            "crit|mon:Weezing,player-1,1",
            "split|side:0",
            "damage|mon:Weezing,player-1,1|health:71/125",
            "damage|mon:Weezing,player-1,1|health:57/100",
            "split|side:1",
            "damage|mon:Staraptor,player-2,1|from:item:Black Sludge|health:127/145",
            "damage|mon:Staraptor,player-2,1|from:item:Black Sludge|health:88/100",
            "split|side:0",
            "heal|mon:Weezing,player-1,1|from:item:Black Sludge|health:78/125",
            "heal|mon:Weezing,player-1,1|from:item:Black Sludge|health:63/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
