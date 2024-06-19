use std::mem;

use crate::{
    battle::{
        ContextCache,
        CoreBattle,
        Mon,
        MonHandle,
        MoveHandle,
        Player,
        Side,
    },
    battler_error,
    common::{
        Error,
        MaybeOwnedMut,
        UnsafelyDetachBorrowMut,
        WrapResultError,
    },
    effect::{
        Effect,
        EffectHandle,
    },
    moves::{
        HitEffect,
        MonOverride,
        Move,
        MoveHitEffectType,
    },
};

/// The context of a [`CoreBattle`].
///
/// A context is a proxy object for getting references to battle data. For safety, Rust does not
/// allow an object to mutably borrowed multiple times. Rather than storing mutable references for
/// as long as they are needed, references must be grabbed dynamically as they are needed. Context
/// objects make this dynamic borrowing easy and safe to do.
///
/// Contexts are dynamic, in that one context can be used to create other contexts scoped to its
/// lifetime. You can think of contexts as a linked list of references. Rust's borrow checker
/// guarantees that child contexts do not outlive their parents, and a context cannot have two
/// mutable child contexts active at the same time.
///
/// Contexts are hierarchical based on the structure of a battle:
///
/// - [`Context`] - Scoped to a single battle.
/// - [`SideContext`] - Every side is in a battle.
/// - [`PlayerContext`] - Every player is on a side.
/// - [`MonContext`] - Every Mon is owned by a player.
/// - [`ActiveMoveContext`] - Every active move is performed by a Mon.
/// - [`ActiveTargetContext`] - Every target Mon is associated with an active move.
/// - [`EffectContext`] - Every effect occurs in a battle.
/// - [`ApplyingEffectContext`] - Every applying effect has an associated effect.
pub struct Context<'battle, 'data>
where
    'data: 'battle,
{
    // We store the battle as a pointer so that we can freely dereference it. Its lifetime is 'b.
    //
    // Here are some implementation notes:
    //  1. Constructing a new context requires a mutable borrow of the battle object. This assures
    //     a mutable reference of the original battle object cannot be obtained at the same time.
    //  2. However, the context itself provides a mutable reference to the battle object. This is
    //     problematic because it potentially allows a malicious user of a context to invalidate
    //     the battle object backing the context.
    //  3. While this should not happen, we protect against improper use by the following:
    //      1. All references returned from the context are bound to the lifetime of the context
    //         itself. Thus, references obtained from the contxt cannot be stored beyond the
    //         lifetime of the context.
    //      2. Obtaining a mutable reference from a context requires a mutable borrow on the
    //         context itself. This assures that only one mutable reference is "checked out" of the
    //         context at a time. Thus, if a malicious user overwrites the battle object, there
    //         should be no interior references being used at the same time.
    //
    // Thus, most of our design takes in a mutable context and uses it to obtain mutable
    // references, rather than mutably borrowing self. This allows a method that "belongs" to a
    // child object to also reference its parent object through the context object.
    battle: &'battle mut CoreBattle<'data>,
    // Cache of resources borrowed by the context chain.
    //
    // SAFETY: To create a new context, the entire parent context must be borrowed mutably, which
    // means it cannot be used while the child context exists.
    cache: ContextCache<'battle>,
}

impl<'battle, 'data> Context<'battle, 'data> {
    /// Creates a new [`Context`], which contains a reference to a [`CoreBattle`].
    pub fn new(battle: &'battle mut CoreBattle<'data>) -> Self {
        Self {
            battle,
            cache: ContextCache::new(),
        }
    }

    /// Creates a new [`SideContext`], scoped to the lifetime of this context.
    pub fn side_context<'context>(
        &'context mut self,
        side: usize,
    ) -> Result<SideContext<'context, 'battle, 'data>, Error> {
        SideContext::new(self.into(), side)
    }

    /// Creates a new [`PlayerContext`], scoped to the lifetime of this context.
    pub fn player_context<'context>(
        &'context mut self,
        player: usize,
    ) -> Result<PlayerContext<'context, 'context, 'battle, 'data>, Error> {
        PlayerContext::new(self.into(), player)
    }

    /// Creates a new [`MonContext`], scoped to the lifetime of this context.
    pub fn mon_context<'context>(
        &'context mut self,
        mon_handle: MonHandle,
    ) -> Result<MonContext<'context, 'context, 'context, 'battle, 'data>, Error> {
        MonContext::new(self.into(), mon_handle)
    }

    /// Creates a new [`EffectContext`], scoped to the lifetime of this context.
    pub fn effect_context<'context>(
        &'context mut self,
        effect_handle: EffectHandle,
        source_effect_handle: Option<EffectHandle>,
    ) -> Result<EffectContext<'context, 'battle, 'data>, Error> {
        EffectContext::new(self.into(), effect_handle, source_effect_handle)
    }

    /// Creates a new [`ApplyingEffectContext`], scoped to the lifetime of this context.
    pub fn applying_effect_context<'context>(
        &'context mut self,
        effect_handle: EffectHandle,
        source_handle: Option<MonHandle>,
        target_handle: MonHandle,
        source_effect_handle: Option<EffectHandle>,
    ) -> Result<ApplyingEffectContext<'context, 'context, 'battle, 'data>, Error> {
        ApplyingEffectContext::new(
            self.effect_context(effect_handle, source_effect_handle)?
                .into(),
            source_handle,
            target_handle,
        )
    }

    /// Creates a new [`ActiveMoveContext`], scoped to the lifetime of this context.
    pub fn active_move_context<'context>(
        &'context mut self,
        active_move_handle: MoveHandle,
        hit_effect_type: MoveHitEffectType,
    ) -> Result<ActiveMoveContext<'context, 'context, 'context, 'context, 'battle, 'data>, Error>
    {
        let user = self
            .active_move(active_move_handle)?
            .used_by
            .wrap_error_with_message("active move handle does not have a user")?;
        ActiveMoveContext::new_from_mon_context(self.mon_context(user)?.into(), hit_effect_type)
    }

    /// Returns a reference to the [`CoreBattle`].
    pub fn battle(&self) -> &CoreBattle<'data> {
        self.battle
    }

    /// Returns a mutable reference to the [`CoreBattle`].
    pub fn battle_mut(&mut self) -> &mut CoreBattle<'data> {
        self.battle
    }

    /// Returns a reference to a [`Mon`].
    pub fn mon(&self, mon_handle: MonHandle) -> Result<&Mon, Error> {
        self.cache.mon(self.battle(), mon_handle).map(|mon| &*mon)
    }

    /// Returns a mutable reference to a [`Mon`].
    pub fn mon_mut(&mut self, mon_handle: MonHandle) -> Result<&mut Mon, Error> {
        self.cache.mon(self.battle(), mon_handle)
    }

    /// Returns a reference to an active [`Move`].
    pub fn active_move(&self, active_move_handle: MoveHandle) -> Result<&Move, Error> {
        self.cache
            .active_move(self.battle(), active_move_handle)
            .map(|mov| &*mov)
    }

    /// Returns a mutable reference to an active [`Move`].
    pub fn active_move_mut(&self, active_move_handle: MoveHandle) -> Result<&mut Move, Error> {
        self.cache.active_move(self.battle(), active_move_handle)
    }
}

// Manual `Drop` implementation, so that the borrow checker does not allow `Context` references to
// dangle.
impl<'battle, 'data> Drop for Context<'battle, 'data> {
    fn drop(&mut self) {}
}

/// The context of a [`Side`] in a battle.
///
/// See [`Context`] for more information on how context objects work.
pub struct SideContext<'context, 'battle, 'data>
where
    'data: 'battle,
    'battle: 'context,
{
    context: MaybeOwnedMut<'context, Context<'battle, 'data>>,
    // SAFETY: [`CoreBattle::sides`] cannot be modified for the lifetime of the battle.
    side: *mut Side,
    // SAFETY: [`CoreBattle::sides`] cannot be modified for the lifetime of the battle.
    foe_side: *mut Side,
}

