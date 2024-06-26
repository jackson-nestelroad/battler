#[cfg(test)]
mod stat_boost_drop_test {
    use battler::{
        battle::{
            Battle,
            BattleType,
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
        assert_new_logs_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn make_singles_battle(
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
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    fn make_doubles_battle(
        data: &dyn DataStore,
        team_1: TeamData,
        team_2: TeamData,
        seed: u64,
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Doubles)
            .with_seed(seed)
            .with_team_validation(false)
            .with_pass_allowed(true)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn boost_stops_at_max_6() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Pikachu",
                        "species": "Pikachu",
                        "ability": "No Ability",
                        "moves": [
                            "Double Team"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_singles_battle(&data, team.clone(), team, 0).unwrap();

        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "switch|player:player-2|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
                "residual",
                "turn|turn:6",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
                "residual",
                "turn|turn:7",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Double Team|noanim",
                "boost|mon:Pikachu,player-1,1|stat:eva|by:0",
                "fail|mon:Pikachu,player-1,1",
                "residual",
                "turn|turn:8"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn drop_stops_at_max_6() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Pikachu",
                        "species": "Pikachu",
                        "ability": "No Ability",
                        "moves": [
                            "Sand Attack"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_singles_battle(&data, team.clone(), team, 0).unwrap();

        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "switch|player:player-2|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
                "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
                "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
                "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
                "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
                "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
                "residual",
                "turn|turn:6",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
                "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
                "residual",
                "turn|turn:7",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Sand Attack|noanim",
                "unboost|mon:Pikachu,player-2,1|stat:acc|by:0",
                "fail|mon:Pikachu,player-1,1",
                "residual",
                "turn|turn:8"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn boosts_and_drops_cancel_out() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Pikachu",
                        "species": "Pikachu",
                        "ability": "No Ability",
                        "moves": [
                            "Agility",
                            "Cotton Spore"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_singles_battle(&data, team.clone(), team, 0).unwrap();

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
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "switch|player:player-2|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Cotton Spore",
                "unboost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "move|mon:Pikachu,player-2,1|name:Cotton Spore",
                "unboost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Cotton Spore",
                "unboost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "move|mon:Pikachu,player-2,1|name:Cotton Spore",
                "unboost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "move|mon:Pikachu,player-2,1|name:Cotton Spore",
                "unboost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "residual",
                "turn|turn:6",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "move|mon:Pikachu,player-2,1|name:Cotton Spore",
                "unboost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "residual",
                "turn|turn:7"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn multi_stat_boost() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Pikachu",
                        "species": "Pikachu",
                        "ability": "No Ability",
                        "moves": [
                            "Growth"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_singles_battle(&data, team.clone(), team, 0).unwrap();

        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "switch|player:player-2|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Growth|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:atk|by:1",
                "boost|mon:Pikachu,player-1,1|stat:spa|by:1",
                "residual",
                "turn|turn:2"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn raise_all_stats_at_once() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Pikachu",
                        "species": "Pikachu",
                        "ability": "No Ability",
                        "moves": [
                            "Ancient Power"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_singles_battle(&data, team.clone(), team, 777).unwrap();

        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "switch|player:player-2|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Ancient Power|target:Pikachu,player-1,1",
                "split|side:0",
                "damage|mon:Pikachu,player-1,1|health:69/95",
                "damage|mon:Pikachu,player-1,1|health:73/100",
                "move|mon:Pikachu,player-1,1|name:Ancient Power|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:68/95",
                "damage|mon:Pikachu,player-2,1|health:72/100",
                "boost|mon:Pikachu,player-1,1|stat:atk|by:1",
                "boost|mon:Pikachu,player-1,1|stat:def|by:1",
                "boost|mon:Pikachu,player-1,1|stat:spa|by:1",
                "boost|mon:Pikachu,player-1,1|stat:spd|by:1",
                "boost|mon:Pikachu,player-1,1|stat:spe|by:1",
                "residual",
                "turn|turn:2"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn drop_stats_of_all_targets() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Pikachu",
                        "species": "Pikachu",
                        "ability": "No Ability",
                        "moves": [
                            "Tail Whip"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Pikachu",
                        "species": "Pikachu",
                        "ability": "No Ability",
                        "moves": [
                            "Tail Whip"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_doubles_battle(&data, team.clone(), team, 0).unwrap();

        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:2",
                "teamsize|player:player-2|size:2",
                "start",
                "switch|player:player-1|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "switch|player:player-1|position:2|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "switch|player:player-2|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "switch|player:player-2|position:2|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Tail Whip|spread:Pikachu,player-1,1;Pikachu,player-1,2",
                "unboost|mon:Pikachu,player-1,1|stat:def|by:1",
                "unboost|mon:Pikachu,player-1,2|stat:def|by:1",
                "move|mon:Pikachu,player-1,1|name:Tail Whip|spread:Pikachu,player-2,1;Pikachu,player-2,2",
                "unboost|mon:Pikachu,player-2,1|stat:def|by:1",
                "unboost|mon:Pikachu,player-2,2|stat:def|by:1",
                "residual",
                "turn|turn:2"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn modified_speed_impacts_order() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team_1: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Pikachu",
                        "species": "Pikachu",
                        "ability": "No Ability",
                        "moves": [
                            "Agility",
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
        .unwrap();
        let team_2: TeamData = serde_json::from_str(
            r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Agility",
                        "Tackle"
                    ],
                    "nature": "Timid",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }
            ]
        }"#,
        )
        .unwrap();
        let mut battle = make_singles_battle(&data, team_1, team_2, 0).unwrap();

        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "switch|player:player-2|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Tackle|target:Pikachu,player-1,1",
                "split|side:0",
                "damage|mon:Pikachu,player-1,1|health:73/95",
                "damage|mon:Pikachu,player-1,1|health:77/100",
                "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Tackle|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:72/95",
                "damage|mon:Pikachu,player-2,1|health:76/100",
                "move|mon:Pikachu,player-2,1|name:Tackle|target:Pikachu,player-1,1",
                "split|side:0",
                "damage|mon:Pikachu,player-1,1|health:53/95",
                "damage|mon:Pikachu,player-1,1|health:56/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Tackle|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:49/95",
                "damage|mon:Pikachu,player-2,1|health:52/100",
                "move|mon:Pikachu,player-2,1|name:Tackle|target:Pikachu,player-1,1",
                "split|side:0",
                "damage|mon:Pikachu,player-1,1|health:34/95",
                "damage|mon:Pikachu,player-1,1|health:36/100",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Tackle|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:27/95",
                "damage|mon:Pikachu,player-2,1|health:29/100",
                "move|mon:Pikachu,player-2,1|name:Tackle|target:Pikachu,player-1,1",
                "split|side:0",
                "damage|mon:Pikachu,player-1,1|health:14/95",
                "damage|mon:Pikachu,player-1,1|health:15/100",
                "residual",
                "turn|turn:5"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
