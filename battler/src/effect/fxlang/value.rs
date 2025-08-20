use std::{
    cmp::Ordering,
    fmt::{
        self,
        Display,
    },
    str::FromStr,
};

use ahash::HashMap;
use anyhow::{
    Error,
    Result,
};
use battler_data::{
    Accuracy,
    Boost,
    BoostTable,
    FlingData,
    Fraction,
    Gender,
    HitEffect,
    Id,
    Identifiable,
    JudgmentData,
    MoveCategory,
    MoveTarget,
    MultihitType,
    NaturalGiftData,
    Nature,
    SecondaryEffectData,
    SpecialItemData,
    Stat,
    StatTable,
    Type,
};
use num::traits::{
    WrappingAdd,
    WrappingMul,
    WrappingSub,
};
use zone_alloc::ElementRef;

use crate::{
    battle::{
        FieldEnvironment,
        MonHandle,
        MoveEventResult,
        MoveHandle,
        MoveOutcomeOnTarget,
        MoveSlot,
    },
    effect::{
        EffectHandle,
        fxlang::{
            DynamicEffectStateConnector,
            EvaluationContext,
        },
    },
    error::{
        WrapOptionError,
        general_error,
        integer_overflow_error,
    },
    moves::MoveHitEffectType,
};

/// The type of an fxlang value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    // Primitive types.
    Undefined,
    Boolean,
    Fraction,
    UFraction,
    String,

    // Battle reference types.
    Battle,
    Field,
    Format,
    Side,
    Player,
    Mon,

    // Effects.
    Effect,
    ActiveMove,
    HitEffect,
    SecondaryHitEffect,

    // Battle value types.
    Accuracy,
    Boost,
    BoostTable,
    FieldEnvironment,
    FlingData,
    Gender,
    JudgmentData,
    MoveCategory,
    MoveSlot,
    MoveTarget,
    NaturalGiftData,
    Nature,
    SpecialItemData,
    Stat,
    StatTable,
    Type,

    // Effect state.
    EffectState,

    // Object types.
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
///
/// Owned and storable.
#[derive(Debug, Clone)]
pub enum Value {
    Undefined,
    Boolean(bool),
    Fraction(Fraction<i64>),
    UFraction(Fraction<u64>),
    String(String),

    Battle,
    Field,
    Format,
    Side(usize),
    Player(usize),
    Mon(MonHandle),

    Effect(EffectHandle),
    ActiveMove(MoveHandle),
    HitEffect(HitEffect),
    SecondaryHitEffect(SecondaryEffectData),

    Accuracy(Accuracy),
    Boost(Boost),
    BoostTable(BoostTable),
    FieldEnvironment(FieldEnvironment),
    FlingData(FlingData),
    Gender(Gender),
    JudgmentData(JudgmentData),
    MoveCategory(MoveCategory),
    MoveSlot(MoveSlot),
    MoveTarget(MoveTarget),
    NaturalGiftData(NaturalGiftData),
    Nature(Nature),
    SpecialItemData(SpecialItemData),
    Stat(Stat),
    StatTable(StatTable),
    Type(Type),

    EffectState(DynamicEffectStateConnector),

    List(Vec<Value>),
    Object(HashMap<String, Value>),
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

            Self::Battle => ValueType::Battle,
            Self::Field => ValueType::Field,
            Self::Format => ValueType::Format,
            Self::Side(_) => ValueType::Side,
            Self::Player(_) => ValueType::Player,
            Self::Mon(_) => ValueType::Mon,

            Self::Effect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::HitEffect(_) => ValueType::HitEffect,
            Self::SecondaryHitEffect(_) => ValueType::SecondaryHitEffect,

            Self::Accuracy(_) => ValueType::Accuracy,
            Self::Boost(_) => ValueType::Boost,
            Self::BoostTable(_) => ValueType::BoostTable,
            Self::FieldEnvironment(_) => ValueType::FieldEnvironment,
            Self::FlingData(_) => ValueType::FlingData,
            Self::Gender(_) => ValueType::Gender,
            Self::JudgmentData(_) => ValueType::JudgmentData,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveSlot(_) => ValueType::MoveSlot,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::NaturalGiftData(_) => ValueType::NaturalGiftData,
            Self::Nature(_) => ValueType::Nature,
            Self::SpecialItemData(_) => ValueType::SpecialItemData,
            Self::Stat(_) => ValueType::Stat,
            Self::StatTable(_) => ValueType::StatTable,
            Self::Type(_) => ValueType::Type,

            Self::EffectState(_) => ValueType::EffectState,