// All transmute calls are safe because the battle object and all references obtained from it live
// longer than the context.
impl<'context, 'battle, 'data> SideContext<'context, 'battle, 'data> {
    /// Creates a new [`SideContext`], which contains a reference to a [`CoreBattle`] and a
    /// [`Side`].
    fn new(
        mut context: MaybeOwnedMut<'context, Context<'battle, 'data>>,
        side: usize,
    ) -> Result<Self, Error> {
        // SAFETY: No side is added or removed for the duration of the battle.
        let foe_side = side ^ 1;
        let side = unsafe { mem::transmute(&mut *context.battle_mut().side_mut(side)?) };
        let foe_side = unsafe { mem::transmute(&mut *context.battle_mut().side_mut(foe_side)?) };
        Ok(Self {
            context: context.into(),
            side,
            foe_side,
        })
    }

    /// Returns a reference to the inner [`Context`].
    pub fn as_battle_context<'side>(&'side self) -> &'side Context<'battle, 'data> {
        &self.context
    }

    /// Returns a mutable reference to the inner [`Context`].
    pub fn as_battle_context_mut<'side>(&'side mut self) -> &'side mut Context<'battle, 'data> {
        &mut self.context
    }

    /// Creates a new [`SideContext`] for the opposite side, scoped to the lifetime of this context.
    pub fn foe_side_context<'side>(
        &'side mut self,
    ) -> Result<SideContext<'side, 'battle, 'data>, Error> {
        let foe_side = self.foe_side().index;
        self.as_battle_context_mut().side_context(foe_side)
    }

    /// Creates a new [`PlayerContext`], scoped to the lifetime of this context.
    pub fn player_context<'side>(
        &'side mut self,
        position: usize,
    ) -> Result<PlayerContext<'side, 'context, 'battle, 'data>, Error> {
        let player = Side::player_position_to_index(self, position)
            .wrap_error_with_format(format_args!("side has no player in position {position}"))?;
        PlayerContext::new_from_side_context(self.into(), player)
    }

    /// Returns a reference to the [`CoreBattle`].
    pub fn battle(&self) -> &CoreBattle<'data> {
        self.context.battle()
    }

    /// Returns a mutable reference to the [`CoreBattle`].
    pub fn battle_mut(&mut self) -> &mut CoreBattle<'data> {
        self.context.battle_mut()
    }

    /// Returns a reference to the player's [`Side`].
    pub fn side(&self) -> &Side {
        unsafe { &*self.side }
    }

    /// Returns a mutable reference to the player's [`Side`].
    pub fn side_mut(&mut self) -> &mut Side {
        unsafe { &mut *self.side }
    }

    /// Returns a reference to the foe [`Side`].
    pub fn foe_side(&self) -> &Side {
        unsafe { &*self.foe_side }
    }

    /// Returns a mutable reference to the foe [`Side`].
    pub fn foe_side_mut(&mut self) -> &mut Side {
        unsafe { &mut *self.foe_side }
    }

    /// Returns a reference to a [`Mon`].
    pub fn mon(&self, mon_handle: MonHandle) -> Result<&Mon, Error> {
        self.context.mon(mon_handle)
    }

    /// Returns a mutable reference to a [`Mon`].
    pub fn mon_mut(&mut self, mon_handle: MonHandle) -> Result<&mut Mon, Error> {
        self.context.mon_mut(mon_handle)
    }
}

/// The context of a [`Player`] in a battle.
///
/// See [`Context`] for more information on how context objects work.
pub struct PlayerContext<'side, 'context, 'battle, 'data>
where
    'data: 'battle,
    'battle: 'context,
    'context: 'side,
{
    context: MaybeOwnedMut<'side, SideContext<'context, 'battle, 'data>>,
    // SAFETY: [`CoreBattle::players`] cannot be modified for the lifetime of the battle.
    player: *mut Player,
}

// All transmute calls are safe because the battle object and all references obtained from it live
// longer than the context.
impl<'side, 'context, 'battle, 'data> PlayerContext<'side, 'context, 'battle, 'data> {
    /// Creates a new [`PlayerContext`], which contains a reference to a [`CoreBattle`] and a
    /// [`Player`].
    fn new(
        mut context: MaybeOwnedMut<'context, Context<'battle, 'data>>,
        player: usize,
    ) -> Result<Self, Error> {
        // SAFETY: Players are not added or removed for the duration of the battle.
        let player: &mut Player =
            unsafe { mem::transmute(&mut *context.battle_mut().player_mut(player)?) };
        let context = SideContext::new(context, player.side)?;
        Ok(Self {
            context: context.into(),
            player,
        })
    }

    fn new_from_side_context(
        mut context: MaybeOwnedMut<'side, SideContext<'context, 'battle, 'data>>,
        player: usize,
    ) -> Result<Self, Error> {
        // SAFETY: Players are not added or removed for the duration of the battle.
        let player = &mut *context.battle_mut().player_mut(player)?;
        let player = unsafe { player.unsafely_detach_borrow_mut() };
        Ok(Self {
            context: context.into(),
            player,
        })
    }

    /// Returns a reference to the inner [`Context`].
    pub fn as_battle_context<'player>(&'player self) -> &'player Context<'battle, 'data> {
        self.context.as_battle_context()
    }

    /// Returns a mutable reference to the inner [`Context`].
    pub fn as_battle_context_mut<'player>(
        &'player mut self,
    ) -> &'player mut Context<'battle, 'data> {
        self.context.as_battle_context_mut()
    }

    /// Returns a reference to the inner [`SideContext`].
    pub fn as_side_context<'player>(
        &'player self,
    ) -> &'player SideContext<'context, 'battle, 'data> {
        &self.context
    }

    /// Returns a mutable reference to the inner [`SideContext`].
    pub fn as_side_context_mut<'player>(
        &'player mut self,
    ) -> &'player mut SideContext<'context, 'battle, 'data> {
        &mut self.context
    }

    /// Returns a new [`SideContext`] for the opposing side.
    pub fn foe_side_context<'player>(
        &'player mut self,
    ) -> Result<SideContext<'player, 'battle, 'data>, Error> {
        let foe_side = self.foe_side().index;
        self.as_battle_context_mut().side_context(foe_side)
    }

    /// Returns the [`SideContext`] for either the same side or the opposing side, depending on the
    /// `same_side` parameter.
    pub fn pick_side_context<'player>(
        &'player mut self,
        same_side: bool,
    ) -> Result<SideContext<'player, 'battle, 'data>, Error> {
        if same_side {
            let side = self.side().index;
            self.as_battle_context_mut().side_context(side)
        } else {
            Ok(self.foe_side_context()?.into())
        }
    }

    /// Creates a new [`MonContext`], scoped to the lifetime of this context.
    ///
    /// This method assumes that the Mon identified by `mon_handle` belongs to this player. If this
    /// is not guaranteed, you should use [`Context::mon_context`].
    pub fn mon_context<'player>(
        &'player mut self,
        mon_handle: MonHandle,
    ) -> Result<MonContext<'player, 'side, 'context, 'battle, 'data>, Error> {
        MonContext::new_from_player_context(self, mon_handle)
    }

    /// Returns a reference to the [`CoreBattle`].
    pub fn battle(&self) -> &CoreBattle<'data> {
        self.context.battle()
    }

    /// Returns a mutable reference to the [`CoreBattle`].
    pub fn battle_mut(&mut self) -> &mut CoreBattle<'data> {
        self.context.battle_mut()
    }

    /// Returns a reference to the player's [`Side`].
    pub fn side(&self) -> &Side {
        self.context.side()
    }

    /// Returns a mutable reference to the player's [`Side`].
    pub fn side_mut(&mut self) -> &mut Side {
        self.context.side_mut()
    }

    /// Returns a reference to the foe [`Side`].
    pub fn foe_side(&self) -> &Side {
        self.context.foe_side()
    }

    /// Returns a mutable reference to the foe [`Side`].
    pub fn foe_side_mut(&mut self) -> &mut Side {
        self.context.foe_side_mut()
    }

    /// Returns a reference to the [`Player`].
    pub fn player(&self) -> &Player {
        unsafe { &*self.player }
    }

    /// Returns a mutable reference to the [`Player`].
    pub fn player_mut(&mut self) -> &mut Player {
        unsafe { &mut *self.player }
    }

    /// Returns a reference to a [`Mon`].
    pub fn mon(&self, mon_handle: MonHandle) -> Result<&Mon, Error> {
        self.context.mon(mon_handle)
    }

    /// Returns a mutable reference to a [`Mon`].
    pub fn mon_mut(&mut self, mon_handle: MonHandle) -> Result<&mut Mon, Error> {
        self.context.mon_mut(mon_handle)
    }
}

