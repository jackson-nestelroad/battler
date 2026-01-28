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

fn mewtwo() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Mewtwo",
                    "species": "Mewtwo",
                    "ability": "Pressure",
                    "moves": [
                        "Psychic",
                        "Psyshock"
                    ],
                    "nature": "Modest",
                    "level": 100,
                    "evs": {
                        "spa": 252,
                        "spe": 252
                    }
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn blissey() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blissey",
                    "species": "Blissey",
                    "ability": "Natural Cure",
                    "moves": [],
                    "nature": "Bold",
                    "level": 100,
                    "evs": {
                        "hp": 252,
                        "def": 252
                    }
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
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn psyshock_targets_defense() {
    let mut battle = make_battle(123456, mewtwo().unwrap(), blissey().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Mewtwo uses Psychic. Blissey should take relatively low damage (targeting SpD).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Mewtwo uses Psyshock. Blissey should faint (targeting Def).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Mewtwo,player-1,1|name:Psychic|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:524/683",
            "damage|mon:Blissey,player-2,1|health:77/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Mewtwo,player-1,1|name:Psyshock|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:124/683",
            "damage|mon:Blissey,player-2,1|health:19/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
