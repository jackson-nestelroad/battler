use ahash::HashMap;
use anyhow::Result;
use battler::{
    BattleType,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_new_logs_eq,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur F",
                    "species": "Bulbasaur",
                    "ability": "Chlorophyll",
                    "moves": ["Tackle"],
                    "nature": "Modest",
                    "gender": "F",
                    "level": 100
                },
                {
                    "name": "Charmander F",
                    "species": "Charmander",
                    "ability": "Blaze",
                    "moves": ["Scratch"],
                    "nature": "Modest",
                    "gender": "F",
                    "level": 100
                },
                {
                    "name": "Squirtle F",
                    "species": "Squirtle",
                    "ability": "Torrent",
                    "moves": ["Tackle"],
                    "nature": "Modest",
                    "gender": "F",
                    "level": 100
                },
                {
                    "name": "Bulbasaur M",
                    "species": "Bulbasaur",
                    "ability": "Chlorophyll",
                    "moves": ["Tackle"],
                    "nature": "Modest",
                    "gender": "M",
                    "level": 100
                },
                {
                    "name": "Charmander M",
                    "species": "Charmander",
                    "ability": "Blaze",
                    "moves": ["Scratch"],
                    "nature": "Modest",
                    "gender": "M",
                    "level": 100
                },
                {
                    "name": "Squirtle M",
                    "species": "Squirtle",
                    "ability": "Torrent",
                    "moves": ["Tackle"],
                    "nature": "Modest",
                    "gender": "M",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_multi_battle(data: &dyn DataStore) -> Result<PublicCoreBattle<'_>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Multi)
        .with_rule("Standard")
        .with_rule("! Species Clause")
        .with_rule("Force Level = 100")
        .with_rule("Min Team Size = 3")
        .with_rule("Picked Team Size = 3")
        .with_rule("Team Preview")
        .with_seed(0)
        .with_auto_continue(false)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_1("player-2", "Player 2")
        .add_player_to_side_2("player-3", "Player 3")
        .add_player_to_side_2("player-4", "Player 4")
        // All players have the same team. We are testing that each player can pick a different
        // order.
        .with_team("player-1", team()?)
        .with_team("player-2", team()?)
        .with_team("player-3", team()?)
        .with_team("player-4", team()?)
        .build(data)
}