/// The context of a [`Mon`] in a battle.
///
/// See [`Context`] for more information on how context objects work.
pub struct MonContext<'player, 'side, 'context, 'battle, 'data>
where
    'data: 'battle,
    'battle: 'context,
    'context: 'side,
    'side: 'player,
{
    context: MaybeOwnedMut<'player, PlayerContext<'side, 'context, 'battle, 'data>>,
    mon_handle: MonHandle,
    mon: &'context mut Mon,
}

impl<'player, 'side, 'context, 'battle, 'data>
    MonContext<'player, 'side, 'context, 'battle, 'data>
{
    /// Creates a new [`MonContext`], which contains a reference to a [`CoreBattle`] and a
    /// [`Mon`].
    fn new(
        context: MaybeOwnedMut<'context, Context<'battle, 'data>>,
        mon_handle: MonHandle,
    ) -> Result<Self, Error> {
        let player = context.cache.mon(context.battle(), mon_handle)?.player;
        let context = PlayerContext::new(context, player)?;
        let mon = context
            .as_battle_context()
            .cache
            .mon(context.battle(), mon_handle)?;
        // SAFETY: Mons live as long as the battle itself, since they are stored in a registry. The
        // reference can be borrowed as long as the element reference exists in the root context. We
        // ensure that element references are borrowed for the lifetime of the root context.
        let mon = unsafe { mon.unsafely_detach_borrow_mut() };
        Ok(Self {
            context: context.into(),
            mon_handle,
            mon,
        })
    }

    fn new_from_player_context(
        player_context: &'player mut PlayerContext<'side, 'context, 'battle, 'data>,
        mon_handle: MonHandle,
    ) -> Result<Self, Error> {
        let mon = player_context
            .as_battle_context()
            .cache
            .mon(player_context.battle(), mon_handle)?;
        // SAFETY: Mons live as long as the battle itself, since they are stored in a registry. The
        // reference can be borrowed as long as the element reference exists in the root context. We
        // ensure that element references are borrowed for the lifetime of the root context.
        let mon = unsafe { mon.unsafely_detach_borrow_mut() };
        Ok(Self {
            context: player_context.into(),
            mon_handle,
            mon,
        })
    }

    fn new_from_mon_ref(
        player_context: PlayerContext<'side, 'context, 'battle, 'data>,
        mon_handle: MonHandle,
        mon: &'context mut Mon,
    ) -> Self {
        Self {
            context: player_context.into(),
            mon_handle,
            mon,
        }
    }

    /// Returns a reference to the inner [`Context`].
    pub fn as_battle_context<'mon>(&'mon self) -> &'mon Context<'battle, 'data> {
        self.context.as_battle_context()
    }

    /// Returns a mutable reference to the inner [`Context`].
    pub fn as_battle_context_mut<'mon>(&'mon mut self) -> &'mon mut Context<'battle, 'data> {
        self.context.as_battle_context_mut()
    }

    /// Returns a reference to the inner [`SideContext`].
    pub fn as_side_context<'mon>(&'mon self) -> &'mon SideContext<'side, 'battle, 'data> {
        self.context.as_side_context()
    }

    /// Returns a mutable reference to the inner [`SideContext`].
    pub fn as_side_context_mut<'mon>(
        &'mon mut self,
    ) -> &'mon mut SideContext<'context, 'battle, 'data> {
        self.context.as_side_context_mut()
    }

    /// Returns a new [`SideContext`] for the opposing side.
    pub fn foe_side_context<'mon>(
        &'mon mut self,
    ) -> Result<SideContext<'mon, 'battle, 'data>, Error> {
        self.context.foe_side_context()
    }

    /// Returns the [`SideContext`] for either the same side or the opposing side, depending on the
    /// `same_side` parameter.
    pub fn pick_side_context<'mon>(
        &'mon mut self,
        same_side: bool,
    ) -> Result<SideContext<'mon, 'battle, 'data>, Error> {
        self.context.pick_side_context(same_side)
    }

    /// Returns a reference to the inner [`PlayerContext`].
    pub fn as_player_context<'mon>(
        &'mon self,
    ) -> &'mon PlayerContext<'side, 'context, 'battle, 'data> {
        &self.context
    }

    /// Returns a mutable reference to the inner [`PlayerContext`].
    pub fn as_player_context_mut<'mon>(
        &'mon mut self,
    ) -> &'mon mut PlayerContext<'side, 'context, 'battle, 'data> {
        &mut self.context
    }

    /// Creates a new [`ActiveMoveContext`], scoped to the lifetime of this context.
    pub fn active_move_context<'mon>(
        &'mon mut self,
    ) -> Result<ActiveMoveContext<'mon, 'player, 'side, 'context, 'battle, 'data>, Error> {
        ActiveMoveContext::new_from_mon_context(self.into(), MoveHitEffectType::PrimaryEffect)
    }

    /// Returns a reference to the [`CoreBattle`].
    pub fn battle(&self) -> &CoreBattle<'data> {
        self.context.battle()
    }

    /// Returns a mutable reference to the [`CoreBattle`].
    pub fn battle_mut(&mut self) -> &mut CoreBattle<'data> {
        self.context.battle_mut()
    }

    /// Returns a reference to the Mon's [`Side`].
    pub fn side(&self) -> &Side {
        self.context.side()
    }

    /// Returns a mutable reference to the Mon's [`Side`].
    pub fn side_mut(&mut self) -> &mut Side {
        self.context.side_mut()
    }

    /// Returns a reference to the foe [`Side`].
    pub fn foe_side(&self) -> &Side {
        self.context.foe_side()
    }

    /// Returns a mutable reference to the foe [`Side`].
    pub fn foe_side_mut(&mut self) -> &mut Side {
        self.context.foe_side_mut()
    }

    /// Returns a reference to the Mon's [`Player`].
    pub fn player(&self) -> &Player {
        self.context.player()
    }

    /// Returns a mutable reference to the Mon's [`Player`].
    pub fn player_mut(&mut self) -> &mut Player {
        self.context.player_mut()
    }

    /// Returns the [`MonHandle`] for this [`Mon`].
    pub fn mon_handle(&self) -> MonHandle {
        self.mon_handle
    }

    /// Returns a reference to the [`Mon`].
    pub fn mon(&self) -> &Mon {
        &*self.mon
    }

    /// Returns a mutable reference to the [`Mon`].
    pub fn mon_mut(&mut self) -> &mut Mon {
        &mut *self.mon
    }

    /// Returns a reference to the active [`Move`], if it exists.
    pub fn active_move(&self) -> Result<&Move, Error> {
        let move_handle = self.mon().active_move.wrap_error_with_format(format_args!(
            "mon {} does not have an active move",
            self.mon_handle()
        ))?;
        let context = self.as_battle_context();
        context
            .cache
            .active_move(context.battle(), move_handle)
            .map(|mov| &*mov)
    }

    /// Returns a mutable reference to the active [`Move`], if it exists.
    pub fn active_move_mut(&mut self) -> Result<&mut Move, Error> {
        let move_handle = self.mon().active_move.wrap_error_with_format(format_args!(
            "mon {} does not have an active move",
            self.mon_handle()
        ))?;
        let context = self.as_battle_context();
        context.cache.active_move(context.battle(), move_handle)
    }
}

