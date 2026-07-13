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

#[tokio::test(flavor = "multi_thread")]
async fn participates_in_fuzz_test_battle() {
    battler_test_utils::collect_logs();
    let store = battler_test_utils::static_local_data_store();
    let options = battler_fuzz_test_generator::generate_random_battle(store, None).unwrap();
    let seed = options.seed.unwrap_or(0);
    log::info!(
        "Fuzz test {} started with seed: {seed}",
        std::thread::current()
            .name()
            .unwrap_or("fuzz_test")
            .to_string()
    );
    let scenario = Scenario::from_options(options, store).await.unwrap();
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
