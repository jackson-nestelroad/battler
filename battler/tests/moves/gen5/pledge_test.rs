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

fn team() -> Result<TeamData, anyhow::Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Emboar",
                    "species": "Emboar",
                    "ability": "No Ability",
                    "moves": [
                        "Fire Pledge",
                        "Flamethrower",
                        "Charge Beam"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Serperior",
                    "species": "Serperior",
                    "ability": "No Ability",
                    "moves": [
                        "Grass Pledge"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Samurott",
                    "species": "Samurott",
                    "ability": "No Ability",
                    "moves": [
                        "Water Pledge"
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
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>, anyhow::Error> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
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
fn fire_pledge_solo() {
    let mut battle = make_battle(BattleType::Singles, 0, team().unwrap(), team().unwrap()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Emboar,player-1,1|name:Fire Pledge|target:Emboar,player-2,1",
            "resisted|mon:Emboar,player-2,1",
            "split|side:1",
            "damage|mon:Emboar,player-2,1|health:131/170",
            "damage|mon:Emboar,player-2,1|health:78/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&mut battle, 1, &expected_logs);
}

#[test]
fn fire_grass_combo_doubles() {
    // Player 1: Emboar (0), Serperior (1).
    let mut team_1 = team().unwrap();
    team_1.members = vec![team_1.members[0].clone(), team_1.members[1].clone()];
    // Player 2: Emboar (0), Samurott (2). Samurott resists Fire but takes Sea of Fire damage.
    let mut team_2 = team().unwrap();
    team_2.members = vec![team_2.members[0].clone(), team_2.members[2].clone()];

    let mut battle = make_battle(BattleType::Doubles, 0, team_1, team_2).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Serperior,player-1,2|name:Grass Pledge|target:Emboar,player-2,1",
            "waiting|mon:Serperior,player-1,2|on:Emboar,player-1,1",
            "move|mon:Emboar,player-1,1|name:Fire Pledge|target:Samurott,player-2,2|from:move:Grass Pledge",
            "combine",
            "resisted|mon:Samurott,player-2,2",
            "split|side:1",
            "damage|mon:Samurott,player-2,2|health:87/155",
            "damage|mon:Samurott,player-2,2|health:57/100",
            "sidestart|side:1|move:Fire Pledge",
            "split|side:1",
            "damage|mon:Samurott,player-2,2|from:move:Fire Pledge|health:68/155",
            "damage|mon:Samurott,player-2,2|from:move:Fire Pledge|health:44/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&mut battle, 1, &expected_logs);
}

#[test]
fn grass_water_combo_doubles() {
    let mut team_1 = team().unwrap();
    team_1.members = vec![team_1.members[1].clone(), team_1.members[2].clone()];
    // Swamp reduces speed to 1/4, allowing slower Samurott (70) to outspeed faster Serperior (113).
    let mut team_2 = team().unwrap();
    team_2.members = vec![team_2.members[1].clone(), team_2.members[2].clone()];

    let mut battle = make_battle(BattleType::Doubles, 0, team_1, team_2).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Serperior,player-1,1|name:Grass Pledge|target:Samurott,player-2,2",
            "waiting|mon:Serperior,player-1,1|on:Samurott,player-1,2",
            "move|mon:Samurott,player-1,2|name:Water Pledge|target:Serperior,player-2,1|from:move:Grass Pledge",
            "combine",
            "resisted|mon:Serperior,player-2,1",
            "split|side:1",
            "damage|mon:Serperior,player-2,1|health:81/135",
            "damage|mon:Serperior,player-2,1|health:60/100",
            "sidestart|side:1|move:Grass Pledge",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Samurott,player-1,2|name:Water Pledge|target:Serperior,player-2,1",
            "resisted|mon:Serperior,player-2,1",
            "split|side:1",
            "damage|mon:Serperior,player-2,1|health:54/135",
            "damage|mon:Serperior,player-2,1|health:40/100",
            "move|mon:Serperior,player-2,1|name:Grass Pledge|target:Serperior,player-1,1",
            "resisted|mon:Serperior,player-1,1",
            "split|side:0",
            "damage|mon:Serperior,player-1,1|health:117/135",
            "damage|mon:Serperior,player-1,1|health:87/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&mut battle, 1, &expected_logs);
}

#[test]
fn water_fire_combo_doubles() {
    let mut team_1 = team().unwrap();
    team_1.members = vec![team_1.members[2].clone(), team_1.members[0].clone()];
    // Emboar (0) has Charge Beam (70% chance). Rainbow boosts secondary effects.
    let mut team_2 = team().unwrap();
    team_2.members = vec![team_2.members[2].clone(), team_2.members[0].clone()];

    let mut battle = make_battle(BattleType::Doubles, 0, team_1, team_2).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    // Rainbow doubles secondary effect chances, guaranteeing Charge Beam's 70% boost.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 2,2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Samurott,player-1,1|name:Water Pledge|target:Emboar,player-2,2",
            "waiting|mon:Samurott,player-1,1|on:Emboar,player-1,2",
            "move|mon:Emboar,player-1,2|name:Fire Pledge|target:Samurott,player-2,1|from:move:Water Pledge",
            "combine",
            "resisted|mon:Samurott,player-2,1",
            "split|side:1",
            "damage|mon:Samurott,player-2,1|health:87/155",
            "damage|mon:Samurott,player-2,1|health:57/100",
            "sidestart|side:1|move:Water Pledge",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Emboar,player-1,2|name:Charge Beam|target:Emboar,player-2,2",
            "split|side:1",
            "damage|mon:Emboar,player-2,2|health:139/170",
            "damage|mon:Emboar,player-2,2|health:82/100",
            "boost|mon:Emboar,player-1,2|stat:spa|by:1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&mut battle, 1, &expected_logs);
}

#[test]
fn triples_combo_interaction() {
    let team = team().unwrap();
    let mut battle = make_battle(BattleType::Triples, 0, team.clone(), team.clone()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;move 0,2;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;pass;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Serperior,player-1,2|name:Grass Pledge|target:Serperior,player-2,2",
            "waiting|mon:Serperior,player-1,2|on:Samurott,player-1,3",
            "move|mon:Samurott,player-1,3|name:Water Pledge|target:Serperior,player-2,2|from:move:Grass Pledge",
            "combine",
            "resisted|mon:Serperior,player-2,2",
            "split|side:1",
            "damage|mon:Serperior,player-2,2|health:81/135",
            "damage|mon:Serperior,player-2,2|health:60/100",
            "sidestart|side:1|move:Grass Pledge",
            "move|mon:Emboar,player-1,1|name:Fire Pledge|target:Serperior,player-2,2",
            "supereffective|mon:Serperior,player-2,2",
            "split|side:1",
            "damage|mon:Serperior,player-2,2|health:0",
            "damage|mon:Serperior,player-2,2|health:0",
            "faint|mon:Serperior,player-2,2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&mut battle, 1, &expected_logs);
}

#[test]
fn gems_are_not_consumed_by_pledge_moves() {
    let mut team = team().unwrap();
    team.members[0].item = Some("Fire Gem".to_owned());

    let mut battle = make_battle(BattleType::Singles, 0, team.clone(), team.clone()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Emboar,player-1,1|name:Fire Pledge|target:Emboar,player-2,1",
            "resisted|mon:Emboar,player-2,1",
            "split|side:1",
            "damage|mon:Emboar,player-2,1|health:131/170",
            "damage|mon:Emboar,player-2,1|health:78/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Emboar,player-1,1|name:Flamethrower|target:Emboar,player-2,1",
            "itemend|mon:Emboar,player-1,1|item:Fire Gem",
            "resisted|mon:Emboar,player-2,1",
            "split|side:1",
            "damage|mon:Emboar,player-2,1|health:78/170",
            "damage|mon:Emboar,player-2,1|health:46/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&mut battle, 1, &expected_logs);
}
