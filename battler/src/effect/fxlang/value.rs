use std::{
    cmp::Ordering,
    fmt::{
        self,
        Display,
    },
    str::FromStr,
};

use zone_alloc::ElementRef;

use crate::{
    battle::{
        Boost,
        BoostTable,
        MonHandle,
        MoveEventResult,
        MoveHandle,
        MoveOutcomeOnTarget,
        MoveSlot,
    },
    battler_error,
    common::{
        Error,
        FastHashMap,
        Fraction,
        Id,
        Identifiable,
        WrapResultError,
    },
    effect::{
        fxlang::{
            DynamicEffectStateConnector,
            EvaluationContext,
        },
        EffectHandle,
    },
    mons::Type,
    moves::{
        Accuracy,
        HitEffect,
        MoveCategory,
        MoveTarget,
    },
};

/// The type of an fxlang value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Undefined,
    Boolean,
    Fraction,
    UFraction,
    String,
    Mon,
    Effect,
    ActiveMove,
    MoveCategory,
    MoveTarget,
    Type,
    Boost,
    BoostTable,
    Side,
    MoveSlot,
    Player,
    Accuracy,
    Field,
    HitEffect,
    EffectState,
    List,
    Object,
}

impl ValueType {
    /// Checks if the value type is a number.
    pub fn is_number(&self) -> bool {
        match self {
            Self::Fraction | Self::UFraction => true,
            _ => false,
        }
    }
}

impl Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// An fxlang value.
#[derive(Debug, Clone)]
pub enum Value {
    Undefined,
    Boolean(bool),
    Fraction(Fraction<i64>),
    UFraction(Fraction<u64>),
    String(String),
    Mon(MonHandle),
    Effect(EffectHandle),
    ActiveMove(MoveHandle),
    MoveCategory(MoveCategory),
    MoveTarget(MoveTarget),
    Type(Type),
    Boost(Boost),
    BoostTable(BoostTable),
    Side(usize),
    MoveSlot(MoveSlot),
    Player(usize),
    Accuracy(Accuracy),
    Field,
    HitEffect(HitEffect),
    EffectState(DynamicEffectStateConnector),
    List(Vec<Value>),
    Object(FastHashMap<String, Value>),
}

impl Value {
    /// The type of the value.
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Undefined => ValueType::Undefined,
            Self::Boolean(_) => ValueType::Boolean,
            Self::Fraction(_) => ValueType::Fraction,
            Self::UFraction(_) => ValueType::UFraction,
            Self::String(_) => ValueType::String,
            Self::Mon(_) => ValueType::Mon,
            Self::Effect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::Boost(_) => ValueType::Boost,
            Self::BoostTable(_) => ValueType::BoostTable,
            Self::Type(_) => ValueType::Type,
            Self::Side(_) => ValueType::Side,
            Self::MoveSlot(_) => ValueType::MoveSlot,
            Self::Player(_) => ValueType::Player,
            Self::Accuracy(_) => ValueType::Accuracy,
            Self::Field => ValueType::Field,
            Self::HitEffect(_) => ValueType::HitEffect,
            Self::EffectState(_) => ValueType::EffectState,
            Self::List(_) => ValueType::List,
            Self::Object(_) => ValueType::Object,
        }
    }

    fn invalid_type(got: ValueType, expected: ValueType) -> Error {
        battler_error!("got {got}, expected {expected}")
    }

    fn incompatible_type(from: ValueType, to: ValueType) -> Error {
        battler_error!("cannot convert from {from} to {to}")
    }

    /// Checks if the value signals an early exit from an event perspective.
    pub fn signals_early_exit(&self) -> bool {
        match self {
            Self::Boolean(false) => true,
            Self::String(val) => {
                MoveEventResult::from_str(val).is_ok_and(|result| !result.advance())
            }
            _ => false,
        }
    }

    /// Converts the value to the given type.
    pub fn convert_to(&self, value_type: ValueType) -> Result<Self, Error> {
        if self.value_type() == value_type {
            return Ok(self.clone());
        }

        match (self, value_type) {
            (Self::Fraction(val), ValueType::UFraction) => Ok(Value::UFraction(
                val.try_convert()
                    .wrap_error_with_message("integer overflow")?,
            )),
            (Self::Fraction(val), ValueType::Accuracy) => {
                Ok(Value::Accuracy(Accuracy::from(val.floor() as u8)))
            }
            (Self::UFraction(val), ValueType::Fraction) => Ok(Value::Fraction(
                val.try_convert()
                    .wrap_error_with_message("integer overflow")?,
            )),
            (Self::UFraction(val), ValueType::Accuracy) => {
                Ok(Value::Accuracy(Accuracy::from(val.floor() as u8)))
            }
            (Self::String(val), ValueType::MoveCategory) => Ok(Value::MoveCategory(
                MoveCategory::from_str(val).wrap_error_with_message("invalid move category")?,
            )),
            (Self::String(val), ValueType::MoveTarget) => Ok(Value::MoveTarget(
                MoveTarget::from_str(val).wrap_error_with_message("invalid move target")?,
            )),
            (Self::String(val), ValueType::Type) => Ok(Value::Type(
                Type::from_str(val).wrap_error_with_message("invalid type")?,
            )),
            (Self::String(val), ValueType::Accuracy) => Ok(Value::Accuracy(
                Accuracy::from_str(val).wrap_error_with_message("invalid accuracy")?,
            )),
            _ => Err(Self::incompatible_type(self.value_type(), value_type)),
        }
    }

    /// Consumes the value into a [`String`].
    pub fn string(self) -> Result<String, Error> {
        match self {
            Self::String(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::String)),
        }
    }

    /// Consumes the value into a [`u64`].
    pub fn integer_u64(self) -> Result<u64, Error> {
        match self {
            Self::Fraction(val) => Ok(val.floor() as u64),
            Self::UFraction(val) => Ok(val.floor() as u64),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::UFraction)),
        }
    }

    /// Consumes the value into a [`i64`].
    pub fn integer_i64(self) -> Result<i64, Error> {
        match self {
            Self::Fraction(val) => Ok(val.floor() as i64),
            Self::UFraction(val) => Ok(val.floor() as i64),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Fraction)),
        }
    }

    /// Consumes the value into an [`i8`].
    pub fn integer_i8(self) -> Result<i8, Error> {
        self.integer_i64()?
            .try_into()
            .wrap_error_with_message("integer overflow")
    }

    /// Consumes the value into a [`u8`].
    pub fn integer_u8(self) -> Result<u8, Error> {
        self.integer_u64()?
            .try_into()
            .wrap_error_with_message("integer overflow")
    }

    /// Consumes the value into a [`u16`].
    pub fn integer_u16(self) -> Result<u16, Error> {
        self.integer_u64()?
            .try_into()
            .wrap_error_with_message("integer overflow")
    }

    /// Consumes the value into a [`u32`].
    pub fn integer_u32(self) -> Result<u32, Error> {
        self.integer_u64()?
            .try_into()
            .wrap_error_with_message("integer overflow")
    }

    /// Consumes the value into an [`isize`].
    pub fn integer_isize(self) -> Result<isize, Error> {
        self.integer_i64()?
            .try_into()
            .wrap_error_with_message("integer overflow")
    }

    /// Consumes the value into a [`usize`].
    pub fn integer_usize(self) -> Result<usize, Error> {
        self.integer_u64()?
            .try_into()
            .wrap_error_with_message("integer overflow")
    }

    /// Checks if the value is undefined.
    pub fn is_undefined(&self) -> bool {
        match self {
            Self::Undefined => true,
            _ => false,
        }
    }

    /// Consumes the value into a [`bool`].
    pub fn boolean(self) -> Result<bool, Error> {
        match self {
            Self::Boolean(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Boolean)),
        }
    }

    /// Consumes the value into a [`MonHandle`].
    pub fn mon_handle(self) -> Result<MonHandle, Error> {
        match self {
            Self::Mon(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Mon)),
        }
    }

    /// Consumes the value into a [`MoveHandle`].
    pub fn active_move(self) -> Result<MoveHandle, Error> {
        match self {
            Self::ActiveMove(val) => Ok(val),
            Self::Effect(EffectHandle::ActiveMove(val, _)) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::ActiveMove)),
        }
    }

    /// Consumes the value into a [`MoveEventResult`].
    pub fn move_result(self) -> Result<MoveEventResult, Error> {
        match self {
            Self::Boolean(val) => Ok(MoveEventResult::from(val)),
            Self::String(val) => MoveEventResult::from_str(&val)
                .wrap_error_with_message("invalid move event result string"),
            val @ _ => Err(battler_error!(
                "value of type {} cannot be converted to a move event result",
                val.value_type()
            )),
        }
    }

    /// Consumes the value into a [`MoveOutcomeOnTarget`].
    pub fn move_outcome_on_target(self) -> Result<MoveOutcomeOnTarget, Error> {
        match self {
            Self::Boolean(val) => Ok(MoveOutcomeOnTarget::from(val)),
            _ => Ok(MoveOutcomeOnTarget::Damage(self.integer_u16()?)),
        }
    }

    /// Consumes the value into a move ID.
    pub fn move_id(self, context: &mut EvaluationContext) -> Result<Id, Error> {
        match self {
            Self::ActiveMove(val) | Self::Effect(EffectHandle::ActiveMove(val, _)) => {
                Ok(context.active_move(val)?.id().clone())
            }
            Self::String(val) => Ok(Id::from(val)),
            val @ _ => Err(battler_error!(
                "value of type {} cannot be converted to a move id",
                val.value_type()
            )),
        }
    }

    /// Consumes the value into an [`EffectHandle`].
    pub fn effect_handle(self) -> Result<EffectHandle, Error> {
        match self {
            Self::Effect(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Effect)),
        }
    }

    /// Consumes the value into a [`Boost`].
    pub fn boost(self) -> Result<Boost, Error> {
        match self {
            Self::Boost(val) => Ok(val),
            Self::String(val) => {
                Boost::from_str(&val).wrap_error_with_message("invalid boost from string")
            }
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Boost)),
        }
    }

    /// Consumes the value into a [`BoostTable`].
    pub fn boost_table(self) -> Result<BoostTable, Error> {
        match self {
            Self::BoostTable(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::BoostTable)),
        }
    }

    /// Consumes the value into a [`Type`].
    pub fn mon_type(self) -> Result<Type, Error> {
        match self {
            Self::Type(val) => Ok(val),
            Self::String(val) => {
                Type::from_str(&val).wrap_error_with_message("invalid type from string")
            }
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Type)),
        }
    }

    /// Consumes the value into a side index.
    pub fn side_index(self) -> Result<usize, Error> {
        match self {
            Self::Side(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Side)),
        }
    }

    /// Consumes the value into a player index.
    pub fn player_index(self) -> Result<usize, Error> {
        match self {
            Self::Player(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Player)),
        }
    }

    /// Consumes the value into a [`MoveSlot`].
    pub fn move_slot(self) -> Result<MoveSlot, Error> {
        match self {
            Self::MoveSlot(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::MoveSlot)),
        }
    }

    /// Consumes the value into a [`MoveTarget`].
    pub fn move_target(self) -> Result<MoveTarget, Error> {
        match self {
            Self::MoveTarget(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::MoveTarget)),
        }
    }

    /// Checks if the value is a list.
    pub fn is_list(&self) -> bool {
        match self {
            Self::List(_) => true,
            _ => false,
        }
    }

    /// Consumes the value into a [`Vec<Value>`].
    pub fn list(self) -> Result<Vec<Value>, Error> {
        match self {
            Self::List(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::List)),
        }
    }

    /// Consumes the value into a [`Vec<Type>`].
    pub fn types_list(self) -> Result<Vec<Type>, Error> {
        self.list()?.into_iter().map(|val| val.mon_type()).collect()
    }

    /// Consumes the value into a [`HitEffect`].
    pub fn hit_effect(self) -> Result<HitEffect, Error> {
        match self {
            Self::HitEffect(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::HitEffect)),
        }
    }
}

