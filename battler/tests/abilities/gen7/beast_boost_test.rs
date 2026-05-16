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
                    "name": "Buzzwole",
                    "species": "Buzzwole",
                    "ability": "Beast Boost",
                    "moves": [
                        "Tackle",
                        "Earthquake"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Nihilego",
                    "species": "Nihilego",
                    "ability": "Beast Boost",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Pheromosa",
                    "species": "Pheromosa",
                    "ability": "Beast Boost",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Xurkitree",
                    "species": "Xurkitree",
                    "ability": "Beast Boost",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Celesteela",
                    "species": "Xurkitree",
                    "ability": "Beast Boost",
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
fn beast_boost_boosts_best_stat_on_fainting_mons() {
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
            "move|mon:Nihilego,player-1,2|name:Tackle|target:Buzzwole,player-2,1",
            "split|side:1",
            "damage|mon:Buzzwole,player-2,1|health:0",
            "damage|mon:Buzzwole,player-2,1|health:0",
            "faint|mon:Buzzwole,player-2,1",
            "boost|mon:Nihilego,player-1,2|stat:spd|by:1|from:ability:Beast Boost",
            "move|mon:Buzzwole,player-1,1|name:Tackle|target:Nihilego,player-2,2",
            "resisted|mon:Nihilego,player-2,2",
            "split|side:1",
            "damage|mon:Nihilego,player-2,2|health:0",
            "damage|mon:Nihilego,player-2,2|health:0",
            "faint|mon:Nihilego,player-2,2",
            "boost|mon:Buzzwole,player-1,1|stat:atk|by:1|from:ability:Beast Boost",
            "residual",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Xurkitree"],
            ["switch", "player-2", "Xurkitree"],
            "split|side:1",
            ["switch", "player-2", "Pheromosa"],
            ["switch", "player-2", "Pheromosa"],
            "turn|turn:2",
            "continue",
            "move|mon:Buzzwole,player-1,1|name:Earthquake|spread:Nihilego,player-1,2;Pheromosa,player-2,1;Xurkitree,player-2,2",
            "supereffective|mon:Nihilego,player-1,2",
            "resisted|mon:Pheromosa,player-2,1",
            "supereffective|mon:Xurkitree,player-2,2",
            "split|side:0",
            "damage|mon:Nihilego,player-1,2|health:0",
            "damage|mon:Nihilego,player-1,2|health:0",
            "split|side:1",
            "damage|mon:Pheromosa,player-2,1|health:0",
            "damage|mon:Pheromosa,player-2,1|health:0",
            "split|side:1",
            "damage|mon:Xurkitree,player-2,2|health:0",
            "damage|mon:Xurkitree,player-2,2|health:0",
            "faint|mon:Nihilego,player-1,2",
            "faint|mon:Pheromosa,player-2,1",
            "faint|mon:Xurkitree,player-2,2",
            "boost|mon:Buzzwole,player-1,1|stat:atk|by:3|from:ability:Beast Boost",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
