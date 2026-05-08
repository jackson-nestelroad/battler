use alloc::{
    borrow::ToOwned,
    format,
    string::String,
    vec::Vec,
};

use anyhow::Result;
use battler_data::{
    BoostTable,
    DefaultTrueBool,
    Fraction,
    MoveTarget,
    SecondaryEffectData,
    StatTable,
    Type,
};
use hashbrown::HashSet;

use crate::{
    battle::{
        ActiveMoveContext,
        ApplyingEffectContext,
        Context,
        CoreBattle,
        EffectContext,
        FieldEffectContext,
        MonContext,
        MonHandle,
        MoveEventResult,
        MoveHandle,
        MoveOutcomeOnTarget,
        PlayerContext,
        PlayerEffectContext,
        SideContext,
        SideEffectContext,
        core_battle_logs,
        mon_states,
    },
    common::MaybeOwnedMut,
    effect::{
        AppliedEffectHandle,
        AppliedEffectLocation,
        EffectHandle,
        EffectManager,
        fxlang::{
            self,
            CallbackFlag,
        },
    },
    general_error,
};

/// Options for running an event.
#[derive(Debug)]
pub struct RunEventOptions {
    /// Forces the first value to be returned, short circuiting the event evaluation.
    ///
    /// Subsequent callbacks are not run at all.
    pub return_first_value: bool,

    /// Signifies that the event should apply to all effects on the field at the end of a turn.
    pub residual: bool,
}

impl Default for RunEventOptions {
    fn default() -> Self {
        Self {
            return_first_value: false,
            residual: false,
        }
    }
}

/// Options for running a single event callback on an effect.
pub struct RunEffectEventOptions {
    /// Overrides the effect that the event callback is triggered on.
    pub effect: Option<AppliedEffectHandle>,
}

impl Default for RunEffectEventOptions {
    fn default() -> Self {
        Self { effect: None }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum AllEffectsTarget {
    Mon(MonHandle),
    Player(usize),
    Side(usize),
    Field,
    Residual,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CallbackHandle {
    applied_effect_handle: AppliedEffectHandle,
    event: fxlang::BattleEvent,
    event_origin_mon_handle: Option<MonHandle>,
    modifier: fxlang::BattleEventModifier,
    suppressed: bool,
}

impl CallbackHandle {
    pub fn new(
        effect_handle: EffectHandle,
        event: fxlang::BattleEvent,
        modifier: fxlang::BattleEventModifier,
        event_origin_mon_handle: Option<MonHandle>,
        location: AppliedEffectLocation,
    ) -> Self {
        Self {
            applied_effect_handle: AppliedEffectHandle::new(effect_handle, location),
            event,
            event_origin_mon_handle,
            modifier,
            suppressed: false,
        }
    }

    /// The speed of the callback.
    pub fn speed(&self, context: &mut Context) -> Result<u32> {
        if let Some(mon_handle) = self.applied_effect_handle.location.mon_handle() {
            return Ok(context.mon(mon_handle)?.volatile_state.speed as u32);
        }
        Ok(0)
    }
}

trait EventInput {
    fn into_fxlang_input(self) -> fxlang::VariableInput;
}

impl EventInput for () {
    fn into_fxlang_input(self) -> fxlang::VariableInput {
        fxlang::VariableInput::default()
    }
}

impl EventInput for fxlang::VariableInput {
    fn into_fxlang_input(self) -> fxlang::VariableInput {
        self
    }
}

impl<T> EventInput for T
where
    T: Into<fxlang::Value>,
{
    fn into_fxlang_input(self) -> fxlang::VariableInput {
        fxlang::VariableInput::from_iter([self.into()])
    }
}

impl<T> EventInput for Option<T>
where
    T: Into<fxlang::Value>,
{
    fn into_fxlang_input(self) -> fxlang::VariableInput {
        fxlang::VariableInput::from_iter([self
            .map(|val| val.into())
            .unwrap_or(fxlang::Value::Undefined)])
    }
}

impl<const N: usize> EventInput for [fxlang::Value; N] {
    fn into_fxlang_input(self) -> fxlang::VariableInput {
        fxlang::VariableInput::from_iter(self)
    }
}

trait EventOutput {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Self;
}

impl EventOutput for () {
    fn from_fxlang_value(_: Option<fxlang::Value>) -> Self {
        ()
    }
}

impl EventOutput for Option<fxlang::Value> {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Self {
        val
    }
}

impl<T> EventOutput for Option<T>
where
    T: OptionalEventOutput,
{
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Self {
        <T as OptionalEventOutput>::from_fxlang_value(val)
    }
}

impl<T> EventOutput for T
where
    T: Default,
    T: OptionalEventOutput,
{
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Self {
        <T as OptionalEventOutput>::from_fxlang_value(val).unwrap_or_default()
    }
}

trait OptionalEventOutput: Sized {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self>;
}

impl OptionalEventOutput for bool {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.boolean().ok()).flatten()
    }
}

impl OptionalEventOutput for DefaultTrueBool {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.boolean().ok())
            .flatten()
            .map(|val| DefaultTrueBool(val))
    }
}

impl OptionalEventOutput for u8 {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.integer_u8().ok()).flatten()
    }
}

impl OptionalEventOutput for u16 {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.integer_u16().ok()).flatten()
    }
}

impl OptionalEventOutput for u32 {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.integer_u32().ok()).flatten()
    }
}

impl OptionalEventOutput for u64 {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.integer_u64().ok()).flatten()
    }
}

impl OptionalEventOutput for i8 {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.integer_i8().ok()).flatten()
    }
}

impl OptionalEventOutput for i32 {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.integer_i32().ok()).flatten()
    }
}

impl OptionalEventOutput for i64 {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.integer_i64().ok()).flatten()
    }
}

impl OptionalEventOutput for Fraction<u32> {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.fraction_u32().ok()).flatten()
    }
}

impl OptionalEventOutput for Fraction<u64> {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.fraction_u64().ok()).flatten()
    }
}

impl OptionalEventOutput for String {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.string().ok()).flatten()
    }
}

impl OptionalEventOutput for BoostTable {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.boost_table().ok()).flatten()
    }
}

impl OptionalEventOutput for MonHandle {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.mon_handle().ok()).flatten()
    }
}

impl OptionalEventOutput for MoveHandle {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.active_move().ok()).flatten()
    }
}

impl OptionalEventOutput for MoveEventResult {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.move_result().ok()).flatten()
    }
}

impl OptionalEventOutput for MoveOutcomeOnTarget {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.move_outcome_on_target().ok()).flatten()
    }
}

impl OptionalEventOutput for MoveTarget {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.move_target().ok()).flatten()
    }
}

impl OptionalEventOutput for StatTable {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.stat_table().ok()).flatten()
    }
}

impl OptionalEventOutput for Vec<SecondaryEffectData> {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.secondary_hit_effects_list().ok())
            .flatten()
    }
}

impl OptionalEventOutput for Vec<String> {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.strings_list().ok()).flatten()
    }
}

impl OptionalEventOutput for Vec<Type> {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.types_list().ok()).flatten()
    }
}

impl OptionalEventOutput for Type {
    fn from_fxlang_value(val: Option<fxlang::Value>) -> Option<Self> {
        val.map(|val| val.mon_type().ok()).flatten()
    }
}

#[warn(private_interfaces)]
pub(crate) trait EventContext<'battle, 'data> {
    fn all_effects_target(&self) -> AllEffectsTarget;
    fn applied_effect_location(&self) -> AppliedEffectLocation;
    fn effect(&self) -> Option<EffectHandle>;
    fn source_effect(&self) -> Option<EffectHandle>;
    fn source(&self) -> Option<MonHandle>;
    fn target(&self) -> Option<MonHandle>;

    fn as_battle_context_mut(&mut self) -> &mut Context<'battle, 'data>;

    fn source_event_context(&mut self) -> Result<Option<impl EventContext<'battle, 'data>>>;

    fn into_upcoming_evaluation_context(&mut self)
    -> UpcomingEvaluationContext<'_, 'battle, 'data>;
}

impl<'battle, 'data> EventContext<'battle, 'data> for ApplyingEffectContext<'_, '_, 'battle, 'data>
where
    'data: 'battle,
{
    fn all_effects_target(&self) -> AllEffectsTarget {
        AllEffectsTarget::Mon(self.target_handle())
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::Mon(self.target_handle())
    }

    fn effect(&self) -> Option<EffectHandle> {
        Some(self.effect_handle().clone())
    }

    fn source_effect(&self) -> Option<EffectHandle> {
        self.source_effect_handle().cloned()
    }

    fn source(&self) -> Option<MonHandle> {
        self.source_handle()
    }

    fn target(&self) -> Option<MonHandle> {
        Some(self.target_handle())
    }

    fn as_battle_context_mut(&mut self) -> &mut Context<'battle, 'data> {
        self.as_battle_context_mut()
    }

    fn source_event_context(&mut self) -> Result<Option<impl EventContext<'battle, 'data>>> {
        self.source_applying_effect_context()
    }

    fn into_upcoming_evaluation_context(
        &mut self,
    ) -> UpcomingEvaluationContext<'_, 'battle, 'data> {
        // SAFETY: UpcomingEvaluationContext uses the lifetime of a mutable borrow of self, so
        // UpcomingEvaluationContext cannot outlive self.
        let context = unsafe {
            core::mem::transmute::<&mut Self, &mut ApplyingEffectContext<'_, '_, 'battle, 'data>>(
                self,
            )
        };
        UpcomingEvaluationContext::ApplyingEffect(context.into())
    }
}

