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
        MonHandle,
        MoveEventResult,
        MoveHandle,
        MoveOutcomeOnTarget,
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
        fxlang::EvaluationContext,
        EffectHandle,
    },
    mons::Type,
    moves::{
        MoveCategory,
        MoveTarget,
    },
};

/// The type of an fxlang value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Undefined,
    Boolean,
    U16,
    U32,
    U64,
    I64,
    Fraction,
    UFraction,
    String,
    Mon,
    Effect,
    ActiveMove,
    MoveCategory,
    MoveTarget,
    Type,
    List,
    Object,

    OptionalISize,
}

impl ValueType {
    /// Checks if the value type is a number.
    pub fn is_number(&self) -> bool {
        match self {
            Self::U16 | Self::U32 | Self::U64 | Self::Fraction | Self::UFraction => true,
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
    U16(u16),
    U32(u32),
    U64(u64),
    I64(i64),
    Fraction(Fraction<i32>),
    UFraction(Fraction<u32>),
    String(String),
    Mon(MonHandle),
    Effect(EffectHandle),
    ActiveMove(MoveHandle),
    MoveCategory(MoveCategory),
    MoveTarget(MoveTarget),
    Type(Type),
    List(Vec<Value>),
    Object(FastHashMap<String, Value>),
}

impl Value {
    /// The type of the value.
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Undefined => ValueType::Undefined,
            Self::Boolean(_) => ValueType::Boolean,
            Self::U16(_) => ValueType::U16,
            Self::U32(_) => ValueType::U32,
            Self::U64(_) => ValueType::U64,
            Self::I64(_) => ValueType::I64,
            Self::Fraction(_) => ValueType::Fraction,
            Self::UFraction(_) => ValueType::UFraction,
            Self::String(_) => ValueType::String,
            Self::Mon(_) => ValueType::Mon,
            Self::Effect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::Type(_) => ValueType::Type,
            Self::List(_) => ValueType::List,
            Self::Object(_) => ValueType::Object,
        }
    }

