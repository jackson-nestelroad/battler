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
                    "name": "Samurott",
                    "species": "Samurott",
                    "ability": "No Ability",
                    "moves": [
                        "Water Gun",
                        "Gastro Acid"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Zebstrika",
                    "species": "Zebstrika",
                    "ability": "No Ability",
                    "moves": [
                        "Discharge"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Shedinja",
                    "species": "Shedinja",
                    "ability": "Wonder Guard",
                    "moves": [
                        "Bug Buzz"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Zoroark",
                    "species": "Zoroark",
                    "ability": "Illusion",
                    "item": "Life Orb",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Weezing",
                    "species": "Weezing",
                    "ability": "Neutralizing Gas",
                    "moves": [
                        "Toxic"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Entei",
                    "species": "Entei",
                    "ability": "No Ability",
                    "moves": [
                        "Flamethrower"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn zoroark() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Zoroark",
                    "species": "Zoroark",
                    "ability": "Illusion",
                    "item": "Life Orb",
                    "moves": [
                        "Tackle"
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
fn illusion_casts_illusion_until_damaged_by_move() {
    let mut battle = make_battle(
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            "switch|player:player-2|position:1|name:Entei|health:120/120|species:Entei|level:50|gender:U",
            "switch|player:player-2|position:1|name:Entei|health:100/100|species:Entei|level:50|gender:U",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Entei,player-2,1|name:Tackle|target:Samurott,player-1,1",
            "split|side:0",
            "damage|mon:Samurott,player-1,1|health:127/155",
            "damage|mon:Samurott,player-1,1|health:82/100",
            "split|side:1",
            "damage|mon:Entei,player-2,1|from:item:Life Orb|health:108/120",
            "damage|mon:Entei,player-2,1|from:item:Life Orb|health:90/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Entei,player-2,1|name:Tackle|target:Samurott,player-1,1",
            "split|side:0",
            "damage|mon:Samurott,player-1,1|health:101/155",
            "damage|mon:Samurott,player-1,1|health:66/100",
            "split|side:1",
            "damage|mon:Entei,player-2,1|from:item:Life Orb|health:96/120",
            "damage|mon:Entei,player-2,1|from:item:Life Orb|health:80/100",
            "move|mon:Samurott,player-1,1|name:Water Gun|target:Entei,player-2,1",
            "split|side:1",
            "damage|mon:Entei,player-2,1|health:56/120",
            "damage|mon:Entei,player-2,1|health:47/100",
            "split|side:1",
            "replace|player:player-2|position:1|name:Zoroark|health:56/120|species:Zoroark|level:50|gender:U",
            "replace|player:player-2|position:1|name:Zoroark|health:47/100|species:Zoroark|level:50|gender:U",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn illusion_target_influenced_by_switches() {
    let mut battle = make_battle(
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            "switch|player:player-2|position:1|name:Shedinja|health:1/1|species:Shedinja|level:50|gender:U",
            "switch|player:player-2|position:1|name:Shedinja|health:100/100|species:Shedinja|level:50|gender:U",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:1|name:Entei|health:175/175|species:Entei|level:50|gender:U",
            "switch|player:player-2|position:1|name:Entei|health:100/100|species:Entei|level:50|gender:U",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:1|name:Shedinja|health:120/120|species:Shedinja|level:50|gender:U",
            "switch|player:player-2|position:1|name:Shedinja|health:100/100|species:Shedinja|level:50|gender:U",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Samurott,player-1,1|name:Water Gun|target:Shedinja,player-2,1",
            "split|side:1",
            "damage|mon:Shedinja,player-2,1|health:74/120",
            "damage|mon:Shedinja,player-2,1|health:62/100",
            "split|side:1",
            "replace|player:player-2|position:1|name:Zoroark|health:74/120|species:Zoroark|level:50|gender:U",
            "replace|player:player-2|position:1|name:Zoroark|health:62/100|species:Zoroark|level:50|gender:U",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn illusion_does_not_activate_if_no_other_team_members() {
    let mut battle = make_battle(
        0,
        zoroark().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Zoroark"],
            ["switch", "player-1", "Zoroark"],
            "split|side:1",
            ["switch", "player-2", "Samurott"],
            ["switch", "player-2", "Samurott"],
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn illusion_does_not_activate_if_no_other_unfainted_team_members() {
    let mut fainted_team = team().unwrap();
    fainted_team.members[0].persistent_battle_data.hp = Some(0);
    fainted_team.members[1].persistent_battle_data.hp = Some(0);
    fainted_team.members[4].persistent_battle_data.hp = Some(0);
    fainted_team.members[5].persistent_battle_data.hp = Some(0);
    let mut battle = make_battle(0, fainted_team, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            "switch|player:player-2|position:1|name:Entei|health:175/175|species:Entei|level:50|gender:U",
            "switch|player:player-2|position:1|name:Entei|health:100/100|species:Entei|level:50|gender:U",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Entei,player-2,1|name:Flamethrower|target:Shedinja,player-1,1",
            "supereffective|mon:Shedinja,player-1,1",
            "split|side:0",
            "damage|mon:Shedinja,player-1,1|health:0",
            "damage|mon:Shedinja,player-1,1|health:0",
            "faint|mon:Shedinja,player-1,1",
            "residual",
            ["time"],
            "split|side:0",
            "switch|player:player-1|position:1|name:Zoroark|health:120/120|species:Zoroark|level:50|gender:U",
            "switch|player:player-1|position:1|name:Zoroark|health:100/100|species:Zoroark|level:50|gender:U",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn illusion_ends_when_ability_is_suppressed() {
    let mut battle = make_battle(
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            "switch|player:player-2|position:1|name:Entei|health:120/120|species:Entei|level:50|gender:U",
            "switch|player:player-2|position:1|name:Entei|health:100/100|species:Entei|level:50|gender:U",
            "move|mon:Samurott,player-1,1|name:Gastro Acid|target:Entei,player-2,1",
            "abilityend|mon:Entei,player-2,1|ability:Illusion|from:move:Gastro Acid|of:Samurott,player-1,1",
            "split|side:1",
            "replace|player:player-2|position:1|name:Zoroark|health:120/120|species:Zoroark|level:50|gender:U",
            "replace|player:player-2|position:1|name:Zoroark|health:100/100|species:Zoroark|level:50|gender:U",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn illusion_ends_when_ability_is_suppressed_with_neutralizing_gass() {
    let mut battle = make_battle(
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            "switch|player:player-1|position:1|name:Weezing|health:125/125|species:Weezing|level:50|gender:U",
            "switch|player:player-1|position:1|name:Weezing|health:100/100|species:Weezing|level:50|gender:U",
            "ability|mon:Weezing,player-1,1|ability:Neutralizing Gas",
            "split|side:1",
            "switch|player:player-2|position:1|name:Zoroark|health:120/120|species:Zoroark|level:50|gender:U",
            "switch|player:player-2|position:1|name:Zoroark|health:100/100|species:Zoroark|level:50|gender:U",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
