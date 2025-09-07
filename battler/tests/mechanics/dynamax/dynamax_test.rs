use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    Id,
    LocalDataStore,
    MonMoveSlotData,
    MoveTarget,
    PublicCoreBattle,
    Request,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "Chlorophyll",
                    "moves": [
                        "Tackle",
                        "Toxic"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "Blaze",
                    "moves": [
                        "Ember",
                        "Guillotine"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Chansey",
                    "species": "Chansey",
                    "ability": "No Ability",
                    "moves": [
                        "Soft-Boiled",
                        "Torment",
                        "Fake Out",
                        "Low Kick",
                        "Encore",
                        "Destiny Bond",
                        "Skill Swap"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Zacian",
                    "species": "Zacian",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Ditto",
                    "species": "Ditto",
                    "ability": "No Ability",
                    "moves": [
                        "Transform"
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
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'_>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
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
        .build(data)
}

#[test]
fn one_mon_can_dynamax_and_use_max_moves() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert!(request.active[0].can_dynamax);
        pretty_assertions::assert_eq!(request.active[0].moves, Vec::from_iter([
            MonMoveSlotData {
                id: Id::from("tackle"),
                name: "Tackle".to_owned(),
                pp: 35,
                max_pp: 35,
                target: MoveTarget::Normal,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("toxic"),
                name: "Toxic".to_owned(),
                pp: 10,
                max_pp: 10,
                target: MoveTarget::Normal,
                disabled: false,
            },
        ]));
        pretty_assertions::assert_eq!(request.active[0].max_moves, Vec::from_iter([
            MonMoveSlotData {
                id: Id::from("maxstrike"),
                name: "Max Strike".to_owned(),
                pp: 35,
                max_pp: 35,
                target: MoveTarget::AdjacentFoe,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("maxguard"),
                name: "Max Guard".to_owned(),
                pp: 10,
                max_pp: 10,
                target: MoveTarget::User,
                disabled: false,
            },
        ]));
    });

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1,dyna"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,dyna"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Venusaur cannot dynamax");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,-1"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: you cannot choose a target for Max Guard");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:210/210",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "move|mon:Venusaur,player-1,1|name:Max Strike|target:Venusaur,player-2,1",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|health:99/140",
            "damage|mon:Venusaur,player-2,1|health:71/100",
            "unboost|mon:Venusaur,player-2,1|stat:spe|by:1",
            "move|mon:Venusaur,player-2,1|name:Tackle|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:191/210",
            "damage|mon:Venusaur,player-1,1|health:91/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Venusaur,player-1,1|name:Max Guard|target:Venusaur,player-1,1",
            "singleturn|mon:Venusaur,player-1,1|move:Max Guard",
            "move|mon:Venusaur,player-2,1|name:Tackle|target:Venusaur,player-1,1",
            "activate|mon:Venusaur,player-1,1|move:Max Guard",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert!(!request.active[0].can_dynamax);
        pretty_assertions::assert_eq!(request.active[0].moves, Vec::from_iter([
            MonMoveSlotData {
                id: Id::from("tackle"),
                name: "Tackle".to_owned(),
                pp: 34,
                max_pp: 35,
                target: MoveTarget::Normal,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("toxic"),
                name: "Toxic".to_owned(),
                pp: 9,
                max_pp: 10,
                target: MoveTarget::Normal,
                disabled: false,
            },
        ]));
        pretty_assertions::assert_eq!(request.active[0].max_moves, Vec::from_iter([
            MonMoveSlotData {
                id: Id::from("maxstrike"),
                name: "Max Strike".to_owned(),
                pp: 34,
                max_pp: 35,
                target: MoveTarget::AdjacentFoe,
                disabled: false,
            },
            MonMoveSlotData {
                id: Id::from("maxguard"),
                name: "Max Guard".to_owned(),
                pp: 9,
                max_pp: 10,
                target: MoveTarget::User,
                disabled: false,
            },
        ]));
    });
}