    fn invalid_type(got: ValueType, expected: ValueType) -> Error {
        battler_error!("got {got}, expected {expected}")
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
            Self::U16(val) => Ok(val as u64),
            Self::U32(val) => Ok(val as u64),
            Self::U64(val) => Ok(val),
            Self::I64(val) => Ok(val as u64),
            Self::Fraction(val) => Ok(val.floor() as u64),
            Self::UFraction(val) => Ok(val.floor() as u64),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::U64)),
        }
    }

    /// Consumes the value into a [`i64`].
    pub fn integer_i64(self) -> Result<i64, Error> {
        match self {
            Self::U16(val) => Ok(val as i64),
            Self::U32(val) => Ok(val as i64),
            Self::U64(val) => val.try_into().wrap_error_with_message("integer overflow"),
            Self::I64(val) => Ok(val),
            Self::Fraction(val) => Ok(val.floor() as i64),
            Self::UFraction(val) => Ok(val.floor() as i64),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::I64)),
        }
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
            Self::ActiveMove(val) => Ok(context.active_move(val)?.id().clone()),
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
    U16(u16),
    U32(u32),
    U64(u64),
    I64(i64),
    Fraction(Fraction<i32>),
    UFraction(Fraction<u32>),
    String(String),
    Mon(MonHandle),
    Effect(EffectHandle),
    ActiveMove(MoveHandle),
    MoveCategory(MoveCategory),
    MoveTarget(MoveTarget),
    Type(Type),
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
            Self::U16(_) => ValueType::U16,
            Self::U32(_) => ValueType::U32,
            Self::U64(_) => ValueType::U64,
            Self::I64(_) => ValueType::I64,
            Self::Fraction(_) => ValueType::Fraction,
            Self::UFraction(_) => ValueType::UFraction,
            Self::String(_) => ValueType::String,
            Self::Mon(_) => ValueType::Mon,
            Self::Effect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::Type(_) => ValueType::Type,
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
            Self::U16(val) => Value::U16(*val),
            Self::U32(val) => Value::U32(*val),
            Self::U64(val) => Value::U64(*val),
            Self::I64(val) => Value::I64(*val),
            Self::Fraction(val) => Value::Fraction(*val),
            Self::UFraction(val) => Value::UFraction(*val),
            Self::String(val) => Value::String(val.clone()),
            Self::Mon(val) => Value::Mon(*val),
            Self::Effect(val) => Value::Effect(val.clone()),
            Self::ActiveMove(val) => Value::ActiveMove(*val),
            Self::MoveCategory(val) => Value::MoveCategory(*val),
            Self::MoveTarget(val) => Value::MoveTarget(*val),
            Self::Type(val) => Value::Type(*val),
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
    /// Returns a [`MaybeReferenceValueForOperation`], because the value can be a reference may come
    /// from a list generated in the statement or one stored in a variable.
    pub fn list_index<'value>(
        &'value self,
        index: usize,
    ) -> Option<MaybeReferenceValueForOperation<'value>> {
        match self {
            Self::List(list) => list
                .get(index)
                .map(|val| MaybeReferenceValueForOperation::from(val)),
            Self::Reference(reference) => reference
                .value_ref()
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
            Value::U16(val) => Self::U16(val),
            Value::U32(val) => Self::U32(val),
            Value::U64(val) => Self::U64(val),
            Value::I64(val) => Self::I64(val),
            Value::Fraction(val) => Self::Fraction(val),
            Value::UFraction(val) => Self::UFraction(val),
            Value::String(val) => Self::String(val),
            Value::Mon(val) => Self::Mon(val),
            Value::Effect(val) => Self::Effect(val),
            Value::ActiveMove(val) => Self::ActiveMove(val),
            Value::MoveCategory(val) => Self::MoveCategory(val),
            Value::MoveTarget(val) => Self::MoveTarget(val),
            Value::Type(val) => Self::Type(val),
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
    U16(u16),
    U32(u32),
    U64(u64),
    I64(i64),
    Fraction(Fraction<i32>),
    UFraction(Fraction<u32>),
    String(&'eval String),
    Str(&'eval str),
    TempString(String),
    Mon(MonHandle),
    Effect(EffectHandle),
    ActiveMove(MoveHandle),
    MoveCategory(MoveCategory),
    MoveTarget(MoveTarget),
    Type(Type),
    List(&'eval Vec<Value>),
    Object(&'eval FastHashMap<String, Value>),
}

impl ValueRef<'_> {
    /// The type of the value.
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Undefined => ValueType::Undefined,
            Self::Boolean(_) => ValueType::Boolean,
            Self::U16(_) => ValueType::U16,
            Self::U32(_) => ValueType::U32,
            Self::U64(_) => ValueType::U64,
            Self::I64(_) => ValueType::I64,
            Self::Fraction(_) => ValueType::Fraction,
            Self::UFraction(_) => ValueType::UFraction,
            Self::String(_) => ValueType::String,
            Self::Str(_) => ValueType::String,
            Self::TempString(_) => ValueType::String,
            Self::Mon(_) => ValueType::Mon,
            Self::Effect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::Type(_) => ValueType::Type,
            Self::List(_) => ValueType::List,
            Self::Object(_) => ValueType::Object,
        }
    }

    /// Converts the reference to a [`Value`], which is guaranteed to contain no references.
    pub fn to_owned(&self) -> Value {
        match self {
            Self::Undefined => Value::Undefined,
            Self::Boolean(val) => Value::Boolean(*val),
            Self::U16(val) => Value::U16(*val),
            Self::U32(val) => Value::U32(*val),
            Self::U64(val) => Value::U64(*val),
            Self::I64(val) => Value::I64(*val),
            Self::Fraction(val) => Value::Fraction(*val),
            Self::UFraction(val) => Value::UFraction(*val),
            Self::String(val) => Value::String(val.to_string()),
            Self::Str(val) => Value::String(val.to_string()),
            Self::TempString(val) => Value::String(val.clone()),
            Self::Mon(val) => Value::Mon(*val),
            Self::Effect(val) => Value::Effect(val.clone()),
            Self::ActiveMove(val) => Value::ActiveMove(*val),
            Self::MoveCategory(val) => Value::MoveCategory(*val),
            Self::MoveTarget(val) => Value::MoveTarget(*val),
            Self::Type(val) => Value::Type(*val),
            Self::List(val) => Value::List((*val).clone()),
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
            _ => false,
        }
    }

    /// Returns the length of the value, if supported.
    pub fn len(&self) -> Option<usize> {
        match self {
            Self::String(val) => Some(val.len()),
            Self::Str(val) => Some(val.len()),
            Self::List(val) => Some(val.len()),
            Self::Object(val) => Some(val.len()),
            _ => None,
        }
    }

    /// Returns the list element at the given index.
    pub fn list_index(&self, index: usize) -> Option<&Value> {
        match self {
            Self::List(list) => list.get(index),
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
            Value::U16(val) => Self::U16(*val),
            Value::U32(val) => Self::U32(*val),
            Value::U64(val) => Self::U64(*val),
            Value::I64(val) => Self::I64(*val),
            Value::Fraction(val) => Self::Fraction(*val),
            Value::UFraction(val) => Self::UFraction(*val),
            Value::String(val) => Self::String(val),
            Value::Mon(val) => Self::Mon(*val),
            Value::Effect(val) => Self::Effect(val.clone()),
            Value::ActiveMove(val) => Self::ActiveMove(*val),
            Value::MoveCategory(val) => Self::MoveCategory(*val),
            Value::MoveTarget(val) => Self::MoveTarget(*val),
            Value::Type(val) => Self::Type(*val),
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
    _stored: ElementRef<'eval, Value>,
    value: ValueRef<'eval>,
}

impl<'eval> ValueRefToStoredValue<'eval> {
    /// Creates a new reference to a stored value.
    pub fn new(stored: ElementRef<'eval, Value>, value: ValueRef<'eval>) -> Self {
        Self {
            _stored: stored,
            value,
        }
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
}

/// A [`Value`], but containing a mutable reference to the underlying value.
pub enum ValueRefMut<'eval> {
    Undefined(&'eval mut Value),
    Boolean(&'eval mut bool),
    U16(&'eval mut u16),
    U32(&'eval mut u32),
    U64(&'eval mut u64),
    I64(&'eval mut i64),
    OptionalISize(&'eval mut Option<isize>),
    Fraction(&'eval mut Fraction<i32>),
    UFraction(&'eval mut Fraction<u32>),
    OptionalString(&'eval mut Option<String>),
    String(&'eval mut String),
    Mon(&'eval mut MonHandle),
    Effect(&'eval mut EffectHandle),
    ActiveMove(&'eval mut MoveHandle),
    MoveCategory(&'eval mut MoveCategory),
    MoveTarget(&'eval mut MoveTarget),
    Type(&'eval mut Type),
    List(&'eval mut Vec<Value>),
    Object(&'eval mut FastHashMap<String, Value>),
}

impl<'eval> ValueRefMut<'eval> {
    /// The type of the value.
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Undefined(_) => ValueType::Undefined,
            Self::Boolean(_) => ValueType::Boolean,
            Self::U16(_) => ValueType::U16,
            Self::U32(_) => ValueType::U32,
            Self::U64(_) => ValueType::U64,
            Self::I64(_) => ValueType::I64,
            Self::OptionalISize(_) => ValueType::OptionalISize,
            Self::Fraction(_) => ValueType::Fraction,
            Self::UFraction(_) => ValueType::UFraction,
            Self::OptionalString(_) => ValueType::String,
            Self::String(_) => ValueType::String,
            Self::Mon(_) => ValueType::Mon,
            Self::Effect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::Type(_) => ValueType::Type,
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
            Value::U16(val) => Self::U16(val),
            Value::U32(val) => Self::U32(val),
            Value::U64(val) => Self::U64(val),
            Value::I64(val) => Self::I64(val),
            Value::Fraction(val) => Self::Fraction(val),
            Value::UFraction(val) => Self::UFraction(val),
            Value::String(val) => Self::String(val),
            Value::Mon(val) => Self::Mon(val),
            Value::Effect(val) => Self::Effect(val),
            Value::ActiveMove(val) => Self::ActiveMove(val),
            Value::MoveCategory(val) => Self::MoveCategory(val),
            Value::MoveTarget(val) => Self::MoveTarget(val),
            Value::Type(val) => Self::Type(val),
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
    U16(u16),
    U32(u32),
    U64(u64),
    I64(i64),
    Fraction(Fraction<i32>),
    UFraction(Fraction<u32>),
    String(&'eval String),
    Str(&'eval str),
    TempString(String),
    Mon(MonHandle),
    Effect(&'eval EffectHandle),
    ActiveMove(MoveHandle),
    MoveCategory(MoveCategory),
    MoveTarget(MoveTarget),
    Type(Type),
    List(&'eval Vec<MaybeReferenceValue<'eval>>),
    Object(&'eval FastHashMap<String, MaybeReferenceValue<'eval>>),
    StoredList(&'eval Vec<Value>),
    StoredObject(&'eval FastHashMap<String, Value>),
}

impl<'eval> MaybeReferenceValueForOperation<'eval> {
    /// The type of the value.
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Undefined => ValueType::Undefined,
            Self::Boolean(_) => ValueType::Boolean,
            Self::U16(_) => ValueType::U16,
            Self::U32(_) => ValueType::U32,
            Self::U64(_) => ValueType::U64,
            Self::I64(_) => ValueType::I64,
            Self::Fraction(_) => ValueType::Fraction,
            Self::UFraction(_) => ValueType::UFraction,
            Self::String(_) => ValueType::String,
            Self::Str(_) => ValueType::String,
            Self::TempString(_) => ValueType::String,
            Self::Mon(_) => ValueType::Mon,
            Self::Effect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::Type(_) => ValueType::Type,
            Self::List(_) => ValueType::List,
            Self::Object(_) => ValueType::Object,
            Self::StoredList(_) => ValueType::List,
            Self::StoredObject(_) => ValueType::Object,
        }
    }

    /// Converts the value to a [`Value`], which is guaranteed to contain no references.
    pub fn to_owned(&self) -> Value {
        match self {
            Self::Undefined => Value::Undefined,
            Self::Boolean(val) => Value::Boolean(*val),
            Self::U16(val) => Value::U16(*val),
            Self::U32(val) => Value::U32(*val),
            Self::U64(val) => Value::U64(*val),
            Self::I64(val) => Value::I64(*val),
            Self::Fraction(val) => Value::Fraction(*val),
            Self::UFraction(val) => Value::UFraction(*val),
            Self::String(val) => Value::String((*val).clone()),
            Self::Str(val) => Value::String(val.to_string()),
            Self::TempString(val) => Value::String(val.clone()),
            Self::Mon(val) => Value::Mon(*val),
            Self::Effect(val) => Value::Effect((*val).clone()),
            Self::ActiveMove(val) => Value::ActiveMove(*val),
            Self::MoveCategory(val) => Value::MoveCategory(*val),
            Self::MoveTarget(val) => Value::MoveTarget(*val),
            Self::Type(val) => Value::Type(*val),
            Self::List(val) => Value::List(val.iter().map(|val| val.to_owned()).collect()),
            Self::Object(val) => Value::Object(
                val.iter()
                    .map(|(key, val)| (key.clone(), val.to_owned()))
                    .collect(),
            ),
            Self::StoredList(val) => Value::List((*val).clone()),
            Self::StoredObject(val) => Value::Object((*val).clone()),
        }
    }

    fn internal_type_index(&self) -> usize {
        match self {
            Self::Undefined => 0,
            Self::Boolean(_) => 1,
            Self::U16(_) => 8,
            Self::U32(_) => 9,
            Self::U64(_) => 10,
            Self::I64(_) => 11,
            Self::Fraction(_) => 32,
            Self::UFraction(_) => 33,
            Self::String(_) => 64,
            Self::Str(_) => 65,
            Self::TempString(_) => 66,
            Self::Mon(_) => 100,
            Self::Effect(_) => 101,
            Self::ActiveMove(_) => 102,
            Self::MoveCategory(_) => 103,
            Self::MoveTarget(_) => 104,
            Self::Type(_) => 105,
            Self::List(_) => 200,
            Self::Object(_) => 201,
            Self::StoredList(_) => 300,
            Self::StoredObject(_) => 301,
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

    fn invalid_operation(operation: &str, value_type: ValueType) -> Error {
        battler_error!("cannot {operation} {value_type} value")
    }

    fn invalid_binary_operation(operation: &str, lhs: ValueType, rhs: ValueType) -> Error {
        battler_error!("cannot {operation} {lhs} and {rhs}")
    }

    pub fn negate(self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match self {
            Self::Undefined => MaybeReferenceValue::Boolean(true),
            Self::Boolean(val) => MaybeReferenceValue::Boolean(!val),
            _ => return Err(Self::invalid_operation("negate", self.value_type())),
        };
        Ok(result)
    }

    pub fn multiply(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match Self::sort_for_commutative_operation(self, rhs) {
            (Self::U16(lhs), Self::U16(rhs)) => MaybeReferenceValue::U16(lhs.wrapping_mul(rhs)),
            (Self::U16(lhs), Self::U32(rhs)) => {
                MaybeReferenceValue::U32((lhs as u32).wrapping_mul(rhs))
            }
            (Self::U16(lhs), Self::U64(rhs)) => {
                MaybeReferenceValue::U64((lhs as u64).wrapping_mul(rhs))
            }
            (Self::U16(lhs), Self::I64(rhs)) => {
                MaybeReferenceValue::I64((lhs as i64).wrapping_mul(rhs))
            }
            (Self::U16(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(Fraction::from(lhs as i32).wrapping_mul(&rhs))
            }
            (Self::U16(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(Fraction::from(lhs as u32).wrapping_mul(&rhs))
            }
            (Self::U32(lhs), Self::U32(rhs)) => MaybeReferenceValue::U32(lhs.wrapping_mul(rhs)),
            (Self::U32(lhs), Self::U64(rhs)) => {
                MaybeReferenceValue::U64((lhs as u64).wrapping_mul(rhs))
            }
            (Self::U32(lhs), Self::I64(rhs)) => {
                MaybeReferenceValue::I64((lhs as i64).wrapping_mul(rhs))
            }
            (Self::U32(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                Fraction::from(
                    TryInto::<i32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_mul(&rhs),
            ),
            (Self::U32(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(Fraction::from(lhs).wrapping_mul(&rhs))
            }
            (Self::U64(lhs), Self::U64(rhs)) => MaybeReferenceValue::U64(lhs.wrapping_mul(rhs)),
            (Self::U64(lhs), Self::I64(rhs)) => MaybeReferenceValue::I64(
                TryInto::<i64>::try_into(lhs)
                    .wrap_error_with_message("integer overflow")?
                    .wrapping_mul(rhs),
            ),
            (Self::U64(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                Fraction::from(
                    TryInto::<i32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_mul(&rhs),
            ),
            (Self::U64(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::UFraction(
                Fraction::from(
                    TryInto::<u32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_mul(&rhs),
            ),
            (Self::I64(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                Fraction::from(
                    TryInto::<i32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_mul(&rhs),
            ),
            (Self::I64(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::UFraction(
                Fraction::from(
                    TryInto::<u32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_mul(&rhs),
            ),
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

    pub fn divide(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match (self, rhs) {
            (Self::U16(lhs), Self::U16(rhs)) => MaybeReferenceValue::U16(lhs / rhs),
            (Self::U16(lhs), Self::U32(rhs)) => MaybeReferenceValue::U32((lhs as u32) / rhs),
            (Self::U16(lhs), Self::U64(rhs)) => MaybeReferenceValue::U64((lhs as u64) / rhs),
            (Self::U16(lhs), Self::I64(rhs)) => MaybeReferenceValue::I64((lhs as i64) / rhs),
            (Self::U16(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(Fraction::from(lhs as i32) / rhs)
            }
            (Self::U16(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(Fraction::from(lhs as u32) / rhs)
            }
            (Self::U32(lhs), Self::U16(rhs)) => MaybeReferenceValue::U32(lhs / (rhs as u32)),
            (Self::U32(lhs), Self::U32(rhs)) => MaybeReferenceValue::U32(lhs / rhs),
            (Self::U32(lhs), Self::U64(rhs)) => MaybeReferenceValue::U64((lhs as u64) / rhs),
            (Self::U32(lhs), Self::I64(rhs)) => MaybeReferenceValue::I64((lhs as i64) / rhs),
            (Self::U32(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                Fraction::from(
                    TryInto::<i32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                ) / rhs,
            ),
            (Self::U32(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(Fraction::from(lhs) / rhs)
            }
            (Self::U64(lhs), Self::U16(rhs)) => MaybeReferenceValue::U64(lhs / (rhs as u64)),
            (Self::U64(lhs), Self::U32(rhs)) => MaybeReferenceValue::U64(lhs / (rhs as u64)),
            (Self::U64(lhs), Self::U64(rhs)) => MaybeReferenceValue::U64(lhs / rhs),
            (Self::U64(lhs), Self::I64(rhs)) => MaybeReferenceValue::I64(
                TryInto::<i64>::try_into(lhs).wrap_error_with_message("integer overflow")? / rhs,
            ),
            (Self::U64(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                Fraction::from(
                    TryInto::<i32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                ) / rhs,
            ),
            (Self::U64(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::UFraction(
                Fraction::from(
                    TryInto::<u32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                ) / rhs,
            ),
            (Self::I64(lhs), Self::U16(rhs)) => MaybeReferenceValue::I64(lhs / (rhs as i64)),
            (Self::I64(lhs), Self::U32(rhs)) => MaybeReferenceValue::I64(lhs / (rhs as i64)),
            (Self::I64(lhs), Self::U64(rhs)) => MaybeReferenceValue::I64(
                lhs / TryInto::<i64>::try_into(rhs).wrap_error_with_message("integer overflow")?,
            ),
            (Self::I64(lhs), Self::I64(rhs)) => MaybeReferenceValue::I64(
                TryInto::<i64>::try_into(lhs).wrap_error_with_message("integer overflow")? / rhs,
            ),
            (Self::I64(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                Fraction::from(
                    TryInto::<i32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                ) / rhs,
            ),
            (Self::I64(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::UFraction(
                Fraction::from(
                    TryInto::<u32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                ) / rhs,
            ),
            (Self::Fraction(lhs), Self::U16(rhs)) => {
                MaybeReferenceValue::Fraction(lhs / Fraction::from(rhs as i32))
            }
            (Self::Fraction(lhs), Self::U32(rhs)) => MaybeReferenceValue::Fraction(
                lhs / Fraction::from(
                    TryInto::<i32>::try_into(rhs).wrap_error_with_message("integer overflow")?,
                ),
            ),
            (Self::Fraction(lhs), Self::U64(rhs)) => MaybeReferenceValue::Fraction(
                lhs / Fraction::from(
                    TryInto::<i32>::try_into(rhs).wrap_error_with_message("integer overflow")?,
                ),
            ),
            (Self::Fraction(lhs), Self::I64(rhs)) => MaybeReferenceValue::Fraction(
                lhs / Fraction::from(
                    TryInto::<i32>::try_into(rhs).wrap_error_with_message("integer overflow")?,
                ),
            ),
            (Self::Fraction(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(lhs / rhs),
            (Self::Fraction(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs / rhs
                    .try_convert()
                    .wrap_error_with_message("integer overflow")?,
            ),

            (Self::UFraction(lhs), Self::U16(rhs)) => {
                MaybeReferenceValue::UFraction(lhs / Fraction::from(rhs as u32))
            }
            (Self::UFraction(lhs), Self::U32(rhs)) => {
                MaybeReferenceValue::UFraction(lhs / Fraction::from(rhs))
            }
            (Self::UFraction(lhs), Self::U64(rhs)) => MaybeReferenceValue::UFraction(
                lhs / Fraction::from(
                    TryInto::<u32>::try_into(rhs).wrap_error_with_message("integer overflow")?,
                ),
            ),
            (Self::UFraction(lhs), Self::I64(rhs)) => MaybeReferenceValue::UFraction(
                lhs / Fraction::from(
                    TryInto::<u32>::try_into(rhs).wrap_error_with_message("integer overflow")?,
                ),
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

    pub fn modulo(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match (self, rhs) {
            (Self::U16(lhs), Self::U16(rhs)) => MaybeReferenceValue::U16(lhs % rhs),
            (Self::U16(lhs), Self::U32(rhs)) => MaybeReferenceValue::U32((lhs as u32) % rhs),
            (Self::U16(lhs), Self::U64(rhs)) => MaybeReferenceValue::U64((lhs as u64) % rhs),
            (Self::U16(lhs), Self::I64(rhs)) => MaybeReferenceValue::I64((lhs as i64) % rhs),
            (Self::U32(lhs), Self::U16(rhs)) => MaybeReferenceValue::U32(lhs % (rhs as u32)),
            (Self::U32(lhs), Self::U32(rhs)) => MaybeReferenceValue::U32(lhs % rhs),
            (Self::U32(lhs), Self::U64(rhs)) => MaybeReferenceValue::U64((lhs as u64) % rhs),
            (Self::U32(lhs), Self::I64(rhs)) => MaybeReferenceValue::I64((lhs as i64) % rhs),
            (Self::U64(lhs), Self::U16(rhs)) => MaybeReferenceValue::U64(lhs % (rhs as u64)),
            (Self::U64(lhs), Self::U32(rhs)) => MaybeReferenceValue::U64(lhs % (rhs as u64)),
            (Self::U64(lhs), Self::U64(rhs)) => MaybeReferenceValue::U64(lhs % rhs),
            (Self::U64(lhs), Self::I64(rhs)) => MaybeReferenceValue::I64(
                TryInto::<i64>::try_into(lhs).wrap_error_with_message("integer overflow")? % rhs,
            ),
            (Self::I64(lhs), Self::U16(rhs)) => MaybeReferenceValue::I64(lhs % (rhs as i64)),
            (Self::I64(lhs), Self::U32(rhs)) => MaybeReferenceValue::I64(lhs % (rhs as i64)),
            (Self::I64(lhs), Self::U64(rhs)) => MaybeReferenceValue::I64(
                lhs % TryInto::<i64>::try_into(rhs).wrap_error_with_message("integer overflow")?,
            ),
            (Self::I64(lhs), Self::I64(rhs)) => MaybeReferenceValue::I64(lhs % rhs),
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

    pub fn add(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match Self::sort_for_commutative_operation(self, rhs) {
            (Self::U16(lhs), Self::U16(rhs)) => MaybeReferenceValue::U16(lhs.wrapping_add(rhs)),
            (Self::U16(lhs), Self::U32(rhs)) => {
                MaybeReferenceValue::U32((lhs as u32).wrapping_add(rhs))
            }
            (Self::U16(lhs), Self::U64(rhs)) => {
                MaybeReferenceValue::U64((lhs as u64).wrapping_add(rhs))
            }
            (Self::U16(lhs), Self::I64(rhs)) => {
                MaybeReferenceValue::I64((lhs as i64).wrapping_add(rhs))
            }
            (Self::U16(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(Fraction::from(lhs as i32).wrapping_add(&rhs))
            }
            (Self::U16(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(Fraction::from(lhs as u32).wrapping_add(&rhs))
            }
            (Self::U32(lhs), Self::U32(rhs)) => MaybeReferenceValue::U32(lhs.wrapping_add(rhs)),
            (Self::U32(lhs), Self::U64(rhs)) => {
                MaybeReferenceValue::U64((lhs as u64).wrapping_add(rhs))
            }
            (Self::U32(lhs), Self::I64(rhs)) => {
                MaybeReferenceValue::I64((lhs as i64).wrapping_add(rhs))
            }
            (Self::U32(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                Fraction::from(
                    TryInto::<i32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_add(&rhs),
            ),
            (Self::U32(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(Fraction::from(lhs).wrapping_add(&rhs))
            }
            (Self::U64(lhs), Self::U64(rhs)) => MaybeReferenceValue::U64(lhs.wrapping_add(rhs)),
            (Self::U64(lhs), Self::I64(rhs)) => {
                MaybeReferenceValue::I64((lhs as i64).wrapping_add(rhs))
            }
            (Self::U64(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                Fraction::from(
                    TryInto::<i32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_add(&rhs),
            ),
            (Self::U64(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::UFraction(
                Fraction::from(
                    TryInto::<u32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_add(&rhs),
            ),
            (Self::I64(lhs), Self::I64(rhs)) => MaybeReferenceValue::I64(lhs.wrapping_add(rhs)),
            (Self::I64(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                Fraction::from(
                    TryInto::<i32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_add(&rhs),
            ),
            (Self::I64(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::UFraction(
                Fraction::from(
                    TryInto::<u32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_add(&rhs),
            ),
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

    pub fn subtract(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match (self, rhs) {
            (Self::U16(lhs), Self::U16(rhs)) => MaybeReferenceValue::U16(lhs.wrapping_sub(rhs)),
            (Self::U16(lhs), Self::U32(rhs)) => {
                MaybeReferenceValue::U32((lhs as u32).wrapping_sub(rhs))
            }
            (Self::U16(lhs), Self::U64(rhs)) => {
                MaybeReferenceValue::U64((lhs as u64).wrapping_sub(rhs))
            }
            (Self::U16(lhs), Self::I64(rhs)) => {
                MaybeReferenceValue::I64((lhs as i64).wrapping_sub(rhs))
            }
            (Self::U16(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(Fraction::from(lhs as i32).wrapping_sub(&rhs))
            }
            (Self::U16(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(Fraction::from(lhs as u32).wrapping_sub(&rhs))
            }
            (Self::U32(lhs), Self::U16(rhs)) => {
                MaybeReferenceValue::U32(lhs.wrapping_sub(rhs as u32))
            }
            (Self::U32(lhs), Self::U32(rhs)) => MaybeReferenceValue::U32(lhs.wrapping_sub(rhs)),
            (Self::U32(lhs), Self::U64(rhs)) => {
                MaybeReferenceValue::U64((lhs as u64).wrapping_sub(rhs))
            }
            (Self::U32(lhs), Self::I64(rhs)) => {
                MaybeReferenceValue::I64((lhs as i64).wrapping_sub(rhs))
            }
            (Self::U32(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                Fraction::from(
                    TryInto::<i32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_sub(&rhs),
            ),
            (Self::U32(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(Fraction::from(lhs).wrapping_sub(&rhs))
            }
            (Self::U64(lhs), Self::U16(rhs)) => {
                MaybeReferenceValue::U64(lhs.wrapping_sub(rhs as u64))
            }
            (Self::U64(lhs), Self::U32(rhs)) => {
                MaybeReferenceValue::U64(lhs.wrapping_sub(rhs as u64))
            }
            (Self::U64(lhs), Self::U64(rhs)) => MaybeReferenceValue::U64(lhs.wrapping_sub(rhs)),
            (Self::U64(lhs), Self::I64(rhs)) => MaybeReferenceValue::I64(
                TryInto::<i64>::try_into(lhs)
                    .wrap_error_with_message("integer overflow")?
                    .wrapping_sub(rhs),
            ),
            (Self::U64(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                Fraction::from(
                    TryInto::<i32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_sub(&rhs),
            ),
            (Self::U64(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::UFraction(
                Fraction::from(
                    TryInto::<u32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_sub(&rhs),
            ),
            (Self::I64(lhs), Self::U16(rhs)) => {
                MaybeReferenceValue::I64(lhs.wrapping_sub(rhs as i64))
            }
            (Self::I64(lhs), Self::U32(rhs)) => {
                MaybeReferenceValue::I64(lhs.wrapping_sub(rhs as i64))
            }
            (Self::I64(lhs), Self::U64(rhs)) => {
                MaybeReferenceValue::I64(lhs.wrapping_sub(rhs as i64))
            }
            (Self::I64(lhs), Self::I64(rhs)) => MaybeReferenceValue::I64(lhs.wrapping_sub(
                TryInto::<i64>::try_into(rhs).wrap_error_with_message("integer overflow")?,
            )),
            (Self::I64(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                Fraction::from(
                    TryInto::<i32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_sub(&rhs),
            ),
            (Self::I64(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::UFraction(
                Fraction::from(
                    TryInto::<u32>::try_into(lhs).wrap_error_with_message("integer overflow")?,
                )
                .wrapping_sub(&rhs),
            ),
            (Self::Fraction(lhs), Self::U16(rhs)) => {
                MaybeReferenceValue::Fraction(lhs.wrapping_sub(&Fraction::from(rhs as i32)))
            }
            (Self::Fraction(lhs), Self::U32(rhs)) => {
                MaybeReferenceValue::Fraction(lhs.wrapping_sub(&Fraction::from(
                    TryInto::<i32>::try_into(rhs).wrap_error_with_message("integer overflow")?,
                )))
            }
            (Self::Fraction(lhs), Self::U64(rhs)) => {
                MaybeReferenceValue::Fraction(lhs.wrapping_sub(&Fraction::from(
                    TryInto::<i32>::try_into(rhs).wrap_error_with_message("integer overflow")?,
                )))
            }
            (Self::Fraction(lhs), Self::I64(rhs)) => {
                MaybeReferenceValue::Fraction(lhs.wrapping_sub(&Fraction::from(
                    TryInto::<i32>::try_into(rhs).wrap_error_with_message("integer overflow")?,
                )))
            }
            (Self::Fraction(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(lhs.wrapping_sub(&rhs))
            }
            (Self::Fraction(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs.wrapping_sub(
                    &rhs.try_convert()
                        .wrap_error_with_message("integer overflow")?,
                ),
            ),
            (Self::UFraction(lhs), Self::U16(rhs)) => {
                MaybeReferenceValue::UFraction(lhs.wrapping_sub(&Fraction::from(rhs as u32)))
            }
            (Self::UFraction(lhs), Self::U32(rhs)) => {
                MaybeReferenceValue::UFraction(lhs.wrapping_sub(&Fraction::from(rhs)))
            }
            (Self::UFraction(lhs), Self::U64(rhs)) => {
                MaybeReferenceValue::UFraction(lhs.wrapping_sub(&Fraction::from(
                    TryInto::<u32>::try_into(rhs).wrap_error_with_message("integer overflow")?,
                )))
            }
            (Self::UFraction(lhs), Self::I64(rhs)) => {
                MaybeReferenceValue::UFraction(lhs.wrapping_sub(&Fraction::from(
                    TryInto::<u32>::try_into(rhs).wrap_error_with_message("integer overflow")?,
                )))
            }
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
            (Self::U16(lhs), Self::U16(rhs)) => lhs.partial_cmp(rhs),
            (Self::U16(lhs), Self::U32(rhs)) => (&(*lhs as u32)).partial_cmp(rhs),
            (Self::U16(lhs), Self::U64(rhs)) => (&(*lhs as u64)).partial_cmp(rhs),
            (Self::U16(lhs), Self::I64(rhs)) => (&(*lhs as i64)).partial_cmp(rhs),
            (Self::U16(lhs), Self::Fraction(rhs)) => Fraction::from(*lhs as i32).partial_cmp(rhs),
            (Self::U16(lhs), Self::UFraction(rhs)) => Fraction::from(*lhs as u32).partial_cmp(rhs),
            (Self::U32(lhs), Self::U16(rhs)) => lhs.partial_cmp(&(*rhs as u32)),
            (Self::U32(lhs), Self::U32(rhs)) => lhs.partial_cmp(rhs),
            (Self::U32(lhs), Self::U64(rhs)) => (&(*lhs as u64)).partial_cmp(rhs),
            (Self::U32(lhs), Self::I64(rhs)) => (&(*lhs as i64)).partial_cmp(rhs),
            (Self::U32(lhs), Self::Fraction(rhs)) => {
                if *lhs > i32::MAX as u32 {
                    Some(Ordering::Greater)
                } else {
                    Fraction::from(*lhs as i32).partial_cmp(rhs)
                }
            }
            (Self::U32(lhs), Self::UFraction(rhs)) => Fraction::from(*lhs).partial_cmp(rhs),
            (Self::U64(lhs), Self::U16(rhs)) => lhs.partial_cmp(&(*rhs as u64)),
            (Self::U64(lhs), Self::U32(rhs)) => lhs.partial_cmp(&(*rhs as u64)),
            (Self::U64(lhs), Self::U64(rhs)) => lhs.partial_cmp(rhs),
            (Self::U64(lhs), Self::I64(rhs)) => lhs.partial_cmp(&(*rhs as u64)),
            (Self::U64(lhs), Self::Fraction(rhs)) => {
                if *lhs > i32::MAX as u64 {
                    Some(Ordering::Greater)
                } else {
                    Fraction::from(*lhs as i32).partial_cmp(rhs)
                }
            }
            (Self::U64(lhs), Self::UFraction(rhs)) => {
                if *lhs > u32::MAX as u64 {
                    Some(Ordering::Greater)
                } else {
                    Fraction::from(*lhs as u32).partial_cmp(rhs)
                }
            }
            (Self::I64(lhs), Self::U16(rhs)) => lhs.partial_cmp(&(*rhs as i64)),
            (Self::I64(lhs), Self::U32(rhs)) => lhs.partial_cmp(&(*rhs as i64)),
            (Self::I64(lhs), Self::U64(rhs)) => (&(*lhs as u64)).partial_cmp(rhs),
            (Self::I64(lhs), Self::I64(rhs)) => lhs.partial_cmp(&(*rhs as i64)),
            (Self::I64(lhs), Self::Fraction(rhs)) => {
                if *lhs > i32::MAX as i64 {
                    Some(Ordering::Greater)
                } else {
                    Fraction::from(*lhs as i32).partial_cmp(rhs)
                }
            }
            (Self::I64(lhs), Self::UFraction(rhs)) => {
                if *lhs > u32::MAX as i64 {
                    Some(Ordering::Greater)
                } else {
                    Fraction::from(*lhs as u32).partial_cmp(rhs)
                }
            }
            (Self::Fraction(lhs), Self::U16(rhs)) => lhs.partial_cmp(&(*rhs as i32)),
            (Self::Fraction(lhs), Self::U32(rhs)) => {
                if *rhs > i32::MAX as u32 {
                    Some(Ordering::Less)
                } else {
                    lhs.partial_cmp(&(*rhs as i32))
                }
            }
            (Self::Fraction(lhs), Self::U64(rhs)) => {
                if *rhs > i32::MAX as u64 {
                    Some(Ordering::Less)
                } else {
                    lhs.partial_cmp(&(*rhs as i32))
                }
            }
            (Self::Fraction(lhs), Self::I64(rhs)) => {
                if *rhs > i32::MAX as i64 {
                    Some(Ordering::Less)
                } else {
                    lhs.partial_cmp(&(*rhs as i32))
                }
            }
            (Self::Fraction(lhs), Self::Fraction(rhs)) => lhs.partial_cmp(rhs),
            (Self::Fraction(lhs), Self::UFraction(rhs)) => {
                if lhs < &0 {
                    Some(Ordering::Less)
                } else {
                    Fraction::new(lhs.numerator().abs() as u32, lhs.denominator().abs() as u32)
                        .partial_cmp(rhs)
                }
            }
            (Self::UFraction(lhs), Self::U16(rhs)) => lhs.partial_cmp(&(*rhs as u32)),
            (Self::UFraction(lhs), Self::U32(rhs)) => lhs.partial_cmp(rhs),
            (Self::UFraction(lhs), Self::U64(rhs)) => {
                if *rhs > u32::MAX as u64 {
                    Some(Ordering::Less)
                } else {
                    lhs.partial_cmp(&(*rhs as u32))
                }
            }
            (Self::UFraction(lhs), Self::I64(rhs)) => {
                if *rhs > u32::MAX as i64 {
                    Some(Ordering::Less)
                } else {
                    lhs.partial_cmp(&(*rhs as u32))
                }
            }
            (Self::UFraction(lhs), Self::Fraction(rhs)) => {
                if rhs < &0 {
                    Some(Ordering::Greater)
                } else {
                    lhs.partial_cmp(&Fraction::new(
                        rhs.numerator().abs() as u32,
                        rhs.denominator().abs() as u32,
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

    pub fn less_than(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        Ok(MaybeReferenceValue::Boolean(
            self.compare_ref(&rhs)?.is_lt(),
        ))
    }

    pub fn greater_than(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        Ok(MaybeReferenceValue::Boolean(
            self.compare_ref(&rhs)?.is_gt(),
        ))
    }

    pub fn less_than_or_equal(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        Ok(MaybeReferenceValue::Boolean(
            self.compare_ref(&rhs)?.is_le(),
        ))
    }

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
            (Self::Boolean(lhs), Self::Boolean(rhs)) => lhs.eq(rhs),
            (lhs @ Self::U16(_), rhs @ Self::U16(_))
            | (lhs @ Self::U16(_), rhs @ Self::U32(_))
            | (lhs @ Self::U16(_), rhs @ Self::U64(_))
            | (lhs @ Self::U16(_), rhs @ Self::I64(_))
            | (lhs @ Self::U16(_), rhs @ Self::Fraction(_))
            | (lhs @ Self::U16(_), rhs @ Self::UFraction(_))
            | (lhs @ Self::U32(_), rhs @ Self::U32(_))
            | (lhs @ Self::U32(_), rhs @ Self::U64(_))
            | (lhs @ Self::U32(_), rhs @ Self::I64(_))
            | (lhs @ Self::U32(_), rhs @ Self::Fraction(_))
            | (lhs @ Self::U32(_), rhs @ Self::UFraction(_))
            | (lhs @ Self::U64(_), rhs @ Self::U64(_))
            | (lhs @ Self::U64(_), rhs @ Self::I64(_))
            | (lhs @ Self::U64(_), rhs @ Self::Fraction(_))
            | (lhs @ Self::U64(_), rhs @ Self::UFraction(_))
            | (lhs @ Self::I64(_), rhs @ Self::I64(_))
            | (lhs @ Self::I64(_), rhs @ Self::Fraction(_))
            | (lhs @ Self::I64(_), rhs @ Self::UFraction(_))
            | (lhs @ Self::Fraction(_), rhs @ Self::Fraction(_))
            | (lhs @ Self::Fraction(_), rhs @ Self::UFraction(_))
            | (lhs @ Self::UFraction(_), rhs @ Self::UFraction(_)) => lhs.compare_ref(rhs)?.is_eq(),
            (Self::String(lhs), Self::String(rhs)) => lhs.eq(rhs),
            (Self::String(lhs), Self::Str(rhs)) => lhs.eq(rhs),
            (Self::String(lhs), Self::TempString(rhs)) => lhs.eq(&rhs),
            (Self::String(lhs), Self::MoveCategory(rhs)) => {
                MoveCategory::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::MoveTarget(rhs)) => {
                MoveTarget::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::Type(rhs)) => {
                Type::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::Str(rhs)) => lhs.eq(rhs),
            (Self::Str(lhs), Self::TempString(rhs)) => lhs.eq(&rhs),
            (Self::Str(lhs), Self::MoveCategory(rhs)) => {
                MoveCategory::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::MoveTarget(rhs)) => {
                MoveTarget::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::Type(rhs)) => Type::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs)),
            (Self::TempString(lhs), Self::TempString(rhs)) => lhs.eq(rhs),
            (Self::TempString(lhs), Self::MoveCategory(rhs)) => {
                MoveCategory::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::MoveTarget(rhs)) => {
                MoveTarget::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::Type(rhs)) => {
                Type::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Mon(lhs), Self::Mon(rhs)) => lhs.eq(rhs),
            (Self::Effect(lhs), Self::Effect(rhs)) => lhs.eq(rhs),
            (Self::ActiveMove(lhs), Self::ActiveMove(rhs)) => lhs.eq(rhs),
            (Self::MoveCategory(lhs), Self::MoveCategory(rhs)) => lhs.eq(rhs),
            (Self::Type(lhs), Self::Type(rhs)) => lhs.eq(rhs),
            (Self::List(lhs), Self::List(rhs)) => Self::equal_lists(lhs, rhs)?,
            (Self::List(lhs), Self::StoredList(rhs)) => Self::equal_lists(lhs, rhs)?,
            (Self::StoredList(lhs), Self::StoredList(rhs)) => Self::equal_lists(lhs, rhs)?,
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

    pub fn equal(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        Ok(MaybeReferenceValue::Boolean(self.equal_ref(&rhs)?))
    }

    pub fn not_equal(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        Ok(MaybeReferenceValue::Boolean(!self.equal_ref(&rhs)?))
    }

    fn list_has_value<T>(list: &'eval Vec<T>, rhs: Self) -> bool
    where
        &'eval T: Into<Self> + 'eval,
    {
        list.iter()
            .map(|val| Into::<Self>::into(val))
            .any(|lhs| lhs.equal_ref(&rhs).is_ok_and(|eq| eq))
    }

    pub fn has(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match (self, rhs) {
            (Self::List(lhs), rhs @ _) => Self::list_has_value(lhs, rhs),
            (Self::StoredList(lhs), rhs @ _) => Self::list_has_value(lhs, rhs),
            _ => {
                return Err(battler_error!(
                    "left-hand side of has operator must be a list"
                ));
            }
        };
        Ok(MaybeReferenceValue::Boolean(result))
    }

    fn list_has_any_value<T, U>(lhs: &'eval Vec<T>, rhs: &'eval Vec<U>) -> bool
    where
        &'eval T: Into<Self> + 'eval,
        &'eval U: Into<Self> + 'eval,
    {
        lhs.iter()
            .map(|a| Into::<Self>::into(a))
            .any(|lhs| Self::list_has_value(rhs, lhs))
    }

    pub fn has_any(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>, Error> {
        let result = match (self, rhs) {
            (Self::List(lhs), Self::List(rhs)) => Self::list_has_any_value(lhs, rhs),
            (Self::List(lhs), Self::StoredList(rhs)) => Self::list_has_any_value(lhs, rhs),
            (Self::StoredList(lhs), Self::List(rhs)) => Self::list_has_any_value(lhs, rhs),
            (Self::StoredList(lhs), Self::StoredList(rhs)) => Self::list_has_any_value(lhs, rhs),
            _ => {
                return Err(battler_error!(
                    "both operands to hasany operator must be a list"
                ));
            }
        };
        Ok(MaybeReferenceValue::Boolean(result))
    }

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
            Self::U16(val) => val.to_string(),
            Self::U32(val) => val.to_string(),
            Self::U64(val) => val.to_string(),
            Self::I64(val) => val.to_string(),
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
            Value::U16(val) => Self::U16(*val),
            Value::U32(val) => Self::U32(*val),
            Value::U64(val) => Self::U64(*val),
            Value::I64(val) => Self::I64(*val),
            Value::Fraction(val) => Self::Fraction(*val),
            Value::UFraction(val) => Self::UFraction(*val),
            Value::String(val) => Self::String(val),
            Value::Mon(val) => Self::Mon(*val),
            Value::Effect(val) => Self::Effect(val),
            Value::ActiveMove(val) => Self::ActiveMove(*val),
            Value::MoveCategory(val) => Self::MoveCategory(*val),
            Value::MoveTarget(val) => Self::MoveTarget(*val),
            Value::Type(val) => Self::Type(*val),
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
            MaybeReferenceValue::U16(val) => Self::U16(*val),
            MaybeReferenceValue::U32(val) => Self::U32(*val),
            MaybeReferenceValue::U64(val) => Self::U64(*val),
            MaybeReferenceValue::I64(val) => Self::I64(*val),
            MaybeReferenceValue::Fraction(val) => Self::Fraction(*val),
            MaybeReferenceValue::UFraction(val) => Self::UFraction(*val),
            MaybeReferenceValue::String(val) => Self::String(val),
            MaybeReferenceValue::Mon(val) => Self::Mon(*val),
            MaybeReferenceValue::Effect(val) => Self::Effect(val),
            MaybeReferenceValue::ActiveMove(val) => Self::ActiveMove(*val),
            MaybeReferenceValue::MoveCategory(val) => Self::MoveCategory(*val),
            MaybeReferenceValue::MoveTarget(val) => Self::MoveTarget(*val),
            MaybeReferenceValue::Type(val) => Self::Type(*val),
            MaybeReferenceValue::List(val) => Self::List(val),
            MaybeReferenceValue::Object(val) => Self::Object(val),
            MaybeReferenceValue::Reference(val) => Self::from(val),
        }
    }
}

impl<'eval> From<&'eval ValueRefToStoredValue<'eval>> for MaybeReferenceValueForOperation<'eval> {
    fn from(value: &'eval ValueRefToStoredValue<'eval>) -> Self {
        match &value.value {
            ValueRef::Undefined => Self::Undefined,
            ValueRef::Boolean(val) => Self::Boolean(*val),
            ValueRef::U16(val) => Self::U16(*val),
            ValueRef::U32(val) => Self::U32(*val),
            ValueRef::U64(val) => Self::U64(*val),
            ValueRef::I64(val) => Self::I64(*val),
            ValueRef::Fraction(val) => Self::Fraction(*val),
            ValueRef::UFraction(val) => Self::UFraction(*val),
            ValueRef::String(val) => Self::String(val),
            ValueRef::Str(val) => Self::Str(val),
            ValueRef::TempString(val) => Self::TempString(val.clone()),
            ValueRef::Mon(val) => Self::Mon(*val),
            ValueRef::Effect(val) => Self::Effect(val),
            ValueRef::ActiveMove(val) => Self::ActiveMove(*val),
            ValueRef::MoveCategory(val) => Self::MoveCategory(*val),
            ValueRef::MoveTarget(val) => Self::MoveTarget(*val),
            ValueRef::Type(val) => Self::Type(*val),
            ValueRef::List(val) => Self::StoredList(val),
            ValueRef::Object(val) => Self::StoredObject(val),
        }
    }
}
