use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    common::{
        Error,
        WrapResultError,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_error_message,
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Overgrow",
                    "moves": [
                        "Thunder Wave"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "Blaze",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Cheri Berry"
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
) -> Result<PublicCoreBattle, Error> {
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
fn cheri_berry_heals_paralysis() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_error_message(
        battle.set_player_choice("protagonist", "item Cheri Berry,-1"),
        "cannot use item: Cheri Berry cannot be used on Bulbasaur",
    );
    assert_eq!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_eq!(
        battle.set_player_choice("protagonist", "item Cheri Berry,-1"),
        Ok(())
    );
    assert_eq!(battle.set_player_choice("trainer", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,trainer,1|name:Thunder Wave|target:Bulbasaur,protagonist,1",
            "status|mon:Bulbasaur,protagonist,1|status:Paralysis",
            "residual",
            "turn|turn:2",
            ["time"],
            "useitem|player:protagonist|name:Cheri Berry|target:Bulbasaur,protagonist,1",
            "curestatus|mon:Bulbasaur,protagonist,1|status:Paralysis|from:item:Cheri Berry",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn cheri_berry_heals_paralysis_of_inactive_mon() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("protagonist", "switch 1"), Ok(()));
    assert_eq!(battle.set_player_choice("trainer", "pass"), Ok(()));
    assert_eq!(
        battle.set_player_choice("protagonist", "item Cheri Berry,-1"),
        Ok(())
    );
    assert_eq!(battle.set_player_choice("trainer", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,trainer,1|name:Thunder Wave|target:Bulbasaur,protagonist,1",
            "status|mon:Bulbasaur,protagonist,1|status:Paralysis",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["switch", "protagonist", "Charmander"],
            ["switch", "protagonist", "Charmander"],
            "residual",
            "turn|turn:3",
            ["time"],
            "useitem|player:protagonist|name:Cheri Berry|target:Bulbasaur,protagonist",
            "curestatus|mon:Bulbasaur,protagonist|status:Paralysis|from:item:Cheri Berry",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn cheri_berry_can_be_eaten_automatically() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("protagonist", "switch 1"), Ok(()));
    assert_eq!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_error_message(
        battle.set_player_choice("protagonist", "item Cheri Berry,-2"),
        "cannot use item: Cheri Berry cannot be used on Charmander",
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "protagonist", "Charmander"],
            ["switch", "protagonist", "Charmander"],
            "move|mon:Bulbasaur,trainer,1|name:Thunder Wave|target:Charmander,protagonist,1",
            "status|mon:Charmander,protagonist,1|status:Paralysis",
            "itemend|mon:Charmander,protagonist,1|item:Cheri Berry|eat",
            "curestatus|mon:Charmander,protagonist,1|status:Paralysis|from:item:Cheri Berry",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
