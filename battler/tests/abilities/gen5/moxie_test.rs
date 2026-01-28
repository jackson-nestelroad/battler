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

fn krookodile() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Krookodile",
                    "species": "Krookodile",
                    "ability": "Moxie",
                    "moves": [
                        "Earthquake"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn opponents() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Patrat",
                    "species": "Patrat",
                    "ability": "No Ability",
                    "moves": [
                        "Protect"
                    ],
                    "nature": "Hardy",
                    "level": 1
                },
                {
                    "name": "Patrat",
                    "species": "Patrat",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 1
                },
                {
                    "name": "Patrat",
                    "species": "Patrat",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 1
                },
                {
                    "name": "Patrat",
                    "species": "Patrat",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 1
                },
                {
                    "name": "Shedinja",
                    "species": "Shedinja",
                    "ability": "Wonder Guard",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 1,
                    "item": "Sticky Barb"
                },
                {
                    "name": "Patrat",
                    "species": "Patrat",
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
fn moxie_boosts_attack_after_fainting_mons() {
    let mut battle = make_battle(0, krookodile().unwrap(), opponents().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 3;switch 4"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 5;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Patrat,player-2,1|name:Protect|target:Patrat,player-2,1",
            "singleturn|mon:Patrat,player-2,1|move:Protect",
            "move|mon:Krookodile,player-1,1|name:Earthquake|spread:Patrat,player-2,2",
            "activate|mon:Patrat,player-2,1|move:Protect",
            "split|side:1",
            "damage|mon:Patrat,player-2,2|health:0",
            "damage|mon:Patrat,player-2,2|health:0",
            "faint|mon:Patrat,player-2,2",
            "boost|mon:Krookodile,player-1,1|stat:atk|by:1|from:ability:Moxie",
            "residual",
            "continue",
            "split|side:1",
            ["switch"],
            ["switch"],
            "turn|turn:2",
            "continue",
            "move|mon:Krookodile,player-1,1|name:Earthquake|spread:Patrat,player-2,1;Patrat,player-2,2",
            "split|side:1",
            "damage|mon:Patrat,player-2,1|health:0",
            "damage|mon:Patrat,player-2,1|health:0",
            "split|side:1",
            "damage|mon:Patrat,player-2,2|health:0",
            "damage|mon:Patrat,player-2,2|health:0",
            "faint|mon:Patrat,player-2,1",
            "faint|mon:Patrat,player-2,2",
            "boost|mon:Krookodile,player-1,1|stat:atk|by:2|from:ability:Moxie",
            "residual",
            "continue",
            "split|side:1",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "turn|turn:3",
            "continue",
            "move|mon:Krookodile,player-1,1|name:Earthquake|spread:Patrat,player-2,1",
            "immune|mon:Shedinja,player-2,2|from:ability:Wonder Guard",
            "split|side:1",
            "damage|mon:Patrat,player-2,1|health:0",
            "damage|mon:Patrat,player-2,1|health:0",
            "faint|mon:Patrat,player-2,1",
            "boost|mon:Krookodile,player-1,1|stat:atk|by:1|from:ability:Moxie",
            "split|side:1",
            "damage|mon:Shedinja,player-2,2|from:item:Sticky Barb|health:0",
            "damage|mon:Shedinja,player-2,2|from:item:Sticky Barb|health:0",
            "residual",
            "faint|mon:Shedinja,player-2,2",
            "continue",
            "split|side:1",
            ["switch"],
            ["switch"],
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
