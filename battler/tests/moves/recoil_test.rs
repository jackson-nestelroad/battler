#[cfg(test)]
mod recoil_test {
    use battler::{
        battle::{
            Battle,
            BattleEngineSpeedSortTieResolution,
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
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn recoils_based_on_damage_dealt() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Slaking",
                        "species": "Slaking",
                        "ability": "No Ability",
                        "moves": ["Double-Edge"],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 100
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_battle(&data, 0, team.clone(), team).unwrap();

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
                "switch|player:player-1|position:1|name:Slaking|health:100/100|species:Slaking|level:100|gender:M",
                "switch|player:player-2|position:1|name:Slaking|health:100/100|species:Slaking|level:100|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Slaking,player-2,1|name:Double-Edge|target:Slaking,player-1,1",
                "split|side:0",
                "damage|mon:Slaking,player-1,1|health:176/410",
                "damage|mon:Slaking,player-1,1|health:43/100",
                "split|side:1",
                "damage|mon:Slaking,player-2,1|from:Recoil|health:332/410",
                "damage|mon:Slaking,player-2,1|from:Recoil|health:81/100",
                "move|mon:Slaking,player-1,1|name:Double-Edge|target:Slaking,player-2,1",
                "split|side:1",
                "damage|mon:Slaking,player-2,1|health:116/410",
                "damage|mon:Slaking,player-2,1|health:29/100",
                "split|side:0",
                "damage|mon:Slaking,player-1,1|from:Recoil|health:104/410",
                "damage|mon:Slaking,player-1,1|from:Recoil|health:26/100",
                "residual",
                "turn|turn:2"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn recoils_based_on_user_hp() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Slaking",
                        "species": "Slaking",
                        "ability": "No Ability",
                        "moves": ["Chloroblast"],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 100
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_battle(&data, 766108902979015, team.clone(), team).unwrap();

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
                "switch|player:player-1|position:1|name:Slaking|health:100/100|species:Slaking|level:100|gender:M",
                "switch|player:player-2|position:1|name:Slaking|health:100/100|species:Slaking|level:100|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Slaking,player-2,1|name:Chloroblast|target:Slaking,player-1,1",
                "split|side:0",
                "damage|mon:Slaking,player-1,1|health:250/410",
                "damage|mon:Slaking,player-1,1|health:61/100",
                "split|side:1",
                "damage|mon:Slaking,player-2,1|from:Recoil|health:205/410",
                "damage|mon:Slaking,player-2,1|from:Recoil|health:50/100",
                "move|mon:Slaking,player-1,1|name:Chloroblast|target:Slaking,player-2,1",
                "split|side:1",
                "damage|mon:Slaking,player-2,1|health:27/410",
                "damage|mon:Slaking,player-2,1|health:7/100",
                "split|side:0",
                "damage|mon:Slaking,player-1,1|from:Recoil|health:45/410",
                "damage|mon:Slaking,player-1,1|from:Recoil|health:11/100",
                "residual",
                "turn|turn:2"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
