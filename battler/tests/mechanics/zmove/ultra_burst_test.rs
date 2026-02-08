use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    Request,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn necrozma() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Necrozma",
                    "species": "Necrozma-Dusk-Mane",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Photon Geyser",
                        "Splash",
                        "Memento"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Ultranecrozium Z"
                },
                {
                    "name": "Solgaleo",
                    "species": "Solgaleo",
                    "gender": "M",
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
    seed: u64,
    battle_type: BattleType,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_z_moves(true)
        .with_ultra_burst(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn ultra_burst_transforms_necrozma_and_then_allows_z_move() {
    let mut battle = make_battle(
        100,
        BattleType::Singles,
        necrozma().unwrap(),
        necrozma().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert!(!request.active[0].can_z_move, "{:?}", request.active[0]);
        assert!(request.active[0].can_ultra_burst, "{:?}", request.active[0]);
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,ultra"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert!(request.active[0].can_z_move, "{:?}", request.active[0]);
        assert!(!request.active[0].can_ultra_burst, "{:?}", request.active[0]);
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["specieschange", "player-1", "species:Necrozma-Ultra"],
            ["specieschange", "player-1", "species:Necrozma-Ultra"],
            "ultra|mon:Necrozma,player-1,1|species:Necrozma-Ultra|from:item:Ultranecrozium Z",
            "move|mon:Necrozma,player-1,1|name:Splash|target:Necrozma,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:2",
            "continue",
            "singleturn|mon:Necrozma,player-1,1|condition:Z-Power",
            "move|mon:Necrozma,player-1,1|name:Light That Burns the Sky|target:Necrozma,player-2,1",
            "resisted|mon:Necrozma,player-2,1",
            "split|side:1",
            "damage|mon:Necrozma,player-2,1|health:107/157",
            "damage|mon:Necrozma,player-2,1|health:69/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ultra_burst_reverts_on_faint() {
    let mut team = necrozma().unwrap();
    team.members[0].species = "Necrozma-Dawn-Wings".to_owned();
    let mut battle = make_battle(100, BattleType::Singles, team, necrozma().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,ultra"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item revive,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,ultra"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Necrozma cannot ultra burst");
    });

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["specieschange", "player-1", "species:Necrozma-Ultra"],
            ["specieschange", "player-1", "species:Necrozma-Ultra"],
            "ultra|mon:Necrozma,player-1,1|species:Necrozma-Ultra|from:item:Ultranecrozium Z",
            "move|mon:Necrozma,player-1,1|name:Splash|target:Necrozma,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Necrozma,player-1,1|name:Memento|target:Necrozma,player-2,1",
            "unboost|mon:Necrozma,player-2,1|stat:atk|by:2",
            "unboost|mon:Necrozma,player-2,1|stat:spa|by:2",
            "faint|mon:Necrozma,player-1,1",
            "split|side:0",
            ["specieschange", "player-1", "species:Necrozma-Dawn-Wings"],
            ["specieschange", "player-1", "species:Necrozma-Dawn-Wings"],
            "revertultra|mon:Necrozma,player-1,1|species:Necrozma-Dawn-Wings|from:Faint",
            "residual",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Solgaleo"],
            ["switch", "player-1", "Solgaleo"],
            "turn|turn:3",
            "continue",
            "useitem|player:player-1|name:Revive|target:Necrozma,player-1",
            "revive|mon:Necrozma,player-1|from:item:Revive",
            "split|side:0",
            "sethp|mon:Necrozma,player-1|health:78/157",
            "sethp|mon:Necrozma,player-1|health:50/100",
            "residual",
            "turn|turn:4",
            "continue",
            "split|side:0",
            ["switch", "player-1", "species:Necrozma-Dawn-Wings"],
            ["switch", "player-1", "species:Necrozma-Dawn-Wings"],
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