/// The context of an active [`Move`] in a battle.
///
/// An active move context also has the concept of an applying [`HitEffect`]. A move can "hit"
/// (e.g., "affect") [`Mon`]s multiple times. For instance:
/// - The primary hit deals damage.
/// - The primary [`HitEffect`] lowers stats of the targets and user.
/// - One or more secondary [`HitEffect`]s affect the targets and users.
///
/// The first [`ActiveMoveContext`] is always for the primary [`HitEffect`]. Child contexts can be
/// created for secondary hit effects on the targets or user. This allows battle logic to
/// distinguish between the primary hit of a move and secondary effects of a move.
///
/// See [`Context`] for more information on how context objects work.
pub struct ActiveMoveContext<'mon, 'player, 'side, 'context, 'battle, 'data>
where
    'data: 'battle,
    'battle: 'context,
    'context: 'side,
    'side: 'player,
    'player: 'mon,
{
    context: MaybeOwnedMut<'mon, MonContext<'player, 'side, 'context, 'battle, 'data>>,
    active_move_handle: MoveHandle,
    active_move: &'context mut Move,
    hit_effect_type: MoveHitEffectType,
    is_self: bool,
    is_external: bool,
}

impl<'mon, 'player, 'side, 'context, 'battle, 'data>
    ActiveMoveContext<'mon, 'player, 'side, 'context, 'battle, 'data>
{
    fn new_from_mon_context(
        context: MaybeOwnedMut<'mon, MonContext<'player, 'side, 'context, 'battle, 'data>>,
        hit_effect_type: MoveHitEffectType,
    ) -> Result<Self, Error> {
        let active_move_handle = context
            .mon()
            .active_move
            .wrap_error_with_format(format_args!(
                "mon {} has no active move",
                context.mon_handle()
            ))?;
        let active_move = context
            .as_battle_context()
            .cache
            .active_move(context.battle(), active_move_handle)?;
        // SAFETY: Active moves live as long as the context itself, assuming that the context cannot
        // exist when the battle moves to the next turn.
        let active_move = unsafe { active_move.unsafely_detach_borrow_mut() };
        Ok(Self {
            context,
            active_move_handle,
            active_move,
            hit_effect_type,
            is_self: false,
            is_external: false,
        })
    }

    fn new_from_active_move_context(
        context: &mut Self,
        hit_effect_type: MoveHitEffectType,
        is_self: bool,
        is_external: bool,
    ) -> Self {
        let active_move_handle = context.active_move_handle.clone();
        let active_move = &mut *context.active_move;
        // SAFETY: Active moves live as long as the context itself, assuming that the context cannot
        // exist when the battle moves to the next turn.
        let active_move = unsafe { active_move.unsafely_detach_borrow_mut() };
        let context = context.as_mon_context_mut();
        // SAFETY: We know that the MonContext has lifetime 'mon. We want it to have lifetime
        // 'active_move (lifetime of &mut self). This is safe because we are changing the scope of
        // this reference to a smaller lifetime.
        let context = unsafe { context.unsafely_detach_borrow_mut() };
        let context = context.into();
        Self {
            context,
            active_move_handle,
            active_move,
            hit_effect_type,
            is_self,
            is_external,
        }
    }

    fn new_from_move_handle(
        context: &'context mut Context<'battle, 'data>,
        active_move_handle: MoveHandle,
        hit_effect_type: MoveHitEffectType,
    ) -> Result<Self, Error>
    where
        'side: 'context,
    {
        let active_move = context
            .cache
            .active_move(context.battle(), active_move_handle)?;
        // SAFETY: Active moves live as long as the context itself, assuming that the context cannot
        // exist when the battle moves to the next turn.
        let active_move = unsafe { active_move.unsafely_detach_borrow_mut() };
        let mon_handle = active_move.used_by.wrap_error_with_format(format_args!(
            "active move {active_move_handle} does not have an associated mon"
        ))?;
        let context = context.mon_context(mon_handle)?;
        Ok(Self {
            context: MaybeOwnedMut::Owned(context),
            active_move_handle,
            active_move,
            hit_effect_type,
            is_self: false,
            is_external: false,
        })
    }

    /// Returns a reference to the inner [`Context`].
    pub fn as_battle_context<'active_move>(
        &'active_move self,
    ) -> &'active_move Context<'battle, 'data> {
        self.context.as_battle_context()
    }

    /// Returns a mutable reference to the inner [`Context`].
    pub fn as_battle_context_mut<'active_move>(
        &'active_move mut self,
    ) -> &'active_move mut Context<'battle, 'data> {
        self.context.as_battle_context_mut()
    }

    /// Returns a reference to the inner [`SideContext`].
    pub fn as_side_context<'active_move>(
        &'active_move self,
    ) -> &'active_move SideContext<'side, 'battle, 'data> {
        self.context.as_side_context()
    }

    /// Returns a mutable reference to the inner [`SideContext`].
    pub fn as_side_context_mut<'active_move>(
        &'active_move mut self,
    ) -> &'active_move mut SideContext<'context, 'battle, 'data> {
        self.context.as_side_context_mut()
    }

    /// Returns a new [`SideContext`] for the opposing side.
    pub fn foe_side_context<'active_move>(
        &'active_move mut self,
    ) -> Result<SideContext<'active_move, 'battle, 'data>, Error> {
        self.context.foe_side_context()
    }

    /// Returns a reference to the inner [`PlayerContext`].
    pub fn as_player_context<'active_move>(
        &'active_move self,
    ) -> &'active_move PlayerContext<'side, 'context, 'battle, 'data> {
        self.context.as_player_context()
    }

    /// Returns a mutable reference to the inner [`PlayerContext`].
    pub fn as_player_context_mut<'active_move>(
        &'active_move mut self,
    ) -> &'active_move mut PlayerContext<'side, 'context, 'battle, 'data> {
        self.context.as_player_context_mut()
    }

    /// Returns a reference to the inner [`MonContext`].
    pub fn as_mon_context<'active_move>(
        &'active_move self,
    ) -> &'active_move MonContext<'player, 'side, 'context, 'battle, 'data> {
        &self.context
    }

    /// Returns a mutable reference to the inner [`MonContext`].
    pub fn as_mon_context_mut<'active_move>(
        &'active_move mut self,
    ) -> &'active_move mut MonContext<'player, 'side, 'context, 'battle, 'data> {
        &mut self.context
    }

    /// Creates a new [`MonContext`] for the targeted [`Mon`], scoped to the lifetime of this
    /// context.
    pub fn target_mon_context<'active_move>(
        &'active_move mut self,
        target_mon_handle: MonHandle,
    ) -> Result<MonContext<'active_move, 'active_move, 'active_move, 'battle, 'data>, Error> {
        let mon = self
            .as_battle_context()
            .cache
            .mon(self.battle(), target_mon_handle)?;
        // SAFETY: Mons live as long as the battle itself, since they are stored in a registry. The
        // reference can be borrowed as long as the element reference exists in the root context. We
        // ensure that element references are borrowed for the lifetime of the root context.
        let mon = unsafe { mon.unsafely_detach_borrow_mut() };
        let player_context = self.as_battle_context_mut().player_context(mon.player)?;
        Ok(MonContext::new_from_mon_ref(
            player_context,
            target_mon_handle,
            mon,
        ))
    }

    /// Creates a new [`MonContext`] for the active target [`Mon`], scoped to the lifetime of this
    /// context.
    pub fn active_target_mon_context<'active_move>(
        &'active_move mut self,
    ) -> Result<MonContext<'active_move, 'active_move, 'active_move, 'battle, 'data>, Error> {
        self.target_mon_context(
            self.mon()
                .active_target
                .wrap_error_with_format(format_args!(
                    "active mon {} has no active target",
                    self.mon_handle()
                ))?,
        )
    }

    /// Creates a new [`MonContext`] for the targeted [`Mon`], scoped to the lifetime of this
    /// context.
    pub fn target_context<'active_move>(
        &'active_move mut self,
        target_mon_handle: MonHandle,
    ) -> Result<
        ActiveTargetContext<'active_move, 'mon, 'player, 'side, 'context, 'battle, 'data>,
        Error,
    > {
        ActiveTargetContext::new_from_active_move_context(self.into(), target_mon_handle)
    }

    /// Creates a new [`ActiveTargetContext`] for the active target set on the curretn [`Mon`],
    /// scoped to the lifetime of this context.
    pub fn active_target_context<'active_move>(
        &'active_move mut self,
    ) -> Result<
        ActiveTargetContext<'active_move, 'mon, 'player, 'side, 'context, 'battle, 'data>,
        Error,
    > {
        self.target_context(
            self.mon()
                .active_target
                .wrap_error_with_format(format_args!(
                    "active mon {} has no active target",
                    self.mon_handle()
                ))?,
        )
    }

    /// Creates a new [`ActiveMoveContext`], scoped to the lifetime of this context.
    ///
    /// This method refetches the active move and target.
    pub fn active_move_context(
        self,
    ) -> Result<ActiveMoveContext<'mon, 'player, 'side, 'context, 'battle, 'data>, Error> {
        ActiveMoveContext::new_from_mon_context(self.context, self.hit_effect_type)
    }

    /// Creates a new [`EffectContext`], scoped to the lifetime of this context.
    pub fn effect_context<'active_move>(
        &'active_move mut self,
    ) -> Result<EffectContext<'active_move, 'battle, 'data>, Error> {
        let effect_handle = self.effect_handle();
        let source_effect_handle = self.source_effect_handle().cloned();
        self.as_battle_context_mut()
            .effect_context(effect_handle, source_effect_handle)
    }

    /// Creates a new [`ApplyingEffectContext`], scoped to the lifetime of this context.
    ///
    /// The Mon's active target is used as the target of the move.
    pub fn applying_effect_context<'active_move>(
        &'active_move mut self,
    ) -> Result<ApplyingEffectContext<'active_move, 'active_move, 'battle, 'data>, Error> {
        let source_handle = self.mon_handle();
        let target_handle = self.active_target_mon_context()?.mon_handle();
        ApplyingEffectContext::new(
            self.effect_context()?.into(),
            Some(source_handle),
            target_handle,
        )
    }

    /// Creates a new [`ApplyingEffectContext`] for the target, scoped to the lifetime of this
    /// context.
    pub fn applying_effect_context_for_target<'active_move>(
        &'active_move mut self,
        target_handle: MonHandle,
    ) -> Result<ApplyingEffectContext<'active_move, 'active_move, 'battle, 'data>, Error> {
        let source_handle = self.mon_handle();
        ApplyingEffectContext::new(
            self.effect_context()?.into(),
            Some(source_handle),
            target_handle,
        )
    }

    /// Creates a new [`ApplyingEffectContext`] with the user set as the target, scoped to the
    /// lifetime of this context.
    ///
    /// The Mon's active target is used as the source of the effect, if there is an active target.
    ///
    /// The inverse of [`applying_effect_context`].
    pub fn user_applying_effect_context<'active_move>(
        &'active_move mut self,
    ) -> Result<ApplyingEffectContext<'active_move, 'active_move, 'battle, 'data>, Error> {
        let source_handle = self.active_target_handle();
        let target_handle = self.mon_handle();
        ApplyingEffectContext::new(self.effect_context()?.into(), source_handle, target_handle)
    }

    /// Creates a new [`ActiveMoveContext`] for a secondary [`HitEffect`], scoped to the lifetime of
    /// this context.
    pub fn secondary_active_move_context(&mut self, index: usize) -> Self {
        ActiveMoveContext::new_from_active_move_context(
            self,
            MoveHitEffectType::SecondaryEffect(index),
            self.is_self,
            self.is_external,
        )
    }

    /// Creates a new [`ActiveMoveContext`] for applying [`HitEffect`]s that affect the user of the
    /// move, scoped to the lifetime of this context.
    pub fn hit_self_active_move_context(&mut self) -> Self {
        ActiveMoveContext::new_from_active_move_context(
            self,
            self.hit_effect_type,
            true,
            self.is_external,
        )
    }

    /// Returns a reference to the [`CoreBattle`].
    pub fn battle(&self) -> &CoreBattle<'data> {
        self.context.battle()
    }

    /// Returns a mutable reference to the [`CoreBattle`].
    pub fn battle_mut(&mut self) -> &mut CoreBattle<'data> {
        self.context.battle_mut()
    }

    /// Returns a reference to the Mon's [`Side`].
    pub fn side(&self) -> &Side {
        self.context.side()
    }

    /// Returns a mutable reference to the Mon's [`Side`].
    pub fn side_mut(&mut self) -> &mut Side {
        self.context.side_mut()
    }

    /// Returns a reference to the foe [`Side`].
    pub fn foe_side(&self) -> &Side {
        self.context.foe_side()
    }

    /// Returns a mutable reference to the foe [`Side`].
    pub fn foe_side_mut(&mut self) -> &mut Side {
        self.context.foe_side_mut()
    }

    /// Returns a reference to the Mon's [`Player`].
    pub fn player(&self) -> &Player {
        self.context.player()
    }

    /// Returns a mutable reference to the Mon's [`Player`].
    pub fn player_mut(&mut self) -> &mut Player {
        self.context.player_mut()
    }

    /// Returns the [`MonHandle`] for the active [`Mon`].
    pub fn mon_handle(&self) -> MonHandle {
        self.context.mon_handle()
    }

    /// Returns a reference to the active [`Mon`].
    pub fn mon(&self) -> &Mon {
        self.context.mon()
    }

    /// Returns a mutable reference to the active [`Mon`].
    pub fn mon_mut(&mut self) -> &mut Mon {
        self.context.mon_mut()
    }

    /// Checks if the [`Mon`] has a single active target.
    pub fn has_active_target(&self) -> bool {
        self.mon().active_target.is_some()
    }

    /// Returns the [`MonHandle`] for the active target, if any.
    pub fn active_target_handle(&self) -> Option<MonHandle> {
        self.mon().active_target
    }

    /// Returns the [`EffectHandle`] for the active [`Move`].
    pub fn effect_handle(&self) -> EffectHandle {
        EffectHandle::ActiveMove(self.active_move_handle, self.hit_effect_type)
    }

    /// Returns the source [`EffectHandle`] for the active [`Move`], if any.
    pub fn source_effect_handle(&self) -> Option<&EffectHandle> {
        self.active_move.source_effect.as_ref()
    }

    /// Returns the [`MoveHandle`] for the active [`Move`].
    pub fn active_move_handle(&self) -> MoveHandle {
        self.active_move_handle
    }

    /// Returns a reference to the active [`Move`].
    pub fn active_move(&self) -> &Move {
        &*self.active_move
    }

    /// Returns a mutable reference to the active [`Move`].
    pub fn active_move_mut(&mut self) -> &mut Move {
        &mut *self.active_move
    }

    /// Checks if the context is scoped to the primary effect of the active [`Move`].
    pub fn is_primary(&self) -> bool {
        match self.hit_effect_type {
            MoveHitEffectType::PrimaryEffect => true,
            _ => false,
        }
    }

    /// Checks if the context is scoped to a secondary effect of the active [`Move`].
    pub fn is_secondary(&self) -> bool {
        match self.hit_effect_type {
            MoveHitEffectType::SecondaryEffect(_) => true,
            _ => false,
        }
    }

    /// Checks if the [`HitEffect`] is applying to the user of the move, as opposed to its targets.
    ///
    /// Returns `false` when a target effect of a move is being applied to the user of the move
    /// because the Mon targeted itself.
    pub fn is_self(&self) -> bool {
        self.is_self
    }

    /// Checks if the [`Move`] orginated from an external source (i.e., the [`Mon`] did not
    /// explicitly select it).
    pub fn is_external(&self) -> bool {
        self.is_external
    }

    /// Returns the applying [`MoveHitEffectType`], which describes the source of [`hit_effect`].
    pub fn hit_effect_type(&self) -> MoveHitEffectType {
        self.hit_effect_type
    }

    /// Returns a reference to the applying [`HitEffect`].
    pub fn hit_effect(&self) -> Option<&HitEffect> {
        if self.is_self {
            self.active_move.user_hit_effect(self.hit_effect_type)
        } else {
            self.active_move.target_hit_effect(self.hit_effect_type)
        }
    }

    /// Returns a mutable reference to the applying [`HitEffect`].
    pub fn hit_effect_mut(&mut self) -> Option<&mut HitEffect> {
        if self.is_self {
            self.active_move.user_hit_effect_mut(self.hit_effect_type)
        } else {
            self.active_move.target_hit_effect_mut(self.hit_effect_type)
        }
    }
}

