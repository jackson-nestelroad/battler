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

fn team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "No Ability",
                    "moves": [
                        "Cotton Spore",
                        "Tail Whip",
                        "Sand Attack",
                        "Double Team",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "No Ability",
                    "moves": [
                        "Mist",
                        "Aromatic Mist",
                        "Superpower"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
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
        .with_battle_type(BattleType::Doubles)
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
fn mist_protects_user_side_from_stat_drops() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_battle(&data, 5456456324231453212, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1;move 1,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2,1;move 2,2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 4,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 4,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blastoise,player-1,2|name:Mist",
            "sidestart|side:0|move:Mist",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Cotton Spore|spread:Charizard,player-2,1;Blastoise,player-2,2",
            "unboost|mon:Charizard,player-2,1|stat:spe|by:2",
            "unboost|mon:Blastoise,player-2,2|stat:spe|by:2",
            "move|mon:Charizard,player-2,1|name:Cotton Spore|noanim",
            "activate|mon:Charizard,player-1,1|move:Mist",
            "activate|mon:Blastoise,player-1,2|move:Mist",
            "fail|mon:Charizard,player-2,1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Tail Whip|spread:Charizard,player-2,1;Blastoise,player-2,2",
            "unboost|mon:Charizard,player-2,1|stat:def|by:1",
            "unboost|mon:Blastoise,player-2,2|stat:def|by:1",
            "move|mon:Blastoise,player-1,2|name:Aromatic Mist|target:Charizard,player-1,1",
            "boost|mon:Charizard,player-1,1|stat:spd|by:1",
            "move|mon:Charizard,player-2,1|name:Tail Whip|noanim",
            "activate|mon:Charizard,player-1,1|move:Mist",
            "activate|mon:Blastoise,player-1,2|move:Mist",
            "fail|mon:Charizard,player-2,1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Sand Attack|target:Charizard,player-2,1",
            "unboost|mon:Charizard,player-2,1|stat:acc|by:1",
            "move|mon:Blastoise,player-1,2|name:Superpower|target:Blastoise,player-2,2",
            "split|side:1",
            "damage|mon:Blastoise,player-2,2|health:76/139",
            "damage|mon:Blastoise,player-2,2|health:55/100",
            "unboost|mon:Blastoise,player-1,2|stat:atk|by:1",
            "unboost|mon:Blastoise,player-1,2|stat:def|by:1",
            "move|mon:Charizard,player-2,1|name:Double Team|target:Charizard,player-2,1",
            "boost|mon:Charizard,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Tackle|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:110/138",
            "damage|mon:Charizard,player-2,1|health:80/100",
            "move|mon:Charizard,player-2,1|name:Tackle|target:Charizard,player-1,1",
            "split|side:0",
            "damage|mon:Charizard,player-1,1|health:121/138",
            "damage|mon:Charizard,player-1,1|health:88/100",
            "sideend|side:0|move:Mist",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Tail Whip|spread:Charizard,player-2,1;Blastoise,player-2,2",
            "unboost|mon:Charizard,player-2,1|stat:def|by:1",
            "unboost|mon:Blastoise,player-2,2|stat:def|by:1",
            "move|mon:Charizard,player-2,1|name:Tail Whip|spread:Blastoise,player-1,2",
            "miss|mon:Charizard,player-1,1",
            "unboost|mon:Blastoise,player-1,2|stat:def|by:1",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
