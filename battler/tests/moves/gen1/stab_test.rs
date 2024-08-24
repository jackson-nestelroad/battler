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

fn squirtle() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Squirtle",
                    "species": "Squirtle",
                    "ability": "Torrent",
                    "moves": [
                        "Tackle",
                        "Aqua Jet"
                    ],
                    "nature": "Adamant",
                    "gender": "F",
                    "ball": "Normal",
                    "level": 40
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn pikachu() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "Static",
                    "moves": [
                        "Quick Attack"
                    ],
                    "nature": "Bold",
                    "gender": "F",
                    "ball": "Normal",
                    "level": 40
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn test_battle_builder(team_1: TeamData, team_2: TeamData) -> TestBattleBuilder {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(0)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
}

fn make_battle(
    data: &dyn DataStore,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
    test_battle_builder(team_1, team_2).build(data)
}

#[test]
fn stab_increases_damage() {
    // Tackle and Aqua Jet are both Physical moves with the same base damage, so STAB makes the
    // difference.
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, squirtle().unwrap(), pikachu().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Quick Attack|target:Squirtle,player-1,1",
            "split|side:0",
            "damage|mon:Squirtle,player-1,1|health:73/85",
            "damage|mon:Squirtle,player-1,1|health:86/100",
            "move|mon:Squirtle,player-1,1|name:Tackle|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:62/78",
            "damage|mon:Pikachu,player-2,1|health:80/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-2,1|name:Quick Attack|target:Squirtle,player-1,1",
            "split|side:0",
            "damage|mon:Squirtle,player-1,1|health:62/85",
            "damage|mon:Squirtle,player-1,1|health:73/100",
            "move|mon:Squirtle,player-1,1|name:Aqua Jet|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:38/78",
            "damage|mon:Pikachu,player-2,1|health:49/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
