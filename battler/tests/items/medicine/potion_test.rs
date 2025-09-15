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
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "Static",
                    "moves": [
                        "Tackle",
                        "Embargo",
                        "Heal Block"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Eevee",
                    "species": "Eevee",
                    "ability": "Run Away",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ],
            "bag": {
                "items": {
                    "Potion": 2
                }
            }
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
        .with_bag_items(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .add_player_to_side_2("trainer", "Trainer")
        .with_team("protagonist", team_1)
        .with_team("trainer", team_2)
        .build(static_local_data_store())
}

#[test]
fn potion_heals_20_hp() {
    let mut battle = make_battle(
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item potion,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,trainer,1|name:Tackle|target:Pikachu,protagonist,1",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,1|health:71/95",
            "damage|mon:Pikachu,protagonist,1|health:75/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "useitem|player:protagonist|name:Potion|target:Pikachu,protagonist,1",
            "split|side:0",
            "heal|mon:Pikachu,protagonist,1|from:item:Potion|health:91/95",
            "heal|mon:Pikachu,protagonist,1|from:item:Potion|health:96/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn using_item_removes_from_bag() {
    let mut battle = make_battle(
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item potion,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item potion,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item potion,-1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot use item: bag contains no Potion")
    );
}

#[test]
fn potion_can_heal_inactive_mon() {
    let mut battle = make_battle(
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item potion,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,trainer,1|name:Tackle|target:Pikachu,protagonist,1",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,1|health:71/95",
            "damage|mon:Pikachu,protagonist,1|health:75/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["switch", "protagonist", "Eevee"],
            ["switch", "protagonist", "Eevee"],
            "residual",
            "turn|turn:3",
            ["time"],
            "useitem|player:protagonist|name:Potion|target:Pikachu,protagonist",
            "split|side:0",
            "heal|mon:Pikachu,protagonist|from:item:Potion|health:91/95",
            "heal|mon:Pikachu,protagonist|from:item:Potion|health:96/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn potion_fails_at_max_hp() {
    let mut battle = make_battle(
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item potion,-1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot use item: Potion cannot be used on Pikachu")
    );
}

#[test]
fn potion_fails_on_fainted_mon() {
    let mut battle = make_battle(
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "switch 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item potion,-1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot use item: Potion cannot be used on Pikachu")
    );
}

#[test]
fn potion_fails_on_foe() {
    let mut battle = make_battle(
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item potion,1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot use item: invalid target for Potion")
    );
}

#[test]
fn embargo_prevents_potion_usage_from_bag() {
    let mut battle = make_battle(
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item potion,-1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot use item: Potion cannot be used on Pikachu")
    );
}

#[test]
fn potion_heals_despite_heal_block() {
    let mut battle = make_battle(
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 2"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item potion,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,trainer,1|name:Tackle|target:Pikachu,protagonist,1",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,1|health:71/95",
            "damage|mon:Pikachu,protagonist,1|health:75/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,trainer,1|name:Heal Block",
            "start|mon:Pikachu,protagonist,1|move:Heal Block",
            "residual",
            "turn|turn:3",
            ["time"],
            "useitem|player:protagonist|name:Potion|target:Pikachu,protagonist,1",
            "split|side:0",
            "heal|mon:Pikachu,protagonist,1|from:item:Potion|health:91/95",
            "heal|mon:Pikachu,protagonist,1|from:item:Potion|health:96/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
