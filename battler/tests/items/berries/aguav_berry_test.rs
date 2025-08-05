use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    Nature,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn blaziken() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blaziken",
                    "species": "Blaziken",
                    "ability": "Blaze",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn swampert() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Swampert",
                    "species": "Swampert",
                    "ability": "Torrent",
                    "moves": [
                        "Surf",
                        "Water Gun",
                        "Pound"
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
fn aguav_berry_heals_one_third_hp() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = blaziken().unwrap();
    team.members[0].item = Some("Aguav Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, swampert().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swampert,player-2,1|name:Surf",
            "supereffective|mon:Blaziken,player-1,1",
            "split|side:0",
            "damage|mon:Blaziken,player-1,1|health:18/140",
            "damage|mon:Blaziken,player-1,1|health:13/100",
            "itemend|mon:Blaziken,player-1,1|item:Aguav Berry|eat",
            "split|side:0",
            "heal|mon:Blaziken,player-1,1|from:item:Aguav Berry|health:64/140",
            "heal|mon:Blaziken,player-1,1|from:item:Aguav Berry|health:46/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn aguav_berry_causes_confusion_to_special_defense_dropping_natures() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = blaziken().unwrap();
    team.members[0].item = Some("Aguav Berry".to_owned());
    team.members[0].nature = Nature::Sassy;
    team.members[0].true_nature = Some(Nature::Naughty);
    let mut battle = make_battle(&data, 0, team, swampert().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swampert,player-2,1|name:Surf",
            "supereffective|mon:Blaziken,player-1,1",
            "split|side:0",
            "damage|mon:Blaziken,player-1,1|health:26/140",
            "damage|mon:Blaziken,player-1,1|health:19/100",
            "itemend|mon:Blaziken,player-1,1|item:Aguav Berry|eat",
            "split|side:0",
            "heal|mon:Blaziken,player-1,1|from:item:Aguav Berry|health:72/140",
            "heal|mon:Blaziken,player-1,1|from:item:Aguav Berry|health:52/100",
            "start|mon:Blaziken,player-1,1|condition:Confusion",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gluttony_eats_aguav_berry_early() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = blaziken().unwrap();
    team.members[0].item = Some("Aguav Berry".to_owned());
    team.members[0].ability = "Gluttony".to_owned();
    let mut battle = make_battle(&data, 0, team, swampert().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swampert,player-2,1|name:Pound|target:Blaziken,player-1,1",
            "split|side:0",
            "damage|mon:Blaziken,player-1,1|health:61/140",
            "damage|mon:Blaziken,player-1,1|health:44/100",
            "itemend|mon:Blaziken,player-1,1|item:Aguav Berry|eat",
            "split|side:0",
            "heal|mon:Blaziken,player-1,1|from:item:Aguav Berry|health:107/140",
            "heal|mon:Blaziken,player-1,1|from:item:Aguav Berry|health:77/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}

#[test]
fn aguav_berry_can_be_used_from_bag() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = blaziken().unwrap();
    team.members[0].nature = Nature::Naughty;
    let mut battle = make_battle(&data, 0, team, swampert().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item aguavberry,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:player-1|name:Aguav Berry|target:Blaziken,player-1,1",
            "split|side:0",
            "heal|mon:Blaziken,player-1,1|from:item:Aguav Berry|health:124/140",
            "heal|mon:Blaziken,player-1,1|from:item:Aguav Berry|health:89/100",
            "start|mon:Blaziken,player-1,1|condition:Confusion",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
