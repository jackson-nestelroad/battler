use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    Request,
    SelectPosition,
    SelectReason,
    SelectRequest,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pawmot",
                    "species": "Pawmot",
                    "ability": "No Ability",
                    "item": "Leppa Berry",
                    "moves": [
                        "Revival Blessing",
                        "Thunderbolt",
                        "Brick Break"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Quaxly",
                    "species": "Quaxly",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Sprigatito",
                    "species": "Sprigatito",
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

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
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
        .build(static_local_data_store())
}

#[test]
fn revival_blessing_revives_selected_mon() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 2,1;pass"),
        Ok(())
    );

    assert_matches::assert_matches!(
        battle.request_for_player("player-1"),
        Ok(Some(Request::Select(request))) => {
            pretty_assertions::assert_eq!(
                request,
                SelectRequest {
                    positions: Vec::from_iter([
                        SelectPosition {
                            position: 0,
                            reason: SelectReason::Revive,
                        }
                    ])
                }
            );
        }
    );

    assert_matches::assert_matches!(battle.request_for_player("player-2"), Ok(None));

    assert_matches::assert_matches!(battle.set_player_choice("player-2", "select 0"), Err(err) => {
        assert_eq!(format!("{err:#}"), "you cannot do anything: no action requested");
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "select 0"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot select: Pawmot is not fainted");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "select 1;select 2"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 1: cannot select: you sent more selections than mons that need a selection");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you cannot switch out of turn");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "select 1"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pawmot,player-1,1|name:Revival Blessing|noanim",
            "fail|mon:Pawmot,player-1,1",
            "itemend|mon:Pawmot,player-1,1|item:Leppa Berry|eat",
            "restorepp|mon:Pawmot,player-1,1|move:Revival Blessing|by:1|from:item:Leppa Berry",
            "move|mon:Pawmot,player-2,1|name:Thunderbolt|target:Quaxly,player-1,2",
            "supereffective|mon:Quaxly,player-1,2",
            "split|side:0",
            "damage|mon:Quaxly,player-1,2|health:0",
            "damage|mon:Quaxly,player-1,2|health:0",
            "faint|mon:Quaxly,player-1,2",
            "residual",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Sprigatito"],
            ["switch", "player-1", "Sprigatito"],
            "turn|turn:2",
            "continue",
            "move|mon:Pawmot,player-1,1|name:Revival Blessing|target:Pawmot,player-1,1",
            "continue",
            "revive|mon:Quaxly,player-1|from:move:Revival Blessing|of:Pawmot,player-1,1",
            "split|side:0",
            "sethp|mon:Quaxly,player-1|health:57/115",
            "sethp|mon:Quaxly,player-1|health:50/100",
            "move|mon:Pawmot,player-2,1|name:Brick Break|target:Pawmot,player-1,1",
            "split|side:0",
            "damage|mon:Pawmot,player-1,1|health:109/250",
            "damage|mon:Pawmot,player-1,1|health:44/100",
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Quaxly"],
            ["switch", "player-1", "Quaxly"],
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
