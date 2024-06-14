#[cfg(test)]
mod freeze_test {
    use battler::{
        battle::{
            Battle,
            BattleEngineRandomizeBaseDamage,
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

    fn cloyster() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
            "members": [
                {
                    "name": "Cloyster",
                    "species": "Cloyster",
                    "ability": "No Ability",
                    "moves": [
                        "Ice Beam",
                        "Scald",
                        "Ember"
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

    fn mewtwo() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
            "members": [
                {
                    "name": "Mewtwo",
                    "species": "Mewtwo",
                    "ability": "No Ability",
                    "moves": [
                        "Psychic",
                        "Flame Wheel"
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
        seed: u64,
        team_1: TeamData,
        team_2: TeamData,
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .with_team_validation(false)
            .with_pass_allowed(true)
            .with_base_damage_randomization(BattleEngineRandomizeBaseDamage::Max)
            .with_seed(seed)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn freeze_prevents_movement_until_unfrozen() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 200, cloyster().unwrap(), mewtwo().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Cloyster|health:100/100|species:Cloyster|level:50|gender:M",
                "switch|player:player-2|position:1|name:Mewtwo|health:100/100|species:Mewtwo|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Cloyster,player-1,1|name:Ice Beam|target:Mewtwo,player-2,1",
                "split|side:1",
                "damage|mon:Mewtwo,player-2,1|health:108/166",
                "damage|mon:Mewtwo,player-2,1|health:66/100",
                "status|mon:Mewtwo,player-2,1|status:Freeze",
                "residual",
                "turn|turn:2",
                ["time"],
                "cant|mon:Mewtwo,player-2,1|reason:Freeze",
                "residual",
                "turn|turn:3",
                ["time"],
                "cant|mon:Mewtwo,player-2,1|reason:Freeze",
                "residual",
                "turn|turn:4",
                ["time"],
                "cant|mon:Mewtwo,player-2,1|reason:Freeze",
                "residual",
                "turn|turn:5",
                ["time"],
                "cant|mon:Mewtwo,player-2,1|reason:Freeze",
                "residual",
                "turn|turn:6",
                ["time"],
                "cant|mon:Mewtwo,player-2,1|reason:Freeze",
                "residual",
                "turn|turn:7",
                ["time"],
                "cant|mon:Mewtwo,player-2,1|reason:Freeze",
                "residual",
                "turn|turn:8",
                ["time"],
                "cant|mon:Mewtwo,player-2,1|reason:Freeze",
                "residual",
                "turn|turn:9",
                ["time"],
                "cant|mon:Mewtwo,player-2,1|reason:Freeze",
                "residual",
                "turn|turn:10",
                ["time"],
                "cant|mon:Mewtwo,player-2,1|reason:Freeze",
                "residual",
                "turn|turn:11",
                ["time"],
                "cant|mon:Mewtwo,player-2,1|reason:Freeze",
                "residual",
                "turn|turn:12",
                ["time"],
                "curestatus|mon:Mewtwo,player-2,1|status:Freeze",
                "move|mon:Mewtwo,player-2,1|name:Psychic|target:Cloyster,player-1,1",
                "split|side:0",
                "damage|mon:Cloyster,player-1,1|health:0",
                "damage|mon:Cloyster,player-1,1|health:0",
                "faint|mon:Cloyster,player-1,1",
                "win|side:1"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn freeze_prevents_movement_until_unfrozen_with_less_severe_seed() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_battle(&data, 11943504949, cloyster().unwrap(), mewtwo().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Cloyster|health:100/100|species:Cloyster|level:50|gender:M",
                "switch|player:player-2|position:1|name:Mewtwo|health:100/100|species:Mewtwo|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Cloyster,player-1,1|name:Ice Beam|target:Mewtwo,player-2,1",
                "split|side:1",
                "damage|mon:Mewtwo,player-2,1|health:108/166",
                "damage|mon:Mewtwo,player-2,1|health:66/100",
                "status|mon:Mewtwo,player-2,1|status:Freeze",
                "residual",
                "turn|turn:2",
                ["time"],
                "curestatus|mon:Mewtwo,player-2,1|status:Freeze",
                "move|mon:Mewtwo,player-2,1|name:Psychic|target:Cloyster,player-1,1",
                "split|side:0",
                "damage|mon:Cloyster,player-1,1|health:0",
                "damage|mon:Cloyster,player-1,1|health:0",
                "faint|mon:Cloyster,player-1,1",
                "win|side:1"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn moves_can_thaw_user() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 200, cloyster().unwrap(), mewtwo().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Cloyster|health:100/100|species:Cloyster|level:50|gender:M",
                "switch|player:player-2|position:1|name:Mewtwo|health:100/100|species:Mewtwo|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Cloyster,player-1,1|name:Ice Beam|target:Mewtwo,player-2,1",
                "split|side:1",
                "damage|mon:Mewtwo,player-2,1|health:108/166",
                "damage|mon:Mewtwo,player-2,1|health:66/100",
                "status|mon:Mewtwo,player-2,1|status:Freeze",
                "residual",
                "turn|turn:2",
                ["time"],
                "curestatus|mon:Mewtwo,player-2,1|status:Freeze|from:Flame Wheel",
                "move|mon:Mewtwo,player-2,1|name:Flame Wheel|target:Cloyster,player-1,1",
                "split|side:0",
                "damage|mon:Cloyster,player-1,1|health:92/110",
                "damage|mon:Cloyster,player-1,1|health:84/100",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn moves_can_thaw_target() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 200, cloyster().unwrap(), mewtwo().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
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
                "switch|player:player-1|position:1|name:Cloyster|health:100/100|species:Cloyster|level:50|gender:M",
                "switch|player:player-2|position:1|name:Mewtwo|health:100/100|species:Mewtwo|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Cloyster,player-1,1|name:Ice Beam|target:Mewtwo,player-2,1",
                "split|side:1",
                "damage|mon:Mewtwo,player-2,1|health:108/166",
                "damage|mon:Mewtwo,player-2,1|health:66/100",
                "status|mon:Mewtwo,player-2,1|status:Freeze",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Cloyster,player-1,1|name:Scald|target:Mewtwo,player-2,1",
                "split|side:1",
                "damage|mon:Mewtwo,player-2,1|health:56/166",
                "damage|mon:Mewtwo,player-2,1|health:34/100",
                "curestatus|mon:Mewtwo,player-2,1|status:Freeze",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn fire_type_moves_thaw_target() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 200, cloyster().unwrap(), mewtwo().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
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
                "switch|player:player-1|position:1|name:Cloyster|health:100/100|species:Cloyster|level:50|gender:M",
                "switch|player:player-2|position:1|name:Mewtwo|health:100/100|species:Mewtwo|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Cloyster,player-1,1|name:Ice Beam|target:Mewtwo,player-2,1",
                "split|side:1",
                "damage|mon:Mewtwo,player-2,1|health:108/166",
                "damage|mon:Mewtwo,player-2,1|health:66/100",
                "status|mon:Mewtwo,player-2,1|status:Freeze",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Cloyster,player-1,1|name:Ember|target:Mewtwo,player-2,1",
                "split|side:1",
                "damage|mon:Mewtwo,player-2,1|health:90/166",
                "damage|mon:Mewtwo,player-2,1|health:55/100",
                "curestatus|mon:Mewtwo,player-2,1|status:Freeze",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn ice_types_resist_freeze() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 200, cloyster().unwrap(), cloyster().unwrap()).unwrap();
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
                "switch|player:player-1|position:1|name:Cloyster|health:100/100|species:Cloyster|level:50|gender:M",
                "switch|player:player-2|position:1|name:Cloyster|health:100/100|species:Cloyster|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Cloyster,player-1,1|name:Ice Beam|target:Cloyster,player-2,1",
                "resisted|mon:Cloyster,player-2,1",
                "split|side:1",
                "damage|mon:Cloyster,player-2,1|health:83/110",
                "damage|mon:Cloyster,player-2,1|health:76/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
