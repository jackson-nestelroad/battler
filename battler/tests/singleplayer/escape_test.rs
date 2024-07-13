#[cfg(test)]
mod escape_test {
    use battler::{
        battle::{
            BattleType,
            CoreBattleEngineSpeedSortTieResolution,
            PublicCoreBattle,
            WildPlayerOptions,
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

    fn jolteon() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Jolteon",
                        "species": "Jolteon",
                        "ability": "No Ability",
                        "moves": [
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

    fn primeape() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Primeape",
                        "species": "Primeape",
                        "ability": "No Ability",
                        "moves": [
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

    fn make_wild_singles_battle(
        data: &dyn DataStore,
        seed: u64,
        team_1: TeamData,
        team_2: TeamData,
        wild_options: WildPlayerOptions,
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .with_seed(seed)
            .with_team_validation(false)
            .with_pass_allowed(true)
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
            .add_protagonist_to_side_1("protagonist", "Protagonist")
            .add_wild_mon_to_side_2("wild", "Wild", wild_options)
            .with_team("protagonist", team_1)
            .with_team("wild", team_2)
            .build(data)
    }

    #[test]
    fn player_escapes_with_higher_speed() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_wild_singles_battle(
            &data,
            0,
            jolteon().unwrap(),
            primeape().unwrap(),
            WildPlayerOptions::default(),
        )
        .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("protagonist", "escape"), Ok(()));
        assert_eq!(battle.set_player_choice("wild", "move 0"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:protagonist|name:Protagonist|side:0|position:0",
                "player|id:wild|name:Wild|side:1|position:0",
                ["time"],
                "teamsize|player:protagonist|size:1",
                "start",
                "appear|player:wild|position:1|name:Primeape|health:100/100|species:Primeape|level:50|gender:M",
                "switch|player:protagonist|position:1|name:Jolteon|health:100/100|species:Jolteon|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "escaped|player:protagonist",
                "win|side:1"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn wild_player_can_escape() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_wild_singles_battle(
            &data,
            0,
            jolteon().unwrap(),
            primeape().unwrap(),
            WildPlayerOptions::default(),
        )
        .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("protagonist", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("wild", "escape"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:protagonist|name:Protagonist|side:0|position:0",
                "player|id:wild|name:Wild|side:1|position:0",
                ["time"],
                "teamsize|player:protagonist|size:1",
                "start",
                "appear|player:wild|position:1|name:Primeape|health:100/100|species:Primeape|level:50|gender:M",
                "switch|player:protagonist|position:1|name:Jolteon|health:100/100|species:Jolteon|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "escaped|player:wild",
                "win|side:0"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
