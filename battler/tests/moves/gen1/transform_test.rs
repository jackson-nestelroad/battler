use battler::{
    battle::{
        BattleType,
        CoreBattleEngineRandomizeBaseDamage,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
        Request,
    },
    common::{
        Error,
        WrapResultError,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    mons::Type,
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn ditto() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ditto",
                    "species": "Ditto",
                    "ability": "No Ability",
                    "moves": [
                        "Transform"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn charizard() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Drill Peck",
                        "Growth",
                        "Conversion"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
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
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_controlled_rng(true)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn transform_transforms_into_target() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, ditto().unwrap(), charizard().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    pretty_assertions::assert_eq!(
        battle.request_for_player("player-1"),
        Some(
            serde_json::from_str(
                r#"{
                    "type": "turn",
                    "active": [
                        {
                            "team_position": 0,
                            "moves": [
                                {
                                    "name": "Tackle",
                                    "id": "tackle",
                                    "pp": 5,
                                    "max_pp": 5,
                                    "target": "Normal",
                                    "disabled": false
                                },
                                {
                                    "name": "Drill Peck",
                                    "id": "drillpeck",
                                    "pp": 5,
                                    "max_pp": 5,
                                    "target": "Any",
                                    "disabled": false
                                },
                                {
                                    "name": "Growth",
                                    "id": "growth",
                                    "pp": 5,
                                    "max_pp": 5,
                                    "target": "User",
                                    "disabled": false
                                },
                                {
                                    "name": "Conversion",
                                    "id": "conversion",
                                    "pp": 5,
                                    "max_pp": 5,
                                    "target": "User",
                                    "disabled": false
                                }
                            ]
                        }
                    ],
                    "player": {
                        "name": "Player 1",
                        "id": "player-1",
                        "side": 0,
                        "position": 0,
                        "mons": [
                            {
                                "name": "Ditto",
                                "species_name": "Charizard",
                                "level": 50,
                                "gender": "Male",
                                "shiny": false,
                                "health": "108/108",
                                "types": ["Fire", "Flying"],
                                "status": "",
                                "active": true,
                                "player_active_position": 0,
                                "side_position": 0,
                                "stats": {
                                    "atk": 89,
                                    "def": 83,
                                    "spa": 114,
                                    "spd": 90,
                                    "spe": 105
                                },
                                "moves": [
                                    {
                                        "name": "Tackle",
                                        "id": "tackle",
                                        "pp": 5,
                                        "max_pp": 5,
                                        "target": "Normal",
                                        "disabled": false
                                    },
                                    {
                                        "name": "Drill Peck",
                                        "id": "drillpeck",
                                        "pp": 5,
                                        "max_pp": 5,
                                        "target": "Any",
                                        "disabled": false
                                    },
                                    {
                                        "name": "Growth",
                                        "id": "growth",
                                        "pp": 5,
                                        "max_pp": 5,
                                        "target": "User",
                                        "disabled": false
                                    },
                                    {
                                        "name": "Conversion",
                                        "id": "conversion",
                                        "pp": 5,
                                        "max_pp": 5,
                                        "target": "User",
                                        "disabled": false
                                    }
                                ],
                                "ability": "No Ability",
                                "ball": "Normal"
                            }
                        ]
                    }
                }"#
            )
            .unwrap()
        )
    );

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ditto,player-1,1|name:Transform|target:Charizard,player-2,1",
            "transform|mon:Ditto,player-1,1|into:Charizard,player-2,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Tackle|target:Ditto,player-1,1",
            "split|side:0",
            "damage|mon:Ditto,player-1,1|health:88/108",
            "damage|mon:Ditto,player-1,1|health:82/100",
            "move|mon:Ditto,player-1,1|name:Tackle|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:118/138",
            "damage|mon:Charizard,player-2,1|health:86/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn transform_copies_stat_boosts() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, ditto().unwrap(), charizard().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-2,1|name:Growth|target:Charizard,player-2,1",
            "boost|mon:Charizard,player-2,1|stat:atk|by:1",
            "boost|mon:Charizard,player-2,1|stat:spa|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Ditto,player-1,1|name:Transform|target:Charizard,player-2,1",
            "transform|mon:Ditto,player-1,1|into:Charizard,player-2,1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Drill Peck|target:Ditto,player-1,1",
            "split|side:0",
            "damage|mon:Ditto,player-1,1|health:21/108",
            "damage|mon:Ditto,player-1,1|health:20/100",
            "move|mon:Ditto,player-1,1|name:Drill Peck|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:51/138",
            "damage|mon:Charizard,player-2,1|health:37/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn transform_copies_type_change() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, ditto().unwrap(), charizard().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_eq!(
        battle
            .request_for_player("player-1")
            .map(|request| {
                if let Request::Turn(request) = request {
                    request.player.mons.get(0).map(|mon| mon.types.clone())
                } else {
                    None
                }
            })
            .flatten(),
        Some(vec![Type::Normal])
    );

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-2,1|name:Conversion|target:Charizard,player-2,1",
            "typechange|mon:Charizard,player-2,1|types:Normal",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Ditto,player-1,1|name:Transform|target:Charizard,player-2,1",
            "transform|mon:Ditto,player-1,1|into:Charizard,player-2,1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Tackle|target:Ditto,player-1,1",
            "split|side:0",
            "damage|mon:Ditto,player-1,1|health:78/108",
            "damage|mon:Ditto,player-1,1|health:73/100",
            "move|mon:Ditto,player-1,1|name:Tackle|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:108/138",
            "damage|mon:Charizard,player-2,1|health:79/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
