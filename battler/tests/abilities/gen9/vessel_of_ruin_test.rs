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

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ting-Lu",
                    "species": "Ting-Lu",
                    "ability": "Vessel of Ruin",
                    "moves": [
                        "Dragon Pulse"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Chi-Yu",
                    "species": "Chi-Yu",
                    "ability": "Beads of Ruin",
                    "moves": [
                        "Dragon Pulse"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Chien-Pao",
                    "species": "Chien-Pao",
                    "ability": "Sword of Ruin",
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn vessel_of_ruin_decreases_special_attack_once() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;move 0,2"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Chi-Yu,player-1,2|name:Dragon Pulse|target:Chi-Yu,player-2,2",
            "split|side:1",
            "damage|mon:Chi-Yu,player-2,2|health:82/115",
            "damage|mon:Chi-Yu,player-2,2|health:72/100",
            "move|mon:Chi-Yu,player-2,2|name:Dragon Pulse|target:Chi-Yu,player-1,2",
            "split|side:0",
            "damage|mon:Chi-Yu,player-1,2|health:82/115",
            "damage|mon:Chi-Yu,player-1,2|health:72/100",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Chien-Pao"],
            ["switch", "player-1", "Chien-Pao"],
            "ability|mon:Chien-Pao,player-1,1|ability:Sword of Ruin",
            "move|mon:Chi-Yu,player-1,2|name:Dragon Pulse|target:Chi-Yu,player-2,2",
            "split|side:1",
            "damage|mon:Chi-Yu,player-2,2|health:49/115",
            "damage|mon:Chi-Yu,player-2,2|health:43/100",
            "move|mon:Chi-Yu,player-2,2|name:Dragon Pulse|target:Chi-Yu,player-1,2",
            "split|side:0",
            "damage|mon:Chi-Yu,player-1,2|health:49/115",
            "damage|mon:Chi-Yu,player-1,2|health:43/100",
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Chien-Pao"],
            ["switch", "player-2", "Chien-Pao"],
            "ability|mon:Chien-Pao,player-2,1|ability:Sword of Ruin",
            "move|mon:Chi-Yu,player-1,2|name:Dragon Pulse|target:Chi-Yu,player-2,2",
            "split|side:1",
            "damage|mon:Chi-Yu,player-2,2|health:6/115",
            "damage|mon:Chi-Yu,player-2,2|health:6/100",
            "move|mon:Chi-Yu,player-2,2|name:Dragon Pulse|target:Chi-Yu,player-1,2",
            "split|side:0",
            "damage|mon:Chi-Yu,player-1,2|health:6/115",
            "damage|mon:Chi-Yu,player-1,2|health:6/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn vessel_of_ruin_decreases_special_attack_except_if_you_have_ability() {
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ting-Lu,player-1,1|name:Dragon Pulse|target:Ting-Lu,player-2,1",
            "split|side:1",
            "damage|mon:Ting-Lu,player-2,1|health:187/215",
            "damage|mon:Ting-Lu,player-2,1|health:87/100",
            "move|mon:Ting-Lu,player-2,1|name:Dragon Pulse|target:Ting-Lu,player-1,1",
            "split|side:0",
            "damage|mon:Ting-Lu,player-1,1|health:194/215",
            "damage|mon:Ting-Lu,player-1,1|health:91/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
