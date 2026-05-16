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
                    "name": "Pyukumuku",
                    "species": "Pyukumuku",
                    "ability": "Innards Out",
                    "moves": [
                        "Thunderbolt"
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
        .with_dynamax(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn innards_out_deals_damage_on_faint() {
    let mut team_2 = team().unwrap();
    team_2.members[0].level = 50;
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pyukumuku,player-1,1|name:Thunderbolt|target:Pyukumuku,player-2,1",
            "supereffective|mon:Pyukumuku,player-2,1",
            "split|side:1",
            "damage|mon:Pyukumuku,player-2,1|health:43/115",
            "damage|mon:Pyukumuku,player-2,1|health:38/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pyukumuku,player-1,1|name:Thunderbolt|target:Pyukumuku,player-2,1",
            "supereffective|mon:Pyukumuku,player-2,1",
            "split|side:1",
            "damage|mon:Pyukumuku,player-2,1|health:0",
            "damage|mon:Pyukumuku,player-2,1|health:0",
            "split|side:0",
            "damage|mon:Pyukumuku,player-1,1|from:ability:Innards Out|of:Pyukumuku,player-2,1|health:177/220",
            "damage|mon:Pyukumuku,player-1,1|from:ability:Innards Out|of:Pyukumuku,player-2,1|health:81/100",
            "faint|mon:Pyukumuku,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn innards_out_loses_battle() {
    let mut team_1 = team().unwrap();
    team_1.members[0].persistent_battle_data.hp = Some(1);
    let mut team_2 = team().unwrap();
    team_2.members[0].level = 25;
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pyukumuku,player-1,1|name:Thunderbolt|target:Pyukumuku,player-2,1",
            "supereffective|mon:Pyukumuku,player-2,1",
            "split|side:1",
            "damage|mon:Pyukumuku,player-2,1|health:0",
            "damage|mon:Pyukumuku,player-2,1|health:0",
            "split|side:0",
            "damage|mon:Pyukumuku,player-1,1|from:ability:Innards Out|of:Pyukumuku,player-2,1|health:0",
            "damage|mon:Pyukumuku,player-1,1|from:ability:Innards Out|of:Pyukumuku,player-2,1|health:0",
            "faint|mon:Pyukumuku,player-2,1",
            "faint|mon:Pyukumuku,player-1,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn innards_out_deals_damage_equal_to_undynamaxed_hp_on_faint() {
    let mut team_2 = team().unwrap();
    team_2.members[0].level = 45;
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "dynamax|mon:Pyukumuku,player-2,1",
            "split|side:1",
            "sethp|mon:Pyukumuku,player-2,1|health:156/156",
            "sethp|mon:Pyukumuku,player-2,1|health:100/100",
            "move|mon:Pyukumuku,player-1,1|name:Thunderbolt|target:Pyukumuku,player-2,1",
            "supereffective|mon:Pyukumuku,player-2,1",
            "split|side:1",
            "damage|mon:Pyukumuku,player-2,1|health:76/156",
            "damage|mon:Pyukumuku,player-2,1|health:49/100",
            "move|mon:Pyukumuku,player-2,1|name:Max Lightning|target:Pyukumuku,player-1,1",
            "supereffective|mon:Pyukumuku,player-1,1",
            "split|side:0",
            "damage|mon:Pyukumuku,player-1,1|health:206/220",
            "damage|mon:Pyukumuku,player-1,1|health:94/100",
            "fieldstart|move:Electric Terrain",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pyukumuku,player-1,1|name:Thunderbolt|target:Pyukumuku,player-2,1",
            "supereffective|mon:Pyukumuku,player-2,1",
            "split|side:1",
            "damage|mon:Pyukumuku,player-2,1|health:0",
            "damage|mon:Pyukumuku,player-2,1|health:0",
            "split|side:0",
            "damage|mon:Pyukumuku,player-1,1|from:ability:Innards Out|of:Pyukumuku,player-2,1|health:155/220",
            "damage|mon:Pyukumuku,player-1,1|from:ability:Innards Out|of:Pyukumuku,player-2,1|health:71/100",
            "faint|mon:Pyukumuku,player-2,1",
            "revertdynamax|mon:Pyukumuku,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
