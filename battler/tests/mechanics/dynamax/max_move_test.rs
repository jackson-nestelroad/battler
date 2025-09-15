use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    Id,
    MonMoveSlotData,
    MoveTarget,
    PublicCoreBattle,
    Request,
    TeamData,
    Type,
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
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Quick Attack",
                        "Thunderbolt",
                        "Thunder Wave",
                        "Hydro Cannon",
                        "Water Gun",
                        "Air Slash",
                        "Psychic",
                        "Earthquake",
                        "Me First"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Eevee",
                    "species": "Eevee",
                    "gender": "F",
                    "ability": "Cloud Nine",
                    "moves": [
                        "Protect",
                        "Feint",
                        "Flamethrower",
                        "Disable",
                        "Mimic",
                        "Sketch",
                        "Spite",
                        "Substitute"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "gigantamax_factor": true
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
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_dynamax(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn max_move_changes_based_on_type() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        pretty_assertions::assert_eq!(request.active[0].max_moves, Vec::from_iter([
            MonMoveSlotData {
                id: Id::from("maxstrike"),
                name: "Max Strike".to_owned(),
                pp: 30,
                max_pp: 30,
                target: MoveTarget::AdjacentFoe,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("maxlightning"),
                name: "Max Lightning".to_owned(),
                pp: 15,
                max_pp: 15,
                target: MoveTarget::AdjacentFoe,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("maxguard"),
                name: "Max Guard".to_owned(),
                pp: 20,
                max_pp: 20,
                target: MoveTarget::User,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("maxgeyser"),
                name: "Max Geyser".to_owned(),
                pp: 5,
                max_pp: 5,
                target: MoveTarget::AdjacentFoe,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("maxgeyser"),
                name: "Max Geyser".to_owned(),
                pp: 25,
                max_pp: 25,
                target: MoveTarget::AdjacentFoe,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("maxairstream"),
                name: "Max Airstream".to_owned(),
                pp: 15,
                max_pp: 15,
                target: MoveTarget::AdjacentFoe,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("maxmindstorm"),
                name: "Max Mindstorm".to_owned(),
                pp: 10,
                max_pp: 10,
                target: MoveTarget::AdjacentFoe,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("maxquake"),
                name: "Max Quake".to_owned(),
                pp: 10,
                max_pp: 10,
                target: MoveTarget::AdjacentFoe,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("maxguard"),
                name: "Max Guard".to_owned(),
                pp: 20,
                max_pp: 20,
                target: MoveTarget::User,
                disabled: false,
            },
        ]));
    });

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1,dyna;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pikachu,player-1,1",
            "split|side:0",
            "sethp|mon:Pikachu,player-1,1|health:142/142",
            "sethp|mon:Pikachu,player-1,1|health:100/100",
            "move|mon:Pikachu,player-1,1|name:Max Strike|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:41/95",
            "damage|mon:Pikachu,player-2,1|health:44/100",
            "unboost|mon:Pikachu,player-2,1|stat:spe|by:1",
            "unboost|mon:Eevee,player-2,2|stat:spe|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Max Lightning|target:Pikachu,player-2,1",
            "resisted|mon:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:0",
            "damage|mon:Pikachu,player-2,1|health:0",
            "fieldstart|move:Electric Terrain",
            "faint|mon:Pikachu,player-2,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn status_move_changes_to_max_guard() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2,dyna;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 4,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pikachu,player-1,1",
            "split|side:0",
            "sethp|mon:Pikachu,player-1,1|health:142/142",
            "sethp|mon:Pikachu,player-1,1|health:100/100",
            "move|mon:Pikachu,player-1,1|name:Max Guard|target:Pikachu,player-1,1",
            "singleturn|mon:Pikachu,player-1,1|move:Max Guard",
            "move|mon:Pikachu,player-2,1|name:Quick Attack|target:Pikachu,player-1,1",
            "activate|mon:Pikachu,player-1,1|move:Max Guard",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Max Guard|noanim",
            "fail|mon:Pikachu,player-1,1",
            "move|mon:Pikachu,player-2,1|name:Water Gun|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:123/142",
            "damage|mon:Pikachu,player-1,1|health:87/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn max_move_changes_based_on_move_with_dynamic_type() {
    let mut team_1 = team().unwrap();
    team_1.members[0].moves = vec!["Hidden Power".to_owned()];
    team_1.members[0].hidden_power_type = Some(Type::Dark);
    let mut battle = make_battle(100, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1,dyna;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pikachu,player-1,1",
            "split|side:0",
            "sethp|mon:Pikachu,player-1,1|health:142/142",
            "sethp|mon:Pikachu,player-1,1|health:100/100",
            "move|mon:Pikachu,player-1,1|name:Max Darkness|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:45/95",
            "damage|mon:Pikachu,player-2,1|health:48/100",
            "unboost|mon:Pikachu,player-2,1|stat:spd|by:1",
            "unboost|mon:Eevee,player-2,2|stat:spd|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn max_move_can_boost_allies_stats() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 5,1,dyna;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pikachu,player-1,1",
            "split|side:0",
            "sethp|mon:Pikachu,player-1,1|health:142/142",
            "sethp|mon:Pikachu,player-1,1|health:100/100",
            "move|mon:Pikachu,player-1,1|name:Max Airstream|target:Pikachu,player-2,1",
            "resisted|mon:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:66/95",
            "damage|mon:Pikachu,player-2,1|health:70/100",
            "boost|mon:Pikachu,player-1,1|stat:spe|by:1",
            "boost|mon:Eevee,player-1,2|stat:spe|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn max_move_varies_power_based_on_base_move() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 3,1,dyna;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 4,1,dyna;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pikachu,player-1,1",
            "split|side:0",
            "sethp|mon:Pikachu,player-1,1|health:142/142",
            "sethp|mon:Pikachu,player-1,1|health:100/100",
            "dynamax|mon:Pikachu,player-2,1",
            "split|side:1",
            "sethp|mon:Pikachu,player-2,1|health:142/142",
            "sethp|mon:Pikachu,player-2,1|health:100/100",
            "move|mon:Pikachu,player-1,1|name:Max Geyser|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:74/142",
            "damage|mon:Pikachu,player-2,1|health:53/100",
            "weather|weather:Rain",
            "move|mon:Pikachu,player-2,1|name:Max Geyser|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:101/142",
            "damage|mon:Pikachu,player-1,1|health:72/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn max_move_hits_through_protect() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,2,dyna;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pikachu,player-1,1",
            "split|side:0",
            "sethp|mon:Pikachu,player-1,1|health:142/142",
            "sethp|mon:Pikachu,player-1,1|health:100/100",
            "move|mon:Eevee,player-2,2|name:Protect|target:Eevee,player-2,2",
            "singleturn|mon:Eevee,player-2,2|move:Protect",
            "move|mon:Pikachu,player-1,1|name:Max Lightning|target:Eevee,player-2,2",
            "protectweaken|mon:Eevee,player-2,2",
            "split|side:1",
            "damage|mon:Eevee,player-2,2|health:98/115",
            "damage|mon:Eevee,player-2,2|health:86/100",
            "fieldstart|move:Electric Terrain",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn feint_hits_through_max_guard() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2,dyna;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 1,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pikachu,player-1,1",
            "split|side:0",
            "sethp|mon:Pikachu,player-1,1|health:142/142",
            "sethp|mon:Pikachu,player-1,1|health:100/100",
            "move|mon:Pikachu,player-1,1|name:Max Guard|target:Pikachu,player-1,1",
            "singleturn|mon:Pikachu,player-1,1|move:Max Guard",
            "move|mon:Eevee,player-2,2|name:Feint|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:114/142",
            "damage|mon:Pikachu,player-1,1|health:81/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gigantamax_gets_gmax_move_for_certain_type() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        pretty_assertions::assert_eq!(request.active[1].max_moves[0..=2], Vec::from_iter([
            MonMoveSlotData {
                id: Id::from("maxguard"),
                name: "Max Guard".to_owned(),
                pp: 10,
                max_pp: 10,
                target: MoveTarget::User,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("gmaxcuddle"),
                name: "G-Max Cuddle".to_owned(),
                pp: 10,
                max_pp: 10,
                target: MoveTarget::AdjacentFoe,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("maxflare"),
                name: "Max Flare".to_owned(),
                pp: 15,
                max_pp: 15,
                target: MoveTarget::AdjacentFoe,
                disabled: false,
            },
        ]));
    });

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 1,1,dyna"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "gigantamax|mon:Eevee,player-1,2|species:Eevee-Gmax",
            "dynamax|mon:Eevee,player-1,2",
            "split|side:0",
            "sethp|mon:Eevee,player-1,2|health:172/172",
            "sethp|mon:Eevee,player-1,2|health:100/100",
            "move|mon:Eevee,player-1,2|name:G-Max Cuddle|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:14/95",
            "damage|mon:Pikachu,player-2,1|health:15/100",
            "start|mon:Pikachu,player-2,1|move:Attract",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn disable_fails_after_max_move() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1,dyna;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 3,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pikachu,player-1,1",
            "split|side:0",
            "sethp|mon:Pikachu,player-1,1|health:142/142",
            "sethp|mon:Pikachu,player-1,1|health:100/100",
            "move|mon:Pikachu,player-1,1|name:Max Strike|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:41/95",
            "damage|mon:Pikachu,player-2,1|health:44/100",
            "unboost|mon:Pikachu,player-2,1|stat:spe|by:1",
            "unboost|mon:Eevee,player-2,2|stat:spe|by:1",
            "move|mon:Eevee,player-2,2|name:Disable|noanim",
            "fail|mon:Eevee,player-2,2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mimic_fails_after_max_move() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1,dyna;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 4,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pikachu,player-1,1",
            "split|side:0",
            "sethp|mon:Pikachu,player-1,1|health:142/142",
            "sethp|mon:Pikachu,player-1,1|health:100/100",
            "move|mon:Pikachu,player-1,1|name:Max Strike|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:41/95",
            "damage|mon:Pikachu,player-2,1|health:44/100",
            "unboost|mon:Pikachu,player-2,1|stat:spe|by:1",
            "unboost|mon:Eevee,player-2,2|stat:spe|by:1",
            "move|mon:Eevee,player-2,2|name:Mimic|noanim",
            "fail|mon:Eevee,player-2,2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sketch_fails_after_max_move() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1,dyna;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 5,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pikachu,player-1,1",
            "split|side:0",
            "sethp|mon:Pikachu,player-1,1|health:142/142",
            "sethp|mon:Pikachu,player-1,1|health:100/100",
            "move|mon:Pikachu,player-1,1|name:Max Strike|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:41/95",
            "damage|mon:Pikachu,player-2,1|health:44/100",
            "unboost|mon:Pikachu,player-2,1|stat:spe|by:1",
            "unboost|mon:Eevee,player-2,2|stat:spe|by:1",
            "move|mon:Eevee,player-2,2|name:Sketch|noanim",
            "fail|mon:Eevee,player-2,2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn spite_deducts_pp_of_base_move() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1,dyna;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 6,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pikachu,player-1,1",
            "split|side:0",
            "sethp|mon:Pikachu,player-1,1|health:142/142",
            "sethp|mon:Pikachu,player-1,1|health:100/100",
            "move|mon:Pikachu,player-1,1|name:Max Strike|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:41/95",
            "damage|mon:Pikachu,player-2,1|health:44/100",
            "unboost|mon:Pikachu,player-2,1|stat:spe|by:1",
            "unboost|mon:Eevee,player-2,2|stat:spe|by:1",
            "move|mon:Eevee,player-2,2|name:Spite|target:Pikachu,player-1,1",
            "deductpp|mon:Pikachu,player-1,1|move:Quick Attack|by:4",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn me_first_fails_for_max_move() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 8,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1,dyna;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pikachu,player-2,1",
            "split|side:1",
            "sethp|mon:Pikachu,player-2,1|health:142/142",
            "sethp|mon:Pikachu,player-2,1|health:100/100",
            "move|mon:Pikachu,player-1,1|name:Me First|noanim",
            "fail|mon:Pikachu,player-1,1",
            "move|mon:Pikachu,player-2,1|name:Max Strike|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:41/95",
            "damage|mon:Pikachu,player-1,1|health:44/100",
            "unboost|mon:Pikachu,player-1,1|stat:spe|by:1",
            "unboost|mon:Eevee,player-1,2|stat:spe|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn max_move_stat_drop_hits_through_substitute() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;move 7"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1,dyna;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eevee,player-2,2|name:Substitute|target:Eevee,player-2,2",
            "start|mon:Eevee,player-2,2|move:Substitute",
            "split|side:1",
            "damage|mon:Eevee,player-2,2|health:87/115",
            "damage|mon:Eevee,player-2,2|health:76/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "dynamax|mon:Pikachu,player-1,1",
            "split|side:0",
            "sethp|mon:Pikachu,player-1,1|health:142/142",
            "sethp|mon:Pikachu,player-1,1|health:100/100",
            "move|mon:Pikachu,player-1,1|name:Max Strike|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:41/95",
            "damage|mon:Pikachu,player-2,1|health:44/100",
            "unboost|mon:Pikachu,player-2,1|stat:spe|by:1",
            "unboost|mon:Eevee,player-2,2|stat:spe|by:1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
