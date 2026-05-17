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
    assert_logs_since_start_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Calyrex",
                    "species": "Calyrex-Ice-Rider",
                    "ability": "As One",
                    "item": "Enigma Berry",
                    "moves": [
                        "Ice Beam",
                        "Earthquake"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Calyrex",
                    "species": "Calyrex-Shadow-Rider",
                    "ability": "As One",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Rillaboom",
                    "species": "Rillaboom",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Cinderace",
                    "species": "Cinderace",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Inteleon",
                    "species": "Inteleon",
                    "ability": "No Ability",
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
        .with_battle_type(BattleType::Doubles)
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
fn as_one_combines_chilling_or_grim_neigh_with_unnerve() {
    let mut team_2 = team().unwrap();
    for member in &mut team_2.members {
        member.persistent_battle_data.hp = Some(1);
    }
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;switch 3"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Calyrex-Ice"],
            ["switch", "player-1", "Calyrex-Ice"],
            "split|side:0",
            ["switch", "player-1", "Calyrex-Shadow"],
            ["switch", "player-1", "Calyrex-Shadow"],
            "split|side:1",
            ["switch", "player-2", "Calyrex-Ice"],
            ["switch", "player-2", "Calyrex-Ice"],
            "split|side:1",
            ["switch", "player-2", "Calyrex-Shadow"],
            ["switch", "player-2", "Calyrex-Shadow"],
            "ability|mon:Calyrex,player-2,2|ability:As One",
            "ability|mon:Calyrex,player-2,2|ability:Unnerve",
            "ability|mon:Calyrex,player-1,2|ability:As One",
            "ability|mon:Calyrex,player-1,2|ability:Unnerve",
            "ability|mon:Calyrex,player-2,1|ability:As One",
            "ability|mon:Calyrex,player-2,1|ability:Unnerve",
            "ability|mon:Calyrex,player-1,1|ability:As One",
            "ability|mon:Calyrex,player-1,1|ability:Unnerve",
            "turn|turn:1",
            "continue",
            "move|mon:Calyrex,player-1,2|name:Tackle|target:Calyrex,player-2,1",
            "split|side:1",
            "damage|mon:Calyrex,player-2,1|health:0",
            "damage|mon:Calyrex,player-2,1|health:0",
            "faint|mon:Calyrex,player-2,1",
            "boost|mon:Calyrex,player-1,2|stat:spa|by:1|from:ability:Grim Neigh",
            "move|mon:Calyrex,player-1,1|name:Ice Beam|target:Calyrex,player-2,2",
            "split|side:1",
            "damage|mon:Calyrex,player-2,2|health:0",
            "damage|mon:Calyrex,player-2,2|health:0",
            "faint|mon:Calyrex,player-2,2",
            "boost|mon:Calyrex,player-1,1|stat:atk|by:1|from:ability:Chilling Neigh",
            "residual",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Cinderace"],
            ["switch", "player-2", "Cinderace"],
            "split|side:1",
            ["switch", "player-2", "Rillaboom"],
            ["switch", "player-2", "Rillaboom"],
            "turn|turn:2",
            "continue",
            "move|mon:Calyrex,player-1,1|name:Earthquake|spread:Calyrex,player-1,2;Rillaboom,player-2,1;Cinderace,player-2,2",
            "resisted|mon:Rillaboom,player-2,1",
            "supereffective|mon:Cinderace,player-2,2",
            "split|side:0",
            "damage|mon:Calyrex,player-1,2|health:135/310",
            "damage|mon:Calyrex,player-1,2|health:44/100",
            "split|side:1",
            "damage|mon:Rillaboom,player-2,1|health:0",
            "damage|mon:Rillaboom,player-2,1|health:0",
            "split|side:1",
            "damage|mon:Cinderace,player-2,2|health:0",
            "damage|mon:Cinderace,player-2,2|health:0",
            "faint|mon:Rillaboom,player-2,1",
            "faint|mon:Cinderace,player-2,2",
            "boost|mon:Calyrex,player-1,1|stat:atk|by:2|from:ability:Chilling Neigh",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
