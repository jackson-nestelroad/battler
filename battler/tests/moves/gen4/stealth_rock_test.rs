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

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Rhyperior",
                    "species": "Rhyperior",
                    "ability": "No Ability",
                    "moves": [
                        "Stealth Rock"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Staraptor",
                    "species": "Staraptor",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Moltres",
                    "species": "Moltres",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Bibarel",
                    "species": "Bibarel",
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
fn stealth_rock_damages_mon_on_switch_in_with_type_effectiveness() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Rhyperior,player-1,1|name:Stealth Rock",
            "sidestart|side:1|move:Stealth Rock",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:1|name:Staraptor|health:145/145|species:Staraptor|level:50|gender:U",
            "switch|player:player-2|position:1|name:Staraptor|health:100/100|species:Staraptor|level:50|gender:U",
            "split|side:1",
            "damage|mon:Staraptor,player-2,1|from:move:Stealth Rock|health:109/145",
            "damage|mon:Staraptor,player-2,1|from:move:Stealth Rock|health:76/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:1|name:Moltres|health:150/150|species:Moltres|level:50|gender:U",
            "switch|player:player-2|position:1|name:Moltres|health:100/100|species:Moltres|level:50|gender:U",
            "split|side:1",
            "damage|mon:Moltres,player-2,1|from:move:Stealth Rock|health:75/150",
            "damage|mon:Moltres,player-2,1|from:move:Stealth Rock|health:50/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:1|name:Bibarel|health:139/139|species:Bibarel|level:50|gender:U",
            "switch|player:player-2|position:1|name:Bibarel|health:100/100|species:Bibarel|level:50|gender:U",
            "split|side:1",
            "damage|mon:Bibarel,player-2,1|from:move:Stealth Rock|health:122/139",
            "damage|mon:Bibarel,player-2,1|from:move:Stealth Rock|health:88/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:1|name:Rhyperior|health:175/175|species:Rhyperior|level:50|gender:U",
            "switch|player:player-2|position:1|name:Rhyperior|health:100/100|species:Rhyperior|level:50|gender:U",
            "split|side:1",
            "damage|mon:Rhyperior,player-2,1|from:move:Stealth Rock|health:165/175",
            "damage|mon:Rhyperior,player-2,1|from:move:Stealth Rock|health:95/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