impl<'battle, 'data> EventContext<'battle, 'data> for EffectContext<'_, 'battle, 'data>
where
    'data: 'battle,
{
    fn all_effects_target(&self) -> AllEffectsTarget {
        AllEffectsTarget::Field
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::None
    }

    fn effect(&self) -> Option<EffectHandle> {
        Some(self.effect_handle().clone())
    }

    fn source_effect(&self) -> Option<EffectHandle> {
        self.source_effect_handle().cloned()
    }

    fn source(&self) -> Option<MonHandle> {
        None
    }

    fn target(&self) -> Option<MonHandle> {
        None
    }

    fn as_battle_context_mut(&mut self) -> &mut Context<'battle, 'data> {
        self.as_battle_context_mut()
    }

    fn source_event_context(&mut self) -> Result<Option<impl EventContext<'battle, 'data>>> {
        self.source_effect_context()
    }

    fn into_upcoming_evaluation_context(
        &mut self,
    ) -> UpcomingEvaluationContext<'_, 'battle, 'data> {
        // SAFETY: UpcomingEvaluationContext uses the lifetime of a mutable borrow of self, so
        // UpcomingEvaluationContext cannot outlive self.
        let context = unsafe {
            core::mem::transmute::<&mut Self, &mut EffectContext<'_, 'battle, 'data>>(self)
        };
        UpcomingEvaluationContext::Effect(context.into())
    }
}

impl<'battle, 'data> EventContext<'battle, 'data> for MonContext<'_, '_, '_, 'battle, 'data>
where
    'data: 'battle,
{
    fn all_effects_target(&self) -> AllEffectsTarget {
        AllEffectsTarget::Mon(self.mon_handle())
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::Mon(self.mon_handle())
    }

    fn effect(&self) -> Option<EffectHandle> {
        None
    }

    fn source_effect(&self) -> Option<EffectHandle> {
        None
    }

    fn source(&self) -> Option<MonHandle> {
        None
    }

    fn target(&self) -> Option<MonHandle> {
        Some(self.mon_handle())
    }

    fn as_battle_context_mut(&mut self) -> &mut Context<'battle, 'data> {
        self.as_battle_context_mut()
    }

    fn source_event_context(&mut self) -> Result<Option<impl EventContext<'battle, 'data>>> {
        Ok(Option::<Self>::None)
    }

    fn into_upcoming_evaluation_context(
        &mut self,
    ) -> UpcomingEvaluationContext<'_, 'battle, 'data> {
        // SAFETY: UpcomingEvaluationContext uses the lifetime of a mutable borrow of self, so
        // UpcomingEvaluationContext cannot outlive self.
        let context = unsafe {
            core::mem::transmute::<&mut Self, &mut MonContext<'_, '_, '_, 'battle, 'data>>(self)
        };
        UpcomingEvaluationContext::Mon(context.into())
    }
}

impl<'battle, 'data> EventContext<'battle, 'data> for PlayerEffectContext<'_, '_, 'battle, 'data>
where
    'data: 'battle,
{
    fn all_effects_target(&self) -> AllEffectsTarget {
        AllEffectsTarget::Player(self.player().index)
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::Player(self.player().index)
    }

    fn effect(&self) -> Option<EffectHandle> {
        Some(self.effect_handle().clone())
    }

    fn source_effect(&self) -> Option<EffectHandle> {
        self.source_effect_handle().cloned()
    }

    fn source(&self) -> Option<MonHandle> {
        self.source_handle()
    }

    fn target(&self) -> Option<MonHandle> {
        None
    }

    fn as_battle_context_mut(&mut self) -> &mut Context<'battle, 'data> {
        self.as_battle_context_mut()
    }

    fn source_event_context(&mut self) -> Result<Option<impl EventContext<'battle, 'data>>> {
        self.source_player_effect_context()
    }

    fn into_upcoming_evaluation_context(
        &mut self,
    ) -> UpcomingEvaluationContext<'_, 'battle, 'data> {
        // SAFETY: UpcomingEvaluationContext uses the lifetime of a mutable borrow of self, so
        // UpcomingEvaluationContext cannot outlive self.
        let context = unsafe {
            core::mem::transmute::<&mut Self, &mut PlayerEffectContext<'_, '_, 'battle, 'data>>(
                self,
            )
        };
        UpcomingEvaluationContext::PlayerEffect(context.into())
    }
}

impl<'battle, 'data> EventContext<'battle, 'data> for PlayerContext<'_, '_, 'battle, 'data>
where
    'data: 'battle,
{
    fn all_effects_target(&self) -> AllEffectsTarget {
        AllEffectsTarget::Player(self.player().index)
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::Player(self.player().index)
    }

    fn effect(&self) -> Option<EffectHandle> {
        None
    }

    fn source_effect(&self) -> Option<EffectHandle> {
        None
    }

    fn source(&self) -> Option<MonHandle> {
        None
    }

    fn target(&self) -> Option<MonHandle> {
        None
    }

    fn as_battle_context_mut(&mut self) -> &mut Context<'battle, 'data> {
        self.as_battle_context_mut()
    }

    fn source_event_context(&mut self) -> Result<Option<impl EventContext<'battle, 'data>>> {
        Ok(Option::<Self>::None)
    }

    fn into_upcoming_evaluation_context(
        &mut self,
    ) -> UpcomingEvaluationContext<'_, 'battle, 'data> {
        // SAFETY: UpcomingEvaluationContext uses the lifetime of a mutable borrow of self, so
        // UpcomingEvaluationContext cannot outlive self.
        let context = unsafe {
            core::mem::transmute::<&mut Self, &mut PlayerContext<'_, '_, 'battle, 'data>>(self)
        };
        UpcomingEvaluationContext::Player(context.into())
    }
}

impl<'battle, 'data> EventContext<'battle, 'data> for SideEffectContext<'_, '_, 'battle, 'data>
where
    'data: 'battle,
{
    fn all_effects_target(&self) -> AllEffectsTarget {
        AllEffectsTarget::Side(self.side().index)
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::Side(self.side().index)
    }

    fn effect(&self) -> Option<EffectHandle> {
        Some(self.effect_handle().clone())
    }

    fn source_effect(&self) -> Option<EffectHandle> {
        self.source_effect_handle().cloned()
    }

    fn source(&self) -> Option<MonHandle> {
        self.source_handle()
    }

    fn target(&self) -> Option<MonHandle> {
        None
    }

    fn as_battle_context_mut(&mut self) -> &mut Context<'battle, 'data> {
        self.as_battle_context_mut()
    }

    fn source_event_context(&mut self) -> Result<Option<impl EventContext<'battle, 'data>>> {
        self.source_side_effect_context()
    }

    fn into_upcoming_evaluation_context(
        &mut self,
    ) -> UpcomingEvaluationContext<'_, 'battle, 'data> {
        // SAFETY: UpcomingEvaluationContext uses the lifetime of a mutable borrow of self, so
        // UpcomingEvaluationContext cannot outlive self.
        let context = unsafe {
            core::mem::transmute::<&mut Self, &mut SideEffectContext<'_, '_, 'battle, 'data>>(self)
        };
        UpcomingEvaluationContext::SideEffect(context.into())
    }
}

