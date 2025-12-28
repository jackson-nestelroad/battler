use battler_ai::random::Random;

use crate::scenario::Scenario;

#[tokio::test(flavor = "multi_thread")]
async fn completes_battle() {
    let scenario = Scenario::from_scenarios_dir("simple_starter_battle_damage_only.json")
        .await
        .unwrap();
    let join_handle_1 = scenario
        .run_ai("player-1", Random::default())
        .await
        .unwrap();
    let join_handle_2 = scenario
        .run_ai("player-2", Random::default())
        .await
        .unwrap();
    assert_matches::assert_matches!(join_handle_1.await, Ok(Ok(())));
    assert_matches::assert_matches!(join_handle_2.await, Ok(Ok(())));
}
