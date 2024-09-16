use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    common::Error,
    dex::{
        DataStore,
        LocalDataStore,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_error_message_contains,
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn make_battle(
    data: &dyn DataStore,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(555432123456)
        .with_team_validation(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn move_can_force_switch_random_target() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Roar"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "No Ability",
                    "moves": ["Tackle"],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                },
                {
                    "name": "Squirtle",
                    "species": "Squirtle",
                    "ability": "No Ability",
                    "moves": ["Tackle"],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let mut battle = make_battle(&data, team.clone(), team).unwrap();

    assert_eq!(battle.start(), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:89/105",
            "damage|mon:Bulbasaur,player-1,1|health:85/100",
            "move|mon:Bulbasaur,player-1,1|name:Roar|target:Bulbasaur,player-2,1",
            "split|side:1",
            "drag|player:player-2|position:1|name:Squirtle|health:104/104|species:Squirtle|level:50|gender:M",
            "drag|player:player-2|position:1|name:Squirtle|health:100/100|species:Squirtle|level:50|gender:M",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Squirtle,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:70/105",
            "damage|mon:Bulbasaur,player-1,1|health:67/100",
            "move|mon:Bulbasaur,player-1,1|name:Roar|target:Squirtle,player-2,1",
            "split|side:1",
            "drag|player:player-2|position:1|name:Bulbasaur|health:105/105|species:Bulbasaur|level:50|gender:M",
            "drag|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:53/105",
            "damage|mon:Bulbasaur,player-1,1|health:51/100",
            "move|mon:Bulbasaur,player-1,1|name:Roar|target:Bulbasaur,player-2,1",
            "split|side:1",
            "drag|player:player-2|position:1|name:Squirtle|health:104/104|species:Squirtle|level:50|gender:M",
            "drag|player:player-2|position:1|name:Squirtle|health:100/100|species:Squirtle|level:50|gender:M",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Squirtle,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:35/105",
            "damage|mon:Bulbasaur,player-1,1|health:34/100",
            "move|mon:Bulbasaur,player-1,1|name:Roar|target:Squirtle,player-2,1",
            "split|side:1",
            "drag|player:player-2|position:1|name:Charmander|health:99/99|species:Charmander|level:50|gender:M",
            "drag|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:50|gender:M",
            "residual",
            "turn|turn:5"
        ]"#).unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn move_can_switch_user() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "U-turn"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "No Ability",
                    "moves": ["Tackle"],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                },
                {
                    "name": "Squirtle",
                    "species": "Squirtle",
                    "ability": "No Ability",
                    "moves": ["Tackle"],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let mut battle = make_battle(&data, team.clone(), team).unwrap();

    assert_eq!(battle.start(), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_error_message_contains(
        battle.set_player_choice("player-2", "switch 2"),
        "you cannot do anything",
    );
    assert_eq!(battle.set_player_choice("player-1", "switch 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,player-1,1|name:U-turn|target:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:77/105",
            "damage|mon:Bulbasaur,player-2,1|health:74/100",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Squirtle"],
            ["switch", "player-1", "Squirtle"],
            "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Squirtle,player-1,1",
            "split|side:0",
            "damage|mon:Squirtle,player-1,1|health:90/104",
            "damage|mon:Squirtle,player-1,1|health:87/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
