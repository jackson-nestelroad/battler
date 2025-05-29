use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    MonSummaryData,
    PublicCoreBattle,
    TeamData,
    WildPlayerOptions,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
    LogMatch,
    TestBattleBuilder,
};

fn pikachu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "Lightning Rod",
                    "moves": [
                        "False Swipe",
                        "Glare",
                        "Thunderbolt",
                        "Sleep Powder"
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

fn magikarp() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Magikarp",
                    "species": "Magikarp",
                    "ability": "Swift Swim",
                    "moves": [
                        "Splash",
                        "Bounce"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 5
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn magikarp_gyarados() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Magikarp",
                    "species": "Magikarp",
                    "ability": "Swift Swim",
                    "moves": [
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 5
                },
                {
                    "name": "Gyarados",
                    "species": "Gyarados",
                    "ability": "Intimidate",
                    "moves": [
                        "Surf"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 30
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn blissey() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blissey",
                    "species": "Blissey",
                    "ability": "Natural Cure",
                    "moves": [
                        "False Swipe",
                        "Belly Drum",
                        "Sleep Powder"
                    ],
                    "nature": "Lonely",
                    "gender": "M",
                    "level": 100,
                    "ivs": {
                        "hp": 31,
                        "atk": 31
                    },
                    "evs": {
                        "hp": 252,
                        "atk": 252
                    }
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn metagross() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Metagross",
                    "species": "Metagross",
                    "ability": "Clear Body",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_wild_singles_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
    wild_options: WildPlayerOptions,
) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .add_wild_mon_to_side_2("wild", "Wild", wild_options)
        .with_team("protagonist", team_1)
        .with_team("wild", team_2)
        .build(data)
}

fn make_wild_multi_battle<'d>(
    data: &'d dyn DataStore,
    seed: u64,
    team: TeamData,
    wild: Vec<TeamData>,
    wild_options: WildPlayerOptions,
) -> Result<PublicCoreBattle<'d>> {
    let mut builder = TestBattleBuilder::new()
        .with_battle_type(BattleType::Multi)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .with_team("protagonist", team);

    for (i, wild) in wild.into_iter().enumerate() {
        let id = format!("wild-{}", i);
        builder = builder
            .add_wild_mon_to_side_2(&id, "Wild", wild_options)
            .with_team(&id, wild);
    }

    builder.build(data)
}

fn make_trainer_singles_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .add_player_to_side_2("trainer", "Trainer")
        .with_team("protagonist", team_1)
        .with_team("trainer", team_2)
        .build(data)
}

#[test]
fn level_5_magikarp_caught_in_poke_ball() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_wild_singles_battle(
        &data,
        65535,
        pikachu().unwrap(),
        magikarp().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item pokeball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item pokeball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Poké Ball|target:Magikarp,wild,1",
            "catchfailed|player:protagonist|mon:Magikarp,wild,1|item:Poké Ball|shakes:1",
            "move|mon:Magikarp,wild,1|name:Splash|target:Magikarp,wild,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,protagonist,1|name:False Swipe|target:Magikarp,wild,1",
            "split|side:1",
            "damage|mon:Magikarp,wild,1|health:1/17",
            "damage|mon:Magikarp,wild,1|health:6/100",
            "move|mon:Magikarp,wild,1|name:Splash|target:Magikarp,wild,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:3",
            ["time"],
            "useitem|player:protagonist|name:Poké Ball|target:Magikarp,wild,1",
            "catch|player:protagonist|mon:Magikarp,wild,1|item:Poké Ball|shakes:4",
            "exp|mon:Pikachu,protagonist,1|exp:2",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    pretty_assertions::assert_eq!(
        battle.player_data("protagonist").unwrap().caught,
        serde_json::from_str::<Vec<MonSummaryData>>(
            r#"[
                {
                    "name": "Magikarp",
                    "species": "Magikarp",
                    "level": 5,
                    "gender": "M",
                    "nature": "Hardy",
                    "shiny": false,
                    "ball": "Poké Ball",
                    "hp": 1,
                    "friendship": 0,
                    "experience": 156,
                    "stats": {
                        "hp": 17,
                        "atk": 6,
                        "def": 10,
                        "spa": 6,
                        "spd": 7,
                        "spe": 13
                    },
                    "evs": {
                        "hp": 0,
                        "atk": 0,
                        "def": 0,
                        "spa": 0,
                        "spd": 0,
                        "spe": 0
                    },
                    "ivs": {
                        "hp": 0,
                        "atk": 0,
                        "def": 0,
                        "spa": 0,
                        "spd": 0,
                        "spe": 0
                    },
                    "moves": [
                        {
                            "name": "Splash",
                            "pp": 38
                        },
                        {
                            "name": "Bounce",
                            "pp": 5
                        }
                    ],
                    "ability": "Swift Swim",
                    "item": null,
                    "status": "fnt",
                    "hidden_power_type": "Fighting"
                }
            ]"#
        )
        .unwrap(),
    );
}

