use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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

fn commander_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Tatsugiri",
                    "species": "Tatsugiri",
                    "ability": "Commander",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }, {
                    "name": "Tatsugiri",
                    "species": "Tatsugiri",
                    "ability": "Commander",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Dondozo",
                    "species": "Dondozo",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Memento"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn non_commander_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Meowscarada",
                    "species": "Meowscarada",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Dragon Tail"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Quaquaval",
                    "species": "Quaquaval",
                    "ability": "No Ability",
                    "moves": [
                        "Surf"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    seed: u64,
    battle_type: BattleType,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn commander_does_nothing_in_singles_battle() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        commander_team().unwrap(),
        commander_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Tatsugiri,player-1,1|name:Tackle|target:Tatsugiri,player-2,1",
            "split|side:1",
            "damage|mon:Tatsugiri,player-2,1|health:112/128",
            "damage|mon:Tatsugiri,player-2,1|health:88/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn commander_activates_at_start_of_battle() {
    let mut team = commander_team().unwrap();
    team.members.drain(0..1);
    let mut battle =
        make_battle(0, BattleType::Doubles, team, non_commander_team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Tatsugiri"],
            ["switch", "player-1", "Tatsugiri"],
            "split|side:0",
            ["switch", "player-1", "Dondozo"],
            ["switch", "player-1", "Dondozo"],
            "split|side:1",
            ["switch", "player-2", "Meowscarada"],
            ["switch", "player-2", "Meowscarada"],
            "split|side:1",
            ["switch", "player-2", "Quaquaval"],
            ["switch", "player-2", "Quaquaval"],
            "activate|mon:Tatsugiri,player-1,1|ability:Commander",
            "start|mon:Tatsugiri,player-1,1|condition:Commanding|of:Dondozo,player-1,2",
            "boost|mon:Dondozo,player-1,2|stat:atk|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:def|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spa|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spd|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spe|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn commander_activates_on_switch() {
    let mut battle = make_battle(
        0,
        BattleType::Doubles,
        commander_team().unwrap(),
        non_commander_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;switch 2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Dondozo"],
            ["switch", "player-1", "Dondozo"],
            "activate|mon:Tatsugiri,player-1,1|ability:Commander",
            "start|mon:Tatsugiri,player-1,1|condition:Commanding|of:Dondozo,player-1,2",
            "boost|mon:Dondozo,player-1,2|stat:atk|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:def|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spa|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spd|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spe|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "move|mon:Meowscarada,player-2,1|name:Tackle|noanim",
            "miss|mon:Tatsugiri,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 1;pass"),
        Err(err) => assert!(format!("{err:#}").contains("Tatsugiri is trapped"), "{err:#}")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;switch 1"),
        Err(err) => assert!(format!("{err:#}").contains("Dondozo is trapped"), "{err:#}")
    );

    assert_matches::assert_matches!(
        battle.request_for_player("player-1"),
        Ok(Some(Request::Turn(request))) => {
            pretty_assertions::assert_eq!(
                request.active[0],
                serde_json::from_str(
                    r#"{
                        "team_position": 0,
                        "moves": [
                            {
                                "name": "Pass",
                                "id": "pass",
                                "pp": 0,
                                "max_pp": 0,
                                "target": "User",
                                "disabled": false
                            }
                        ],
                        "trapped": true,
                        "locked_into_move": true
                    }"#,
                )
                .unwrap()
            );
        }
    );
}

