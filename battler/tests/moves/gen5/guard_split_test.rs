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

fn shuckle() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Shuckle",
                    "species": "Shuckle",
                    "ability": "Sturdy",
                    "moves": [
                        "Guard Split"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "evs": {
                        "def": 252,
                        "spd": 252
                    }
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn pichu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pichu",
                    "species": "Pichu",
                    "ability": "Static",
                    "moves": [
                        "Earthquake"
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
fn guard_split_averages_defenses() {
    let mut battle = make_battle(0, shuckle().unwrap(), pichu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1
    // Pichu uses Tackle. Shuckle has very high defense, so damage is low.
    // Shuckle uses Guard Split. Defenses are averaged.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // Turn 2
    // Pichu uses Tackle again. Shuckle now has much lower defense, so damage is higher.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pichu,player-2,1|name:Earthquake",
            "split|side:0",
            "damage|mon:Shuckle,player-1,1|health:72/80",
            "damage|mon:Shuckle,player-1,1|health:90/100",
            "move|mon:Shuckle,player-1,1|name:Guard Split|target:Pichu,player-2,1",
            "activate|mon:Pichu,player-2,1|move:Guard Split|of:Shuckle,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pichu,player-2,1|name:Earthquake",
            "split|side:0",
            "damage|mon:Shuckle,player-1,1|health:59/80",
            "damage|mon:Shuckle,player-1,1|health:74/100",
            "move|mon:Shuckle,player-1,1|name:Guard Split|target:Pichu,player-2,1",
            "activate|mon:Pichu,player-2,1|move:Guard Split|of:Shuckle,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