#[test]
fn dynamax_ends_on_switch() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:210/210",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "move|mon:Venusaur,player-1,1|name:Max Strike|target:Venusaur,player-2,1",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|health:99/140",
            "damage|mon:Venusaur,player-2,1|health:71/100",
            "unboost|mon:Venusaur,player-2,1|stat:spe|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "revertdynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:140/140",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dynamax_ends_on_faint() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:210/210",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "move|mon:Venusaur,player-1,1|name:Max Strike|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:95/138",
            "damage|mon:Charizard,player-2,1|health:69/100",
            "unboost|mon:Charizard,player-2,1|stat:spe|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Guillotine|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:0",
            "damage|mon:Venusaur,player-1,1|health:0",
            "ohko|mon:Venusaur,player-1,1",
            "faint|mon:Venusaur,player-1,1",
            "revertdynamax|mon:Venusaur,player-1,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dynamax_ends_after_three_turns() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:210/210",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "move|mon:Venusaur,player-1,1|name:Max Strike|target:Venusaur,player-2,1",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|health:99/140",
            "damage|mon:Venusaur,player-2,1|health:71/100",
            "unboost|mon:Venusaur,player-2,1|stat:spe|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "residual",
            "turn|turn:3",
            ["time"],
            "revertdynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:140/140",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dynamax_can_still_struggle() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team_1 = team().unwrap();
    team_1.members[2].moves = vec!["Soft-Boiled".to_owned()];
    let mut battle = make_battle(&data, 100, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Chansey,player-1,1",
            "split|side:0",
            "sethp|mon:Chansey,player-1,1|health:465/465",
            "sethp|mon:Chansey,player-1,1|health:100/100",
            "move|mon:Venusaur,player-2,1|name:Tackle|target:Chansey,player-1,1",
            "split|side:0",
            "damage|mon:Chansey,player-1,1|health:310/465",
            "damage|mon:Chansey,player-1,1|health:67/100",
            "move|mon:Chansey,player-1,1|name:Struggle|target:Venusaur,player-2,1",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|health:136/140",
            "damage|mon:Venusaur,player-2,1|health:98/100",
            "split|side:0",
            "damage|mon:Chansey,player-1,1|from:Struggle Recoil|health:232/465",
            "damage|mon:Chansey,player-1,1|from:Struggle Recoil|health:50/100",
            "residual",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 7, &expected_logs);
}

#[test]
fn dynamax_level_increases_hp() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team_1 = team().unwrap();
    team_1.members[0].dynamax_level = 5;
    let mut team_2 = team().unwrap();
    team_2.members[0].dynamax_level = 10;
    let mut battle = make_battle(&data, 100, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,dyna"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:245/245",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "dynamax|mon:Venusaur,player-2,1",
            "split|side:1",
            "sethp|mon:Venusaur,player-2,1|health:280/280",
            "sethp|mon:Venusaur,player-2,1|health:100/100",
            "move|mon:Venusaur,player-1,1|name:Max Strike|target:Venusaur,player-2,1",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|health:239/280",
            "damage|mon:Venusaur,player-2,1|health:86/100",
            "unboost|mon:Venusaur,player-2,1|stat:spe|by:1",
            "move|mon:Venusaur,player-2,1|name:Max Strike|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:204/245",
            "damage|mon:Venusaur,player-1,1|health:84/100",
            "unboost|mon:Venusaur,player-1,1|stat:spe|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn hp_ratio_stays_the_same_before_and_after_dynamax() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-2,1|name:Tackle|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:121/140",
            "damage|mon:Venusaur,player-1,1|health:87/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:181/210",
            "sethp|mon:Venusaur,player-1,1|health:87/100",
            "move|mon:Venusaur,player-1,1|name:Max Strike|target:Venusaur,player-2,1",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|health:99/140",
            "damage|mon:Venusaur,player-2,1|health:71/100",
            "unboost|mon:Venusaur,player-2,1|stat:spe|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Venusaur,player-2,1|name:Tackle|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:162/210",
            "damage|mon:Venusaur,player-1,1|health:78/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "revertdynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:108/140",
            "sethp|mon:Venusaur,player-1,1|health:78/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dynamax_immune_to_choice_item() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team_1 = team().unwrap();
    team_1.members[0].item = Some("Choice Band".to_owned());
    let mut battle = make_battle(&data, 100, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:210/210",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "dynamax|mon:Venusaur,player-2,1",
            "split|side:1",
            "sethp|mon:Venusaur,player-2,1|health:210/210",
            "sethp|mon:Venusaur,player-2,1|health:100/100",
            "move|mon:Venusaur,player-1,1|name:Max Strike|target:Venusaur,player-2,1",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|health:169/210",
            "damage|mon:Venusaur,player-2,1|health:81/100",
            "unboost|mon:Venusaur,player-2,1|stat:spe|by:1",
            "move|mon:Venusaur,player-2,1|name:Max Strike|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:169/210",
            "damage|mon:Venusaur,player-1,1|health:81/100",
            "unboost|mon:Venusaur,player-1,1|stat:spe|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Venusaur,player-1,1|name:Max Guard|noanim",
            "fail|mon:Venusaur,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn zacian_cannot_dynamax_even_if_transformed() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Zacian cannot dynamax");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,dyna"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Ditto cannot dynamax");
    });
}

