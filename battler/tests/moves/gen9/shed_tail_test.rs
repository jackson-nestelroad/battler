use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
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
                    "name": "Cyclizar",
                    "species": "Cyclizar",
                    "ability": "No Ability",
                    "moves": [
                        "Shed Tail",
                        "Flamethrower",
                        "Pursuit"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Shedinja",
                    "species": "Shedinja",
                    "ability": "No Ability",
                    "moves": [
                        "Shed Tail"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Meowscarada",
                    "species": "Meowscarada",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
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
fn shed_tail_passes_substitute_with_switch_in() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cyclizar,player-1,1|name:Shed Tail|target:Cyclizar,player-1,1",
            "start|mon:Cyclizar,player-1,1|move:Substitute",
            "split|side:0",
            "damage|mon:Cyclizar,player-1,1|health:65/130",
            "damage|mon:Cyclizar,player-1,1|health:50/100",
            "switchout|mon:Cyclizar,player-1,1|copysubstitute",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Meowscarada"],
            ["switch", "player-1", "Meowscarada"],
            "move|mon:Cyclizar,player-2,1|name:Flamethrower|target:Meowscarada,player-1,1",
            "supereffective|mon:Meowscarada,player-1,1",
            "end|mon:Meowscarada,player-1,1|move:Substitute",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            "switch|player:player-1|position:1|name:Cyclizar|health:65/130|species:Cyclizar|level:50|gender:U",
            "switch|player:player-1|position:1|name:Cyclizar|health:50/100|species:Cyclizar|level:50|gender:U",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Cyclizar,player-1,1|name:Shed Tail|noanim",
            "fail|mon:Cyclizar,player-1,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn shed_tail_fails_for_shedinja() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Shedinja"],
            ["switch", "player-1", "Shedinja"],
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Shedinja,player-1,1|name:Shed Tail|noanim",
            "fail|mon:Shedinja,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn shed_tail_switch_avoids_pursuit() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cyclizar,player-1,1|name:Shed Tail|target:Cyclizar,player-1,1",
            "start|mon:Cyclizar,player-1,1|move:Substitute",
            "split|side:0",
            "damage|mon:Cyclizar,player-1,1|health:65/130",
            "damage|mon:Cyclizar,player-1,1|health:50/100",
            "switchout|mon:Cyclizar,player-1,1|copysubstitute",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Meowscarada"],
            ["switch", "player-1", "Meowscarada"],
            "move|mon:Cyclizar,player-2,1|name:Pursuit|target:Meowscarada,player-1,1",
            "resisted|mon:Meowscarada,player-1,1",
            "activate|mon:Meowscarada,player-1,1|move:Substitute|damage",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