impl<'battle, 'data> EventContext<'battle, 'data> for FieldEffectContext<'_, '_, 'battle, 'data>
where
    'data: 'battle,
{
    fn all_effects_target(&self) -> AllEffectsTarget {
        AllEffectsTarget::Field
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::Field
    }

    fn effect(&self) -> Option<EffectHandle> {
        Some(self.effect_handle().clone())
    }

    fn source_effect(&self) -> Option<EffectHandle> {
        self.source_effect_handle().cloned()
    }

    fn source(&self) -> Option<MonHandle> {
        self.source_handle()
    }

    fn target(&self) -> Option<MonHandle> {
        None
    }

    fn as_battle_context_mut(&mut self) -> &mut Context<'battle, 'data> {
        self.as_battle_context_mut()
    }

    fn source_event_context(&mut self) -> Result<Option<impl EventContext<'battle, 'data>>> {
        self.source_field_effect_context()
    }

    fn into_upcoming_evaluation_context(
        &mut self,
    ) -> UpcomingEvaluationContext<'_, 'battle, 'data> {
        // SAFETY: UpcomingEvaluationContext uses the lifetime of a mutable borrow of self, so
        // UpcomingEvaluationContext cannot outlive self.
        let context = unsafe {
            core::mem::transmute::<&mut Self, &mut FieldEffectContext<'_, '_, 'battle, 'data>>(self)
        };
        UpcomingEvaluationContext::FieldEffect(context.into())
    }
}
impl<'battle, 'data> EventContext<'battle, 'data> for Context<'battle, 'data>
where
    'data: 'battle,
{
    fn all_effects_target(&self) -> AllEffectsTarget {
        AllEffectsTarget::Field
    }

    fn applied_effect_location(&self) -> AppliedEffectLocation {
        AppliedEffectLocation::Field
    }

    fn effect(&self) -> Option<EffectHandle> {
        None
    }

    fn source_effect(&self) -> Option<EffectHandle> {
        None
    }

    fn source(&self) -> Option<MonHandle> {
        None
    }

    fn target(&self) -> Option<MonHandle> {
        None
    }

    fn as_battle_context_mut(&mut self) -> &mut Context<'battle, 'data> {
        self
    }

    fn source_event_context(&mut self) -> Result<Option<impl EventContext<'battle, 'data>>> {
        Ok(Option::<Self>::None)
    }

    fn into_upcoming_evaluation_context(
        &mut self,
    ) -> UpcomingEvaluationContext<'_, 'battle, 'data> {
        // SAFETY: UpcomingEvaluationContext uses the lifetime of a mutable borrow of self, so
        // UpcomingEvaluationContext cannot outlive self.
        let context =
            unsafe { core::mem::transmute::<&mut Self, &mut Context<'battle, 'data>>(self) };
        UpcomingEvaluationContext::Field(context.into())
    }
}