/// A [`Value`] that could also be a reference to a value.
///
/// This is the real value type used in fxlang evaluation, because the language supports passing
/// objects and lists by reference, and a single [`ValueType`] can map to multiple Rust types (e.g.,
/// numeric literals can be expressed in multiple ways across the battle engine).
#[derive(Clone)]
pub enum MaybeReferenceValue<'eval> {
    Undefined,
    Boolean(bool),
    Fraction(Fraction<i64>),
    UFraction(Fraction<u64>),
    String(String),
    Mon(MonHandle),
    Effect(EffectHandle),
    ActiveMove(MoveHandle),
    MoveCategory(MoveCategory),
    MoveTarget(MoveTarget),
    Type(Type),
    Boost(Boost),
    BoostTable(BoostTable),
    Side(usize),
    MoveSlot(MoveSlot),
    Player(usize),
    Accuracy(Accuracy),
    Field,
    HitEffect(HitEffect),
    EffectState(DynamicEffectStateConnector),
    List(Vec<MaybeReferenceValue<'eval>>),
    Object(FastHashMap<String, MaybeReferenceValue<'eval>>),
    Reference(ValueRefToStoredValue<'eval>),
}

impl<'eval> MaybeReferenceValue<'eval> {
    /// The type of the value.
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Undefined => ValueType::Undefined,
            Self::Boolean(_) => ValueType::Boolean,
            Self::Fraction(_) => ValueType::Fraction,
            Self::UFraction(_) => ValueType::UFraction,
            Self::String(_) => ValueType::String,
            Self::Mon(_) => ValueType::Mon,
            Self::Effect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::Boost(_) => ValueType::Boost,
            Self::BoostTable(_) => ValueType::BoostTable,
            Self::Type(_) => ValueType::Type,
            Self::Side(_) => ValueType::Side,
            Self::MoveSlot(_) => ValueType::MoveSlot,
            Self::Player(_) => ValueType::Player,
            Self::Accuracy(_) => ValueType::Accuracy,
            Self::Field => ValueType::Field,
            Self::HitEffect(_) => ValueType::HitEffect,
            Self::EffectState(_) => ValueType::EffectState,
            Self::List(_) => ValueType::List,
            Self::Object(_) => ValueType::Object,
            Self::Reference(val) => val.value_type(),
        }
    }

    /// Converts the value to a [`Value`], which is guaranteed to contain no references.
    pub fn to_owned(&self) -> Value {
        match self {
            Self::Undefined => Value::Undefined,
            Self::Boolean(val) => Value::Boolean(*val),
            Self::Fraction(val) => Value::Fraction(*val),
            Self::UFraction(val) => Value::UFraction(*val),
            Self::String(val) => Value::String(val.clone()),
            Self::Mon(val) => Value::Mon(*val),
            Self::Effect(val) => Value::Effect(val.clone()),
            Self::ActiveMove(val) => Value::ActiveMove(*val),
            Self::MoveCategory(val) => Value::MoveCategory(*val),
            Self::MoveTarget(val) => Value::MoveTarget(*val),
            Self::Type(val) => Value::Type(*val),
            Self::Boost(val) => Value::Boost(*val),
            Self::BoostTable(val) => Value::BoostTable(val.clone()),
            Self::Side(val) => Value::Side(*val),
            Self::MoveSlot(val) => Value::MoveSlot(val.clone()),
            Self::Player(val) => Value::Player(*val),
            Self::Accuracy(val) => Value::Accuracy(*val),
            Self::Field => Value::Field,
            Self::HitEffect(val) => Value::HitEffect(val.clone()),
            Self::EffectState(val) => Value::EffectState(val.clone()),
            Self::List(val) => Value::List(val.into_iter().map(|val| val.to_owned()).collect()),
            Self::Object(val) => Value::Object(
                val.into_iter()
                    .map(|(key, val)| (key.clone(), val.to_owned()))
                    .collect(),
            ),
            Self::Reference(val) => val.to_owned(),
        }
    }

    /// Converts the value to a boolean, if possible.
    pub fn boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(val) => Some(*val),
            Self::Reference(val) => val.value_ref().boolean(),
            _ => None,
        }
    }

    /// Checks if the value supports list iteration.
    pub fn supports_list_iteration(&self) -> bool {
        match self {
            Self::List(_) => true,
            Self::Reference(reference) => reference.value_ref().supports_list_iteration(),
            _ => false,
        }
    }

    /// Returns the length of the value, if supported.
    pub fn len(&self) -> Option<usize> {
        match self {
            Self::String(val) => Some(val.len()),
            Self::List(val) => Some(val.len()),
            Self::Object(val) => Some(val.len()),
            Self::Reference(reference) => reference.value_ref().len(),
            _ => None,
        }
    }

    /// Returns the list element at the given index.
    ///
    /// Returns a [`MaybeReferenceValueForOperation`], because the value may come from a list
    /// generated in the statement or one stored in a variable.
    pub fn list_index<'value>(
        &'value self,
        index: usize,
    ) -> Option<MaybeReferenceValueForOperation<'value>> {
        match self {
            Self::List(list) => list
                .get(index)
                .map(|val| MaybeReferenceValueForOperation::from(val)),
            Self::Reference(reference) => reference
                .value
                .list_index(index)
                .map(|val| MaybeReferenceValueForOperation::from(val)),
            _ => None,
        }
    }
}

