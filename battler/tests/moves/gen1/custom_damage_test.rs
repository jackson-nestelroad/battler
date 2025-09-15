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
    assert_turn_logs_eq,
    static_local_data_store,
};

fn pikachu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Seismic Toss",
                        "Psywave",
                        "Super Fang",
                        "Low Kick",
                        "Calm Mind",
                        "Harden"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn mon_by_species(species: &str) -> Result<TeamData> {
    let mut team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "",
                    "species": "",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()?;
    team.members[0].name = species.to_owned();
    team.members[0].species = species.to_owned();
    Ok(team)
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_volatile_status_logs(true)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn seismic_toss_does_damage_equal_to_level() {
    let mut battle = make_battle(0, pikachu().unwrap(), pikachu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Seismic Toss|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:45/95",
            "damage|mon:Pikachu,player-2,1|health:48/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut team = pikachu().unwrap();
    team.members[0].level = 75;
    let mut battle = make_battle(0, team, pikachu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Seismic Toss|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:20/95",
            "damage|mon:Pikachu,player-2,1|health:22/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Between 25 and 75 for level 50.
#[test]
fn psywave_applies_custom_damage_formula() {
    let mut battle = make_battle(777294920103, pikachu().unwrap(), pikachu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // These special boosts should do nothing.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Psywave|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:28/95",
            "damage|mon:Pikachu,player-2,1|health:30/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 4, &expected_logs);
}

#[test]
fn super_fang_does_half_hp_damge() {
    let mut battle = make_battle(0, pikachu().unwrap(), pikachu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // These defense boosts should do nothing.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Super Fang|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:48/95",
            "damage|mon:Pikachu,player-2,1|health:51/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 4, &expected_logs);

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Super Fang|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:24/95",
            "damage|mon:Pikachu,player-2,1|health:26/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 5, &expected_logs);
}

#[test]
fn low_kick_deals_damage_based_on_weight() {
    let mut battle =
        make_battle(0, pikachu().unwrap(), mon_by_species("Chespin").unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Low Kick|target:Chespin,player-2,1",
            "split|side:1",
            "damage|mon:Chespin,player-2,1|health:108/116",
            "damage|mon:Chespin,player-2,1|health:94/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 1, &expected_logs);

    let mut battle =
        make_battle(0, pikachu().unwrap(), mon_by_species("Turtwig").unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Low Kick|target:Turtwig,player-2,1",
            "split|side:1",
            "damage|mon:Turtwig,player-2,1|health:99/115",
            "damage|mon:Turtwig,player-2,1|health:87/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 1, &expected_logs);

    let mut battle =
        make_battle(0, pikachu().unwrap(), mon_by_species("Serperior").unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Low Kick|target:Serperior,player-2,1",
            "split|side:1",
            "damage|mon:Serperior,player-2,1|health:113/135",
            "damage|mon:Serperior,player-2,1|health:84/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 1, &expected_logs);

    let mut battle =
        make_battle(0, pikachu().unwrap(), mon_by_species("Wailord").unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Low Kick|target:Wailord,player-2,1",
            "split|side:1",
            "damage|mon:Wailord,player-2,1|health:167/230",
            "damage|mon:Wailord,player-2,1|health:73/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 1, &expected_logs);
}
