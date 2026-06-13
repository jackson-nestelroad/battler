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
    assert_logs_since_start_eq,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Hydrapple",
                    "species": "Hydrapple",
                    "ability": "Supersweet Syrup",
                    "moves": [
                        "Substitute"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Dipplin",
                    "species": "Dipplin",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Applin",
                    "species": "Applin",
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

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
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
fn supersweet_syrup_drops_foe_evasion_on_start() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Hydrapple"],
            ["switch", "player-1", "Hydrapple"],
            "split|side:0",
            ["switch", "player-1", "Dipplin"],
            ["switch", "player-1", "Dipplin"],
            "split|side:1",
            ["switch", "player-2", "Hydrapple"],
            ["switch", "player-2", "Hydrapple"],
            "split|side:1",
            ["switch", "player-2", "Dipplin"],
            ["switch", "player-2", "Dipplin"],
            "ability|mon:Hydrapple,player-1,1|ability:Supersweet Syrup",
            "unboost|mon:Hydrapple,player-2,1|stat:eva|by:1|from:ability:Supersweet Syrup|of:Hydrapple,player-1,1",
            "unboost|mon:Dipplin,player-2,2|stat:eva|by:1|from:ability:Supersweet Syrup|of:Hydrapple,player-1,1",
            "ability|mon:Hydrapple,player-2,1|ability:Supersweet Syrup",
            "unboost|mon:Hydrapple,player-1,1|stat:eva|by:1|from:ability:Supersweet Syrup|of:Hydrapple,player-2,1",
            "unboost|mon:Dipplin,player-1,2|stat:eva|by:1|from:ability:Supersweet Syrup|of:Hydrapple,player-2,1",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn substitute_resists_supersweet_syrup() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 0;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Applin"],
            ["switch", "player-2", "Applin"],
            "move|mon:Hydrapple,player-1,1|name:Substitute|target:Hydrapple,player-1,1",
            "start|mon:Hydrapple,player-1,1|move:Substitute",
            "split|side:0",
            "damage|mon:Hydrapple,player-1,1|health:125/166",
            "damage|mon:Hydrapple,player-1,1|health:76/100",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Hydrapple"],
            ["switch", "player-2", "Hydrapple"],
            "ability|mon:Hydrapple,player-2,1|ability:Supersweet Syrup",
            "fail|mon:Hydrapple,player-1,1|what:unboost|from:move:Substitute",
            "unboost|mon:Dipplin,player-1,2|stat:eva|by:1|from:ability:Supersweet Syrup|of:Hydrapple,player-2,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
