use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    error::Error,
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

fn make_battle(
    data: &dyn DataStore,
    team_1: TeamData,
    team_2: TeamData,
    seed: u64,
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
fn quad_super_effective() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team_1: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Thunderbolt"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let team_2: TeamData = serde_json::from_str(
        r#"{
        "members": [
            {
                "name": "Gyarados",
                "species": "Gyarados",
                "ability": "No Ability",
                "moves": [],
                "nature": "Hardy",
                "gender": "M",
                "level": 50
            }
        ]
    }"#,
    )
    .unwrap();
    let mut battle = make_battle(&data, team_1, team_2, 0).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Thunderbolt|target:Gyarados,player-2,1",
            "supereffective|mon:Gyarados,player-2,1",
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|health:31/155",
            "damage|mon:Gyarados,player-2,1|health:20/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn quad_resisted() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team_1: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ludicolo",
                    "species": "Ludicolo",
                    "ability": "No Ability",
                    "moves": [
                        "Surf"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let team_2: TeamData = serde_json::from_str(
        r#"{
        "members": [
            {
                "name": "Ludicolo",
                "species": "Ludicolo",
                "ability": "No Ability",
                "moves": [],
                "nature": "Hardy",
                "gender": "M",
                "level": 50
            }
        ]
    }"#,
    )
    .unwrap();
    let mut battle = make_battle(&data, team_1, team_2, 0).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ludicolo,player-1,1|name:Surf",
            "resisted|mon:Ludicolo,player-2,1",
            "split|side:1",
            "damage|mon:Ludicolo,player-2,1|health:127/140",
            "damage|mon:Ludicolo,player-2,1|health:91/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn type_immune() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team_1: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pidgeot",
                    "species": "Pidgeot",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let team_2: TeamData = serde_json::from_str(
        r#"{
        "members": [
            {
                "name": "Gengar",
                "species": "Gengar",
                "ability": "No Ability",
                "moves": [],
                "nature": "Hardy",
                "gender": "M",
                "level": 50
            }
        ]
    }"#,
    )
    .unwrap();
    let mut battle = make_battle(&data, team_1, team_2, 0).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pidgeot,player-1,1|name:Tackle|noanim",
            "immune|mon:Gengar,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
