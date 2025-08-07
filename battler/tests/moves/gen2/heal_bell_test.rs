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

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Miltank",
                    "species": "Miltank",
                    "ability": "Soundproof",
                    "moves": [
                        "Heal Bell",
                        "Thunder Wave",
                        "Sleep Powder",
                        "Toxic"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Meganium",
                    "species": "Meganium",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Typhlosion",
                    "species": "Typhlosion",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Feraligatr",
                    "species": "Feraligatr",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Furret",
                    "species": "Furret",
                    "ability": "Soundproof",
                    "moves": [],
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
fn heal_bell_cures_all_statuses_on_side() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Meganium"],
            ["switch", "player-1", "Meganium"],
            "move|mon:Miltank,player-2,1|name:Thunder Wave|target:Meganium,player-1,1",
            "status|mon:Meganium,player-1,1|status:Paralysis",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Typhlosion"],
            ["switch", "player-1", "Typhlosion"],
            "move|mon:Miltank,player-2,1|name:Sleep Powder|target:Typhlosion,player-1,1",
            "status|mon:Typhlosion,player-1,1|status:Sleep",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Feraligatr"],
            ["switch", "player-1", "Feraligatr"],
            "move|mon:Miltank,player-2,1|name:Toxic|target:Feraligatr,player-1,1",
            "status|mon:Feraligatr,player-1,1|status:Bad Poison",
            "split|side:0",
            "damage|mon:Feraligatr,player-1,1|from:status:Bad Poison|health:136/145",
            "damage|mon:Feraligatr,player-1,1|from:status:Bad Poison|health:94/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Miltank"],
            ["switch", "player-1", "Miltank"],
            "move|mon:Miltank,player-2,1|name:Toxic|target:Miltank,player-1,1",
            "status|mon:Miltank,player-1,1|status:Bad Poison",
            "split|side:0",
            "damage|mon:Miltank,player-1,1|from:status:Bad Poison|health:146/155",
            "damage|mon:Miltank,player-1,1|from:status:Bad Poison|health:95/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Heal Bell",
            "activate|move:Heal Bell|of:Miltank,player-1,1",
            "curestatus|mon:Miltank,player-1,1|status:Bad Poison",
            "curestatus|mon:Meganium,player-1|status:Paralysis",
            "curestatus|mon:Typhlosion,player-1|status:Sleep",
            "curestatus|mon:Feraligatr,player-1|status:Bad Poison",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn soundproof_ignores_heal_bell() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Furret"],
            ["switch", "player-1", "Furret"],
            "move|mon:Miltank,player-2,1|name:Thunder Wave|target:Furret,player-1,1",
            "status|mon:Furret,player-1,1|status:Paralysis",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Miltank"],
            ["switch", "player-1", "Miltank"],
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Heal Bell|noanim",
            "activate|move:Heal Bell|of:Miltank,player-1,1",
            "fail|mon:Miltank,player-1,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
