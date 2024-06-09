use ahash::HashMapExt;

use crate::{
    battler_error,
    common::{
        Error,
        FastHashMap,
        WrapResultError,
    },
    effect::fxlang::Value,
};

/// The persisted state of an individual [`Effect`][`crate::effect::Effect`].
///
/// Allows fxlang variables to be persisted across multiple callbacks.
#[derive(Clone)]
pub struct EffectState {
    values: FastHashMap<String, Value>,
    duration: Option<u8>,
}

impl EffectState {
    const DURATION: &'static str = "duration";

    pub fn new() -> Self {
        Self {
            values: FastHashMap::new(),
            duration: None,
        }
    }

    pub fn from_hash_map(values: FastHashMap<String, Value>) -> Result<Self, Error> {
        let duration = match values.get(Self::DURATION) {
            Some(value) => Some(
                value
                    .clone()
                    .integer_u8()
                    .wrap_error_with_message("duration must be an integer")?,
            ),
            _ => None,
        };
        Ok(Self { values, duration })
    }

    pub fn duration(&self) -> Option<u8> {
        self.duration
    }

    pub fn set_duration(&mut self, duration: u8) {
        self.values
            .insert(Self::DURATION.to_owned(), Value::U16(duration as u16));
        self.duration = Some(duration);
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
            Value::Object(value) => Self::from_hash_map(value.clone()),
            _ => Err(battler_error!(
                "cannot convert value of type {} to EffectState",
                value.value_type()
            )),
        }
    }
}