pub(crate) enum UpcomingEvaluationContext<'context, 'battle, 'data>
where
    'data: 'battle,
    'battle: 'context,
{
    ApplyingEffect(
        MaybeOwnedMut<'context, ApplyingEffectContext<'context, 'context, 'battle, 'data>>,
    ),
    Effect(MaybeOwnedMut<'context, EffectContext<'context, 'battle, 'data>>),
    Mon(MaybeOwnedMut<'context, MonContext<'context, 'context, 'context, 'battle, 'data>>),
    PlayerEffect(MaybeOwnedMut<'context, PlayerEffectContext<'context, 'context, 'battle, 'data>>),
    Player(MaybeOwnedMut<'context, PlayerContext<'context, 'context, 'battle, 'data>>),
    SideEffect(MaybeOwnedMut<'context, SideEffectContext<'context, 'context, 'battle, 'data>>),
    #[allow(unused)]
    Side(MaybeOwnedMut<'context, SideContext<'context, 'battle, 'data>>),
    FieldEffect(MaybeOwnedMut<'context, FieldEffectContext<'context, 'context, 'battle, 'data>>),
    Field(MaybeOwnedMut<'context, Context<'battle, 'data>>),
}

impl<'context, 'battle, 'data> UpcomingEvaluationContext<'context, 'battle, 'data> {
    fn battle_context(&self) -> &Context<'battle, 'data> {
        match self {
            Self::ApplyingEffect(context) => context.as_battle_context(),
            Self::Effect(context) => context.as_battle_context(),
            Self::Mon(context) => context.as_battle_context(),
            Self::Player(context) => context.as_battle_context(),
            Self::PlayerEffect(context) => context.as_battle_context(),
            Self::SideEffect(context) => context.as_battle_context(),
            Self::Side(context) => context.as_battle_context(),
            Self::FieldEffect(context) => context.as_battle_context(),
            Self::Field(context) => context,
        }
    }

    fn battle_context_mut(&mut self) -> &mut Context<'battle, 'data> {
        match self {
            Self::ApplyingEffect(context) => context.as_battle_context_mut(),
            Self::Effect(context) => context.as_battle_context_mut(),
            Self::Mon(context) => context.as_battle_context_mut(),
            Self::Player(context) => context.as_battle_context_mut(),
            Self::PlayerEffect(context) => context.as_battle_context_mut(),
            Self::SideEffect(context) => context.as_battle_context_mut(),
            Self::Side(context) => context.as_battle_context_mut(),
            Self::FieldEffect(context) => context.as_battle_context_mut(),
            Self::Field(context) => context,
        }
    }

    fn target_handle(&self) -> Option<MonHandle> {
        match self {
            Self::ApplyingEffect(context) => Some(context.target_handle()),
            Self::Effect(_) => None,
            Self::Mon(context) => Some(context.mon_handle()),
            Self::PlayerEffect(_) => None,
            Self::Player(_) => None,
            Self::SideEffect(_) => None,
            Self::Side(_) => None,
            Self::FieldEffect(_) => None,
            Self::Field(_) => None,
        }
    }
}

fn run_effect_event_with_errors(
    context: &mut UpcomingEvaluationContext,
    effect_handle: &EffectHandle,
    event: fxlang::BattleEvent,
    modifier: fxlang::BattleEventModifier,
    input: fxlang::VariableInput,
    event_state: &fxlang::EventState,
    effect_state_connector: Option<fxlang::DynamicEffectStateConnector>,
    effect_mon_handle: Option<MonHandle>,
    event_origin_mon_handle: Option<MonHandle>,
    suppressed: bool,
) -> Result<fxlang::ProgramEvalResult> {
    // Effect was suppressed somewhere up the stack, so we should skip the callback.
    //
    // This is important for residual callbacks, which can be suppressed but should still attempt to
    // run in order to properly decrement duration counters.
    if suppressed {
        return Ok(fxlang::ProgramEvalResult::default());
    }

    if !event.state_event() {
        let target = context.target_handle();
        // Mon must be on the field for the callback to run, unless we are targeting that Mon
        // itself.
        if let Some(effect_mon_handle) = &effect_mon_handle
            && target.is_none_or(|target| target != *effect_mon_handle)
        {
            let context = context
                .battle_context_mut()
                .mon_context(*effect_mon_handle)?;
            if !context.mon().active && !context.mon().switch_state.switching_in {
                return Ok(fxlang::ProgramEvalResult::default());
            }
        }

        if let Some(effect_state_connector) = &effect_state_connector {
            // Effect state no longer exists, so we should skip the callback.
            if !effect_state_connector.exists(context.battle_context_mut())? {
                return Ok(fxlang::ProgramEvalResult::default());
            }

            if event.starts_effect() {
                effect_state_connector.set_starting(context.battle_context_mut())?;
            }
            // Ending flag ensures that nested events don't use this callback.
            if event.ends_effect() {
                effect_state_connector.set_ending(context.battle_context_mut())?;
            }
        }
    }

    let mut context = match context {
        UpcomingEvaluationContext::ApplyingEffect(context) => {
            fxlang::EvaluationContext::ApplyingEffect(
                context.forward_applying_effect_context(effect_handle.clone())?,
            )
        }
        UpcomingEvaluationContext::Effect(context) => fxlang::EvaluationContext::Effect(
            context.forward_effect_context(effect_handle.clone())?,
        ),
        UpcomingEvaluationContext::Mon(context) => fxlang::EvaluationContext::ApplyingEffect(
            context.applying_effect_context(effect_handle.clone(), None, None)?,
        ),
        UpcomingEvaluationContext::PlayerEffect(context) => {
            fxlang::EvaluationContext::PlayerEffect(
                context.forward_player_effect_context(effect_handle.clone())?,
            )
        }
        UpcomingEvaluationContext::Player(context) => fxlang::EvaluationContext::PlayerEffect(
            context.player_effect_context(effect_handle.clone(), None, None)?,
        ),
        UpcomingEvaluationContext::SideEffect(context) => fxlang::EvaluationContext::SideEffect(
            context.forward_side_effect_context(effect_handle.clone())?,
        ),
        UpcomingEvaluationContext::Side(context) => fxlang::EvaluationContext::SideEffect(
            context.side_effect_context(effect_handle.clone(), None, None)?,
        ),
        UpcomingEvaluationContext::FieldEffect(context) => fxlang::EvaluationContext::FieldEffect(
            context.forward_field_effect_context(effect_handle.clone())?,
        ),
        UpcomingEvaluationContext::Field(context) => fxlang::EvaluationContext::FieldEffect(
            context.field_effect_context(effect_handle.clone(), None, None)?,
        ),
    };
    let result = EffectManager::evaluate(
        &mut context,
        effect_handle,
        event,
        modifier,
        input,
        event_state,
        effect_state_connector.clone(),
        effect_mon_handle,
        event_origin_mon_handle,
    );

    if let Some(effect_state_connector) = &effect_state_connector {
        if event.starts_effect() {
            effect_state_connector.set_started(context.battle_context_mut())?;
        }
        // Ending flag ensures that nested events don't use this callback.
        if event.ends_effect() {
            effect_state_connector.set_ended(context.battle_context_mut())?;
        }
    }

    result
}

fn run_effect_event_by_handle(
    context: &mut UpcomingEvaluationContext,
    effect: &EffectHandle,
    event: fxlang::BattleEvent,
    modifier: fxlang::BattleEventModifier,
    input: fxlang::VariableInput,
    event_state: &fxlang::EventState,
    effect_state_connector: Option<fxlang::DynamicEffectStateConnector>,
    effect_mon_handle: Option<MonHandle>,
    event_origin_mon_handle: Option<MonHandle>,
    suppressed: bool,
) -> fxlang::ProgramEvalResult {
    match run_effect_event_with_errors(
        context,
        &effect,
        event,
        modifier,
        input,
        event_state,
        effect_state_connector,
        effect_mon_handle,
        event_origin_mon_handle,
        suppressed,
    ) {
        Ok(result) => result,
        Err(error) => {
            let effect_name =
                match CoreBattle::get_effect_by_handle(context.battle_context(), effect) {
                    Ok(effect) => effect.name().to_owned(),
                    Err(_) => format!("{effect:?}"),
                };
            core_battle_logs::debug_event_failure(
                context.battle_context_mut(),
                event,
                &effect_name,
                &&format!("{error:#}"),
            );
            fxlang::ProgramEvalResult::default()
        }
    }
}

fn run_callback_with_errors(
    context: &mut UpcomingEvaluationContext,
    input: fxlang::VariableInput,
    event_state: &fxlang::EventState,
    callback_handle: CallbackHandle,
) -> Result<Option<fxlang::Value>> {
    // Run the event callback for the event.
    let result = run_effect_event_by_handle(
        context,
        &callback_handle.applied_effect_handle.effect_handle,
        callback_handle.event,
        callback_handle.modifier,
        input,
        event_state,
        callback_handle
            .applied_effect_handle
            .effect_state_connector(),
        callback_handle.applied_effect_handle.mon_handle(),
        callback_handle.event_origin_mon_handle,
        callback_handle.suppressed,
    );

    Ok(result.value)
}

fn run_callback(
    mut context: UpcomingEvaluationContext,
    input: fxlang::VariableInput,
    callback_handle: CallbackHandle,
) -> Option<fxlang::Value> {
    let event = callback_handle.event;
    let effect = callback_handle.applied_effect_handle.effect_handle.clone();
    match run_callback_with_errors(
        &mut context,
        input,
        &fxlang::EventState::default(),
        callback_handle,
    ) {
        Ok(value) => value,
        Err(error) => {
            let effect_name =
                match CoreBattle::get_effect_by_handle(context.battle_context(), &effect) {
                    Ok(effect) => effect.name().to_owned(),
                    Err(_) => format!("{effect:?}"),
                };
            core_battle_logs::debug_event_failure(
                context.battle_context_mut(),
                event,
                &effect_name,
                &&format!("{error:#}"),
            );
            None
        }
    }
}

fn run_callbacks_with_forwarding_input_with_errors(
    mut context: UpcomingEvaluationContext,
    input: &mut fxlang::VariableInput,
    event_state: &fxlang::EventState,
    callback_handle: CallbackHandle,
    options: &RunEventOptions,
) -> Result<Option<fxlang::Value>> {
    let event = callback_handle.event;
    let value =
        run_callback_with_errors(&mut context, input.clone(), event_state, callback_handle)?;
    // Support for early exit.
    if let Some(value) = value {
        if value.signals_early_exit() || options.return_first_value {
            return Ok(Some(value));
        }

        let should_not_relay_output = event.has_flag(CallbackFlag::ReturnsBoolean)
            && event
                .input_vars()
                .get(0)
                .is_some_and(|input| input.1 != fxlang::ValueType::Boolean);

        // Pass the output to the next effect.
        //
        // Events that return a boolean likely do not want to do this.
        if !should_not_relay_output {
            if let Some(forward_input) = input.get_mut(0) {
                *forward_input = value;
            } else {
                *input = fxlang::VariableInput::from_iter([value]);
            }
        }
    }

    Ok(None)
}

fn run_callbacks_with_errors<'battle, 'data, Context>(
    context: &mut Context,
    mut input: fxlang::VariableInput,
    options: &RunEventOptions,
    callbacks: Vec<CallbackHandle>,
) -> Result<Option<fxlang::Value>>
where
    'data: 'battle,
    Context: EventContext<'battle, 'data>,
{
    let event_state = fxlang::EventState::default();
    for callback_handle in callbacks {
        if let Some(id) = callback_handle.applied_effect_handle.effect_handle.try_id() {
            if let Some(id) = context
                .as_battle_context_mut()
                .battle_mut()
                .resolve_effect_id(id)
                && !event_state.effect_should_run(id.as_ref())
            {
                continue;
            }
        }

        let result = run_callbacks_with_forwarding_input_with_errors(
            context.into_upcoming_evaluation_context(),
            &mut input,
            &event_state,
            callback_handle,
            options,
        )?;

        if let Some(return_value) = result {
            return Ok(Some(return_value));
        }
    }

    // The first input variable is always returned as the result.
    Ok(input.get(0).cloned())
}

fn run_residual_callbacks_with_errors<'battle, 'data, Context>(
    context: &mut Context,
    callbacks: Vec<CallbackHandle>,
) -> Result<()>
where
    'data: 'battle,
    Context: EventContext<'battle, 'data>,
{
    // Ensure we only decrease the duration of each event once.
    let mut duration_decreased = HashSet::new();

    let event_state = fxlang::EventState::default();
    for callback_handle in callbacks {
        if let Some(id) = callback_handle.applied_effect_handle.effect_handle.try_id() {
            if let Some(id) = context
                .as_battle_context_mut()
                .battle_mut()
                .resolve_effect_id(id)
                && !event_state.effect_should_run(id.as_ref())
            {
                continue;
            }
        }

        if context.as_battle_context_mut().battle().ending() {
            break;
        }

        let mut context = match context.as_battle_context_mut().effect_context(
            callback_handle.applied_effect_handle.effect_handle.clone(),
            None,
        ) {
            Ok(context) => context,
            Err(_) => continue,
        };

        let mut ended = false;
        if duration_decreased.insert((
            callback_handle.applied_effect_handle.effect_handle.clone(),
            callback_handle
                .applied_effect_handle
                .location
                .for_residual(),
        )) {
            if let Some(effect_state_connector) = callback_handle
                .applied_effect_handle
                .effect_state_connector()
            {
                if effect_state_connector.exists(context.as_battle_context_mut())? {
                    let effect_state =
                        effect_state_connector.get_mut(context.as_battle_context_mut())?;
                    if let Some(duration) = effect_state.duration() {
                        let duration = if duration > 0 { duration - 1 } else { duration };
                        effect_state.set_duration(duration);
                        if duration == 0 {
                            ended = true;
                        }
                    }
                }
            }
        }

        if ended {
            if callback_handle.applied_effect_handle.end(&mut context)? {
                continue;
            }
        }

        let mut context = match callback_handle.applied_effect_handle.location {
            AppliedEffectLocation::None => UpcomingEvaluationContext::Effect(context.into()),
            AppliedEffectLocation::ActiveMove(_) => {
                return Err(general_error(
                    "residual callback cannot apply to an active move",
                ));
            }
            AppliedEffectLocation::Mon(mon)
            | AppliedEffectLocation::MonAbility(mon)
            | AppliedEffectLocation::MonInactiveMove(mon)
            | AppliedEffectLocation::MonItem(mon)
            | AppliedEffectLocation::MonPseudoWeather(mon)
            | AppliedEffectLocation::MonSideCondition(_, mon)
            | AppliedEffectLocation::MonSlotCondition(_, _, mon)
            | AppliedEffectLocation::MonStatus(mon)
            | AppliedEffectLocation::MonTerastallization(mon)
            | AppliedEffectLocation::MonTerrain(mon)
            | AppliedEffectLocation::MonType(mon)
            | AppliedEffectLocation::MonVolatile(mon)
            | AppliedEffectLocation::MonWeather(mon) => UpcomingEvaluationContext::ApplyingEffect(
                context.applying_effect_context(None, mon)?.into(),
            ),
            AppliedEffectLocation::Player(player) => UpcomingEvaluationContext::PlayerEffect(
                context.player_effect_context(player, None)?.into(),
            ),
            AppliedEffectLocation::Side(side)
            | AppliedEffectLocation::SideCondition(side)
            | AppliedEffectLocation::SlotCondition(side, _) => {
                UpcomingEvaluationContext::SideEffect(
                    context.side_effect_context(side, None)?.into(),
                )
            }
            AppliedEffectLocation::Field
            | AppliedEffectLocation::PseudoWeather
            | AppliedEffectLocation::Terrain
            | AppliedEffectLocation::Weather => {
                UpcomingEvaluationContext::FieldEffect(context.field_effect_context(None)?.into())
            }
        };
        run_callback_with_errors(
            &mut context,
            fxlang::VariableInput::default(),
            &event_state,
            callback_handle,
        )?;
    }
    Ok(())
}

mod callbacks {
    use alloc::{
        format,
        vec::Vec,
    };

    use anyhow::Result;
    use battler_data::Id;

    use super::{
        AllEffectsTarget,
        CallbackHandle,
    };
    use crate::{
        WrapOptionError,
        battle::{
            Context,
            CoreBattle,
            Field,
            Mon,
            MonHandle,
            SpeedOrderable,
            mon_states,
        },
        effect::{
            AppliedEffectLocation,
            EffectHandle,
            fxlang,
        },
    };

    fn find_callbacks_on_mon(
        context: &mut Context,
        event: fxlang::BattleEvent,
        modifier: fxlang::BattleEventModifier,
        origin: Option<MonHandle>,
        mon: MonHandle,
    ) -> Result<Vec<CallbackHandle>> {
        let mut callbacks = Vec::new();
        let mut context = context.mon_context(mon)?;

        callbacks.push(CallbackHandle::new(
            EffectHandle::Condition(Id::from_known("mon")),
            event,
            modifier,
            origin,
            AppliedEffectLocation::None,
        ));

        if event.callback_lookup_layer() > fxlang::BattleEvent::Types.callback_lookup_layer() {
            let types = mon_states::effective_types(&mut context);
            for typ in types {
                callbacks.push(CallbackHandle::new(
                    EffectHandle::Condition(Id::from(format!("{typ}type"))),
                    event,
                    modifier,
                    origin,
                    AppliedEffectLocation::MonType(mon),
                ));
            }
        }

        if let Some(status) = context.mon().status.clone() {
            let status_effect_handle = context.battle_mut().get_effect_handle_by_id(&status)?;
            callbacks.push(CallbackHandle::new(
                status_effect_handle.clone(),
                event,
                modifier,
                origin,
                AppliedEffectLocation::MonStatus(mon),
            ));
        }
        for volatile in context.mon().volatile_state.volatiles.clone().keys() {
            let volatile_status_handle = context.battle_mut().get_effect_handle_by_id(&volatile)?;
            callbacks.push(CallbackHandle::new(
                volatile_status_handle.clone(),
                event,
                modifier,
                origin,
                AppliedEffectLocation::MonVolatile(mon),
            ));
        }

        if event.callback_lookup_layer()
            > fxlang::BattleEvent::SuppressMonAbility.callback_lookup_layer()
        {
            let ability = context.mon().volatile_state.ability.id.clone();
            let effective_ability = mon_states::effective_ability(&mut context);
            let suppressed = effective_ability.is_none();
            if event.force_default_callback() || !suppressed {
                let ability = effective_ability.unwrap_or(ability);
                let mut callback_handle = CallbackHandle::new(
                    EffectHandle::Ability(ability),
                    event,
                    modifier,
                    origin,
                    AppliedEffectLocation::MonAbility(mon),
                );
                callback_handle.suppressed = suppressed;
                callbacks.push(callback_handle);
            }
        }

        if event.callback_lookup_layer()
            > fxlang::BattleEvent::SuppressMonItem.callback_lookup_layer()
            && let Some(item) = context.mon().item.clone()
        {
            let effective_item = mon_states::effective_item(&mut context);
            let suppressed = effective_item.is_none();
            if event.force_default_callback() || !suppressed {
                let item = effective_item.unwrap_or(item);
                let mut callback_handle = CallbackHandle::new(
                    EffectHandle::Item(item),
                    event,
                    modifier,
                    origin,
                    AppliedEffectLocation::MonItem(mon),
                );
                callback_handle.suppressed = suppressed;
                callbacks.push(callback_handle);
            }
        }

        callbacks.push(CallbackHandle::new(
            EffectHandle::Species(context.mon().volatile_state.species.clone()),
            event,
            modifier,
            origin,
            AppliedEffectLocation::Mon(context.mon_handle()),
        ));

        if let Some(ball) = context.mon().ball.clone() {
            callbacks.push(CallbackHandle::new(
                EffectHandle::ItemCondition(ball),
                event,
                modifier,
                origin,
                AppliedEffectLocation::Mon(context.mon_handle()),
            ));
        }

        if context.mon().different_original_trainer
            && context.mon().level > context.battle().format.rules.numeric_rules.obedience_cap
        {
            callbacks.push(CallbackHandle::new(
                EffectHandle::Condition(Id::from_known("disobedience")),
                event,
                modifier,
                origin,
                AppliedEffectLocation::Mon(context.mon_handle()),
            ));
        }

        if context.player().player_options.has_affection {
            callbacks.push(CallbackHandle::new(
                EffectHandle::Condition(Id::from_known("affection")),
                event,
                modifier,
                origin,
                AppliedEffectLocation::Mon(context.mon_handle()),
            ));
        }

        if context.mon().terastallized.is_some() {
            callbacks.push(CallbackHandle::new(
                EffectHandle::Condition(Id::from_known("terastallization")),
                event,
                modifier,
                origin,
                AppliedEffectLocation::MonTerastallization(mon),
            ));
        }

        Ok(callbacks)
    }

    fn find_callbacks_on_side(
        context: &mut Context,
        event: fxlang::BattleEvent,
        modifier: fxlang::BattleEventModifier,
        origin: Option<MonHandle>,
        side: usize,
    ) -> Result<Vec<CallbackHandle>> {
        let mut callbacks = Vec::new();
        let mut context = context.side_context(side)?;

        for side_condition in context.side().conditions.clone().keys() {
            let side_condition_handle = context
                .battle_mut()
                .get_effect_handle_by_id(&side_condition)?;
            callbacks.push(CallbackHandle::new(
                side_condition_handle.clone(),
                event,
                modifier,
                origin,
                AppliedEffectLocation::SideCondition(side),
            ));
        }

        for (slot, slot_conditions) in context.side().slot_conditions.clone() {
            for slot_condition in slot_conditions.keys() {
                let slot_condition_handle = context
                    .battle_mut()
                    .get_effect_handle_by_id(&slot_condition)?;
                callbacks.push(CallbackHandle::new(
                    slot_condition_handle.clone(),
                    event,
                    modifier,
                    origin,
                    AppliedEffectLocation::SlotCondition(side, slot),
                ));
            }
        }

        Ok(callbacks)
    }

    fn find_callbacks_on_side_on_mon(
        context: &mut Context,
        event: fxlang::BattleEvent,
        modifier: fxlang::BattleEventModifier,
        origin: Option<MonHandle>,
        mon: MonHandle,
    ) -> Result<Vec<CallbackHandle>> {
        let mut callbacks = Vec::new();
        let mut context = context.mon_context(mon)?;
        let side = context.mon().side;

        for side_condition in context.side().conditions.clone().keys() {
            let side_condition_handle = context
                .battle_mut()
                .get_effect_handle_by_id(&side_condition)?;
            callbacks.push(CallbackHandle::new(
                side_condition_handle.clone(),
                event,
                modifier,
                origin,
                AppliedEffectLocation::MonSideCondition(side, mon),
            ));
        }

        if context.mon().active {
            let slot =
                Mon::position_on_side(&context).wrap_expectation("expected target to be active")?;
            if let Some(slot_conditions) = context.side().slot_conditions.get(&slot).cloned() {
                for slot_condition in slot_conditions.keys() {
                    let slot_condition_handle = context
                        .battle_mut()
                        .get_effect_handle_by_id(&slot_condition)?;
                    callbacks.push(CallbackHandle::new(
                        slot_condition_handle.clone(),
                        event,
                        modifier,
                        origin,
                        AppliedEffectLocation::MonSlotCondition(side, slot, mon),
                    ));
                }
            }
        }

        Ok(callbacks)
    }

    fn find_callbacks_on_field(
        context: &mut Context,
        event: fxlang::BattleEvent,
        modifier: fxlang::BattleEventModifier,
        origin: Option<MonHandle>,
    ) -> Result<Vec<CallbackHandle>> {
        let mut callbacks = Vec::new();

        if event.callback_lookup_layer()
            > fxlang::BattleEvent::SuppressFieldWeather.callback_lookup_layer()
        {
            if let Some(weather) = context.battle().field.weather.clone() {
                let effective_weather = Field::effective_weather(context);
                let suppressed = effective_weather.is_none();
                if event.force_default_callback() || !suppressed {
                    let weather_handle = context
                        .battle_mut()
                        .get_effect_handle_by_id(&effective_weather.unwrap_or(weather))?;
                    let mut callback_handle = CallbackHandle::new(
                        weather_handle.clone(),
                        event,
                        modifier,
                        origin,
                        AppliedEffectLocation::Weather,
                    );
                    callback_handle.suppressed = suppressed;
                    callbacks.push(callback_handle);
                }
            }
        }

        if event.callback_lookup_layer()
            > fxlang::BattleEvent::SuppressFieldTerrain.callback_lookup_layer()
        {
            if let Some(terrain) = context.battle().field.terrain.clone() {
                let effective_terrain = Field::effective_terrain(context);
                let suppressed = effective_terrain.is_none();
                if event.force_default_callback() || !suppressed {
                    let terrain_handle = context
                        .battle_mut()
                        .get_effect_handle_by_id(&effective_terrain.unwrap_or(terrain))?;
                    let mut callback_handle = CallbackHandle::new(
                        terrain_handle.clone(),
                        event,
                        modifier,
                        origin,
                        AppliedEffectLocation::Terrain,
                    );
                    callback_handle.suppressed = suppressed;
                    callbacks.push(callback_handle);
                }
            }
        }

        for pseudo_weather in context.battle().field.pseudo_weathers.clone().keys() {
            let pseudo_weather_handle = context
                .battle_mut()
                .get_effect_handle_by_id(&pseudo_weather)?;
            callbacks.push(CallbackHandle::new(
                pseudo_weather_handle.clone(),
                event,
                modifier,
                origin,
                AppliedEffectLocation::PseudoWeather,
            ));
        }

        for rule in context.battle().format.rules.rules() {
            callbacks.push(CallbackHandle::new(
                EffectHandle::Clause(rule.clone()),
                event,
                modifier,
                origin,
                AppliedEffectLocation::None,
            ));
        }

        Ok(callbacks)
    }

    fn find_callbacks_on_field_on_mon(
        context: &mut Context,
        event: fxlang::BattleEvent,
        modifier: fxlang::BattleEventModifier,
        origin: Option<MonHandle>,
        mon: MonHandle,
    ) -> Result<Vec<CallbackHandle>> {
        let mut callbacks = Vec::new();
        let mut context = context.mon_context(mon)?;

        if event.callback_lookup_layer()
            > fxlang::BattleEvent::SuppressMonTerrain.callback_lookup_layer()
        {
            let terrain = context.battle().field.terrain.clone();
            let effective_terrain = mon_states::effective_terrain(&mut context);
            let suppressed = terrain.is_some() && effective_terrain.is_none();
            if (effective_terrain.is_some() && !suppressed)
                || (terrain.is_some() && event.force_default_callback())
            {
                let terrain_handle = context.battle_mut().get_effect_handle_by_id(
                    &effective_terrain
                        .or(terrain)
                        .wrap_expectation("expected terrain")?,
                )?;
                let mut callback_handle = CallbackHandle::new(
                    terrain_handle.clone(),
                    event,
                    modifier,
                    origin,
                    AppliedEffectLocation::MonTerrain(mon),
                );
                callback_handle.suppressed = suppressed;
                callbacks.push(callback_handle);
            }
        }

        if event.callback_lookup_layer()
            > fxlang::BattleEvent::SuppressMonWeather.callback_lookup_layer()
        {
            let weather = context.battle().field.weather.clone();
            let effective_weather = mon_states::effective_weather(&mut context, origin)?;
            let suppressed = weather.is_some() && effective_weather.is_none();
            if (effective_weather.is_some() && !suppressed)
                || (weather.is_some() && event.force_default_callback())
            {
                let weather_handle = context.battle_mut().get_effect_handle_by_id(
                    &effective_weather
                        .or(weather.clone())
                        .wrap_expectation("expected weather")?,
                )?;
                let mut callback_handle = CallbackHandle::new(
                    weather_handle.clone(),
                    event,
                    modifier,
                    origin,
                    if weather.is_some() {
                        AppliedEffectLocation::MonWeather(mon)
                    } else {
                        AppliedEffectLocation::Mon(mon)
                    },
                );
                callback_handle.suppressed = suppressed;
                callbacks.push(callback_handle);
            }
        }

        for pseudo_weather in context.battle().field.pseudo_weathers.clone().keys() {
            let pseudo_weather_handle = context
                .battle_mut()
                .get_effect_handle_by_id(&pseudo_weather)?;
            callbacks.push(CallbackHandle::new(
                pseudo_weather_handle.clone(),
                event,
                modifier,
                origin,
                AppliedEffectLocation::MonPseudoWeather(mon),
            ));
        }

        for rule in context.battle().format.rules.rules() {
            callbacks.push(CallbackHandle::new(
                EffectHandle::Clause(rule.clone()),
                event,
                modifier,
                origin,
                AppliedEffectLocation::None,
            ));
        }

        Ok(callbacks)
    }

    pub fn find_all_callbacks(
        context: &mut Context,
        event: fxlang::BattleEvent,
        target: AllEffectsTarget,
        source: Option<MonHandle>,
        origin: Option<MonHandle>,
    ) -> Result<Vec<CallbackHandle>> {
        let mut callbacks = Vec::new();

        match target {
            AllEffectsTarget::Mon(mon) => {
                callbacks.extend(find_callbacks_on_mon(
                    context,
                    event,
                    fxlang::BattleEventModifier::None,
                    origin,
                    mon,
                )?);
                let mut context = context.mon_context(mon)?;
                for mon in Mon::active_allies_and_self(&mut context).collect::<Vec<_>>() {
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        event,
                        fxlang::BattleEventModifier::Ally,
                        origin,
                        mon,
                    )?);
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        event,
                        fxlang::BattleEventModifier::Any,
                        origin,
                        mon,
                    )?);
                }
                for mon in Mon::active_foes(&mut context).collect::<Vec<_>>() {
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        event,
                        fxlang::BattleEventModifier::Foe,
                        origin,
                        mon,
                    )?);
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        event,
                        fxlang::BattleEventModifier::Any,
                        origin,
                        mon,
                    )?);
                }
                callbacks.extend(find_callbacks_on_side_on_mon(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::None,
                    origin,
                    mon,
                )?);
                let side = context.side().index;
                callbacks.extend(find_callbacks_on_side(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::Side,
                    origin,
                    side,
                )?);
                let foe_side = context.foe_side().index;
                callbacks.extend(find_callbacks_on_side(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::Foe,
                    origin,
                    foe_side,
                )?);

                callbacks.extend(find_callbacks_on_field_on_mon(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::None,
                    origin,
                    mon,
                )?);
            }
            AllEffectsTarget::Player(player) => {
                let mut context = context.player_context(player)?;
                let side = context.side().index;
                callbacks.extend(find_callbacks_on_side(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::None,
                    origin,
                    side,
                )?);
                let foe_side = context.foe_side().index;
                callbacks.extend(find_callbacks_on_side(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::Foe,
                    origin,
                    foe_side,
                )?);

                callbacks.extend(find_callbacks_on_field(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::None,
                    origin,
                )?);

                for mon in context
                    .battle()
                    .active_mon_handles_on_side(side)
                    .collect::<Vec<_>>()
                {
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        event,
                        fxlang::BattleEventModifier::Side,
                        origin,
                        mon,
                    )?);
                }
            }
            AllEffectsTarget::Side(side) => {
                callbacks.extend(find_callbacks_on_side(
                    context,
                    event,
                    fxlang::BattleEventModifier::None,
                    origin,
                    side,
                )?);
                let mut context = context.side_context(side)?;
                let foe_side = context.foe_side().index;
                callbacks.extend(find_callbacks_on_side(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::Foe,
                    origin,
                    foe_side,
                )?);

                callbacks.extend(find_callbacks_on_field(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::None,
                    origin,
                )?);

                for mon in context
                    .battle()
                    .active_mon_handles_on_side(side)
                    .collect::<Vec<_>>()
                {
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        event,
                        fxlang::BattleEventModifier::Side,
                        origin,
                        mon,
                    )?);
                }
            }
            AllEffectsTarget::Field => {
                for mon in context
                    .battle()
                    .all_active_mon_handles()
                    .collect::<Vec<_>>()
                {
                    callbacks.extend(find_callbacks_on_mon(
                        context,
                        event,
                        fxlang::BattleEventModifier::None,
                        origin,
                        mon,
                    )?);
                }
                for side in context.battle().side_indices() {
                    callbacks.extend(find_callbacks_on_side(
                        context,
                        event,
                        fxlang::BattleEventModifier::None,
                        origin,
                        side,
                    )?);
                }
                callbacks.extend(find_callbacks_on_field(
                    context,
                    event,
                    fxlang::BattleEventModifier::None,
                    origin,
                )?);
            }
            AllEffectsTarget::Residual => {
                for mon in context
                    .battle()
                    .all_active_mon_handles()
                    .collect::<Vec<_>>()
                {
                    callbacks.extend(find_callbacks_on_mon(
                        context,
                        event,
                        fxlang::BattleEventModifier::None,
                        origin,
                        mon,
                    )?);
                    callbacks.extend(find_callbacks_on_side_on_mon(
                        context,
                        event,
                        fxlang::BattleEventModifier::None,
                        origin,
                        mon,
                    )?);
                    callbacks.extend(find_callbacks_on_field_on_mon(
                        context,
                        event,
                        fxlang::BattleEventModifier::None,
                        origin,
                        mon,
                    )?);
                }
                for side in context.battle().side_indices() {
                    callbacks.extend(find_callbacks_on_side(
                        context,
                        event
                            .side_event()
                            .wrap_expectation("residual event has no side event")?,
                        fxlang::BattleEventModifier::None,
                        origin,
                        side,
                    )?);
                }
                callbacks.extend(find_callbacks_on_field(
                    context,
                    event
                        .field_event()
                        .wrap_expectation("residual event has no field event")?,
                    fxlang::BattleEventModifier::None,
                    origin,
                )?);
            }
        }

        if let Some(source) = source {
            callbacks.extend(find_callbacks_on_mon(
                context,
                event,
                fxlang::BattleEventModifier::Source,
                origin,
                source,
            )?);
            callbacks.extend(find_callbacks_on_side_on_mon(
                context,
                event,
                fxlang::BattleEventModifier::Source,
                origin,
                source,
            )?);
            callbacks.extend(find_callbacks_on_field_on_mon(
                context,
                event,
                fxlang::BattleEventModifier::Source,
                origin,
                source,
            )?);
        }

        Ok(callbacks)
    }

    struct SpeedOrderableCallbackHandle {
        pub callback_handle: CallbackHandle,
        pub order: u32,
        pub priority: i32,
        pub speed: u32,
        pub sub_order: u32,
        pub effect_order: u32,
    }

    impl SpeedOrderableCallbackHandle {
        pub fn new(callback_handle: CallbackHandle, speed: u32) -> Self {
            Self {
                callback_handle,
                order: u32::MAX,
                priority: 0,
                speed,
                sub_order: 0,
                effect_order: 0,
            }
        }
    }

    impl SpeedOrderable for SpeedOrderableCallbackHandle {
        fn order(&self) -> u32 {
            self.order
        }

        fn priority(&self) -> i32 {
            self.priority
        }

        fn sub_priority(&self) -> i32 {
            0
        }

        fn speed(&self) -> u32 {
            self.speed
        }

        fn sub_order(&self) -> u32 {
            self.sub_order
        }

        fn effect_order(&self) -> u32 {
            self.effect_order
        }
    }

    fn get_speed_orderable_effect_handle_internal(
        context: &mut Context,
        event: fxlang::BattleEvent,
        callback_handle: CallbackHandle,
    ) -> Result<Option<SpeedOrderableCallbackHandle>> {
        // Ensure the effect is not ending.
        let effect_order = if let Some(effect_state) = callback_handle
            .applied_effect_handle
            .effect_state_connector()
            && effect_state.exists(context)?
        {
            let effect_state = effect_state.get_mut(context)?;
            if effect_state.ending() {
                return Ok(None);
            }

            effect_state.effect_order()
        } else {
            0
        };

        let effect_order = if event.order_using_effect_order() {
            effect_order
        } else {
            0
        };

        let speed = callback_handle.speed(context)?;

        // Ensure the event callback exists. An empty callback is ignored.
        let effect = CoreBattle::get_parsed_effect_by_handle(
            context,
            &callback_handle.applied_effect_handle.effect_handle,
        )?;
        let callback = match effect
            .as_ref()
            .map(|effect| effect.event(callback_handle.event, callback_handle.modifier))
            .flatten()
        {
            Some(callback) => callback,
            None => return Ok(None),
        };

        let mut result = SpeedOrderableCallbackHandle::new(callback_handle, speed);
        result.order = callback.order();
        result.priority = callback.priority();
        result.sub_order = callback.sub_order();
        result.effect_order = effect_order;
        Ok(Some(result))
    }

    fn get_speed_orderable_effect_handle(
        context: &mut Context,
        event: fxlang::BattleEvent,
        callback_handle: CallbackHandle,
    ) -> Result<Option<SpeedOrderableCallbackHandle>> {
        match get_speed_orderable_effect_handle_internal(context, event, callback_handle.clone())? {
            Some(handle) => Ok(Some(handle)),
            None => {
                if callback_handle.event.force_default_callback() {
                    Ok(Some(SpeedOrderableCallbackHandle::new(callback_handle, 0)))
                } else {
                    Ok(None)
                }
            }
        }
    }

    pub fn filter_and_order_effects_for_event(
        context: &mut Context,
        event: fxlang::BattleEvent,
        callback_handles: Vec<CallbackHandle>,
    ) -> Result<Vec<CallbackHandle>> {
        let mut speed_orderable_handles = Vec::new();
        speed_orderable_handles.reserve(callback_handles.len());
        for effect_handle in callback_handles {
            match get_speed_orderable_effect_handle(context, event, effect_handle)? {
                Some(handle) => speed_orderable_handles.push(handle),
                None => (),
            }
        }

        CoreBattle::speed_sort(context, speed_orderable_handles.as_mut_slice());
        Ok(speed_orderable_handles
            .into_iter()
            .map(|handle| handle.callback_handle)
            .collect())
    }
}

