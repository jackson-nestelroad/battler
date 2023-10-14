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
    common::Error,
};

/// The context of a [`CoreBattle`].
///
/// A context is a proxy object for getting references to battle data. Rust does not make storing
/// references easy, so references must be grabbed dynamically as needed.
pub struct Context<'b, 'd> {
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
    battle: *mut CoreBattle<'d>,
    _phantom: PhantomData<&'b mut CoreBattle<'d>>,
}

impl<'b, 'd> Context<'b, 'd> {
    /// Creates a new [`Context`], which contains a reference to a [`CoreBattle`].
    pub(in crate::battle) fn new(battle: &'b mut CoreBattle<'d>) -> Self {
        Self {
            battle: &mut *battle,
            _phantom: PhantomData,
        }
    }

    /// Returns a reference to the [`CoreBattle`].
    pub fn battle(&self) -> &CoreBattle {
        unsafe { &*self.battle }
    }

    /// Returns a mutable reference to the [`CoreBattle`].
    pub fn battle_mut(&mut self) -> &mut CoreBattle<'d> {
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

/// The context of a [`Player`] in a battle.
///
/// A context is a proxy object for getting references to battle data. Rust does not make
/// storing references easy, so references must be grabbed dynamically as needed.
pub struct PlayerContext<'b, 'd> {
    context: Context<'b, 'd>,
    side: *mut Side,
    foe_side: *mut Side,
    player: *mut Player,
}

// All transmute calls are safe because the battle object and all references obtained from it live
// longer than the context.
impl<'b, 'd> PlayerContext<'b, 'd> {
    /// Creates a new [`PlayerContext`], which contains a reference to a [`CoreBattle`] and a
    /// [`Player`].
    pub(in crate::battle) fn new(
        mut context: Context<'b, 'd>,
        player: usize,
    ) -> Result<Self, Error> {
        // See comments on [`Context::new`] for why this is safe.
        let player: &mut Player =
            unsafe { mem::transmute(&mut *context.battle_mut().player_mut(player)?) };
        let side = player.side;
        let foe_side = side ^ 1;
        let side = unsafe { mem::transmute(&mut *context.battle_mut().side_mut(side)?) };
        let foe_side = unsafe { mem::transmute(&mut *context.battle_mut().side_mut(foe_side)?) };
        Ok(Self {
            context,
            side,
            foe_side,
            player,
        })
    }

    /// Returns a reference to the [`CoreBattle`].
    pub fn battle(&self) -> &CoreBattle {
        self.context.battle()
    }

    /// Returns a mutable reference to the [`CoreBattle`].
    pub fn battle_mut(&mut self) -> &mut CoreBattle<'d> {
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
pub struct MonContext<'b, 'd> {
    context: PlayerContext<'b, 'd>,
    mon: *mut Mon,
}

impl<'b, 'd> MonContext<'b, 'd> {
    /// Creates a new [`MonContext`], which contains a reference to a [`CoreBattle`] and a
    /// [`Mon`].
    pub(in crate::battle) fn new(context: Context<'b, 'd>, mon: MonHandle) -> Result<Self, Error> {
        // See comments on [`Context::new`] for why this is safe.
        let mon: &mut Mon =
            unsafe { mem::transmute(&mut *context.battle().registry.mon_mut(mon)?) };
        let player = mon.player;
        let context = PlayerContext::new(context, player)?;
        Ok(Self { context, mon })
    }

    /// Returns a reference to the [`CoreBattle`].
    pub fn battle(&self) -> &CoreBattle {
        self.context.battle()
    }

    /// Returns a mutable reference to the [`CoreBattle`].
    pub fn battle_mut(&mut self) -> &mut CoreBattle<'d> {
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

    /// Returns a reference to the [`Mon`].
    pub fn mon(&self) -> &Mon {
        unsafe { &*self.mon }
    }

    /// Returns a mutable reference to the [`Mon`].
    pub fn mon_mut(&mut self) -> &mut Mon {
        unsafe { &mut *self.mon }
    }
}
