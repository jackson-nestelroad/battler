use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn staraptor() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Staraptor",
                    "species": "Staraptor",
                    "ability": "No Ability",
                    "moves": [
                        "Roost"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn pachirisu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pachirisu",
                    "species": "Pachirisu",
                    "ability": "No Ability",
                    "moves": [
                        "Thunderbolt",
                        "Thunder Shock"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'_>> {
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
        .build(data)
}

#[test]
fn roost_heals_damage_and_removes_flying_type() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, staraptor().unwrap(), pachirisu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pachirisu,player-2,1|name:Thunderbolt|target:Staraptor,player-1,1",
            "supereffective|mon:Staraptor,player-1,1",
            "split|side:0",
            "damage|mon:Staraptor,player-1,1|health:53/145",
            "damage|mon:Staraptor,player-1,1|health:37/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Staraptor,player-1,1|name:Roost|target:Staraptor,player-1,1",
            "split|side:0",
            "heal|mon:Staraptor,player-1,1|health:126/145",
            "heal|mon:Staraptor,player-1,1|health:87/100",
            "singleturn|mon:Staraptor,player-1,1|move:Roost",
            "move|mon:Pachirisu,player-2,1|name:Thunderbolt|target:Staraptor,player-1,1",
            "split|side:0",
            "damage|mon:Staraptor,player-1,1|health:83/145",
            "damage|mon:Staraptor,player-1,1|health:58/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pachirisu,player-2,1|name:Thunder Shock|target:Staraptor,player-1,1",
            "supereffective|mon:Staraptor,player-1,1",
            "split|side:0",
            "damage|mon:Staraptor,player-1,1|health:45/145",
            "damage|mon:Staraptor,player-1,1|health:32/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
