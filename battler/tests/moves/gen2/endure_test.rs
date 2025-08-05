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

fn heracross() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Heracross",
                    "species": "Heracross",
                    "ability": "No Ability",
                    "moves": [
                        "Endure"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn fearow() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Fearow",
                    "species": "Fearow",
                    "ability": "No Ability",
                    "moves": [
                        "Aerial Ace"
                    ],
                    "nature": "Hardy",
                    "level": 100
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
fn endure_survives_fainting_attacks() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        901516767912874,
        heracross().unwrap(),
        fearow().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Heracross,player-1,1|name:Endure|target:Heracross,player-1,1",
            "singleturn|mon:Heracross,player-1,1|move:Endure",
            "move|mon:Fearow,player-2,1|name:Aerial Ace|target:Heracross,player-1,1",
            "supereffective|mon:Heracross,player-1,1",
            "activate|mon:Heracross,player-1,1|move:Endure",
            "split|side:0",
            "damage|mon:Heracross,player-1,1|health:1/140",
            "damage|mon:Heracross,player-1,1|health:1/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Heracross,player-1,1|name:Endure|target:Heracross,player-1,1",
            "singleturn|mon:Heracross,player-1,1|move:Endure",
            "move|mon:Fearow,player-2,1|name:Aerial Ace|target:Heracross,player-1,1",
            "supereffective|mon:Heracross,player-1,1",
            "activate|mon:Heracross,player-1,1|move:Endure",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Heracross,player-1,1|name:Endure|noanim",
            "fail|mon:Heracross,player-1,1",
            "move|mon:Fearow,player-2,1|name:Aerial Ace|target:Heracross,player-1,1",
            "supereffective|mon:Heracross,player-1,1",
            "split|side:0",
            "damage|mon:Heracross,player-1,1|health:0",
            "damage|mon:Heracross,player-1,1|health:0",
            "faint|mon:Heracross,player-1,1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
