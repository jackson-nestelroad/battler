#[cfg(test)]
mod secondary_effect_callback_test {
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

    fn dugtrio() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Dugtrio",
                        "species": "Dugtrio",
                        "ability": "No Ability",
                        "moves": [
                            "Tri Attack"
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
    fn tri_attack_applies_random_status() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(
            &data,
            605915376004016,
            dugtrio().unwrap(),
            dugtrio().unwrap(),
        )
        .unwrap();
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
                "switch|player:player-1|position:1|name:Dugtrio|health:100/100|species:Dugtrio|level:50|gender:M",
                "switch|player:player-2|position:1|name:Dugtrio|health:100/100|species:Dugtrio|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Dugtrio,player-2,1|name:Tri Attack|target:Dugtrio,player-1,1",
                "split|side:0",
                "damage|mon:Dugtrio,player-1,1|health:71/95",
                "damage|mon:Dugtrio,player-1,1|health:75/100",
                "move|mon:Dugtrio,player-1,1|name:Tri Attack|target:Dugtrio,player-2,1",
                "split|side:1",
                "damage|mon:Dugtrio,player-2,1|health:68/95",
                "damage|mon:Dugtrio,player-2,1|health:72/100",
                "status|mon:Dugtrio,player-2,1|status:Freeze",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
