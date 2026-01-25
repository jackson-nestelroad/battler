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

fn dragonite() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Dragonite",
                    "species": "Dragonite",
                    "ability": "Multiscale",
                    "moves": [
                        "Dragon Claw"
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
fn multiscale_reduces_damage_at_max_hp() {
    let mut team = dragonite().unwrap();
    team.members[0].persistent_battle_data.hp = Some(150);
    let mut battle = make_battle(0, team, dragonite().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Dragonite,player-1,1|name:Dragon Claw|target:Dragonite,player-2,1",
            "supereffective|mon:Dragonite,player-2,1",
            "split|side:1",
            "damage|mon:Dragonite,player-2,1|health:79/151",
            "damage|mon:Dragonite,player-2,1|health:53/100",
            "move|mon:Dragonite,player-2,1|name:Dragon Claw|target:Dragonite,player-1,1",
            "supereffective|mon:Dragonite,player-1,1",
            "split|side:0",
            "damage|mon:Dragonite,player-1,1|health:16/151",
            "damage|mon:Dragonite,player-1,1|health:11/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
