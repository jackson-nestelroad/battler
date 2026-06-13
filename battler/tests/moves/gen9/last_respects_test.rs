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
                    "name": "Basculegion",
                    "species": "Basculegion",
                    "ability": "No Ability",
                    "moves": [
                        "Last Respects",
                        "Recover",
                        "Thunderbolt",
                        "Revival Blessing"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Basculin",
                    "species": "Basculin-Red-Striped",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Basculin",
                    "species": "Basculin-Blue-Striped",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Basculin",
                    "species": "Basculin-White-Striped",
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
fn last_respects_increases_power_with_each_mon_fainted() {
    let mut team_2 = team().unwrap();
    team_2.members[0].level = 100;
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "select 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Basculegion,player-2,1|name:Recover|noanim",
            "fail|mon:Basculegion,player-2,1|what:heal",
            "move|mon:Basculegion,player-1,1|name:Last Respects|target:Basculegion,player-2,1",
            "supereffective|mon:Basculegion,player-2,1",
            "split|side:1",
            "damage|mon:Basculegion,player-2,1|health:288/350",
            "damage|mon:Basculegion,player-2,1|health:83/100",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["switch"],
            ["switch"],
            "move|mon:Basculegion,player-2,1|name:Thunderbolt|target:Basculin,player-1,1",
            "supereffective|mon:Basculin,player-1,1",
            "split|side:0",
            "damage|mon:Basculin,player-1,1|health:0",
            "damage|mon:Basculin,player-1,1|health:0",
            "faint|mon:Basculin,player-1,1",
            "residual",
            "continue",
            "split|side:0",
            ["switch"],
            ["switch"],
            "turn|turn:3",
            "continue",
            "move|mon:Basculegion,player-2,1|name:Recover|target:Basculegion,player-2,1",
            "split|side:1",
            "heal|mon:Basculegion,player-2,1|health:350/350",
            "heal|mon:Basculegion,player-2,1|health:100/100",
            "move|mon:Basculegion,player-1,1|name:Last Respects|target:Basculegion,player-2,1",
            "supereffective|mon:Basculegion,player-2,1",
            "split|side:1",
            "damage|mon:Basculegion,player-2,1|health:230/350",
            "damage|mon:Basculegion,player-2,1|health:66/100",
            "residual",
            "turn|turn:4",
            "continue",
            "split|side:0",
            ["switch"],
            ["switch"],
            "move|mon:Basculegion,player-2,1|name:Thunderbolt|target:Basculin,player-1,1",
            "supereffective|mon:Basculin,player-1,1",
            "split|side:0",
            "damage|mon:Basculin,player-1,1|health:0",
            "damage|mon:Basculin,player-1,1|health:0",
            "faint|mon:Basculin,player-1,1",
            "residual",
            "continue",
            "split|side:0",
            ["switch"],
            ["switch"],
            "turn|turn:5",
            "continue",
            "move|mon:Basculegion,player-2,1|name:Thunderbolt|target:Basculin,player-1,1",
            "supereffective|mon:Basculin,player-1,1",
            "split|side:0",
            "damage|mon:Basculin,player-1,1|health:0",
            "damage|mon:Basculin,player-1,1|health:0",
            "faint|mon:Basculin,player-1,1",
            "residual",
            "continue",
            "split|side:0",
            ["switch"],
            ["switch"],
            "turn|turn:6",
            "continue",
            "move|mon:Basculegion,player-2,1|name:Recover|target:Basculegion,player-2,1",
            "split|side:1",
            "heal|mon:Basculegion,player-2,1|health:350/350",
            "heal|mon:Basculegion,player-2,1|health:100/100",
            "move|mon:Basculegion,player-1,1|name:Last Respects|target:Basculegion,player-2,1",
            "supereffective|mon:Basculegion,player-2,1",
            "split|side:1",
            "damage|mon:Basculegion,player-2,1|health:116/350",
            "damage|mon:Basculegion,player-2,1|health:34/100",
            "residual",
            "turn|turn:7",
            "continue",
            "move|mon:Basculegion,player-2,1|name:Recover|target:Basculegion,player-2,1",
            "split|side:1",
            "heal|mon:Basculegion,player-2,1|health:291/350",
            "heal|mon:Basculegion,player-2,1|health:84/100",
            "move|mon:Basculegion,player-1,1|name:Revival Blessing|target:Basculegion,player-1,1",
            "continue",
            "revive|mon:Basculin,player-1|from:move:Revival Blessing|of:Basculegion,player-1,1",
            "split|side:0",
            "sethp|mon:Basculin,player-1|health:65/130",
            "sethp|mon:Basculin,player-1|health:50/100",
            "residual",
            "turn|turn:8",
            "continue",
            "split|side:0",
            ["switch"],
            ["switch"],
            "move|mon:Basculegion,player-2,1|name:Thunderbolt|target:Basculin,player-1,1",
            "supereffective|mon:Basculin,player-1,1",
            "split|side:0",
            "damage|mon:Basculin,player-1,1|health:0",
            "damage|mon:Basculin,player-1,1|health:0",
            "faint|mon:Basculin,player-1,1",
            "residual",
            "continue",
            "split|side:0",
            ["switch"],
            ["switch"],
            "turn|turn:9",
            "continue",
            "move|mon:Basculegion,player-2,1|name:Recover|target:Basculegion,player-2,1",
            "split|side:1",
            "heal|mon:Basculegion,player-2,1|health:350/350",
            "heal|mon:Basculegion,player-2,1|health:100/100",
            "move|mon:Basculegion,player-1,1|name:Last Respects|target:Basculegion,player-2,1",
            "supereffective|mon:Basculegion,player-2,1",
            "split|side:1",
            "damage|mon:Basculegion,player-2,1|health:60/350",
            "damage|mon:Basculegion,player-2,1|health:18/100",
            "residual",
            "turn|turn:10"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
