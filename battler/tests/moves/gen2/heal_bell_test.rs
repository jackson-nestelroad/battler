#[cfg(test)]
mod heal_bell_test {
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
                        "name": "Miltank",
                        "species": "Miltank",
                        "ability": "Soundproof",
                        "moves": [
                            "Heal Bell",
                            "Thunder Wave",
                            "Sleep Powder",
                            "Toxic"
                        ],
                        "nature": "Hardy",
                        "level": 50
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
                        "level": 50
                    },
                    {
                        "name": "Furret",
                        "species": "Furret",
                        "ability": "Soundproof",
                        "moves": [],
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
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Reverse)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn heal_bell_cures_all_statuses_on_side() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "switch 3"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 3"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 3"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "switch|player:player-1|position:1|name:Meganium|health:100/100|species:Meganium|level:50|gender:F",
                "move|mon:Miltank,player-2,1|name:Thunder Wave|target:Meganium,player-1,1",
                "status|mon:Meganium,player-1,1|status:Paralysis",
                "residual",
                "turn|turn:2",
                ["time"],
                "switch|player:player-1|position:1|name:Typhlosion|health:100/100|species:Typhlosion|level:50|gender:F",
                "move|mon:Miltank,player-2,1|name:Sleep Powder|target:Typhlosion,player-1,1",
                "status|mon:Typhlosion,player-1,1|status:Sleep|from:move:Sleep Powder",
                "residual",
                "turn|turn:3",
                ["time"],
                "switch|player:player-1|position:1|name:Feraligatr|health:100/100|species:Feraligatr|level:50|gender:F",
                "move|mon:Miltank,player-2,1|name:Toxic|target:Feraligatr,player-1,1",
                "status|mon:Feraligatr,player-1,1|status:Bad Poison",
                "split|side:0",
                "damage|mon:Feraligatr,player-1,1|from:status:Bad Poison|health:136/145",
                "damage|mon:Feraligatr,player-1,1|from:status:Bad Poison|health:94/100",
                "residual",
                "turn|turn:4",
                ["time"],
                "switch|player:player-1|position:1|name:Miltank|health:100/100|species:Miltank|level:50|gender:F",
                "move|mon:Miltank,player-2,1|name:Toxic|target:Miltank,player-1,1",
                "status|mon:Miltank,player-1,1|status:Bad Poison",
                "split|side:0",
                "damage|mon:Miltank,player-1,1|from:status:Bad Poison|health:146/155",
                "damage|mon:Miltank,player-1,1|from:status:Bad Poison|health:95/100",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Miltank,player-1,1|name:Heal Bell",
                "activate|move:Heal Bell|of:Miltank,player-1,1",
                "curestatus|mon:Miltank,player-1,1|status:Bad Poison",
                "curestatus|mon:Meganium,player-1,1|status:Paralysis",
                "curestatus|mon:Typhlosion,player-1,1|status:Sleep",
                "curestatus|mon:Feraligatr,player-1,1|status:Bad Poison",
                "residual",
                "turn|turn:6"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }

    #[test]
    fn soundproof_ignores_heal_bell() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "switch 4"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "switch|player:player-1|position:1|name:Furret|health:100/100|species:Furret|level:50|gender:F",
                "move|mon:Miltank,player-2,1|name:Thunder Wave|target:Furret,player-1,1",
                "status|mon:Furret,player-1,1|status:Paralysis",
                "residual",
                "turn|turn:2",
                ["time"],
                "switch|player:player-1|position:1|name:Miltank|health:100/100|species:Miltank|level:50|gender:F",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Miltank,player-1,1|name:Heal Bell|noanim",
                "activate|move:Heal Bell|of:Miltank,player-1,1",
                "fail|mon:Miltank,player-1,1",
                "residual",
                "turn|turn:4"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }
}
