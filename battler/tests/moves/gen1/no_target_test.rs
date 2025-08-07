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
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Overgrow",
                    "moves": ["Tackle", "Air Cutter"],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "Blaze",
                    "moves": ["Scratch"],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 5
                },
                {
                    "name": "Squirtle",
                    "species": "Squirtle",
                    "ability": "Torrent",
                    "moves": ["Tackle"],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 5
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle_builder() -> TestBattleBuilder {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
        .with_seed(0)
        .with_pass_allowed(true)
        .with_team_validation(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
}

fn make_battle(data: &dyn DataStore) -> Result<PublicCoreBattle<'_>> {
    make_battle_builder()
        .with_team("player-1", team()?)
        .with_team("player-2", team()?)
        .build(data)
}

#[test]
fn retargets_foe_after_original_target_faints() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Charmander,player-2,2",
            "split|side:1",
            "damage|mon:Charmander,player-2,2|health:0",
            "damage|mon:Charmander,player-2,2|health:0",
            "faint|mon:Charmander,player-2,2",
            "move|mon:Charmander,player-1,2|name:Scratch|target:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:104/105",
            "damage|mon:Bulbasaur,player-2,1|health:99/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&mut battle, 1, &expected_logs);
}

#[test]
fn move_fails_with_no_target() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Squirtle"],
            ["switch", "player-2", "Squirtle"],
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Bulbasaur,player-1,1|name:Air Cutter|spread:Squirtle,player-2,1;Charmander,player-2,2",
            "crit|mon:Charmander,player-2,2",
            "split|side:1",
            "damage|mon:Squirtle,player-2,1|health:0",
            "damage|mon:Squirtle,player-2,1|health:0",
            "split|side:1",
            "damage|mon:Charmander,player-2,2|health:0",
            "damage|mon:Charmander,player-2,2|health:0",
            "faint|mon:Squirtle,player-2,1",
            "faint|mon:Charmander,player-2,2",
            "move|mon:Charmander,player-1,2|name:Scratch|notarget",
            "fail|mon:Charmander,player-1,2",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
