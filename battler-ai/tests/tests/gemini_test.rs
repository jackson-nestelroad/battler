use battler_ai::gemini::Gemini;

use crate::scenario::Scenario;

#[tokio::test]
async fn picks_valid_move() {
    let scenario = Scenario::from_scenarios_dir("simple_starter_battle.json")
        .await
        .unwrap();
    let mut gemini = Gemini::default();
    assert_matches::assert_matches!(scenario.validate_expected_result(&mut gemini).await, Ok(()));
}

#[tokio::test]
async fn picks_valid_move_for_double_battle() {
    let scenario = Scenario::from_scenarios_dir("simple_double_battle.json")
        .await
        .unwrap();
    let mut gemini = Gemini::default();
    assert_matches::assert_matches!(scenario.validate_expected_result(&mut gemini).await, Ok(()));
}
