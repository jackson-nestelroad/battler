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
    assert_new_logs_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Overgrow",
                    "moves": ["Tackle", "Air Cutter"],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "Blaze",
                    "moves": ["Scratch"],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 5
                },
                {
                    "name": "Squirtle",
                    "species": "Squirtle",
                    "ability": "Torrent",
                    "moves": ["Tackle"],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 5
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle_builder() -> TestBattleBuilder {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
        .with_seed(0)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
}

fn make_battle(data: &dyn DataStore, auto_continue: bool) -> Result<PublicCoreBattle> {
    make_battle_builder()
        .with_auto_continue(auto_continue)
        .with_team("player-1", team()?)
        .with_team("player-2", team()?)
        .build(data)
}

#[test]
fn must_switch_after_faint() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, true).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Charmander,player-2,2",
            "split|side:1",
            "damage|mon:Charmander,player-2,2|health:0",
            "damage|mon:Charmander,player-2,2|health:0",
            "faint|mon:Charmander,player-2,2",
            "move|mon:Charmander,player-1,2|name:Scratch|target:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:104/105",
            "damage|mon:Bulbasaur,player-2,1|health:99/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    let _ = battle.new_log_entries();

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(None));
    assert_matches::assert_matches!(battle.request_for_player("player-2"), Ok(Some(Request::Switch(request))) => {
        assert_eq!(request.needs_switch, vec![1]);
    });

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2"),
        Err(err) => assert_eq!(format!("{err:#}"), "you cannot do anything: no action requested")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;move 0,1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: you cannot move out of turn")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 0"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you cannot switch to an active mon")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you cannot switch to a fainted mon")
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:2|name:Squirtle|health:19/19|species:Squirtle|level:5|gender:F",
            "switch|player:player-2|position:2|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:F",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_new_logs_eq(&mut battle, &expected_logs);

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        let player_data = battle.player_data("player-1").unwrap();
        assert_eq!(request.active.len(), 2);
        assert_eq!(player_data.mons[request.active[0].team_position].summary.name, "Bulbasaur");
        assert_eq!(player_data.mons[request.active[1].team_position].summary.name, "Charmander");
    });
    assert_matches::assert_matches!(battle.request_for_player("player-2"), Ok(Some(Request::Turn(request))) => {
        let player_data = battle.player_data("player-2").unwrap();
        assert_eq!(request.active.len(), 2);
        assert_eq!(player_data.mons[request.active[0].team_position].summary.name, "Bulbasaur");
        assert_eq!(player_data.mons[request.active[1].team_position].summary.name, "Squirtle");
    });

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            ["time"],
            "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Squirtle,player-2,2",
            "split|side:1",
            "damage|mon:Squirtle,player-2,2|health:0",
            "damage|mon:Squirtle,player-2,2|health:0",
            "faint|mon:Squirtle,player-2,2",
            "move|mon:Charmander,player-1,2|name:Scratch|target:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:103/105",
            "damage|mon:Bulbasaur,player-2,1|health:99/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_new_logs_eq(&mut battle, &expected_logs);

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        let player_data = battle.player_data("player-1").unwrap();
        assert_eq!(request.active.len(), 2);
        assert_eq!(player_data.mons[request.active[0].team_position].summary.name, "Bulbasaur");
        assert_eq!(player_data.mons[request.active[1].team_position].summary.name, "Charmander");
    });
    assert_matches::assert_matches!(battle.request_for_player("player-2"), Ok(Some(Request::Turn(request))) => {
        let player_data = battle.player_data("player-2").unwrap();
        assert_eq!(request.active.len(), 1);
        assert_eq!(player_data.mons[request.active[0].team_position].summary.name, "Bulbasaur");
    });
}

#[test]
fn must_switch_one_after_two_faint() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, false).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            "switch|player:player-2|position:1|name:Squirtle|health:19/19|species:Squirtle|level:5|gender:F",
            "switch|player:player-2|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:F",
            "move|mon:Bulbasaur,player-1,1|name:Air Cutter|spread:Squirtle,player-2,1;Charmander,player-2,2",
            "crit|mon:Charmander,player-2,2",
            "split|side:1",
            "damage|mon:Squirtle,player-2,1|health:0",
            "damage|mon:Squirtle,player-2,1|health:0",
            "split|side:1",
            "damage|mon:Charmander,player-2,2|health:0",
            "damage|mon:Charmander,player-2,2|health:0",
            "faint|mon:Squirtle,player-2,1",
            "faint|mon:Charmander,player-2,2",
            "move|mon:Charmander,player-1,2|name:Scratch|notarget",
            "fail|mon:Charmander,player-1,2",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    let _ = battle.new_log_entries();

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(None));
    assert_matches::assert_matches!(battle.request_for_player("player-2"), Ok(Some(Request::Switch(request))) => {
        assert_eq!(request.needs_switch, vec![0, 1]);
    });

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2"),
        Err(err) => assert_eq!(format!("{err:#}"), "you cannot do anything: no action requested")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;move 0,1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: you cannot move out of turn")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you cannot switch to a fainted mon")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 0;switch 2"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 1: cannot switch: you cannot switch to a fainted mon")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 0;switch 0"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 1: cannot switch: the mon in slot 0 can only switch in once")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 0"),
        Err(err) => assert_eq!(format!("{err:#}"), "incomplete choice: missing actions for mons")
    );

    // We have a choice as to where the single Mon can be switched into.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 0;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;switch 0"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:2|name:Bulbasaur|health:105/105|species:Bulbasaur|level:50|gender:F",
            "switch|player:player-2|position:2|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_new_logs_eq(&mut battle, &expected_logs);

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        let player_data = battle.player_data("player-1").unwrap();
        assert_eq!(request.active.len(), 2);
        assert_eq!(player_data.mons[request.active[0].team_position].summary.name, "Bulbasaur");
        assert_eq!(player_data.mons[request.active[1].team_position].summary.name, "Charmander");
    });
    assert_matches::assert_matches!(battle.request_for_player("player-2"), Ok(Some(Request::Turn(request))) => {
        let player_data = battle.player_data("player-2").unwrap();
        assert_eq!(request.active.len(), 1);
        assert_eq!(player_data.mons[request.active[0].team_position].summary.name, "Bulbasaur");
    });
}
