use anyhow::Result;
use battler::{
    WrapResultError,
    battle::{
        BattleType,
        CoreBattleEngineRandomizeBaseDamage,
        PublicCoreBattle,
    },
    teams::TeamData,
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
                    "species": "Stoutland",
                    "name": "Stoutland",
                    "moves": ["Retaliate", "Memento", "Recover"],
                    "ability": "No Ability",
                    "level": 50
                },
                {
                    "species": "Stoutland",
                    "name": "Stoutland",
                    "moves": ["Retaliate"],
                    "ability": "No Ability",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(0)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn retaliate_boosted_after_ally_faints() {
    let mut battle = make_battle(team().unwrap(), team().unwrap()).unwrap();
    battle.start().unwrap();

    battle.set_player_choice("player-1", "move 0").unwrap();
    battle.set_player_choice("player-2", "pass").unwrap();

    battle.set_player_choice("player-1", "move 1").unwrap();
    battle.set_player_choice("player-2", "move 2").unwrap();

    battle.set_player_choice("player-1", "switch 1").unwrap();
    battle.set_player_choice("player-1", "move 0").unwrap();
    battle.set_player_choice("player-2", "pass").unwrap();

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Stoutland,player-1,1|name:Retaliate|target:Stoutland,player-2,1",
            "split|side:1",
            "damage|mon:Stoutland,player-2,1|health:87/145",
            "damage|mon:Stoutland,player-2,1|health:60/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Stoutland,player-1,1|name:Memento|target:Stoutland,player-2,1",
            "unboost|mon:Stoutland,player-2,1|stat:atk|by:2",
            "unboost|mon:Stoutland,player-2,1|stat:spa|by:2",
            "faint|mon:Stoutland,player-1,1",
            "move|mon:Stoutland,player-2,1|name:Recover|target:Stoutland,player-2,1",
            "split|side:1",
            "heal|mon:Stoutland,player-2,1|health:145/145",
            "heal|mon:Stoutland,player-2,1|health:100/100",
            "residual",
            "continue",
            "split|side:0",
            ["switch"],
            ["switch"],
            "turn|turn:3",
            "continue",
            "move|mon:Stoutland,player-1,1|name:Retaliate|target:Stoutland,player-2,1",
            "split|side:1",
            "damage|mon:Stoutland,player-2,1|health:31/145",
            "damage|mon:Stoutland,player-2,1|health:22/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
