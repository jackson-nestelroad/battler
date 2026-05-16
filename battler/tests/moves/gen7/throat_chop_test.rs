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
                    "name": "Incineroar",
                    "species": "Incineroar",
                    "ability": "No Ability",
                    "item": "Normalium Z",
                    "moves": [
                        "Throat Chop",
                        "Growl"
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
        .with_z_moves(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn throat_chop_prevents_sounds_moves() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Incineroar,player-1,1|name:Throat Chop|target:Incineroar,player-2,1",
            "resisted|mon:Incineroar,player-2,1",
            "split|side:1",
            "damage|mon:Incineroar,player-2,1|health:237/300",
            "damage|mon:Incineroar,player-2,1|health:79/100",
            "start|mon:Incineroar,player-2,1|move:Throat Chop|silent",
            "cant|mon:Incineroar,player-2,1|from:move:Throat Chop",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn throat_chop_does_not_prevent_z_sound_move() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1,zmove"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Incineroar,player-1,1|name:Throat Chop|target:Incineroar,player-2,1",
            "resisted|mon:Incineroar,player-2,1",
            "split|side:1",
            "damage|mon:Incineroar,player-2,1|health:237/300",
            "damage|mon:Incineroar,player-2,1|health:79/100",
            "start|mon:Incineroar,player-2,1|move:Throat Chop|silent",
            "singleturn|mon:Incineroar,player-2,1|condition:Z-Power",
            "move|mon:Incineroar,player-2,1|name:Growl|zpower",
            "boost|mon:Incineroar,player-2,1|stat:def|by:1|from:Z-Power",
            "unboost|mon:Incineroar,player-1,1|stat:atk|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
