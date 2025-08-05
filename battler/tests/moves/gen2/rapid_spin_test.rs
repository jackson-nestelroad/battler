use anyhow::Result;
use battler::{
    BattleType,
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

fn forretress() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Forretress",
                    "species": "Forretress",
                    "ability": "No Ability",
                    "moves": [
                        "Rapid Spin",
                        "Spikes",
                        "Toxic Spikes",
                        "Leech Seed",
                        "Bind"
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn rapid_spin_clears_user_entry_hazards() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, forretress().unwrap(), forretress().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Forretress,player-1,1|name:Rapid Spin|target:Forretress,player-2,1",
            "resisted|mon:Forretress,player-2,1",
            "split|side:1",
            "damage|mon:Forretress,player-2,1|health:128/135",
            "damage|mon:Forretress,player-2,1|health:95/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Forretress,player-2,1|name:Spikes",
            "sidestart|side:0|move:Spikes|count:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Forretress,player-2,1|name:Toxic Spikes",
            "sidestart|side:0|move:Toxic Spikes|count:1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Forretress,player-2,1|name:Leech Seed|target:Forretress,player-1,1",
            "start|mon:Forretress,player-1,1|move:Leech Seed",
            "split|side:0",
            "damage|mon:Forretress,player-1,1|from:move:Leech Seed|health:119/135",
            "damage|mon:Forretress,player-1,1|from:move:Leech Seed|health:89/100",
            "split|side:1",
            "heal|mon:Forretress,player-2,1|from:move:Leech Seed|of:Forretress,player-1,1|health:135/135",
            "heal|mon:Forretress,player-2,1|from:move:Leech Seed|of:Forretress,player-1,1|health:100/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Forretress,player-2,1|name:Bind|target:Forretress,player-1,1",
            "resisted|mon:Forretress,player-1,1",
            "split|side:0",
            "damage|mon:Forretress,player-1,1|health:117/135",
            "damage|mon:Forretress,player-1,1|health:87/100",
            "activate|mon:Forretress,player-1,1|move:Bind|of:Forretress,player-2,1",
            "split|side:0",
            "damage|mon:Forretress,player-1,1|from:move:Bind|health:101/135",
            "damage|mon:Forretress,player-1,1|from:move:Bind|health:75/100",
            "split|side:0",
            "damage|mon:Forretress,player-1,1|from:move:Leech Seed|health:85/135",
            "damage|mon:Forretress,player-1,1|from:move:Leech Seed|health:63/100",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Forretress,player-1,1|name:Rapid Spin|target:Forretress,player-2,1",
            "resisted|mon:Forretress,player-2,1",
            "split|side:1",
            "damage|mon:Forretress,player-2,1|health:128/135",
            "damage|mon:Forretress,player-2,1|health:95/100",
            "end|mon:Forretress,player-1,1|move:Bind",
            "end|mon:Forretress,player-1,1|move:Leech Seed",
            "sideend|side:0|move:Spikes",
            "sideend|side:0|move:Toxic Spikes",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
