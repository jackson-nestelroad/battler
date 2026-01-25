use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn pikachu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn scale_team(team: TeamData, size: usize) -> TeamData {
    let mut new_team = team.clone();
    for _ in 0..(size - 1) {
        new_team.members.push(new_team.members[0].clone());
    }
    new_team
}

#[test]
fn auto_shifts_to_center_in_triples() {
    let mut battle = TestBattleBuilder::new()
        .with_battle_type(BattleType::Triples)
        .with_seed(0)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", scale_team(pikachu().unwrap(), 3))
        .with_team("player-2", scale_team(pikachu().unwrap(), 3))
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice(
            "player-1",
            "pass;item selfdestructbutton;item selfdestructbutton"
        ),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice(
            "player-2",
            "pass;item selfdestructbutton;item selfdestructbutton"
        ),
        Ok(())
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:player-1|name:Self-Destruct Button|target:Pikachu,player-1,2",
            "faint|mon:Pikachu,player-1,2",
            "useitem|player:player-1|name:Self-Destruct Button|target:Pikachu,player-1,3",
            "faint|mon:Pikachu,player-1,3",
            "useitem|player:player-2|name:Self-Destruct Button|target:Pikachu,player-2,2",
            "faint|mon:Pikachu,player-2,2",
            "useitem|player:player-2|name:Self-Destruct Button|target:Pikachu,player-2,3",
            "faint|mon:Pikachu,player-2,3",
            "residual",
            "swap|mon:Pikachu,player-1,1|position:2|from:Auto-Shift",
            "swap|mon:Pikachu,player-2,1|position:2|from:Auto-Shift",
            "turn|turn:2",
            "continue",
            "move|mon:Pikachu,player-1,2|name:Tackle|target:Pikachu,player-2,2",
            "split|side:1",
            "damage|mon:Pikachu,player-2,2|health:71/95",
            "damage|mon:Pikachu,player-2,2|health:75/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn does_not_auto_shift_to_center_in_triples_if_already_shifted() {
    let mut battle = TestBattleBuilder::new()
        .with_battle_type(BattleType::Triples)
        .with_seed(0)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", scale_team(pikachu().unwrap(), 3))
        .with_team("player-2", scale_team(pikachu().unwrap(), 3))
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice(
            "player-1",
            "shift;item selfdestructbutton;item selfdestructbutton"
        ),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice(
            "player-2",
            "move 0,2;item selfdestructbutton;item selfdestructbutton"
        ),
        Ok(())
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:player-1|name:Self-Destruct Button|target:Pikachu,player-1,2",
            "faint|mon:Pikachu,player-1,2",
            "useitem|player:player-1|name:Self-Destruct Button|target:Pikachu,player-1,3",
            "faint|mon:Pikachu,player-1,3",
            "useitem|player:player-2|name:Self-Destruct Button|target:Pikachu,player-2,2",
            "faint|mon:Pikachu,player-2,2",
            "useitem|player:player-2|name:Self-Destruct Button|target:Pikachu,player-2,3",
            "faint|mon:Pikachu,player-2,3",
            "swap|mon:Pikachu,player-1,1|position:2|from:Player Choice",
            "move|mon:Pikachu,player-2,1|name:Tackle|target:Pikachu,player-1,2",
            "split|side:0",
            "damage|mon:Pikachu,player-1,2|health:71/95",
            "damage|mon:Pikachu,player-1,2|health:75/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pikachu,player-2,1|name:Tackle|target:Pikachu,player-1,2",
            "split|side:0",
            "damage|mon:Pikachu,player-1,2|health:49/95",
            "damage|mon:Pikachu,player-1,2|health:52/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn does_not_auto_shift_to_center_in_triples_with_extended_adjacency() {
    let mut battle = TestBattleBuilder::new()
        .with_battle_type(BattleType::Triples)
        .with_seed(0)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_adjacency_reach(3)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", scale_team(pikachu().unwrap(), 3))
        .with_team("player-2", scale_team(pikachu().unwrap(), 3))
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice(
            "player-1",
            "pass;item selfdestructbutton;item selfdestructbutton"
        ),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice(
            "player-2",
            "pass;item selfdestructbutton;item selfdestructbutton"
        ),
        Ok(())
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:player-1|name:Self-Destruct Button|target:Pikachu,player-1,2",
            "faint|mon:Pikachu,player-1,2",
            "useitem|player:player-1|name:Self-Destruct Button|target:Pikachu,player-1,3",
            "faint|mon:Pikachu,player-1,3",
            "useitem|player:player-2|name:Self-Destruct Button|target:Pikachu,player-2,2",
            "faint|mon:Pikachu,player-2,2",
            "useitem|player:player-2|name:Self-Destruct Button|target:Pikachu,player-2,3",
            "faint|mon:Pikachu,player-2,3",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:71/95",
            "damage|mon:Pikachu,player-2,1|health:75/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn auto_shifts_player_in_3v3_multi() {
    let mut battle = TestBattleBuilder::new()
        .with_battle_type(BattleType::Multi)
        .with_seed(0)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_1("player-2", "Player 2")
        .add_player_to_side_1("player-3", "Player 3")
        .add_player_to_side_2("player-4", "Player 4")
        .add_player_to_side_2("player-5", "Player 5")
        .add_player_to_side_2("player-6", "Player 6")
        .with_team("player-1", pikachu().unwrap())
        .with_team("player-2", pikachu().unwrap())
        .with_team("player-3", pikachu().unwrap())
        .with_team("player-4", pikachu().unwrap())
        .with_team("player-5", pikachu().unwrap())
        .with_team("player-6", pikachu().unwrap())
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "item selfdestructbutton"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-3", "item selfdestructbutton"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-4", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-5", "item selfdestructbutton"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-6", "item selfdestructbutton"),
        Ok(())
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-4", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:player-2|name:Self-Destruct Button|target:Pikachu,player-2,2",
            "faint|mon:Pikachu,player-2,2",
            "useitem|player:player-3|name:Self-Destruct Button|target:Pikachu,player-3,3",
            "faint|mon:Pikachu,player-3,3",
            "useitem|player:player-5|name:Self-Destruct Button|target:Pikachu,player-5,2",
            "faint|mon:Pikachu,player-5,2",
            "useitem|player:player-6|name:Self-Destruct Button|target:Pikachu,player-6,3",
            "faint|mon:Pikachu,player-6,3",
            "residual",
            "swapplayer|player:player-1|position:1",
            "swapplayer|player:player-4|position:1",
            "turn|turn:2",
            "continue",
            "move|mon:Pikachu,player-1,2|name:Tackle|target:Pikachu,player-4,2",
            "split|side:1",
            "damage|mon:Pikachu,player-4,2|health:71/95",
            "damage|mon:Pikachu,player-4,2|health:75/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn auto_shifts_one_player_in_4v4_multi() {
    let mut battle = TestBattleBuilder::new()
        .with_battle_type(BattleType::Multi)
        .with_seed(0)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_1("player-2", "Player 2")
        .add_player_to_side_1("player-3", "Player 3")
        .add_player_to_side_1("player-4", "Player 4")
        .add_player_to_side_2("player-5", "Player 5")
        .add_player_to_side_2("player-6", "Player 6")
        .add_player_to_side_2("player-7", "Player 7")
        .add_player_to_side_2("player-8", "Player 8")
        .with_team("player-1", pikachu().unwrap())
        .with_team("player-2", pikachu().unwrap())
        .with_team("player-3", pikachu().unwrap())
        .with_team("player-4", pikachu().unwrap())
        .with_team("player-5", pikachu().unwrap())
        .with_team("player-6", pikachu().unwrap())
        .with_team("player-7", pikachu().unwrap())
        .with_team("player-8", pikachu().unwrap())
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-3", "item selfdestructbutton"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-4", "item selfdestructbutton"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-5", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-6", "item selfdestructbutton"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-7", "item selfdestructbutton"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-8", "item selfdestructbutton"),
        Ok(())
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-5", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:player-3|name:Self-Destruct Button|target:Pikachu,player-3,3",
            "faint|mon:Pikachu,player-3,3",
            "useitem|player:player-4|name:Self-Destruct Button|target:Pikachu,player-4,4",
            "faint|mon:Pikachu,player-4,4",
            "useitem|player:player-6|name:Self-Destruct Button|target:Pikachu,player-6,2",
            "faint|mon:Pikachu,player-6,2",
            "useitem|player:player-7|name:Self-Destruct Button|target:Pikachu,player-7,3",
            "faint|mon:Pikachu,player-7,3",
            "useitem|player:player-8|name:Self-Destruct Button|target:Pikachu,player-8,4",
            "faint|mon:Pikachu,player-8,4",
            "residual",
            "swapplayer|player:player-5|position:2",
            "turn|turn:2",
            "continue",
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Pikachu,player-5,3",
            "split|side:1",
            "damage|mon:Pikachu,player-5,3|health:71/95",
            "damage|mon:Pikachu,player-5,3|health:75/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
