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
                    "name": "Genesect",
                    "species": "Genesect",
                    "ability": "No Ability",
                    "moves": [
                        "Fell Stinger"
                    ],
                    "nature": "Hardy",
                    "level": 100,
                    "persistent_battle_data": {
                        "hp": 50
                    }
                },
                {
                    "name": "Genesect",
                    "species": "Genesect",
                    "ability": "No Ability",
                    "moves": [
                        "Fell Stinger"
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
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn fell_stinger_boosts_attack_if_target_faints() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Genesect,player-1,1|name:Fell Stinger|target:Genesect,player-2,1",
            "resisted|mon:Genesect,player-2,1",
            "split|side:1",
            "damage|mon:Genesect,player-2,1|health:11/252",
            "damage|mon:Genesect,player-2,1|health:5/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Genesect,player-1,1|name:Fell Stinger|target:Genesect,player-2,1",
            "resisted|mon:Genesect,player-2,1",
            "split|side:1",
            "damage|mon:Genesect,player-2,1|health:0",
            "damage|mon:Genesect,player-2,1|health:0",
            "faint|mon:Genesect,player-2,1",
            "boost|mon:Genesect,player-1,1|stat:atk|by:3",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
