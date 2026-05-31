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

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Darmanitan",
                    "species": "Darmanitan-Galar",
                    "ability": "Gorilla Tactics",
                    "moves": [
                        "Tackle",
                        "Splash",
                        "Gastro Acid",
                        "Knock Off"
                    ],
                    "nature": "Hardy",
                    "level": 100
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn gorilla_tactics_acts_as_choice_band() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Darmanitan's Splash is disabled")
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Darmanitan,player-1,1|name:Tackle|target:Darmanitan,player-2,1",
            "split|side:1",
            "damage|mon:Darmanitan,player-2,1|health:194/320",
            "damage|mon:Darmanitan,player-2,1|health:61/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gorilla_tactics_stacks_with_choice_band() {
    let mut team_1 = team().unwrap();
    team_1.members[0].item = Some("Choice Band".to_owned());
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Darmanitan's Tackle is disabled")
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Darmanitan's Tackle is disabled")
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Darmanitan,player-1,1|name:Splash|target:Darmanitan,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Darmanitan,player-1,1|name:Splash|target:Darmanitan,player-1,1",
            "activate|move:Splash",
            "move|mon:Darmanitan,player-2,1|name:Knock Off|target:Darmanitan,player-1,1",
            "split|side:0",
            "damage|mon:Darmanitan,player-1,1|health:117/320",
            "damage|mon:Darmanitan,player-1,1|health:37/100",
            "itemend|mon:Darmanitan,player-1,1|item:Choice Band|from:move:Knock Off|of:Darmanitan,player-2,1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Darmanitan,player-1,1|name:Splash|target:Darmanitan,player-1,1",
            "activate|move:Splash",
            "move|mon:Darmanitan,player-2,1|name:Gastro Acid|target:Darmanitan,player-1,1",
            "abilityend|mon:Darmanitan,player-1,1|ability:Gorilla Tactics|from:move:Gastro Acid|of:Darmanitan,player-2,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