#[test]
fn catching_mon_continues_battle() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_wild_singles_battle(
        &data,
        65535,
        pikachu().unwrap(),
        magikarp_gyarados().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item ultraball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("wild", "switch 0"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you cannot switch to a caught mon")
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "switch 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("wild", "item revive,-1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot use item: Revive cannot be used on Magikarp")
    );
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,protagonist,1|name:Glare|target:Magikarp,wild,1",
            "status|mon:Magikarp,wild,1|status:Paralysis",
            "move|mon:Magikarp,wild,1|name:Splash|target:Magikarp,wild,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:2",
            ["time"],
            "useitem|player:protagonist|name:Ultra Ball|target:Magikarp,wild,1",
            "catch|player:protagonist|mon:Magikarp,wild,1|item:Ultra Ball|shakes:4",
            "exp|mon:Pikachu,protagonist,1|exp:2",
            "residual",
            ["time"],
            "split|side:1",
            "appear|player:wild|position:1|name:Gyarados|health:97/97|species:Gyarados|level:30|gender:M",
            "appear|player:wild|position:1|name:Gyarados|health:100/100|species:Gyarados|level:30|gender:M",
            "activate|mon:Gyarados,wild,1|ability:Intimidate",
            "unboost|mon:Pikachu,protagonist,1|stat:atk|by:1",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ball_can_only_be_used_on_isolated_foe() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_wild_multi_battle(
        &data,
        65535,
        pikachu().unwrap(),
        vec![magikarp().unwrap(), magikarp().unwrap()],
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item greatball"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot use item: Great Ball requires one target")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item greatball,1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot use item: invalid target for Great Ball")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item greatball,2"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot use item: invalid target for Great Ball")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item greatball,-1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot use item: invalid target for Great Ball")
    );
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-0", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item greatball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild-1", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,protagonist,1|name:Thunderbolt|target:Magikarp,wild-0,1",
            "supereffective|mon:Magikarp,wild-0,1",
            "split|side:1",
            "damage|mon:Magikarp,wild-0,1|health:0",
            "damage|mon:Magikarp,wild-0,1|health:0",
            "faint|mon:Magikarp,wild-0,1",
            "exp|mon:Pikachu,protagonist,1|exp:2",
            "move|mon:Magikarp,wild-1,2|name:Splash|target:Magikarp,wild-1,2",
            "activate|move:Splash",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,protagonist,1|name:Glare|target:Magikarp,wild-1,2",
            "status|mon:Magikarp,wild-1,2|status:Paralysis",
            "move|mon:Magikarp,wild-1,2|name:Splash|target:Magikarp,wild-1,2",
            "activate|move:Splash",
            "residual",
            "turn|turn:3",
            ["time"],
            "useitem|player:protagonist|name:Great Ball|target:Magikarp,wild-1,2",
            "catch|player:protagonist|mon:Magikarp,wild-1,2|item:Great Ball|shakes:4",
            "exp|mon:Pikachu,protagonist,1|exp:2",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn level_100_metagross_caught_in_master_ball() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_wild_singles_battle(
        &data,
        65535,
        pikachu().unwrap(),
        metagross().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item masterball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Master Ball|target:Metagross,wild,1",
            "catch|player:protagonist|mon:Metagross,wild,1|item:Master Ball|shakes:4",
            "exp|mon:Pikachu,protagonist,1|exp:12059",
            "levelup|mon:Pikachu,protagonist,1|level:51|hp:96|atk:61|def:45|spa:56|spd:56|spe:96",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    pretty_assertions::assert_eq!(
        battle.player_data("protagonist").unwrap().caught,
        serde_json::from_str::<Vec<MonSummaryData>>(
            r#"[
                {
                    "name": "Metagross",
                    "species": "Metagross",
                    "level": 100,
                    "gender": "U",
                    "nature": "Hardy",
                    "shiny": false,
                    "ball": "Master Ball",
                    "hp": 270,
                    "friendship": 0,
                    "experience": 1250000,
                    "stats": {
                        "hp": 270,
                        "atk": 275,
                        "def": 265,
                        "spa": 195,
                        "spd": 185,
                        "spe": 145
                    },
                    "evs": {
                        "hp": 0,
                        "atk": 0,
                        "def": 0,
                        "spa": 0,
                        "spd": 0,
                        "spe": 0
                    },
                    "ivs": {
                        "hp": 0,
                        "atk": 0,
                        "def": 0,
                        "spa": 0,
                        "spd": 0,
                        "spe": 0
                    },
                    "moves": [],
                    "ability": "Clear Body",
                    "item": null,
                    "status": "fnt",
                    "hidden_power_type": "Fighting"
                }
            ]"#
        )
        .unwrap(),
    );
}

