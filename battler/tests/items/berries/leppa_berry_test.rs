use battler::{
    battle::{
        BattleType,
        CoreBattleEngineRandomizeBaseDamage,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    error::{
        Error,
        WrapResultError,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn smeargle() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Smeargle",
                    "species": "Smeargle",
                    "ability": "No Ability",
                    "moves": [
                        "Sketch",
                        "Draco Meteor"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn blissey() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blissey",
                    "species": "Blissey",
                    "ability": "No Ability",
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

fn make_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
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
fn leppa_berry_restores_move_with_zero_pp() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = smeargle().unwrap();
    team.members[0].item = Some("Leppa Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, blissey().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Smeargle,player-1,1|name:Draco Meteor|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:265/315",
            "damage|mon:Blissey,player-2,1|health:85/100",
            "itemend|mon:Smeargle,player-1,1|item:Leppa Berry|eat",
            "restorepp|mon:Smeargle,player-1,1|move:Draco Meteor|by:5|from:item:Leppa Berry",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 5, &expected_logs);
}

#[test]
fn leppa_berry_fails_on_move_with_full_pp() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, smeargle().unwrap(), blissey().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item leppaberry,-1,sketch"),
        Err(err) => assert_eq!(err.full_description(), "cannot use item: Leppa Berry cannot be used on Smeargle")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item leppaberry,-1,draco meteor"),
        Err(err) => assert_eq!(err.full_description(), "cannot use item: Leppa Berry cannot be used on Smeargle")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item leppaberry,-1,Water Gun"),
        Err(err) => assert_eq!(err.full_description(), "cannot use item: Smeargle does not have the given move")
    );
}

#[test]
fn leppa_berry_restores_pp_when_used_from_bag() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, smeargle().unwrap(), blissey().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item leppaberry,-1,sketch"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item leppaberry,-1,Draco Meteor"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Smeargle,player-1,1|name:Sketch|noanim",
            "fail|mon:Smeargle,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "useitem|player:player-1|name:Leppa Berry|target:Smeargle,player-1,1",
            "restorepp|mon:Smeargle,player-1,1|move:Sketch|by:1|from:item:Leppa Berry",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Smeargle,player-1,1|name:Draco Meteor|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:305/315",
            "damage|mon:Blissey,player-2,1|health:97/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "useitem|player:player-1|name:Leppa Berry|target:Smeargle,player-1,1",
            "restorepp|mon:Smeargle,player-1,1|move:Draco Meteor|by:1|from:item:Leppa Berry",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