/// The context of an active target [`Mon`] of a [`Move`] in a battle.
///
/// See [`Context`] for more information on how context objects work.
pub struct ActiveTargetContext<'active_move, 'mon, 'player, 'side, 'context, 'battle, 'data>
where
    'data: 'battle,
    'battle: 'context,
    'context: 'side,
    'side: 'player,
    'mon: 'active_move,
{
    context: MaybeOwnedMut<
        'active_move,
        ActiveMoveContext<'mon, 'player, 'side, 'context, 'battle, 'data>,
    >,
    active_target_handle: MonHandle,
    active_target: &'context mut Mon,
}

impl<'active_move, 'mon, 'player, 'side, 'context, 'battle, 'data>
    ActiveTargetContext<'active_move, 'mon, 'player, 'side, 'context, 'battle, 'data>
{
    fn new_from_active_move_context(
        context: MaybeOwnedMut<
            'active_move,
            ActiveMoveContext<'mon, 'player, 'side, 'context, 'battle, 'data>,
        >,
        active_target_handle: MonHandle,
    ) -> Result<Self, Error> {
        let active_target = context
            .as_battle_context()
            .cache
            .mon(context.battle(), active_target_handle)?;
        // SAFETY: Mons live as long as the battle itself, since they are stored in a registry. The
        // reference can be borrowed as long as the element reference exists in the root context. We
        // ensure that element references are borrowed for the lifetime of the root context.
        let active_target = unsafe { active_target.unsafely_detach_borrow_mut() };
        Ok(Self {
            context,
            active_target_handle,
            active_target,
        })
    }

    /// Returns a reference to the inner [`Context`].
    pub fn as_battle_context<'active_target>(
        &'active_target self,
    ) -> &'active_target Context<'battle, 'data> {
        self.context.as_battle_context()
    }

    /// Returns a mutable reference to the inner [`Context`].
    pub fn as_battle_context_mut<'active_target>(
        &'active_target mut self,
    ) -> &'active_target mut Context<'battle, 'data> {
        self.context.as_battle_context_mut()
    }

    /// Returns a reference to the inner [`SideContext`].
    pub fn as_side_context<'active_target>(
        &'active_target self,
    ) -> &'active_target SideContext<'side, 'battle, 'data> {
        self.context.as_side_context()
    }

    /// Returns a mutable reference to the inner [`SideContext`].
    pub fn as_side_context_mut<'active_target>(
        &'active_target mut self,
    ) -> &'active_target mut SideContext<'context, 'battle, 'data> {
        self.context.as_side_context_mut()
    }

    /// Returns a new [`SideContext`] for the opposing side.
    pub fn foe_side_context<'active_target>(
        &'active_target mut self,
    ) -> Result<SideContext<'active_target, 'battle, 'data>, Error> {
        self.context.foe_side_context()
    }

    /// Returns a reference to the inner [`PlayerContext`].
    pub fn as_player_context<'active_target>(
        &'active_target self,
    ) -> &'active_target PlayerContext<'side, 'context, 'battle, 'data> {
        self.context.as_player_context()
    }

    /// Returns a mutable reference to the inner [`PlayerContext`].
    pub fn as_player_context_mut<'active_target>(
        &'active_target mut self,
    ) -> &'active_target mut PlayerContext<'side, 'context, 'battle, 'data> {
        self.context.as_player_context_mut()
    }

    /// Returns a reference to the inner [`MonContext`].
    pub fn as_mon_context<'active_target>(
        &'active_target self,
    ) -> &'active_target MonContext<'player, 'side, 'context, 'battle, 'data> {
        self.context.as_mon_context()
    }

    /// Returns a mutable reference to the inner [`MonContext`].
    pub fn as_mon_context_mut<'active_target>(
        &'active_target mut self,
    ) -> &'active_target mut MonContext<'player, 'side, 'context, 'battle, 'data> {
        self.context.as_mon_context_mut()
    }

    /// Returns a reference to the inner [`ActiveMoveContext`].
    pub fn as_active_move_context<'active_target>(
        &'active_target self,
    ) -> &'active_target ActiveMoveContext<'mon, 'player, 'side, 'context, 'battle, 'data> {
        &self.context
    }

    /// Returns a mutable reference to the inner [`ActiveMoveContext`].
    pub fn as_active_move_context_mut<'active_target>(
        &'active_target mut self,
    ) -> &'active_target mut ActiveMoveContext<'mon, 'player, 'side, 'context, 'battle, 'data> {
        &mut self.context
    }

    /// Creates a new [`MonContext`] for the targeted [`Mon`], scoped to the lifetime of this
    /// context.
    pub fn target_mon_context<'active_target>(
        &'active_target mut self,
    ) -> Result<MonContext<'active_target, 'active_target, 'active_target, 'battle, 'data>, Error>
    {
        let target_handle = self.target_mon_handle();
        self.as_battle_context_mut().mon_context(target_handle)
    }

    /// Creates a new [`MonContext`] for the attacker [`Mon`] for stat calculations, scoped to the
    /// lifetime of this context.
    pub fn attacker_context<'active_target>(
        &'active_target mut self,
    ) -> Result<MonContext<'active_target, 'active_target, 'active_target, 'battle, 'data>, Error>
    {
        match self.active_move().data.override_offensive_mon {
            Some(MonOverride::Target) => self.target_mon_context(),
            _ => {
                let mon_handle = self.mon_handle();
                self.as_battle_context_mut().mon_context(mon_handle)
            }
        }
    }

    /// Creates a new [`MonContext`] for the defender [`Mon`] for stat calculations, scoped to the
    /// lifetime of this context.
    pub fn defender_context<'active_target>(
        &'active_target mut self,
    ) -> Result<MonContext<'active_target, 'active_target, 'active_target, 'battle, 'data>, Error>
    {
        match self.active_move().data.override_defensive_mon {
            Some(MonOverride::User) => {
                let mon_handle = self.mon_handle();
                self.as_battle_context_mut().mon_context(mon_handle)
            }
            _ => self.target_mon_context(),
        }
    }

    /// Creates a new [`ApplyingEffectContext`], scoped to the lifetime of this context.
    pub fn applying_effect_context<'active_target>(
        &'active_target mut self,
    ) -> Result<ApplyingEffectContext<'active_target, 'active_target, 'battle, 'data>, Error> {
        let source_handle = self.mon_handle();
        let target_handle = self.target_mon_handle();
        ApplyingEffectContext::new(
            self.as_active_move_context_mut().effect_context()?.into(),
            Some(source_handle),
            target_handle,
        )
    }

    /// Creates a new [`ApplyingEffectContext`] with the user set as the target, scoped to the
    /// lifetime of this context.
    ///
    /// The target is used as the source of the effect.
    ///
    /// The inverse of [`applying_effect_context`].
    pub fn user_applying_effect_context<'active_target>(
        &'active_target mut self,
    ) -> Result<ApplyingEffectContext<'active_target, 'active_target, 'battle, 'data>, Error> {
        let source_handle = self.target_mon_handle();
        let target_handle = self.mon_handle();
        ApplyingEffectContext::new(
            self.as_active_move_context_mut().effect_context()?.into(),
            Some(source_handle),
            target_handle,
        )
    }

    /// Returns a reference to the [`CoreBattle`].
    pub fn battle(&self) -> &CoreBattle<'data> {
        self.context.battle()
    }

    /// Returns a mutable reference to the [`CoreBattle`].
    pub fn battle_mut(&mut self) -> &mut CoreBattle<'data> {
        self.context.battle_mut()
    }

    /// Returns a reference to the Mon's [`Side`].
    pub fn side(&self) -> &Side {
        self.context.side()
    }

    /// Returns a mutable reference to the Mon's [`Side`].
    pub fn side_mut(&mut self) -> &mut Side {
        self.context.side_mut()
    }

    /// Returns a reference to the foe [`Side`].
    pub fn foe_side(&self) -> &Side {
        self.context.foe_side()
    }

    /// Returns a mutable reference to the foe [`Side`].
    pub fn foe_side_mut(&mut self) -> &mut Side {
        self.context.foe_side_mut()
    }

    /// Returns a reference to the Mon's [`Player`].
    pub fn player(&self) -> &Player {
        self.context.player()
    }

    /// Returns a mutable reference to the Mon's [`Player`].
    pub fn player_mut(&mut self) -> &mut Player {
        self.context.player_mut()
    }

    /// Returns the [`MonHandle`] for the active [`Mon`].
    pub fn mon_handle(&self) -> MonHandle {
        self.context.mon_handle()
    }

    /// Returns a reference to the active [`Mon`].
    pub fn mon(&self) -> &Mon {
        self.context.mon()
    }

    /// Returns a mutable reference to the active [`Mon`].
    pub fn mon_mut(&mut self) -> &mut Mon {
        self.context.mon_mut()
    }

    /// Returns the [`MoveHandle`] for the active [`Move`].
    pub fn active_move_handle(&self) -> MoveHandle {
        self.context.active_move_handle()
    }

    /// Returns a reference to the active [`Move`].
    pub fn active_move(&self) -> &Move {
        self.context.active_move()
    }

    /// Returns a mutable reference to the active [`Move`].
    pub fn active_move_mut(&mut self) -> &mut Move {
        self.context.active_move_mut()
    }

    /// Returns a reference to the source [`EffectHandle`], if any.
    pub fn source_effect_handle(&mut self) -> Option<&EffectHandle> {
        self.context.source_effect_handle()
    }

    /// Checks if the context is scoped to the primary effect of the active [`Move`].
    pub fn is_primary(&self) -> bool {
        self.context.is_primary()
    }

    /// Checks if the context is scoped to a secondary effect of the active [`Move`].
    pub fn is_secondary(&self) -> bool {
        self.context.is_secondary()
    }

    /// Checks if the [`HitEffect`] is applying to the user of the move, as opposed to its targets.
    ///
    /// Returns `false` when a target effect of a move is being applied to the user of the move
    /// because the Mon targeted itself.
    pub fn is_self(&self) -> bool {
        self.context.is_self()
    }

    /// Checks if the [`Move`] orginated from an external source (i.e., the [`Mon`] did not
    /// explicitly select it).
    pub fn is_external(&self) -> bool {
        self.context.is_external()
    }

    /// Returns a reference to the applying [`HitEffect`].
    pub fn hit_effect(&self) -> Option<&HitEffect> {
        self.context.hit_effect()
    }

    /// Returns a mutable reference to the applying [`HitEffect`].
    pub fn hit_effect_mut(&mut self) -> Option<&mut HitEffect> {
        self.context.hit_effect_mut()
    }

    /// Returns the [`MonHandle`] for the active target [`Mon`].
    pub fn target_mon_handle(&self) -> MonHandle {
        self.active_target_handle
    }

    /// Returns a reference to the active target [`Mon`].
    pub fn target_mon(&self) -> &Mon {
        &self.active_target
    }

    /// Returns a mutable reference to the active target [`Mon`].
    pub fn target_mon_mut(&mut self) -> &mut Mon {
        &mut self.active_target
    }
}