#[test]
fn team_preview_orders_all_player_teams() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_multi_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.ready_to_continue(), Ok(false));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "info|battletype:Multi",
            "info|environment:Normal",
            "info|rule:Endless Battle Clause: Forcing endless battles is banned",
            "info|rule:Sleep Clause: Limit one foe put to sleep",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:0|position:1",
            "player|id:player-3|name:Player 3|side:1|position:0",
            "player|id:player-4|name:Player 4|side:1|position:1",
            "teamsize|player:player-1|size:6",
            "teamsize|player:player-2|size:6",
            "teamsize|player:player-3|size:6",
            "teamsize|player:player-4|size:6",
            "teampreviewstart",
            "mon|player:player-1|species:Bulbasaur|level:100|gender:F",
            "mon|player:player-1|species:Charmander|level:100|gender:F",
            "mon|player:player-1|species:Squirtle|level:100|gender:F",
            "mon|player:player-1|species:Bulbasaur|level:100|gender:M",
            "mon|player:player-1|species:Charmander|level:100|gender:M",
            "mon|player:player-1|species:Squirtle|level:100|gender:M",
            "mon|player:player-2|species:Bulbasaur|level:100|gender:F",
            "mon|player:player-2|species:Charmander|level:100|gender:F",
            "mon|player:player-2|species:Squirtle|level:100|gender:F",
            "mon|player:player-2|species:Bulbasaur|level:100|gender:M",
            "mon|player:player-2|species:Charmander|level:100|gender:M",
            "mon|player:player-2|species:Squirtle|level:100|gender:M",
            "mon|player:player-3|species:Bulbasaur|level:100|gender:F",
            "mon|player:player-3|species:Charmander|level:100|gender:F",
            "mon|player:player-3|species:Squirtle|level:100|gender:F",
            "mon|player:player-3|species:Bulbasaur|level:100|gender:M",
            "mon|player:player-3|species:Charmander|level:100|gender:M",
            "mon|player:player-3|species:Squirtle|level:100|gender:M",
            "mon|player:player-4|species:Bulbasaur|level:100|gender:F",
            "mon|player:player-4|species:Charmander|level:100|gender:F",
            "mon|player:player-4|species:Squirtle|level:100|gender:F",
            "mon|player:player-4|species:Bulbasaur|level:100|gender:M",
            "mon|player:player-4|species:Charmander|level:100|gender:M",
            "mon|player:player-4|species:Squirtle|level:100|gender:M",
            "teampreview|pick:3"
        ]"#,
    )
    .unwrap();
    assert_new_logs_eq(&mut battle, &expected_logs);

    // Player 1 made their choice.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "team 0 1 2"), Ok(()));
    assert!(
        !battle
            .active_requests()
            .collect::<HashMap<_, _>>()
            .contains_key("player-1")
    );
    assert_matches::assert_matches!(battle.ready_to_continue(), Ok(false));
    assert!(!battle.has_new_log_entries());

    // Auto choose.
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "team"), Ok(()));
    // Not enough Mons, auto choose the rest.
    assert_matches::assert_matches!(battle.set_player_choice("player-3", "team 1 2"), Ok(()));
    // Reselect Mons.
    assert_matches::assert_matches!(battle.set_player_choice("player-3", "team 2 5"), Ok(()));
    // Too many Mons, truncate the list.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-4", "team 5 4 3 2 1 0"),
        Ok(())
    );
    // No more active requests.
    assert!(battle.active_requests().collect::<Vec<_>>().is_empty());
    assert_matches::assert_matches!(battle.ready_to_continue(), Ok(true));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));

    // New logs show updated team size and selected team leads.
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            ["time"],
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "teamsize|player:player-3|size:3",
            "teamsize|player:player-4|size:3",
            "battlestart",
            "split|side:0",
            "switch|player:player-1|position:1|name:Bulbasaur F|health:200/200|species:Bulbasaur|level:100|gender:F",
            "switch|player:player-1|position:1|name:Bulbasaur F|health:100/100|species:Bulbasaur|level:100|gender:F",
            "split|side:0",
            "switch|player:player-2|position:2|name:Bulbasaur F|health:200/200|species:Bulbasaur|level:100|gender:F",
            "switch|player:player-2|position:2|name:Bulbasaur F|health:100/100|species:Bulbasaur|level:100|gender:F",
            "split|side:1",
            "switch|player:player-3|position:1|name:Squirtle F|health:198/198|species:Squirtle|level:100|gender:F",
            "switch|player:player-3|position:1|name:Squirtle F|health:100/100|species:Squirtle|level:100|gender:F",
            "split|side:1",
            "switch|player:player-4|position:2|name:Squirtle M|health:198/198|species:Squirtle|level:100|gender:M",
            "switch|player:player-4|position:2|name:Squirtle M|health:100/100|species:Squirtle|level:100|gender:M",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_new_logs_eq(&mut battle, &expected_logs);

    // Turn 1: each player switches to Mon 1.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-3", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-4", "switch 1"), Ok(()));

    assert!(battle.active_requests().collect::<Vec<_>>().is_empty());
    assert_matches::assert_matches!(battle.ready_to_continue(), Ok(true));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            ["time"],
            "split|side:0",
            "switch|player:player-1|position:1|name:Charmander F|health:188/188|species:Charmander|level:100|gender:F",
            "switch|player:player-1|position:1|name:Charmander F|health:100/100|species:Charmander|level:100|gender:F",
            "split|side:0",
            "switch|player:player-2|position:2|name:Charmander F|health:188/188|species:Charmander|level:100|gender:F",
            "switch|player:player-2|position:2|name:Charmander F|health:100/100|species:Charmander|level:100|gender:F",
            "split|side:1",
            "switch|player:player-3|position:1|name:Squirtle M|health:198/198|species:Squirtle|level:100|gender:M",
            "switch|player:player-3|position:1|name:Squirtle M|health:100/100|species:Squirtle|level:100|gender:M",
            "split|side:1",
            "switch|player:player-4|position:2|name:Charmander M|health:188/188|species:Charmander|level:100|gender:M",
            "switch|player:player-4|position:2|name:Charmander M|health:100/100|species:Charmander|level:100|gender:M",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_new_logs_eq(&mut battle, &expected_logs);

    // Turn 2: each player switches to Mon 2.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-3", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-4", "switch 2"), Ok(()));

    assert!(battle.active_requests().collect::<Vec<_>>().is_empty());
    assert_matches::assert_matches!(battle.ready_to_continue(), Ok(true));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            ["time"],
            "split|side:0",
            "switch|player:player-2|position:2|name:Squirtle F|health:198/198|species:Squirtle|level:100|gender:F",
            "switch|player:player-2|position:2|name:Squirtle F|health:100/100|species:Squirtle|level:100|gender:F",
            "split|side:1",
            "switch|player:player-4|position:2|name:Bulbasaur M|health:200/200|species:Bulbasaur|level:100|gender:M",
            "switch|player:player-4|position:2|name:Bulbasaur M|health:100/100|species:Bulbasaur|level:100|gender:M",
            "split|side:0",
            "switch|player:player-1|position:1|name:Squirtle F|health:198/198|species:Squirtle|level:100|gender:F",
            "switch|player:player-1|position:1|name:Squirtle F|health:100/100|species:Squirtle|level:100|gender:F",
            "split|side:1",
            "switch|player:player-3|position:1|name:Bulbasaur F|health:200/200|species:Bulbasaur|level:100|gender:F",
            "switch|player:player-3|position:1|name:Bulbasaur F|health:100/100|species:Bulbasaur|level:100|gender:F",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_new_logs_eq(&mut battle, &expected_logs);

    // Turn 3: each player tries to switch to Mon 3.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 3"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you do not have a mon in slot 3 to switch to")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 3"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you do not have a mon in slot 3 to switch to")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-3", "switch 3"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you do not have a mon in slot 3 to switch to")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-4", "switch 3"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you do not have a mon in slot 3 to switch to")
    );

    // Verify other slots fail for good measure.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 4"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you do not have a mon in slot 4 to switch to")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 5"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you do not have a mon in slot 5 to switch to")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 6"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you do not have a mon in slot 6 to switch to")
    );

    // Switch back to Mon 0 (the lead that started the battle).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-3", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-4", "switch 0"), Ok(()));

    assert!(battle.active_requests().collect::<Vec<_>>().is_empty());
    assert_matches::assert_matches!(battle.ready_to_continue(), Ok(true));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            ["time"],
            "split|side:1",
            "switch|player:player-3|position:1|name:Squirtle F|health:198/198|species:Squirtle|level:100|gender:F",
            "switch|player:player-3|position:1|name:Squirtle F|health:100/100|species:Squirtle|level:100|gender:F",
            "split|side:1",
            "switch|player:player-4|position:2|name:Squirtle M|health:198/198|species:Squirtle|level:100|gender:M",
            "switch|player:player-4|position:2|name:Squirtle M|health:100/100|species:Squirtle|level:100|gender:M",
            "split|side:0",
            "switch|player:player-1|position:1|name:Bulbasaur F|health:200/200|species:Bulbasaur|level:100|gender:F",
            "switch|player:player-1|position:1|name:Bulbasaur F|health:100/100|species:Bulbasaur|level:100|gender:F",
            "split|side:0",
            "switch|player:player-2|position:2|name:Bulbasaur F|health:200/200|species:Bulbasaur|level:100|gender:F",
            "switch|player:player-2|position:2|name:Bulbasaur F|health:100/100|species:Bulbasaur|level:100|gender:F",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_new_logs_eq(&mut battle, &expected_logs);
}
