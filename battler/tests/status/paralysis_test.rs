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
    get_controlled_rng_for_battle,
    LogMatch,
    TestBattleBuilder,
};

fn pikachu() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Thunder Wave",
                        "Tackle"
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

fn alakazam() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
        "members": [
            {
                "name": "Alakazam",
                "species": "Alakazam",
                "ability": "No Ability",
                "moves": [
                    "Lick"
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
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(48205749111)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn paralysis_reduces_speed_and_prevents_movement() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, pikachu().unwrap(), alakazam().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 0)]);

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Alakazam,player-2,1|name:Lick|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:77/95",
            "damage|mon:Pikachu,player-1,1|health:82/100",
            "move|mon:Pikachu,player-1,1|name:Thunder Wave|target:Alakazam,player-2,1",
            "status|mon:Alakazam,player-2,1|status:Paralysis",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Alakazam,player-2,1",
            "split|side:1",
            "damage|mon:Alakazam,player-2,1|health:96/115",
            "damage|mon:Alakazam,player-2,1|health:84/100",
            "move|mon:Alakazam,player-2,1|name:Lick|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:60/95",
            "damage|mon:Pikachu,player-1,1|health:64/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Alakazam,player-2,1",
            "split|side:1",
            "damage|mon:Alakazam,player-2,1|health:77/115",
            "damage|mon:Alakazam,player-2,1|health:67/100",
            "cant|mon:Alakazam,player-2,1|reason:status:Paralysis",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn electric_types_resist_paralysis() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, pikachu().unwrap(), pikachu().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 0)]);

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Thunder Wave|noanim",
            "immune|mon:Pikachu,player-2,1",
            "move|mon:Pikachu,player-2,1|name:Thunder Wave|noanim",
            "immune|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
