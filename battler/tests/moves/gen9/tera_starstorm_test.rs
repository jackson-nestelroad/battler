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
                    "name": "Terapagos",
                    "species": "Terapagos",
                    "ability": "Tera Shift",
                    "moves": [
                        "Tera Starstorm"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Pecharunt",
                    "species": "Pecharunt",
                    "ability": "No Ability",
                    "moves": [
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
        .with_battle_type(BattleType::Doubles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_terastallization(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn tera_starstorm_targets_all_adjacent_foes_when_terapagos_terastallizes() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1,tera;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,tera"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Terapagos,player-1,1|name:Tera Starstorm|target:Terapagos,player-2,1",
            "activate|mon:Terapagos,player-2,1|ability:Tera Shell",
            "resisted|mon:Terapagos,player-2,1",
            "split|side:1",
            "damage|mon:Terapagos,player-2,1|health:229/300",
            "damage|mon:Terapagos,player-2,1|health:77/100",
            "residual",
            "turn|turn:2",
            "continue",
            "tera|mon:Terapagos,player-1,1|type:Stellar",
            "split|side:0",
            ["specieschange", "player-1", "Terapagos-Stellar"],
            ["specieschange", "player-1", "Terapagos-Stellar"],
            "formechange|mon:Terapagos,player-1,1|species:Terapagos-Stellar|from:species:Terapagos-Terastal",
            "tera|mon:Pecharunt,player-2,2|type:Poison",
            "move|mon:Pecharunt,player-2,2|name:Splash|target:Pecharunt,player-2,2",
            "activate|move:Splash",
            "move|mon:Terapagos,player-1,1|name:Tera Starstorm|spread:Terapagos,player-2,1;Pecharunt,player-2,2",
            "supereffective|mon:Pecharunt,player-2,2",
            "split|side:1",
            "damage|mon:Terapagos,player-2,1|health:132/300",
            "damage|mon:Terapagos,player-2,1|health:44/100",
            "split|side:1",
            "damage|mon:Pecharunt,player-2,2|health:58/286",
            "damage|mon:Pecharunt,player-2,2|health:21/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
