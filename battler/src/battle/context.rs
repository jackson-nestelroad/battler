use std::{
    marker::PhantomData,
    mem,
    ops::{
        Deref,
        DerefMut,
    },
};

use ahash::HashMapExt;
use zone_alloc::{
    ElementRef,
    ElementRefMut,
};

use crate::{
    battle::{
        BattleQueue,
        CoreBattle,
        Mon,
        MonHandle,
        MoveHandle,
        Player,
        Side,
    },
    common::{
        Error,
        FastHashMap,
        MaybeOwnedMut,
        UnsafelyDetachBorrowMut,
        WrapResultError,
    },
    moves::{
        MonOverride,
        Move,
    },
};

/// The context of a [`CoreBattle`].
///
/// A context is a proxy object for getting references to battle data. Rust does not make storing
/// references easy, so references must be grabbed dynamically as needed.
///
/// Contexts are dynamic, in that one context can be used to create other contexts scoped to its
/// lifetime. You can think of contexts as a linked list of references. Rust's borrow checker
/// guarantees that child contexts do not outlive their parents, and a context cannot have two
/// mutable child contexts active at the same time.
///
/// Contexts are hierarchical based on the strucutre of a battle:
///
/// - [`ActiveTargetContext`] - Every target Mon is associated with an active move.
/// - [`ActiveMoveContext`] - Every active move is performed by a Mon.
/// - [`MonContext`] - Every Mon is owned by a player.
/// - [`PlayerContext`] - Every player is on a side.
/// - [`SideContext`] - Every side is in a battle.
/// - [`Context`] - Scoped to a single battle.
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
    battle: *mut CoreBattle<'data>,
    // Collection of mons borrowed by the context chain.
    //
    // Mons are borrowed for the lifetime of the context chain. For instance, if a [`MonContext`]
    // borrows a Mon, that Mon will stay borrowed as long as the parent context lives, even if the
    // original [`MonContext`] is dropped.
    //
    // Borrowing Mons at the root of the chain allows multiple contexts to borrow the same Mon at
    // different parts in the chain. For example, consider the following chain:
    // - [`Context`]
    // - [`SideContext`]
    // - [`PlayerContext`]
    // - [`MonContext`]
    // - [`ActiveMoveContext`]
    // - [`ActiveTargetContext`]
    // - [`MonContext`] (attacker)
    //
    // In this chain, the two [`MonContext`] instances borrow the same Mon. However, the
    // construction of the latter context might not know that the Mon is already borrowed in the
    // chain. By holding the element reference at the root of the chain, the same borrow can be
    // used for both.
    //
    // SAFETY: To create a new context, the entire parent context must be borrowed mutably, which
    // means it cannot be used while the child context exists. In the above example, this means the
    // mutable reference to the Mon in the latter [`MonContext`] can never be used at the same time
    // as the mutable reference to the Mon in other parts of the chain.
    //
    // SAFETY: Never remove elements from this container. We could use a [`KeyedRegistry`] to help
    // make this guarantee, but that is slightly overkill.
    mons: FastHashMap<MonHandle, ElementRefMut<'battle, Mon>>,
    _phantom: PhantomData<&'battle mut CoreBattle<'data>>,
}

impl<'battle, 'data> Context<'battle, 'data> {
    /// Creates a new [`Context`], which contains a reference to a [`CoreBattle`].
    pub(in crate::battle) fn new(battle: &'battle mut CoreBattle<'data>) -> Self {
        Self {
            battle: &mut *battle,
            mons: FastHashMap::new(),
            _phantom: PhantomData,
        }
    }

