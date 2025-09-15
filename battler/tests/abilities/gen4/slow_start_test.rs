use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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

fn regigigas() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Regigigas",
                    "species": "Regigigas",
                    "ability": "Slow Start",
                    "moves": [
                        "Dragon Dance",
                        "Tackle"
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn slow_start_halves_attack_and_speed_for_five_turns() {
    let mut team = regigigas().unwrap();
    team.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(0, regigigas().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Regigigas,player-1,1|name:Dragon Dance|target:Regigigas,player-1,1",
            "boost|mon:Regigigas,player-1,1|stat:atk|by:1",
            "boost|mon:Regigigas,player-1,1|stat:spe|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Regigigas,player-2,1|name:Tackle|target:Regigigas,player-1,1",
            "split|side:0",
            "damage|mon:Regigigas,player-1,1|health:130/170",
            "damage|mon:Regigigas,player-1,1|health:77/100",
            "move|mon:Regigigas,player-1,1|name:Tackle|target:Regigigas,player-2,1",
            "split|side:1",
            "damage|mon:Regigigas,player-2,1|health:140/170",
            "damage|mon:Regigigas,player-2,1|health:83/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "residual",
            "turn|turn:4",
            ["time"],
            "residual",
            "turn|turn:5",
            ["time"],
            "end|mon:Regigigas,player-1,1|ability:Slow Start",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Regigigas,player-1,1|name:Tackle|target:Regigigas,player-2,1",
            "split|side:1",
            "damage|mon:Regigigas,player-2,1|health:82/170",
            "damage|mon:Regigigas,player-2,1|health:49/100",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
