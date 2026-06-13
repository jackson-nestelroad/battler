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
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Espathra",
                    "species": "Espathra",
                    "ability": "No Ability",
                    "item": "Mirror Herb",
                    "moves": [
                        "Swords Dance",
                        "Torch Song",
                        "Tackle"
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn mirror_herb_copies_stat_boosts() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Espathra,player-1,1|name:Swords Dance|target:Espathra,player-1,1",
            "boost|mon:Espathra,player-1,1|stat:atk|by:2",
            "itemend|mon:Espathra,player-2,1|item:Mirror Herb",
            "boost|mon:Espathra,player-2,1|stat:atk|by:2|from:item:Mirror Herb",
            "move|mon:Espathra,player-2,1|name:Swords Dance|target:Espathra,player-2,1",
            "boost|mon:Espathra,player-2,1|stat:atk|by:2",
            "itemend|mon:Espathra,player-1,1|item:Mirror Herb",
            "boost|mon:Espathra,player-1,1|stat:atk|by:2|from:item:Mirror Herb",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mirror_herb_compounds_stat_boosts() {
    let mut team_1 = team().unwrap();
    team_1.members[0].item = Some("Throat Spray".to_owned());
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Espathra,player-1,1|name:Torch Song|target:Espathra,player-2,1",
            "split|side:1",
            "damage|mon:Espathra,player-2,1|health:98/155",
            "damage|mon:Espathra,player-2,1|health:64/100",
            "boost|mon:Espathra,player-1,1|stat:spa|by:1",
            "itemend|mon:Espathra,player-1,1|item:Throat Spray",
            "boost|mon:Espathra,player-1,1|stat:spa|by:1|from:item:Throat Spray",
            "itemend|mon:Espathra,player-2,1|item:Mirror Herb",
            "boost|mon:Espathra,player-2,1|stat:spa|by:2|from:item:Mirror Herb",
            "move|mon:Espathra,player-2,1|name:Tackle|target:Espathra,player-1,1",
            "split|side:0",
            "damage|mon:Espathra,player-1,1|health:138/155",
            "damage|mon:Espathra,player-1,1|health:90/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