fn run_event_with_errors<'battle, 'data, Context>(
    context: &mut Context,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    options: &RunEventOptions,
) -> Result<Option<fxlang::Value>>
where
    'data: 'battle,
    Context: EventContext<'battle, 'data>,
{
    let target = if options.residual {
        AllEffectsTarget::Residual
    } else {
        context.all_effects_target()
    };
    let source = context.source();
    let origin = event_origin_mon_handle(event, context.target(), context.source());

    let mut callbacks = callbacks::find_all_callbacks(
        context.as_battle_context_mut(),
        event,
        target,
        source,
        origin,
    )?;
    if event.run_callback_on_source_effect() {
        if let Some(source_effect) = context.effect() {
            callbacks.push(CallbackHandle::new(
                source_effect,
                event,
                fxlang::BattleEventModifier::None,
                origin,
                AppliedEffectLocation::None,
            ));
        }
    }

    let mut callbacks = callbacks::filter_and_order_effects_for_event(
        context.as_battle_context_mut(),
        event,
        callbacks,
    )?;
    callbacks.dedup();

    if options.residual {
        run_residual_callbacks_with_errors(context, callbacks)?;
        Ok(None)
    } else {
        run_callbacks_with_errors(context, input, options, callbacks)
    }
}

fn event_origin_mon_handle(
    event: fxlang::BattleEvent,
    target: Option<MonHandle>,
    source: Option<MonHandle>,
) -> Option<MonHandle> {
    if event.target_is_event_origin() {
        target
    } else {
        source
    }
}

