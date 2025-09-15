use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    WildPlayerOptions,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn jolteon() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Jolteon",
                    "species": "Jolteon",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Bind",
                        "Dig"
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

fn primeape() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Primeape",
                    "species": "Primeape",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
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

fn low_level_pikachu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
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

fn ralts() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ralts",
                    "species": "Ralts",
                    "ability": "No Ability",
                    "moves": [
                        "Teleport"
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

fn make_wild_singles_battle(
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
    wild_options: WildPlayerOptions,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .add_wild_mon_to_side_2("wild", "Wild", wild_options)
        .with_team("protagonist", team_1)
        .with_team("wild", team_2)
        .build(static_local_data_store())
}

fn make_wild_multi_battle<'d>(
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .with_team("protagonist", team);

    for (i, wild) in wild.into_iter().enumerate() {
        let id = format!("wild-{}", i);
        builder = builder
            .add_wild_mon_to_side_2(&id, "Wild", wild_options)
            .with_team(&id, wild);
    }

    builder.build(static_local_data_store())
}

fn make_trainer_singles_battle(
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .add_player_to_side_2("trainer", "Trainer")
        .with_team("protagonist", team_1)
        .with_team("trainer", team_2)
        .build(static_local_data_store())
}

#[test]
fn player_escapes_with_higher_speed() {
    let mut battle = make_wild_singles_battle(
        0,
        jolteon().unwrap(),
        primeape().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "escape"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "switchout|mon:Jolteon,protagonist,1",
            "escaped|player:protagonist",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mon_cannot_escape_with_locked_move() {
    let mut battle = make_wild_singles_battle(
        0,
        jolteon().unwrap(),
        primeape().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "escape"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot escape: Jolteon must use a move")
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));
}

#[test]
fn wild_player_can_escape() {
    let mut battle = make_wild_singles_battle(
        0,
        jolteon().unwrap(),
        primeape().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "escape"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "switchout|mon:Primeape,wild,1",
            "escaped|player:wild",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn player_escapes_with_lower_speed() {
    let mut battle = make_wild_singles_battle(
        3245467,
        low_level_pikachu().unwrap(),
        primeape().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "escape"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "escape"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "escape"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "escape"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "escape"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "cannotescape|player:protagonist",
            "residual",
            "turn|turn:2",
            ["time"],
            "cannotescape|player:protagonist",
            "residual",
            "turn|turn:3",
            ["time"],
            "cannotescape|player:protagonist",
            "residual",
            "turn|turn:4",
            ["time"],
            "cannotescape|player:protagonist",
            "residual",
            "turn|turn:5",
            ["time"],
            "switchout|mon:Pikachu,protagonist,1",
            "escaped|player:protagonist",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn player_escapes_with_smoke_ball() {
    let mut team = low_level_pikachu().unwrap();
    team.members[0].item = Some("Smoke Ball".to_owned());
    let mut battle = make_wild_singles_battle(
        3245467,
        team,
        primeape().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "escape"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "activate|mon:Pikachu,protagonist,1|item:Smoke Ball",
            "switchout|mon:Pikachu,protagonist,1",
            "escaped|player:protagonist",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn player_escapes_with_poke_doll() {
    let mut team = low_level_pikachu().unwrap();
    team.bag.items.insert("Poke Doll".to_owned(), 1);
    let mut battle = make_wild_singles_battle(
        3245467,
        team,
        primeape().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item pokedoll"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Poké Doll|target:Pikachu,protagonist,1",
            "switchout|mon:Pikachu,protagonist,1",
            "escaped|player:protagonist",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn run_away_escapes_immediately() {
    let mut low_level_pikachu = low_level_pikachu().unwrap();
    low_level_pikachu.members[0].ability = "Run Away".to_owned();
    let mut battle = make_wild_singles_battle(
        3245467,
        low_level_pikachu,
        primeape().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "escape"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "switchout|mon:Pikachu,protagonist,1",
            "escaped|player:protagonist",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn cannot_escape_trainer_battle() {
    let mut battle =
        make_trainer_singles_battle(0, jolteon().unwrap(), primeape().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "escape"),
        Err(err) => assert!(format!("{err:#}").contains("you cannot escape"))
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("trainer", "escape"),
        Err(err) => assert!(format!("{err:#}").contains("you cannot escape"))
    );
}

#[test]
fn wild_players_escape_individually() {
    let mut battle = make_wild_multi_battle(
        0,
        jolteon().unwrap(),
        vec![
            low_level_pikachu().unwrap(),
            low_level_pikachu().unwrap(),
            low_level_pikachu().unwrap(),
        ],
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-0", "escape"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 0,2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-2", "escape"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "switchout|mon:Pikachu,wild-0,1",
            "escaped|player:wild-0",
            "move|mon:Jolteon,protagonist,2|name:Tackle|target:Pikachu,wild-1,2",
            "split|side:1",
            "damage|mon:Pikachu,wild-1,2|health:0",
            "damage|mon:Pikachu,wild-1,2|health:0",
            "faint|mon:Pikachu,wild-1,2",
            "exp|mon:Jolteon,protagonist,2|exp:6",
            "move|mon:Pikachu,wild-2,3|name:Tackle|target:Jolteon,protagonist,2",
            "split|side:0",
            "damage|mon:Jolteon,protagonist,2|health:124/125",
            "damage|mon:Jolteon,protagonist,2|health:99/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,wild-2,3|name:Tackle|target:Jolteon,protagonist,2",
            "split|side:0",
            "damage|mon:Jolteon,protagonist,2|health:123/125",
            "damage|mon:Jolteon,protagonist,2|health:99/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "switchout|mon:Pikachu,wild-2,3",
            "escaped|player:wild-2",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn teleport_escapes_wild_battle() {
    let mut battle = make_wild_singles_battle(
        40920948098,
        ralts().unwrap(),
        primeape().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ralts,protagonist,1|name:Teleport|target:Ralts,protagonist,1",
            "switchout|mon:Ralts,protagonist,1",
            "escaped|player:protagonist",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn cannot_escape_partially_trapping_move() {
    let mut battle = make_wild_singles_battle(
        0,
        jolteon().unwrap(),
        primeape().unwrap(),
        WildPlayerOptions::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("wild", "escape"),
        Err(err) => assert!(format!("{err:#}").contains("you cannot escape"))
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Jolteon,protagonist,1|name:Bind|target:Primeape,wild,1",
            "split|side:1",
            "damage|mon:Primeape,wild,1|health:117/125",
            "damage|mon:Primeape,wild,1|health:94/100",
            "activate|mon:Primeape,wild,1|move:Bind|of:Jolteon,protagonist,1",
            "split|side:1",
            "damage|mon:Primeape,wild,1|from:move:Bind|health:102/125",
            "damage|mon:Primeape,wild,1|from:move:Bind|health:82/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Primeape,wild,1|name:Tackle|target:Jolteon,protagonist,1",
            "split|side:0",
            "damage|mon:Jolteon,protagonist,1|health:97/125",
            "damage|mon:Jolteon,protagonist,1|health:78/100",
            "split|side:1",
            "damage|mon:Primeape,wild,1|from:move:Bind|health:87/125",
            "damage|mon:Primeape,wild,1|from:move:Bind|health:70/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
