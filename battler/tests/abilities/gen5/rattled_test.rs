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
                    "name": "Basculin",
                    "species": "Basculin-White-Striped",
                    "ability": "Rattled",
                    "moves": [
                        "Dark Pulse"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Mightyena",
                    "species": "Mightyena",
                    "ability": "Intimidate",
                    "moves": [],
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
fn rattled_boosts_speed() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1 "), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Basculin,player-1,1|name:Dark Pulse|target:Basculin,player-2,1",
            "split|side:1",
            "damage|mon:Basculin,player-2,1|health:81/130",
            "damage|mon:Basculin,player-2,1|health:63/100",
            "boost|mon:Basculin,player-2,1|stat:spe|by:1|from:ability:Rattled",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            "switch|player:player-1|position:1|name:Mightyena|health:130/130|species:Mightyena|level:50|gender:U",
            "switch|player:player-1|position:1|name:Mightyena|health:100/100|species:Mightyena|level:50|gender:U",
            "activate|mon:Mightyena,player-1,1|ability:Intimidate",
            "unboost|mon:Basculin,player-2,1|stat:atk|by:1|from:ability:Intimidate|of:Mightyena,player-1,1",
            "boost|mon:Basculin,player-2,1|stat:spe|by:1|from:ability:Rattled",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
