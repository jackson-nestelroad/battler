use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    Request,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
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
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn disable_disables_last_used_move() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Aerodactyl's Tackle is disabled")
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Aerodactyl"],
            ["switch", "player-2", "Aerodactyl"],
            "move|mon:Alakazam,player-1,1|name:Disable|noanim",
            "fail|mon:Alakazam,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Aerodactyl,player-2,1|name:Tackle|target:Alakazam,player-1,1",
            "split|side:0",
            "damage|mon:Alakazam,player-1,1|health:77/115",
            "damage|mon:Alakazam,player-1,1|health:67/100",
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
            "damage|mon:Alakazam,player-1,1|health:21/115",
            "damage|mon:Alakazam,player-1,1|health:19/100",
            "residual",
            "turn|turn:8",
            ["time"],
            "move|mon:Alakazam,player-1,1|name:Disable|target:Aerodactyl,player-2,1",
            "start|mon:Aerodactyl,player-2,1|move:Disable|disabledmove:Tackle",
            "residual",
            "turn|turn:9"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn disable_ends_locked_move_and_forces_struggle() {
    let mut battle = make_battle(1060328782717467, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-2"), Ok(Some(Request::Turn(request))) => {
        assert_matches::assert_matches!(request.active.first(), Some(mon) => {
            assert_eq!(mon.moves.len(), 1);
            assert_matches::assert_matches!(mon.moves.first(), Some(move_slot) => {
                assert_eq!(move_slot.name, "Struggle");
                assert_eq!(&move_slot.id, "struggle");
            });
        });
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Slowbro"],
            ["switch", "player-2", "Slowbro"],
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
            "cant|mon:Slowbro,player-2,1|from:move:Disable",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Alakazam,player-1,1|name:Disable|noanim",
            "fail|mon:Alakazam,player-1,1",
            "move|mon:Slowbro,player-2,1|name:Struggle|target:Alakazam,player-1,1",
            "split|side:0",
            "damage|mon:Alakazam,player-1,1|health:6/115",
            "damage|mon:Alakazam,player-1,1|health:6/100",
            "split|side:1",
            "damage|mon:Slowbro,player-2,1|from:Struggle Recoil|health:116/155",
            "damage|mon:Slowbro,player-2,1|from:Struggle Recoil|health:75/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
