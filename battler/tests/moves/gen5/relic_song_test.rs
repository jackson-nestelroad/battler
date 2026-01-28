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

fn meloetta() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Meloetta",
                    "species": "Meloetta",
                    "ability": "No Ability",
                    "moves": [
                        "Relic Song"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn smeargle() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Smeargle",
                    "species": "Smeargle",
                    "ability": "No Ability",
                    "moves": [
                        "Relic Song"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(0)
        .with_pass_allowed(true)
        .with_team_validation(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn relic_song_transforms_meloetta() {
    let mut battle = make_battle(meloetta().unwrap(), meloetta().unwrap()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Meloetta,player-1,1|name:Relic Song",
            "split|side:1",
            "damage|mon:Meloetta,player-2,1|health:111/160",
            "damage|mon:Meloetta,player-2,1|health:70/100",
            "formechange|mon:Meloetta,player-1,1|species:Meloetta-Pirouette|from:move:Relic Song",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Meloetta,player-1,1|name:Relic Song",
            "split|side:1",
            "damage|mon:Meloetta,player-2,1|health:81/160",
            "damage|mon:Meloetta,player-2,1|health:51/100",
            "formechange|mon:Meloetta,player-1,1|species:Meloetta|from:move:Relic Song",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn relic_song_transformation_blocked_by_sheer_force() {
    let mut team = meloetta().unwrap();
    team.members[0].ability = "Sheer Force".to_owned();
    let mut battle = make_battle(team, meloetta().unwrap()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Meloetta,player-1,1|name:Relic Song",
            "split|side:1",
            "damage|mon:Meloetta,player-2,1|health:97/160",
            "damage|mon:Meloetta,player-2,1|health:61/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn relic_song_does_not_transform_non_meloetta() {
    let mut battle = make_battle(smeargle().unwrap(), smeargle().unwrap()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Smeargle,player-1,1|name:Relic Song",
            "split|side:1",
            "damage|mon:Smeargle,player-2,1|health:90/115",
            "damage|mon:Smeargle,player-2,1|health:79/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
