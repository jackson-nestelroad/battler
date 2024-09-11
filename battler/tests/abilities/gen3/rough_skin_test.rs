use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    common::{
        Error,
        WrapResultError,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn sharpedo() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Sharpedo",
                    "species": "Sharpedo",
                    "ability": "Rough Skin",
                    "moves": [
                        "Scratch",
                        "Bubble"
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn rough_skin_damages_attacker_on_contact() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, sharpedo().unwrap(), sharpedo().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Sharpedo,player-1,1|name:Scratch|target:Sharpedo,player-2,1",
            "split|side:1",
            "damage|mon:Sharpedo,player-2,1|health:82/130",
            "damage|mon:Sharpedo,player-2,1|health:64/100",
            "split|side:0",
            "damage|mon:Sharpedo,player-1,1|from:ability:Rough Skin|of:Sharpedo,player-2,1|health:114/130",
            "damage|mon:Sharpedo,player-1,1|from:ability:Rough Skin|of:Sharpedo,player-2,1|health:88/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Sharpedo,player-1,1|name:Bubble",
            "resisted|mon:Sharpedo,player-2,1",
            "split|side:1",
            "damage|mon:Sharpedo,player-2,1|health:55/130",
            "damage|mon:Sharpedo,player-2,1|health:43/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn protective_pads_protect_from_rough_skin() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = sharpedo().unwrap();
    player.members[0].item = Some("Protective Pads".to_owned());
    let mut battle = make_battle(&data, 0, player, sharpedo().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Sharpedo,player-1,1|name:Scratch|target:Sharpedo,player-2,1",
            "split|side:1",
            "damage|mon:Sharpedo,player-2,1|health:82/130",
            "damage|mon:Sharpedo,player-2,1|health:64/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