    fn get_cached_mon<'context>(
        &'context mut self,
        mon_handle: MonHandle,
    ) -> Result<&'context mut Mon, Error> {
        // Multiple map look ups because the borrow checker cannot handle otherwise.
        if self.mons.contains_key(&mon_handle) {
            return self
                .mons
                .get_mut(&mon_handle)
                .wrap_error_with_format(format_args!("expected Mon {mon_handle} to exist in cache"))
                .map(|mon| mon.as_mut());
        }
        let mon = self.battle().mon_mut(mon_handle)?;
        let mon = unsafe { mem::transmute(mon) };
        self.mons.insert(mon_handle, mon);
        let mon = self
            .mons
            .get_mut(&mon_handle)
            .wrap_error_with_format(format_args!(
                "expected Mon {mon_handle} to have been inserted"
            ))?;
        Ok(mon.as_mut())
    }

    /// Creates a new [`SideContext`], scoped to the lifetime of this context.
    pub fn side_context(&mut self, side: usize) -> Result<SideContext<'_, 'battle, 'data>, Error> {
        SideContext::new(self.into(), side)
    }

    /// Creates a new [`PlayerContext`], scoped to the lifetime of this context.
    pub fn player_context(
        &mut self,
        player: usize,
    ) -> Result<PlayerContext<'_, '_, 'battle, 'data>, Error> {
        PlayerContext::new(self.into(), player)
    }

    /// Creates a new [`MonContext`], scoped to the lifetime of this context.
    pub fn mon_context(
        &mut self,
        mon_handle: MonHandle,
    ) -> Result<MonContext<'_, '_, '_, 'battle, 'data>, Error> {
        MonContext::new(self.into(), mon_handle)
    }

    /// Returns a reference to the [`CoreBattle`].
    pub fn battle(&self) -> &CoreBattle<'data> {
        unsafe { &*self.battle }
    }

    /// Returns a mutable reference to the [`CoreBattle`].
    pub fn battle_mut(&mut self) -> &mut CoreBattle<'data> {
        unsafe { &mut *self.battle }
    }

    /// Returns a reference to the [`BattleQueue`].
    pub fn battle_queue(&self) -> &BattleQueue {
        &self.battle().queue
    }

    /// Returns a mutable reference to the [`BattleQueue`].
    pub fn battle_queue_mut(&mut self) -> &mut BattleQueue {
        &mut self.battle_mut().queue
    }

    /// Returns a mutable iterator over the [`Side`]s of the battle.
    pub fn sides_mut(&mut self) -> impl Iterator<Item = &mut Side> {
        self.battle_mut().sides_mut()
    }

    /// Returns a mutable iterator over the [`Player`]s of the battle.
    pub fn players_mut(&mut self) -> impl Iterator<Item = &mut Player> {
        self.battle_mut().players_mut()
    }
}

/// The context of a [`Side`] in a battle.
///
/// A context is a proxy object for getting references to battle data. Rust does not make
/// storing references easy, so references must be grabbed dynamically as needed.
pub struct SideContext<'context, 'battle, 'data>
where
    'data: 'battle,
    'battle: 'context,
{
    context: MaybeOwnedMut<'context, Context<'battle, 'data>>,
    side: *mut Side,
    foe_side: *mut Side,
}

