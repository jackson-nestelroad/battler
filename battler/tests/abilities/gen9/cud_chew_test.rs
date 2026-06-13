use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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
                    "name": "Farigiraf",
                    "species": "Farigiraf",
                    "ability": "Cud Chew",
                    "item": "Sitrus Berry",
                    "moves": [
                        "Dark Pulse"
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn cud_chew_reconsumes_berry_at_end_of_next_turn_once() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Farigiraf,player-1,1|name:Dark Pulse|target:Farigiraf,player-2,1",
            "supereffective|mon:Farigiraf,player-2,1",
            "split|side:1",
            "damage|mon:Farigiraf,player-2,1|health:70/180",
            "damage|mon:Farigiraf,player-2,1|health:39/100",
            "itemend|mon:Farigiraf,player-2,1|item:Sitrus Berry|eat",
            "split|side:1",
            "heal|mon:Farigiraf,player-2,1|from:item:Sitrus Berry|health:115/180",
            "heal|mon:Farigiraf,player-2,1|from:item:Sitrus Berry|health:64/100",
            "residual",
            "turn|turn:2",
            "continue",
            "activate|mon:Farigiraf,player-2,1|ability:Cud Chew",
            "split|side:1",
            "heal|mon:Farigiraf,player-2,1|from:item:Sitrus Berry|health:160/180",
            "heal|mon:Farigiraf,player-2,1|from:item:Sitrus Berry|health:89/100",
            "residual",
            "turn|turn:3",
            "continue",
            "residual",
            "turn|turn:4",
            "continue",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
