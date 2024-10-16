use battler::{
    battle::{
        BattleType,
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

fn grumpig() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Grumpig",
                    "species": "Grumpig",
                    "ability": "No Ability",
                    "moves": [
                        "Ice Beam"
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
fn thick_fat_reduces_attack_power_of_ice_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = grumpig().unwrap();
    player.members[0].ability = "Thick Fat".to_owned();
    let mut battle = make_battle(&data, 0, player, grumpig().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Grumpig,player-1,1|name:Ice Beam|target:Grumpig,player-2,1",
            "split|side:1",
            "damage|mon:Grumpig,player-2,1|health:108/140",
            "damage|mon:Grumpig,player-2,1|health:78/100",
            "move|mon:Grumpig,player-2,1|name:Ice Beam|target:Grumpig,player-1,1",
            "split|side:0",
            "damage|mon:Grumpig,player-1,1|health:124/140",
            "damage|mon:Grumpig,player-1,1|health:89/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