// All transmute calls are safe because the battle object and all references obtained from it live
// longer than the context.
impl<'context, 'battle, 'data> SideContext<'context, 'battle, 'data> {
    /// Creates a new [`SideContext`], which contains a reference to a [`CoreBattle`] and a
    /// [`Side`].
    pub(in crate::battle) fn new(
        mut context: MaybeOwnedMut<'context, Context<'battle, 'data>>,
        side: usize,
    ) -> Result<Self, Error> {
        // See comments on [`Context::new`] for why this is safe.
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
    pub fn foe_side_context(&mut self) -> Result<SideContext<'_, 'battle, 'data>, Error> {
        let foe_side = self.foe_side().index;
        self.as_battle_context_mut().side_context(foe_side)
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
}

/// The context of a [`Player`] in a battle.
///
/// A context is a proxy object for getting references to battle data. Rust does not make
/// storing references easy, so references must be grabbed dynamically as needed.
pub struct PlayerContext<'side, 'context, 'battle, 'data>
where
    'data: 'battle,
    'battle: 'context,
    'context: 'side,
{
    context: MaybeOwnedMut<'side, SideContext<'context, 'battle, 'data>>,
    player: *mut Player,
}

// All transmute calls are safe because the battle object and all references obtained from it live
// longer than the context.
impl<'side, 'context, 'battle, 'data> PlayerContext<'side, 'context, 'battle, 'data> {
    /// Creates a new [`PlayerContext`], which contains a reference to a [`CoreBattle`] and a
    /// [`Player`].
    pub(in crate::battle) fn new(
        mut context: MaybeOwnedMut<'context, Context<'battle, 'data>>,
        player: usize,
    ) -> Result<Self, Error> {
        // See comments on [`Context::new`] for why this is safe.
        let player: &mut Player =
            unsafe { mem::transmute(&mut *context.battle_mut().player_mut(player)?) };
        let context = SideContext::new(context, player.side)?;
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
}

/// Similar to [`MaybeOwned`][`crate::common::MaybeOwned`], but for an optional mutable reference
/// backed by a [`ElementRefMut`].
///
/// If the reference is owned the [`ElementRefMut`] is stored directly. If the reference is unowned,
/// it is stored directly with the assumption that it originates from an [`ElementRefMut`].
enum MaybeElementRef<'a, T> {
    Owned(ElementRefMut<'a, T>),
    Unowned(&'a mut T),
}

impl<T> Deref for MaybeElementRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(val) => val.deref(),
            Self::Unowned(val) => val,
        }
    }
}

impl<T> DerefMut for MaybeElementRef<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Owned(val) => val.deref_mut(),
            Self::Unowned(val) => val,
        }
    }
}

impl<T> AsMut<T> for MaybeElementRef<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

impl<'a, T> From<ElementRefMut<'a, T>> for MaybeElementRef<'a, T> {
    fn from(value: ElementRefMut<'a, T>) -> Self {
        Self::Owned(value)
    }
}

impl<'a, T> From<&'a mut T> for MaybeElementRef<'a, T> {
    fn from(value: &'a mut T) -> Self {
        Self::Unowned(value)
    }
}

/// The context of a [`Mon`] in a battle.
///
/// A context is a proxy object for getting references to battle data. Rust does not make
/// storing references easy, so references must be grabbed dynamically as needed.
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
    pub(in crate::battle) fn new(
        mut context: MaybeOwnedMut<'context, Context<'battle, 'data>>,
        mon_handle: MonHandle,
    ) -> Result<Self, Error> {
        let player = context.get_cached_mon(mon_handle)?.player;
        let mut context = PlayerContext::new(context, player)?;
        let mon = context.as_battle_context_mut().get_cached_mon(mon_handle)?;
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
            .as_battle_context_mut()
            .get_cached_mon(mon_handle)?;
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
        ActiveMoveContext::new_from_mon_context(self.into())
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
    pub fn active_move(&self) -> Result<ElementRef<Move>, Error> {
        Mon::active_move(self)
    }

    /// Returns a mutable reference to the active [`Move`], if it exists.
    pub fn active_move_mut(&mut self) -> Result<ElementRefMut<Move>, Error> {
        Mon::active_move_mut(self)
    }
}

/// The context of an active [`Move`] in a battle.
///
/// A context is a proxy object for getting references to battle data. Rust does not make
/// storing references easy, so references must be grabbed dynamically as needed.
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
    active_move: ElementRefMut<'context, Move>,
}

