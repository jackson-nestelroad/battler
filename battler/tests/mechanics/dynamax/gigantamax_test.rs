use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
    static_local_data_store,
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
                    "level": 50,
                    "gigantamax_factor": true
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
fn mon_with_gigantamax_factor_changes_forme_on_dynamax() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1,dyna"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "gigantamax|mon:Venusaur,player-1,1|species:Venusaur-Gmax",
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
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gigantamax_ends_on_switch() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "gigantamax|mon:Venusaur,player-1,1|species:Venusaur-Gmax",
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
            "revertgigantamax|mon:Venusaur,player-1,1|species:Venusaur",
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
fn gigantamax_ends_on_faint() {
    let mut battle = make_battle(
        100,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
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
            "gigantamax|mon:Venusaur,player-1,1|species:Venusaur-Gmax",
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
            "revertgigantamax|mon:Venusaur,player-1,1|species:Venusaur",
            "revertdynamax|mon:Venusaur,player-1,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
