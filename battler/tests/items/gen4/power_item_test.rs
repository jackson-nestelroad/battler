use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    StatTable,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn empoleon() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Empoleon",
                    "species": "Empoleon",
                    "ability": "Torrent",
                    "item": "Power Anklet",
                    "moves": [
                        "Waterfall"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn bidoof() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bidoof",
                    "species": "Bidoof",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 20
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
        .add_protagonist_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn power_anklet_gives_8_additional_speed_evs() {
    let mut battle = make_battle(0, empoleon().unwrap(), bidoof().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Empoleon,player-1,1|name:Waterfall|target:Bidoof,player-2,1",
            "split|side:1",
            "damage|mon:Bidoof,player-2,1|health:0",
            "damage|mon:Bidoof,player-2,1|health:0",
            "faint|mon:Bidoof,player-2,1",
            "exp|mon:Empoleon,player-1,1|exp:69",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    assert_matches::assert_matches!(battle.player_data("player-1"), Ok(data) => {
        pretty_assertions::assert_eq!(
            data.mons[0].summary.evs,
            StatTable {
                hp: 1,
                spe: 8,
                ..Default::default()
            }
        )
    });
}
