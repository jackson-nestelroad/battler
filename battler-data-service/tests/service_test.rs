use battler_data_service::{
    BatchQuery,
    BattlerDataService,
    ResourceOptions,
};
use battler_test_utils::static_local_data_store;

#[test]
fn resolves_move_by_id_and_name() {
    let service = BattlerDataService::new(static_local_data_store());

    // By ID
    let by_id = service
        .get_move("thunderbolt", ResourceOptions::default())
        .unwrap();
    assert_matches::assert_matches!(by_id, Some(data) => {
        assert_eq!(data.name, "Thunderbolt");
    });

    // By Name
    let by_name = service
        .get_move("Thunderbolt", ResourceOptions::default())
        .unwrap();
    assert_matches::assert_matches!(by_name, Some(data) => {
        assert_eq!(data.name, "Thunderbolt");
    });
}

#[test]
fn resolves_condition_weather_sandstorm_without_collision() {
    let service = BattlerDataService::new(static_local_data_store());

    // Sandstorm move
    let sandstorm_move = service
        .get_move("sandstorm", ResourceOptions::default())
        .unwrap();
    assert_matches::assert_matches!(sandstorm_move, Some(data) => {
        assert_eq!(data.name, "Sandstorm");
        assert_eq!(data.primary_type.to_string(), "Rock");
    });

    // Sandstorm condition/weather
    let sandstorm_cond = service
        .get_condition("sandstorm", ResourceOptions::default())
        .unwrap();
    assert_matches::assert_matches!(sandstorm_cond, Some(data) => {
        assert_eq!(data.name, "Sandstorm");
        assert_eq!(data.condition_type.to_string(), "Weather");
    });
}

#[test]
fn resolves_resource_aliases() {
    let service = BattlerDataService::new(static_local_data_store());

    // Condition alias: "burn" -> "brn"
    let burn_cond = service
        .get_condition("burn", ResourceOptions::default())
        .unwrap();
    assert_matches::assert_matches!(burn_cond, Some(data) => {
        assert_eq!(data.name, "Burn");
    });

    // Item alias: "leek" -> "stick" (named "Leek")
    let leek_item = service
        .get_item("leek", ResourceOptions::default())
        .unwrap();
    assert_matches::assert_matches!(leek_item, Some(data) => {
        assert_eq!(data.name, "Leek");
    });

    // Species alias: "aegislashshield" -> "aegislash"
    let aegislash = service
        .get_species("aegislashshield", ResourceOptions::default())
        .unwrap();
    assert_matches::assert_matches!(aegislash, Some(data) => {
        assert_eq!(data.name, "Aegislash");
    });
}

#[test]
fn sanitizes_ast_fields_when_include_fxlang_is_false() {
    let service = BattlerDataService::new(static_local_data_store());

    let move_data = service.get_move("fly", ResourceOptions::default()).unwrap();
    assert_matches::assert_matches!(move_data, Some(data) => {
        assert_eq!(data.effect, serde_json::Value::Null);
        assert_eq!(data.condition, serde_json::Value::Null);
    });

    let ability_data = service
        .get_ability("intimidate", ResourceOptions::default())
        .unwrap();
    assert_matches::assert_matches!(ability_data, Some(data) => {
        assert_eq!(data.effect, serde_json::Value::Null);
        assert_eq!(data.condition, serde_json::Value::Null);
    });
}

#[test]
fn preserves_ast_fields_when_include_fxlang_is_true() {
    let service = BattlerDataService::new(static_local_data_store());

    let move_data = service
        .get_move("fly", ResourceOptions {
            include_fxlang: true,
        })
        .unwrap();
    assert_matches::assert_matches!(move_data, Some(data) => {
        assert_ne!(data.effect, serde_json::Value::Null);
        assert_ne!(data.condition, serde_json::Value::Null);
    });

    let ability_data = service
        .get_ability("intimidate", ResourceOptions {
            include_fxlang: true,
        })
        .unwrap();
    assert_matches::assert_matches!(ability_data, Some(data) => {
        assert_ne!(data.effect, serde_json::Value::Null);
    });
}

#[test]
fn batch_query_with_partial_matches() {
    let service = BattlerDataService::new(static_local_data_store());

    let result = service
        .batch(BatchQuery {
            moves: vec!["thunderbolt".to_owned(), "fake_move_123".to_owned()],
            abilities: vec!["overgrow".to_owned()],
            items: vec!["leftovers".to_owned(), "nonexistent_item".to_owned()],
            conditions: vec!["sandstorm".to_owned()],
            species: vec!["pikachu".to_owned()],
            options: ResourceOptions::default(),
        })
        .unwrap();

    assert_matches::assert_matches!(result.moves.get("thunderbolt"), Some(Some(_)));
    assert_matches::assert_matches!(result.moves.get("fake_move_123"), Some(None));
    assert_matches::assert_matches!(result.abilities.get("overgrow"), Some(Some(_)));
    assert_matches::assert_matches!(result.items.get("leftovers"), Some(Some(_)));
    assert_matches::assert_matches!(result.items.get("nonexistent_item"), Some(None));
    assert_matches::assert_matches!(result.conditions.get("sandstorm"), Some(Some(_)));
    assert_matches::assert_matches!(result.species.get("pikachu"), Some(Some(_)));
}
