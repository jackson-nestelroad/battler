use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Raichu",
                    "species": "Raichu",
                    "ability": "No Ability",
                    "moves": [
                        "Light Screen",
                        "Tackle",
                        "Thunderbolt",
                        "Psychic",
                        "Razor Wind"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Alakazam",
                    "species": "Alakazam",
                    "ability": "No Ability",
                    "moves": [
                        "Psychic"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_controlled_rng(true)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn light_screen_halves_special_damage_in_singles() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Singles,
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Raichu,player-1,1|name:Light Screen",
            "sidestart|side:0|move:Light Screen",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Raichu,player-1,1|name:Tackle|target:Raichu,player-2,1",
            "split|side:1",
            "damage|mon:Raichu,player-2,1|health:91/120",
            "damage|mon:Raichu,player-2,1|health:76/100",
            "move|mon:Raichu,player-2,1|name:Tackle|target:Raichu,player-1,1",
            "split|side:0",
            "damage|mon:Raichu,player-1,1|health:91/120",
            "damage|mon:Raichu,player-1,1|health:76/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Raichu,player-1,1|name:Psychic|target:Raichu,player-2,1",
            "split|side:1",
            "damage|mon:Raichu,player-2,1|health:45/120",
            "damage|mon:Raichu,player-2,1|health:38/100",
            "move|mon:Raichu,player-2,1|name:Psychic|target:Raichu,player-1,1",
            "split|side:0",
            "damage|mon:Raichu,player-1,1|health:68/120",
            "damage|mon:Raichu,player-1,1|health:57/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "residual",
            "turn|turn:5",
            ["time"],
            "sideend|side:0|move:Light Screen",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Raichu,player-1,1|name:Psychic|target:Raichu,player-2,1",
            "split|side:1",
            "damage|mon:Raichu,player-2,1|health:0",
            "damage|mon:Raichu,player-2,1|health:0",
            "faint|mon:Raichu,player-2,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn light_screen_applies_two_thirds_special_damage_in_doubles() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Doubles,
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 2,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Raichu,player-1,1|name:Light Screen",
            "sidestart|side:0|move:Light Screen",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Alakazam,player-1,2|name:Psychic|target:Raichu,player-2,1",
            "split|side:1",
            "damage|mon:Raichu,player-2,1|health:20/120",
            "damage|mon:Raichu,player-2,1|health:17/100",
            "move|mon:Alakazam,player-2,2|name:Psychic|target:Raichu,player-1,1",
            "split|side:0",
            "damage|mon:Raichu,player-1,1|health:54/120",
            "damage|mon:Raichu,player-1,1|health:45/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Raichu,player-1,1|name:Tackle|target:Alakazam,player-2,2",
            "split|side:1",
            "damage|mon:Alakazam,player-2,2|health:80/115",
            "damage|mon:Alakazam,player-2,2|health:70/100",
            "move|mon:Raichu,player-2,1|name:Tackle|target:Alakazam,player-1,2",
            "split|side:0",
            "damage|mon:Alakazam,player-1,2|health:80/115",
            "damage|mon:Alakazam,player-1,2|health:70/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Raichu,player-1,1|name:Thunderbolt|target:Alakazam,player-2,2",
            "split|side:1",
            "damage|mon:Alakazam,player-2,2|health:22/115",
            "damage|mon:Alakazam,player-2,2|health:20/100",
            "move|mon:Raichu,player-2,1|name:Thunderbolt|target:Alakazam,player-1,2",
            "split|side:0",
            "damage|mon:Alakazam,player-1,2|health:42/115",
            "damage|mon:Alakazam,player-1,2|health:37/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn critical_hit_bypasses_light_screen() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Singles,
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Raichu,player-1,1|name:Light Screen",
            "sidestart|side:0|move:Light Screen",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Raichu,player-1,1|name:Razor Wind|noanim",
            "prepare|mon:Raichu,player-1,1|move:Razor Wind",
            "move|mon:Raichu,player-2,1|name:Razor Wind|noanim",
            "prepare|mon:Raichu,player-2,1|move:Razor Wind",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Raichu,player-1,1|name:Razor Wind",
            "split|side:1",
            "damage|mon:Raichu,player-2,1|health:79/120",
            "damage|mon:Raichu,player-2,1|health:66/100",
            "move|mon:Raichu,player-2,1|name:Razor Wind",
            "crit|mon:Raichu,player-1,1",
            "split|side:0",
            "damage|mon:Raichu,player-1,1|health:59/120",
            "damage|mon:Raichu,player-1,1|health:50/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
