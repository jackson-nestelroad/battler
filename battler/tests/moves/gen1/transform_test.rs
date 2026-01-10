use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    Type,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn ditto() -> Result<TeamData> {
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
                    "ability": "Blaze",
                    "moves": [
                        "Tackle",
                        "Drill Peck",
                        "Growth",
                        "Conversion"
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

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
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
        .build(static_local_data_store())
}

#[test]
fn transform_transforms_into_target() {
    let mut battle = make_battle(0, ditto().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    pretty_assertions::assert_eq!(
        battle.request_for_player("player-1").unwrap(),
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
                ]
            }"#
        )
        .unwrap()
    );

    pretty_assertions::assert_eq!(
        battle.player_data("player-1").unwrap(),
        serde_json::from_str(
            r#"{
                "name": "Player 1",
                "id": "player-1",
                "player_type": {
                    "type": "trainer"
                },
                "side": 0,
                "position": 0,
                "mons": [
                    {
                        "summary": {
                            "name": "Ditto",
                            "species": "Ditto",
                            "level": 50,
                            "gender": "M",
                            "nature": "Hardy",
                            "shiny": false,
                            "hp": 108,
                            "friendship": 0,
                            "experience": 125000,
                            "stats": {
                                "hp": 108,
                                "atk": 53,
                                "def": 53,
                                "spa": 53,
                                "spd": 53,
                                "spe": 53
                            },
                            "evs": {
                                "hp": 0,
                                "atk": 0,
                                "def": 0,
                                "spa": 0,
                                "spd": 0,
                                "spe": 0
                            },
                            "ivs": {
                                "hp": 0,
                                "atk": 0,
                                "def": 0,
                                "spa": 0,
                                "spd": 0,
                                "spe": 0
                            },
                            "moves": [
                                {
                                    "name": "Transform",
                                    "pp": 9
                                }
                            ],
                            "ability": "No Ability",
                            "item": null,
                            "status": null,
                            "hidden_power_type": "Fighting"
                        },
                        "species": "Charizard",
                        "hp": 108,
                        "max_hp": 108,
                        "health": "108/108",
                        "types": [
                            "Fire",
                            "Flying"
                        ],
                        "active": true,
                        "player_team_position": 0,
                        "player_effective_team_position": 0,
                        "player_active_position": 0,
                        "side_position": 0,
                        "stats": {
                            "atk": 89,
                            "def": 83,
                            "spa": 114,
                            "spd": 90,
                            "spe": 105
                        },
                        "boosts": {
                            "atk": 0,
                            "def": 0,
                            "spa": 0,
                            "spd": 0,
                            "spe": 0,
                            "acc": 0,
                            "eva": 0
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
                        "ability": "Blaze",
                        "item": null,
                        "status": null
                    }
                ]
            }"#
        )
        .unwrap()
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ditto,player-1,1|name:Transform|target:Charizard,player-2,1",
            "transform|mon:Ditto,player-1,1|into:Charizard,player-2,1|species:Charizard",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Ditto,player-1,1|name:Tackle|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:118/138",
            "damage|mon:Charizard,player-2,1|health:86/100",
            "move|mon:Charizard,player-2,1|name:Tackle|target:Ditto,player-1,1",
            "split|side:0",
            "damage|mon:Ditto,player-1,1|health:88/108",
            "damage|mon:Ditto,player-1,1|health:82/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn transform_copies_stat_boosts() {
    let mut battle = make_battle(0, ditto().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-2,1|name:Growth|target:Charizard,player-2,1",
            "boost|mon:Charizard,player-2,1|stat:atk|by:1",
            "boost|mon:Charizard,player-2,1|stat:spa|by:1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Ditto,player-1,1|name:Transform|target:Charizard,player-2,1",
            "transform|mon:Ditto,player-1,1|into:Charizard,player-2,1|species:Charizard",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Ditto,player-1,1|name:Drill Peck|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:51/138",
            "damage|mon:Charizard,player-2,1|health:37/100",
            "move|mon:Charizard,player-2,1|name:Drill Peck|target:Ditto,player-1,1",
            "split|side:0",
            "damage|mon:Ditto,player-1,1|health:21/108",
            "damage|mon:Ditto,player-1,1|health:20/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn transform_copies_type_change() {
    let mut battle = make_battle(0, ditto().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_eq!(
        battle
            .player_data("player-1")
            .map(|data| { data.mons.get(0).map(|mon| mon.types.clone()) })
            .unwrap(),
        Some(vec![Type::Normal])
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-2,1|name:Conversion|target:Charizard,player-2,1",
            "typechange|mon:Charizard,player-2,1|types:Normal",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Ditto,player-1,1|name:Transform|target:Charizard,player-2,1",
            "transform|mon:Ditto,player-1,1|into:Charizard,player-2,1|species:Charizard",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Ditto,player-1,1|name:Tackle|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:108/138",
            "damage|mon:Charizard,player-2,1|health:79/100",
            "move|mon:Charizard,player-2,1|name:Tackle|target:Ditto,player-1,1",
            "split|side:0",
            "damage|mon:Ditto,player-1,1|health:78/108",
            "damage|mon:Ditto,player-1,1|health:73/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
