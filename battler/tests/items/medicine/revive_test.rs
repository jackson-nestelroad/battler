use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    error::{
        Error,
        WrapResultError,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "Static",
                    "moves": [
                        "Earthquake"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Pichu",
                    "species": "Pichu",
                    "ability": "Static",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ],
            "bag": {
                "items": {
                    "Revive": 1,
                    "Max Revive": 1
                }
            }
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .add_player_to_side_2("trainer", "Trainer")
        .with_team("protagonist", team_1)
        .with_team("trainer", team_2)
        .build(data)
}

#[test]
fn revive_revives_fainted_mon_to_half_health() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "switch 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item revive,-2"),
        Err(err) => assert_eq!(err.full_description(), "cannot use item: Revive cannot be used on Pichu")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item revive,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,trainer,1|name:Earthquake",
            "supereffective|mon:Pikachu,protagonist,1",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,1|health:0",
            "damage|mon:Pikachu,protagonist,1|health:0",
            "faint|mon:Pikachu,protagonist,1",
            "residual",
            ["time"],
            "split|side:0",
            ["switch", "protagonist", "Pichu"],
            ["switch", "protagonist", "Pichu"],
            "turn|turn:2",
            ["time"],
            "useitem|player:protagonist|name:Revive|target:Pikachu,protagonist",
            "revive|mon:Pikachu,protagonist|from:item:Revive",
            "split|side:0",
            "sethp|mon:Pikachu,protagonist|health:47/95",
            "sethp|mon:Pikachu,protagonist|health:50/100",
            "move|mon:Pikachu,trainer,1|name:Earthquake",
            "supereffective|mon:Pichu,protagonist,1",
            "split|side:0",
            "damage|mon:Pichu,protagonist,1|health:0",
            "damage|mon:Pichu,protagonist,1|health:0",
            "faint|mon:Pichu,protagonist,1",
            "residual",
            ["time"],
            "split|side:0",
            ["switch", "protagonist", "Pikachu"],
            ["switch", "protagonist", "Pikachu"],
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,trainer,1|name:Earthquake",
            "supereffective|mon:Pikachu,protagonist,1",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,1|health:0",
            "damage|mon:Pikachu,protagonist,1|health:0",
            "faint|mon:Pikachu,protagonist,1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn max_revive_revives_fainted_mon_to_full_health() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "switch 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item maxrevive,-2"),
        Err(err) => assert_eq!(err.full_description(), "cannot use item: Max Revive cannot be used on Pichu")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item maxrevive,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,trainer,1|name:Earthquake",
            "supereffective|mon:Pikachu,protagonist,1",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,1|health:0",
            "damage|mon:Pikachu,protagonist,1|health:0",
            "faint|mon:Pikachu,protagonist,1",
            "residual",
            ["time"],
            "split|side:0",
            ["switch", "protagonist", "Pichu"],
            ["switch", "protagonist", "Pichu"],
            "turn|turn:2",
            ["time"],
            "useitem|player:protagonist|name:Max Revive|target:Pikachu,protagonist",
            "revive|mon:Pikachu,protagonist|from:item:Max Revive",
            "split|side:0",
            "sethp|mon:Pikachu,protagonist|health:95/95",
            "sethp|mon:Pikachu,protagonist|health:100/100",
            "move|mon:Pikachu,trainer,1|name:Earthquake",
            "supereffective|mon:Pichu,protagonist,1",
            "split|side:0",
            "damage|mon:Pichu,protagonist,1|health:0",
            "damage|mon:Pichu,protagonist,1|health:0",
            "faint|mon:Pichu,protagonist,1",
            "residual",
            ["time"],
            "split|side:0",
            ["switch", "protagonist", "Pikachu"],
            ["switch", "protagonist", "Pikachu"],
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,trainer,1|name:Earthquake",
            "supereffective|mon:Pikachu,protagonist,1",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,1|health:0",
            "damage|mon:Pikachu,protagonist,1|health:0",
            "faint|mon:Pikachu,protagonist,1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
