use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    Request,
    TeamData,
    Type,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
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
                        "Thunderbolt",
                        "Water Gun"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn eevee() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Eevee",
                    "species": "Eevee",
                    "ability": "No Ability",
                    "moves": [
                        "Earthquake",
                        "Thunderbolt",
                        "Tackle",
                        "Mach Punch"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "No Ability",
                    "moves": [],
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
        .with_terastallization(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn one_mon_can_terastallize() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = pikachu().unwrap();
    team.members[0].tera_type = Some(Type::Flying);
    let mut battle = make_battle(&data, 0, team, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert!(request.active[0].can_terastallize);
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert!(!request.active[0].can_terastallize);
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Pikachu cannot terastallize");
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Pikachu,player-1,1|type:Flying",
            "move|mon:Pikachu,player-1,1|name:Thunderbolt|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:66/115",
            "damage|mon:Eevee,player-2,1|health:58/100",
            "move|mon:Eevee,player-2,1|name:Earthquake|noanim",
            "immune|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Eevee,player-2,1|name:Thunderbolt|target:Pikachu,player-1,1",
            "supereffective|mon:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:19/95",
            "damage|mon:Pikachu,player-1,1|health:20/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn terastallization_preserved_on_switch() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, pikachu().unwrap(), eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Pikachu,player-1,1|type:Electric",
            "move|mon:Pikachu,player-1,1|name:Thunderbolt|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:49/115",
            "damage|mon:Eevee,player-2,1|health:43/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Bulbasaur"],
            ["switch", "player-1", "Bulbasaur"],
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:0",
            "switch|player:player-1|position:1|name:Pikachu|health:95/95|tera:Electric|species:Pikachu|level:50|gender:U",
            "switch|player:player-1|position:1|name:Pikachu|health:100/100|tera:Electric|species:Pikachu|level:50|gender:U",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn terastallization_ends_on_faint() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, pikachu().unwrap(), eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item revive,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert!(!request.active[0].can_terastallize);
    });

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Pikachu,player-1,1|type:Electric",
            "move|mon:Pikachu,player-1,1|name:Thunderbolt|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:49/115",
            "damage|mon:Eevee,player-2,1|health:43/100",
            "move|mon:Eevee,player-2,1|name:Earthquake",
            "supereffective|mon:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:0",
            "damage|mon:Pikachu,player-1,1|health:0",
            "faint|mon:Pikachu,player-1,1",
            "reverttera|mon:Pikachu,player-1,1",
            "residual",
            ["time"],
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "turn|turn:2",
            ["time"],
            "useitem|player:player-1|name:Revive|target:Pikachu,player-1",
            "revive|mon:Pikachu,player-1|from:item:Revive",
            "split|side:0",
            "sethp|mon:Pikachu,player-1|health:47/95",
            "sethp|mon:Pikachu,player-1|health:50/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:0",
            "switch|player:player-1|position:1|name:Pikachu|health:47/95|species:Pikachu|level:50|gender:U",
            "switch|player:player-1|position:1|name:Pikachu|health:50/100|species:Pikachu|level:50|gender:U",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn terastallization_gives_stab_for_tera_type() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = pikachu().unwrap();
    team.members[0].tera_type = Some(Type::Water);
    let mut battle = make_battle(&data, 0, team, pikachu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Pikachu,player-1,1|type:Water",
            "move|mon:Pikachu,player-1,1|name:Water Gun|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:67/95",
            "damage|mon:Pikachu,player-2,1|health:71/100",
            "move|mon:Pikachu,player-2,1|name:Water Gun|target:Pikachu,player-1,1",
            "resisted|mon:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:86/95",
            "damage|mon:Pikachu,player-1,1|health:91/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn terastallization_boosts_stab_for_tera_type_if_mon_has_original_type() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, eevee().unwrap(), eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Eevee,player-1,1|type:Normal",
            "move|mon:Eevee,player-1,1|name:Tackle|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:73/115",
            "damage|mon:Eevee,player-2,1|health:64/100",
            "move|mon:Eevee,player-2,1|name:Tackle|target:Eevee,player-1,1",
            "split|side:0",
            "damage|mon:Eevee,player-1,1|health:84/115",
            "damage|mon:Eevee,player-1,1|health:74/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn terastallization_prevents_setting_types() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = pikachu().unwrap();
    team.members[0].ability = "Color Change".to_owned();
    let mut battle = make_battle(&data, 0, team, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eevee,player-2,1|name:Tackle|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:58/95",
            "damage|mon:Pikachu,player-1,1|health:62/100",
            "typechange|mon:Pikachu,player-1,1|types:Normal",
            "residual",
            "turn|turn:2",
            ["time"],
            "tera|mon:Pikachu,player-1,1|type:Electric",
            "move|mon:Eevee,player-2,1|name:Mach Punch|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:33/95",
            "damage|mon:Pikachu,player-1,1|health:35/100",
            "move|mon:Pikachu,player-1,1|name:Thunderbolt|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:66/115",
            "damage|mon:Eevee,player-2,1|health:58/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
