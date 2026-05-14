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
                    "name": "Mimikyu",
                    "species": "Mimikyu",
                    "ability": "Disguise",
                    "moves": [
                        "Shadow Claw"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Mimikyu",
                    "species": "Mimikyu-Totem",
                    "ability": "Disguise",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Ditto",
                    "species": "Ditto",
                    "ability": "Imposter",
                    "moves": [],
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
fn disguise_consumes_super_effective_damage() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Mimikyu,player-1,1|name:Shadow Claw|target:Mimikyu,player-2,1",
            "split|side:1",
            "specieschange|player:player-2|position:1|name:Mimikyu|health:220/220|species:Mimikyu-Busted|level:100|gender:U",
            "specieschange|player:player-2|position:1|name:Mimikyu|health:100/100|species:Mimikyu-Busted|level:100|gender:U",
            "formechange|mon:Mimikyu,player-2,1|species:Mimikyu-Busted|from:ability:Disguise",
            "split|side:1",
            "damage|mon:Mimikyu,player-2,1|from:ability:Disguise|health:193/220",
            "damage|mon:Mimikyu,player-2,1|from:ability:Disguise|health:88/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Mimikyu,player-1,1|name:Shadow Claw|target:Mimikyu,player-2,1",
            "crit|mon:Mimikyu,player-2,1",
            "split|side:1",
            "damage|mon:Mimikyu,player-2,1|health:58/220",
            "damage|mon:Mimikyu,player-2,1|health:27/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn disguise_works_for_mimikyu_totem() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Mimikyu-Totem-Disguised"],
            ["switch", "player-2", "Mimikyu-Totem-Disguised"],
            "move|mon:Mimikyu,player-1,1|name:Shadow Claw|target:Mimikyu,player-2,1",
            "split|side:1",
            "specieschange|player:player-2|position:1|name:Mimikyu|health:220/220|species:Mimikyu-Totem-Busted|level:100|gender:U",
            "specieschange|player:player-2|position:1|name:Mimikyu|health:100/100|species:Mimikyu-Totem-Busted|level:100|gender:U",
            "formechange|mon:Mimikyu,player-2,1|species:Mimikyu-Totem-Busted|from:ability:Disguise",
            "split|side:1",
            "damage|mon:Mimikyu,player-2,1|from:ability:Disguise|health:193/220",
            "damage|mon:Mimikyu,player-2,1|from:ability:Disguise|health:88/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Mimikyu,player-1,1|name:Shadow Claw|target:Mimikyu,player-2,1",
            "crit|mon:Mimikyu,player-2,1",
            "split|side:1",
            "damage|mon:Mimikyu,player-2,1|health:58/220",
            "damage|mon:Mimikyu,player-2,1|health:27/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn transform_takes_on_mimikyu_forme_at_transformation() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Ditto"],
            ["switch", "player-2", "Ditto"],
            "transform|mon:Ditto,player-2,1|into:Mimikyu,player-1,1|species:Mimikyu|from:ability:Imposter",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Mimikyu,player-1,1|name:Shadow Claw|target:Ditto,player-2,1",
            "supereffective|mon:Ditto,player-2,1",
            "split|side:1",
            "damage|mon:Ditto,player-2,1|health:14/206",
            "damage|mon:Ditto,player-2,1|health:7/100",
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Mimikyu"],
            ["switch", "player-2", "Mimikyu"],
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Mimikyu,player-2,1|name:Shadow Claw|target:Mimikyu,player-1,1",
            "split|side:0",
            "specieschange|player:player-1|position:1|name:Mimikyu|health:220/220|species:Mimikyu-Busted|level:100|gender:U",
            "specieschange|player:player-1|position:1|name:Mimikyu|health:100/100|species:Mimikyu-Busted|level:100|gender:U",
            "formechange|mon:Mimikyu,player-1,1|species:Mimikyu-Busted|from:ability:Disguise",
            "split|side:0",
            "damage|mon:Mimikyu,player-1,1|from:ability:Disguise|health:193/220",
            "damage|mon:Mimikyu,player-1,1|from:ability:Disguise|health:88/100",
            "residual",
            "turn|turn:5",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Ditto"],
            ["switch", "player-2", "Ditto"],
            "transform|mon:Ditto,player-2,1|into:Mimikyu,player-1,1|species:Mimikyu-Busted|from:ability:Imposter",
            "residual",
            "turn|turn:6",
            "continue",
            "move|mon:Mimikyu,player-1,1|name:Shadow Claw|target:Ditto,player-2,1",
            "supereffective|mon:Ditto,player-2,1",
            "split|side:1",
            "damage|mon:Ditto,player-2,1|health:0",
            "damage|mon:Ditto,player-2,1|health:0",
            "faint|mon:Ditto,player-2,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
