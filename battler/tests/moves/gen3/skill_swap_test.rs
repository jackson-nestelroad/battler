use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Plusle",
                    "species": "Plusle",
                    "ability": "No Ability",
                    "moves": [
                        "Skill Swap",
                        "Growl"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Minun",
                    "species": "Minun",
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
    data: &dyn DataStore,
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'_>> {
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
        .build(data)
}

#[test]
fn skill_swap_swaps_abilities() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut user = team().unwrap();
    user.members[0].ability = "Soundproof".to_owned();
    let mut target = team().unwrap();
    target.members[0].ability = "Drizzle".to_owned();
    let mut battle = make_battle(&data, BattleType::Singles, 0, user, target).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Plusle,player-1,1|name:Skill Swap|target:Plusle,player-2,1",
            "activate|mon:Plusle,player-2,1|move:Skill Swap|of:Plusle,player-1,1",
            "abilityend|mon:Plusle,player-1,1|ability:Soundproof|from:move:Skill Swap|of:Plusle,player-2,1",
            "ability|mon:Plusle,player-1,1|ability:Drizzle|from:move:Skill Swap|of:Plusle,player-2,1",
            "abilityend|mon:Plusle,player-2,1|ability:Drizzle|from:move:Skill Swap|of:Plusle,player-1,1",
            "ability|mon:Plusle,player-2,1|ability:Soundproof|from:move:Skill Swap|of:Plusle,player-1,1",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Plusle,player-2,1|name:Growl",
            "unboost|mon:Plusle,player-1,1|stat:atk|by:1",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn skill_swap_fails_for_forbidden_ability() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut user = team().unwrap();
    user.members[0].ability = "Wonder Guard".to_owned();
    let mut battle = make_battle(&data, BattleType::Singles, 0, user, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Plusle,player-1,1|name:Skill Swap|noanim",
            "fail|mon:Plusle,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn skill_swap_is_private_for_allies() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut user = team().unwrap();
    user.members[0].ability = "Soundproof".to_owned();
    let mut battle = make_battle(&data, BattleType::Doubles, 0, user, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Plusle,player-1,1|name:Skill Swap|target:Minun,player-1,2",
            "activate|mon:Minun,player-1,2|move:Skill Swap|of:Plusle,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Plusle,player-2,1|name:Growl|spread:Plusle,player-1,1",
            "immune|mon:Minun,player-1,2|from:ability:Soundproof",
            "unboost|mon:Plusle,player-1,1|stat:atk|by:1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