impl<'mon, 'player, 'side, 'context, 'battle, 'data>
    ActiveMoveContext<'mon, 'player, 'side, 'context, 'battle, 'data>
{
    fn new_from_mon_context(
        context: MaybeOwnedMut<'mon, MonContext<'player, 'side, 'context, 'battle, 'data>>,
    ) -> Result<Self, Error> {
        let active_move_handle = context
            .mon()
            .active_move
            .wrap_error_with_format(format_args!(
                "mon {} has no active move",
                context.mon_handle()
            ))?;
        let active_move = context
            .battle()
            .registry
            .this_turn_move_mut(active_move_handle)?;
        let active_move = unsafe { mem::transmute(active_move) };
        Ok(Self {
            context,
            active_move_handle,
            active_move,
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
    pub fn target_mon_context(
        &mut self,
        target_mon_handle: MonHandle,
    ) -> Result<MonContext<'_, '_, '_, 'battle, 'data>, Error> {
        let mon = self
            .as_battle_context_mut()
            .get_cached_mon(target_mon_handle)?;
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
    pub fn active_target_mon_context(
        &mut self,
    ) -> Result<MonContext<'_, '_, '_, 'battle, 'data>, Error> {
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
    pub fn target_context(
        &mut self,
        target_mon_handle: MonHandle,
    ) -> Result<ActiveTargetContext<'_, 'mon, 'player, 'side, 'context, 'battle, 'data>, Error>
    {
        ActiveTargetContext::new_from_active_move_context(self.into(), target_mon_handle)
    }

    pub fn active_target_context(
        &mut self,
    ) -> Result<ActiveTargetContext<'_, 'mon, 'player, 'side, 'context, 'battle, 'data>, Error>
    {
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
        ActiveMoveContext::new_from_mon_context(self.context)
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
}

/// The context of an active target [`Mon`] of a [`Move`] in a battle.
///
/// A context is a proxy object for getting references to battle data. Rust does not make
/// storing references easy, so references must be grabbed dynamically as needed.
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
        mut context: MaybeOwnedMut<
            'active_move,
            ActiveMoveContext<'mon, 'player, 'side, 'context, 'battle, 'data>,
        >,
        active_target_handle: MonHandle,
    ) -> Result<Self, Error> {
        let active_target = context
            .as_battle_context_mut()
            .get_cached_mon(active_target_handle)?;
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
        let active_target_handle = self.active_target_handle;
        let active_target = self
            .as_battle_context_mut()
            .get_cached_mon(active_target_handle)?;
        // SAFETY: Mons live as long as the battle itself, since they are stored in a registry. The
        // reference can be borrowed as long as the element reference exists in the root context. We
        // ensure that element references are borrowed for the lifetime of the root context.
        let active_target = unsafe { active_target.unsafely_detach_borrow_mut() };
        let player_context = self
            .as_battle_context_mut()
            .player_context(active_target.player)?;
        Ok(MonContext::new_from_mon_ref(
            player_context,
            active_target_handle,
            active_target,
        ))
    }

    /// Creates a new [`MonContext`] for the targeted [`Mon`], scoped to the lifetime of this
    /// context.
    fn mon_context<'active_target>(
        &'active_target mut self,
    ) -> Result<MonContext<'active_target, 'active_target, 'active_target, 'battle, 'data>, Error>
    {
        let handle = self.mon_handle();
        let mon = self.mon_mut();
        // SAFETY: Mons live as long as the battle itself, since they are stored in a registry. The
        // reference can be borrowed as long as the element reference exists in the root context. We
        // ensure that element references are borrowed for the lifetime of the root context.
        let mon = unsafe { mon.unsafely_detach_borrow_mut() };
        let player_context = self.as_battle_context_mut().player_context(mon.player)?;
        Ok(MonContext::new_from_mon_ref(player_context, handle, mon))
    }

    /// Creates a new [`MonContext`] for the attacker [`Mon`] for stat calculations, scoped to the
    /// lifetime of this context.
    pub fn attacker_context<'active_target>(
        &'active_target mut self,
    ) -> Result<MonContext<'active_target, 'active_target, 'active_target, 'battle, 'data>, Error>
    {
        match self.active_move().data.override_offensive_mon {
            Some(MonOverride::Target) => self.target_mon_context(),
            _ => self.mon_context(),
        }
    }

    /// Creates a new [`MonContext`] for the defender [`Mon`] for stat calculations, scoped to the
    /// lifetime of this context.
    pub fn defender_context<'active_target>(
        &'active_target mut self,
    ) -> Result<MonContext<'active_target, 'active_target, 'active_target, 'battle, 'data>, Error>
    {
        match self.active_move().data.override_defensive_mon {
            Some(MonOverride::User) => self.mon_context(),
            _ => self.target_mon_context(),
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
