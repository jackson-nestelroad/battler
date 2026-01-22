use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    static_local_data_store,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pansear",
                    "species": "Pansear",
                    "ability": "No Ability",
                    "moves": ["Incinerate"],
                    "level": 50
                },
                {
                    "name": "Pansear",
                    "species": "Pansear",
                    "ability": "No Ability",
                    "moves": [],
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(seed: u64) -> TestBattleBuilder {
    TestBattleBuilder::new()
        .with_seed(seed)
        .with_battle_type(BattleType::Singles)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
}

#[test]
fn incinerate_destroys_berry() {
    let mut battle = make_battle(0)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team())
        .with_team("player-2", team())
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Give target an Oran Berry.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass"),
        Ok(())
    );
    // Cheat to set the item.
    let item_id = battler::Id::from("oranberry");
    let player_index = battle.internal.players().find(|p| p.id == "player-2").unwrap().index;
    let mon_handle = *battle.internal.player(player_index).unwrap().mon_handles().next().unwrap();
    {
        let mut context = battle.internal.context();
        let mut context = context.mon_context(mon_handle).unwrap();
        context.mon_mut().item = Some(item_id);
    }

    // Incinerate.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pansear,player-1,1|name:Incinerate",
            "resisted|mon:Pansear,player-2,1",
            "split|side:1",
            ["damage", "mon:Pansear,player-2,1"],
            ["damage", "mon:Pansear,player-2,1"],
            "itemend|mon:Pansear,player-2,1|item:Oran Berry|from:move:Incinerate|of:Pansear,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn incinerate_destroys_gem() {
    let mut battle = make_battle(0)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team())
        .with_team("player-2", team())
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Give target a Fire Gem.
    let item_id = battler::Id::from("firegem");
    let player_index = battle.internal.players().find(|p| p.id == "player-2").unwrap().index;
    let mon_handle = *battle.internal.player(player_index).unwrap().mon_handles().next().unwrap();
    {
        let mut context = battle.internal.context();
        let mut context = context.mon_context(mon_handle).unwrap();
        context.mon_mut().item = Some(item_id);
    }

    // Incinerate.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pansear,player-1,1|name:Incinerate",
            "resisted|mon:Pansear,player-2,1",
            "split|side:1",
            ["damage", "mon:Pansear,player-2,1"],
            ["damage", "mon:Pansear,player-2,1"],
            "itemend|mon:Pansear,player-2,1|item:Fire Gem|from:move:Incinerate|of:Pansear,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn incinerate_does_not_destroy_other_items() {
    let mut battle = make_battle(0)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team())
        .with_team("player-2", team())
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Give target Leftovers.
    let item_id = battler::Id::from("leftovers");
    let player_index = battle.internal.players().find(|p| p.id == "player-2").unwrap().index;
    let mon_handle = *battle.internal.player(player_index).unwrap().mon_handles().next().unwrap();
    {
        let mut context = battle.internal.context();
        let mut context = context.mon_context(mon_handle).unwrap();
        context.mon_mut().item = Some(item_id);
    }

    // Incinerate.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Item should NOT be destroyed.
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pansear,player-1,1|name:Incinerate",
            "resisted|mon:Pansear,player-2,1",
            "split|side:1",
            ["damage", "mon:Pansear,player-2,1"],
            ["damage", "mon:Pansear,player-2,1"],
            "split|side:1",
            ["heal", "mon:Pansear,player-2,1"],
            ["heal", "mon:Pansear,player-2,1"],
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
