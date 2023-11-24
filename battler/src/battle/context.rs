use std::{
    marker::PhantomData,
    mem,
};

use crate::{
    battle::{
        BattleQueue,
        CoreBattle,
        Mon,
        MonHandle,
        Player,
        Side,
    },
    common::{
        Error,
        MaybeOwnedMut,
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
    _phantom: PhantomData<&'battle mut CoreBattle<'data>>,
}

impl<'battle, 'data> Context<'battle, 'data> {
    /// Creates a new [`Context`], which contains a reference to a [`CoreBattle`].
    pub(in crate::battle) fn new(battle: &'battle mut CoreBattle<'data>) -> Self {
        Self {
            battle: &mut *battle,
            _phantom: PhantomData,
        }
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
    mon: *mut Mon,
}

impl<'player, 'side, 'context, 'battle, 'data>
    MonContext<'player, 'side, 'context, 'battle, 'data>
{
    /// Creates a new [`MonContext`], which contains a reference to a [`CoreBattle`] and a
    /// [`Mon`].
    pub(in crate::battle) fn new(
        context: MaybeOwnedMut<'context, Context<'battle, 'data>>,
        mon_handle: MonHandle,
    ) -> Result<Self, Error> {
        // See comments on [`Context::new`] for why this is safe.
        let mon: &mut Mon = unsafe { mem::transmute(&mut *context.battle().mon_mut(mon_handle)?) };
        let player = mon.player;
        let context = PlayerContext::new(context, player)?;
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
        // See comments on [`Context::new`] for why this is safe.
        let mon: &mut Mon =
            unsafe { mem::transmute(&mut *player_context.battle().mon_mut(mon_handle)?) };
        Ok(Self {
            context: player_context.into(),
            mon_handle,
            mon,
        })
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
        unsafe { &*self.mon }
    }

    /// Returns a mutable reference to the [`Mon`].
    pub fn mon_mut(&mut self) -> &mut Mon {
        unsafe { &mut *self.mon }
    }
}
