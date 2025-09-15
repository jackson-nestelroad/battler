use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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

fn snivy() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Snivy",
                    "species": "Snivy",
                    "ability": "Overgrow",
                    "moves": [
                        "Leaf Blade",
                        "Substitute"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn tepig() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Tepig",
                    "species": "Tepig",
                    "ability": "Blaze",
                    "moves": [
                        "Air Slash",
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

fn oshawott() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Oshawott",
                    "species": "Oshawott",
                    "ability": "Torrent",
                    "moves": [
                        "Surf"
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
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Min)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn occa_berry_reduces_fire_type_super_effective_damage() {
    let mut team = snivy().unwrap();
    team.members[0].item = Some("Occa Berry".to_owned());
    let mut battle = make_battle(0, team, tepig().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Tepig,player-2,1|name:Air Slash|target:Snivy,player-1,1",
            "supereffective|mon:Snivy,player-1,1",
            "split|side:0",
            "damage|mon:Snivy,player-1,1|health:57/105",
            "damage|mon:Snivy,player-1,1|health:55/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Tepig,player-2,1|name:Flamethrower|target:Snivy,player-1,1",
            "supereffective|mon:Snivy,player-1,1",
            "itemend|mon:Snivy,player-1,1|item:Occa Berry|eat",
            "activate|mon:Snivy,player-1,1|item:Occa Berry|weaken",
            "split|side:0",
            "damage|mon:Snivy,player-1,1|health:14/105",
            "damage|mon:Snivy,player-1,1|health:14/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn occa_berry_does_not_activate_on_substitute() {
    let mut team = snivy().unwrap();
    team.members[0].item = Some("Occa Berry".to_owned());
    let mut battle = make_battle(100, team, tepig().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snivy,player-1,1|name:Substitute|target:Snivy,player-1,1",
            "start|mon:Snivy,player-1,1|move:Substitute",
            "split|side:0",
            "damage|mon:Snivy,player-1,1|health:79/105",
            "damage|mon:Snivy,player-1,1|health:76/100",
            "move|mon:Tepig,player-2,1|name:Flamethrower|target:Snivy,player-1,1",
            "supereffective|mon:Snivy,player-1,1",
            "end|mon:Snivy,player-1,1|move:Substitute",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Tepig,player-2,1|name:Flamethrower|target:Snivy,player-1,1",
            "supereffective|mon:Snivy,player-1,1",
            "itemend|mon:Snivy,player-1,1|item:Occa Berry|eat",
            "activate|mon:Snivy,player-1,1|item:Occa Berry|weaken",
            "split|side:0",
            "damage|mon:Snivy,player-1,1|health:36/105",
            "damage|mon:Snivy,player-1,1|health:35/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn passho_berry_reduces_water_type_super_effective_damage() {
    let mut team = tepig().unwrap();
    team.members[0].item = Some("Passho Berry".to_owned());
    let mut battle = make_battle(0, team, oshawott().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oshawott,player-2,1|name:Surf",
            "supereffective|mon:Tepig,player-1,1",
            "itemend|mon:Tepig,player-1,1|item:Passho Berry|eat",
            "activate|mon:Tepig,player-1,1|item:Passho Berry|weaken",
            "split|side:0",
            "damage|mon:Tepig,player-1,1|health:56/125",
            "damage|mon:Tepig,player-1,1|health:45/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn enigma_berry_reduces_super_effective_damage() {
    let mut team = oshawott().unwrap();
    team.members[0].item = Some("Enigma Berry".to_owned());
    let mut battle = make_battle(0, team, snivy().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snivy,player-2,1|name:Leaf Blade|target:Oshawott,player-1,1",
            "supereffective|mon:Oshawott,player-1,1",
            "itemend|mon:Oshawott,player-1,1|item:Enigma Berry|eat",
            "activate|mon:Oshawott,player-1,1|item:Enigma Berry|weaken",
            "split|side:0",
            "damage|mon:Oshawott,player-1,1|health:64/115",
            "damage|mon:Oshawott,player-1,1|health:56/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
