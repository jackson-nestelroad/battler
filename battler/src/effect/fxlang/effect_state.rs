use ahash::HashMapExt;

use crate::{
    battler_error,
    common::{
        Error,
        FastHashMap,
    },
    effect::fxlang::Value,
};

/// The persisted state of an individual [`Effect`][`crate::effect::Effect`].
///
/// Allows fxlang variables to be persisted across multiple callbacks.
#[derive(Clone)]
pub struct EffectState {
    values: FastHashMap<String, Value>,
}

impl EffectState {
    pub fn new() -> Self {
        Self {
            values: FastHashMap::new(),
        }
    }
}

impl From<EffectState> for Value {
    fn from(value: EffectState) -> Self {
        Self::Object(value.values)
    }
}

impl TryFrom<&Value> for EffectState {
    type Error = Error;
    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Object(value) => Ok(Self {
                values: value.clone(),
            }),
            _ => Err(battler_error!(
                "cannot convert value of type {} to EffectState",
                value.value_type()
            )),
        }
    }
}
