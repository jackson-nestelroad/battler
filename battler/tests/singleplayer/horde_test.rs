use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WildPlayerOptions,
    WrapResultError,
};
use battler_test_utils::{
    assert_new_logs_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Thunderbolt",
                        "Surf"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 20
                },
                {
                    "name": "Eevee",
                    "species": "Eevee",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 10
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn rattata() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Rattata",
                    "species": "Rattata",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 5
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_horde_battle(
    data: &dyn DataStore,
    seed: u64,
    team: TeamData,
    wild: TeamData,
) -> Result<PublicCoreBattle> {
    let mut builder = TestBattleBuilder::new()
        .with_battle_type(BattleType::Multi)
        .with_adjacenecy_reach(3)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .with_team("protagonist", team);
    for i in 0..5 {
        let id = format!("wild-{i}");
        builder = builder
            .add_wild_mon_to_side_2(&id, "Horde", WildPlayerOptions::default())
            .with_team(&id, wild.clone());
    }
    builder.build(data)
}

#[test]
fn player_can_hit_all_adjacent_foes() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_horde_battle(&data, 0, team().unwrap(), rattata().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 0,5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-0", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-3", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-4", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-0", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild-3", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "info|battletype:Multi",
            "info|environment:Normal",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:5",
            "player|id:protagonist|name:Protagonist|side:0|position:2",
            "player|id:wild-0|name:Horde|side:1|position:0",
            "player|id:wild-1|name:Horde|side:1|position:1",
            "player|id:wild-2|name:Horde|side:1|position:2",
            "player|id:wild-3|name:Horde|side:1|position:3",
            "player|id:wild-4|name:Horde|side:1|position:4",
            ["time"],
            "teamsize|player:protagonist|size:2",
            "battlestart",
            "split|side:1",
            "appear|player:wild-0|position:1|name:Rattata|health:18/18|species:Rattata|level:5|gender:M",
            "appear|player:wild-0|position:1|name:Rattata|health:100/100|species:Rattata|level:5|gender:M",
            "split|side:1",
            "appear|player:wild-1|position:2|name:Rattata|health:18/18|species:Rattata|level:5|gender:M",
            "appear|player:wild-1|position:2|name:Rattata|health:100/100|species:Rattata|level:5|gender:M",
            "split|side:1",
            "appear|player:wild-2|position:3|name:Rattata|health:18/18|species:Rattata|level:5|gender:M",
            "appear|player:wild-2|position:3|name:Rattata|health:100/100|species:Rattata|level:5|gender:M",
            "split|side:1",
            "appear|player:wild-3|position:4|name:Rattata|health:18/18|species:Rattata|level:5|gender:M",
            "appear|player:wild-3|position:4|name:Rattata|health:100/100|species:Rattata|level:5|gender:M",
            "split|side:1",
            "appear|player:wild-4|position:5|name:Rattata|health:18/18|species:Rattata|level:5|gender:M",
            "appear|player:wild-4|position:5|name:Rattata|health:100/100|species:Rattata|level:5|gender:M",
            "split|side:0",
            "switch|player:protagonist|position:3|name:Pikachu|health:44/44|species:Pikachu|level:20|gender:M",
            "switch|player:protagonist|position:3|name:Pikachu|health:100/100|species:Pikachu|level:20|gender:M",
            "turn|turn:1",
            ["time"],
            "move|mon:Pikachu,protagonist,3|name:Thunderbolt|target:Rattata,wild-4,5",
            "split|side:1",
            "damage|mon:Rattata,wild-4,5|health:0",
            "damage|mon:Rattata,wild-4,5|health:0",
            "faint|mon:Rattata,wild-4,5",
            "exp|mon:Pikachu,protagonist,3|exp:14",
            "move|mon:Rattata,wild-0,1|name:Tackle|target:Pikachu,protagonist,3",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,3|health:41/44",
            "damage|mon:Pikachu,protagonist,3|health:94/100",
            "move|mon:Rattata,wild-1,2|name:Tackle|target:Pikachu,protagonist,3",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,3|health:38/44",
            "damage|mon:Pikachu,protagonist,3|health:87/100",
            "move|mon:Rattata,wild-2,3|name:Tackle|target:Pikachu,protagonist,3",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,3|health:35/44",
            "damage|mon:Pikachu,protagonist,3|health:80/100",
            "move|mon:Rattata,wild-3,4|name:Tackle|target:Pikachu,protagonist,3",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,3|health:32/44",
            "damage|mon:Pikachu,protagonist,3|health:73/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,protagonist,3|name:Surf|spread:Rattata,wild-0,1;Rattata,wild-1,2;Rattata,wild-2,3;Rattata,wild-3,4",
            "split|side:1",
            "damage|mon:Rattata,wild-0,1|health:0",
            "damage|mon:Rattata,wild-0,1|health:0",
            "split|side:1",
            "damage|mon:Rattata,wild-1,2|health:0",
            "damage|mon:Rattata,wild-1,2|health:0",
            "split|side:1",
            "damage|mon:Rattata,wild-2,3|health:0",
            "damage|mon:Rattata,wild-2,3|health:0",
            "split|side:1",
            "damage|mon:Rattata,wild-3,4|health:0",
            "damage|mon:Rattata,wild-3,4|health:0",
            "faint|mon:Rattata,wild-0,1",
            "faint|mon:Rattata,wild-1,2",
            "faint|mon:Rattata,wild-2,3",
            "faint|mon:Rattata,wild-3,4",
            "exp|mon:Pikachu,protagonist,3|exp:56",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_new_logs_eq(&mut battle, &expected_logs);
}
