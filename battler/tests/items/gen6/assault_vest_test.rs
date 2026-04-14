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
                    "name": "Bunnelby",
                    "species": "Bunnelby",
                    "ability": "No Ability",
                    "item": "Assault Vest",
                    "moves": [
                        "Swords Dance",
                        "Water Gun"
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
fn assault_vest_disables_status_moves_and_boosts_special_defense() {
    let mut team_2 = team().unwrap();
    team_2.members[0].item = None;
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Bunnelby's Swords Dance is disabled");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bunnelby,player-1,1|name:Water Gun|target:Bunnelby,player-2,1",
            "split|side:1",
            "damage|mon:Bunnelby,player-2,1|health:155/186",
            "damage|mon:Bunnelby,player-2,1|health:84/100",
            "move|mon:Bunnelby,player-2,1|name:Water Gun|target:Bunnelby,player-1,1",
            "split|side:0",
            "damage|mon:Bunnelby,player-1,1|health:167/186",
            "damage|mon:Bunnelby,player-1,1|health:90/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
