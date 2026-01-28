use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    WildPlayerOptions,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn bidoof() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bidoof",
                    "species": "Bidoof",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Thunder Wave"
                    ],
                    "nature": "Hardy",
                    "level": 50
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .add_wild_mon_to_side_2("wild", "Wild", WildPlayerOptions::default())
        .with_team("protagonist", team_1)
        .with_team("wild", team_2)
        .build(static_local_data_store())
}

#[test]
fn regular_ball_does_not_heal_caught_mon() {
    let mut battle = make_battle(0, bidoof().unwrap(), bidoof().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item pokeball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bidoof,protagonist,1|name:Tackle|target:Bidoof,wild,1",
            "split|side:1",
            "damage|mon:Bidoof,wild,1|health:89/119",
            "damage|mon:Bidoof,wild,1|health:75/100",
            "move|mon:Bidoof,wild,1|name:Tackle|target:Bidoof,protagonist,1",
            "split|side:0",
            "damage|mon:Bidoof,protagonist,1|health:92/119",
            "damage|mon:Bidoof,protagonist,1|health:78/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Bidoof,protagonist,1|name:Thunder Wave|target:Bidoof,wild,1",
            "status|mon:Bidoof,wild,1|status:Paralysis",
            "residual",
            "turn|turn:3",
            "continue",
            "useitem|player:protagonist|name:Poké Ball|target:Bidoof,wild,1",
            "catch|player:protagonist|mon:Bidoof,wild,1|item:Poké Ball|shakes:4",
            "exp|mon:Bidoof,protagonist,1|exp:501",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    assert_matches::assert_matches!(battle.player_data("protagonist"), Ok(data) => {
        assert_matches::assert_matches!(data.caught.get(0), Some(caught) => {
            assert_eq!(caught.hp, 89);
            assert_eq!(caught.status, Some("par".to_owned()));
            assert_eq!(caught.moves[0].pp, 34);
        });
    });
}

#[test]
fn heal_ball_heals_caught_mon() {
    let mut battle = make_battle(0, bidoof().unwrap(), bidoof().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item healball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bidoof,protagonist,1|name:Tackle|target:Bidoof,wild,1",
            "split|side:1",
            "damage|mon:Bidoof,wild,1|health:89/119",
            "damage|mon:Bidoof,wild,1|health:75/100",
            "move|mon:Bidoof,wild,1|name:Tackle|target:Bidoof,protagonist,1",
            "split|side:0",
            "damage|mon:Bidoof,protagonist,1|health:92/119",
            "damage|mon:Bidoof,protagonist,1|health:78/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Bidoof,protagonist,1|name:Thunder Wave|target:Bidoof,wild,1",
            "status|mon:Bidoof,wild,1|status:Paralysis",
            "residual",
            "turn|turn:3",
            "continue",
            "useitem|player:protagonist|name:Heal Ball|target:Bidoof,wild,1",
            "catch|player:protagonist|mon:Bidoof,wild,1|item:Heal Ball|shakes:4",
            "exp|mon:Bidoof,protagonist,1|exp:501",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    assert_matches::assert_matches!(battle.player_data("protagonist"), Ok(data) => {
        assert_matches::assert_matches!(data.caught.get(0), Some(caught) => {
            assert_eq!(caught.hp, caught.stats.hp);
            assert_eq!(caught.status, None);
            assert_eq!(caught.moves[0].pp, 35);
        });
    });
}
