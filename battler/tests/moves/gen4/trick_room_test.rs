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

fn porygonz() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Porygon-Z",
                    "species": "Porygon-Z",
                    "ability": "No Ability",
                    "moves": [
                        "Trick Room",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn deoxys() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Deoxys",
                    "species": "Deoxys-Speed",
                    "ability": "No Ability",
                    "item": "Choice Scarf",
                    "moves": [
                        "Quick Attack",
                        "Tackle",
                        "Agility"
                    ],
                    "nature": "Jolly",
                    "level": 100,
                    "ivs": {
                        "spe": 31
                    },
                    "evs": {
                        "spe": 255
                    }
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
fn trick_room_reverses_speed_order() {
    let mut battle = make_battle(0, porygonz().unwrap(), deoxys().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Porygon-Z,player-1,1|name:Trick Room",
            "fieldstart|move:Trick Room",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Porygon-Z,player-1,1|name:Tackle|target:Deoxys,player-2,1",
            "split|side:1",
            "damage|mon:Deoxys,player-2,1|health:165/210",
            "damage|mon:Deoxys,player-2,1|health:79/100",
            "move|mon:Deoxys,player-2,1|name:Tackle|target:Porygon-Z,player-1,1",
            "split|side:0",
            "damage|mon:Porygon-Z,player-1,1|health:238/280",
            "damage|mon:Porygon-Z,player-1,1|health:85/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn trick_room_does_not_reverse_priority_order() {
    let mut battle = make_battle(0, porygonz().unwrap(), deoxys().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Porygon-Z,player-1,1|name:Trick Room",
            "fieldstart|move:Trick Room",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Deoxys,player-2,1|name:Quick Attack|target:Porygon-Z,player-1,1",
            "split|side:0",
            "damage|mon:Porygon-Z,player-1,1|health:235/280",
            "damage|mon:Porygon-Z,player-1,1|health:84/100",
            "move|mon:Porygon-Z,player-1,1|name:Tackle|target:Deoxys,player-2,1",
            "split|side:1",
            "damage|mon:Deoxys,player-2,1|health:170/210",
            "damage|mon:Deoxys,player-2,1|health:81/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn trick_room_can_be_outsped() {
    let mut battle = make_battle(0, porygonz().unwrap(), deoxys().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Deoxys,player-2,1|name:Agility|target:Deoxys,player-2,1",
            "boost|mon:Deoxys,player-2,1|stat:spe|by:2",
            "move|mon:Porygon-Z,player-1,1|name:Tackle|target:Deoxys,player-2,1",
            "split|side:1",
            "damage|mon:Deoxys,player-2,1|health:86/210",
            "damage|mon:Deoxys,player-2,1|health:41/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 4, &expected_logs);
}
