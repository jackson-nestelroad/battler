use ahash::HashMapExt;

use crate::{
    battler_error,
    common::{
        Error,
        FastHashMap,
        Id,
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
    move_id: Option<Id>,
    target_location: Option<isize>,
}

impl EffectState {
    const DURATION: &'static str = "duration";
    const MOVE_ID: &'static str = "move";
    const TARGET_LOCATION: &'static str = "target_location";

    pub fn new() -> Self {
        Self {
            values: FastHashMap::new(),
            duration: None,
            move_id: None,
            target_location: None,
        }
    }

    pub fn from_hash_map(values: FastHashMap<String, Value>) -> Result<Self, Error> {
        let duration = match values.get(Self::DURATION) {
            Some(value) => Some(
                value
                    .clone()
                    .integer_u8()
                    .wrap_error_with_message("duration must be a u8 integer")?,
            ),
            _ => None,
        };
        let move_id = match values.get(Self::MOVE_ID) {
            Some(value) => Some(Id::from(
                value
                    .clone()
                    .string()
                    .wrap_error_with_message("move must be a string")?,
            )),
            _ => None,
        };
        let mut target_location = match values.get(Self::TARGET_LOCATION) {
            Some(value) => Some(
                value
                    .clone()
                    .integer_isize()
                    .wrap_error_with_message("target location must be an isize integer")?,
            ),
            _ => None,
        };
        // If no target was set, 0 is returned, which is equivalent to no last target.
        if let Some(0) = target_location {
            target_location = None;
        }
        Ok(Self {
            values,
            duration,
            move_id,
            target_location,
        })
    }

    pub fn duration(&self) -> Option<u8> {
        self.duration
    }

    pub fn set_duration(&mut self, duration: u8) {
        self.values
            .insert(Self::DURATION.to_owned(), Value::U16(duration as u16));
        self.duration = Some(duration);
    }

    pub fn move_id(&self) -> Option<&Id> {
        self.move_id.as_ref()
    }

    pub fn target_location(&self) -> Option<isize> {
        self.target_location
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
