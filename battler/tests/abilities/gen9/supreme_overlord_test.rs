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
                    "name": "Kingambit",
                    "species": "Kingambit",
                    "ability": "Supreme Overlord",
                    "moves": [
                        "Bite"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Dudunsparce",
                    "species": "Dudunsparce",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 1
                },
                {
                    "name": "Farigiraf",
                    "species": "Farigiraf",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 1
                },
                {
                    "name": "Clodsire",
                    "species": "Clodsire",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 1
                },
                {
                    "name": "Annihilape",
                    "species": "Annihilape",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 1
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn supreme_overlord_boosts_move_power_based_on_number_of_fallen_allies() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Dudunsparce"],
            ["switch", "player-1", "Dudunsparce"],
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Kingambit"],
            ["switch", "player-1", "Kingambit"],
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Kingambit,player-1,1|name:Bite|target:Kingambit,player-2,1",
            "resisted|mon:Kingambit,player-2,1",
            "split|side:1",
            "damage|mon:Kingambit,player-2,1|health:137/160",
            "damage|mon:Kingambit,player-2,1|health:86/100",
            "residual",
            "turn|turn:4",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Dudunsparce"],
            ["switch", "player-1", "Dudunsparce"],
            "move|mon:Kingambit,player-2,1|name:Bite|target:Dudunsparce,player-1,1",
            "split|side:0",
            "damage|mon:Dudunsparce,player-1,1|health:0",
            "damage|mon:Dudunsparce,player-1,1|health:0",
            "faint|mon:Dudunsparce,player-1,1",
            "residual",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Farigiraf"],
            ["switch", "player-1", "Farigiraf"],
            "turn|turn:5",
            "continue",
            "move|mon:Kingambit,player-2,1|name:Bite|target:Farigiraf,player-1,1",
            "supereffective|mon:Farigiraf,player-1,1",
            "split|side:0",
            "damage|mon:Farigiraf,player-1,1|health:0",
            "damage|mon:Farigiraf,player-1,1|health:0",
            "faint|mon:Farigiraf,player-1,1",
            "residual",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Kingambit"],
            ["switch", "player-1", "Kingambit"],
            "activate|mon:Kingambit,player-1,1|ability:Supreme Overlord",
            "start|mon:Kingambit,player-1,1|ability:Supreme Overlord|fallen:2|silent",
            "turn|turn:6",
            "continue",
            "move|mon:Kingambit,player-1,1|name:Bite|target:Kingambit,player-2,1",
            "resisted|mon:Kingambit,player-2,1",
            "split|side:1",
            "damage|mon:Kingambit,player-2,1|health:110/160",
            "damage|mon:Kingambit,player-2,1|health:69/100",
            "residual",
            "turn|turn:7",
            "continue",
            "end|mon:Kingambit,player-1,1|ability:Supreme Overlord|silent",
            "split|side:0",
            ["switch", "player-1", "Clodsire"],
            ["switch", "player-1", "Clodsire"],
            "move|mon:Kingambit,player-2,1|name:Bite|target:Clodsire,player-1,1",
            "split|side:0",
            "damage|mon:Clodsire,player-1,1|health:0",
            "damage|mon:Clodsire,player-1,1|health:0",
            "faint|mon:Clodsire,player-1,1",
            "residual",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Annihilape"],
            ["switch", "player-1", "Annihilape"],
            "turn|turn:8",
            "continue",
            "move|mon:Kingambit,player-2,1|name:Bite|target:Annihilape,player-1,1",
            "split|side:0",
            "damage|mon:Annihilape,player-1,1|health:0",
            "damage|mon:Annihilape,player-1,1|health:0",
            "faint|mon:Annihilape,player-1,1",
            "residual",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Kingambit"],
            ["switch", "player-1", "Kingambit"],
            "activate|mon:Kingambit,player-1,1|ability:Supreme Overlord",
            "start|mon:Kingambit,player-1,1|ability:Supreme Overlord|fallen:4|silent",
            "turn|turn:9",
            "continue",
            "move|mon:Kingambit,player-1,1|name:Bite|target:Kingambit,player-2,1",
            "resisted|mon:Kingambit,player-2,1",
            "split|side:1",
            "damage|mon:Kingambit,player-2,1|health:78/160",
            "damage|mon:Kingambit,player-2,1|health:49/100",
            "residual",
            "turn|turn:10"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
