use battler::{
    battle::{
        BattleType,
        CoreBattleEngineRandomizeBaseDamage,
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
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn sandslash() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
        "members": [
            {
                "name": "Sandslash",
                "species": "Sandslash",
                "ability": "No Ability",
                "moves": [
                    "Magnitude",
                    "Recover"
                ],
                "pp_boosts": [0, 3],
                "nature": "Hardy",
                "gender": "M",
                "level": 50
            }
        ]
    }"#,
    )
    .wrap_error()
}

fn make_battle(data: &dyn DataStore) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(204759285930)
        .with_team_validation(false)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", sandslash()?)
        .with_team("player-2", sandslash()?)
        .build(data)
}

#[test]
fn magnitude_randomly_sets_base_power() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Sandslash,player-1,1|name:Magnitude",
            "activate|move:Magnitude|magnitude:8",
            "split|side:1",
            "damage|mon:Sandslash,player-2,1|health:78/135",
            "damage|mon:Sandslash,player-2,1|health:58/100",
            "move|mon:Sandslash,player-2,1|name:Recover|target:Sandslash,player-2,1",
            "split|side:1",
            "heal|mon:Sandslash,player-2,1|health:135/135",
            "heal|mon:Sandslash,player-2,1|health:100/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Sandslash,player-1,1|name:Magnitude",
            "activate|move:Magnitude|magnitude:7",
            "split|side:1",
            "damage|mon:Sandslash,player-2,1|health:90/135",
            "damage|mon:Sandslash,player-2,1|health:67/100",
            "move|mon:Sandslash,player-2,1|name:Recover|target:Sandslash,player-2,1",
            "split|side:1",
            "heal|mon:Sandslash,player-2,1|health:135/135",
            "heal|mon:Sandslash,player-2,1|health:100/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Sandslash,player-1,1|name:Magnitude",
            "activate|move:Magnitude|magnitude:8",
            "split|side:1",
            "damage|mon:Sandslash,player-2,1|health:78/135",
            "damage|mon:Sandslash,player-2,1|health:58/100",
            "move|mon:Sandslash,player-2,1|name:Recover|target:Sandslash,player-2,1",
            "split|side:1",
            "heal|mon:Sandslash,player-2,1|health:135/135",
            "heal|mon:Sandslash,player-2,1|health:100/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Sandslash,player-1,1|name:Magnitude",
            "activate|move:Magnitude|magnitude:7",
            "split|side:1",
            "damage|mon:Sandslash,player-2,1|health:90/135",
            "damage|mon:Sandslash,player-2,1|health:67/100",
            "move|mon:Sandslash,player-2,1|name:Recover|target:Sandslash,player-2,1",
            "split|side:1",
            "heal|mon:Sandslash,player-2,1|health:135/135",
            "heal|mon:Sandslash,player-2,1|health:100/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Sandslash,player-1,1|name:Magnitude",
            "activate|move:Magnitude|magnitude:6",
            "split|side:1",
            "damage|mon:Sandslash,player-2,1|health:102/135",
            "damage|mon:Sandslash,player-2,1|health:76/100",
            "move|mon:Sandslash,player-2,1|name:Recover|target:Sandslash,player-2,1",
            "split|side:1",
            "heal|mon:Sandslash,player-2,1|health:135/135",
            "heal|mon:Sandslash,player-2,1|health:100/100",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Sandslash,player-1,1|name:Magnitude",
            "activate|move:Magnitude|magnitude:4",
            "split|side:1",
            "damage|mon:Sandslash,player-2,1|health:126/135",
            "damage|mon:Sandslash,player-2,1|health:94/100",
            "move|mon:Sandslash,player-2,1|name:Recover|target:Sandslash,player-2,1",
            "split|side:1",
            "heal|mon:Sandslash,player-2,1|health:135/135",
            "heal|mon:Sandslash,player-2,1|health:100/100",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
