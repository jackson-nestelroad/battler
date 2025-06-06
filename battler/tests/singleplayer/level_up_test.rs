use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    Request,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn level_5_gastly_and_gengar() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gastly",
                    "species": "Gastly",
                    "ability": "No Ability",
                    "moves": [
                        "Lick"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "experience": 140
                },
                {
                    "name": "Gengar",
                    "species": "Gengar",
                    "ability": "No Ability",
                    "moves": [
                        "Dark Pulse"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn level_100_blissey_and_pikachu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blissey",
                    "species": "Blissey",
                    "ability": "No Ability",
                    "moves": [
                        "Self-Destruct"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 100
                },
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Thunderbolt"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 5
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
) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_protagonist_to_side_1("player-1", "Red")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn massive_level_up_before_battle_ends() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        level_5_gastly_and_gengar().unwrap(),
        level_100_blissey_and_pikachu().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_eq!(
        battle.request_for_player("player-1").unwrap(),
        serde_json::from_str(
            r#"{
                "type": "learnmove",
                "can_learn_move": {
                    "team_position": 0,
                    "id": "confuseray",
                    "name": "Confuse Ray"
                }
            }"#
        )
        .unwrap()
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 4"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert_eq!(request.active.first().map(|mon| mon.moves.iter().map(|move_slot| move_slot.name.clone()).collect()), Some(vec![
            "Dark Pulse".to_owned(),
            "Shadow Ball".to_owned(),
            "Confuse Ray".to_owned(),
            "Night Shade".to_owned(),
        ]));
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blissey,player-2,1|name:Self-Destruct|noanim",
            "immune|mon:Gastly,player-1,1",
            "faint|mon:Blissey,player-2,1",
            "exp|mon:Gastly,player-1,1|exp:59290",
            "levelup|mon:Gastly,player-1,1|level:6|hp:19|atk:9|def:8|spa:17|spd:9|spe:14",
            "levelup|mon:Gastly,player-1,1|level:7|hp:21|atk:9|def:9|spa:19|spd:9|spe:16",
            "levelup|mon:Gastly,player-1,1|level:8|hp:22|atk:10|def:9|spa:21|spd:10|spe:17",
            "learnedmove|mon:Gastly,player-1,1|move:Mean Look",
            "levelup|mon:Gastly,player-1,1|level:9|hp:24|atk:11|def:10|spa:23|spd:11|spe:19",
            "levelup|mon:Gastly,player-1,1|level:10|hp:26|atk:12|def:11|spa:25|spd:12|spe:21",
            "levelup|mon:Gastly,player-1,1|level:11|hp:27|atk:12|def:11|spa:27|spd:12|spe:22",
            "levelup|mon:Gastly,player-1,1|level:12|hp:29|atk:13|def:12|spa:29|spd:13|spe:24",
            "learnedmove|mon:Gastly,player-1,1|move:Curse",
            "levelup|mon:Gastly,player-1,1|level:13|hp:30|atk:14|def:12|spa:31|spd:14|spe:25",
            "levelup|mon:Gastly,player-1,1|level:14|hp:32|atk:14|def:13|spa:33|spd:14|spe:27",
            "levelup|mon:Gastly,player-1,1|level:15|hp:34|atk:15|def:14|spa:35|spd:15|spe:29",
            "learnedmove|mon:Gastly,player-1,1|move:Night Shade",
            "levelup|mon:Gastly,player-1,1|level:16|hp:35|atk:16|def:14|spa:37|spd:16|spe:30",
            "levelup|mon:Gastly,player-1,1|level:17|hp:37|atk:16|def:15|spa:39|spd:16|spe:32",
            "levelup|mon:Gastly,player-1,1|level:18|hp:38|atk:17|def:15|spa:41|spd:17|spe:33",
            "levelup|mon:Gastly,player-1,1|level:19|hp:40|atk:18|def:16|spa:43|spd:18|spe:35",
            ["time"],
            "learnedmove|mon:Gastly,player-1,1|move:Confuse Ray|forgot:Curse",
            "levelup|mon:Gastly,player-1,1|level:20|hp:42|atk:19|def:17|spa:45|spd:19|spe:37",
            "levelup|mon:Gastly,player-1,1|level:21|hp:43|atk:19|def:17|spa:47|spd:19|spe:38",
            "levelup|mon:Gastly,player-1,1|level:22|hp:45|atk:20|def:18|spa:49|spd:20|spe:40",
            ["time"],
            "learnedmove|mon:Gastly,player-1,1|move:Sucker Punch|forgot:Lick",
            "levelup|mon:Gastly,player-1,1|level:23|hp:46|atk:21|def:18|spa:51|spd:21|spe:41",
            "levelup|mon:Gastly,player-1,1|level:24|hp:48|atk:21|def:19|spa:53|spd:21|spe:43",
            "levelup|mon:Gastly,player-1,1|level:25|hp:50|atk:22|def:20|spa:55|spd:22|spe:45",
            "levelup|mon:Gastly,player-1,1|level:26|hp:51|atk:23|def:20|spa:57|spd:23|spe:46",
            ["time"],
            "didnotlearnmove|mon:Gastly,player-1,1|move:Payback",
            "levelup|mon:Gastly,player-1,1|level:27|hp:53|atk:23|def:21|spa:59|spd:23|spe:48",
            "levelup|mon:Gastly,player-1,1|level:28|hp:54|atk:24|def:21|spa:61|spd:24|spe:49",
            "levelup|mon:Gastly,player-1,1|level:29|hp:56|atk:25|def:22|spa:63|spd:25|spe:51",
            ["time"],
            "learnedmove|mon:Gastly,player-1,1|move:Shadow Ball|forgot:Mean Look",
            "levelup|mon:Gastly,player-1,1|level:30|hp:58|atk:26|def:23|spa:65|spd:26|spe:53",
            "levelup|mon:Gastly,player-1,1|level:31|hp:59|atk:26|def:23|spa:67|spd:26|spe:54",
            "levelup|mon:Gastly,player-1,1|level:32|hp:61|atk:27|def:24|spa:69|spd:27|spe:56",
            "levelup|mon:Gastly,player-1,1|level:33|hp:62|atk:28|def:24|spa:71|spd:28|spe:57",
            ["time"],
            "didnotlearnmove|mon:Gastly,player-1,1|move:Dream Eater",
            "levelup|mon:Gastly,player-1,1|level:34|hp:64|atk:28|def:25|spa:73|spd:28|spe:59",
            "levelup|mon:Gastly,player-1,1|level:35|hp:66|atk:29|def:26|spa:75|spd:29|spe:61",
            "levelup|mon:Gastly,player-1,1|level:36|hp:67|atk:30|def:26|spa:77|spd:30|spe:62",
            ["time"],
            "learnedmove|mon:Gastly,player-1,1|move:Dark Pulse|forgot:Sucker Punch",
            "levelup|mon:Gastly,player-1,1|level:37|hp:69|atk:30|def:27|spa:79|spd:30|spe:64",
            "levelup|mon:Gastly,player-1,1|level:38|hp:70|atk:31|def:27|spa:81|spd:31|spe:65",
            "levelup|mon:Gastly,player-1,1|level:39|hp:72|atk:32|def:28|spa:83|spd:32|spe:67",
            "levelup|mon:Gastly,player-1,1|level:40|hp:74|atk:33|def:29|spa:85|spd:33|spe:69",
            ["time"],
            "didnotlearnmove|mon:Gastly,player-1,1|move:Destiny Bond",
            "residual",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Pikachu"],
            ["switch", "player-2", "Pikachu"],
            "turn|turn:2",
            ["time"],
            "move|mon:Gastly,player-1,1|name:Shadow Ball|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:0",
            "damage|mon:Pikachu,player-2,1|health:0",
            "faint|mon:Pikachu,player-2,1",
            "exp|mon:Gastly,player-1,1|exp:10",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn inactive_mon_levels_up_directly_to_level() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        level_5_gastly_and_gengar().unwrap(),
        level_100_blissey_and_pikachu().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "learnmove 4"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert_eq!(request.active.first().map(|mon| mon.moves.iter().map(|move_slot| move_slot.name.clone()).collect()), Some(vec![
            "Shadow Ball".to_owned(),
            "Dream Eater".to_owned(),
            "Confuse Ray".to_owned(),
            "Night Shade".to_owned(),
        ]));
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Gengar"],
            ["switch", "player-1", "Gengar"],
            "move|mon:Blissey,player-2,1|name:Self-Destruct|noanim",
            "immune|mon:Gengar,player-1,1",
            "faint|mon:Blissey,player-2,1",
            "exp|mon:Gastly,player-1|exp:59290",
            "levelup|mon:Gastly,player-1|level:40|hp:74|atk:33|def:29|spa:85|spd:33|spe:69",
            "learnedmove|mon:Gastly,player-1|move:Mean Look",
            "learnedmove|mon:Gastly,player-1|move:Curse",
            "learnedmove|mon:Gastly,player-1|move:Night Shade",
            ["time"],
            "learnedmove|mon:Gastly,player-1|move:Confuse Ray|forgot:Curse",
            ["time"],
            "learnedmove|mon:Gastly,player-1|move:Dark Pulse|forgot:Lick",
            ["time"],
            "didnotlearnmove|mon:Gastly,player-1|move:Destiny Bond",
            ["time"],
            "learnedmove|mon:Gastly,player-1|move:Dream Eater|forgot:Mean Look",
            ["time"],
            "didnotlearnmove|mon:Gastly,player-1|move:Payback",
            ["time"],
            "learnedmove|mon:Gastly,player-1|move:Shadow Ball|forgot:Dark Pulse",
            ["time"],
            "didnotlearnmove|mon:Gastly,player-1|move:Sucker Punch",
            "residual",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Pikachu"],
            ["switch", "player-2", "Pikachu"],
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Gastly"],
            ["switch", "player-1", "Gastly"],
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Gastly,player-1,1|name:Shadow Ball|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:0",
            "damage|mon:Pikachu,player-2,1|health:0",
            "faint|mon:Pikachu,player-2,1",
            "exp|mon:Gastly,player-1,1|exp:10",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
