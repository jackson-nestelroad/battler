use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    Id,
    MonMoveSlotData,
    MoveTarget,
    PublicCoreBattle,
    Request,
    TeamData,
    Type,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn pikachu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Volt Tackle",
                        "Tackle",
                        "Thunderbolt"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn eevee() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Eevee",
                    "species": "Eevee",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Quick Attack"
                    ],
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
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_z_moves(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn species_based_z_crystal_only_allows_single_move_and_user() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Pikanium Z".to_owned());
    let mut eevee = eevee().unwrap();
    eevee.members[0].item = Some("Pikanium Z".to_owned());
    let mut battle = make_battle(100, pikachu, eevee).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert_matches::assert_matches!(request.active[0].z_moves.get(0), Some(Some(data)) => {
            pretty_assertions::assert_eq!(
                *data,
                MonMoveSlotData {
                    id: Id::from("catastropika"),
                    name: "Catastropika".to_owned(),
                    pp: 15,
                    max_pp: 15,
                    target: MoveTarget::Normal,
                    disabled: false,
                }
            );
        });
        assert!(
            request
                .active[0]
                .z_moves
                .iter()
                .enumerate()
                .all(|(i, z_move)| i == 0 || z_move.is_none()),
            "{:?}",
            request.active[0].z_moves
        );
    });
    assert_matches::assert_matches!(battle.request_for_player("player-2"), Ok(Some(Request::Turn(request))) => {
        assert!(!request.active[0].can_z_move, "{:?}", request.active[0]);
        assert!(
            request
                .active[0]
                .z_moves
                .iter()
                .all(|z_move| z_move.is_none()),
            "{:?}",
            request.active[0].z_moves
        );
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,zmove"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: move in slot 1 cannot be upgraded to z-move");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2,zmove"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: move in slot 2 cannot be upgraded to z-move");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,zmove"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: move in slot 0 cannot be upgraded to z-move");
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Catastropika|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:0",
            "damage|mon:Eevee,player-2,1|health:0",
            "faint|mon:Eevee,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
