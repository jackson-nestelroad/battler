use alloc::{
    boxed::Box,
    string::String,
    vec::Vec,
};

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{
    BattleState,
    Log,
};

#[wasm_bindgen(typescript_custom_section)]
const TS_IMPORTS: &'static str = r#"
import type { BattleState } from './bindings/BattleState.js';
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "BattleState")]
    pub type BattleStateWasm;
}

/// Returns a new, default BattleState object.
#[wasm_bindgen(js_name = newBattleState)]
pub fn new_battle_state() -> Result<BattleStateWasm, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    let state_js = BattleState::default()
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(state_js.unchecked_into())
}

/// Alters a battle state given an array of log lines.
#[wasm_bindgen(js_name = alterBattleState)]
pub fn alter_battle_state(
    state: &BattleStateWasm,
    log_lines: Box<[String]>,
) -> Result<BattleStateWasm, JsValue> {
    let state_js = JsValue::from(state);
    let state: BattleState =
        serde_wasm_bindgen::from_value(state_js).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let log_lines_refs: Vec<&str> = log_lines.iter().map(|s| s.as_str()).collect();
    let log = Log::new(log_lines_refs).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let new_state =
        crate::alter_battle_state(state, &log).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    let new_state_js = new_state
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(new_state_js.unchecked_into())
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod wasm_test {
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::*;

    use crate::{
        BattleState,
        wasm::{
            alter_battle_state,
            new_battle_state,
        },
    };

    #[wasm_bindgen_test]
    fn alters_battle_state_from_log_lines() {
        let state = new_battle_state().unwrap();
        let log_lines = Box::new([
            "info|battletype:Singles".to_string(),
            "turn|turn:1".to_string(),
        ]);

        let result = alter_battle_state(&state, log_lines).unwrap();

        let updated_state: BattleState =
            serde_wasm_bindgen::from_value(JsValue::from(result)).unwrap();
        assert_eq!(updated_state.battle_type, "singles");
        assert_eq!(updated_state.turn, 1);
    }
}
