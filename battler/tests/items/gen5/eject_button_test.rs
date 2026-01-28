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
                    "name": "Haxorus",
                    "species": "Haxorus",
                    "ability": "No Ability",
                    "moves": [
                        "U-Turn",
                        "Earthquake"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Eject Button"
                },
                {
                    "name": "Druddigon",
                    "species": "Druddigon",
                    "ability": "Telepathy",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Eject Button"
                },
                {
                    "name": "Kyurem",
                    "species": "Kyurem",
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

fn make_battle(
    seed: u64,
    battle_type: BattleType,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
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
fn eject_button_switches_target_out_on_hit() {
    let mut battle = make_battle(0, BattleType::Singles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Haxorus,player-1,1|name:U-turn|target:Haxorus,player-2,1",
            "split|side:1",
            "damage|mon:Haxorus,player-2,1|health:87/136",
            "damage|mon:Haxorus,player-2,1|health:64/100",
            "itemend|mon:Haxorus,player-2,1|item:Eject Button",
            "switchout|mon:Haxorus,player-2,1",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Druddigon"],
            ["switch", "player-2", "Druddigon"],
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn one_eject_button_triggers_at_a_time() {
    let mut battle = make_battle(0, BattleType::Doubles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Haxorus,player-1,1|name:Earthquake|spread:Haxorus,player-2,1;Druddigon,player-2,2",
            "activate|mon:Druddigon,player-1,2|ability:Telepathy",
            "split|side:1",
            "damage|mon:Haxorus,player-2,1|health:85/136",
            "damage|mon:Haxorus,player-2,1|health:63/100",
            "split|side:1",
            "damage|mon:Druddigon,player-2,2|health:89/137",
            "damage|mon:Druddigon,player-2,2|health:65/100",
            "itemend|mon:Haxorus,player-2,1|item:Eject Button",
            "switchout|mon:Haxorus,player-2,1",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Kyurem"],
            ["switch", "player-2", "Kyurem"],
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Haxorus,player-1,1|name:Earthquake|spread:Kyurem,player-2,1;Druddigon,player-2,2",
            "activate|mon:Druddigon,player-1,2|ability:Telepathy",
            "split|side:1",
            "damage|mon:Kyurem,player-2,1|health:132/185",
            "damage|mon:Kyurem,player-2,1|health:72/100",
            "split|side:1",
            "damage|mon:Druddigon,player-2,2|health:40/137",
            "damage|mon:Druddigon,player-2,2|health:30/100",
            "itemend|mon:Druddigon,player-2,2|item:Eject Button",
            "switchout|mon:Druddigon,player-2,2",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Haxorus"],
            ["switch", "player-2", "Haxorus"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
