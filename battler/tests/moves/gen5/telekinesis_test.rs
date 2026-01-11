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
                    "name": "Musharna",
                    "species": "Musharna",
                    "ability": "No Ability",
                    "moves": [
                        "Telekinesis",
                        "Earth Power",
                        "Hypnosis",
                        "Gravity",
                        "Work Up",
                        "Double Team"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn diglett_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Diglett",
                    "species": "Diglett",
                    "ability": "No Ability",
                    "moves": [
                        "Growl"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn gengar_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gengar",
                    "species": "Gengar",
                    "ability": "No Ability",
                    "item": "Gengarite",
                    "moves": [
                        "Shadow Ball"
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
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
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
fn telekinesis_grants_ground_immunity() {
    let mut battle = make_battle(BattleType::Singles, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Musharna uses Telekinesis on Musharna.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Musharna uses Earth Power on Musharna. Should be immune.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 3: Telekinesis continues. Musharna uses Earth Power on Musharna. Should be immune.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 4: Telekinesis ends. Musharna uses Earth Power on Musharna. Should hit.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Musharna,player-1,1|name:Telekinesis|target:Musharna,player-2,1",
            "start|mon:Musharna,player-2,1|move:Telekinesis",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Musharna,player-1,1|name:Earth Power|noanim",
            "immune|mon:Musharna,player-2,1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Musharna,player-1,1|name:Earth Power|noanim",
            "immune|mon:Musharna,player-2,1",
            "end|mon:Musharna,player-2,1|move:Telekinesis",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Musharna,player-1,1|name:Earth Power|target:Musharna,player-2,1",
            "split|side:1",
            "damage|mon:Musharna,player-2,1|health:132/176",
            "damage|mon:Musharna,player-2,1|health:75/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn telekinesis_bypasses_accuracy() {
    let mut battle = make_battle(BattleType::Singles, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: P2 boosts evasion by 6 stages. P1 passes.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));

    // Turn 7: P1 uses Telekinesis on P2.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 8: P1 uses Hypnosis (60% acc) on P2. Should hit despite +6 evasion.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Musharna,player-1,1|name:Telekinesis|target:Musharna,player-2,1",
            "start|mon:Musharna,player-2,1|move:Telekinesis",
            "residual",
            "turn|turn:8",
            "continue",
            "move|mon:Musharna,player-1,1|name:Hypnosis|target:Musharna,player-2,1",
            "status|mon:Musharna,player-2,1|status:Sleep",
            "residual",
            "turn|turn:9"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 7, &expected_logs);
}

#[test]
fn gravity_removes_telekinesis() {
    let mut battle = make_battle(BattleType::Singles, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Musharna uses Telekinesis.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Musharna uses Gravity. Telekinesis ends.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 3: Musharna uses Telekinesis. Fails because Gravity is active.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0"),
        Err(err) if format!("{err:?}").contains("Musharna's Telekinesis is disabled")
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Musharna,player-1,1|name:Telekinesis|target:Musharna,player-2,1",
            "start|mon:Musharna,player-2,1|move:Telekinesis",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Musharna,player-1,1|name:Gravity",
            "fieldstart|move:Gravity",
            "activate|mon:Musharna,player-2,1|move:Gravity",
            "end|mon:Musharna,player-2,1|move:Telekinesis",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn telekinesis_fails_against_diglett() {
    let mut battle = make_battle(
        BattleType::Singles,
        0,
        team().unwrap(),
        diglett_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Musharna uses Telekinesis on Diglett. Fails.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Musharna,player-1,1|name:Telekinesis|noanim",
            "immune|mon:Diglett,player-2,1",
            "fail|mon:Musharna,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn telekinesis_ends_on_mega_evolution_to_gengar() {
    let mut battle = make_battle(
        BattleType::Singles,
        0,
        team().unwrap(),
        gengar_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Musharna uses Telekinesis on Gengar. Effect starts.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Gengar Mega Evolves. Telekinesis ends.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,mega"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Musharna,player-1,1|name:Telekinesis|target:Gengar,player-2,1",
            "start|mon:Gengar,player-2,1|move:Telekinesis",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:1",
            "specieschange|player:player-2|position:1|name:Gengar|health:120/120|species:Gengar-Mega|level:50|gender:U",
            "specieschange|player:player-2|position:1|name:Gengar|health:100/100|species:Gengar-Mega|level:50|gender:U",
            "mega|mon:Gengar,player-2,1|species:Gengar-Mega|from:item:Gengarite",
            "end|mon:Gengar,player-2,1|move:Telekinesis",
            "move|mon:Gengar,player-2,1|name:Shadow Ball|target:Musharna,player-1,1",
            "supereffective|mon:Musharna,player-1,1",
            "split|side:0",
            "damage|mon:Musharna,player-1,1|health:0",
            "damage|mon:Musharna,player-1,1|health:0",
            "faint|mon:Musharna,player-1,1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
