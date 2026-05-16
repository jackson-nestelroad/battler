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
                    "name": "Greninja",
                    "species": "Greninja-Battle-Bond",
                    "ability": "Battle Bond",
                    "moves": [
                        "Water Shuriken",
                        "Thunderbolt"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Delphox",
                    "species": "Delphox",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Chesnaught",
                    "species": "Chesnaught",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100,
                    "persistent_battle_data": {
                        "hp": 1
                    }
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
fn battle_bond_transforms_into_ash_greninja_and_gives_boost() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Chesnaught"],
            ["switch", "player-2", "Chesnaught"],
            "move|mon:Greninja,player-1,1|name:Water Shuriken|target:Chesnaught,player-2,1",
            "resisted|mon:Chesnaught,player-2,1",
            "split|side:1",
            "damage|mon:Chesnaught,player-2,1|health:0",
            "damage|mon:Chesnaught,player-2,1|health:0",
            "faint|mon:Chesnaught,player-2,1",
            "activate|mon:Greninja,player-1,1|ability:Battle Bond",
            "boost|mon:Greninja,player-1,1|stat:atk|by:1|from:ability:Battle Bond",
            "boost|mon:Greninja,player-1,1|stat:spa|by:1|from:ability:Battle Bond",
            "boost|mon:Greninja,player-1,1|stat:spe|by:1|from:ability:Battle Bond",
            "split|side:0",
            ["specieschange", "player-1", "species:Greninja-Ash"],
            ["specieschange", "player-1", "species:Greninja-Ash"],
            "formechange|mon:Greninja,player-1,1|species:Greninja-Ash|from:ability:Battle Bond",
            "hitcount|hits:1",
            "residual",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Delphox"],
            ["switch", "player-2", "Delphox"],
            "turn|turn:2",
            "continue",
            "move|mon:Greninja,player-1,1|name:Water Shuriken|target:Delphox,player-2,1",
            "supereffective|mon:Delphox,player-2,1",
            "split|side:1",
            "damage|mon:Delphox,player-2,1|health:152/260",
            "damage|mon:Delphox,player-2,1|health:59/100",
            "animatemove|mon:Greninja,player-1,1|name:Water Shuriken|target:Delphox,player-2,1",
            "supereffective|mon:Delphox,player-2,1",
            "split|side:1",
            "damage|mon:Delphox,player-2,1|health:50/260",
            "damage|mon:Delphox,player-2,1|health:20/100",
            "animatemove|mon:Greninja,player-1,1|name:Water Shuriken|target:Delphox,player-2,1",
            "supereffective|mon:Delphox,player-2,1",
            "split|side:1",
            "damage|mon:Delphox,player-2,1|health:0",
            "damage|mon:Delphox,player-2,1|health:0",
            "faint|mon:Delphox,player-2,1",
            "hitcount|hits:3",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ash_greninja_reverts_on_faint() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Greninja,player-2,1|name:Thunderbolt|target:Greninja,player-1,1",
            "supereffective|mon:Greninja,player-1,1",
            "split|side:0",
            "damage|mon:Greninja,player-1,1|health:0",
            "damage|mon:Greninja,player-1,1|health:0",
            "faint|mon:Greninja,player-1,1",
            "split|side:0",
            ["specieschange", "player-1", "species:Greninja-Battle-Bond"],
            ["specieschange", "player-1", "species:Greninja-Battle-Bond"],
            "formechange|mon:Greninja,player-1,1|species:Greninja-Battle-Bond|from:Faint",
            "activate|mon:Greninja,player-2,1|ability:Battle Bond",
            "boost|mon:Greninja,player-2,1|stat:atk|by:1|from:ability:Battle Bond",
            "boost|mon:Greninja,player-2,1|stat:spa|by:1|from:ability:Battle Bond",
            "boost|mon:Greninja,player-2,1|stat:spe|by:1|from:ability:Battle Bond",
            "split|side:1",
            ["specieschange", "player-2", "species:Greninja-Ash"],
            ["specieschange", "player-2", "species:Greninja-Ash"],
            "formechange|mon:Greninja,player-2,1|species:Greninja-Ash|from:ability:Battle Bond",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 3, &expected_logs);
}
