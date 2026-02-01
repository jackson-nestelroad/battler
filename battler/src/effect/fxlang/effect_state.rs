use alloc::{
    borrow::ToOwned,
    boxed::Box,
    string::String,
    vec::Vec,
};
use core::fmt::Debug;

use anyhow::Result;
use battler_data::Fraction;
use hashbrown::HashMap;

use crate::{
    battle::{
        Context,
        CoreBattle,
        Mon,
        MonHandle,
    },
    effect::{
        AppliedEffectLocation,
        EffectHandle,
        fxlang::Value,
    },
    error::{
        WrapOptionError,
        integer_overflow_error,
    },
};

/// The activation state of an effect.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
enum EffectActivationState {
    Starting,
    Started,
    Ending,
    #[default]
    Ended,
}

/// The persisted state of an individual [`Effect`][`crate::effect::Effect`].
///
/// Allows fxlang variables to be persisted across multiple callbacks.
#[derive(Debug, Default, Clone)]
pub struct EffectState {
    initialized: bool,
    activation_state: EffectActivationState,
    values: HashMap<String, Value>,
    linked_id: Option<u32>,
    linked_to: Vec<u32>,
    effect_order: u32,
}

impl EffectState {
    const DURATION: &'static str = "duration";
    const TARGET: &'static str = "target";
    const SOURCE_EFFECT: &'static str = "source_effect";
    const SOURCE: &'static str = "source";
    const SOURCE_SIDE: &'static str = "source_side";
    const SOURCE_POSITION: &'static str = "source_position";

    /// Creates an initial effect state for a new effect.
    pub fn initial_effect_state(
        context: &mut Context,
        source_effect: Option<&EffectHandle>,
        target: Option<MonHandle>,
        source: Option<MonHandle>,
    ) -> Result<Self> {
        let mut effect_state = Self::default();
        effect_state.initialize(context, source_effect, target, source)?;
        Ok(effect_state)
    }

    /// Initializes an existing effect state object.
    pub fn initialize(
        &mut self,
        context: &mut Context,
        source_effect: Option<&EffectHandle>,
        target: Option<MonHandle>,
        source: Option<MonHandle>,
    ) -> Result<()> {
        self.activation_state = EffectActivationState::Ended;
        self.effect_order = context.battle_mut().next_effect_order();

        if let Some(source_effect) = source_effect {
            self.set_source_effect(source_effect.stable_effect_handle(context)?);
        }
        if let Some(target_handle) = target {
            self.set_target(target_handle);
        }
        if let Some(source_handle) = source {
            self.set_source(source_handle);
            let mut context = context.mon_context(source_handle)?;
            self.set_source_side(context.mon().side);
            if let Some(source_position) = Mon::position_on_side(&mut context) {
                self.set_source_position(source_position)?;
            }
        }

        self.initialized = true;
        Ok(())
    }

    /// Returns true if the state is initialized.
    pub fn initialized(&self) -> bool {
        self.initialized
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
        self.get(Self::DURATION)?.clone().integer_u8().ok()
    }

    /// Sets the duration of the effect.
    pub fn set_duration(&mut self, duration: u8) {
        self.values
            .insert(Self::DURATION.to_owned(), Value::UFraction(duration.into()));
    }

    /// The target of the effect.
    pub fn target(&self) -> Option<MonHandle> {
        self.get(Self::TARGET)?.clone().mon_handle().ok()
    }

    /// Sets the target of the effect.
    pub fn set_target(&mut self, target: MonHandle) {
        self.values
            .insert(Self::TARGET.to_owned(), Value::Mon(target));
    }

    /// The source effect of the effect.
    pub fn source_effect(&self) -> Option<EffectHandle> {
        self.get(Self::SOURCE_EFFECT)?.clone().effect_handle().ok()
    }

    /// Sets the source effect of the effect.
    pub fn set_source_effect(&mut self, source_effect: EffectHandle) {
        self.values.insert(
            Self::SOURCE_EFFECT.to_owned(),
            Value::Effect(source_effect.clone()),
        );
    }

    /// The source of the effect.
    pub fn source(&self) -> Option<MonHandle> {
        self.get(Self::SOURCE)?.clone().mon_handle().ok()
    }

    /// Sets the source of the effect.
    pub fn set_source(&mut self, source: MonHandle) {
        self.values
            .insert(Self::SOURCE.to_owned(), Value::Mon(source));
    }

    /// The source side of the effect.
    pub fn source_side(&self) -> Option<usize> {
        self.get(Self::SOURCE_SIDE)?.clone().side_index().ok()
    }

    /// Sets the source side of the effect.
    pub fn set_source_side(&mut self, source_side: usize) {
        self.values
            .insert(Self::SOURCE_SIDE.to_owned(), Value::Side(source_side));
    }

    /// The source position of the effect.
    pub fn source_position(&self) -> Option<usize> {
        self.get(Self::SOURCE_POSITION)?
            .clone()
            .integer_usize()
            .ok()
    }

