use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
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
                    "ability": "Overgrow",
                    "item": "Venusaurite",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "Blaze",
                    "item": "Charizardite X",
                    "moves": [
                        "Ember",
                        "Skill Swap",
                        "Guillotine"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "Torrent",
                    "item": "Blastoisinite",
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
        .with_mega_evolution(true)
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
fn one_mon_can_mega_evolve() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert!(request.active[0].can_mega_evolve);
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            "switch|player:player-2|position:1|name:Charizard|health:138/138|species:Charizard|level:50|gender:U",
            "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:U",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Ember|target:Venusaur,player-1,1",
            "supereffective|mon:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:78/140",
            "damage|mon:Venusaur,player-1,1|health:56/100",
            "move|mon:Venusaur,player-1,1|name:Tackle|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:118/138",
            "damage|mon:Charizard,player-2,1|health:86/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:0",
            "specieschange|player:player-1|position:1|name:Venusaur|health:78/140|species:Venusaur-Mega|level:50|gender:U",
            "specieschange|player:player-1|position:1|name:Venusaur|health:56/100|species:Venusaur-Mega|level:50|gender:U",
            "mega|mon:Venusaur,player-1,1|species:Venusaur-Mega|from:item:Venusaurite",
            "move|mon:Charizard,player-2,1|name:Ember|target:Venusaur,player-1,1",
            "supereffective|mon:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:48/140",
            "damage|mon:Venusaur,player-1,1|health:35/100",
            "move|mon:Venusaur,player-1,1|name:Tackle|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:94/138",
            "damage|mon:Charizard,player-2,1|health:69/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Skill Swap|target:Venusaur,player-1,1",
            "activate|mon:Venusaur,player-1,1|move:Skill Swap|of:Charizard,player-2,1",
            "abilityend|mon:Charizard,player-2,1|ability:Blaze|from:move:Skill Swap|of:Venusaur,player-1,1",
            "ability|mon:Charizard,player-2,1|ability:Thick Fat|from:move:Skill Swap|of:Venusaur,player-1,1",
            "abilityend|mon:Venusaur,player-1,1|ability:Thick Fat|from:move:Skill Swap|of:Charizard,player-2,1",
            "ability|mon:Venusaur,player-1,1|ability:Blaze|from:move:Skill Swap|of:Charizard,player-2,1",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert!(!request.active[0].can_mega_evolve);
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Charizard cannot mega evolve");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
}

#[test]
fn mega_evolution_persists_on_switch() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["specieschange", "player-1", "species:Venusaur-Mega"],
            ["specieschange", "player-1", "species:Venusaur-Mega"],
            "mega|mon:Venusaur,player-1,1|species:Venusaur-Mega|from:item:Venusaurite",
            "move|mon:Venusaur,player-1,1|name:Tackle|target:Venusaur,player-2,1",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|health:117/140",
            "damage|mon:Venusaur,player-2,1|health:84/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "species:Charizard"],
            ["switch", "player-1", "species:Charizard"],
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "species:Venusaur-Mega"],
            ["switch", "player-1", "species:Venusaur-Mega"],
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mega_evolution_reverts_on_faint() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item revive,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert!(!request.active[0].can_mega_evolve);
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Charizard"],
            ["switch", "player-2", "Charizard"],
            "split|side:0",
            ["specieschange", "player-1", "species:Venusaur-Mega"],
            ["specieschange", "player-1", "species:Venusaur-Mega"],
            "mega|mon:Venusaur,player-1,1|species:Venusaur-Mega|from:item:Venusaurite",
            "move|mon:Venusaur,player-1,1|name:Tackle|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:114/138",
            "damage|mon:Charizard,player-2,1|health:83/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Guillotine|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:0",
            "damage|mon:Venusaur,player-1,1|health:0",
            "ohko|mon:Venusaur,player-1,1",
            "faint|mon:Venusaur,player-1,1",
            "split|side:0",
            ["specieschange", "player-1", "species:Venusaur|"],
            ["specieschange", "player-1", "species:Venusaur|"],
            "revertmega|mon:Venusaur,player-1,1|species:Venusaur|from:Faint",
            "residual",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Charizard"],
            ["switch", "player-1", "Charizard"],
            "turn|turn:3",
            ["time"],
            "useitem|player:player-1|name:Revive|target:Venusaur,player-1",
            "revive|mon:Venusaur,player-1|from:item:Revive",
            "split|side:0",
            "sethp|mon:Venusaur,player-1|health:70/140",
            "sethp|mon:Venusaur,player-1|health:50/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "species:Venusaur|"],
            ["switch", "player-1", "species:Venusaur|"],
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Skill Swap|target:Venusaur,player-1,1",
            "activate|mon:Venusaur,player-1,1|move:Skill Swap|of:Charizard,player-2,1",
            "abilityend|mon:Charizard,player-2,1|ability:Blaze|from:move:Skill Swap|of:Venusaur,player-1,1",
            "ability|mon:Charizard,player-2,1|ability:Overgrow|from:move:Skill Swap|of:Venusaur,player-1,1",
            "abilityend|mon:Venusaur,player-1,1|ability:Overgrow|from:move:Skill Swap|of:Charizard,player-2,1",
            "ability|mon:Venusaur,player-1,1|ability:Blaze|from:move:Skill Swap|of:Charizard,player-2,1",
            "move|mon:Venusaur,player-1,1|name:Tackle|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:94/138",
            "damage|mon:Charizard,player-2,1|health:69/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
