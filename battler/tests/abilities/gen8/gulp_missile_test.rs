use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
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
                    "name": "Cramorant",
                    "species": "Cramorant",
                    "ability": "Gulp Missile",
                    "moves": [
                        "Surf",
                        "Dive",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn gulp_missile_transforms_cramorant_and_attacks_on_hit() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cramorant,player-2,1|name:Tackle|target:Cramorant,player-1,1",
            "split|side:0",
            "damage|mon:Cramorant,player-1,1|health:199/250",
            "damage|mon:Cramorant,player-1,1|health:80/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Cramorant,player-1,1|name:Surf",
            "formechange|mon:Cramorant,player-1,1|species:Cramorant-Gulping|from:ability:Gulp Missile",
            "resisted|mon:Cramorant,player-2,1",
            "split|side:1",
            "damage|mon:Cramorant,player-2,1|health:204/250",
            "damage|mon:Cramorant,player-2,1|health:82/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Cramorant,player-2,1|name:Tackle|target:Cramorant,player-1,1",
            "split|side:0",
            "damage|mon:Cramorant,player-1,1|health:154/250",
            "damage|mon:Cramorant,player-1,1|health:62/100",
            "split|side:1",
            "damage|mon:Cramorant,player-2,1|from:ability:Gulp Missile|of:Cramorant,player-1,1|health:142/250",
            "damage|mon:Cramorant,player-2,1|from:ability:Gulp Missile|of:Cramorant,player-1,1|health:57/100",
            "unboost|mon:Cramorant,player-2,1|stat:def|by:1|from:ability:Gulp Missile|of:Cramorant,player-1,1",
            "formechange|mon:Cramorant,player-1,1|species:Cramorant|from:ability:Gulp Missile",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Cramorant,player-1,1|name:Dive|noanim",
            "prepare|mon:Cramorant,player-1,1|move:Dive",
            "formechange|mon:Cramorant,player-1,1|species:Cramorant-Gulping|from:ability:Gulp Missile",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Cramorant,player-1,1|name:Dive|target:Cramorant,player-2,1",
            "resisted|mon:Cramorant,player-2,1",
            "split|side:1",
            "damage|mon:Cramorant,player-2,1|health:37/250",
            "damage|mon:Cramorant,player-2,1|health:15/100",
            "move|mon:Cramorant,player-2,1|name:Tackle|target:Cramorant,player-1,1",
            "split|side:0",
            "damage|mon:Cramorant,player-1,1|health:107/250",
            "damage|mon:Cramorant,player-1,1|health:43/100",
            "split|side:1",
            "damage|mon:Cramorant,player-2,1|from:ability:Gulp Missile|of:Cramorant,player-1,1|health:0",
            "damage|mon:Cramorant,player-2,1|from:ability:Gulp Missile|of:Cramorant,player-1,1|health:0",
            "formechange|mon:Cramorant,player-1,1|species:Cramorant|from:ability:Gulp Missile",
            "faint|mon:Cramorant,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gulp_missile_trasnforms_into_gorging_forme_when_hp_below_half() {
    let mut team_1 = team().unwrap();
    team_1.members[0].persistent_battle_data.hp = Some(50);
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cramorant,player-1,1|name:Surf",
            "formechange|mon:Cramorant,player-1,1|species:Cramorant-Gorging|from:ability:Gulp Missile",
            "resisted|mon:Cramorant,player-2,1",
            "split|side:1",
            "damage|mon:Cramorant,player-2,1|health:201/250",
            "damage|mon:Cramorant,player-2,1|health:81/100",
            "move|mon:Cramorant,player-2,1|name:Tackle|target:Cramorant,player-1,1",
            "split|side:0",
            "damage|mon:Cramorant,player-1,1|health:3/250",
            "damage|mon:Cramorant,player-1,1|health:2/100",
            "split|side:1",
            "damage|mon:Cramorant,player-2,1|from:ability:Gulp Missile|of:Cramorant,player-1,1|health:139/250",
            "damage|mon:Cramorant,player-2,1|from:ability:Gulp Missile|of:Cramorant,player-1,1|health:56/100",
            "status|mon:Cramorant,player-2,1|status:Paralysis|from:ability:Gulp Missile|of:Cramorant,player-1,1",
            "formechange|mon:Cramorant,player-1,1|species:Cramorant|from:ability:Gulp Missile",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