    /// Sets the source position of the effect.
    pub fn set_source_position(&mut self, source_position: usize) -> Result<()> {
        self.values.insert(
            Self::SOURCE_POSITION.to_owned(),
            Value::UFraction(Fraction::from(
                TryInto::<u32>::try_into(source_position).map_err(integer_overflow_error)?,
            )),
        );
        Ok(())
    }

    fn set_activation_state(&mut self, activation_state: EffectActivationState) {
        self.activation_state = activation_state;
        *self.get_mut("started") = Value::Boolean(self.started());
        *self.get_mut("ending") = Value::Boolean(self.ending());
    }

    /// Whether or not the effect is started.
    ///
    /// An effect is started if any corresponding "Start" event has run. Unlike [`Self::ending`],
    /// this is purely used for tracking that event has been started and does not need to be
    /// "restarted," in the case of suppression effects.
    ///
    /// In other words, un-started effects will still apply their event callbacks if they exist on
    /// the battle field. If an effect should *not* apply before a `Start` event finishes, you
    /// *must* check `$effect_state.started` as a condition in the corresponding event callback. The
    /// reason for this difference is that not all effects "start," so it's not feasible to know
    /// what effects should apply even if they don't have a notion of "being started."
    pub fn started(&self) -> bool {
        self.activation_state == EffectActivationState::Started
    }

    /// Whether or not the effect is ending and should be ignored.
    ///
    /// If an effect is ending, its corresponding `End` event is running and it is assumed that the
    /// effect is about to be deleted (or at least suppressed) from the battle field. While an
    /// effect is ending, event callbacks will *not* trigger.
    pub fn ending(&self) -> bool {
        self.activation_state == EffectActivationState::Ending
    }

    /// The unique ID for linking this effect to another.
    pub fn linked_id(&self) -> Option<u32> {
        self.linked_id
    }

    /// Sets the unique ID for linking this effect to another
    pub fn set_linked_id(&mut self, linked_id: u32) {
        self.linked_id = Some(linked_id);
    }

    /// The unique IDs of effects this effect is linked to.
    pub fn linked_to(&self) -> &[u32] {
        &self.linked_to
    }

    /// Adds the unique ID of a linked effect.
    pub fn add_link(&mut self, linked_id: u32) {
        self.linked_to.push(linked_id);
    }

    /// The effect order.
    pub fn effect_order(&self) -> u32 {
        self.effect_order
    }

    /// Updates the effect order.
    pub fn set_effect_order(&mut self, effect_order: u32) {
        self.effect_order = effect_order;
    }
}

/// An object that connects an [`EffectState`] instance to the [`Context`] of a battle.
///
/// Used for dynamically reading an [`EffectState`] instance during fxlang program evaluation.
pub trait EffectStateConnector: Debug + Send {
    /// Checks if the underlying effect state exists.
    fn exists(&self, context: &mut Context) -> Result<bool>;

    /// Gets a mutable reference to the effect state, for reading and assignment.
    fn get_mut<'a>(&self, context: &'a mut Context) -> Result<Option<&'a mut EffectState>>;

    /// The applied effect location.
    fn applied_effect_location(&self) -> AppliedEffectLocation;

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
    pub fn exists(&self, context: &mut Context) -> Result<bool> {
        self.0.exists(context)
    }

    /// Gets a mutable reference to the effect state, for reading and assignment.
    pub fn get_mut<'a>(&self, context: &'a mut Context) -> Result<&'a mut EffectState> {
        Ok(self
            .0
            .get_mut(context)?
            .wrap_expectation("effect state is not defined")?)
    }

    /// The applied effect location.
    pub fn applied_effect_location(&self) -> AppliedEffectLocation {
        self.0.applied_effect_location()
    }

    fn set_activation_state(
        &self,
        context: &mut Context,
        state: EffectActivationState,
    ) -> Result<()> {
        self.get_mut(context)?.set_activation_state(state);
        CoreBattle::invalidate_effect_caches(context)?;
        Ok(())
    }

    /// Sets that the effect is starting.
    pub fn set_starting(&self, context: &mut Context) -> Result<()> {
        self.set_activation_state(context, EffectActivationState::Starting)
    }

    /// Sets that the effect has started.
    pub fn set_started(&self, context: &mut Context) -> Result<()> {
        self.set_activation_state(context, EffectActivationState::Started)
    }

    /// Sets that the effect is ending.
    pub fn set_ending(&self, context: &mut Context) -> Result<()> {
        self.set_activation_state(context, EffectActivationState::Ending)
    }

    /// Sets that the effect has ended.
    pub fn set_ended(&self, context: &mut Context) -> Result<()> {
        self.set_activation_state(context, EffectActivationState::Ended)
    }
}

impl Clone for DynamicEffectStateConnector {
    fn clone(&self) -> Self {
        self.0.make_dynamic()
    }
}
