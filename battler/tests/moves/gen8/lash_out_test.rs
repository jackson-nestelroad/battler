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
                    "name": "Grimmsnarl",
                    "species": "Grimmsnarl",
                    "ability": "No Ability",
                    "moves": [
                        "Lash Out",
                        "Growl"
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn lash_out_doubles_power_if_stats_lowered_this_turn() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Grimmsnarl,player-2,1|name:Lash Out|target:Grimmsnarl,player-1,1",
            "resisted|mon:Grimmsnarl,player-1,1",
            "split|side:0",
            "damage|mon:Grimmsnarl,player-1,1|health:257/300",
            "damage|mon:Grimmsnarl,player-1,1|health:86/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Grimmsnarl,player-1,1|name:Growl",
            "unboost|mon:Grimmsnarl,player-2,1|stat:atk|by:1",
            "move|mon:Grimmsnarl,player-2,1|name:Lash Out|target:Grimmsnarl,player-1,1",
            "resisted|mon:Grimmsnarl,player-1,1",
            "split|side:0",
            "damage|mon:Grimmsnarl,player-1,1|health:200/300",
            "damage|mon:Grimmsnarl,player-1,1|health:67/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
