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
                    "name": "Kommo-o",
                    "species": "Kommo-o",
                    "ability": "No Ability",
                    "moves": [
                        "Clangorous Soulblaze"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Mew 1",
                    "species": "Mew",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Mew 2",
                    "species": "Mew",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Clefable 1",
                    "species": "Clefable",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Clefable 2",
                    "species": "Clefable",
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
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
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
fn clangorous_soulblaze_boosts_user_once_in_doubles() {
    let mut team_2 = team().unwrap();
    team_2.members.remove(0);
    team_2.members.truncate(2);
    let mut battle = make_battle(BattleType::Doubles, 0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Kommo-o,player-1,1|name:Clangorous Soulblaze|spread:Mew 1,player-2,1;Mew 2,player-2,2",
            "crit|mon:Mew 1,player-2,1",
            "split|side:1",
            "damage|mon:Mew 1,player-2,1|health:21/160",
            "damage|mon:Mew 1,player-2,1|health:14/100",
            "split|side:1",
            "damage|mon:Mew 2,player-2,2|health:67/160",
            "damage|mon:Mew 2,player-2,2|health:42/100",
            "boost|mon:Kommo-o,player-1,1|stat:atk|by:1",
            "boost|mon:Kommo-o,player-1,1|stat:def|by:1",
            "boost|mon:Kommo-o,player-1,1|stat:spa|by:1",
            "boost|mon:Kommo-o,player-1,1|stat:spd|by:1",
            "boost|mon:Kommo-o,player-1,1|stat:spe|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn clangorous_soulblaze_does_not_boost_if_all_targets_immune() {
    let mut team_2 = team().unwrap();
    team_2.members.retain(|mon| mon.species == "Clefable");
    let mut battle = make_battle(BattleType::Doubles, 0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Kommo-o,player-1,1|name:Clangorous Soulblaze|noanim",
            "immune|mon:Clefable 1,player-2,1",
            "immune|mon:Clefable 2,player-2,2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn clangorous_soulblaze_boosts_if_at_least_one_target_hit() {
    let mut team_2 = team().unwrap();
    team_2.members.swap(0, 3);
    team_2.members.truncate(2);
    let mut battle = make_battle(BattleType::Doubles, 0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Kommo-o,player-1,1|name:Clangorous Soulblaze|spread:Mew 1,player-2,2",
            "immune|mon:Clefable 1,player-2,1",
            "crit|mon:Mew 1,player-2,2",
            "split|side:1",
            "damage|mon:Mew 1,player-2,2|health:21/160",
            "damage|mon:Mew 1,player-2,2|health:14/100",
            "boost|mon:Kommo-o,player-1,1|stat:atk|by:1",
            "boost|mon:Kommo-o,player-1,1|stat:def|by:1",
            "boost|mon:Kommo-o,player-1,1|stat:spa|by:1",
            "boost|mon:Kommo-o,player-1,1|stat:spd|by:1",
            "boost|mon:Kommo-o,player-1,1|stat:spe|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sheer_force_boosts_clangorous_soulblaze_and_negates_boosts() {
    let mut team_1 = team().unwrap();
    team_1.members[0].ability = "Sheer Force".to_owned();
    let mut team_2 = team().unwrap();
    team_2.members.remove(0);
    team_2.members.truncate(2);
    let mut battle = make_battle(BattleType::Doubles, 0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Kommo-o,player-1,1|name:Clangorous Soulblaze|spread:Mew 1,player-2,1;Mew 2,player-2,2",
            "crit|mon:Mew 1,player-2,1",
            "split|side:1",
            "damage|mon:Mew 1,player-2,1|health:0",
            "damage|mon:Mew 1,player-2,1|health:0",
            "split|side:1",
            "damage|mon:Mew 2,player-2,2|health:40/160",
            "damage|mon:Mew 2,player-2,2|health:25/100",
            "faint|mon:Mew 1,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
