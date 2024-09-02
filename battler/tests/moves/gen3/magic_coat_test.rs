use battler::{
    battle::{
        BattleType,
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
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Grumpig",
                    "species": "Grumpig",
                    "ability": "No Ability",
                    "moves": [
                        "Magic Coat"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Seviper",
                    "species": "Seviper",
                    "ability": "No Ability",
                    "moves": [
                        "Will-O-Wisp",
                        "Spikes"
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
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn magic_coat_reflects_status_moves_for_the_turn() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_eq!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );
    assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass;move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Grumpig,player-1,1|name:Magic Coat|target:Grumpig,player-1,1",
            "singleturn|mon:Grumpig,player-1,1|move:Magic Coat",
            "move|mon:Seviper,player-2,2|name:Will-O-Wisp|noanim",
            "activate|mon:Grumpig,player-1,1|move:Magic Coat",
            "move|mon:Grumpig,player-1,1|name:Will-O-Wisp|target:Seviper,player-2,2",
            "status|mon:Seviper,player-2,2|status:Burn",
            "split|side:1",
            "damage|mon:Seviper,player-2,2|from:status:Burn|health:125/133",
            "damage|mon:Seviper,player-2,2|from:status:Burn|health:94/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Grumpig,player-1,1|name:Magic Coat|target:Grumpig,player-1,1",
            "singleturn|mon:Grumpig,player-1,1|move:Magic Coat",
            "move|mon:Seviper,player-2,2|name:Spikes|noanim",
            "activate|mon:Grumpig,player-1,1|move:Magic Coat",
            "move|mon:Grumpig,player-1,1|name:Spikes",
            "sidestart|side:1|move:Spikes",
            "split|side:1",
            "damage|mon:Seviper,player-2,2|from:status:Burn|health:117/133",
            "damage|mon:Seviper,player-2,2|from:status:Burn|health:88/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
