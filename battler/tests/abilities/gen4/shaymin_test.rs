use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    FieldEnvironment,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
    static_local_data_store,
};

fn shaymin() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Shaymin",
                    "species": "Shaymin-Sky",
                    "ability": "No Ability",
                    "moves": [
                        "Secret Power"
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
        .with_controlled_rng(true)
        .with_field_environment(FieldEnvironment::Ice)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn shaymin_sky_reverts_when_frozen() {
    let mut battle = make_battle(0, shaymin().unwrap(), shaymin().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Shaymin,player-1,1|name:Secret Power|target:Shaymin,player-2,1",
            "split|side:1",
            "damage|mon:Shaymin,player-2,1|health:119/160",
            "damage|mon:Shaymin,player-2,1|health:75/100",
            "status|mon:Shaymin,player-2,1|status:Freeze",
            "split|side:1",
            "specieschange|player:player-2|position:1|name:Shaymin|health:119/160|status:Freeze|species:Shaymin|level:50|gender:U",
            "specieschange|player:player-2|position:1|name:Shaymin|health:75/100|status:Freeze|species:Shaymin|level:50|gender:U",
            "formechange|mon:Shaymin,player-2,1|species:Shaymin|from:species:Shaymin-Sky",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
