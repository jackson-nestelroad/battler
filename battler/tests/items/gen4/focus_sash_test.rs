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

fn pikachu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "item": "Focus Sash",
                    "moves": [],
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
                    "ability": "Skill Link",
                    "moves": [
                        "Earthquake",
                        "Pin Missile"
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn focus_sash_protects_from_ko_at_full_health() {
    let mut battle = make_battle(0, pikachu().unwrap(), eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item fullrestore,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eevee,player-2,1|name:Earthquake",
            "supereffective|mon:Pikachu,player-1,1",
            "itemend|mon:Pikachu,player-1,1|item:Focus Sash",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:1/95",
            "damage|mon:Pikachu,player-1,1|health:2/100",
            "residual",
            "turn|turn:2",
            "continue",
            "useitem|player:player-1|name:Full Restore|target:Pikachu,player-1,1",
            "split|side:0",
            "heal|mon:Pikachu,player-1,1|from:item:Full Restore|health:95/95",
            "heal|mon:Pikachu,player-1,1|from:item:Full Restore|health:100/100",
            "move|mon:Eevee,player-2,1|name:Earthquake",
            "supereffective|mon:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:0",
            "damage|mon:Pikachu,player-1,1|health:0",
            "faint|mon:Pikachu,player-1,1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn focus_sash_protects_from_first_hit_of_multihit_move() {
    let mut team = pikachu().unwrap();
    team.members[0].level = 1;
    let mut battle = make_battle(0, team, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eevee,player-2,1|name:Pin Missile|target:Pikachu,player-1,1",
            "itemend|mon:Pikachu,player-1,1|item:Focus Sash",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:1/11",
            "damage|mon:Pikachu,player-1,1|health:10/100",
            "animatemove|mon:Eevee,player-2,1|name:Pin Missile|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:0",
            "damage|mon:Pikachu,player-1,1|health:0",
            "faint|mon:Pikachu,player-1,1",
            "hitcount|hits:2",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