impl From<Value> for MaybeReferenceValue<'_> {
    fn from(value: Value) -> Self {
        match value {
            Value::Undefined => Self::Undefined,
            Value::Boolean(val) => Self::Boolean(val),
            Value::Fraction(val) => Self::Fraction(val),
            Value::UFraction(val) => Self::UFraction(val),
            Value::String(val) => Self::String(val),
            Value::Mon(val) => Self::Mon(val),
            Value::Effect(val) => Self::Effect(val),
            Value::ActiveMove(val) => Self::ActiveMove(val),
            Value::MoveCategory(val) => Self::MoveCategory(val),
            Value::MoveTarget(val) => Self::MoveTarget(val),
            Value::Type(val) => Self::Type(val),
            Value::Boost(val) => Self::Boost(val),
            Value::BoostTable(val) => Self::BoostTable(val),
            Value::Side(val) => Self::Side(val),
            Value::MoveSlot(val) => Self::MoveSlot(val),
            Value::Player(val) => Self::Player(val),
            Value::Accuracy(val) => Self::Accuracy(val),
            Value::Field => Self::Field,
            Value::HitEffect(val) => Self::HitEffect(val),
            Value::EffectState(val) => Self::EffectState(val),
            Value::List(val) => Self::List(
                val.into_iter()
                    .map(|val| MaybeReferenceValue::from(val))
                    .collect(),
            ),
            Value::Object(val) => Self::Object(
                val.into_iter()
                    .map(|(key, val)| (key, MaybeReferenceValue::from(val)))
                    .collect(),
            ),
        }
    }
}

impl<'eval> From<ValueRefToStoredValue<'eval>> for MaybeReferenceValue<'eval> {
    fn from(value: ValueRefToStoredValue<'eval>) -> Self {
        Self::Reference(value)
    }
}

/// A [`Value`], but containing a reference to the underlying value.
#[derive(Clone)]
pub enum ValueRef<'eval> {
    Undefined,
    Boolean(bool),
    Fraction(Fraction<i64>),
    UFraction(Fraction<u64>),
    String(&'eval String),
    Str(&'eval str),
    TempString(String),
    Mon(MonHandle),
    Effect(&'eval EffectHandle),
    TempEffect(EffectHandle),
    ActiveMove(MoveHandle),
    MoveCategory(MoveCategory),
    MoveTarget(MoveTarget),
    Type(Type),
    Boost(Boost),
    BoostTable(&'eval BoostTable),
    Side(usize),
    MoveSlot(&'eval MoveSlot),
    Player(usize),
    Accuracy(Accuracy),
    Field,
    HitEffect(&'eval HitEffect),
    EffectState(DynamicEffectStateConnector),
    List(&'eval Vec<Value>),
    TempList(Vec<ValueRefToStoredValue<'eval>>),
    Object(&'eval FastHashMap<String, Value>),
}

impl<'eval> ValueRef<'eval> {
    /// The type of the value.
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Undefined => ValueType::Undefined,
            Self::Boolean(_) => ValueType::Boolean,
            Self::Fraction(_) => ValueType::Fraction,
            Self::UFraction(_) => ValueType::UFraction,
            Self::String(_) => ValueType::String,
            Self::Str(_) => ValueType::String,
            Self::TempString(_) => ValueType::String,
            Self::Mon(_) => ValueType::Mon,
            Self::Effect(_) => ValueType::Effect,
            Self::TempEffect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::Type(_) => ValueType::Type,
            Self::Boost(_) => ValueType::Boost,
            Self::BoostTable(_) => ValueType::BoostTable,
            Self::Side(_) => ValueType::Side,
            Self::MoveSlot(_) => ValueType::MoveSlot,
            Self::Player(_) => ValueType::Player,
            Self::Accuracy(_) => ValueType::Accuracy,
            Self::Field => ValueType::Field,
            Self::HitEffect(_) => ValueType::HitEffect,
            Self::EffectState(_) => ValueType::EffectState,
            Self::List(_) => ValueType::List,
            Self::TempList(_) => ValueType::List,
            Self::Object(_) => ValueType::Object,
        }
    }

    /// Converts the reference to a [`Value`], which is guaranteed to contain no references.
    pub fn to_owned(&self) -> Value {
        match self {
            Self::Undefined => Value::Undefined,
            Self::Boolean(val) => Value::Boolean(*val),
            Self::Fraction(val) => Value::Fraction(*val),
            Self::UFraction(val) => Value::UFraction(*val),
            Self::String(val) => Value::String(val.to_string()),
            Self::Str(val) => Value::String(val.to_string()),
            Self::TempString(val) => Value::String(val.clone()),
            Self::Mon(val) => Value::Mon(*val),
            Self::Effect(val) => Value::Effect((*val).clone()),
            Self::TempEffect(val) => Value::Effect(val.clone()),
            Self::ActiveMove(val) => Value::ActiveMove(*val),
            Self::MoveCategory(val) => Value::MoveCategory(*val),
            Self::MoveTarget(val) => Value::MoveTarget(*val),
            Self::Type(val) => Value::Type(*val),
            Self::Boost(val) => Value::Boost(*val),
            Self::BoostTable(val) => Value::BoostTable((*val).clone()),
            Self::Side(val) => Value::Side(*val),
            Self::MoveSlot(val) => Value::MoveSlot((*val).clone()),
            Self::Player(val) => Value::Player(*val),
            Self::Accuracy(val) => Value::Accuracy(*val),
            Self::Field => Value::Field,
            Self::HitEffect(val) => Value::HitEffect((*val).clone()),
            Self::EffectState(val) => Value::EffectState(val.clone()),
            Self::List(val) => Value::List((*val).clone()),
            Self::TempList(val) => Value::List(val.iter().map(|val| val.to_owned()).collect()),
            Self::Object(val) => Value::Object((*val).clone()),
        }
    }

    /// Converts the value to a boolean, if possible.
    pub fn boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(val) => Some(*val),
            _ => None,
        }
    }

    /// Checks if the value supports list iteration.
    pub fn supports_list_iteration(&self) -> bool {
        match self {
            Self::List(_) => true,
            Self::TempList(_) => true,
            _ => false,
        }
    }

    /// Returns the length of the value, if supported.
    pub fn len(&self) -> Option<usize> {
        match self {
            Self::String(val) => Some(val.len()),
            Self::Str(val) => Some(val.len()),
            Self::TempString(val) => Some(val.len()),
            Self::List(val) => Some(val.len()),
            Self::TempList(val) => Some(val.len()),
            Self::Object(val) => Some(val.len()),
            _ => None,
        }
    }

    /// Returns the list element at the given index.
    pub fn list_index(&self, index: usize) -> Option<ValueRef<'eval>> {
        match self {
            Self::List(list) => list.get(index).map(|val| ValueRef::from(val)),
            Self::TempList(list) => list.get(index).map(|val| val.value.clone()),
            _ => None,
        }
    }

    /// Returns the [`MonHandle`] associated with a Mon reference.
    pub fn mon_handle(&self) -> Option<MonHandle> {
        match self {
            Self::Mon(mon_handle) => Some(*mon_handle),
            _ => None,
        }
    }

    /// Returns the [`EffectHandle`] associated with an effect reference.
    pub fn effect_handle(&self) -> Option<&EffectHandle> {
        match self {
            Self::Effect(effect_handle) => Some(effect_handle),
            Self::TempEffect(effect_handle) => Some(effect_handle),
            _ => None,
        }
    }

    /// Returns the [`MoveHandle`] associated with an active move reference.
    pub fn active_move_handle(&self) -> Option<MoveHandle> {
        match self {
            Self::ActiveMove(move_handle) => Some(*move_handle),
            _ => None,
        }
    }
}

impl<'element, 'value> From<&'element ElementRef<'value, Value>> for ValueRef<'element> {
    fn from(value: &'element ElementRef<'value, Value>) -> Self {
        Self::from(value.as_ref())
    }
}

impl<'eval> From<&'eval Value> for ValueRef<'eval> {
    fn from(value: &'eval Value) -> Self {
        match value {
            Value::Undefined => Self::Undefined,
            Value::Boolean(val) => Self::Boolean(*val),
            Value::Fraction(val) => Self::Fraction(*val),
            Value::UFraction(val) => Self::UFraction(*val),
            Value::String(val) => Self::String(val),
            Value::Mon(val) => Self::Mon(*val),
            Value::Effect(val) => Self::Effect(val),
            Value::ActiveMove(val) => Self::ActiveMove(*val),
            Value::MoveCategory(val) => Self::MoveCategory(*val),
            Value::MoveTarget(val) => Self::MoveTarget(*val),
            Value::Type(val) => Self::Type(*val),
            Value::Boost(val) => Self::Boost(*val),
            Value::BoostTable(val) => Self::BoostTable(val),
            Value::Side(val) => Self::Side(*val),
            Value::MoveSlot(val) => Self::MoveSlot(val),
            Value::Player(val) => Self::Player(*val),
            Value::Accuracy(val) => Self::Accuracy(*val),
            Value::Field => Self::Field,
            Value::HitEffect(val) => Self::HitEffect(val),
            Value::EffectState(val) => Self::EffectState(val.clone()),
            Value::List(val) => Self::List(val),
            Value::Object(val) => Self::Object(val),
        }
    }
}

/// A reference to some stored [`Value`].
///
/// Assumes the underlying value is stored in an [`ElementRef`].
#[derive(Clone)]
pub struct ValueRefToStoredValue<'eval> {
    element_ref: Option<ElementRef<'eval, Value>>,
    value: ValueRef<'eval>,
}

