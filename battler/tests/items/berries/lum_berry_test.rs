use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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

fn crobat() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Crobat",
                    "species": "Crobat",
                    "ability": "No Ability",
                    "moves": [
                        "Confuse Ray",
                        "Thunder Wave"
                    ],
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
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Min)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn lum_berry_heals_status_and_confusion() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, crobat().unwrap(), crobat().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item lumberry,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:player-1|name:Lum Berry|target:Crobat,player-1,1",
            "curestatus|mon:Crobat,player-1,1|status:Paralysis|from:item:Lum Berry",
            "end|mon:Crobat,player-1,1|condition:Confusion",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 3, &expected_logs);
}

#[test]
fn lum_berry_eaten_on_status_set() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = crobat().unwrap();
    team.members[0].item = Some("Lum Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, crobat().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Crobat,player-2,1|name:Thunder Wave|target:Crobat,player-1,1",
            "status|mon:Crobat,player-1,1|status:Paralysis",
            "itemend|mon:Crobat,player-1,1|item:Lum Berry|eat",
            "curestatus|mon:Crobat,player-1,1|status:Paralysis|from:item:Lum Berry",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn lum_berry_eaten_on_confusion_added() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = crobat().unwrap();
    team.members[0].item = Some("Lum Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, crobat().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Crobat,player-2,1|name:Confuse Ray|target:Crobat,player-1,1",
            "start|mon:Crobat,player-1,1|condition:Confusion",
            "itemend|mon:Crobat,player-1,1|item:Lum Berry|eat",
            "end|mon:Crobat,player-1,1|condition:Confusion",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
