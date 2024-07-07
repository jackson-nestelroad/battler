use ahash::HashMapExt;

use crate::{
    battle::MonHandle,
    battler_error,
    common::{
        Error,
        FastHashMap,
        Id,
        WrapResultError,
    },
    effect::{
        fxlang::{
            Value,
            ValueType,
        },
        EffectHandle,
    },
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
    source_effect: Option<EffectHandle>,
    source: Option<MonHandle>,
    source_side: Option<usize>,
    source_position: Option<usize>,
}

impl EffectState {
    const DURATION: &'static str = "duration";
    const MOVE_ID: &'static str = "move";
    const TARGET_LOCATION: &'static str = "target_location";
    const SOURCE_EFFECT: &'static str = "source_effect";
    const SOURCE: &'static str = "source";
    const SOURCE_SIDE: &'static str = "source_side";
    const SOURCE_POSITION: &'static str = "source_position";

    /// Creates a new, default instance.
    pub fn new() -> Self {
        Self {
            values: FastHashMap::new(),
            duration: None,
            move_id: None,
            target_location: None,
            source_effect: None,
            source: None,
            source_side: None,
            source_position: None,
        }
    }

    /// Creates a new state object from the values in the given hash map.
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

        let source_effect = match values.get(Self::SOURCE_EFFECT) {
            Some(value) => Some(
                value
                    .clone()
                    .effect_handle()
                    .wrap_error_with_message("source effect must be an effect handle")?,
            ),
            _ => None,
        };

        let source = match values.get(Self::SOURCE) {
            Some(value) => Some(
                value
                    .clone()
                    .mon_handle()
                    .wrap_error_with_message("source must be a mon handle")?,
            ),
            _ => None,
        };

        let source_side = match values.get(Self::SOURCE_SIDE) {
            Some(value) => Some(
                value
                    .clone()
                    .side_index()
                    .wrap_error_with_message("source side must be a side index")?,
            ),
            _ => None,
        };

        let source_position = match values.get(Self::SOURCE_POSITION) {
            Some(value) => Some(
                value
                    .clone()
                    .integer_usize()
                    .wrap_error_with_message("source position must be a usize")?,
            ),
            _ => None,
        };

        for value in values.values() {
            match value.value_type() {
                ValueType::ActiveMove => return Err(battler_error!("active moves cannot be stored on EffectState because they can be dropped at the end of a turn")),
                _ => (),
            }
        }

        Ok(Self {
            values,
            duration,
            move_id,
            target_location,
            source_effect,
            source,
            source_side,
            source_position,
        })
    }

    /// The duration of the effect.
    pub fn duration(&self) -> Option<u8> {
        self.duration
    }

    /// Sets the duration of the effect.
    pub fn set_duration(&mut self, duration: u8) {
        self.values
            .insert(Self::DURATION.to_owned(), Value::U64(duration as u64));
        self.duration = Some(duration);
    }

    /// The move ID of the effect.
    pub fn move_id(&self) -> Option<&Id> {
        self.move_id.as_ref()
    }

    /// The target location of the effect.
    pub fn target_location(&self) -> Option<isize> {
        self.target_location
    }

    /// The source effect of the effect.
    pub fn source_effect(&self) -> Option<EffectHandle> {
        self.source_effect.clone()
    }

    /// Sets the source effect of the effect.
    pub fn set_source_effect(&mut self, source_effect: EffectHandle) {
        self.values.insert(
            Self::SOURCE_EFFECT.to_owned(),
            Value::Effect(source_effect.clone()),
        );
        self.source_effect = Some(source_effect);
    }

    /// The source of the effect.
    pub fn source(&self) -> Option<MonHandle> {
        self.source
    }

    /// Sets the source of the effect.
    pub fn set_source(&mut self, source: MonHandle) {
        self.values
            .insert(Self::SOURCE.to_owned(), Value::Mon(source));
        self.source = Some(source);
    }

    /// The source side of the effect.
    pub fn source_side(&self) -> Option<usize> {
        self.source_side
    }

    /// Sets the source side of the effect.
    pub fn set_source_side(&mut self, source_side: usize) {
        self.values
            .insert(Self::SOURCE_SIDE.to_owned(), Value::Side(source_side));
        self.source_side = Some(source_side);
    }

    /// The source position of the effect.
    pub fn source_position(&self) -> Option<usize> {
        self.source_position
    }

    /// Sets the source position of the effect.
    pub fn set_source_position(&mut self, source_position: usize) {
        self.values.insert(
            Self::SOURCE_POSITION.to_owned(),
            Value::U64(source_position as u64),
        );
        self.source_position = Some(source_position);
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
