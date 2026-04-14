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
                    "name": "Chesnaught",
                    "species": "Chesnaught",
                    "ability": "No Ability",
                    "moves": [
                        "Spiky Shield",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Incineroar",
                    "species": "Incineroar",
                    "ability": "No Ability",
                    "item": "Incinium Z",
                    "moves": [
                        "Darkest Lariat"
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
        .with_z_moves(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn spiky_shield_protects_user_and_deals_damage() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Chesnaught,player-1,1|name:Spiky Shield|target:Chesnaught,player-1,1",
            "singleturn|mon:Chesnaught,player-1,1|move:Spiky Shield",
            "move|mon:Chesnaught,player-2,1|name:Tackle|noanim",
            "activate|mon:Chesnaught,player-1,1|move:Spiky Shield",
            "split|side:1",
            "damage|mon:Chesnaught,player-2,1|from:move:Spiky Shield|health:251/286",
            "damage|mon:Chesnaught,player-2,1|from:move:Spiky Shield|health:88/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn spiky_shield_deals_damage_even_if_bypassed() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,zmove"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Chesnaught,player-1,1|name:Spiky Shield|target:Chesnaught,player-1,1",
            "singleturn|mon:Chesnaught,player-1,1|move:Spiky Shield",
            "singleturn|mon:Incineroar,player-2,1|condition:Z-Power",
            "move|mon:Incineroar,player-2,1|name:Malicious Moonsault|target:Chesnaught,player-1,1",
            "resisted|mon:Chesnaught,player-1,1",
            "crit|mon:Chesnaught,player-1,1",
            "protectweaken|mon:Chesnaught,player-1,1",
            "split|side:0",
            "damage|mon:Chesnaught,player-1,1|health:251/286",
            "damage|mon:Chesnaught,player-1,1|health:88/100",
            "split|side:1",
            "damage|mon:Incineroar,player-2,1|from:move:Spiky Shield|of:Chesnaught,player-1,1|health:263/300",
            "damage|mon:Incineroar,player-2,1|from:move:Spiky Shield|of:Chesnaught,player-1,1|health:88/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
