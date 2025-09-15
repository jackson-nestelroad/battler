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

fn squirtle() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Squirtle",
                    "species": "Squirtle",
                    "ability": "Torrent",
                    "moves": [
                        "Tackle",
                        "Aqua Jet"
                    ],
                    "nature": "Adamant",
                    "gender": "F",
                    "level": 40
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn pikachu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "Static",
                    "moves": [
                        "Quick Attack"
                    ],
                    "nature": "Bold",
                    "gender": "F",
                    "level": 40
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn test_battle_builder(team_1: TeamData, team_2: TeamData) -> TestBattleBuilder {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(0)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
}

fn make_battle(team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    test_battle_builder(team_1, team_2).build(static_local_data_store())
}

#[test]
fn stab_increases_damage() {
    // Tackle and Aqua Jet are both Physical moves with the same base damage, so STAB makes the
    // difference.
    let mut battle = make_battle(squirtle().unwrap(), pikachu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Quick Attack|target:Squirtle,player-1,1",
            "split|side:0",
            "damage|mon:Squirtle,player-1,1|health:73/85",
            "damage|mon:Squirtle,player-1,1|health:86/100",
            "move|mon:Squirtle,player-1,1|name:Tackle|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:62/78",
            "damage|mon:Pikachu,player-2,1|health:80/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-2,1|name:Quick Attack|target:Squirtle,player-1,1",
            "split|side:0",
            "damage|mon:Squirtle,player-1,1|health:61/85",
            "damage|mon:Squirtle,player-1,1|health:72/100",
            "move|mon:Squirtle,player-1,1|name:Aqua Jet|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:37/78",
            "damage|mon:Pikachu,player-2,1|health:48/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
