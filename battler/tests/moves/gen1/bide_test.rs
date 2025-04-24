use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,

    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blissey",
                    "species": "Blissey",
                    "ability": "No Ability",
                    "moves": [
                        "Bide",
                        "Peck",
                        "Toxic"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Eevee",
                    "species": "Eevee",
                    "ability": "No Ability",
                    "moves": [
                        "Quick Attack"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn bide_deals_double_damage_back_to_last_source() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Doubles,
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;move 0,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blissey,player-1,1|name:Bide|target:Blissey,player-1,1",
            "start|mon:Blissey,player-1,1|move:Bide",
            "move|mon:Eevee,player-2,2|name:Quick Attack|target:Blissey,player-1,1",
            "split|side:0",
            "damage|mon:Blissey,player-1,1|health:212/315",
            "damage|mon:Blissey,player-1,1|health:68/100",
            "move|mon:Blissey,player-2,1|name:Peck|target:Blissey,player-1,1",
            "split|side:0",
            "damage|mon:Blissey,player-1,1|health:197/315",
            "damage|mon:Blissey,player-1,1|health:63/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "activate|move:Bide|mon:Blissey,player-1,1",
            "move|mon:Blissey,player-1,1|name:Bide|target:Blissey,player-1,1",
            "move|mon:Blissey,player-2,1|name:Peck|target:Blissey,player-1,1",
            "split|side:0",
            "damage|mon:Blissey,player-1,1|health:183/315",
            "damage|mon:Blissey,player-1,1|health:59/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "end|mon:Blissey,player-1,1|move:Bide",
            "move|mon:Blissey,player-1,1|name:Bide|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:51/315",
            "damage|mon:Blissey,player-2,1|health:17/100",
            "move|mon:Eevee,player-2,2|name:Quick Attack|target:Blissey,player-1,1",
            "split|side:0",
            "damage|mon:Blissey,player-1,1|health:86/315",
            "damage|mon:Blissey,player-1,1|health:28/100",
            "move|mon:Blissey,player-2,1|name:Peck|target:Blissey,player-1,1",
            "split|side:0",
            "damage|mon:Blissey,player-1,1|health:71/315",
            "damage|mon:Blissey,player-1,1|health:23/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn bide_fails_if_no_damage_is_directly_received() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Singles,
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blissey,player-1,1|name:Bide|target:Blissey,player-1,1",
            "start|mon:Blissey,player-1,1|move:Bide",
            "move|mon:Blissey,player-2,1|name:Toxic|target:Blissey,player-1,1",
            "status|mon:Blissey,player-1,1|status:Bad Poison",
            "split|side:0",
            "damage|mon:Blissey,player-1,1|from:status:Bad Poison|health:296/315",
            "damage|mon:Blissey,player-1,1|from:status:Bad Poison|health:94/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "activate|move:Bide|mon:Blissey,player-1,1",
            "move|mon:Blissey,player-1,1|name:Bide|target:Blissey,player-1,1",
            "split|side:0",
            "damage|mon:Blissey,player-1,1|from:status:Bad Poison|health:257/315",
            "damage|mon:Blissey,player-1,1|from:status:Bad Poison|health:82/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "end|mon:Blissey,player-1,1|move:Bide",
            "move|mon:Blissey,player-1,1|name:Bide|notarget",
            "fail|mon:Blissey,player-1,1",
            "split|side:0",
            "damage|mon:Blissey,player-1,1|from:status:Bad Poison|health:198/315",
            "damage|mon:Blissey,player-1,1|from:status:Bad Poison|health:63/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
