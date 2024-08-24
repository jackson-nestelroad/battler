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

fn hitmonlee() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Hitmonlee",
                    "species": "Hitmonlee",
                    "ability": "No Ability",
                    "moves": [
                        "Jump Kick",
                        "Double Team"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(69384300492104)
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
fn hit_jump_damages_user_when_misses() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, hitmonlee().unwrap(), hitmonlee().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hitmonlee,player-2,1|name:Double Team|target:Hitmonlee,player-2,1",
            "boost|mon:Hitmonlee,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Hitmonlee,player-2,1|name:Double Team|target:Hitmonlee,player-2,1",
            "boost|mon:Hitmonlee,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Hitmonlee,player-1,1|name:Jump Kick|noanim",
            "miss|mon:Hitmonlee,player-2,1",
            "split|side:0",
            "damage|mon:Hitmonlee,player-1,1|from:move:Jump Kick|health:55/110",
            "damage|mon:Hitmonlee,player-1,1|from:move:Jump Kick|health:50/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
