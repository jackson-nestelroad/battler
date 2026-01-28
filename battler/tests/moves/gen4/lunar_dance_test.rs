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

fn cresselia() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Cresselia",
                    "species": "Cresselia",
                    "ability": "No Ability",
                    "moves": [
                        "Lunar Dance",
                        "Tackle",
                        "Spore"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn three_cresselia() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Cresselia",
                    "species": "Cresselia",
                    "ability": "No Ability",
                    "moves": [
                        "Lunar Dance",
                        "Tackle",
                        "Spore"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Cresselia",
                    "species": "Cresselia",
                    "ability": "No Ability",
                    "moves": [
                        "Lunar Dance",
                        "Tackle",
                        "Spore"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Cresselia",
                    "species": "Cresselia",
                    "ability": "No Ability",
                    "moves": [
                        "Lunar Dance",
                        "Tackle",
                        "Spore"
                    ],
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
fn lunar_dance_fails_if_cannot_switch() {
    let mut battle = make_battle(0, cresselia().unwrap(), cresselia().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cresselia,player-1,1|name:Lunar Dance|noanim",
            "fail|mon:Cresselia,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn lunar_dance_faints_user_and_heals_slot() {
    let mut battle =
        make_battle(0, three_cresselia().unwrap(), three_cresselia().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // PP was restored.
    assert_matches::assert_matches!(battle.request_for_player("player-1"),
    Ok(Some(Request::Turn(request))) => {     assert_eq!(request.active[0].moves[1].pp,
    request.active[0].moves[1].max_pp); });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cresselia,player-1,1|name:Lunar Dance|target:Cresselia,player-1,1",
            "faint|mon:Cresselia,player-1,1",
            "residual",
            "continue",
            "split|side:0",
            "switch|player:player-1|position:1|name:Cresselia|health:180/180|species:Cresselia|level:50|gender:U",
            "switch|player:player-1|position:1|name:Cresselia|health:100/100|species:Cresselia|level:50|gender:U",
            "turn|turn:5",
            "continue",
            "move|mon:Cresselia,player-2,1|name:Tackle|target:Cresselia,player-1,1",
            "split|side:0",
            "damage|mon:Cresselia,player-1,1|health:168/180",
            "damage|mon:Cresselia,player-1,1|health:94/100",
            "residual",
            "turn|turn:6",
            "continue",
            "split|side:0",
            "switch|player:player-1|position:1|name:Cresselia|health:169/180|status:Sleep|species:Cresselia|level:50|gender:U",
            "switch|player:player-1|position:1|name:Cresselia|health:94/100|status:Sleep|species:Cresselia|level:50|gender:U",
            "activate|mon:Cresselia,player-1,1|move:Lunar Dance",
            "split|side:0",
            "heal|mon:Cresselia,player-1,1|from:move:Lunar Dance|health:180/180",
            "heal|mon:Cresselia,player-1,1|from:move:Lunar Dance|health:100/100",
            "curestatus|mon:Cresselia,player-1,1|status:Sleep|from:move:Lunar Dance",
            "residual",
            "turn|turn:7",
            "continue",
            "split|side:0",
            "switch|player:player-1|position:1|name:Cresselia|health:168/180|species:Cresselia|level:50|gender:U",
            "switch|player:player-1|position:1|name:Cresselia|health:94/100|species:Cresselia|level:50|gender:U",
            "residual",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 4, &expected_logs);
}
