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
                    "name": "Zygarde",
                    "species": "Zygarde",
                    "ability": "Power Construct",
                    "item": "Zygardite",
                    "moves": [
                        "Recover",
                        "Dragon Claw"
                    ],
                    "nature": "Hardy",
                    "level": 100,
                    "persistent_battle_data": {
                        "hp": 1
                    }
                },
                {
                    "name": "Magikarp",
                    "species": "Magikarp",
                    "ability": "No Ability",
                    "moves": [],
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
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_mega_evolution(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn power_construct_transforms_zygarde_into_complete_forme() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Zygarde,player-1,1|name:Recover|target:Zygarde,player-1,1",
            "split|side:0",
            "heal|mon:Zygarde,player-1,1|health:164/326",
            "heal|mon:Zygarde,player-1,1|health:51/100",
            "split|side:1",
            ["specieschange", "player-2", "species:Zygarde-Complete", "health:217/542"],
            ["specieschange", "player-2", "species:Zygarde-Complete", "health:41/100"],
            "formechange|mon:Zygarde,player-2,1|species:Zygarde-Complete|from:ability:Power Construct",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn power_construct_transforms_zygarde_10_percent_into_complete_forme() {
    let mut team_1 = team().unwrap();
    team_1.members[0].species = "Zygarde-10".to_owned();
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Zygarde,player-2,1|name:Recover|target:Zygarde,player-2,1",
            "split|side:1",
            "heal|mon:Zygarde,player-2,1|health:164/326",
            "heal|mon:Zygarde,player-2,1|health:51/100",
            "split|side:0",
            ["specieschange", "player-1", "species:Zygarde-Complete", "health:325/542"],
            ["specieschange", "player-1", "species:Zygarde-Complete", "health:60/100"],
            "formechange|mon:Zygarde,player-1,1|species:Zygarde-Complete|from:ability:Power Construct",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn power_construct_reverts_on_faint() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item revive,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["specieschange", "player-1", "species:Zygarde-Complete", "health:217/542"],
            ["specieschange", "player-1", "species:Zygarde-Complete", "health:41/100"],
            "formechange|mon:Zygarde,player-1,1|species:Zygarde-Complete|from:ability:Power Construct",
            "split|side:1",
            ["specieschange", "player-2", "species:Zygarde-Complete", "health:217/542"],
            ["specieschange", "player-2", "species:Zygarde-Complete", "health:41/100"],
            "formechange|mon:Zygarde,player-2,1|species:Zygarde-Complete|from:ability:Power Construct",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Zygarde,player-2,1|name:Dragon Claw|target:Zygarde,player-1,1",
            "supereffective|mon:Zygarde,player-1,1",
            "split|side:0",
            "damage|mon:Zygarde,player-1,1|health:53/542",
            "damage|mon:Zygarde,player-1,1|health:10/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Zygarde,player-2,1|name:Dragon Claw|target:Zygarde,player-1,1",
            "supereffective|mon:Zygarde,player-1,1",
            "split|side:0",
            "damage|mon:Zygarde,player-1,1|health:0",
            "damage|mon:Zygarde,player-1,1|health:0",
            "faint|mon:Zygarde,player-1,1",
            "split|side:0",
            ["specieschange", "player-1", "species:Zygarde|", "health:0"],
            ["specieschange", "player-1", "species:Zygarde|", "health:0"],
            "formechange|mon:Zygarde,player-1,1|species:Zygarde|from:Faint",
            "residual",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Magikarp"],
            ["switch", "player-1", "Magikarp"],
            "turn|turn:4",
            "continue",
            "useitem|player:player-1|name:Revive|target:Zygarde,player-1",
            "revive|mon:Zygarde,player-1|from:item:Revive",
            "split|side:0",
            "sethp|mon:Zygarde,player-1|health:163/326",
            "sethp|mon:Zygarde,player-1|health:50/100",
            "residual",
            "turn|turn:5",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Zygarde"],
            ["switch", "player-1", "Zygarde"],
            "split|side:0",
            ["specieschange", "player-1", "species:Zygarde-Complete", "health:379/542"],
            ["specieschange", "player-1", "species:Zygarde-Complete", "health:70/100"],
            "formechange|mon:Zygarde,player-1,1|species:Zygarde-Complete|from:ability:Power Construct",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn power_construct_enables_mega_evolution() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Zygarde cannot mega evolve");
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["specieschange", "player-1", "species:Zygarde-Mega"],
            ["specieschange", "player-1", "species:Zygarde-Mega"],
            "mega|mon:Zygarde,player-1,1|species:Zygarde-Mega|from:item:Zygardite",
            "move|mon:Zygarde,player-1,1|name:Recover|target:Zygarde,player-1,1",
            "split|side:0",
            "heal|mon:Zygarde,player-1,1|health:488/542",
            "heal|mon:Zygarde,player-1,1|health:91/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}

#[test]
fn zygarde_mega_and_complete_forme_revert_on_faint() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Zygarde cannot mega evolve");
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Zygarde,player-2,1|name:Dragon Claw|target:Zygarde,player-1,1",
            "supereffective|mon:Zygarde,player-1,1",
            "split|side:0",
            "damage|mon:Zygarde,player-1,1|health:0",
            "damage|mon:Zygarde,player-1,1|health:0",
            "faint|mon:Zygarde,player-1,1",
            "split|side:0",
            ["specieschange", "player-1", "species:Zygarde|"],
            ["specieschange", "player-1", "species:Zygarde|"],
            "revertmega|mon:Zygarde,player-1,1|species:Zygarde|from:Faint",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 3, &expected_logs);
}
