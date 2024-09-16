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

fn team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Sneasel",
                    "species": "Sneasel",
                    "ability": "No Ability",
                    "moves": [
                        "Beat Up",
                        "Toxic",
                        "Ice Beam"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Meganium",
                    "species": "Meganium",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Typhlosion",
                    "species": "Typhlosion",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Feraligatr",
                    "species": "Feraligatr",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 75
                },
                {
                    "name": "Lugia",
                    "species": "Lugia",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
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
fn beat_up_attacks_for_each_mon_in_party() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "switch 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Feraligatr"],
            ["switch", "player-2", "Feraligatr"],
            "move|mon:Sneasel,player-1,1|name:Beat Up|target:Feraligatr,player-2,1",
            "activate|move:Beat Up|hit:Sneasel",
            "split|side:1",
            "damage|mon:Feraligatr,player-2,1|health:175/212",
            "damage|mon:Feraligatr,player-2,1|health:83/100",
            "animatemove|mon:Sneasel,player-1,1|name:Beat Up|target:Feraligatr,player-2,1",
            "activate|move:Beat Up|hit:Feraligatr",
            "split|side:1",
            "damage|mon:Feraligatr,player-2,1|health:141/212",
            "damage|mon:Feraligatr,player-2,1|health:67/100",
            "animatemove|mon:Sneasel,player-1,1|name:Beat Up|target:Feraligatr,player-2,1",
            "activate|move:Beat Up|hit:Lugia",
            "split|side:1",
            "damage|mon:Feraligatr,player-2,1|health:107/212",
            "damage|mon:Feraligatr,player-2,1|health:51/100",
            "hitcount|hits:3",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 4, &expected_logs);
}