/// The context of some [`Effect`] in a battle.
///
/// See [`Context`] for more information on how context objects work.
pub struct EffectContext<'context, 'battle, 'data>
where
    'data: 'battle,
    'battle: 'context,
{
    context: MaybeOwnedMut<'context, Context<'battle, 'data>>,
    effect: Effect<'context>,
    effect_handle: EffectHandle,
    source_effect_handle: Option<EffectHandle>,
}

impl<'context, 'battle, 'data> EffectContext<'context, 'battle, 'data> {
    fn new(
        context: MaybeOwnedMut<'context, Context<'battle, 'data>>,
        effect_handle: EffectHandle,
        source_effect_handle: Option<EffectHandle>,
    ) -> Result<Self, Error> {
        let effect = CoreBattle::get_effect_by_handle(context.as_ref(), &effect_handle)?;
        // SAFETY: Effect contains an internal reference that live as long as the battle itself. The
        // context will always live less time than the battle itself.
        //
        // For active moves, they currently live for two turns, since they are stored in a registry
        // that empties after two turns. The reference can be borrowed as long as the element
        // reference exists in the root context. We ensure that element references are
        // borrowed for the lifetime of the root context.
        let effect: Effect = unsafe { mem::transmute(effect) };
        Ok(Self {
            context,
            effect,
            effect_handle,
            source_effect_handle,
        })
    }