impl<'eval> ValueRefToStoredValue<'eval> {
    /// Creates a new reference to a stored value.
    pub fn new(element_ref: Option<ElementRef<'eval, Value>>, value: ValueRef<'eval>) -> Self {
        Self { element_ref, value }
    }

    /// The type of the value.
    pub fn value_type(&self) -> ValueType {
        self.value.value_type()
    }

    /// Converts the reference to a [`Value`], which is guaranted to contain no references.
    pub fn to_owned(&self) -> Value {
        self.value.to_owned()
    }

    /// Returns a reference to the internal reference.
    pub fn value_ref(&self) -> &ValueRef<'eval> {
        &self.value
    }

    /// Returns the list element at the given index.
    ///
    /// Copies the underlying [`ElementRef`] to the indexed value.
    pub fn list_index(&self, index: usize) -> Option<Self> {
        Some(Self::new(
            self.element_ref.clone(),
            self.value.list_index(index)?,
        ))
    }
}

/// A [`Value`], but containing a mutable reference to the underlying value.
pub enum ValueRefMut<'eval> {
    Undefined(&'eval mut Value),
    Boolean(&'eval mut bool),
    OptionalBoolean(&'eval mut Option<bool>),
    I8(&'eval mut i8),
    U16(&'eval mut u16),
    U32(&'eval mut u32),
    U64(&'eval mut u64),
    I64(&'eval mut i64),
    OptionalISize(&'eval mut Option<isize>),
    OptionalU16(&'eval mut Option<u16>),
    Fraction(&'eval mut Fraction<i64>),
    UFraction(&'eval mut Fraction<u64>),
    String(&'eval mut String),
    OptionalString(&'eval mut Option<String>),
    Mon(&'eval mut MonHandle),
    Effect(&'eval mut EffectHandle),
    ActiveMove(&'eval mut MoveHandle),
    MoveCategory(&'eval mut MoveCategory),
    MoveTarget(&'eval mut MoveTarget),
    Type(&'eval mut Type),
    Boost(&'eval mut Boost),
    BoostTable(&'eval mut BoostTable),
    OptionalBoostTable(&'eval mut Option<BoostTable>),
    Side(&'eval mut usize),
    MoveSlot(&'eval mut MoveSlot),
    Player(&'eval mut usize),
    Accuracy(&'eval mut Accuracy),
    Field,
    HitEffect(&'eval mut HitEffect),
    OptionalHitEffect(&'eval mut Option<HitEffect>),
    EffectState(&'eval mut DynamicEffectStateConnector),
    TempEffectState(DynamicEffectStateConnector),
    List(&'eval mut Vec<Value>),
    Object(&'eval mut FastHashMap<String, Value>),
}

impl<'eval> ValueRefMut<'eval> {
    /// The type of the value.
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Undefined(_) => ValueType::Undefined,
            Self::Boolean(_) => ValueType::Boolean,
            Self::OptionalBoolean(_) => ValueType::Boolean,
            Self::I8(_) => ValueType::Fraction,
            Self::U16(_) => ValueType::UFraction,
            Self::U32(_) => ValueType::UFraction,
            Self::U64(_) => ValueType::UFraction,
            Self::I64(_) => ValueType::Fraction,
            Self::OptionalISize(_) => ValueType::Fraction,
            Self::OptionalU16(_) => ValueType::Fraction,
            Self::Fraction(_) => ValueType::Fraction,
            Self::UFraction(_) => ValueType::UFraction,
            Self::String(_) => ValueType::String,
            Self::OptionalString(_) => ValueType::String,
            Self::Mon(_) => ValueType::Mon,
            Self::Effect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::Type(_) => ValueType::Type,
            Self::Boost(_) => ValueType::Boost,
            Self::BoostTable(_) => ValueType::BoostTable,
            Self::OptionalBoostTable(_) => ValueType::BoostTable,
            Self::Side(_) => ValueType::Side,
            Self::MoveSlot(_) => ValueType::MoveSlot,
            Self::Player(_) => ValueType::Player,
            Self::Accuracy(_) => ValueType::Accuracy,
            Self::Field => ValueType::Field,
            Self::HitEffect(_) => ValueType::HitEffect,
            Self::OptionalHitEffect(_) => ValueType::HitEffect,
            Self::EffectState(_) => ValueType::EffectState,
            Self::TempEffectState(_) => ValueType::EffectState,
            Self::List(_) => ValueType::List,
            Self::Object(_) => ValueType::Object,
        }
    }
}

impl<'eval> From<&'eval mut Value> for ValueRefMut<'eval> {
    fn from(value: &'eval mut Value) -> Self {
        match value {
            Value::Undefined => Self::Undefined(value),
            Value::Boolean(val) => Self::Boolean(val),
            Value::Fraction(val) => Self::Fraction(val),
            Value::UFraction(val) => Self::UFraction(val),
            Value::String(val) => Self::String(val),
            Value::Mon(val) => Self::Mon(val),
            Value::Effect(val) => Self::Effect(val),
            Value::ActiveMove(val) => Self::ActiveMove(val),
            Value::MoveCategory(val) => Self::MoveCategory(val),
            Value::MoveTarget(val) => Self::MoveTarget(val),
            Value::Type(val) => Self::Type(val),
            Value::Boost(val) => Self::Boost(val),
            Value::BoostTable(val) => Self::BoostTable(val),
            Value::Side(val) => Self::Side(val),
            Value::MoveSlot(val) => Self::MoveSlot(val),
            Value::Player(val) => Self::Player(val),
            Value::Accuracy(val) => Self::Accuracy(val),
            Value::Field => Self::Field,
            Value::HitEffect(val) => Self::HitEffect(val),
            Value::EffectState(val) => Self::EffectState(val),
            Value::List(val) => Self::List(val),
            Value::Object(val) => Self::Object(val),
        }
    }
}

/// The value type used for operations.
///
/// Practically a union of [`MaybeReferenceValue`] and [`ValueRef`]. This type is needed because
/// there is a distinction between owned values and reference values. For example, a program may
/// compare a list stored as a variable (consisting of [`Value`] objects) and a list generated at
/// runtime that is not stored as a variable (consisting of [`MaybeReferenceValue`] objects). This
/// type allows these two lists to be operated on directly, without needing to allocate memory for
/// an extra list for either one.
///
/// Primitive types are always passed by value. More complex types, like lists and objects, are
/// passed by reference.
pub enum MaybeReferenceValueForOperation<'eval> {
    Undefined,
    Boolean(bool),
    Fraction(Fraction<i64>),
    UFraction(Fraction<u64>),
    String(&'eval String),
    Str(&'eval str),
    TempString(String),
    Mon(MonHandle),
    Effect(&'eval EffectHandle),
    TempEffect(EffectHandle),
    ActiveMove(MoveHandle),
    MoveCategory(MoveCategory),
    MoveTarget(MoveTarget),
    Type(Type),
    Boost(Boost),
    BoostTable(&'eval BoostTable),
    Side(usize),
    MoveSlot(&'eval MoveSlot),
    Player(usize),
    Accuracy(Accuracy),
    Field,
    HitEffect(&'eval HitEffect),
    EffectState(DynamicEffectStateConnector),
    List(&'eval Vec<MaybeReferenceValue<'eval>>),
    StoredList(&'eval Vec<Value>),
    TempList(Vec<MaybeReferenceValue<'eval>>),
    Object(&'eval FastHashMap<String, MaybeReferenceValue<'eval>>),
    StoredObject(&'eval FastHashMap<String, Value>),
}

impl<'eval> MaybeReferenceValueForOperation<'eval> {
    /// The type of the value.
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Undefined => ValueType::Undefined,
            Self::Boolean(_) => ValueType::Boolean,
            Self::Fraction(_) => ValueType::Fraction,
            Self::UFraction(_) => ValueType::UFraction,
            Self::String(_) => ValueType::String,
            Self::Str(_) => ValueType::String,
            Self::TempString(_) => ValueType::String,
            Self::Mon(_) => ValueType::Mon,
            Self::Effect(_) => ValueType::Effect,
            Self::TempEffect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::Type(_) => ValueType::Type,
            Self::Boost(_) => ValueType::Boost,
            Self::BoostTable(_) => ValueType::BoostTable,
            Self::Side(_) => ValueType::Side,
            Self::MoveSlot(_) => ValueType::MoveSlot,
            Self::Player(_) => ValueType::Player,
            Self::Accuracy(_) => ValueType::Accuracy,
            Self::Field => ValueType::Field,
            Self::HitEffect(_) => ValueType::HitEffect,
            Self::EffectState(_) => ValueType::EffectState,
            Self::List(_) => ValueType::List,
            Self::StoredList(_) => ValueType::List,
            Self::TempList(_) => ValueType::List,
            Self::Object(_) => ValueType::Object,
            Self::StoredObject(_) => ValueType::Object,
        }
    }

    /// Converts the value to a [`Value`], which is guaranteed to contain no references.
    pub fn to_owned(&self) -> Value {
        match self {
            Self::Undefined => Value::Undefined,
            Self::Boolean(val) => Value::Boolean(*val),
            Self::Fraction(val) => Value::Fraction(*val),
            Self::UFraction(val) => Value::UFraction(*val),
            Self::String(val) => Value::String((*val).clone()),
            Self::Str(val) => Value::String(val.to_string()),
            Self::TempString(val) => Value::String(val.clone()),
            Self::Mon(val) => Value::Mon(*val),
            Self::Effect(val) => Value::Effect((*val).clone()),
            Self::TempEffect(val) => Value::Effect(val.clone()),
            Self::ActiveMove(val) => Value::ActiveMove(*val),
            Self::MoveCategory(val) => Value::MoveCategory(*val),
            Self::MoveTarget(val) => Value::MoveTarget(*val),
            Self::Type(val) => Value::Type(*val),
            Self::Boost(val) => Value::Boost(*val),
            Self::BoostTable(val) => Value::BoostTable((*val).clone()),
            Self::Side(val) => Value::Side(*val),
            Self::MoveSlot(val) => Value::MoveSlot((*val).clone()),
            Self::Player(val) => Value::Player(*val),
            Self::Accuracy(val) => Value::Accuracy(*val),
            Self::Field => Value::Field,
            Self::HitEffect(val) => Value::HitEffect((*val).clone()),
            Self::EffectState(val) => Value::EffectState(val.clone()),
            Self::List(val) => Value::List(val.iter().map(|val| val.to_owned()).collect()),
            Self::StoredList(val) => Value::List((*val).clone()),
            Self::TempList(val) => Value::List(val.into_iter().map(|val| val.to_owned()).collect()),
            Self::Object(val) => Value::Object(
                val.iter()
                    .map(|(key, val)| (key.clone(), val.to_owned()))
                    .collect(),
            ),
            Self::StoredObject(val) => Value::Object((*val).clone()),
        }
    }

    fn internal_type_index(&self) -> usize {
        match self {
            Self::Undefined => 0,
            Self::Boolean(_) => 1,
            Self::Fraction(_) => 32,
            Self::UFraction(_) => 33,
            Self::String(_) => 64,
            Self::Str(_) => 65,
            Self::TempString(_) => 66,
            Self::Mon(_) => 100,
            Self::Effect(_) => 101,
            Self::TempEffect(_) => 102,
            Self::ActiveMove(_) => 103,
            Self::MoveCategory(_) => 104,
            Self::MoveTarget(_) => 105,
            Self::Type(_) => 106,
            Self::Boost(_) => 107,
            Self::BoostTable(_) => 108,
            Self::Side(_) => 109,
            Self::MoveSlot(_) => 110,
            Self::Player(_) => 111,
            Self::Accuracy(_) => 112,
            Self::Field => 113,
            Self::HitEffect(_) => 115,
            Self::EffectState(_) => 175,
            Self::List(_) => 200,
            Self::StoredList(_) => 201,
            Self::TempList(_) => 202,
            Self::Object(_) => 250,
            Self::StoredObject(_) => 251,
        }
    }

    fn sort_for_commutative_operation(a: Self, b: Self) -> (Self, Self) {
        if a.internal_type_index() < b.internal_type_index() {
            (a, b)
        } else {
            (b, a)
        }
    }

    fn sort_for_commutative_operation_ref(
        a: &'eval Self,
        b: &'eval Self,
    ) -> (&'eval Self, &'eval Self) {
        if a.internal_type_index() < b.internal_type_index() {
            (a, b)
        } else {
            (b, a)
        }
    }

    fn invalid_binary_operation(operation: &str, lhs: ValueType, rhs: ValueType) -> Error {
        battler_error!("cannot {operation} {lhs} and {rhs}")
    }

    /// Implements negation.
    ///
    /// For boolean coercion, all values coerce to `true` except for:
    /// - `undefined`
    /// - `false`
    /// - `0`
    pub fn negate(self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match self {
            Self::Undefined => MaybeReferenceValue::Boolean(true),
            Self::Boolean(val) => MaybeReferenceValue::Boolean(!val),
            val @ _ if self.value_type().is_number() => val.equal(
                MaybeReferenceValueForOperation::UFraction(Fraction::from(0u32)),
            )?,
            _ => MaybeReferenceValue::Boolean(false),
        };
        Ok(result)
    }

    /// Implements multiplication.
    pub fn multiply(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match Self::sort_for_commutative_operation(self, rhs) {
            (Self::Fraction(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(lhs.wrapping_mul(&rhs))
            }
            (Self::Fraction(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::Fraction(lhs.wrapping_mul(
                    &Fraction::try_convert(rhs).wrap_error_with_message("integer overflow")?,
                ))
            }
            (Self::UFraction(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(lhs.wrapping_mul(&rhs))
            }
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "multiply",
                    lhs.value_type(),
                    rhs.value_type(),
                ))
            }
        };
        Ok(result)
    }

    /// Implements division.
    pub fn divide(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match (self, rhs) {
            (Self::Fraction(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(lhs / rhs),
            (Self::Fraction(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs / rhs
                    .try_convert()
                    .wrap_error_with_message("integer overflow")?,
            ),
            (Self::UFraction(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs.try_convert()
                    .wrap_error_with_message("integer overflow")?
                    / rhs,
            ),
            (Self::UFraction(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(lhs / rhs)
            }
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "divide",
                    lhs.value_type(),
                    rhs.value_type(),
                ))
            }
        };
        Ok(result)
    }

    /// Implements modulo.
    pub fn modulo(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match (self, rhs) {
            (Self::Fraction(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(Fraction::from(lhs.floor() % rhs.floor()))
            }
            (Self::Fraction(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::Fraction(Fraction::from(
                    lhs.floor()
                        % TryInto::<i64>::try_into(rhs.floor())
                            .wrap_error_with_message("integer overflow")?,
                ))
            }
            (Self::UFraction(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(Fraction::from(
                    TryInto::<i64>::try_into(lhs.floor())
                        .wrap_error_with_message("integer overflow")?
                        % rhs.floor(),
                ))
            }
            (Self::UFraction(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(Fraction::from(lhs.floor() as u64 % rhs.floor()))
            }
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "modulo",
                    lhs.value_type(),
                    rhs.value_type(),
                ))
            }
        };
        Ok(result)
    }

    /// Implements addition.
    pub fn add(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match Self::sort_for_commutative_operation(self, rhs) {
            (Self::Fraction(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(lhs.wrapping_add(&rhs))
            }
            (Self::Fraction(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs.wrapping_add(
                    &rhs.try_convert()
                        .wrap_error_with_message("integer overflow")?,
                ),
            ),
            (Self::UFraction(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(lhs.wrapping_add(&rhs))
            }
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "add",
                    lhs.value_type(),
                    rhs.value_type(),
                ))
            }
        };
        Ok(result)
    }

    /// Implements subtraction.
    pub fn subtract(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match (self, rhs) {
            (Self::Fraction(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(lhs.wrapping_sub(&rhs))
            }
            (Self::Fraction(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs.wrapping_sub(
                    &rhs.try_convert()
                        .wrap_error_with_message("integer overflow")?,
                ),
            ),
            (Self::UFraction(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs.try_convert()
                    .wrap_error_with_message("integer overflow")?
                    .wrapping_sub(&rhs),
            ),
            (Self::UFraction(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(lhs.wrapping_sub(&rhs))
            }
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "subtract",
                    lhs.value_type(),
                    rhs.value_type(),
                ))
            }
        };
        Ok(result)
    }

    fn compare_ref(&'eval self, rhs: &'eval Self) -> Result<Ordering, Error> {
        let result = match (self, rhs) {
            (Self::Fraction(lhs), Self::Fraction(rhs)) => lhs.partial_cmp(rhs),
            (Self::Fraction(lhs), Self::UFraction(rhs)) => {
                if lhs < &0 {
                    Some(Ordering::Less)
                } else {
                    Fraction::new(lhs.numerator().abs() as u64, lhs.denominator().abs() as u64)
                        .partial_cmp(rhs)
                }
            }
            (Self::UFraction(lhs), Self::Fraction(rhs)) => {
                if rhs < &0 {
                    Some(Ordering::Greater)
                } else {
                    lhs.partial_cmp(&Fraction::new(
                        rhs.numerator().abs() as u64,
                        rhs.denominator().abs() as u64,
                    ))
                }
            }
            (Self::UFraction(lhs), Self::UFraction(rhs)) => lhs.partial_cmp(rhs),
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "compare",
                    lhs.value_type(),
                    rhs.value_type(),
                ))
            }
        };
        result.wrap_error_with_message("comparison yielded no result")
    }

    /// Implements less than comparison.
    pub fn less_than(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        Ok(MaybeReferenceValue::Boolean(
            self.compare_ref(&rhs)?.is_lt(),
        ))
    }

    /// Implements greater than comparison.
    pub fn greater_than(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        Ok(MaybeReferenceValue::Boolean(
            self.compare_ref(&rhs)?.is_gt(),
        ))
    }

    /// Implements less than or equal to comparison.
    pub fn less_than_or_equal(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        Ok(MaybeReferenceValue::Boolean(
            self.compare_ref(&rhs)?.is_le(),
        ))
    }

    /// Implements greater than or equal to comparison.
    pub fn greater_than_or_equal(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        Ok(MaybeReferenceValue::Boolean(
            self.compare_ref(&rhs)?.is_ge(),
        ))
    }

    fn equal_lists<T, U>(lhs: &'eval Vec<T>, rhs: &'eval Vec<U>) -> Result<bool, Error>
    where
        &'eval T: Into<Self> + 'eval,
        &'eval U: Into<Self> + 'eval,
    {
        Ok(lhs.len() == rhs.len()
            && lhs
                .iter()
                .map(|a| Into::<Self>::into(a))
                .zip(rhs.iter().map(|b| b.into()))
                .map(|(lhs, rhs)| lhs.equal_ref(&rhs))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .all(|eq| eq))
    }

    fn equal_objects<T, U>(
        lhs: &'eval FastHashMap<String, T>,
        rhs: &'eval FastHashMap<String, U>,
    ) -> Result<bool, Error>
    where
        &'eval T: Into<Self> + 'eval,
        &'eval U: Into<Self> + 'eval,
    {
        Ok(lhs.len() == rhs.len()
            && lhs
                .iter()
                .map(|(key, lhs)| match rhs.get(key) {
                    None => Ok(false),
                    Some(rhs) => Into::<Self>::into(lhs).equal_ref(&rhs.into()),
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .all(|eq| eq))
    }

    fn equal_ref(&'eval self, rhs: &'eval Self) -> Result<bool, Error> {
        let result = match Self::sort_for_commutative_operation_ref(self, rhs) {
            (Self::Undefined, Self::Undefined) => true,
            (Self::Undefined, _) => false,
            (Self::Boolean(lhs), Self::Boolean(rhs)) => lhs.eq(rhs),
            (lhs @ Self::Fraction(_), rhs @ Self::Fraction(_))
            | (lhs @ Self::Fraction(_), rhs @ Self::UFraction(_))
            | (lhs @ Self::UFraction(_), rhs @ Self::UFraction(_)) => lhs.compare_ref(rhs)?.is_eq(),
            (Self::Fraction(lhs), Self::Accuracy(rhs)) => rhs
                .percentage()
                .is_some_and(|rhs| lhs.eq(&Fraction::from(rhs as i32))),
            (Self::UFraction(lhs), Self::Accuracy(rhs)) => rhs
                .percentage()
                .is_some_and(|rhs| lhs.eq(&Fraction::from(rhs as u32))),
            (Self::String(lhs), Self::String(rhs)) => lhs.eq(rhs),
            (Self::String(lhs), Self::Str(rhs)) => lhs.eq(rhs),
            (Self::String(lhs), Self::TempString(rhs)) => lhs.eq(&rhs),
            (Self::String(lhs), Self::Effect(rhs)) => rhs
                .try_id()
                .map(|id| id.as_ref() == lhs.as_str())
                .unwrap_or(false),
            (Self::String(lhs), Self::MoveCategory(rhs)) => {
                MoveCategory::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::MoveTarget(rhs)) => {
                MoveTarget::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::Type(rhs)) => {
                Type::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::Boost(rhs)) => {
                Boost::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::Accuracy(rhs)) => {
                Accuracy::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::Str(rhs)) => lhs.eq(rhs),
            (Self::Str(lhs), Self::TempString(rhs)) => lhs.eq(&rhs),
            (Self::Str(lhs), Self::Effect(rhs)) => {
                rhs.try_id().map(|id| id.as_ref() == *lhs).unwrap_or(false)
            }
            (Self::Str(lhs), Self::MoveCategory(rhs)) => {
                MoveCategory::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::MoveTarget(rhs)) => {
                MoveTarget::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::Type(rhs)) => Type::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs)),
            (Self::Str(lhs), Self::Boost(rhs)) => Boost::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs)),
            (Self::Str(lhs), Self::Accuracy(rhs)) => {
                Accuracy::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::TempString(rhs)) => lhs.eq(rhs),
            (Self::TempString(lhs), Self::Effect(rhs)) => rhs
                .try_id()
                .map(|id| id.as_ref() == lhs.as_str())
                .unwrap_or(false),
            (Self::TempString(lhs), Self::MoveCategory(rhs)) => {
                MoveCategory::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::MoveTarget(rhs)) => {
                MoveTarget::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::Type(rhs)) => {
                Type::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::Boost(rhs)) => {
                Boost::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::Accuracy(rhs)) => {
                Accuracy::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Mon(lhs), Self::Mon(rhs)) => lhs.eq(rhs),
            (Self::Effect(lhs), Self::Effect(rhs)) => lhs.eq(rhs),
            (Self::Effect(lhs), Self::TempEffect(rhs)) => lhs.eq(&rhs),
            (Self::TempEffect(lhs), Self::TempEffect(rhs)) => lhs.eq(rhs),
            (Self::ActiveMove(lhs), Self::ActiveMove(rhs)) => lhs.eq(rhs),
            (Self::MoveCategory(lhs), Self::MoveCategory(rhs)) => lhs.eq(rhs),
            (Self::MoveTarget(lhs), Self::MoveTarget(rhs)) => lhs.eq(rhs),
            (Self::Type(lhs), Self::Type(rhs)) => lhs.eq(rhs),
            (Self::Boost(lhs), Self::Boost(rhs)) => lhs.eq(rhs),
            (Self::BoostTable(lhs), Self::BoostTable(rhs)) => lhs.eq(rhs),
            (Self::Side(lhs), Self::Side(rhs)) => lhs.eq(rhs),
            (Self::MoveSlot(lhs), Self::MoveSlot(rhs)) => lhs.eq(rhs),
            (Self::Player(lhs), Self::Player(rhs)) => lhs.eq(rhs),
            (Self::Accuracy(lhs), Self::Accuracy(rhs)) => lhs.eq(rhs),
            (Self::Field, Self::Field) => true,
            (Self::HitEffect(lhs), Self::HitEffect(rhs)) => lhs.eq(rhs),
            (Self::List(lhs), Self::List(rhs)) => Self::equal_lists(lhs, rhs)?,
            (Self::List(lhs), Self::StoredList(rhs)) => Self::equal_lists(lhs, rhs)?,
            (Self::List(lhs), Self::TempList(rhs)) => Self::equal_lists(lhs, rhs)?,
            (Self::StoredList(lhs), Self::StoredList(rhs)) => Self::equal_lists(lhs, rhs)?,
            (Self::StoredList(lhs), Self::TempList(rhs)) => Self::equal_lists(lhs, rhs)?,
            (Self::TempList(lhs), Self::TempList(rhs)) => Self::equal_lists(lhs, rhs)?,
            (Self::Object(lhs), Self::Object(rhs)) => Self::equal_objects(lhs, rhs)?,
            (Self::Object(lhs), Self::StoredObject(rhs)) => Self::equal_objects(lhs, rhs)?,
            (Self::StoredObject(lhs), Self::StoredObject(rhs)) => Self::equal_objects(lhs, rhs)?,
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "check equality of",
                    lhs.value_type(),
                    rhs.value_type(),
                ))
            }
        };
        Ok(result)
    }

    /// Implements equality.
    pub fn equal(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        Ok(MaybeReferenceValue::Boolean(self.equal_ref(&rhs)?))
    }

    /// Implements inequality.
    pub fn not_equal(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        Ok(MaybeReferenceValue::Boolean(!self.equal_ref(&rhs)?))
    }

    fn list_has_value<'a, T>(list: &'a Vec<T>, rhs: Self) -> bool
    where
        &'a T: Into<MaybeReferenceValueForOperation<'a>>,
    {
        list.iter()
            .map(|val| Into::<MaybeReferenceValueForOperation<'a>>::into(val))
            .any(|lhs| lhs.equal_ref(&rhs).is_ok_and(|eq| eq))
    }

    /// Implements list lookup.
    pub fn has(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match (self, rhs) {
            (Self::List(lhs), rhs @ _) => Self::list_has_value(lhs, rhs),
            (Self::StoredList(lhs), rhs @ _) => Self::list_has_value(lhs, rhs),
            (Self::TempList(lhs), rhs @ _) => Self::list_has_value(&lhs, rhs),
            _ => {
                return Err(battler_error!(
                    "left-hand side of has operator must be a list"
                ));
            }
        };
        Ok(MaybeReferenceValue::Boolean(result))
    }

    fn list_has_any_value<'a, 'b, T, U>(lhs: &'a Vec<T>, rhs: &'b Vec<U>) -> bool
    where
        &'a T: Into<MaybeReferenceValueForOperation<'a>>,
        &'b U: Into<MaybeReferenceValueForOperation<'b>>,
    {
        lhs.iter()
            .map(|a| Into::<MaybeReferenceValueForOperation<'a>>::into(a))
            .any(|lhs| MaybeReferenceValueForOperation::list_has_value(rhs, lhs))
    }

    /// Implements list subset check.
    pub fn has_any(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match (self, rhs) {
            (Self::List(lhs), Self::List(rhs)) => Self::list_has_any_value(lhs, rhs),
            (Self::List(lhs), Self::StoredList(rhs)) => Self::list_has_any_value(lhs, rhs),
            (Self::List(lhs), Self::TempList(rhs)) => Self::list_has_any_value(lhs, &rhs),
            (Self::StoredList(lhs), Self::List(rhs)) => Self::list_has_any_value(lhs, rhs),
            (Self::StoredList(lhs), Self::StoredList(rhs)) => Self::list_has_any_value(lhs, rhs),
            (Self::StoredList(lhs), Self::TempList(rhs)) => Self::list_has_any_value(lhs, &rhs),
            (Self::TempList(lhs), Self::List(rhs)) => Self::list_has_any_value(&lhs, rhs),
            (Self::TempList(lhs), Self::StoredList(rhs)) => Self::list_has_any_value(&lhs, rhs),
            (Self::TempList(lhs), Self::TempList(rhs)) => Self::list_has_any_value(&lhs, &rhs),
            _ => {
                return Err(battler_error!(
                    "both operands to hasany operator must be a list"
                ));
            }
        };
        Ok(MaybeReferenceValue::Boolean(result))
    }

    /// Implements boolean conjunction.
    pub fn and(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match (self, rhs) {
            (Self::Boolean(lhs), Self::Boolean(rhs)) => lhs && rhs,
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "and",
                    lhs.value_type(),
                    rhs.value_type(),
                ))
            }
        };
        Ok(MaybeReferenceValue::Boolean(result))
    }

    /// Implements boolean disjunction.
    pub fn or(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match (self, rhs) {
            (Self::Boolean(lhs), Self::Boolean(rhs)) => lhs || rhs,
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "or",
                    lhs.value_type(),
                    rhs.value_type(),
                ))
            }
        };
        Ok(MaybeReferenceValue::Boolean(result))
    }

    /// Converts the value to a string for a formatted string.
    pub fn for_formatted_string(&self) -> Result<String, Error> {
        let string = match self {
            Self::Boolean(val) => {
                if *val {
                    "true".to_owned()
                } else {
                    "false".to_owned()
                }
            }
            Self::Fraction(val) => val.to_string(),
            Self::UFraction(val) => val.to_string(),
            Self::String(val) => (*val).clone(),
            Self::Str(val) => val.to_string(),
            Self::TempString(val) => val.clone(),
            _ => {
                return Err(battler_error!(
                    "{} value is not string formattable",
                    self.value_type()
                ))
            }
        };
        Ok(string)
    }
}

impl<'eval> From<&'eval Value> for MaybeReferenceValueForOperation<'eval> {
    fn from(value: &'eval Value) -> Self {
        match value {
            Value::Undefined => Self::Undefined,
            Value::Boolean(val) => Self::Boolean(*val),
            Value::Fraction(val) => Self::Fraction(*val),
            Value::UFraction(val) => Self::UFraction(*val),
            Value::String(val) => Self::String(val),
            Value::Mon(val) => Self::Mon(*val),
            Value::Effect(val) => Self::Effect(val),
            Value::ActiveMove(val) => Self::ActiveMove(*val),
            Value::MoveCategory(val) => Self::MoveCategory(*val),
            Value::MoveTarget(val) => Self::MoveTarget(*val),
            Value::Type(val) => Self::Type(*val),
            Value::Boost(val) => Self::Boost(*val),
            Value::BoostTable(val) => Self::BoostTable(val),
            Value::Side(val) => Self::Side(*val),
            Value::MoveSlot(val) => Self::MoveSlot(val),
            Value::Player(val) => Self::Player(*val),
            Value::Accuracy(val) => Self::Accuracy(*val),
            Value::Field => Self::Field,
            Value::HitEffect(val) => Self::HitEffect(val),
            Value::EffectState(val) => Self::EffectState(val.clone()),
            Value::List(val) => Self::StoredList(val),
            Value::Object(val) => Self::StoredObject(val),
        }
    }
}

impl<'eval> From<&'eval MaybeReferenceValue<'eval>> for MaybeReferenceValueForOperation<'eval> {
    fn from(value: &'eval MaybeReferenceValue<'eval>) -> Self {
        match value {
            MaybeReferenceValue::Undefined => Self::Undefined,
            MaybeReferenceValue::Boolean(val) => Self::Boolean(*val),
            MaybeReferenceValue::Fraction(val) => Self::Fraction(*val),
            MaybeReferenceValue::UFraction(val) => Self::UFraction(*val),
            MaybeReferenceValue::String(val) => Self::String(val),
            MaybeReferenceValue::Mon(val) => Self::Mon(*val),
            MaybeReferenceValue::Effect(val) => Self::Effect(val),
            MaybeReferenceValue::ActiveMove(val) => Self::ActiveMove(*val),
            MaybeReferenceValue::MoveCategory(val) => Self::MoveCategory(*val),
            MaybeReferenceValue::MoveTarget(val) => Self::MoveTarget(*val),
            MaybeReferenceValue::Type(val) => Self::Type(*val),
            MaybeReferenceValue::Boost(val) => Self::Boost(*val),
            MaybeReferenceValue::BoostTable(val) => Self::BoostTable(val),
            MaybeReferenceValue::Side(val) => Self::Side(*val),
            MaybeReferenceValue::MoveSlot(val) => Self::MoveSlot(val),
            MaybeReferenceValue::Player(val) => Self::Player(*val),
            MaybeReferenceValue::Accuracy(val) => Self::Accuracy(*val),
            MaybeReferenceValue::Field => Self::Field,
            MaybeReferenceValue::HitEffect(val) => Self::HitEffect(val),
            MaybeReferenceValue::EffectState(val) => Self::EffectState(val.clone()),
            MaybeReferenceValue::List(val) => Self::List(val),
            MaybeReferenceValue::Object(val) => Self::Object(val),
            MaybeReferenceValue::Reference(val) => Self::from(val),
        }
    }
}

impl<'eval> From<ValueRef<'eval>> for MaybeReferenceValueForOperation<'eval> {
    fn from(value: ValueRef<'eval>) -> Self {
        match value {
            ValueRef::Undefined => Self::Undefined,
            ValueRef::Boolean(val) => Self::Boolean(val),
            ValueRef::Fraction(val) => Self::Fraction(val),
            ValueRef::UFraction(val) => Self::UFraction(val),
            ValueRef::String(val) => Self::String(val),
            ValueRef::Str(val) => Self::Str(val),
            ValueRef::TempString(val) => Self::TempString(val),
            ValueRef::Mon(val) => Self::Mon(val),
            ValueRef::Effect(val) => Self::Effect(val),
            ValueRef::TempEffect(val) => Self::TempEffect(val),
            ValueRef::ActiveMove(val) => Self::ActiveMove(val),
            ValueRef::MoveCategory(val) => Self::MoveCategory(val),
            ValueRef::MoveTarget(val) => Self::MoveTarget(val),
            ValueRef::Type(val) => Self::Type(val),
            ValueRef::Boost(val) => Self::Boost(val),
            ValueRef::BoostTable(val) => Self::BoostTable(val),
            ValueRef::Side(val) => Self::Side(val),
            ValueRef::MoveSlot(val) => Self::MoveSlot(val),
            ValueRef::Player(val) => Self::Player(val),
            ValueRef::Accuracy(val) => Self::Accuracy(val),
            ValueRef::Field => Self::Field,
            ValueRef::HitEffect(val) => Self::HitEffect(val),
            ValueRef::EffectState(val) => Self::EffectState(val),
            ValueRef::List(val) => Self::StoredList(val),
            ValueRef::TempList(val) => Self::TempList(
                val.into_iter()
                    .map(|val| MaybeReferenceValue::from(val))
                    .collect(),
            ),
            ValueRef::Object(val) => Self::StoredObject(val),
        }
    }
}

impl<'eval> From<&'eval ValueRefToStoredValue<'eval>> for MaybeReferenceValueForOperation<'eval> {
    fn from(value: &'eval ValueRefToStoredValue<'eval>) -> Self {
        match &value.value {
            ValueRef::Undefined => Self::Undefined,
            ValueRef::Boolean(val) => Self::Boolean(*val),
            ValueRef::Fraction(val) => Self::Fraction(*val),
            ValueRef::UFraction(val) => Self::UFraction(*val),
            ValueRef::String(val) => Self::String(val),
            ValueRef::Str(val) => Self::Str(val),
            ValueRef::TempString(val) => Self::TempString(val.clone()),
            ValueRef::Mon(val) => Self::Mon(*val),
            ValueRef::Effect(val) => Self::Effect(val),
            ValueRef::TempEffect(val) => Self::Effect(val),
            ValueRef::ActiveMove(val) => Self::ActiveMove(*val),
            ValueRef::MoveCategory(val) => Self::MoveCategory(*val),
            ValueRef::MoveTarget(val) => Self::MoveTarget(*val),
            ValueRef::Type(val) => Self::Type(*val),
            ValueRef::Boost(val) => Self::Boost(*val),
            ValueRef::BoostTable(val) => Self::BoostTable(val),
            ValueRef::Side(val) => Self::Side(*val),
            ValueRef::MoveSlot(val) => Self::MoveSlot(val),
            ValueRef::Player(val) => Self::Player(*val),
            ValueRef::Accuracy(val) => Self::Accuracy(*val),
            ValueRef::Field => Self::Field,
            ValueRef::HitEffect(val) => Self::HitEffect(val),
            ValueRef::EffectState(val) => Self::EffectState(val.clone()),
            ValueRef::List(val) => Self::StoredList(val),
            ValueRef::TempList(val) => Self::TempList(
                (0..val.len())
                    .map(|i| MaybeReferenceValue::from(value.list_index(i).unwrap()))
                    .collect(),
            ),
            ValueRef::Object(val) => Self::StoredObject(val),
        }
    }
}
