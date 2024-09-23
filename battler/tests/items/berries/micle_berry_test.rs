use battler::{
    battle::{
        BattleType,
        CoreBattleEngineRandomizeBaseDamage,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    common::{
        Error,
        WrapResultError,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
    LogMatch,
    TestBattleBuilder,
};

fn cryogonal() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Cryogonal",
                    "species": "Cryogonal",
                    "ability": "No Ability",
                    "moves": [
                        "Blizzard",
                        "Flamethrower"
                    ],
                    "nature": "Naughty",
                    "level": 50,
                    "item": "Micle Berry"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Min)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn micle_berry_boosts_accuracy_for_next_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, cryogonal().unwrap(), cryogonal().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 80)]);

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 80)]);

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cryogonal,player-2,1|name:Flamethrower|target:Cryogonal,player-1,1",
            "supereffective|mon:Cryogonal,player-1,1",
            "split|side:0",
            "damage|mon:Cryogonal,player-1,1|health:28/140",
            "damage|mon:Cryogonal,player-1,1|health:20/100",
            "itemend|mon:Cryogonal,player-1,1|item:Micle Berry|eat",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Cryogonal,player-1,1|name:Blizzard",
            "resisted|mon:Cryogonal,player-2,1",
            "split|side:1",
            "damage|mon:Cryogonal,player-2,1|health:115/140",
            "damage|mon:Cryogonal,player-2,1|health:83/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Cryogonal,player-1,1|name:Blizzard|noanim",
            "miss|mon:Cryogonal,player-2,1",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}