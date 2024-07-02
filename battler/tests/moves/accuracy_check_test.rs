#[cfg(test)]
mod accuracy_check_tests {
    use battler::{
        battle::{
            Battle,
            BattleEngineSpeedSortTieResolution,
            BattleType,
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
        assert_new_logs_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn pikachu_team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Pikachu",
                        "species": "Pikachu",
                        "ability": "Lightning Rod",
                        "moves": [
                            "Thunder",
                            "Sand Attack",
                            "Double Team",
                            "Fury Attack",
                            "Triple Kick",
                            "Chip Away"
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

    fn doubles_pikachu_team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Pikachu",
                        "species": "Pikachu",
                        "ability": "Static",
                        "moves": [
                            "Sand Attack",
                            "Double Team",
                            "Icy Wind"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Pikachu",
                        "species": "Pikachu",
                        "ability": "Static",
                        "moves": [
                            "Sand Attack",
                            "Double Team",
                            "Icy Wind"
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

    fn make_singles_battle(
        data: &dyn DataStore,
        seed: u64,
        team: TeamData,
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .with_seed(seed)
            .with_team_validation(false)
            .with_pass_allowed(true)
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team.clone())
            .with_team("player-2", team)
            .build(data)
    }

    fn make_doubles_battle(
        data: &dyn DataStore,
        seed: u64,
        team: TeamData,
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Doubles)
            .with_seed(seed)
            .with_team_validation(false)
            .with_pass_allowed(true)
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team.clone())
            .with_team("player-2", team)
            .build(data)
    }

    #[test]
    fn accuracy_check_applies_normally() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_singles_battle(&data, 143256777503747, pikachu_team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

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
                "move|mon:Pikachu,player-2,1|name:Thunder|target:Pikachu,player-1,1",
                "resisted|mon:Pikachu,player-1,1",
                "split|side:0",
                "damage|mon:Pikachu,player-1,1|health:60/95",
                "damage|mon:Pikachu,player-1,1|health:64/100",
                "move|mon:Pikachu,player-1,1|name:Thunder|target:Pikachu,player-2,1|notarget",
                "miss|mon:Pikachu,player-2,1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Thunder|target:Pikachu,player-1,1",
                "resisted|mon:Pikachu,player-1,1",
                "split|side:0",
                "damage|mon:Pikachu,player-1,1|health:27/95",
                "damage|mon:Pikachu,player-1,1|health:29/100",
                "move|mon:Pikachu,player-1,1|name:Thunder|target:Pikachu,player-2,1",
                "resisted|mon:Pikachu,player-2,1",
                "crit|mon:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:40/95",
                "damage|mon:Pikachu,player-2,1|health:43/100",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn accuracy_check_impacted_by_lowered_accuracy() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_singles_battle(&data, 716958313281881, pikachu_team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

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
                "move|mon:Pikachu,player-2,1|name:Sand Attack|target:Pikachu,player-1,1",
                "unboost|mon:Pikachu,player-1,1|stat:acc|by:1",
                "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1|notarget",
                "miss|mon:Pikachu,player-2,1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Sand Attack|target:Pikachu,player-1,1",
                "unboost|mon:Pikachu,player-1,1|stat:acc|by:1",
                "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
                "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Sand Attack|target:Pikachu,player-1,1",
                "unboost|mon:Pikachu,player-1,1|stat:acc|by:1",
                "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
                "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Thunder|target:Pikachu,player-1,1|notarget",
                "miss|mon:Pikachu,player-1,1",
                "move|mon:Pikachu,player-1,1|name:Thunder|target:Pikachu,player-2,1|notarget",
                "miss|mon:Pikachu,player-2,1",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Thunder|target:Pikachu,player-1,1",
                "resisted|mon:Pikachu,player-1,1",
                "split|side:0",
                "damage|mon:Pikachu,player-1,1|health:61/95",
                "damage|mon:Pikachu,player-1,1|health:65/100",
                "move|mon:Pikachu,player-1,1|name:Thunder|target:Pikachu,player-2,1|notarget",
                "miss|mon:Pikachu,player-2,1",
                "residual",
                "turn|turn:6"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn accuracy_check_impacted_by_raised_evasion() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_singles_battle(&data, 0, pikachu_team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

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
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
                "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Thunder|target:Pikachu,player-1,1",
                "resisted|mon:Pikachu,player-1,1",
                "split|side:0",
                "damage|mon:Pikachu,player-1,1|health:59/95",
                "damage|mon:Pikachu,player-1,1|health:63/100",
                "move|mon:Pikachu,player-1,1|name:Thunder|target:Pikachu,player-2,1|notarget",
                "miss|mon:Pikachu,player-2,1",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Thunder|target:Pikachu,player-1,1|notarget",
                "miss|mon:Pikachu,player-1,1",
                "move|mon:Pikachu,player-1,1|name:Thunder|target:Pikachu,player-2,1|notarget",
                "miss|mon:Pikachu,player-2,1",
                "residual",
                "turn|turn:6"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn accuracy_check_for_each_target() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_doubles_battle(&data, 65564654, doubles_pikachu_team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "move 0,1;move 1"),
            Ok(())
        );

        assert_eq!(battle.set_player_choice("player-1", "move 2;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

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
                "move|mon:Pikachu,player-2,2|name:Double Team|target:Pikachu,player-2,2",
                "boost|mon:Pikachu,player-2,2|stat:eva|by:1",
                "move|mon:Pikachu,player-2,1|name:Sand Attack|target:Pikachu,player-1,1",
                "unboost|mon:Pikachu,player-1,1|stat:acc|by:1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Icy Wind|spread:Pikachu,player-2,2",
                "miss|mon:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,2|health:78/95",
                "damage|mon:Pikachu,player-2,2|health:83/100",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn accuracy_check_only_once_for_multihit_moves() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_singles_battle(&data, 453950743359796, pikachu_team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 3"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 3"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 3"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 3"), Ok(()));
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
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "move|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:86/95",
                "damage|mon:Pikachu,player-2,1|health:91/100",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:77/95",
                "damage|mon:Pikachu,player-2,1|health:82/100",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:68/95",
                "damage|mon:Pikachu,player-2,1|health:72/100",
                "hitcount|hits:3",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "move|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:60/95",
                "damage|mon:Pikachu,player-2,1|health:64/100",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:51/95",
                "damage|mon:Pikachu,player-2,1|health:54/100",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:42/95",
                "damage|mon:Pikachu,player-2,1|health:45/100",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:33/95",
                "damage|mon:Pikachu,player-2,1|health:35/100",
                "hitcount|hits:4",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1|notarget",
                "miss|mon:Pikachu,player-2,1",
                "residual",
                "turn|turn:6",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
                "crit|mon:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:21/95",
                "damage|mon:Pikachu,player-2,1|health:23/100",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:13/95",
                "damage|mon:Pikachu,player-2,1|health:14/100",
                "hitcount|hits:2",
                "residual",
                "turn|turn:7"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn accuracy_check_for_multiaccuracy_moves() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_singles_battle(&data, 21241564315, pikachu_team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 4"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 4"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 4"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 4"), Ok(()));
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
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "move|mon:Pikachu,player-1,1|name:Triple Kick|target:Pikachu,player-2,1|notarget",
                "miss|mon:Pikachu,player-2,1",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Triple Kick|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:89/95",
                "damage|mon:Pikachu,player-2,1|health:94/100",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:78/95",
                "damage|mon:Pikachu,player-2,1|health:83/100",
                "miss|mon:Pikachu,player-2,1",
                "hitcount|hits:2",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Triple Kick|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:72/95",
                "damage|mon:Pikachu,player-2,1|health:76/100",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:61/95",
                "damage|mon:Pikachu,player-2,1|health:65/100",
                "crit|mon:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:35/95",
                "damage|mon:Pikachu,player-2,1|health:37/100",
                "hitcount|hits:3",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Triple Kick|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:29/95",
                "damage|mon:Pikachu,player-2,1|health:31/100",
                "miss|mon:Pikachu,player-2,1",
                "hitcount|hits:1",
                "residual",
                "turn|turn:6"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn moves_can_ignore_evasion() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_singles_battle(&data, 0, pikachu_team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 5"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 5"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 5"), Ok(()));
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
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
                "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
                "residual",
                "turn|turn:6",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Chip Away|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:54/95",
                "damage|mon:Pikachu,player-2,1|health:57/100",
                "residual",
                "turn|turn:7",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Chip Away|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:16/95",
                "damage|mon:Pikachu,player-2,1|health:17/100",
                "residual",
                "turn|turn:8",
                ["time"],
                "move|mon:Pikachu,player-1,1|name:Chip Away|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:0",
                "damage|mon:Pikachu,player-2,1|health:0",
                "faint|mon:Pikachu,player-2,1",
                "win|side:0"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