            Self::List(_) => ValueType::List,
            Self::Object(_) => ValueType::Object,
        }
    }

    fn invalid_type(got: ValueType, expected: ValueType) -> Error {
        general_error(format!("got {got}, expected {expected}"))
    }

    fn incompatible_type(from: ValueType, to: ValueType) -> Error {
        general_error(format!("cannot convert from {from} to {to}"))
    }

    /// Checks if the value signals an early exit from an event perspective.
    pub fn signals_early_exit(&self) -> bool {
        match self {
            Self::Boolean(false) => true,
            Self::Fraction(val) if val == &Fraction::from(0i64) => true,
            Self::UFraction(val) if val == &Fraction::from(0u64) => true,
            Self::String(val) => {
                MoveEventResult::from_str(val).is_ok_and(|result| !result.advance())
            }
            _ => false,
        }
    }

    /// Converts the value to the given type.
    pub fn convert_to(self, value_type: ValueType) -> Result<Self> {
        if self.value_type() == value_type {
            return Ok(self.clone());
        }

        match value_type {
            ValueType::Undefined => {
                if self.is_undefined() {
                    Ok(Value::Undefined)
                } else {
                    Err(Self::incompatible_type(self.value_type(), value_type))
                }
            }
            ValueType::Boolean => self.boolean().map(Value::Boolean),
            ValueType::Fraction => self.fraction_i64().map(Value::Fraction),
            ValueType::UFraction => self.fraction_u64().map(Value::UFraction),
            ValueType::String => self.string().map(Value::String),
            ValueType::Battle => {
                if self.is_battle() {
                    Ok(Value::Battle)
                } else {
                    Err(Self::incompatible_type(self.value_type(), value_type))
                }
            }
            ValueType::Field => {
                if self.is_field() {
                    Ok(Value::Field)
                } else {
                    Err(Self::incompatible_type(self.value_type(), value_type))
                }
            }
            ValueType::Format => {
                if self.is_format() {
                    Ok(Value::Format)
                } else {
                    Err(Self::incompatible_type(self.value_type(), value_type))
                }
            }
            ValueType::Side => self.side_index().map(Value::Side),
            ValueType::Player => self.player_index().map(Value::Player),
            ValueType::Mon => self.mon_handle().map(Value::Mon),
            ValueType::Effect => self.effect_handle().map(Value::Effect),
            ValueType::ActiveMove => self.active_move().map(Value::ActiveMove),
            ValueType::HitEffect => self.hit_effect().map(Value::HitEffect),
            ValueType::SecondaryHitEffect => {
                self.secondary_hit_effect().map(Value::SecondaryHitEffect)
            }
            ValueType::Accuracy => self.accuracy().map(Value::Accuracy),
            ValueType::Boost => self.boost().map(Value::Boost),
            ValueType::BoostTable => self.boost_table().map(Value::BoostTable),
            ValueType::FieldEnvironment => self.field_environment().map(Value::FieldEnvironment),
            ValueType::FlingData => self.fling_data().map(Value::FlingData),
            ValueType::Gender => self.gender().map(Value::Gender),
            ValueType::JudgmentData => self.judgment_data().map(Value::JudgmentData),
            ValueType::MoveCategory => self.move_category().map(Value::MoveCategory),
            ValueType::MoveSlot => self.move_slot().map(Value::MoveSlot),
            ValueType::MoveTarget => self.move_target().map(Value::MoveTarget),
            ValueType::NaturalGiftData => self.natural_gift_data().map(Value::NaturalGiftData),
            ValueType::Nature => self.nature().map(Value::Nature),
            ValueType::SpecialItemData => self.special_item_data().map(Value::SpecialItemData),
            ValueType::Stat => self.stat().map(Value::Stat),
            ValueType::StatTable => self.stat_table().map(Value::StatTable),
            ValueType::Type => self.mon_type().map(Value::Type),
            ValueType::EffectState => self.effect_state().map(Value::EffectState),
            ValueType::List => self.list().map(Value::List),
            ValueType::Object => self.object().map(Value::Object),
        }
    }

    /// Checks if the value is undefined.
    pub fn is_undefined(&self) -> bool {
        match self {
            Self::Undefined => true,
            _ => false,
        }
    }

    /// Consumes the value into a [`bool`].
    pub fn boolean(self) -> Result<bool> {
        match self {
            Self::Undefined => Ok(false),
            Self::Boolean(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Boolean)),
        }
    }

    /// Consumes the value into a [`Fraction<u64>`].
    pub fn fraction_u64(self) -> Result<Fraction<u64>> {
        match self {
            Self::Fraction(val) => val.try_convert().map_err(integer_overflow_error),
            Self::UFraction(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::UFraction)),
        }
    }

    /// Consumes the value into a [`Fraction<i64>`].
    pub fn fraction_i64(self) -> Result<Fraction<i64>> {
        match self {
            Self::Fraction(val) => Ok(val),
            Self::UFraction(val) => val.try_convert().map_err(integer_overflow_error),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::UFraction)),
        }
    }

    /// Consumes the value into a [`Fraction<u16>`].
    pub fn fraction_u16(self) -> Result<Fraction<u16>> {
        self.fraction_u64()?
            .try_convert()
            .map_err(integer_overflow_error)
    }

    /// Consumes the value into a [`u64`].
    pub fn integer_u64(self) -> Result<u64> {
        match self {
            Self::Fraction(val) => Ok(val.floor() as u64),
            Self::UFraction(val) => Ok(val.floor() as u64),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::UFraction)),
        }
    }

    /// Consumes the value into a [`i64`].
    pub fn integer_i64(self) -> Result<i64> {
        match self {
            Self::Fraction(val) => Ok(val.floor() as i64),
            Self::UFraction(val) => Ok(val.floor() as i64),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Fraction)),
        }
    }

    /// Consumes the value into an [`i32`].
    pub fn integer_i32(self) -> Result<i32> {
        self.integer_i64()?
            .try_into()
            .map_err(integer_overflow_error)
    }

    /// Consumes the value into an [`i8`].
    pub fn integer_i8(self) -> Result<i8> {
        self.integer_i64()?
            .try_into()
            .map_err(integer_overflow_error)
    }

    /// Consumes the value into a [`u8`].
    pub fn integer_u8(self) -> Result<u8> {
        self.integer_u64()?
            .try_into()
            .map_err(integer_overflow_error)
    }

    /// Consumes the value into a [`u16`].
    pub fn integer_u16(self) -> Result<u16> {
        self.integer_u64()?
            .try_into()
            .map_err(integer_overflow_error)
    }

    /// Consumes the value into a [`u32`].
    pub fn integer_u32(self) -> Result<u32> {
        self.integer_u64()?
            .try_into()
            .map_err(integer_overflow_error)
    }

    /// Consumes the value into an [`isize`].
    pub fn integer_isize(self) -> Result<isize> {
        self.integer_i64()?
            .try_into()
            .map_err(integer_overflow_error)
    }

    /// Consumes the value into a [`usize`].
    pub fn integer_usize(self) -> Result<usize> {
        self.integer_u64()?
            .try_into()
            .map_err(integer_overflow_error)
    }

    /// Consumes the value into a [`String`].
    pub fn string(self) -> Result<String> {
        match self {
            Self::String(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::String)),
        }
    }

    /// Consumes the value into an [`Id`].
    pub fn id(self) -> Result<Id> {
        self.string().map(|val| Id::from(val))
    }

    /// Checks if the value is a battle.
    pub fn is_battle(&self) -> bool {
        match self {
            Self::Battle => true,
            _ => false,
        }
    }

    /// Checks if the value is a field.
    pub fn is_field(&self) -> bool {
        match self {
            Self::Field => true,
            _ => false,
        }
    }

    /// Checks if the value is a format.
    pub fn is_format(&self) -> bool {
        match self {
            Self::Format => true,
            _ => false,
        }
    }

    /// Consumes the value into a side index.
    pub fn side_index(self) -> Result<usize> {
        match self {
            Self::Side(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Side)),
        }
    }

    /// Consumes the value into a player index.
    pub fn player_index(self) -> Result<usize> {
        match self {
            Self::Player(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Player)),
        }
    }

    /// Consumes the value into a [`MonHandle`].
    pub fn mon_handle(self) -> Result<MonHandle> {
        match self {
            Self::Mon(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Mon)),
        }
    }

    /// Consumes the value into an [`EffectHandle`].
    pub fn effect_handle(self) -> Result<EffectHandle> {
        match self {
            Self::Effect(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Effect)),
        }
    }

    /// Consumes the value into a move ID.
    pub fn move_id(self, context: &mut EvaluationContext) -> Result<Id> {
        match self {
            Self::String(val) => Ok(Id::from(val)),
            Self::Effect(EffectHandle::InactiveMove(val)) => Ok(val),
            Self::ActiveMove(val) | Self::Effect(EffectHandle::ActiveMove(val, _)) => {
                Ok(context.active_move(val)?.id().clone())
            }
            val @ _ => Err(general_error(format!(
                "value of type {} cannot be converted to a move id",
                val.value_type(),
            ))),
        }
    }

    /// Consumes the value into an ability ID.
    pub fn ability_id(self) -> Result<Id> {
        match self {
            Self::String(val) => Ok(Id::from(val)),
            Self::Effect(EffectHandle::Ability(val)) => Ok(val),
            val @ _ => Err(general_error(format!(
                "value of type {} cannot be converted to an ability id",
                val.value_type(),
            ))),
        }
    }

    /// Consumes the value into a item ID.
    pub fn item_id(self) -> Result<Id> {
        match self {
            Self::String(val) => Ok(Id::from(val)),
            Self::Effect(EffectHandle::Item(val)) => Ok(val),
            val @ _ => Err(general_error(format!(
                "value of type {} cannot be converted to a item id",
                val.value_type(),
            ))),
        }
    }

    /// Consumes the value into a clause ID.
    pub fn clause_id(self) -> Result<Id> {
        match self {
            Self::String(val) => Ok(Id::from(val)),
            Self::Effect(EffectHandle::Clause(val)) => Ok(val),
            val @ _ => Err(general_error(format!(
                "value of type {} cannot be converted to a clause id",
                val.value_type(),
            ))),
        }
    }

    /// Consumes the value into a species ID.
    pub fn species_id(self) -> Result<Id> {
        match self {
            Self::String(val) => Ok(Id::from(val)),
            Self::Effect(EffectHandle::Species(val)) => Ok(val),
            val @ _ => Err(general_error(format!(
                "value of type {} cannot be converted to a species id",
                val.value_type(),
            ))),
        }
    }

    /// Consumes the value into a [`MoveHandle`].
    pub fn active_move(self) -> Result<MoveHandle> {
        match self {
            Self::Effect(EffectHandle::ActiveMove(val, _)) => Ok(val),
            Self::ActiveMove(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::ActiveMove)),
        }
    }

    /// Consumes the value into a [`HitEffect`].
    pub fn hit_effect(self) -> Result<HitEffect> {
        match self {
            Self::HitEffect(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::HitEffect)),
        }
    }

    /// Consumes the value into a [`SecondaryEffectData`].
    pub fn secondary_hit_effect(self) -> Result<SecondaryEffectData> {
        match self {
            Self::SecondaryHitEffect(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(
                val.value_type(),
                ValueType::SecondaryHitEffect,
            )),
        }
    }

    /// Consumes the value into an [`Accuracy`].
    pub fn accuracy(self) -> Result<Accuracy> {
        match self {
            Self::Fraction(val) => Ok(Accuracy::from(
                TryInto::<u8>::try_into((val * 100).floor()).map_err(general_error)?,
            )),
            Self::UFraction(val) => Ok(Accuracy::from(
                TryInto::<u8>::try_into((val * 100).floor()).map_err(general_error)?,
            )),
            Self::String(val) => Accuracy::from_str(&val).map_err(general_error),
            Self::Accuracy(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Accuracy)),
        }
    }

    /// Consumes the value into a [`Boost`].
    pub fn boost(self) -> Result<Boost> {
        match self {
            Self::String(val) => Boost::from_str(&val).map_err(general_error),
            Self::Boost(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Boost)),
        }
    }

    /// Consumes the value into a [`BoostTable`].
    pub fn boost_table(self) -> Result<BoostTable> {
        match self {
            Self::BoostTable(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::BoostTable)),
        }
    }

    /// Consumes the value into a [`FieldEnvironment`].
    pub fn field_environment(self) -> Result<FieldEnvironment> {
        match self {
            Self::String(val) => FieldEnvironment::from_str(&val).map_err(general_error),
            Self::FieldEnvironment(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(
                val.value_type(),
                ValueType::FieldEnvironment,
            )),
        }
    }

    /// Consumes the value into a [`FlingData`].
    pub fn fling_data(self) -> Result<FlingData> {
        match self {
            Self::FlingData(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::FlingData)),
        }
    }

    /// Consumes the value into a [`Gender`].
    pub fn gender(self) -> Result<Gender> {
        match self {
            Self::String(val) => Gender::from_str(&val).map_err(general_error),
            Self::Gender(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Gender)),
        }
    }

    /// Consumes the value into a [`JudgmentData`].
    pub fn judgment_data(self) -> Result<JudgmentData> {
        match self {
            Self::JudgmentData(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(
                val.value_type(),
                ValueType::JudgmentData,
            )),
        }
    }

    /// Consumes the value into a [`MoveCategory`].
    pub fn move_category(self) -> Result<MoveCategory> {
        match self {
            Self::String(val) => MoveCategory::from_str(&val).map_err(general_error),
            Self::MoveCategory(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(
                val.value_type(),
                ValueType::MoveCategory,
            )),
        }
    }

    /// Consumes the value into a [`MoveOutcomeOnTarget`].
    pub fn move_outcome_on_target(self) -> Result<MoveOutcomeOnTarget> {
        match self {
            Self::Boolean(val) => Ok(MoveOutcomeOnTarget::from(val)),
            _ => Ok(MoveOutcomeOnTarget::Damage(self.integer_u16()?)),
        }
    }

    /// Consumes the value into a [`MoveEventResult`].
    pub fn move_result(self) -> Result<MoveEventResult> {
        match self {
            Self::Boolean(val) => Ok(MoveEventResult::from(val)),
            Self::String(val) => MoveEventResult::from_str(&val).map_err(general_error),
            val @ _ => Err(general_error(format!(
                "value of type {} cannot be converted to a move event result",
                val.value_type(),
            ))),
        }
    }

    /// Consumes the value into a [`MoveSlot`].
    pub fn move_slot(self) -> Result<MoveSlot> {
        match self {
            Self::MoveSlot(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::MoveSlot)),
        }
    }

    /// Consumes the value into a [`MoveTarget`].
    pub fn move_target(self) -> Result<MoveTarget> {
        match self {
            Self::String(val) => MoveTarget::from_str(&val).map_err(general_error),
            Self::MoveTarget(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::MoveTarget)),
        }
    }

    /// Consumes the value into a [`MultihitType`].
    pub fn multihit_type(self) -> Result<MultihitType> {
        match self {
            Self::Fraction(val) => Ok(MultihitType::Static(
                val.floor().try_into().map_err(integer_overflow_error)?,
            )),
            Self::UFraction(val) => Ok(MultihitType::Static(
                val.floor().try_into().map_err(integer_overflow_error)?,
            )),
            val @ _ => Err(general_error(format!(
                "value of type {} cannot be converted to a multihit type",
                val.value_type(),
            ))),
        }
    }

    /// Consumes the value into a [`NaturalGiftData`].
    pub fn natural_gift_data(self) -> Result<NaturalGiftData> {
        match self {
            Self::NaturalGiftData(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(
                val.value_type(),
                ValueType::NaturalGiftData,
            )),
        }
    }

    /// Consumes the value into a [`Nature`].
    pub fn nature(self) -> Result<Nature> {
        match self {
            Self::String(val) => Nature::from_str(&val).map_err(general_error),
            Self::Nature(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Nature)),
        }
    }

    /// Consumes the value into a [`SpecialItemData`].
    pub fn special_item_data(self) -> Result<SpecialItemData> {
        match self {
            Self::SpecialItemData(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(
                val.value_type(),
                ValueType::SpecialItemData,
            )),
        }
    }

    /// Consumes the value into a [`Stat`].
    pub fn stat(self) -> Result<Stat> {
        match self {
            Self::Stat(val) => Ok(val),
            Self::String(val) => Stat::from_str(&val).map_err(general_error),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Stat)),
        }
    }

    /// Consumes the value into a [`StatTable`].
    pub fn stat_table(self) -> Result<StatTable> {
        match self {
            Self::StatTable(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::StatTable)),
        }
    }

    /// Consumes the value into a [`Type`].
    pub fn mon_type(self) -> Result<Type> {
        match self {
            Self::String(val) => Type::from_str(&val).map_err(general_error),
            Self::Type(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Type)),
        }
    }

    /// Consumes the value into a [`DynamicEffectStateConnector`].
    pub fn effect_state(self) -> Result<DynamicEffectStateConnector> {
        match self {
            Self::EffectState(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::EffectState)),
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
    pub fn list(self) -> Result<Vec<Value>> {
        match self {
            Self::List(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::List)),
        }
    }

    /// Consumes the value into a [`Vec<Type>`].
    pub fn types_list(self) -> Result<Vec<Type>> {
        self.list()?.into_iter().map(|val| val.mon_type()).collect()
    }

    /// Consumes the value into a [`Vec<MonHandle>`].
    pub fn mons_list(self) -> Result<Vec<MonHandle>> {
        self.list()?
            .into_iter()
            .map(|val| val.mon_handle())
            .collect()
    }

    /// Consumes the value into a [`Vec<String>`].
    pub fn strings_list(self) -> Result<Vec<String>> {
        self.list()?.into_iter().map(|val| val.string()).collect()
    }

    /// Consumes the value into a [`Vec<SecondaryEffectData>`].
    pub fn secondary_hit_effects_list(self) -> Result<Vec<SecondaryEffectData>> {
        self.list()?
            .into_iter()
            .map(|val| val.secondary_hit_effect())
            .collect()
    }

    /// Consumes the value into a [`HashMap<String, Value>`].
    pub fn object(self) -> Result<HashMap<String, Value>> {
        match self {
            Self::Object(val) => Ok(val),
            val @ _ => Err(Self::invalid_type(val.value_type(), ValueType::Object)),
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

    Battle,
    Field,
    Format,
    Side(usize),
    Player(usize),
    Mon(MonHandle),

    Effect(EffectHandle),
    ActiveMove(MoveHandle),
    HitEffect(HitEffect),
    SecondaryHitEffect(SecondaryEffectData),

    Accuracy(Accuracy),
    Boost(Boost),
    BoostTable(BoostTable),
    FieldEnvironment(FieldEnvironment),
    FlingData(FlingData),
    Gender(Gender),
    JudgmentData(JudgmentData),
    MoveCategory(MoveCategory),
    MoveSlot(MoveSlot),
    MoveTarget(MoveTarget),
    NaturalGiftData(NaturalGiftData),
    Nature(Nature),
    SpecialItemData(SpecialItemData),
    Stat(Stat),
    StatTable(StatTable),
    Type(Type),

    EffectState(DynamicEffectStateConnector),

    List(Vec<MaybeReferenceValue<'eval>>),
    Object(HashMap<String, MaybeReferenceValue<'eval>>),

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

            Self::Battle => ValueType::Battle,
            Self::Field => ValueType::Field,
            Self::Format => ValueType::Format,
            Self::Side(_) => ValueType::Side,
            Self::Player(_) => ValueType::Player,
            Self::Mon(_) => ValueType::Mon,

            Self::Effect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::HitEffect(_) => ValueType::HitEffect,
            Self::SecondaryHitEffect(_) => ValueType::SecondaryHitEffect,

            Self::Accuracy(_) => ValueType::Accuracy,
            Self::Boost(_) => ValueType::Boost,
            Self::BoostTable(_) => ValueType::BoostTable,
            Self::FieldEnvironment(_) => ValueType::FieldEnvironment,
            Self::FlingData(_) => ValueType::FlingData,
            Self::Gender(_) => ValueType::Gender,
            Self::JudgmentData(_) => ValueType::JudgmentData,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveSlot(_) => ValueType::MoveSlot,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::NaturalGiftData(_) => ValueType::NaturalGiftData,
            Self::Nature(_) => ValueType::Nature,
            Self::SpecialItemData(_) => ValueType::SpecialItemData,
            Self::Stat(_) => ValueType::Stat,
            Self::StatTable(_) => ValueType::StatTable,
            Self::Type(_) => ValueType::Type,

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

            Self::Battle => Value::Battle,
            Self::Field => Value::Field,
            Self::Format => Value::Format,
            Self::Side(val) => Value::Side(*val),
            Self::Player(val) => Value::Player(*val),
            Self::Mon(val) => Value::Mon(*val),

            Self::Effect(val) => Value::Effect(val.clone()),
            Self::ActiveMove(val) => Value::ActiveMove(*val),
            Self::HitEffect(val) => Value::HitEffect(val.clone()),
            Self::SecondaryHitEffect(val) => Value::SecondaryHitEffect(val.clone()),

            Self::Accuracy(val) => Value::Accuracy(*val),
            Self::Boost(val) => Value::Boost(*val),
            Self::BoostTable(val) => Value::BoostTable(val.clone()),
            Self::FieldEnvironment(val) => Value::FieldEnvironment(*val),
            Self::FlingData(val) => Value::FlingData(val.clone()),
            Self::Gender(val) => Value::Gender(*val),
            Self::JudgmentData(val) => Value::JudgmentData(val.clone()),
            Self::MoveCategory(val) => Value::MoveCategory(*val),
            Self::MoveTarget(val) => Value::MoveTarget(*val),
            Self::MoveSlot(val) => Value::MoveSlot(val.clone()),
            Self::NaturalGiftData(val) => Value::NaturalGiftData(val.clone()),
            Self::Nature(val) => Value::Nature(*val),
            Self::SpecialItemData(val) => Value::SpecialItemData(val.clone()),
            Self::Stat(val) => Value::Stat(*val),
            Self::StatTable(val) => Value::StatTable(val.clone()),
            Self::Type(val) => Value::Type(*val),

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
            Self::Undefined => Some(false),
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

            Value::Battle => Self::Battle,
            Value::Field => Self::Field,
            Value::Format => Self::Field,
            Value::Side(val) => Self::Side(val),
            Value::Player(val) => Self::Player(val),
            Value::Mon(val) => Self::Mon(val),

            Value::Effect(val) => Self::Effect(val),
            Value::ActiveMove(val) => Self::ActiveMove(val),
            Value::HitEffect(val) => Self::HitEffect(val),
            Value::SecondaryHitEffect(val) => Self::SecondaryHitEffect(val),

            Value::Accuracy(val) => Self::Accuracy(val),
            Value::Boost(val) => Self::Boost(val),
            Value::BoostTable(val) => Self::BoostTable(val),
            Value::FieldEnvironment(val) => Self::FieldEnvironment(val),
            Value::FlingData(val) => Self::FlingData(val),
            Value::Gender(val) => Self::Gender(val),
            Value::JudgmentData(val) => Self::JudgmentData(val),
            Value::MoveCategory(val) => Self::MoveCategory(val),
            Value::MoveSlot(val) => Self::MoveSlot(val),
            Value::MoveTarget(val) => Self::MoveTarget(val),
            Value::NaturalGiftData(val) => Self::NaturalGiftData(val),
            Value::Nature(val) => Self::Nature(val),
            Value::SpecialItemData(val) => Self::SpecialItemData(val),
            Value::Stat(val) => Self::Stat(val),
            Value::StatTable(val) => Self::StatTable(val),
            Value::Type(val) => Self::Type(val),

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
///
/// Primitive types are copied for simplicity. Some types have "temporary" alternatives that
/// represent a new value in a place where a reference would be expected.
#[derive(Clone)]
pub enum ValueRef<'eval> {
    Undefined,
    Boolean(bool),
    Fraction(Fraction<i64>),
    UFraction(Fraction<u64>),
    String(&'eval String),
    Str(&'eval str),
    TempString(String),

    Battle,
    Field,
    Format,
    Side(usize),
    Player(usize),
    Mon(MonHandle),

    Effect(&'eval EffectHandle),
    TempEffect(EffectHandle),
    ActiveMove(MoveHandle),
    HitEffect(&'eval HitEffect),
    SecondaryHitEffect(&'eval SecondaryEffectData),

    Accuracy(Accuracy),
    Boost(Boost),
    BoostTable(&'eval BoostTable),
    FieldEnvironment(FieldEnvironment),
    FlingData(&'eval FlingData),
    Gender(Gender),
    JudgmentData(&'eval JudgmentData),
    MoveCategory(MoveCategory),
    MoveSlot(&'eval MoveSlot),
    MoveTarget(MoveTarget),
    NaturalGiftData(&'eval NaturalGiftData),
    Nature(Nature),
    SpecialItemData(&'eval SpecialItemData),
    Stat(Stat),
    StatTable(&'eval StatTable),
    Type(Type),

    EffectState(DynamicEffectStateConnector),

    List(&'eval Vec<Value>),
    TempList(Vec<ValueRefToStoredValue<'eval>>),
    Object(&'eval HashMap<String, Value>),
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

            Self::Battle => ValueType::Battle,
            Self::Field => ValueType::Field,
            Self::Format => ValueType::Format,
            Self::Side(_) => ValueType::Side,
            Self::Player(_) => ValueType::Player,
            Self::Mon(_) => ValueType::Mon,

            Self::Effect(_) => ValueType::Effect,
            Self::TempEffect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::HitEffect(_) => ValueType::HitEffect,
            Self::SecondaryHitEffect(_) => ValueType::SecondaryHitEffect,

            Self::Accuracy(_) => ValueType::Accuracy,
            Self::Boost(_) => ValueType::Boost,
            Self::BoostTable(_) => ValueType::BoostTable,
            Self::FieldEnvironment(_) => ValueType::FieldEnvironment,
            Self::FlingData(_) => ValueType::FlingData,
            Self::Gender(_) => ValueType::Gender,
            Self::JudgmentData(_) => ValueType::JudgmentData,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveSlot(_) => ValueType::MoveSlot,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::NaturalGiftData(_) => ValueType::NaturalGiftData,
            Self::Nature(_) => ValueType::Nature,
            Self::SpecialItemData(_) => ValueType::SpecialItemData,
            Self::Stat(_) => ValueType::Stat,
            Self::StatTable(_) => ValueType::StatTable,
            Self::Type(_) => ValueType::Type,

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

            Self::Battle => Value::Battle,
            Self::Field => Value::Field,
            Self::Format => Value::Format,
            Self::Side(val) => Value::Side(*val),
            Self::Player(val) => Value::Player(*val),
            Self::Mon(val) => Value::Mon(*val),

            Self::Effect(val) => Value::Effect((*val).clone()),
            Self::TempEffect(val) => Value::Effect(val.clone()),
            Self::ActiveMove(val) => Value::ActiveMove(*val),
            Self::HitEffect(val) => Value::HitEffect((*val).clone()),
            Self::SecondaryHitEffect(val) => Value::SecondaryHitEffect((*val).clone()),

            Self::Accuracy(val) => Value::Accuracy(*val),
            Self::Boost(val) => Value::Boost(*val),
            Self::BoostTable(val) => Value::BoostTable((*val).clone()),
            Self::FieldEnvironment(val) => Value::FieldEnvironment(*val),
            Self::FlingData(val) => Value::FlingData((*val).clone()),
            Self::Gender(val) => Value::Gender(*val),
            Self::JudgmentData(val) => Value::JudgmentData((*val).clone()),
            Self::MoveCategory(val) => Value::MoveCategory(*val),
            Self::MoveSlot(val) => Value::MoveSlot((*val).clone()),
            Self::MoveTarget(val) => Value::MoveTarget(*val),
            Self::NaturalGiftData(val) => Value::NaturalGiftData((*val).clone()),
            Self::Nature(val) => Value::Nature(*val),
            Self::SpecialItemData(val) => Value::SpecialItemData((*val).clone()),
            Self::Stat(val) => Value::Stat(*val),
            Self::StatTable(val) => Value::StatTable((*val).clone()),
            Self::Type(val) => Value::Type(*val),

            Self::EffectState(val) => Value::EffectState(val.clone()),

            Self::List(val) => Value::List((*val).clone()),
            Self::TempList(val) => Value::List(val.iter().map(|val| val.to_owned()).collect()),
            Self::Object(val) => Value::Object((*val).clone()),
        }
    }

    /// Checks if the value is undefined.
    pub fn is_undefined(&self) -> bool {
        match self {
            Self::Undefined => true,
            _ => false,
        }
    }

    /// Checks if the value is a [`bool`].
    pub fn is_boolean(&self) -> bool {
        match self {
            Self::Boolean(_) => true,
            _ => false,
        }
    }

    /// Converts the value to a boolean, if possible.
    pub fn boolean(&self) -> Option<bool> {
        match self {
            Self::Undefined => Some(false),
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

    /// Returns if the container is empty.
    ///
    /// `false` by default.
    pub fn is_empty(&self) -> bool {
        match self.len() {
            Some(len) => len == 0,
            None => false,
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
    pub fn effect_handle(&self) -> Option<EffectHandle> {
        match self {
            Self::ActiveMove(move_handle) => Some(EffectHandle::ActiveMove(
                *move_handle,
                MoveHitEffectType::PrimaryEffect,
            )),
            Self::Effect(effect_handle) => Some((*effect_handle).clone()),
            Self::TempEffect(effect_handle) => Some((*effect_handle).clone()),
            _ => None,
        }
    }

    /// Returns the [`MoveHandle`] associated with an active move reference.
    pub fn active_move_handle(&self) -> Option<MoveHandle> {
        match self {
            Self::ActiveMove(move_handle) => Some(*move_handle),
            Self::Effect(EffectHandle::ActiveMove(move_handle, _)) => Some(*move_handle),
            Self::TempEffect(EffectHandle::ActiveMove(move_handle, _)) => Some(*move_handle),
            _ => None,
        }
    }

    /// Returns the [`SpecialItemData`] associated with a reference.
    pub fn special_item_data(&self) -> Option<&SpecialItemData> {
        match self {
            Self::SpecialItemData(data) => Some(data),
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

            Value::Battle => Self::Battle,
            Value::Field => Self::Field,
            Value::Format => Self::Format,
            Value::Side(val) => Self::Side(*val),
            Value::Player(val) => Self::Player(*val),
            Value::Mon(val) => Self::Mon(*val),

            Value::Effect(val) => Self::Effect(val),
            Value::ActiveMove(val) => Self::ActiveMove(*val),
            Value::HitEffect(val) => Self::HitEffect(val),
            Value::SecondaryHitEffect(val) => Self::SecondaryHitEffect(val),

            Value::Accuracy(val) => Self::Accuracy(*val),
            Value::Boost(val) => Self::Boost(*val),
            Value::BoostTable(val) => Self::BoostTable(val),
            Value::FieldEnvironment(val) => Self::FieldEnvironment(*val),
            Value::FlingData(val) => Self::FlingData(val),
            Value::Gender(val) => Self::Gender(*val),
            Value::JudgmentData(val) => Self::JudgmentData(val),
            Value::MoveCategory(val) => Self::MoveCategory(*val),
            Value::MoveSlot(val) => Self::MoveSlot(val),
            Value::MoveTarget(val) => Self::MoveTarget(*val),
            Value::NaturalGiftData(val) => Self::NaturalGiftData(val),
            Value::Nature(val) => Self::Nature(*val),
            Value::SpecialItemData(val) => Self::SpecialItemData(val),
            Value::Stat(val) => Self::Stat(*val),
            Value::StatTable(val) => Self::StatTable(val),
            Value::Type(val) => Self::Type(*val),

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

    /// Converts the reference to a [`Value`], which is guaranteed to contain no references.
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
///
/// Any type that should be writable during evaluation must be represented here.
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
    OptionalFractionU16(&'eval mut Option<Fraction<u16>>),
    String(&'eval mut String),
    OptionalString(&'eval mut Option<String>),
    OptionalId(&'eval mut Option<Id>),

    Battle,
    Field,
    Format,
    Side(&'eval mut usize),
    Player(&'eval mut usize),
    Mon(&'eval mut MonHandle),
    OptionalMon(&'eval mut Option<MonHandle>),

    Effect(&'eval mut EffectHandle),
    ActiveMove(&'eval mut MoveHandle),
    HitEffect(&'eval mut HitEffect),
    OptionalHitEffect(&'eval mut Option<HitEffect>),
    SecondaryHitEffect(&'eval mut SecondaryEffectData),
    SecondaryHitEffectList(&'eval mut Vec<SecondaryEffectData>),

    Accuracy(&'eval mut Accuracy),
    Boost(&'eval mut Boost),
    BoostTable(&'eval mut BoostTable),
    OptionalBoostTable(&'eval mut Option<BoostTable>),
    FieldEnvironment(&'eval mut FieldEnvironment),
    FlingData(&'eval mut FlingData),
    Gender(&'eval mut Gender),
    JudgmentData(&'eval mut JudgmentData),
    MoveCategory(&'eval mut MoveCategory),
    MoveSlot(&'eval mut MoveSlot),
    MoveTarget(&'eval mut MoveTarget),
    OptionalMultihitType(&'eval mut Option<MultihitType>),
    NaturalGiftData(&'eval mut NaturalGiftData),
    Nature(&'eval mut Nature),
    SpecialItemData(&'eval mut SpecialItemData),
    Stat(&'eval mut Stat),
    StatTable(&'eval mut StatTable),
    Type(&'eval mut Type),

    EffectState(&'eval mut DynamicEffectStateConnector),
    TempEffectState(DynamicEffectStateConnector),

    List(&'eval mut Vec<Value>),
    Object(&'eval mut HashMap<String, Value>),
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
            Self::OptionalU16(_) => ValueType::UFraction,
            Self::Fraction(_) => ValueType::Fraction,
            Self::UFraction(_) => ValueType::UFraction,
            Self::OptionalFractionU16(_) => ValueType::UFraction,
            Self::String(_) => ValueType::String,
            Self::OptionalString(_) => ValueType::String,
            Self::OptionalId(_) => ValueType::String,

            Self::Battle => ValueType::Battle,
            Self::Field => ValueType::Field,
            Self::Format => ValueType::Format,
            Self::Side(_) => ValueType::Side,
            Self::Player(_) => ValueType::Player,
            Self::Mon(_) => ValueType::Mon,
            Self::OptionalMon(_) => ValueType::Mon,

            Self::Effect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::HitEffect(_) => ValueType::HitEffect,
            Self::OptionalHitEffect(_) => ValueType::HitEffect,
            Self::SecondaryHitEffect(_) => ValueType::SecondaryHitEffect,
            Self::SecondaryHitEffectList(_) => ValueType::List,

            Self::Accuracy(_) => ValueType::Accuracy,
            Self::Boost(_) => ValueType::Boost,
            Self::BoostTable(_) => ValueType::BoostTable,
            Self::OptionalBoostTable(_) => ValueType::BoostTable,
            Self::FieldEnvironment(_) => ValueType::FieldEnvironment,
            Self::FlingData(_) => ValueType::FlingData,
            Self::Gender(_) => ValueType::Gender,
            Self::JudgmentData(_) => ValueType::JudgmentData,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveSlot(_) => ValueType::MoveSlot,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::OptionalMultihitType(_) => ValueType::UFraction,
            Self::NaturalGiftData(_) => ValueType::NaturalGiftData,
            Self::Nature(_) => ValueType::Nature,
            Self::SpecialItemData(_) => ValueType::SpecialItemData,
            Self::Stat(_) => ValueType::Stat,
            Self::StatTable(_) => ValueType::StatTable,
            Self::Type(_) => ValueType::Type,

            Self::EffectState(_) => ValueType::EffectState,
            Self::TempEffectState(_) => ValueType::EffectState,

            Self::List(_) => ValueType::List,
            Self::Object(_) => ValueType::Object,
        }
    }

    pub fn assign(self, val: Value) -> Result<()> {
        let self_type = self.value_type();
        let assigning_type = val.value_type();
        match self {
            // A variable can be initialized to any value.
            ValueRefMut::Undefined(var) => *var = val,
            ValueRefMut::Boolean(var) => {
                *var = val.boolean()?;
            }
            ValueRefMut::OptionalBoolean(var) => {
                *var = (!val.is_undefined()).then(|| val.boolean()).transpose()?;
            }
            ValueRefMut::I8(var) => {
                *var = val.integer_i8()?;
            }
            ValueRefMut::U16(var) => {
                *var = val.integer_u16()?;
            }
            ValueRefMut::U32(var) => {
                *var = val.integer_u32()?;
            }
            ValueRefMut::U64(var) => {
                *var = val.integer_u64()?;
            }
            ValueRefMut::I64(var) => {
                *var = val.integer_i64()?;
            }
            ValueRefMut::OptionalISize(var) => {
                *var = (!val.is_undefined())
                    .then(|| val.integer_isize())
                    .transpose()?;
            }
            ValueRefMut::OptionalU16(var) => {
                *var = Some(val.integer_u16()?);
            }
            ValueRefMut::Fraction(var) => {
                *var = val.fraction_i64()?;
            }
            ValueRefMut::UFraction(var) => {
                *var = val.fraction_u64()?;
            }
            ValueRefMut::OptionalFractionU16(var) => {
                *var = (!val.is_undefined())
                    .then(|| val.fraction_u16())
                    .transpose()?;
            }
            ValueRefMut::String(var) => {
                *var = val.string()?;
            }
            ValueRefMut::OptionalString(var) => {
                *var = (!val.is_undefined()).then(|| val.string()).transpose()?;
            }
            ValueRefMut::OptionalId(var) => {
                *var = (!val.is_undefined()).then(|| val.id()).transpose()?;
            }
            ValueRefMut::Battle => {
                if !val.is_battle() {
                    return Err(Value::incompatible_type(assigning_type, self_type));
                }
            }
            ValueRefMut::Field => {
                if !val.is_field() {
                    return Err(Value::incompatible_type(assigning_type, self_type));
                }
            }
            ValueRefMut::Format => {
                if !val.is_format() {
                    return Err(Value::incompatible_type(assigning_type, self_type));
                }
            }
            ValueRefMut::Side(var) => {
                *var = val.side_index()?;
            }
            ValueRefMut::Player(var) => {
                *var = val.player_index()?;
            }
            ValueRefMut::Mon(var) => {
                *var = val.mon_handle()?;
            }
            ValueRefMut::OptionalMon(var) => {
                *var = (!val.is_undefined())
                    .then(|| val.mon_handle())
                    .transpose()?;
            }
            ValueRefMut::Effect(var) => {
                *var = val.effect_handle()?;
            }
            ValueRefMut::ActiveMove(var) => {
                *var = val.active_move()?;
            }
            ValueRefMut::HitEffect(var) => {
                *var = val.hit_effect()?;
            }
            ValueRefMut::OptionalHitEffect(var) => {
                *var = (!val.is_undefined())
                    .then(|| val.hit_effect())
                    .transpose()?;
            }
            ValueRefMut::SecondaryHitEffect(var) => {
                *var = val.secondary_hit_effect()?;
            }
            ValueRefMut::SecondaryHitEffectList(var) => {
                *var = val.secondary_hit_effects_list()?;
            }
            ValueRefMut::Accuracy(var) => {
                *var = val.accuracy()?;
            }
            ValueRefMut::Boost(var) => {
                *var = val.boost()?;
            }
            ValueRefMut::BoostTable(var) => {
                *var = val.boost_table()?;
            }
            ValueRefMut::OptionalBoostTable(var) => {
                *var = (!val.is_undefined())
                    .then(|| val.boost_table())
                    .transpose()?;
            }
            ValueRefMut::FieldEnvironment(var) => {
                *var = val.field_environment()?;
            }
            ValueRefMut::FlingData(var) => {
                *var = val.fling_data()?;
            }
            ValueRefMut::Gender(var) => {
                *var = val.gender()?;
            }
            ValueRefMut::JudgmentData(var) => {
                *var = val.judgment_data()?;
            }
            ValueRefMut::MoveCategory(var) => {
                *var = val.move_category()?;
            }
            ValueRefMut::MoveSlot(var) => {
                *var = val.move_slot()?;
            }
            ValueRefMut::MoveTarget(var) => {
                *var = val.move_target()?;
            }
            ValueRefMut::OptionalMultihitType(var) => {
                *var = (!val.is_undefined())
                    .then(|| val.multihit_type())
                    .transpose()?;
            }
            ValueRefMut::NaturalGiftData(var) => {
                *var = val.natural_gift_data()?;
            }
            ValueRefMut::Nature(var) => {
                *var = val.nature()?;
            }
            ValueRefMut::SpecialItemData(var) => {
                *var = val.special_item_data()?;
            }
            ValueRefMut::Stat(var) => {
                *var = val.stat()?;
            }
            ValueRefMut::StatTable(var) => {
                *var = val.stat_table()?;
            }
            ValueRefMut::Type(var) => {
                *var = val.mon_type()?;
            }
            ValueRefMut::EffectState(var) => {
                *var = val.effect_state()?;
            }
            ValueRefMut::List(var) => {
                *var = val.list()?;
            }
            ValueRefMut::Object(var) => {
                *var = val.object()?;
            }
            _ => return Err(general_error("variable is unassignable")),
        }
        Ok(())
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

            Value::Battle => Self::Battle,
            Value::Field => Self::Field,
            Value::Format => Self::Format,
            Value::Side(val) => Self::Side(val),
            Value::Player(val) => Self::Player(val),
            Value::Mon(val) => Self::Mon(val),

            Value::Effect(val) => Self::Effect(val),
            Value::ActiveMove(val) => Self::ActiveMove(val),
            Value::HitEffect(val) => Self::HitEffect(val),
            Value::SecondaryHitEffect(val) => Self::SecondaryHitEffect(val),

            Value::Accuracy(val) => Self::Accuracy(val),
            Value::Boost(val) => Self::Boost(val),
            Value::BoostTable(val) => Self::BoostTable(val),
            Value::FieldEnvironment(val) => Self::FieldEnvironment(val),
            Value::FlingData(val) => Self::FlingData(val),
            Value::Gender(val) => Self::Gender(val),
            Value::JudgmentData(val) => Self::JudgmentData(val),
            Value::MoveCategory(val) => Self::MoveCategory(val),
            Value::MoveSlot(val) => Self::MoveSlot(val),
            Value::MoveTarget(val) => Self::MoveTarget(val),
            Value::NaturalGiftData(val) => Self::NaturalGiftData(val),
            Value::Nature(val) => Self::Nature(val),
            Value::SpecialItemData(val) => Self::SpecialItemData(val),
            Value::Stat(val) => Self::Stat(val),
            Value::StatTable(val) => Self::StatTable(val),
            Value::Type(val) => Self::Type(val),

            Value::EffectState(val) => Self::EffectState(val),

            Value::List(val) => Self::List(val),
            Value::Object(val) => Self::Object(val),
        }
    }
}

/// The value type used for operations.
///
/// Conceptually, this type is a union of [`MaybeReferenceValue`] and [`ValueRef`]. This type is
/// needed because there is a distinction between owned values and reference values. For example, a
/// program may compare a list stored as a variable (consisting of [`Value`] objects) and a list
/// generated at runtime that is not stored as a variable (consisting of [`MaybeReferenceValue`]
/// objects). This type allows these two lists to be operated on directly, without needing to
/// allocate memory for an extra list for either one.
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

    Battle,
    Field,
    Format,
    Side(usize),
    Player(usize),
    Mon(MonHandle),

    Effect(&'eval EffectHandle),
    TempEffect(EffectHandle),
    ActiveMove(MoveHandle),
    HitEffect(&'eval HitEffect),
    SecondaryHitEffect(&'eval SecondaryEffectData),

    Accuracy(Accuracy),
    Boost(Boost),
    BoostTable(&'eval BoostTable),
    FieldEnvironment(FieldEnvironment),
    FlingData(&'eval FlingData),
    Gender(Gender),
    JudgmentData(&'eval JudgmentData),
    MoveCategory(MoveCategory),
    MoveSlot(&'eval MoveSlot),
    MoveTarget(MoveTarget),
    NaturalGiftData(&'eval NaturalGiftData),
    Nature(Nature),
    SpecialItemData(&'eval SpecialItemData),
    TempSpecialItemData(SpecialItemData),
    Stat(Stat),
    StatTable(&'eval StatTable),
    Type(Type),

    EffectState(DynamicEffectStateConnector),

    List(&'eval Vec<MaybeReferenceValue<'eval>>),
    StoredList(&'eval Vec<Value>),
    TempList(Vec<MaybeReferenceValue<'eval>>),
    Object(&'eval HashMap<String, MaybeReferenceValue<'eval>>),
    StoredObject(&'eval HashMap<String, Value>),
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

            Self::Battle => ValueType::Battle,
            Self::Field => ValueType::Field,
            Self::Format => ValueType::Format,
            Self::Side(_) => ValueType::Side,
            Self::Player(_) => ValueType::Player,
            Self::Mon(_) => ValueType::Mon,

            Self::Effect(_) => ValueType::Effect,
            Self::TempEffect(_) => ValueType::Effect,
            Self::ActiveMove(_) => ValueType::ActiveMove,
            Self::HitEffect(_) => ValueType::HitEffect,
            Self::SecondaryHitEffect(_) => ValueType::SecondaryHitEffect,

            Self::Accuracy(_) => ValueType::Accuracy,
            Self::Boost(_) => ValueType::Boost,
            Self::BoostTable(_) => ValueType::BoostTable,
            Self::FieldEnvironment(_) => ValueType::FieldEnvironment,
            Self::FlingData(_) => ValueType::FlingData,
            Self::Gender(_) => ValueType::Gender,
            Self::JudgmentData(_) => ValueType::JudgmentData,
            Self::MoveCategory(_) => ValueType::MoveCategory,
            Self::MoveSlot(_) => ValueType::MoveSlot,
            Self::MoveTarget(_) => ValueType::MoveTarget,
            Self::NaturalGiftData(_) => ValueType::NaturalGiftData,
            Self::Nature(_) => ValueType::Nature,
            Self::SpecialItemData(_) => ValueType::SpecialItemData,
            Self::TempSpecialItemData(_) => ValueType::SpecialItemData,
            Self::Stat(_) => ValueType::Stat,
            Self::StatTable(_) => ValueType::StatTable,
            Self::Type(_) => ValueType::Type,

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

            Self::Battle => Value::Battle,
            Self::Field => Value::Field,
            Self::Format => Value::Format,
            Self::Side(val) => Value::Side(*val),
            Self::Player(val) => Value::Player(*val),
            Self::Mon(val) => Value::Mon(*val),

            Self::Effect(val) => Value::Effect((*val).clone()),
            Self::TempEffect(val) => Value::Effect(val.clone()),
            Self::ActiveMove(val) => Value::ActiveMove(*val),
            Self::HitEffect(val) => Value::HitEffect((*val).clone()),
            Self::SecondaryHitEffect(val) => Value::SecondaryHitEffect((*val).clone()),

            Self::Accuracy(val) => Value::Accuracy(*val),
            Self::Boost(val) => Value::Boost(*val),
            Self::BoostTable(val) => Value::BoostTable((*val).clone()),
            Self::FieldEnvironment(val) => Value::FieldEnvironment(*val),
            Self::FlingData(val) => Value::FlingData((*val).clone()),
            Self::Gender(val) => Value::Gender(*val),
            Self::JudgmentData(val) => Value::JudgmentData((*val).clone()),
            Self::MoveCategory(val) => Value::MoveCategory(*val),
            Self::MoveSlot(val) => Value::MoveSlot((*val).clone()),
            Self::MoveTarget(val) => Value::MoveTarget(*val),
            Self::NaturalGiftData(val) => Value::NaturalGiftData((*val).clone()),
            Self::Nature(val) => Value::Nature(*val),
            Self::SpecialItemData(val) => Value::SpecialItemData((*val).clone()),
            Self::TempSpecialItemData(val) => Value::SpecialItemData(val.clone()),
            Self::Stat(val) => Value::Stat(*val),
            Self::StatTable(val) => Value::StatTable((*val).clone()),
            Self::Type(val) => Value::Type(*val),

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

            Self::Battle => 100,
            Self::Field => 101,
            Self::Format => 102,
            Self::Player(_) => 103,
            Self::Side(_) => 104,
            Self::Mon(_) => 105,

            Self::Effect(_) => 116,
            Self::TempEffect(_) => 117,
            Self::ActiveMove(_) => 118,
            Self::HitEffect(_) => 119,
            Self::SecondaryHitEffect(_) => 120,

            Self::Accuracy(_) => 150,
            Self::Boost(_) => 151,
            Self::BoostTable(_) => 152,
            Self::FieldEnvironment(_) => 153,
            Self::FlingData(_) => 154,
            Self::Gender(_) => 155,
            Self::JudgmentData(_) => 156,
            Self::MoveCategory(_) => 157,
            Self::MoveSlot(_) => 158,
            Self::MoveTarget(_) => 159,
            Self::NaturalGiftData(_) => 160,
            Self::Nature(_) => 161,
            Self::SpecialItemData(_) => 162,
            Self::TempSpecialItemData(_) => 163,
            Self::Stat(_) => 164,
            Self::StatTable(_) => 165,
            Self::Type(_) => 166,

            Self::EffectState(_) => 200,

            Self::List(_) => 225,
            Self::StoredList(_) => 226,
            Self::TempList(_) => 227,
            Self::Object(_) => 228,
            Self::StoredObject(_) => 229,
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

    fn invalid_unary_operation(operation: &str, val: ValueType) -> Error {
        general_error(format!("cannot apply {operation} on value of type {val}"))
    }

    fn invalid_binary_operation(operation: &str, lhs: ValueType, rhs: ValueType) -> Error {
        general_error(format!("cannot {operation} {lhs} and {rhs}"))
    }

    /// Implements negation.
    ///
    /// For boolean coercion, all values coerce to `true` except for:
    /// - `undefined`
    /// - `false`
    /// - `0`
    pub fn negate(self) -> Result<MaybeReferenceValue<'eval>> {
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

    /// Implements the unary plus operator.
    pub fn unary_plus(self) -> Result<MaybeReferenceValue<'eval>> {
        match self {
            Self::Fraction(val) => Ok(MaybeReferenceValue::Fraction(val)),
            Self::UFraction(val) => Ok(MaybeReferenceValue::Fraction(
                val.try_convert().map_err(integer_overflow_error)?,
            )),
            val @ _ => Err(Self::invalid_unary_operation(
                "unary plus",
                val.value_type(),
            )),
        }
    }

    /// Implements exponentiation.
    pub fn pow(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        let result = match (self, rhs) {
            (Self::Fraction(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs.pow(rhs.try_convert::<i32>().map_err(integer_overflow_error)?)
                    .map_err(integer_overflow_error)?,
            ),
            (Self::Fraction(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs.pow(rhs.try_convert::<i32>().map_err(integer_overflow_error)?)
                    .map_err(integer_overflow_error)?,
            ),
            (Self::UFraction(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::UFraction(
                lhs.pow(rhs.try_convert::<i32>().map_err(integer_overflow_error)?)
                    .map_err(integer_overflow_error)?,
            ),
            (Self::UFraction(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::UFraction(
                lhs.pow(rhs.try_convert::<i32>().map_err(integer_overflow_error)?)
                    .map_err(integer_overflow_error)?,
            ),
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "exponentiate",
                    lhs.value_type(),
                    rhs.value_type(),
                ));
            }
        };
        Ok(result)
    }

    /// Implements multiplication.
    pub fn multiply(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        let result = match Self::sort_for_commutative_operation(self, rhs) {
            (Self::Fraction(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(lhs.wrapping_mul(&rhs))
            }
            (Self::Fraction(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs.wrapping_mul(&Fraction::try_convert(rhs).map_err(integer_overflow_error)?),
            ),
            (Self::UFraction(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(lhs.wrapping_mul(&rhs))
            }
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "multiply",
                    lhs.value_type(),
                    rhs.value_type(),
                ));
            }
        };
        Ok(result)
    }

    /// Implements division.
    pub fn divide(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        let result = match (self, rhs) {
            (Self::Fraction(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(lhs / rhs),
            (Self::Fraction(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs / rhs.try_convert().map_err(integer_overflow_error)?,
            ),
            (Self::UFraction(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs.try_convert().map_err(integer_overflow_error)? / rhs,
            ),
            (Self::UFraction(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(lhs / rhs)
            }
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "divide",
                    lhs.value_type(),
                    rhs.value_type(),
                ));
            }
        };
        Ok(result)
    }

    /// Implements modulo.
    pub fn modulo(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        let result = match (self, rhs) {
            (Self::Fraction(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(Fraction::from(lhs.floor() % rhs.floor()))
            }
            (Self::Fraction(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::Fraction(Fraction::from(
                    lhs.floor()
                        % TryInto::<i64>::try_into(rhs.floor()).map_err(integer_overflow_error)?,
                ))
            }
            (Self::UFraction(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(Fraction::from(
                    TryInto::<i64>::try_into(lhs.floor()).map_err(integer_overflow_error)?
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
                ));
            }
        };
        Ok(result)
    }

    /// Implements addition.
    pub fn add(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        let result = match Self::sort_for_commutative_operation(self, rhs) {
            (Self::Fraction(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(lhs.wrapping_add(&rhs))
            }
            (Self::Fraction(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs.wrapping_add(&rhs.try_convert().map_err(integer_overflow_error)?),
            ),
            (Self::UFraction(lhs), Self::UFraction(rhs)) => {
                MaybeReferenceValue::UFraction(lhs.wrapping_add(&rhs))
            }
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "add",
                    lhs.value_type(),
                    rhs.value_type(),
                ));
            }
        };
        Ok(result)
    }

    /// Implements subtraction.
    pub fn subtract(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        let result = match (self, rhs) {
            (Self::Fraction(lhs), Self::Fraction(rhs)) => {
                MaybeReferenceValue::Fraction(lhs.wrapping_sub(&rhs))
            }
            (Self::Fraction(lhs), Self::UFraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs.wrapping_sub(&rhs.try_convert().map_err(integer_overflow_error)?),
            ),
            (Self::UFraction(lhs), Self::Fraction(rhs)) => MaybeReferenceValue::Fraction(
                lhs.try_convert()
                    .map_err(integer_overflow_error)?
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
                ));
            }
        };
        Ok(result)
    }

    fn compare_ref(&'eval self, rhs: &'eval Self) -> Result<Ordering> {
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
                ));
            }
        };
        result.wrap_expectation("comparison yielded no result")
    }

    /// Implements less than comparison.
    pub fn less_than(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        Ok(MaybeReferenceValue::Boolean(
            self.compare_ref(&rhs)?.is_lt(),
        ))
    }

    /// Implements greater than comparison.
    pub fn greater_than(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        Ok(MaybeReferenceValue::Boolean(
            self.compare_ref(&rhs)?.is_gt(),
        ))
    }

    /// Implements less than or equal to comparison.
    pub fn less_than_or_equal(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        Ok(MaybeReferenceValue::Boolean(
            self.compare_ref(&rhs)?.is_le(),
        ))
    }

    /// Implements greater than or equal to comparison.
    pub fn greater_than_or_equal(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        Ok(MaybeReferenceValue::Boolean(
            self.compare_ref(&rhs)?.is_ge(),
        ))
    }

    fn equal_lists<T, U>(lhs: &'eval Vec<T>, rhs: &'eval Vec<U>) -> Result<bool>
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
        lhs: &'eval HashMap<String, T>,
        rhs: &'eval HashMap<String, U>,
    ) -> Result<bool>
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

    fn equal_ref(&'eval self, rhs: &'eval Self) -> Result<bool> {
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
            (Self::String(lhs), Self::Accuracy(rhs)) => {
                Accuracy::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::Boost(rhs)) => {
                Boost::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::FieldEnvironment(rhs)) => {
                FieldEnvironment::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::Gender(rhs)) => {
                Gender::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::MoveCategory(rhs)) => {
                MoveCategory::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::MoveTarget(rhs)) => {
                MoveTarget::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::Nature(rhs)) => {
                Nature::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::Stat(rhs)) => {
                Stat::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::String(lhs), Self::Type(rhs)) => {
                Type::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::Str(rhs)) => lhs.eq(rhs),
            (Self::Str(lhs), Self::TempString(rhs)) => lhs.eq(&rhs),
            (Self::Str(lhs), Self::Effect(rhs)) => {
                rhs.try_id().map(|id| id.as_ref() == *lhs).unwrap_or(false)
            }
            (Self::Str(lhs), Self::Accuracy(rhs)) => {
                Accuracy::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::Boost(rhs)) => Boost::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs)),
            (Self::Str(lhs), Self::FieldEnvironment(rhs)) => {
                FieldEnvironment::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::Gender(rhs)) => {
                Gender::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::MoveCategory(rhs)) => {
                MoveCategory::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::MoveTarget(rhs)) => {
                MoveTarget::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::Nature(rhs)) => {
                Nature::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Str(lhs), Self::Stat(rhs)) => Stat::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs)),
            (Self::Str(lhs), Self::Type(rhs)) => Type::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs)),
            (Self::TempString(lhs), Self::TempString(rhs)) => lhs.eq(rhs),
            (Self::TempString(lhs), Self::Effect(rhs)) => rhs
                .try_id()
                .map(|id| id.as_ref() == lhs.as_str())
                .unwrap_or(false),
            (Self::TempString(lhs), Self::Accuracy(rhs)) => {
                Accuracy::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::Boost(rhs)) => {
                Boost::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::FieldEnvironment(rhs)) => {
                FieldEnvironment::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::Gender(rhs)) => {
                Gender::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::MoveCategory(rhs)) => {
                MoveCategory::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::MoveTarget(rhs)) => {
                MoveTarget::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::Nature(rhs)) => {
                Nature::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::Stat(rhs)) => {
                Stat::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::TempString(lhs), Self::Type(rhs)) => {
                Type::from_str(lhs).is_ok_and(|lhs| lhs.eq(rhs))
            }
            (Self::Battle, Self::Battle) => true,
            (Self::Field, Self::Field) => true,
            (Self::Format, Self::Format) => true,
            (Self::Side(lhs), Self::Side(rhs)) => lhs.eq(rhs),
            (Self::Player(lhs), Self::Player(rhs)) => lhs.eq(rhs),
            (Self::Mon(lhs), Self::Mon(rhs)) => lhs.eq(rhs),
            (Self::Effect(lhs), Self::Effect(rhs)) => lhs.eq(rhs),
            (Self::Effect(lhs), Self::TempEffect(rhs)) => lhs.eq(&rhs),
            (Self::TempEffect(lhs), Self::TempEffect(rhs)) => lhs.eq(rhs),
            (Self::ActiveMove(lhs), Self::ActiveMove(rhs)) => lhs.eq(rhs),
            (Self::HitEffect(lhs), Self::HitEffect(rhs)) => lhs.eq(rhs),
            (Self::Accuracy(lhs), Self::Accuracy(rhs)) => lhs.eq(rhs),
            (Self::Boost(lhs), Self::Boost(rhs)) => lhs.eq(rhs),
            (Self::BoostTable(lhs), Self::BoostTable(rhs)) => lhs.eq(rhs),
            (Self::FieldEnvironment(lhs), Self::FieldEnvironment(rhs)) => lhs.eq(rhs),
            (Self::FlingData(lhs), Self::FlingData(rhs)) => lhs.eq(rhs),
            (Self::Gender(lhs), Self::Gender(rhs)) => lhs.eq(rhs),
            (Self::JudgmentData(lhs), Self::JudgmentData(rhs)) => lhs.eq(rhs),
            (Self::MoveCategory(lhs), Self::MoveCategory(rhs)) => lhs.eq(rhs),
            (Self::MoveSlot(lhs), Self::MoveSlot(rhs)) => lhs.eq(rhs),
            (Self::MoveTarget(lhs), Self::MoveTarget(rhs)) => lhs.eq(rhs),
            (Self::NaturalGiftData(lhs), Self::NaturalGiftData(rhs)) => lhs.eq(rhs),
            (Self::Nature(lhs), Self::Nature(rhs)) => lhs.eq(rhs),
            (Self::SpecialItemData(lhs), Self::SpecialItemData(rhs)) => lhs.eq(rhs),
            (Self::Stat(lhs), Self::Stat(rhs)) => lhs.eq(rhs),
            (Self::StatTable(lhs), Self::StatTable(rhs)) => lhs.eq(rhs),
            (Self::Type(lhs), Self::Type(rhs)) => lhs.eq(rhs),
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
                ));
            }
        };
        Ok(result)
    }

    /// Implements equality.
    pub fn equal(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        Ok(MaybeReferenceValue::Boolean(self.equal_ref(&rhs)?))
    }

    /// Implements inequality.
    pub fn not_equal(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
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
    pub fn has(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        let result = match (self, rhs) {
            (Self::List(lhs), rhs @ _) => Self::list_has_value(lhs, rhs),
            (Self::StoredList(lhs), rhs @ _) => Self::list_has_value(lhs, rhs),
            (Self::TempList(lhs), rhs @ _) => Self::list_has_value(&lhs, rhs),
            _ => {
                return Err(general_error(
                    "left-hand side of has operator must be a list",
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
    pub fn has_any(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
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
                return Err(general_error(
                    "both operands to hasany operator must be a list",
                ));
            }
        };
        Ok(MaybeReferenceValue::Boolean(result))
    }

    /// Implements boolean conjunction.
    pub fn and(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        let result = match (self, rhs) {
            (Self::Undefined, Self::Undefined) => false,
            (Self::Undefined, Self::Boolean(_)) => false,
            (Self::Boolean(_), Self::Undefined) => false,
            (Self::Boolean(lhs), Self::Boolean(rhs)) => lhs && rhs,
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "and",
                    lhs.value_type(),
                    rhs.value_type(),
                ));
            }
        };
        Ok(MaybeReferenceValue::Boolean(result))
    }

    /// Implements boolean disjunction.
    pub fn or(self, rhs: Self) -> Result<MaybeReferenceValue<'eval>> {
        let result = match (self, rhs) {
            (Self::Undefined, Self::Undefined) => false,
            (Self::Undefined, Self::Boolean(rhs)) => rhs,
            (Self::Boolean(lhs), Self::Undefined) => lhs,
            (Self::Boolean(lhs), Self::Boolean(rhs)) => lhs || rhs,
            (lhs @ _, rhs @ _) => {
                return Err(Self::invalid_binary_operation(
                    "or",
                    lhs.value_type(),
                    rhs.value_type(),
                ));
            }
        };
        Ok(MaybeReferenceValue::Boolean(result))
    }

    /// Converts the value to a string for a formatted string.
    pub fn for_formatted_string(&self) -> Result<String> {
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
            Self::Boost(val) => val.to_string(),
            Self::FieldEnvironment(val) => val.to_string(),
            Self::Gender(val) => val.to_string(),
            Self::MoveCategory(val) => val.to_string(),
            Self::MoveTarget(val) => val.to_string(),
            Self::Nature(val) => val.to_string(),
            Self::Type(val) => val.to_string(),
            Self::Stat(val) => val.to_string(),
            _ => {
                return Err(general_error(format!(
                    "{} value is not string formattable",
                    self.value_type(),
                )));
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

            Value::Battle => Self::Battle,
            Value::Field => Self::Field,
            Value::Format => Self::Format,
            Value::Player(val) => Self::Player(*val),
            Value::Side(val) => Self::Side(*val),
            Value::Mon(val) => Self::Mon(*val),

            Value::Effect(val) => Self::Effect(val),
            Value::ActiveMove(val) => Self::ActiveMove(*val),
            Value::HitEffect(val) => Self::HitEffect(val),
            Value::SecondaryHitEffect(val) => Self::SecondaryHitEffect(val),

            Value::Accuracy(val) => Self::Accuracy(*val),
            Value::Boost(val) => Self::Boost(*val),
            Value::BoostTable(val) => Self::BoostTable(val),
            Value::FieldEnvironment(val) => Self::FieldEnvironment(*val),
            Value::FlingData(val) => Self::FlingData(val),
            Value::Gender(val) => Self::Gender(*val),
            Value::JudgmentData(val) => Self::JudgmentData(val),
            Value::MoveCategory(val) => Self::MoveCategory(*val),
            Value::MoveSlot(val) => Self::MoveSlot(val),
            Value::MoveTarget(val) => Self::MoveTarget(*val),
            Value::NaturalGiftData(val) => Self::NaturalGiftData(val),
            Value::Nature(val) => Self::Nature(*val),
            Value::SpecialItemData(val) => Self::SpecialItemData(val),
            Value::Stat(val) => Self::Stat(*val),
            Value::StatTable(val) => Self::StatTable(val),
            Value::Type(val) => Self::Type(*val),

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

            MaybeReferenceValue::Battle => Self::Battle,
            MaybeReferenceValue::Field => Self::Field,
            MaybeReferenceValue::Format => Self::Format,
            MaybeReferenceValue::Side(val) => Self::Side(*val),
            MaybeReferenceValue::Player(val) => Self::Player(*val),
            MaybeReferenceValue::Mon(val) => Self::Mon(*val),

            MaybeReferenceValue::Effect(val) => Self::Effect(val),
            MaybeReferenceValue::ActiveMove(val) => Self::ActiveMove(*val),
            MaybeReferenceValue::HitEffect(val) => Self::HitEffect(val),
            MaybeReferenceValue::SecondaryHitEffect(val) => Self::SecondaryHitEffect(val),

            MaybeReferenceValue::Accuracy(val) => Self::Accuracy(*val),
            MaybeReferenceValue::Boost(val) => Self::Boost(*val),
            MaybeReferenceValue::BoostTable(val) => Self::BoostTable(val),
            MaybeReferenceValue::FieldEnvironment(val) => Self::FieldEnvironment(*val),
            MaybeReferenceValue::FlingData(val) => Self::FlingData(val),
            MaybeReferenceValue::Gender(val) => Self::Gender(*val),
            MaybeReferenceValue::JudgmentData(val) => Self::JudgmentData(val),
            MaybeReferenceValue::MoveCategory(val) => Self::MoveCategory(*val),
            MaybeReferenceValue::MoveSlot(val) => Self::MoveSlot(val),
            MaybeReferenceValue::MoveTarget(val) => Self::MoveTarget(*val),
            MaybeReferenceValue::NaturalGiftData(val) => Self::NaturalGiftData(val),
            MaybeReferenceValue::Nature(val) => Self::Nature(*val),
            MaybeReferenceValue::SpecialItemData(val) => Self::SpecialItemData(val),
            MaybeReferenceValue::Stat(val) => Self::Stat(*val),
            MaybeReferenceValue::StatTable(val) => Self::StatTable(val),
            MaybeReferenceValue::Type(val) => Self::Type(*val),

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

            ValueRef::Battle => Self::Battle,
            ValueRef::Field => Self::Field,
            ValueRef::Format => Self::Format,
            ValueRef::Side(val) => Self::Side(val),
            ValueRef::Player(val) => Self::Player(val),
            ValueRef::Mon(val) => Self::Mon(val),

            ValueRef::Effect(val) => Self::Effect(val),
            ValueRef::TempEffect(val) => Self::TempEffect(val),
            ValueRef::ActiveMove(val) => Self::ActiveMove(val),
            ValueRef::HitEffect(val) => Self::HitEffect(val),
            ValueRef::SecondaryHitEffect(val) => Self::SecondaryHitEffect(val),

            ValueRef::Accuracy(val) => Self::Accuracy(val),
            ValueRef::Boost(val) => Self::Boost(val),
            ValueRef::BoostTable(val) => Self::BoostTable(val),
            ValueRef::FieldEnvironment(val) => Self::FieldEnvironment(val),
            ValueRef::FlingData(val) => Self::FlingData(val),
            ValueRef::Gender(val) => Self::Gender(val),
            ValueRef::JudgmentData(val) => Self::JudgmentData(val),
            ValueRef::MoveCategory(val) => Self::MoveCategory(val),
            ValueRef::MoveSlot(val) => Self::MoveSlot(val),
            ValueRef::MoveTarget(val) => Self::MoveTarget(val),
            ValueRef::NaturalGiftData(val) => Self::NaturalGiftData(val),
            ValueRef::Nature(val) => Self::Nature(val),
            ValueRef::SpecialItemData(val) => Self::SpecialItemData(val),
            ValueRef::Stat(val) => Self::Stat(val),
            ValueRef::StatTable(val) => Self::StatTable(val),
            ValueRef::Type(val) => Self::Type(val),

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
            ValueRef::TempString(val) => Self::String(val),

            ValueRef::Battle => Self::Battle,
            ValueRef::Field => Self::Field,
            ValueRef::Format => Self::Format,
            ValueRef::Side(val) => Self::Side(*val),
            ValueRef::Player(val) => Self::Player(*val),
            ValueRef::Mon(val) => Self::Mon(*val),

            ValueRef::Effect(val) => Self::Effect(val),
            ValueRef::TempEffect(val) => Self::Effect(val),
            ValueRef::ActiveMove(val) => Self::ActiveMove(*val),
            ValueRef::HitEffect(val) => Self::HitEffect(val),
            ValueRef::SecondaryHitEffect(val) => Self::SecondaryHitEffect(val),

            ValueRef::Accuracy(val) => Self::Accuracy(*val),
            ValueRef::Boost(val) => Self::Boost(*val),
            ValueRef::BoostTable(val) => Self::BoostTable(val),
            ValueRef::FieldEnvironment(val) => Self::FieldEnvironment(*val),
            ValueRef::FlingData(val) => Self::FlingData(*val),
            ValueRef::Gender(val) => Self::Gender(*val),
            ValueRef::JudgmentData(val) => Self::JudgmentData(*val),
            ValueRef::MoveCategory(val) => Self::MoveCategory(*val),
            ValueRef::MoveSlot(val) => Self::MoveSlot(val),
            ValueRef::MoveTarget(val) => Self::MoveTarget(*val),
            ValueRef::NaturalGiftData(val) => Self::NaturalGiftData(*val),
            ValueRef::Nature(val) => Self::Nature(*val),
            ValueRef::SpecialItemData(val) => Self::SpecialItemData(*val),
            ValueRef::Stat(val) => Self::Stat(*val),
            ValueRef::StatTable(val) => Self::StatTable(val),
            ValueRef::Type(val) => Self::Type(*val),

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
