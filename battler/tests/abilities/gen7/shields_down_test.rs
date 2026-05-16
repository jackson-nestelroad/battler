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
    assert_logs_since_start_eq,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Minior",
                    "species": "Minior-Green",
                    "ability": "Shields Down",
                    "moves": [
                        "Recover",
                        "Stone Edge",
                        "Toxic",
                        "Yawn"
                    ],
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
fn shields_down_transforms_minior_based_on_hp() {
    let mut team_1 = team().unwrap();
    team_1.members[0].persistent_battle_data.hp = Some(50);
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Minior"],
            ["switch", "player-1", "Minior"],
            "split|side:1",
            ["switch", "player-2", "Minior"],
            ["switch", "player-2", "Minior"],
            "formechange|mon:Minior,player-2,1|species:Minior-Meteor|from:ability:Shields Down",
            "turn|turn:1",
            "continue",
            "move|mon:Minior,player-1,1|name:Recover|target:Minior,player-1,1",
            "split|side:0",
            "heal|mon:Minior,player-1,1|health:165/230",
            "heal|mon:Minior,player-1,1|health:72/100",
            "formechange|mon:Minior,player-1,1|species:Minior-Meteor|from:ability:Shields Down",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Minior,player-1,1|name:Stone Edge|target:Minior,player-2,1",
            "supereffective|mon:Minior,player-2,1",
            "split|side:1",
            "damage|mon:Minior,player-2,1|health:78/230",
            "damage|mon:Minior,player-2,1|health:34/100",
            "formechange|mon:Minior,player-2,1|species:Minior-Green|from:ability:Shields Down",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn minior_meteor_is_immune_to_status_conditions() {
    let mut battle = make_battle(12345, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Minior,player-1,1|name:Toxic|noanim",
            "immune|mon:Minior,player-2,1|from:ability:Shields Down",
            "fail|mon:Minior,player-1,1",
            "move|mon:Minior,player-2,1|name:Yawn|noanim",
            "immune|mon:Minior,player-1,1|from:ability:Shields Down",
            "fail|mon:Minior,player-2,1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Minior,player-1,1|name:Stone Edge|target:Minior,player-2,1",
            "supereffective|mon:Minior,player-2,1",
            "split|side:1",
            "damage|mon:Minior,player-2,1|health:92/230",
            "damage|mon:Minior,player-2,1|health:40/100",
            "formechange|mon:Minior,player-2,1|species:Minior-Green|from:ability:Shields Down",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Minior,player-1,1|name:Toxic|target:Minior,player-2,1",
            "status|mon:Minior,player-2,1|status:Bad Poison",
            "split|side:1",
            "damage|mon:Minior,player-2,1|from:status:Bad Poison|health:78/230",
            "damage|mon:Minior,player-2,1|from:status:Bad Poison|health:34/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