/// Triggers an event, running all event callbacks on impacted battle effects.
#[allow(private_bounds)]
pub fn run_event_with_options<'battle, 'data, Context, Input, Output>(
    context: &mut Context,
    event: fxlang::BattleEvent,
    input: Input,
    options: RunEventOptions,
) -> Output
where
    'data: 'battle,
    Context: EventContext<'battle, 'data>,
    Input: EventInput,
    Output: EventOutput,
{
    let result = match run_event_with_errors(context, event, input.into_fxlang_input(), &options) {
        Ok(value) => value,
        Err(error) => {
            core_battle_logs::debug_full_event_failure(
                context.as_battle_context_mut(),
                event,
                &&format!("{error:#}"),
            );
            None
        }
    };

    Output::from_fxlang_value(result)
}

/// Triggers an event, running all event callbacks on impacted battle effects.
#[allow(private_bounds)]
pub fn run_event_with_input<'battle, 'data, Context, Input, Output>(
    context: &mut Context,
    event: fxlang::BattleEvent,
    input: Input,
) -> Output
where
    'data: 'battle,
    Context: EventContext<'battle, 'data>,
    Input: EventInput,
    Output: EventOutput,
{
    run_event_with_options(context, event, input, RunEventOptions::default())
}

/// Triggers an event, running all event callbacks on impacted battle effects.
///
/// Functionally the same as [`run_event_with_input`], except the input value is used as the default
/// for the output value. This function is largely supplied as a convenience.
#[allow(private_bounds)]
pub fn run_event_with_relay<'battle, 'data, Context, InputOutput>(
    context: &mut Context,
    event: fxlang::BattleEvent,
    input: InputOutput,
) -> InputOutput
where
    'data: 'battle,
    Context: EventContext<'battle, 'data>,
    InputOutput: EventInput + EventOutput + Clone,
    Option<InputOutput>: EventOutput,
{
    run_event_with_options::<Context, InputOutput, Option<InputOutput>>(
        context,
        event,
        input.clone(),
        RunEventOptions::default(),
    )
    .unwrap_or(input)
}