    /// Returns a reference to the inner [`Context`].
    pub fn as_battle_context<'effect>(&'effect self) -> &'effect Context<'battle, 'data> {
        &self.context
    }

    /// Returns a mutable reference to the inner [`Context`].
    pub fn as_battle_context_mut<'effect>(
        &'effect mut self,
    ) -> &'effect mut Context<'battle, 'data> {
        &mut self.context
    }

    /// Creates a new [`ApplyingEffectContext`], scoped to the lifetime of this context.
    pub fn applying_effect_context<'effect>(
        &'effect mut self,
        source_handle: Option<MonHandle>,
        target_handle: MonHandle,
    ) -> Result<ApplyingEffectContext<'effect, 'context, 'battle, 'data>, Error> {
        ApplyingEffectContext::new(self.into(), source_handle, target_handle)
    }

    /// Creates a new [`ActiveMoveContext`], scoped to the lifetime of this context.
    ///
    /// Fails if the effect is not an active move.
    pub fn active_move_context<'effect>(
        &'effect mut self,
    ) -> Result<ActiveMoveContext<'effect, 'effect, 'effect, 'effect, 'battle, 'data>, Error> {
        match self.effect_handle {
            EffectHandle::ActiveMove(active_move_handle, hit_effect_type) => {
                ActiveMoveContext::new_from_move_handle(
                    self.context.as_mut(),
                    active_move_handle,
                    hit_effect_type,
                )
            }
            _ => Err(battler_error!(
                "effect context does not contain an active move"
            )),
        }
    }

    /// Returns a reference to the [`CoreBattle`].
    pub fn battle(&self) -> &CoreBattle<'data> {
        self.context.battle()
    }

    /// Returns a mutable reference to the [`CoreBattle`].
    pub fn battle_mut(&mut self) -> &mut CoreBattle<'data> {
        self.context.battle_mut()
    }

    /// Returns the [`EffectHandle`] for the [`Effect`].
    pub fn effect_handle(&self) -> EffectHandle {
        self.effect_handle.clone()
    }

    /// Returns a reference to the [`Effect`].
    pub fn effect(&self) -> &Effect {
        &self.effect
    }

    /// Returns a mutable reference to the [`Effect`].
    pub fn effect_mut(&mut self) -> &mut Effect<'context> {
        &mut self.effect
    }

    /// Returns a reference to the source [`EffectHandle`], if any.
    pub fn source_effect_handle(&self) -> Option<&EffectHandle> {
        self.source_effect_handle.as_ref()
    }
}