#[test]
fn dynamax_immune_to_torment() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Chansey"],
            ["switch", "player-2", "Chansey"],
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:210/210",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "move|mon:Venusaur,player-1,1|name:Max Guard|noanim",
            "fail|mon:Venusaur,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Chansey,player-2,1|name:Torment|noanim",
            "fail|mon:Chansey,player-2,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dynamax_immune_to_flinch() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Chansey"],
            ["switch", "player-2", "Chansey"],
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:210/210",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "move|mon:Venusaur,player-1,1|name:Max Guard|noanim",
            "fail|mon:Venusaur,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Chansey,player-2,1|name:Fake Out|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:204/210",
            "damage|mon:Venusaur,player-1,1|health:98/100",
            "move|mon:Venusaur,player-1,1|name:Max Strike|target:Chansey,player-2,1",
            "split|side:1",
            "damage|mon:Chansey,player-2,1|health:0",
            "damage|mon:Chansey,player-2,1|health:0",
            "faint|mon:Chansey,player-2,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dynamax_immune_to_low_kick() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Chansey"],
            ["switch", "player-2", "Chansey"],
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:210/210",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "move|mon:Venusaur,player-1,1|name:Max Guard|noanim",
            "fail|mon:Venusaur,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Chansey,player-2,1|name:Low Kick|noanim",
            "fail|mon:Chansey,player-2,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dynamax_immune_to_encore() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Chansey"],
            ["switch", "player-2", "Chansey"],
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:210/210",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "move|mon:Venusaur,player-1,1|name:Max Guard|noanim",
            "fail|mon:Venusaur,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Chansey,player-2,1|name:Encore|noanim",
            "fail|mon:Chansey,player-2,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dynamax_immune_to_destiny_bond() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Chansey"],
            ["switch", "player-2", "Chansey"],
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:210/210",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "move|mon:Venusaur,player-1,1|name:Max Guard|noanim",
            "fail|mon:Venusaur,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Chansey,player-2,1|name:Destiny Bond|target:Chansey,player-2,1",
            "singlemove|mon:Chansey,player-2,1|move:Destiny Bond",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Venusaur,player-1,1|name:Max Strike|target:Chansey,player-2,1",
            "split|side:1",
            "damage|mon:Chansey,player-2,1|health:0",
            "damage|mon:Chansey,player-2,1|health:0",
            "faint|mon:Chansey,player-2,1",
            "revertdynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:140/140",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dynamax_immune_to_skill_swap() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 6"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Chansey"],
            ["switch", "player-2", "Chansey"],
            "dynamax|mon:Venusaur,player-1,1",
            "split|side:0",
            "sethp|mon:Venusaur,player-1,1|health:210/210",
            "sethp|mon:Venusaur,player-1,1|health:100/100",
            "move|mon:Venusaur,player-1,1|name:Max Guard|noanim",
            "fail|mon:Venusaur,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Chansey,player-2,1|name:Skill Swap|noanim",
            "fail|mon:Chansey,player-2,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
