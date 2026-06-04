use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    Request,
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

fn zacian() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Zacian",
                    "species": "Zacian",
                    "ability": "Intrepid Sword",
                    "item": "Rusted Sword",
                    "moves": [
                        "Tackle",
                        "Iron Head",
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn zamazenta() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Zamazenta",
                    "species": "Zamazenta",
                    "ability": "Dauntless Shield",
                    "item": "Rusted Shield",
                    "moves": [
                        "Tackle",
                        "Iron Head",
                        "Splash"
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
        .with_dynamax(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn zacian_and_zamazenta_transform_with_rusted_items() {
    let mut battle = make_battle(0, zacian().unwrap(), zamazenta().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Zacian cannot dynamax");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,dyna"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Zamazenta cannot dynamax");
    });

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "species:Zacian|"],
            ["switch", "player-1", "species:Zacian|"],
            "split|side:1",
            ["switch", "player-2", "species:Zamazenta|"],
            ["switch", "player-2", "species:Zamazenta|"],
            "split|side:0",
            ["specieschange", "player-1", "species:Zacian-Crowned"],
            ["specieschange", "player-1", "species:Zacian-Crowned"],
            "formechange|mon:Zacian,player-1,1|species:Zacian-Crowned|from:species:Zacian",
            "boost|mon:Zacian,player-1,1|stat:atk|by:1|from:ability:Intrepid Sword",
            "split|side:1",
            ["specieschange", "player-2", "species:Zamazenta-Crowned"],
            ["specieschange", "player-2", "species:Zamazenta-Crowned"],
            "formechange|mon:Zamazenta,player-2,1|species:Zamazenta-Crowned|from:species:Zamazenta",
            "boost|mon:Zamazenta,player-2,1|stat:def|by:1|from:ability:Dauntless Shield",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn iron_head_becomes_behemoth_move_for_zacian_and_zamazenta_crowned_formes() {
    let mut battle = make_battle(0, zacian().unwrap(), zamazenta().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert_eq!(request.active[0].moves[1].name, "Behemoth Blade");
    });
    assert_matches::assert_matches!(battle.request_for_player("player-2"), Ok(Some(Request::Turn(request))) => {
        assert_eq!(request.active[0].moves[1].name, "Behemoth Bash");
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Zacian,player-1,1|name:Behemoth Blade|target:Zamazenta,player-2,1",
            "resisted|mon:Zamazenta,player-2,1",
            "split|side:1",
            "damage|mon:Zamazenta,player-2,1|health:228/294",
            "damage|mon:Zamazenta,player-2,1|health:78/100",
            "move|mon:Zamazenta,player-2,1|name:Behemoth Bash|target:Zacian,player-1,1",
            "split|side:0",
            "damage|mon:Zacian,player-1,1|health:174/294",
            "damage|mon:Zacian,player-1,1|health:60/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
