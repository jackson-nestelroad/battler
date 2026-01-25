use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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
                    "name": "Zekrom",
                    "species": "Zekrom",
                    "ability": "No Ability",
                    "moves": [
                        "Fusion Bolt",
                        "Fusion Flare",
                        "Splash",
                        "Thunder Wave"
                    ],
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
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn fusion_bolt_and_flare_double_damage_sequentially() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Mon 1 Bolt (Standard), Mon 2 Flare (Boosted).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    // Turn 2: Both heal.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "item maxpotion, -1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "item maxpotion, -1"),
        Ok(())
    );

    // Turn 3: Mon 1 Flare (Standard), Mon 2 Bolt (Boosted).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // Turn 4: Sequence broken by Splash.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    // Turn 5: Sequence restored.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 6: Persistence through failure (Thunder Wave on Electric type).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Zekrom,player-1,1|name:Fusion Bolt|target:Zekrom,player-2,1",
            "resisted|mon:Zekrom,player-2,1",
            "split|side:1",
            "damage|mon:Zekrom,player-2,1|health:271/310",
            "damage|mon:Zekrom,player-2,1|health:88/100",
            "move|mon:Zekrom,player-2,1|name:Fusion Flare|target:Zekrom,player-1,1",
            "resisted|mon:Zekrom,player-1,1",
            "split|side:0",
            "damage|mon:Zekrom,player-1,1|health:209/310",
            "damage|mon:Zekrom,player-1,1|health:68/100",
            "residual",
            "turn|turn:2",
            "continue",
            "useitem|player:player-1|name:Max Potion|target:Zekrom,player-1,1",
            "split|side:0",
            "heal|mon:Zekrom,player-1,1|from:item:Max Potion|health:310/310",
            "heal|mon:Zekrom,player-1,1|from:item:Max Potion|health:100/100",
            "useitem|player:player-2|name:Max Potion|target:Zekrom,player-2,1",
            "split|side:1",
            "heal|mon:Zekrom,player-2,1|from:item:Max Potion|health:310/310",
            "heal|mon:Zekrom,player-2,1|from:item:Max Potion|health:100/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Zekrom,player-1,1|name:Fusion Flare|target:Zekrom,player-2,1",
            "resisted|mon:Zekrom,player-2,1",
            "split|side:1",
            "damage|mon:Zekrom,player-2,1|health:259/310",
            "damage|mon:Zekrom,player-2,1|health:84/100",
            "move|mon:Zekrom,player-2,1|name:Fusion Bolt|target:Zekrom,player-1,1",
            "resisted|mon:Zekrom,player-1,1",
            "split|side:0",
            "damage|mon:Zekrom,player-1,1|health:231/310",
            "damage|mon:Zekrom,player-1,1|health:75/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Zekrom,player-1,1|name:Splash|target:Zekrom,player-1,1",
            "activate|move:Splash",
            "move|mon:Zekrom,player-2,1|name:Fusion Flare|target:Zekrom,player-1,1",
            "resisted|mon:Zekrom,player-1,1",
            "split|side:0",
            "damage|mon:Zekrom,player-1,1|health:180/310",
            "damage|mon:Zekrom,player-1,1|health:59/100",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Zekrom,player-1,1|name:Fusion Bolt|target:Zekrom,player-2,1",
            "resisted|mon:Zekrom,player-2,1",
            "split|side:1",
            "damage|mon:Zekrom,player-2,1|health:180/310",
            "damage|mon:Zekrom,player-2,1|health:59/100",
            "residual",
            "turn|turn:6",
            "continue",
            "move|mon:Zekrom,player-1,1|name:Thunder Wave|noanim",
            "immune|mon:Zekrom,player-2,1",
            "move|mon:Zekrom,player-2,1|name:Fusion Flare|target:Zekrom,player-1,1",
            "resisted|mon:Zekrom,player-1,1",
            "split|side:0",
            "damage|mon:Zekrom,player-1,1|health:79/310",
            "damage|mon:Zekrom,player-1,1|health:26/100",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
