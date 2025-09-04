use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_start_eq,
    assert_logs_since_turn_eq,
};

fn groudon() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Groudon",
                    "species": "Groudon",
                    "ability": "Drought",
                    "item": "Red Orb",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn kyogre() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Kyogre",
                    "species": "Kyogre",
                    "ability": "Drizzle",
                    "item": "Blue Orb",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Rayquaza",
                    "species": "Rayquaza",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
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
        .with_primal_reversion(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn groudon_undergoes_primal_reversion_on_switch_with_red_orb() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = groudon().unwrap();
    team.members[0].item = Some("Blue Orb".to_owned());
    let mut battle = make_battle(&data, 0, groudon().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "species:Groudon|"],
            ["switch", "player-1", "species:Groudon|"],
            "split|side:1",
            ["switch", "player-2", "species:Groudon|"],
            ["switch", "player-2", "species:Groudon|"],
            "weather|weather:Harsh Sunlight|from:ability:Drought|of:Groudon,player-2,1",
            "split|side:0",
            ["specieschange", "player-1", "species:Groudon-Primal"],
            ["specieschange", "player-1", "species:Groudon-Primal"],
            "primal|mon:Groudon,player-1,1|species:Groudon-Primal|from:item:Red Orb",
            "weather|weather:Extremely Harsh Sunlight|from:ability:Desolate Land|of:Groudon,player-1,1",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn kyogre_undergoes_primal_reversion_on_switch_with_blue_orb() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = kyogre().unwrap();
    team.members[0].item = Some("Red Orb".to_owned());
    let mut battle = make_battle(&data, 0, kyogre().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "species:Kyogre|"],
            ["switch", "player-1", "species:Kyogre|"],
            "split|side:1",
            ["switch", "player-2", "species:Kyogre|"],
            ["switch", "player-2", "species:Kyogre|"],
            "weather|weather:Rain|from:ability:Drizzle|of:Kyogre,player-2,1",
            "split|side:0",
            ["specieschange", "player-1", "species:Kyogre-Primal"],
            ["specieschange", "player-1", "species:Kyogre-Primal"],
            "primal|mon:Kyogre,player-1,1|species:Kyogre-Primal|from:item:Blue Orb",
            "weather|weather:Heavy Rain|from:ability:Primordial Sea|of:Kyogre,player-1,1",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn primal_reversion_not_reverted_on_faint_and_revive() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team_1 = kyogre().unwrap();
    team_1.members[0].level = 1;
    let mut team_2 = kyogre().unwrap();
    team_2.members[0].item = None;
    let mut battle = make_battle(&data, 0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
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
            "move|mon:Kyogre,player-2,1|name:Tackle|target:Kyogre,player-1,1",
            "split|side:0",
            "damage|mon:Kyogre,player-1,1|health:0",
            "damage|mon:Kyogre,player-1,1|health:0",
            "faint|mon:Kyogre,player-1,1",
            "clearweather",
            "residual",
            ["time"],
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "turn|turn:2",
            ["time"],
            "useitem|player:player-1|name:Revive|target:Kyogre,player-1",
            "revive|mon:Kyogre,player-1|from:item:Revive",
            "split|side:0",
            "sethp|mon:Kyogre,player-1|health:6/13",
            "sethp|mon:Kyogre,player-1|health:47/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "species:Kyogre-Primal"],
            ["switch", "player-1", "species:Kyogre-Primal"],
            "weather|weather:Heavy Rain|from:ability:Primordial Sea|of:Kyogre,player-1,1",
            "weather|weather:Heavy Rain|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
