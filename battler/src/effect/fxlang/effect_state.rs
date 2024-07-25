use std::fmt::Debug;

use ahash::HashMapExt;

use crate::{
    battle::{
        Context,
        MonHandle,
    },
    common::{
        Error,
        FastHashMap,
        Fraction,
        WrapResultError,
    },
    effect::{
        fxlang::Value,
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
    source_effect: Option<EffectHandle>,
    source: Option<MonHandle>,
    source_side: Option<usize>,
    source_position: Option<usize>,
}

impl EffectState {
    const DURATION: &'static str = "duration";
    const SOURCE_EFFECT: &'static str = "source_effect";
    const SOURCE: &'static str = "source";
    const SOURCE_SIDE: &'static str = "source_side";
    const SOURCE_POSITION: &'static str = "source_position";

    /// Creates a new, default instance.
    pub fn new() -> Self {
        Self {
            values: FastHashMap::new(),
            duration: None,
            source_effect: None,
            source: None,
            source_side: None,
            source_position: None,
        }
    }

    /// Gets the value associated with the given key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    /// Gets a mutable reference to the value associated with the given key.
    ///
    /// If the key does not exist, an undefined entry is created for assignment.
    pub fn get_mut(&mut self, key: &str) -> &mut Value {
        self.values
            .entry(key.to_owned())
            .or_insert(Value::Undefined)
    }

    /// The duration of the effect.
    pub fn duration(&self) -> Option<u8> {
        self.duration
    }

    /// Sets the duration of the effect.
    pub fn set_duration(&mut self, duration: u8) {
        self.values
            .insert(Self::DURATION.to_owned(), Value::UFraction(duration.into()));
        self.duration = Some(duration);
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
    pub fn set_source_position(&mut self, source_position: usize) -> Result<(), Error> {
        self.values.insert(
            Self::SOURCE_POSITION.to_owned(),
            Value::UFraction(Fraction::from(
                TryInto::<u32>::try_into(source_position)
                    .wrap_error_with_message("integer overflow")?,
            )),
        );
        self.source_position = Some(source_position);
        Ok(())
    }
}

/// An object that connects an [`EffectState`] instance to the [`Context`] of a battle.
///
/// Used for dynamically reading an [`EffectState`] instance during fxlang program evaluation.
pub trait EffectStateConnector: Debug {
    /// Checks if the underlying effect state exists.
    fn exists(&self, context: &mut Context) -> Result<bool, Error>;

    /// Gets a mutable reference to the effect state, for reading and assignment.
    fn get_mut<'a>(&self, context: &'a mut Context) -> Result<Option<&'a mut EffectState>, Error>;

    /// Clones the connection into a dynamic value.
    fn make_dynamic(&self) -> DynamicEffectStateConnector;
}

/// A dynamic [`EffectStateConnector`], which can be passed around like a value.
#[derive(Debug)]
pub struct DynamicEffectStateConnector(Box<dyn EffectStateConnector>);

impl DynamicEffectStateConnector {
    /// Creates a new dynamic connector from a connector implementation.
    pub fn new<T>(connector: T) -> Self
    where
        T: EffectStateConnector + 'static,
    {
        Self(Box::new(connector))
    }

    /// Checks if the underlying effect state exists.
    pub fn exists(&self, context: &mut Context) -> Result<bool, Error> {
        self.0.exists(context)
    }

    /// Gets a mutable reference to the effect state, for reading and assignment.
    pub fn get_mut<'a>(&self, context: &'a mut Context) -> Result<&'a mut EffectState, Error> {
        Ok(self
            .0
            .get_mut(context)?
            .wrap_error_with_message("effect state is not defined")?)
    }
}

impl Clone for DynamicEffectStateConnector {
    fn clone(&self) -> Self {
        self.0.make_dynamic()
    }
}
