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
                    "name": "Wooloo",
                    "species": "Wooloo",
                    "ability": "Fluffy",
                    "moves": [
                        "Tackle",
                        "Ember",
                        "Fire Punch"
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
fn fluffy_reduces_contact_move_damage() {
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Wooloo,player-1,1|name:Tackle|target:Wooloo,player-2,1",
            "split|side:1",
            "damage|mon:Wooloo,player-2,1|health:155/194",
            "damage|mon:Wooloo,player-2,1|health:80/100",
            "move|mon:Wooloo,player-2,1|name:Tackle|target:Wooloo,player-1,1",
            "split|side:0",
            "damage|mon:Wooloo,player-1,1|health:175/194",
            "damage|mon:Wooloo,player-1,1|health:91/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn fluffy_increases_fire_move_damage() {
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Wooloo,player-1,1|name:Ember|target:Wooloo,player-2,1",
            "split|side:1",
            "damage|mon:Wooloo,player-2,1|health:162/194",
            "damage|mon:Wooloo,player-2,1|health:84/100",
            "move|mon:Wooloo,player-2,1|name:Ember|target:Wooloo,player-1,1",
            "split|side:0",
            "damage|mon:Wooloo,player-1,1|health:130/194",
            "damage|mon:Wooloo,player-1,1|health:68/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn fluffy_does_not_modify_fire_contact_move_damage() {
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Wooloo,player-1,1|name:Fire Punch|target:Wooloo,player-2,1",
            "split|side:1",
            "damage|mon:Wooloo,player-2,1|health:146/194",
            "damage|mon:Wooloo,player-2,1|health:76/100",
            "move|mon:Wooloo,player-2,1|name:Fire Punch|target:Wooloo,player-1,1",
            "split|side:0",
            "damage|mon:Wooloo,player-1,1|health:146/194",
            "damage|mon:Wooloo,player-1,1|health:76/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
