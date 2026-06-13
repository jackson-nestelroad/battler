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
    assert_logs_since_start_eq,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Iron Treads",
                    "species": "Iron Treads",
                    "ability": "Quark Drive",
                    "moves": [
                        "Electric Terrain",
                        "Grassy Terrain"
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
fn quark_drive_boosts_best_stat_in_electric_terrain() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Iron Treads,player-1,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "activate|mon:Iron Treads,player-1,1|ability:Quark Drive",
            "start|mon:Iron Treads,player-1,1|ability:Quark Drive|stat:def",
            "activate|mon:Iron Treads,player-2,1|ability:Quark Drive",
            "start|mon:Iron Treads,player-2,1|ability:Quark Drive|stat:def",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Iron Treads,player-1,1|name:Grassy Terrain",
            "fieldstart|move:Grassy Terrain",
            "end|mon:Iron Treads,player-1,1|ability:Quark Drive",
            "end|mon:Iron Treads,player-2,1|ability:Quark Drive",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn booster_energy_starts_quark_drive_regardless_of_electric_terrain() {
    let mut team_1 = team().unwrap();
    team_1.members[0].item = Some("Booster Energy".to_owned());
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Iron Treads"],
            ["switch", "player-1", "Iron Treads"],
            "split|side:1",
            ["switch", "player-2", "Iron Treads"],
            ["switch", "player-2", "Iron Treads"],
            "itemend|mon:Iron Treads,player-1,1|item:Booster Energy",
            "activate|mon:Iron Treads,player-1,1|ability:Quark Drive|from:item:Booster Energy",
            "start|mon:Iron Treads,player-1,1|ability:Quark Drive|stat:def",
            "turn|turn:1",
            "continue",
            "move|mon:Iron Treads,player-1,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "activate|mon:Iron Treads,player-2,1|ability:Quark Drive",
            "start|mon:Iron Treads,player-2,1|ability:Quark Drive|stat:def",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Iron Treads,player-1,1|name:Grassy Terrain",
            "fieldstart|move:Grassy Terrain",
            "end|mon:Iron Treads,player-2,1|ability:Quark Drive",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
