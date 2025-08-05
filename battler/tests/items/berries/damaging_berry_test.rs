use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn oshawott() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Oshawott",
                    "species": "Oshawott",
                    "ability": "Torrent",
                    "moves": [
                        "Pound",
                        "Water Gun"
                    ],
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
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'_>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Min)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn jaboca_berry_damages_attacker_after_physical_damage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = oshawott().unwrap();
    team.members[0].item = Some("Jaboca Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, oshawott().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oshawott,player-2,1|name:Pound|target:Oshawott,player-1,1",
            "split|side:0",
            "damage|mon:Oshawott,player-1,1|health:96/115",
            "damage|mon:Oshawott,player-1,1|health:84/100",
            "itemend|mon:Oshawott,player-1,1|item:Jaboca Berry|eat",
            "split|side:1",
            "damage|mon:Oshawott,player-2,1|from:item:Jaboca Berry|of:Oshawott,player-1,1|health:101/115",
            "damage|mon:Oshawott,player-2,1|from:item:Jaboca Berry|of:Oshawott,player-1,1|health:88/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn rowap_berry_damages_attacker_after_special_damage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = oshawott().unwrap();
    team.members[0].item = Some("Rowap Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, oshawott().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oshawott,player-2,1|name:Water Gun|target:Oshawott,player-1,1",
            "resisted|mon:Oshawott,player-1,1",
            "split|side:0",
            "damage|mon:Oshawott,player-1,1|health:100/115",
            "damage|mon:Oshawott,player-1,1|health:87/100",
            "itemend|mon:Oshawott,player-1,1|item:Rowap Berry|eat",
            "split|side:1",
            "damage|mon:Oshawott,player-2,1|from:item:Rowap Berry|of:Oshawott,player-1,1|health:101/115",
            "damage|mon:Oshawott,player-2,1|from:item:Rowap Berry|of:Oshawott,player-1,1|health:88/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
