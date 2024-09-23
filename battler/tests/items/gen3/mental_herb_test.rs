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
    mons::Gender,
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn mudkip() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Mudkip",
                    "species": "Mudkip",
                    "ability": "Torrent",
                    "moves": [
                        "Attract",
                        "Heal Block"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Mental Herb"
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
fn mental_herb_removes_heal_block() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = mudkip().unwrap();
    team.members[0].gender = Gender::Male;
    let mut opponent = mudkip().unwrap();
    opponent.members[0].gender = Gender::Female;
    let mut battle = make_battle(&data, 0, team, opponent).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Mudkip,player-2,1|name:Heal Block",
            "start|mon:Mudkip,player-1,1|move:Heal Block",
            "itemend|mon:Mudkip,player-1,1|item:Mental Herb",
            "end|mon:Mudkip,player-1,1|move:Heal Block",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}