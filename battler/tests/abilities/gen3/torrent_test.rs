use battler::{
    battle::{
        BattleType,
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

fn swampert() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Swampert",
                    "species": "Swampert",
                    "ability": "Torrent",
                    "moves": [
                        "Water Gun"
                    ],
                    "nature": "Hardy",
                    "level": 75
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn sceptile() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Sceptile",
                    "species": "Sceptile",
                    "ability": "Overgrow",
                    "moves": [
                        "Leaf Blade"
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
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn torrent_boosts_water_attack_with_one_third_hp() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, swampert().unwrap(), sceptile().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swampert,player-1,1|name:Water Gun|target:Sceptile,player-2,1",
            "resisted|mon:Sceptile,player-2,1",
            "split|side:1",
            "damage|mon:Sceptile,player-2,1|health:103/130",
            "damage|mon:Sceptile,player-2,1|health:80/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Sceptile,player-2,1|name:Leaf Blade|target:Swampert,player-1,1",
            "supereffective|mon:Swampert,player-1,1",
            "crit|mon:Swampert,player-1,1",
            "split|side:0",
            "damage|mon:Swampert,player-1,1|health:19/235",
            "damage|mon:Swampert,player-1,1|health:9/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Swampert,player-1,1|name:Water Gun|target:Sceptile,player-2,1",
            "resisted|mon:Sceptile,player-2,1",
            "split|side:1",
            "damage|mon:Sceptile,player-2,1|health:67/130",
            "damage|mon:Sceptile,player-2,1|health:52/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
