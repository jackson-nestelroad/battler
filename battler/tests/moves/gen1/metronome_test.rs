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
    get_controlled_rng_for_battle,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Togepi",
                    "species": "Togepi",
                    "ability": "No Ability",
                    "moves": [
                        "Metronome"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Togepi",
                    "species": "Togepi",
                    "ability": "No Ability",
                    "moves": [
                        "Metronome"
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
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn metronome_uses_random_move() {
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

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 0), (6, 293), (12, 515), (22, 420)]);

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0;move 0"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Togepi,player-1,1|name:Metronome|target:Togepi,player-1,1",
            "move|mon:Togepi,player-1,1|name:Absorb|target:Togepi,player-2,1|from:move:Metronome",
            "split|side:1",
            "damage|mon:Togepi,player-2,1|health:88/95",
            "damage|mon:Togepi,player-2,1|health:93/100",
            "move|mon:Togepi,player-1,2|name:Metronome|target:Togepi,player-1,2",
            "move|mon:Togepi,player-1,2|name:Ice Beam|target:Togepi,player-2,2|from:move:Metronome",
            "split|side:1",
            "damage|mon:Togepi,player-2,2|health:69/95",
            "damage|mon:Togepi,player-2,2|health:73/100",
            "move|mon:Togepi,player-2,1|name:Metronome|target:Togepi,player-2,1",
            "move|mon:Togepi,player-2,1|name:Self-Destruct|from:move:Metronome|spread:Togepi,player-2,2;Togepi,player-1,1;Togepi,player-1,2",
            "split|side:1",
            "damage|mon:Togepi,player-2,2|health:45/95",
            "damage|mon:Togepi,player-2,2|health:48/100",
            "split|side:0",
            "damage|mon:Togepi,player-1,1|health:74/95",
            "damage|mon:Togepi,player-1,1|health:78/100",
            "split|side:0",
            "damage|mon:Togepi,player-1,2|health:71/95",
            "damage|mon:Togepi,player-1,2|health:75/100",
            "faint|mon:Togepi,player-2,1",
            "move|mon:Togepi,player-2,2|name:Metronome|target:Togepi,player-2,2",
            "move|mon:Togepi,player-2,2|name:Pin Missile|target:Togepi,player-1,1|from:move:Metronome",
            "resisted|mon:Togepi,player-1,1",
            "split|side:0",
            "damage|mon:Togepi,player-1,1|health:72/95",
            "damage|mon:Togepi,player-1,1|health:76/100",
            "animatemove|mon:Togepi,player-2,2|name:Pin Missile|target:Togepi,player-1,1",
            "resisted|mon:Togepi,player-1,1",
            "split|side:0",
            "damage|mon:Togepi,player-1,1|health:70/95",
            "damage|mon:Togepi,player-1,1|health:74/100",
            "animatemove|mon:Togepi,player-2,2|name:Pin Missile|target:Togepi,player-1,1",
            "resisted|mon:Togepi,player-1,1",
            "split|side:0",
            "damage|mon:Togepi,player-1,1|health:68/95",
            "damage|mon:Togepi,player-1,1|health:72/100",
            "hitcount|hits:3",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
