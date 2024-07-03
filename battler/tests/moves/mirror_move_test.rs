#[cfg(test)]
mod mirror_move_test {
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

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Pidgeot",
                        "species": "Pidgeot",
                        "ability": "No Ability",
                        "moves": [
                            "Mirror Move",
                            "Agility",
                            "Quick Attack",
                            "Razor Wind",
                            "Fly"
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
                            "Thunder Shock"
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
        battle_type: BattleType,
        seed: u64,
        team_1: TeamData,
        team_2: TeamData,
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(battle_type)
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
    fn mirror_move_copies_targets_last_move() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(
            &data,
            BattleType::Singles,
            0,
            team().unwrap(),
            team().unwrap(),
        )
        .unwrap();
        assert_eq!(battle.start(), Ok(()));

        // Fails with no last move.
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        // Fails to copy itself.
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

        // Copy fails.
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

        // Copy succeeds.
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        // Copy of the copy fails.
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

        // Two-turn move, last move is copied (not this one).
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 3"), Ok(()));

        // Two-turn move finishes and is copied.
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

        // Last move was Mirror Move, so the copy fails. Copied two-turn move finishes.
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

        // The copied two-turn move cannot be copied (there is actually a big difference in the
        // battle engine).
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
                "teamsize|player:player-1|size:2",
                "teamsize|player:player-2|size:2",
                "start",
                "switch|player:player-1|position:1|name:Pidgeot|health:100/100|species:Pidgeot|level:50|gender:M",
                "switch|player:player-2|position:1|name:Pidgeot|health:100/100|species:Pidgeot|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Pidgeot,player-1,1|name:Mirror Move|noanim",
                "fail|mon:Pidgeot,player-1,1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pidgeot,player-2,1|name:Mirror Move|noanim",
                "fail|mon:Pidgeot,player-2,1",
                "move|mon:Pidgeot,player-1,1|name:Mirror Move|noanim",
                "fail|mon:Pidgeot,player-1,1",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Pidgeot,player-2,1|name:Agility|target:Pidgeot,player-2,1",
                "boost|mon:Pidgeot,player-2,1|stat:spe|by:2",
                "move|mon:Pidgeot,player-1,1|name:Mirror Move|noanim",
                "fail|mon:Pidgeot,player-1,1",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Pidgeot,player-2,1|name:Quick Attack|target:Pidgeot,player-1,1",
                "split|side:0",
                "damage|mon:Pidgeot,player-1,1|health:115/143",
                "damage|mon:Pidgeot,player-1,1|health:81/100",
                "move|mon:Pidgeot,player-1,1|name:Mirror Move|target:Pidgeot,player-2,1",
                "move|mon:Pidgeot,player-1,1|name:Quick Attack|target:Pidgeot,player-2,1",
                "split|side:1",
                "damage|mon:Pidgeot,player-2,1|health:116/143",
                "damage|mon:Pidgeot,player-2,1|health:82/100",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Pidgeot,player-2,1|name:Mirror Move|noanim",
                "fail|mon:Pidgeot,player-2,1",
                "residual",
                "turn|turn:6",
                ["time"],
                "move|mon:Pidgeot,player-2,1|name:Razor Wind|noanim",
                "prepare|mon:Pidgeot,player-2,1|move:Razor Wind",
                "move|mon:Pidgeot,player-1,1|name:Mirror Move|noanim",
                "fail|mon:Pidgeot,player-1,1",
                "residual",
                "turn|turn:7",
                ["time"],
                "move|mon:Pidgeot,player-2,1|name:Razor Wind",
                "split|side:0",
                "damage|mon:Pidgeot,player-1,1|health:69/143",
                "damage|mon:Pidgeot,player-1,1|health:49/100",
                "move|mon:Pidgeot,player-1,1|name:Mirror Move|target:Pidgeot,player-2,1",
                "move|mon:Pidgeot,player-1,1|name:Razor Wind|noanim",
                "prepare|mon:Pidgeot,player-1,1|move:Razor Wind",
                "residual",
                "turn|turn:8",
                ["time"],
                "move|mon:Pidgeot,player-2,1|name:Mirror Move|noanim",
                "fail|mon:Pidgeot,player-2,1",
                "move|mon:Pidgeot,player-1,1|name:Razor Wind",
                "split|side:1",
                "damage|mon:Pidgeot,player-2,1|health:67/143",
                "damage|mon:Pidgeot,player-2,1|health:47/100",
                "residual",
                "turn|turn:9",
                ["time"],
                "move|mon:Pidgeot,player-2,1|name:Mirror Move|noanim",
                "fail|mon:Pidgeot,player-2,1",
                "residual",
                "turn|turn:10"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn mirror_move_locks_target_like_source_move() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(
            &data,
            BattleType::Doubles,
            0,
            team().unwrap(),
            team().unwrap(),
        )
        .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "move 4,2;pass"),
            Ok(())
        );

        assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

        assert_eq!(
            battle.set_player_choice("player-1", "move 0,1;pass"),
            Ok(())
        );
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

        assert_eq!(
            battle.set_player_choice("player-1", "move 0,2;pass"),
            Ok(())
        );
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
                "switch|player:player-1|position:1|name:Pidgeot|health:100/100|species:Pidgeot|level:50|gender:M",
                "switch|player:player-1|position:2|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "switch|player:player-2|position:1|name:Pidgeot|health:100/100|species:Pidgeot|level:50|gender:M",
                "switch|player:player-2|position:2|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Pidgeot,player-2,1|name:Fly|noanim",
                "prepare|mon:Pidgeot,player-2,1|move:Fly",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pidgeot,player-2,1|name:Fly|target:Pikachu,player-1,2",
                "resisted|mon:Pikachu,player-1,2",
                "split|side:0",
                "damage|mon:Pikachu,player-1,2|health:41/95",
                "damage|mon:Pikachu,player-1,2|health:44/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Pidgeot,player-1,1|name:Mirror Move|target:Pidgeot,player-2,1",
                "move|mon:Pidgeot,player-1,1|name:Fly|noanim",
                "prepare|mon:Pidgeot,player-1,1|move:Fly",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Pidgeot,player-1,1|name:Fly|target:Pidgeot,player-2,1",
                "split|side:1",
                "damage|mon:Pidgeot,player-2,1|health:85/143",
                "damage|mon:Pidgeot,player-2,1|health:60/100",
                "residual",
                "turn|turn:5"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
