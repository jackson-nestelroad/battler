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
                    "name": "Delphox",
                    "species": "Delphox",
                    "ability": "Magician",
                    "moves": [
                        "Tackle",
                        "Earthquake"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Chesnaught",
                    "species": "Chesnaught",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Greninja",
                    "species": "Greninja",
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

fn make_battle(
    seed: u64,
    battle_type: BattleType,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
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
fn magician_steals_target_item_after_damage() {
    let mut team_2 = team().unwrap();
    team_2.members[0].item = Some("Leppa Berry".to_owned());
    let mut battle = make_battle(0, BattleType::Singles, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Delphox,player-1,1|name:Tackle|target:Delphox,player-2,1",
            "split|side:1",
            "damage|mon:Delphox,player-2,1|health:228/260",
            "damage|mon:Delphox,player-2,1|health:88/100",
            "itemend|mon:Delphox,player-2,1|item:Leppa Berry|from:ability:Magician|of:Delphox,player-1,1",
            "item|mon:Delphox,player-1,1|item:Leppa Berry|from:ability:Magician",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn magician_steals_item_from_fastest_target() {
    let mut team_2 = team().unwrap();
    team_2.members[0].item = Some("Leppa Berry".to_owned());
    team_2.members[1].item = Some("Oran Berry".to_owned());
    team_2.members[2].item = Some("Cheri Berry".to_owned());
    let mut battle = make_battle(0, BattleType::Triples, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1;pass;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;pass;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Delphox,player-1,1|name:Earthquake|spread:Chesnaught,player-1,2;Chesnaught,player-2,2;Greninja,player-2,3",
            "resisted|mon:Chesnaught,player-1,2",
            "resisted|mon:Chesnaught,player-2,2",
            "split|side:0",
            "damage|mon:Chesnaught,player-1,2|health:268/286",
            "damage|mon:Chesnaught,player-1,2|health:94/100",
            "split|side:1",
            "damage|mon:Chesnaught,player-2,2|health:270/286",
            "damage|mon:Chesnaught,player-2,2|health:95/100",
            "split|side:1",
            "damage|mon:Greninja,player-2,3|health:198/254",
            "damage|mon:Greninja,player-2,3|health:78/100",
            "itemend|mon:Greninja,player-2,3|item:Cheri Berry|from:ability:Magician|of:Delphox,player-1,1",
            "item|mon:Delphox,player-1,1|item:Cheri Berry|from:ability:Magician",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn magician_steals_item_from_other_target_when_item_is_untakable() {
    let mut team_2 = team().unwrap();
    team_2.members[0].item = Some("Leppa Berry".to_owned());
    team_2.members[1].item = Some("Oran Berry".to_owned());
    team_2.members[2].item = Some("Waterium Z".to_owned());
    let mut battle = make_battle(0, BattleType::Triples, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1;pass;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;pass;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Delphox,player-1,1|name:Earthquake|spread:Chesnaught,player-1,2;Chesnaught,player-2,2;Greninja,player-2,3",
            "resisted|mon:Chesnaught,player-1,2",
            "resisted|mon:Chesnaught,player-2,2",
            "split|side:0",
            "damage|mon:Chesnaught,player-1,2|health:268/286",
            "damage|mon:Chesnaught,player-1,2|health:94/100",
            "split|side:1",
            "damage|mon:Chesnaught,player-2,2|health:270/286",
            "damage|mon:Chesnaught,player-2,2|health:95/100",
            "split|side:1",
            "damage|mon:Greninja,player-2,3|health:198/254",
            "damage|mon:Greninja,player-2,3|health:78/100",
            "itemend|mon:Chesnaught,player-2,2|item:Oran Berry|from:ability:Magician|of:Delphox,player-1,1",
            "item|mon:Delphox,player-1,1|item:Oran Berry|from:ability:Magician",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
