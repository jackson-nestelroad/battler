use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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
    assert_logs_since_turn_eq,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Copperajah",
                    "species": "Copperajah",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Metal Claw"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "gigantamax_factor": true
                },
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Glalie",
                    "species": "Glalie",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [],
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
        .with_dynamax(true)
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
fn gmax_steel_surge_adds_entry_hazard_with_steel_effectiveness() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "gigantamax|mon:Copperajah,player-1,1|species:Copperajah-Gmax",
            "dynamax|mon:Copperajah,player-1,1",
            "split|side:0",
            "sethp|mon:Copperajah,player-1,1|health:273/273",
            "sethp|mon:Copperajah,player-1,1|health:100/100",
            "move|mon:Copperajah,player-1,1|name:G-Max Steelsurge|target:Copperajah,player-2,1",
            "resisted|mon:Copperajah,player-2,1",
            "split|side:1",
            "damage|mon:Copperajah,player-2,1|health:121/182",
            "damage|mon:Copperajah,player-2,1|health:67/100",
            "sidestart|side:1|move:G-Max Steelsurge",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:1|name:Pikachu|health:95/95|species:Pikachu|level:50|gender:M",
            "switch|player:player-2|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|from:move:G-Max Steelsurge|health:90/95",
            "damage|mon:Pikachu,player-2,1|from:move:G-Max Steelsurge|health:95/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:1|name:Glalie|health:140/140|species:Glalie|level:50|gender:M",
            "switch|player:player-2|position:1|name:Glalie|health:100/100|species:Glalie|level:50|gender:M",
            "split|side:1",
            "damage|mon:Glalie,player-2,1|from:move:G-Max Steelsurge|health:105/140",
            "damage|mon:Glalie,player-2,1|from:move:G-Max Steelsurge|health:75/100",
            "revertgigantamax|mon:Copperajah,player-1,1|species:Copperajah",
            "revertdynamax|mon:Copperajah,player-1,1",
            "split|side:0",
            "sethp|mon:Copperajah,player-1,1|health:182/182",
            "sethp|mon:Copperajah,player-1,1|health:100/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
