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

fn weavile() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Weavile",
                    "species": "Weavile",
                    "ability": "Pickpocket",
                    "moves": [
                        "Tackle"
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
fn pickpocket_steals_attacker_item_on_contact() {
    let mut team = weavile().unwrap();
    team.members[0].item = Some("Toxic Orb".to_owned());
    let mut battle = make_battle(0, team, weavile().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Weavile,player-1,1|name:Tackle|target:Weavile,player-2,1",
            "split|side:1",
            "damage|mon:Weavile,player-2,1|health:98/130",
            "damage|mon:Weavile,player-2,1|health:76/100",
            "itemend|mon:Weavile,player-1,1|item:Toxic Orb|from:ability:Pickpocket|of:Weavile,player-2,1",
            "item|mon:Weavile,player-2,1|item:Toxic Orb|from:ability:Pickpocket",
            "status|mon:Weavile,player-2,1|status:Bad Poison|from:item:Toxic Orb",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
