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
                    "name": "Greninja",
                    "species": "Greninja",
                    "ability": "No Ability",
                    "moves": [
                        "Water Shuriken"
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn water_shuriken_stronger_from_ash_greninja() {
    let mut team_2 = team().unwrap();
    team_2.members[0].species = "Greninja-Ash".to_owned();
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Greninja,player-2,1|name:Water Shuriken|target:Greninja,player-1,1",
            "resisted|mon:Greninja,player-1,1",
            "split|side:0",
            "damage|mon:Greninja,player-1,1|health:233/254",
            "damage|mon:Greninja,player-1,1|health:92/100",
            "animatemove|mon:Greninja,player-2,1|name:Water Shuriken|target:Greninja,player-1,1",
            "resisted|mon:Greninja,player-1,1",
            "split|side:0",
            "damage|mon:Greninja,player-1,1|health:212/254",
            "damage|mon:Greninja,player-1,1|health:84/100",
            "hitcount|hits:2",
            "move|mon:Greninja,player-1,1|name:Water Shuriken|target:Greninja,player-2,1",
            "resisted|mon:Greninja,player-2,1",
            "split|side:1",
            "damage|mon:Greninja,player-2,1|health:239/254",
            "damage|mon:Greninja,player-2,1|health:95/100",
            "animatemove|mon:Greninja,player-1,1|name:Water Shuriken|target:Greninja,player-2,1",
            "resisted|mon:Greninja,player-2,1",
            "split|side:1",
            "damage|mon:Greninja,player-2,1|health:224/254",
            "damage|mon:Greninja,player-2,1|health:89/100",
            "hitcount|hits:2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