#[test]
fn level_100_metagross_critical_capture() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_wild_singles_battle(
        &data,
        65535,
        pikachu().unwrap(),
        metagross().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 0)]);

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item masterball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Master Ball|target:Metagross,wild,1",
            "catch|player:protagonist|mon:Metagross,wild,1|item:Master Ball|shakes:1|critical",
            "exp|mon:Pikachu,protagonist,1|exp:12059",
            "levelup|mon:Pikachu,protagonist,1|level:51|hp:96|atk:61|def:45|spa:56|spd:56|spe:96",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn level_50_magikarp_critical_capture() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut wild = magikarp().unwrap();
    wild.members[0].level = 50;
    let mut battle = make_wild_singles_battle(
        &data,
        65535,
        pikachu().unwrap(),
        wild,
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 0), (2, 0), (3, 0)]);

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item pokeball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Poké Ball|target:Magikarp,wild,1",
            "catch|player:protagonist|mon:Magikarp,wild,1|item:Poké Ball|shakes:1|critical",
            "exp|mon:Pikachu,protagonist,1|exp:401",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn level_100_sleeping_blissey_in_master_ball() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_wild_singles_battle(
        &data,
        65535,
        blissey().unwrap(),
        blissey().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item masterball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blissey,protagonist,1|name:False Swipe|target:Blissey,wild,1",
            "split|side:1",
            "damage|mon:Blissey,wild,1|health:420/714",
            "damage|mon:Blissey,wild,1|health:59/100",
            "move|mon:Blissey,wild,1|name:Belly Drum|target:Blissey,wild,1",
            "split|side:1",
            "damage|mon:Blissey,wild,1|health:63/714",
            "damage|mon:Blissey,wild,1|health:9/100",
            "boost|mon:Blissey,wild,1|stat:atk|by:6",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Blissey,protagonist,1|name:False Swipe|target:Blissey,wild,1",
            "split|side:1",
            "damage|mon:Blissey,wild,1|health:1/714",
            "damage|mon:Blissey,wild,1|health:1/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Blissey,protagonist,1|name:Sleep Powder|target:Blissey,wild,1",
            "status|mon:Blissey,wild,1|status:Sleep",
            "residual",
            "turn|turn:4",
            ["time"],
            "useitem|player:protagonist|name:Master Ball|target:Blissey,wild,1",
            "catch|player:protagonist|mon:Blissey,wild,1|item:Master Ball|shakes:4",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn level_100_sleeping_magikarp_critical_in_master_ball() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = pikachu().unwrap();
    team.members[0].level = 100;
    let mut wild = magikarp().unwrap();
    wild.members[0].level = 100;
    wild.members[0].moves.clear();
    let mut battle =
        make_wild_singles_battle(&data, 65535, team, wild, WildPlayerOptions::default()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    for _ in 0..5 {
        assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 0"), Ok(()));
        assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));
    }

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 0)]);

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item masterball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,protagonist,1|name:Sleep Powder|target:Magikarp,wild,1",
            "status|mon:Magikarp,wild,1|status:Sleep",
            "residual",
            "turn|turn:7",
            ["time"],
            "useitem|player:protagonist|name:Master Ball|target:Magikarp,wild,1",
            "catch|player:protagonist|mon:Magikarp,wild,1|item:Master Ball|shakes:1|critical",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 6, &expected_logs);
}

#[test]
fn uncatchable_wild_player_fails_catch() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_wild_singles_battle(
        &data,
        65535,
        pikachu().unwrap(),
        metagross().unwrap(),
        WildPlayerOptions {
            catchable: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item masterball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Master Ball|target:Metagross,wild,1",
            "uncatchable|player:protagonist|mon:Metagross,wild,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn trainer_mons_are_uncatchable() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_trainer_singles_battle(&data, 65535, pikachu().unwrap(), magikarp().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item masterball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Master Ball|target:Magikarp,trainer,1",
            "uncatchable|player:protagonist|mon:Magikarp,trainer,1|thief",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn cannot_throw_ball_at_semi_invulnerable_mon() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_wild_singles_battle(
        &data,
        65535,
        pikachu().unwrap(),
        magikarp().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item pokeball"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot use item: Poké Ball cannot be used on Magikarp")
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 0"), Ok(()));
}
