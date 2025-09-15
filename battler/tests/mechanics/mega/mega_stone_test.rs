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

fn gengar() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gengar",
                    "species": "Gengar",
                    "ability": "No Ability",
                    "item": "Gengarite",
                    "moves": [
                        "Fling",
                        "Thief",
                        "Embargo"
                    ],
                    "nature": "Hardy",
                    "level": 50
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
fn cannot_fling_mega_stone_before_mega_evolution() {
    let mut battle = make_battle(
        0,
        gengar().unwrap(),
        gengar().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gengar,player-1,1|name:Fling|noanim",
            "fail|mon:Gengar,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn cannot_fling_mega_stone_after_mega_evolution() {
    let mut battle = make_battle(
        0,
        gengar().unwrap(),
        gengar().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["specieschange", "player-1", "species:Gengar-Mega"],
            ["specieschange", "player-1", "species:Gengar-Mega"],
            "mega|mon:Gengar,player-1,1|species:Gengar-Mega|from:item:Gengarite",
            "move|mon:Gengar,player-1,1|name:Fling|noanim",
            "fail|mon:Gengar,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn can_fling_mega_stone_for_different_species() {
    let mut team = gengar().unwrap();
    team.members[0].item = Some("Venusaurite".to_owned());
    let mut battle = make_battle(0, team, gengar().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gengar,player-1,1|name:Fling|target:Gengar,player-2,1",
            "activate|mon:Gengar,player-1,1|move:Fling|item:Venusaurite",
            "supereffective|mon:Gengar,player-2,1",
            "split|side:1",
            "damage|mon:Gengar,player-2,1|health:46/120",
            "damage|mon:Gengar,player-2,1|health:39/100",
            "itemend|mon:Gengar,player-1,1|item:Venusaurite|silent|from:move:Fling",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn cannot_take_mega_stone_before_mega_evolution() {
    let mut battle = make_battle(
        0,
        gengar().unwrap(),
        gengar().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gengar,player-2,1|name:Thief|target:Gengar,player-1,1",
            "supereffective|mon:Gengar,player-1,1",
            "split|side:0",
            "damage|mon:Gengar,player-1,1|health:62/120",
            "damage|mon:Gengar,player-1,1|health:52/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn cannot_take_mega_stone_after_mega_evolution() {
    let mut battle = make_battle(
        0,
        gengar().unwrap(),
        gengar().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["specieschange", "player-1", "species:Gengar-Mega"],
            ["specieschange", "player-1", "species:Gengar-Mega"],
            "mega|mon:Gengar,player-1,1|species:Gengar-Mega|from:item:Gengarite",
            "move|mon:Gengar,player-1,1|name:Fling|noanim",
            "fail|mon:Gengar,player-1,1",
            "move|mon:Gengar,player-2,1|name:Thief|target:Gengar,player-1,1",
            "supereffective|mon:Gengar,player-1,1",
            "split|side:0",
            "damage|mon:Gengar,player-1,1|health:76/120",
            "damage|mon:Gengar,player-1,1|health:64/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn embargo_cannot_stop_mega_evolution() {
    let mut battle = make_battle(
        0,
        gengar().unwrap(),
        gengar().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gengar,player-2,1|name:Embargo|target:Gengar,player-1,1",
            "start|mon:Gengar,player-1,1|move:Embargo",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["specieschange", "player-1", "species:Gengar-Mega"],
            ["specieschange", "player-1", "species:Gengar-Mega"],
            "mega|mon:Gengar,player-1,1|species:Gengar-Mega|from:item:Gengarite",
            "move|mon:Gengar,player-1,1|name:Fling|noanim",
            "fail|mon:Gengar,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