/// Triggers an event, running all event callbacks on impacted battle effects.
#[allow(private_bounds)]
pub fn run_event<'battle, 'data, Context, Output>(
    context: &mut Context,
    event: fxlang::BattleEvent,
) -> Output
where
    'data: 'battle,
    Context: EventContext<'battle, 'data>,
    Output: EventOutput,
{
    run_event_with_options(context, event, (), RunEventOptions::default())
}

/// Runs an event callback for a single effect.
#[allow(private_bounds)]
pub fn run_effect_event_with_options<'battle, 'data, Context, Input, Output>(
    context: &mut Context,
    event: fxlang::BattleEvent,
    input: Input,
    options: RunEffectEventOptions,
) -> Output
where
    'data: 'battle,
    Context: EventContext<'battle, 'data>,
    Input: EventInput,
    Output: EventOutput,
{
    let effect_override = options.effect.is_some();
    let (effect, location) = match options.effect {
        Some(effect) => (Some(effect.effect_handle), effect.location),
        None => (context.effect(), context.applied_effect_location()),
    };

    // If the effect simply does not exist, we do not have any callback to run.
    let effect = match effect {
        Some(effect) => effect,
        None => return EventOutput::from_fxlang_value(None),
    };

    let origin = event_origin_mon_handle(event, context.target(), context.source());

    let callback = CallbackHandle::new(
        effect,
        event,
        fxlang::BattleEventModifier::default(),
        origin,
        location,
    );

    // If running against a specific effect, do not use the source context.
    let source_context = if effect_override {
        Ok(None)
    } else {
        context.source_event_context()
    };

    let output = match source_context {
        Ok(Some(mut context)) => run_callback(
            context.into_upcoming_evaluation_context(),
            input.into_fxlang_input(),
            callback,
        ),
        _ => {
            // The borrow checker does not allow context to be used while source_context exists, so
            // drop it early.
            drop(source_context);
            run_callback(
                context.into_upcoming_evaluation_context(),
                input.into_fxlang_input(),
                callback,
            )
        }
    };

    Output::from_fxlang_value(output)
}

