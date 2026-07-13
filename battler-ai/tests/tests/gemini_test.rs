use battler_ai::gemini::Gemini;

use crate::scenario::Scenario;

#[tokio::test(flavor = "multi_thread")]
async fn picks_valid_move() {
    battler_test_utils::collect_logs();
    let scenario = Scenario::from_scenarios_dir("simple_starter_battle.json")
        .await
        .unwrap();
    let mut gemini = Gemini::default();
    scenario
        .record_explanations("player-2", gemini.explanations())
        .await;
    assert_matches::assert_matches!(scenario.validate_expected_result(&mut gemini).await, Ok(()));
}

#[tokio::test(flavor = "multi_thread")]
async fn picks_valid_move_for_double_battle() {
    battler_test_utils::collect_logs();
    let scenario = Scenario::from_scenarios_dir("simple_double_battle.json")
        .await
        .unwrap();
    let mut gemini = Gemini::default();
    scenario
        .record_explanations("player-2", gemini.explanations())
        .await;
    assert_matches::assert_matches!(scenario.validate_expected_result(&mut gemini).await, Ok(()));
}

#[tokio::test(flavor = "multi_thread")]
async fn competes_in_fuzz_test_battle() {
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
    let scenario = Scenario::from_options(options, store)
        .await
        .unwrap()
        .with_error_on_exceeded_attempts(true);
    let gemini_1 = Gemini::default();
    let gemini_2 = Gemini::default();
    scenario
        .record_explanations("player-1", gemini_1.explanations())
        .await;
    scenario
        .record_explanations("player-2", gemini_2.explanations())
        .await;
    let join_handle_1 = scenario
        .run_ai_for_requests("player-1", gemini_1, 2)
        .await
        .unwrap();
    let join_handle_2 = scenario
        .run_ai_for_requests("player-2", gemini_2, 2)
        .await
        .unwrap();

    assert_matches::assert_matches!(join_handle_1.await, Ok(Ok(())));
    assert_matches::assert_matches!(join_handle_2.await, Ok(Ok(())));
}