/// The context of an applying [`Effect`] in a battle.
///
/// See [`Context`] for more information on how context objects work.
pub struct ApplyingEffectContext<'effect, 'context, 'battle, 'data>
where
    'data: 'battle,
    'battle: 'context,
    'context: 'effect,
{
    context: MaybeOwnedMut<'effect, EffectContext<'context, 'battle, 'data>>,
    source_handle: Option<MonHandle>,
    source: Option<&'context mut Mon>,
    target_handle: MonHandle,
    target: &'context mut Mon,
}

impl<'effect, 'context, 'battle, 'data> ApplyingEffectContext<'effect, 'context, 'battle, 'data> {
    fn new(
        context: MaybeOwnedMut<'effect, EffectContext<'context, 'battle, 'data>>,
        source_handle: Option<MonHandle>,
        target_handle: MonHandle,
    ) -> Result<ApplyingEffectContext<'effect, 'context, 'battle, 'data>, Error> {
        let target = context
            .as_battle_context()
            .cache
            .mon(context.battle(), target_handle)?;
        // SAFETY: Mons live as long as the battle itself, since they are stored in a registry. The
        // reference can be borrowed as long as the element reference exists in the root context. We
        // ensure that element references are borrowed for the lifetime of the root context.
        let target = unsafe { target.unsafely_detach_borrow_mut() };
        let source = match source_handle {
            None => None,
            Some(source_handle) => {
                let source = context
                    .as_battle_context()
                    .cache
                    .mon(context.battle(), source_handle)?;
                // SAFETY: Mons live as long as the battle itself, since they are stored in a
                // registry. The reference can be borrowed as long as the element reference exists
                // in the root context. We ensure that element references are borrowed for the
                // lifetime of the root context.
                let source = unsafe { source.unsafely_detach_borrow_mut() };
                Some(source)
            }
        };
        Ok(Self {
            context,
            source_handle,
            source,
            target_handle,
            target,
        })
    }

    /// Returns a reference to the inner [`Context`].
    pub fn as_battle_context<'applying_effect>(
        &'applying_effect self,
    ) -> &'applying_effect Context<'battle, 'data> {
        self.context.as_battle_context()
    }

    /// Returns a mutable reference to the inner [`Context`].
    pub fn as_battle_context_mut<'applying_effect>(
        &'applying_effect mut self,
    ) -> &'applying_effect mut Context<'battle, 'data> {
        self.context.as_battle_context_mut()
    }

    /// Returns a reference to the inner [`EffectContext`].
    pub fn as_effect_context<'applying_effect>(
        &'applying_effect self,
    ) -> &'applying_effect EffectContext<'context, 'battle, 'data> {
        &self.context
    }

    /// Returns a mutable reference to the inner [`EffectContext`].
    pub fn as_effect_context_mut<'applying_effect>(
        &'applying_effect mut self,
    ) -> &'applying_effect mut EffectContext<'context, 'battle, 'data> {
        &mut self.context
    }

    /// Creates a new [`MonContext`] for the effect source, scoped to the lifetime of this context.
    pub fn source_context<'applying_effect>(
        &'applying_effect mut self,
    ) -> Result<
        Option<MonContext<'applying_effect, 'applying_effect, 'applying_effect, 'battle, 'data>>,
        Error,
    > {
        match self.source_handle {
            None => Ok(None),
            Some(source_handle) => self
                .as_battle_context_mut()
                .mon_context(source_handle)
                .map(|mon_context| Some(mon_context)),
        }
    }

    /// Creates a new [`MonContext`] for the effect target, scoped to the lifetime of this context.
    pub fn target_context<'applying_effect>(
        &'applying_effect mut self,
    ) -> Result<
        MonContext<'applying_effect, 'applying_effect, 'applying_effect, 'battle, 'data>,
        Error,
    > {
        let target_handle = self.target_handle;
        self.as_battle_context_mut().mon_context(target_handle)
    }

    /// Creates a new [`ApplyingEffectContext`] for the same effect but different target, scoped to
    /// the lifetime of this context.
    pub fn change_target_context<'applying_effect>(
        &'applying_effect mut self,
        target_handle: MonHandle,
    ) -> Result<ApplyingEffectContext<'applying_effect, 'context, 'battle, 'data>, Error> {
        let source_handle = self.source_handle;
        self.as_effect_context_mut()
            .applying_effect_context(source_handle, target_handle)
    }

    /// Creates a new [`ApplyingEffectContext`] for the same target and source but different effect,
    /// scoped to the lifetime of this context.
    ///
    /// The current effect becomes the source of the new applying effect.
    pub fn forward_applying_effect_context<'applying_effect>(
        &'applying_effect mut self,
        effect_handle: EffectHandle,
    ) -> Result<ApplyingEffectContext<'applying_effect, 'applying_effect, 'battle, 'data>, Error>
    {
        let source_handle = self.source_handle;
        let target_handle = self.target_handle;
        let source_effect_handle = self.effect_handle();
        self.as_battle_context_mut().applying_effect_context(
            effect_handle,
            source_handle,
            target_handle,
            Some(source_effect_handle),
        )
    }

    /// Creates a new [`ActiveMoveContext`] for the effect if it is an active move, scoped to the
    /// lifetime of this context.
    pub fn active_move_context<'applying_effect>(
        &'applying_effect mut self,
    ) -> Result<
        Option<
            ActiveMoveContext<
                'applying_effect,
                'applying_effect,
                'applying_effect,
                'applying_effect,
                'battle,
                'data,
            >,
        >,
        Error,
    > {
        match self.effect_handle() {
            EffectHandle::ActiveMove(active_move_handle, hit_effect_type) => self
                .as_battle_context_mut()
                .active_move_context(active_move_handle, hit_effect_type)
                .map(|context| Some(context)),
            _ => Ok(None),
        }
    }

    /// Returns a reference to the [`CoreBattle`].
    pub fn battle(&self) -> &CoreBattle<'data> {
        self.context.battle()
    }

    /// Returns a mutable reference to the [`CoreBattle`].
    pub fn battle_mut(&mut self) -> &mut CoreBattle<'data> {
        self.context.battle_mut()
    }

    /// Returns the [`EffectHandle`] for the [`Effect`].
    pub fn effect_handle(&self) -> EffectHandle {
        self.context.effect_handle()
    }

    /// Returns a reference to the [`Effect`].
    pub fn effect(&self) -> &Effect {
        self.context.effect()
    }

    /// Returns a mutable reference to the [`Effect`].
    pub fn effect_mut(&mut self) -> &mut Effect<'context> {
        self.context.effect_mut()
    }

    /// Returns a reference to the source [`EffectHandle`], if it exists.
    pub fn source_effect_handle(&self) -> Option<&EffectHandle> {
        self.context.source_effect_handle()
    }

    /// Returns the [`MonHandle`] for the source [`Mon`], if one exists.
    pub fn source_handle(&self) -> Option<MonHandle> {
        self.source_handle
    }

    /// Checks if the effect has a source [`Mon`].
    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }

    /// Returns a reference to the source [`Mon`], if one exists.
    pub fn source(&self) -> Option<&Mon> {
        self.source.as_deref()
    }

    /// Returns a mutable reference to the source [`Mon`], if one exists.
    pub fn source_mut(&mut self) -> Option<&mut Mon> {
        self.source.as_deref_mut()
    }

    /// Returns the [`MonHandle`] for the target [`Mon`].
    pub fn target_handle(&self) -> MonHandle {
        self.target_handle
    }

    /// Returns a reference to the target [`Mon`].
    pub fn target(&self) -> &Mon {
        &self.target
    }

    /// Returns a mutable reference to the target [`Mon`].
    pub fn target_mut(&mut self) -> &mut Mon {
        &mut self.target
    }
}