/// Runs an event callback for a single effect.
#[allow(private_bounds)]
pub fn run_effect_event_with_input<'battle, 'data, Context, Input, Output>(
    context: &mut Context,
    event: fxlang::BattleEvent,
    input: Input,
) -> Output
where
    'data: 'battle,
    Context: EventContext<'battle, 'data>,
    Input: EventInput,
    Output: EventOutput,
{
    run_effect_event_with_options(context, event, input, RunEffectEventOptions::default())
}

/// Runs an event callback for a single effect.
#[allow(private_bounds)]
pub fn run_effect_event<'battle, 'data, Context, Output>(
    context: &mut Context,
    event: fxlang::BattleEvent,
) -> Output
where
    'data: 'battle,
    Context: EventContext<'battle, 'data>,
    Output: EventOutput,
{
    run_effect_event_with_input(context, event, ())
}

/// Runs an event on a Mon's effective ability.
#[allow(private_bounds)]
pub fn run_ability_event<'battle, 'data, Context, Input, Output>(
    context: &mut Context,
    event: fxlang::BattleEvent,
    input: Input,
) -> Output
where
    'data: 'battle,
    Context: EventContext<'battle, 'data>,
    Input: EventInput,
    Output: EventOutput,
{
    let ability = if let Some(target) = context.target() {
        context
            .as_battle_context_mut()
            .mon_context(target)
            .ok()
            .and_then(|mut mon_context| mon_states::effective_ability(&mut mon_context))
    } else {
        None
    };

    if let Some(ability) = ability {
        let target_handle = context.target().unwrap();
        run_effect_event_with_options::<Context, Input, Output>(
            context,
            event,
            input,
            RunEffectEventOptions {
                effect: Some(AppliedEffectHandle::new(
                    EffectHandle::Ability(ability),
                    AppliedEffectLocation::MonAbility(target_handle),
                )),
            },
        )
    } else {
        Output::from_fxlang_value(None)
    }
}

/// Runs an event on a Mon's effective item.
#[allow(private_bounds)]
pub fn run_item_event<'battle, 'data, Context, Input, Output>(
    context: &mut Context,
    event: fxlang::BattleEvent,
    input: Input,
) -> Output
where
    'data: 'battle,
    Context: EventContext<'battle, 'data>,
    Input: EventInput,
    Output: EventOutput,
{
    let item = if let Some(target) = context.target() {
        context
            .as_battle_context_mut()
            .mon_context(target)
            .ok()
            .and_then(|mut mon_context| mon_states::effective_item(&mut mon_context))
    } else {
        None
    };

    if let Some(item) = item {
        let target_handle = context.target().unwrap();
        run_effect_event_with_options::<Context, Input, Output>(
            context,
            event,
            input,
            RunEffectEventOptions {
                effect: Some(AppliedEffectHandle::new(
                    EffectHandle::Item(item),
                    AppliedEffectLocation::MonItem(target_handle),
                )),
            },
        )
    } else {
        Output::from_fxlang_value(None)
    }
}

/// The target of a move for effect callbacks that run directly on an active move.
pub enum MoveTargetForEvent {
    /// The effect runs with no target.
    None,
    /// The effect runs with respect to the user of the move.
    ///
    /// This does not mean the target of the move is the user.
    User,
    /// The effect runs with respect to the user of the move, additionally with an optional target
    /// if one is available.
    UserWithTarget(Option<MonHandle>),
    /// The effect runs with respect to a single target of the move.
    Mon(MonHandle),
    /// The effect runs with respect to the target side of the move.
    Side(usize),
    /// The effect runs with respect to the field as a whole.
    Field,
}

fn run_callback_against_active_move_with_errors(
    context: &mut ActiveMoveContext,
    target: MoveTargetForEvent,
    input: fxlang::VariableInput,
    callback_handle: CallbackHandle,
) -> Result<Option<fxlang::Value>> {
    let context = match target {
        MoveTargetForEvent::None => {
            UpcomingEvaluationContext::Effect(context.effect_context()?.into())
        }
        MoveTargetForEvent::User => UpcomingEvaluationContext::ApplyingEffect(
            context.user_applying_effect_context(None)?.into(),
        ),
        MoveTargetForEvent::UserWithTarget(target) => UpcomingEvaluationContext::ApplyingEffect(
            context.user_applying_effect_context(target)?.into(),
        ),
        MoveTargetForEvent::Mon(target) => UpcomingEvaluationContext::ApplyingEffect(
            context.applying_effect_context_for_target(target)?.into(),
        ),
        MoveTargetForEvent::Side(side) => {
            UpcomingEvaluationContext::SideEffect(context.side_effect_context(side)?.into())
        }
        MoveTargetForEvent::Field => {
            UpcomingEvaluationContext::FieldEffect(context.field_effect_context()?.into())
        }
    };

    Ok(run_callback(context, input, callback_handle))
}

fn run_callback_against_active_move(
    context: &mut ActiveMoveContext,
    target: MoveTargetForEvent,
    input: fxlang::VariableInput,
    callback_handle: CallbackHandle,
) -> Option<fxlang::Value> {
    let event = callback_handle.event;
    match run_callback_against_active_move_with_errors(context, target, input, callback_handle) {
        Ok(value) => value,
        Err(error) => {
            let move_name = context.active_move().data.name.clone();
            core_battle_logs::debug_event_failure(
                context.as_battle_context_mut(),
                event,
                &move_name,
                &&format!("{error:#}"),
            );
            None
        }
    }
}

/// Runs an event callback for a single active move.
#[allow(private_bounds)]
pub fn run_active_move_event_with_input<Input, Output>(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    target: MoveTargetForEvent,
    input: Input,
) -> Output
where
    Input: EventInput,
    Output: EventOutput,
{
    let effect = context.effect_handle();
    let origin = Some(context.mon_handle());

    let callback = CallbackHandle::new(
        effect,
        event,
        fxlang::BattleEventModifier::default(),
        origin,
        AppliedEffectLocation::ActiveMove(context.active_move_handle()),
    );

    let output =
        run_callback_against_active_move(context, target, input.into_fxlang_input(), callback);

    Output::from_fxlang_value(output)
}

/// Runs an event callback for a single active move.
#[allow(private_bounds)]
pub fn run_active_move_event<Output>(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    target: MoveTargetForEvent,
) -> Output
where
    Output: EventOutput,
{
    run_active_move_event_with_input(context, event, target, ())
}

/// Runs an event, triggered by an effect, for each active Mon on the field.
pub fn run_event_for_each_active_mon_with_effect(
    context: &mut EffectContext,
    event: fxlang::BattleEvent,
) -> Result<()> {
    for mon_handle in
        CoreBattle::all_active_mon_handles_in_speed_order(context.as_battle_context_mut())?
    {
        run_event::<_, ()>(
            &mut context.applying_effect_context(None, mon_handle)?,
            event,
        );
    }
    Ok(())
}

/// Runs an event for each active Mon on the field.
pub fn run_event_for_each_active_mon(
    context: &mut Context,
    event: fxlang::BattleEvent,
) -> Result<()> {
    for mon_handle in CoreBattle::all_active_mon_handles_in_speed_order(context)? {
        run_event::<_, ()>(&mut context.mon_context(mon_handle)?, event);
    }
    Ok(())
}
