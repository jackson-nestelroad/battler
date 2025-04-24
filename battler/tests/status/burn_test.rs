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
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn gengar() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gengar",
                    "species": "Gengar",
                    "ability": "No Ability",
                    "moves": [
                        "Will-O-Wisp",
                        "Knock Off"
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

fn charizard() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
        "members": [
            {
                "name": "Charizard",
                "species": "Charizard",
                "ability": "No Ability",
                "moves": [],
                "nature": "Hardy",
                "gender": "M",
                "level": 50
            }
        ]
    }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(1234566456456)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn burn_applies_residual_damage_and_modifies_attack() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, gengar().unwrap(), gengar().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[

            "move|mon:Gengar,player-1,1|name:Will-O-Wisp|target:Gengar,player-2,1",
            "status|mon:Gengar,player-2,1|status:Burn",
            "split|side:1",
            "damage|mon:Gengar,player-2,1|from:status:Burn|health:113/120",
            "damage|mon:Gengar,player-2,1|from:status:Burn|health:95/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Gengar,player-1,1|name:Will-O-Wisp|noanim",
            "fail|mon:Gengar,player-1,1",
            "split|side:1",
            "damage|mon:Gengar,player-2,1|from:status:Burn|health:106/120",
            "damage|mon:Gengar,player-2,1|from:status:Burn|health:89/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Gengar,player-1,1|name:Knock Off|target:Gengar,player-2,1",
            "supereffective|mon:Gengar,player-2,1",
            "split|side:1",
            "damage|mon:Gengar,player-2,1|health:42/120",
            "damage|mon:Gengar,player-2,1|health:35/100",
            "move|mon:Gengar,player-2,1|name:Knock Off|target:Gengar,player-1,1",
            "supereffective|mon:Gengar,player-1,1",
            "split|side:0",
            "damage|mon:Gengar,player-1,1|health:88/120",
            "damage|mon:Gengar,player-1,1|health:74/100",
            "split|side:1",
            "damage|mon:Gengar,player-2,1|from:status:Burn|health:35/120",
            "damage|mon:Gengar,player-2,1|from:status:Burn|health:30/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn fire_types_resist_burn() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, gengar().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gengar,player-1,1|name:Will-O-Wisp|noanim",
            "immune|mon:Charizard,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
