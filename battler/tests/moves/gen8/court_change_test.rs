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
                    "name": "Rillaboom",
                    "species": "Rillaboom",
                    "ability": "No Ability",
                    "moves": [
                        "Mist",
                        "Spikes",
                        "Stealth Rock",
                        "Tailwind"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Cinderace",
                    "species": "Cinderace",
                    "ability": "No Ability",
                    "moves": [
                        "Court Change",
                        "Light Screen",
                        "Reflect"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Inteleon",
                    "species": "Inteleon",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
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
fn court_change_swaps_side_conditions() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0;move 1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1;move 2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cinderace,player-1,2|name:Light Screen",
            "sidestart|side:0|move:Light Screen",
            "move|mon:Cinderace,player-2,2|name:Reflect",
            "sidestart|side:1|move:Reflect",
            "move|mon:Rillaboom,player-1,1|name:Mist",
            "sidestart|side:0|move:Mist",
            "move|mon:Rillaboom,player-2,1|name:Spikes",
            "sidestart|side:0|move:Spikes|count:1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Rillaboom,player-1,1|name:Stealth Rock",
            "sidestart|side:1|move:Stealth Rock",
            "move|mon:Rillaboom,player-2,1|name:Tailwind",
            "sidestart|side:1|move:Tailwind",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Cinderace,player-1,2|name:Court Change",
            "swapsideconditions|side:1|with:0|from:move:Court Change|of:Cinderace,player-1,2",
            "swapsidecondition|side:0|condition:Reflect|source:1|from:move:Court Change|of:Cinderace,player-1,2",
            "swapsidecondition|side:0|condition:Stealth Rock|source:1|from:move:Court Change|of:Cinderace,player-1,2",
            "swapsidecondition|side:0|condition:Tailwind|source:1|from:move:Court Change|of:Cinderace,player-1,2",
            "swapsidecondition|side:1|condition:Light Screen|source:0|from:move:Court Change|of:Cinderace,player-1,2",
            "swapsidecondition|side:1|condition:Mist|source:0|from:move:Court Change|of:Cinderace,player-1,2",
            "swapsidecondition|side:1|condition:Spikes|source:0|from:move:Court Change|of:Cinderace,player-1,2",
            "activate|move:Court Change|of:Cinderace,player-1,2",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn court_change_swaps_effect_states_and_durations() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Rillaboom,player-1,1|name:Spikes",
            "sidestart|side:1|move:Spikes|count:1",
            "move|mon:Rillaboom,player-2,1|name:Tailwind",
            "sidestart|side:1|move:Tailwind",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Rillaboom,player-2,1|name:Spikes",
            "sidestart|side:0|move:Spikes|count:1",
            "move|mon:Rillaboom,player-1,1|name:Spikes",
            "sidestart|side:1|move:Spikes|count:2",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Rillaboom,player-1,1|name:Tailwind",
            "sidestart|side:0|move:Tailwind",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Cinderace,player-1,2|name:Court Change",
            "swapsideconditions|side:1|with:0|from:move:Court Change|of:Cinderace,player-1,2",
            "swapsidecondition|side:0|condition:Spikes|source:1|from:move:Court Change|of:Cinderace,player-1,2",
            "swapsidecondition|side:0|condition:Tailwind|source:1|from:move:Court Change|of:Cinderace,player-1,2",
            "swapsidecondition|side:1|condition:Spikes|source:0|from:move:Court Change|of:Cinderace,player-1,2",
            "swapsidecondition|side:1|condition:Tailwind|source:0|from:move:Court Change|of:Cinderace,player-1,2",
            "activate|move:Court Change|of:Cinderace,player-1,2",
            "sideend|side:0|move:Tailwind",
            "residual",
            "turn|turn:5",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Inteleon"],
            ["switch", "player-2", "Inteleon"],
            "split|side:0",
            ["switch", "player-1", "Inteleon"],
            ["switch", "player-1", "Inteleon"],
            "split|side:1",
            "damage|mon:Inteleon,player-2,1|from:move:Spikes|health:219/250",
            "damage|mon:Inteleon,player-2,1|from:move:Spikes|health:88/100",
            "split|side:0",
            "damage|mon:Inteleon,player-1,1|from:move:Spikes|health:209/250",
            "damage|mon:Inteleon,player-1,1|from:move:Spikes|health:84/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
