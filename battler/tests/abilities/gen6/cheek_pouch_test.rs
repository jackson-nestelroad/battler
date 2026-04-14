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
                    "name": "Diggersby",
                    "species": "Diggersby",
                    "ability": "Cheek Pouch",
                    "item": "Lum Berry",
                    "moves": [
                        "Brick Break",
                        "Toxic",
                        "Pluck"
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
fn cheek_pouch_heals_on_eat() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Diggersby,player-2,1|name:Brick Break|target:Diggersby,player-1,1",
            "supereffective|mon:Diggersby,player-1,1",
            "split|side:0",
            "damage|mon:Diggersby,player-1,1|health:188/280",
            "damage|mon:Diggersby,player-1,1|health:68/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Diggersby,player-2,1|name:Toxic|target:Diggersby,player-1,1",
            "status|mon:Diggersby,player-1,1|status:Bad Poison",
            "itemend|mon:Diggersby,player-1,1|item:Lum Berry|eat",
            "curestatus|mon:Diggersby,player-1,1|status:Bad Poison|from:item:Lum Berry",
            "split|side:0",
            "heal|mon:Diggersby,player-1,1|from:ability:Cheek Pouch|health:280/280",
            "heal|mon:Diggersby,player-1,1|from:ability:Cheek Pouch|health:100/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Diggersby,player-2,1|name:Brick Break|target:Diggersby,player-1,1",
            "supereffective|mon:Diggersby,player-1,1",
            "split|side:0",
            "damage|mon:Diggersby,player-1,1|health:194/280",
            "damage|mon:Diggersby,player-1,1|health:70/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Diggersby,player-1,1|name:Pluck|target:Diggersby,player-2,1",
            "split|side:1",
            "damage|mon:Diggersby,player-2,1|health:242/280",
            "damage|mon:Diggersby,player-2,1|health:87/100",
            "itemend|mon:Diggersby,player-2,1|item:Lum Berry|from:move:Pluck|of:Diggersby,player-1,1",
            "split|side:0",
            "heal|mon:Diggersby,player-1,1|from:ability:Cheek Pouch|health:280/280",
            "heal|mon:Diggersby,player-1,1|from:ability:Cheek Pouch|health:100/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
