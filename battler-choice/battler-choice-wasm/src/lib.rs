#![no_std]

extern crate alloc;

use alloc::string::{
    String,
    ToString,
};
use core::str::FromStr;

use battler_choice::Choice;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = parseChoice)]
pub fn parse_choice(choice_str: &str) -> Result<JsValue, JsValue> {
    let choice = Choice::from_str(choice_str).map_err(|err| JsValue::from_str(&err.to_string()))?;
    serde_wasm_bindgen::to_value(&choice).map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen(js_name = serializeChoice)]
pub fn serialize_choice(choice_js: JsValue) -> Result<String, JsValue> {
    let choice: Choice = serde_wasm_bindgen::from_value(choice_js)
        .map_err(|err| JsValue::from_str(&err.to_string()))?;
    Ok(choice.to_string())
}