#[test]
fn commander_prevents_movement_and_attacks() {
    let mut battle = make_battle(
        0,
        BattleType::Doubles,
        commander_team().unwrap(),
        non_commander_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;switch 2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;move 0"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Dondozo"],
            ["switch", "player-1", "Dondozo"],
            "activate|mon:Tatsugiri,player-1,1|ability:Commander",
            "start|mon:Tatsugiri,player-1,1|condition:Commanding|of:Dondozo,player-1,2",
            "boost|mon:Dondozo,player-1,2|stat:atk|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:def|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spa|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spd|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spe|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "move|mon:Meowscarada,player-2,1|name:Tackle|noanim",
            "miss|mon:Tatsugiri,player-1,1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Meowscarada,player-2,1|name:Tackle|noanim",
            "miss|mon:Tatsugiri,player-1,1",
            "move|mon:Quaquaval,player-2,2|name:Surf|spread:Meowscarada,player-2,1;Dondozo,player-1,2",
            "miss|mon:Tatsugiri,player-1,1",
            "resisted|mon:Meowscarada,player-2,1",
            "resisted|mon:Dondozo,player-1,2",
            "split|side:1",
            "damage|mon:Meowscarada,player-2,1|health:109/136",
            "damage|mon:Meowscarada,player-2,1|health:81/100",
            "split|side:0",
            "damage|mon:Dondozo,player-1,2|health:195/210",
            "damage|mon:Dondozo,player-1,2|health:93/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn commander_prevents_drag_out() {
    let mut battle = make_battle(
        0,
        BattleType::Doubles,
        commander_team().unwrap(),
        non_commander_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;switch 2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Dondozo"],
            ["switch", "player-1", "Dondozo"],
            "activate|mon:Tatsugiri,player-1,1|ability:Commander",
            "start|mon:Tatsugiri,player-1,1|condition:Commanding|of:Dondozo,player-1,2",
            "boost|mon:Dondozo,player-1,2|stat:atk|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:def|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spa|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spd|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spe|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "move|mon:Meowscarada,player-2,1|name:Dragon Tail|target:Dondozo,player-1,2",
            "split|side:0",
            "damage|mon:Dondozo,player-1,2|health:196/210",
            "damage|mon:Dondozo,player-1,2|health:94/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn commander_ends_when_dondozo_faints() {
    let mut battle = make_battle(
        0,
        BattleType::Doubles,
        commander_team().unwrap(),
        non_commander_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;switch 2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0;move 1,1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Dondozo,player-1,2|name:Memento|target:Meowscarada,player-2,1",
            "unboost|mon:Meowscarada,player-2,1|stat:atk|by:2",
            "unboost|mon:Meowscarada,player-2,1|stat:spa|by:2",
            "faint|mon:Dondozo,player-1,2",
            "end|mon:Tatsugiri,player-1,1|condition:Commanding",
            "move|mon:Meowscarada,player-2,1|name:Dragon Tail|target:Tatsugiri,player-1,1",
            "supereffective|mon:Tatsugiri,player-1,1",
            "split|side:0",
            "damage|mon:Tatsugiri,player-1,1|health:78/128",
            "damage|mon:Tatsugiri,player-1,1|health:61/100",
            "split|side:0",
            ["drag", "player-1", "Tatsugiri"],
            ["drag", "player-1", "Tatsugiri"],
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}

#[test]
fn tatsugiri_can_faint_inside_dondozo() {
    let mut team = commander_team().unwrap();
    team.members[0].item = Some("Toxic Orb".to_owned());
    let mut battle =
        make_battle(0, BattleType::Doubles, team, non_commander_team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;switch 2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    for _ in 0..6 {
        assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
        assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    }
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            "damage|mon:Tatsugiri,player-1,1|from:status:Bad Poison|health:0",
            "damage|mon:Tatsugiri,player-1,1|from:status:Bad Poison|health:0",
            "residual",
            "faint|mon:Tatsugiri,player-1,1",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Tatsugiri"],
            ["switch", "player-1", "Tatsugiri"],
            "activate|mon:Tatsugiri,player-1,1|ability:Commander",
            "start|mon:Tatsugiri,player-1,1|condition:Commanding|of:Dondozo,player-1,2",
            "boost|mon:Dondozo,player-1,2|stat:atk|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:def|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spa|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spd|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spe|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 7, &expected_logs);
}

#[test]
fn one_commander_activates_for_single_dondozo() {
    let mut team = commander_team().unwrap();
    team.members.swap(1, 2);
    let mut battle =
        make_battle(0, BattleType::Triples, team, non_commander_team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "activate|mon:Tatsugiri,player-1,1|ability:Commander",
            "start|mon:Tatsugiri,player-1,1|condition:Commanding|of:Dondozo,player-1,2",
            "boost|mon:Dondozo,player-1,2|stat:atk|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:def|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spa|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spd|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-1,2|stat:spe|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn multiple_commanders_can_activate_at_once_in_multi_battle() {
    let mut tatsugiri = commander_team().unwrap();
    tatsugiri.members.drain(1..);
    let mut dondozo = commander_team().unwrap();
    dondozo.members.drain(0..2);
    let mut meoscarada = non_commander_team().unwrap();
    meoscarada.members.drain(1..);
    let mut battle = TestBattleBuilder::new()
        .with_battle_type(BattleType::Multi)
        .with_seed(0)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_1("player-2", "Player 2")
        .add_player_to_side_1("player-3", "Player 3")
        .add_player_to_side_1("player-4", "Player 4")
        .add_player_to_side_2("player-5", "Player 5")
        .add_player_to_side_2("player-6", "Player 6")
        .add_player_to_side_2("player-7", "Player 7")
        .add_player_to_side_2("player-8", "Player 8")
        .with_team("player-1", tatsugiri.clone())
        .with_team("player-2", dondozo.clone())
        .with_team("player-3", tatsugiri)
        .with_team("player-4", dondozo)
        .with_team("player-5", meoscarada.clone())
        .with_team("player-6", meoscarada.clone())
        .with_team("player-7", meoscarada.clone())
        .with_team("player-8", meoscarada)
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "activate|mon:Tatsugiri,player-1,1|ability:Commander",
            "start|mon:Tatsugiri,player-1,1|condition:Commanding|of:Dondozo,player-2,2",
            "boost|mon:Dondozo,player-2,2|stat:atk|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-2,2|stat:def|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-2,2|stat:spa|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-2,2|stat:spd|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "boost|mon:Dondozo,player-2,2|stat:spe|by:2|from:ability:Commander|of:Tatsugiri,player-1,1",
            "activate|mon:Tatsugiri,player-3,3|ability:Commander",
            "start|mon:Tatsugiri,player-3,3|condition:Commanding|of:Dondozo,player-4,4",
            "boost|mon:Dondozo,player-4,4|stat:atk|by:2|from:ability:Commander|of:Tatsugiri,player-3,3",
            "boost|mon:Dondozo,player-4,4|stat:def|by:2|from:ability:Commander|of:Tatsugiri,player-3,3",
            "boost|mon:Dondozo,player-4,4|stat:spa|by:2|from:ability:Commander|of:Tatsugiri,player-3,3",
            "boost|mon:Dondozo,player-4,4|stat:spd|by:2|from:ability:Commander|of:Tatsugiri,player-3,3",
            "boost|mon:Dondozo,player-4,4|stat:spe|by:2|from:ability:Commander|of:Tatsugiri,player-3,3",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
