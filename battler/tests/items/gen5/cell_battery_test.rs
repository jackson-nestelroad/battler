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
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn emolga() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Emolga",
                    "species": "Emolga",
                    "ability": "No Ability",
                    "moves": [
                        "Thunder Shock",
                        "Belly Drum",
                        "Embargo"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Cell Battery"
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
fn cell_battery_increases_atk_on_electric_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, emolga().unwrap(), emolga().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Emolga,player-1,1|name:Thunder Shock|target:Emolga,player-2,1",
            "split|side:1",
            "damage|mon:Emolga,player-2,1|health:82/115",
            "damage|mon:Emolga,player-2,1|health:72/100",
            "itemend|mon:Emolga,player-2,1|item:Cell Battery",
            "boost|mon:Emolga,player-2,1|stat:atk|by:1|from:item:Cell Battery",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn cell_battery_does_not_activate_if_atk_cannot_boost() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, emolga().unwrap(), emolga().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Emolga,player-1,1|name:Belly Drum|target:Emolga,player-1,1",
            "split|side:0",
            "damage|mon:Emolga,player-1,1|health:58/115",
            "damage|mon:Emolga,player-1,1|health:51/100",
            "boost|mon:Emolga,player-1,1|stat:atk|by:6|max",
            "move|mon:Emolga,player-2,1|name:Thunder Shock|target:Emolga,player-1,1",
            "split|side:0",
            "damage|mon:Emolga,player-1,1|health:25/115",
            "damage|mon:Emolga,player-1,1|health:22/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn embargo_prevents_cell_battery() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, emolga().unwrap(), emolga().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Emolga,player-1,1|name:Embargo|target:Emolga,player-2,1",
            "start|mon:Emolga,player-2,1|move:Embargo",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Emolga,player-1,1|name:Thunder Shock|target:Emolga,player-2,1",
            "split|side:1",
            "damage|mon:Emolga,player-2,1|health:82/115",
            "damage|mon:Emolga,player-2,1|health:72/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn cell_battery_on_contrary_mon_does_not_activate_if_atk_cannot_lower() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = emolga().unwrap();
    team.members[0].ability = "Contrary".to_owned();
    let mut battle = make_battle(&data, 0, team, emolga().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Emolga,player-1,1|name:Belly Drum|target:Emolga,player-1,1",
            "split|side:0",
            "damage|mon:Emolga,player-1,1|health:58/115",
            "damage|mon:Emolga,player-1,1|health:51/100",
            "unboost|mon:Emolga,player-1,1|stat:atk|by:6|min",
            "move|mon:Emolga,player-2,1|name:Thunder Shock|target:Emolga,player-1,1",
            "split|side:0",
            "damage|mon:Emolga,player-1,1|health:25/115",
            "damage|mon:Emolga,player-1,1|health:22/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
