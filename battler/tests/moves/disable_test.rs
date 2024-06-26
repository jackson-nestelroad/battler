#[cfg(test)]
mod disable_test {
    use battler::{
        battle::{
            Battle,
            BattleType,
            PublicCoreBattle,
            Request,
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
        assert_error_message,
        assert_new_logs_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Alakazam",
                        "species": "Alakazam",
                        "ability": "No Ability",
                        "moves": [
                            "Disable",
                            "Tackle",
                            "Psychic"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Aerodactyl",
                        "species": "Aerodactyl",
                        "ability": "No Ability",
                        "moves": [
                            "Disable",
                            "Tackle",
                            "Razor Wind"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Slowbro",
                        "species": "Slowbro",
                        "ability": "No Ability",
                        "moves": [
                            "Thrash"
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
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn disable_disables_last_used_move() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_error_message(
            battle.set_player_choice("player-2", "move 1"),
            "cannot move: Aerodactyl's Tackle is disabled",
        );
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
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
                "teamsize|player:player-1|size:3",
                "teamsize|player:player-2|size:3",
                "start",
                "switch|player:player-1|position:1|name:Alakazam|health:100/100|species:Alakazam|level:50|gender:M",
                "switch|player:player-2|position:1|name:Alakazam|health:100/100|species:Alakazam|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "switch|player:player-2|position:1|name:Aerodactyl|health:100/100|species:Aerodactyl|level:50|gender:M",
                "move|mon:Alakazam,player-1,1|name:Disable|noanim",
                "fail|mon:Alakazam,player-1,1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Aerodactyl,player-2,1|name:Tackle|target:Alakazam,player-1,1",
                "split|side:0",
                "damage|mon:Alakazam,player-1,1|health:79/115",
                "damage|mon:Alakazam,player-1,1|health:69/100",
                "move|mon:Alakazam,player-1,1|name:Disable|target:Aerodactyl,player-2,1",
                "start|mon:Aerodactyl,player-2,1|move:Disable|disabledmove:Tackle",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Aerodactyl,player-2,1|name:Razor Wind|noanim",
                "prepare|mon:Aerodactyl,player-2,1|move:Razor Wind",
                "move|mon:Alakazam,player-1,1|name:Disable|noanim",
                "fail|mon:Alakazam,player-1,1",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Aerodactyl,player-2,1|name:Razor Wind",
                "split|side:0",
                "damage|mon:Alakazam,player-1,1|health:57/115",
                "damage|mon:Alakazam,player-1,1|health:50/100",
                "residual",
                "turn|turn:5",
                ["time"],
                "residual",
                "turn|turn:6",
                ["time"],
                "end|mon:Aerodactyl,player-2,1|move:Disable",
                "residual",
                "turn|turn:7",
                ["time"],
                "move|mon:Aerodactyl,player-2,1|name:Tackle|target:Alakazam,player-1,1",
                "split|side:0",
                "damage|mon:Alakazam,player-1,1|health:18/115",
                "damage|mon:Alakazam,player-1,1|health:16/100",
                "residual",
                "turn|turn:8",
                ["time"],
                "move|mon:Alakazam,player-1,1|name:Disable|target:Aerodactyl,player-2,1",
                "start|mon:Aerodactyl,player-2,1|move:Disable|disabledmove:Disable",
                "residual",
                "turn|turn:9"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn disable_ends_locked_move_and_forces_struggle() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_battle(&data, 1060328782717467, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

        assert!(battle
            .request_for_player("player-2")
            .is_some_and(|request| match request {
                Request::Turn(request) => request.active.first().is_some_and(|mon| mon.moves.len()
                    == 1
                    && mon.moves.first().is_some_and(
                        |move_slot| move_slot.name == "Struggle" && move_slot.id.eq("struggle")
                    )),
                _ => false,
            }));

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
                "teamsize|player:player-1|size:3",
                "teamsize|player:player-2|size:3",
                "start",
                "switch|player:player-1|position:1|name:Alakazam|health:100/100|species:Alakazam|level:50|gender:M",
                "switch|player:player-2|position:1|name:Alakazam|health:100/100|species:Alakazam|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "switch|player:player-2|position:1|name:Slowbro|health:100/100|species:Slowbro|level:50|gender:M",
                "move|mon:Alakazam,player-1,1|name:Disable|noanim",
                "fail|mon:Alakazam,player-1,1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Alakazam,player-1,1|name:Disable|noanim",
                "fail|mon:Alakazam,player-1,1",
                "move|mon:Slowbro,player-2,1|name:Thrash|target:Alakazam,player-1,1",
                "split|side:0",
                "damage|mon:Alakazam,player-1,1|health:40/115",
                "damage|mon:Alakazam,player-1,1|health:35/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Alakazam,player-1,1|name:Disable|target:Slowbro,player-2,1",
                "start|mon:Slowbro,player-2,1|move:Disable|disabledmove:Thrash",
                "cant|mon:Slowbro,player-2,1|reason:Disable",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Alakazam,player-1,1|name:Disable|noanim",
                "fail|mon:Alakazam,player-1,1",
                "cant|mon:Slowbro,player-2,1|reason:Disable",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Slowbro,player-2,1|name:Struggle|target:Alakazam,player-1,1",
                "split|side:0",
                "damage|mon:Alakazam,player-1,1|health:6/115",
                "damage|mon:Alakazam,player-1,1|health:6/100",
                "split|side:1",
                "damage|mon:Slowbro,player-2,1|from:Struggle Recoil|health:116/155",
                "damage|mon:Slowbro,player-2,1|from:Struggle Recoil|health:75/100",
                "residual",
                "turn|turn:6"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
