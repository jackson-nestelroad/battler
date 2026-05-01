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
                    "name": "Aerodactyl",
                    "species": "Aerodactyl",
                    "ability": "No Ability",
                    "item": "Aerodactylite",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Camerupt",
                    "species": "Camerupt",
                    "ability": "No Ability",
                    "item": "Cameruptite",
                    "moves": [
                        "Speed Swap"
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
        .with_battle_type(BattleType::Doubles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_mega_evolution(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn speed_swap_swaps_speed_and_is_preserved_on_mega_evolution() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1,mega;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Camerupt,player-1,2|name:Speed Swap|target:Aerodactyl,player-2,1",
            "activate|mon:Aerodactyl,player-2,1|move:Speed Swap|of:Camerupt,player-1,2",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:1",
            "specieschange|player:player-2|position:1|name:Aerodactyl|health:270/270|species:Aerodactyl-Mega|level:100|gender:U",
            "specieschange|player:player-2|position:1|name:Aerodactyl|health:100/100|species:Aerodactyl-Mega|level:100|gender:U",
            "mega|mon:Aerodactyl,player-2,1|species:Aerodactyl-Mega|from:item:Aerodactylite",
            "move|mon:Aerodactyl,player-1,1|name:Tackle|target:Aerodactyl,player-2,1",
            "resisted|mon:Aerodactyl,player-2,1",
            "split|side:1",
            "damage|mon:Aerodactyl,player-2,1|health:250/270",
            "damage|mon:Aerodactyl,player-2,1|health:93/100",
            "move|mon:Aerodactyl,player-2,1|name:Tackle|target:Aerodactyl,player-1,1",
            "resisted|mon:Aerodactyl,player-1,1",
            "split|side:0",
            "damage|mon:Aerodactyl,player-1,1|health:230/270",
            "damage|mon:Aerodactyl,player-1,1|health:86/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
