use std::{
    fmt,
    fmt::Display,
    mem,
};

use zone_alloc::{
    ElementRef,
    ElementRefMut,
    Handle,
    StrongRegistry,
};
use zone_alloc_strong_handle_derive::StrongHandle;

use crate::{
    battle::Mon,
    common::{
        Error,
        WrapResultError,
    },
    moves::Move,
};

/// A [`Mon`] handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, StrongHandle)]
pub struct MonHandle(Handle);

impl Display for MonHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A [`Move`] handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, StrongHandle)]
pub struct MoveHandle(Handle);

impl Display for MoveHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A [`Mon`] registry, which is a main memory allocation area for [`Mon`]s in a single
/// [`Battle`][`crate::battle::Battle`].
pub type MonRegistry = StrongRegistry<MonHandle, Mon>;

/// A [`Move`] registry, which is used for storing all move objects for a single turn of a
/// [`Battle`][`crate::battle::Battle`].
pub type MoveRegistry = StrongRegistry<MoveHandle, Move>;

/// A centralized place for objects that must be accessed by reference all across the different
/// modules of a [`Battle`][`crate::battle::Battle`]. These objects are guaranteed to live as long
/// as the battle itself.
pub struct BattleRegistry {
    mons: MonRegistry,
    last_turn_moves: MoveRegistry,
    this_turn_moves: MoveRegistry,
}

impl BattleRegistry {
    /// Creates a new [`BattleRegistry`].
    pub fn new() -> Self {
        Self {
            mons: MonRegistry::with_capacity(12),
            last_turn_moves: MoveRegistry::new(),
            this_turn_moves: MoveRegistry::new(),
        }
    }

    /// Registers a new [`Mon`], returning out the associated [`MonHandle`].
    pub fn register_mon(&self, mon: Mon) -> MonHandle {
        self.mons.register(mon)
    }

    /// Returns a reference to the [`Mon`] by [`MonHandle`].
    pub fn mon(&self, mon: MonHandle) -> Result<ElementRef<Mon>, Error> {
        self.mons
            .get(mon)
            .wrap_error_with_format(format_args!("failed to access Mon {mon}"))
    }

    /// Returns a mutable reference to the [`Mon`] by [`MonHandle`].
    pub fn mon_mut(&self, mon: MonHandle) -> Result<ElementRefMut<Mon>, Error> {
        self.mons
            .get_mut(mon)
            .wrap_error_with_format(format_args!("failed to access Mon {mon}"))
    }

    /// Registers a new [`Move`], returning out the associated [`MoveHandle`].
    pub fn register_move(&self, mov: Move) -> MoveHandle {
        self.this_turn_moves.register(mov)
    }

    /// Returns a reference to the [`Move`] (from this turn) by [`MoveHandle`].
    pub fn this_turn_move(&self, mov: MoveHandle) -> Result<ElementRef<Move>, Error> {
        self.this_turn_moves
            .get(mov)
            .wrap_error_with_format(format_args!("failed to access move from this turn {mov}"))
    }

    /// Returns a mutable reference to the [`Move`] (from this turn) by [`MoveHandle`].
    pub fn this_turn_move_mut(&self, mov: MoveHandle) -> Result<ElementRefMut<Move>, Error> {
        self.this_turn_moves
            .get_mut(mov)
            .wrap_error_with_format(format_args!("failed to access move from this turn {mov}"))
    }

    /// Returns a reference to the [`Move`] (from last turn) by [`MoveHandle`].
    pub fn last_turn_move(&self, mov: MoveHandle) -> Result<ElementRef<Move>, Error> {
        self.this_turn_moves
            .get(mov)
            .wrap_error_with_format(format_args!("failed to access move from last turn {mov}"))
    }

    /// Returns a mutable reference to the [`Move`] (from last turn) by [`MoveHandle`].
    pub fn last_turn_move_mut(&self, mov: MoveHandle) -> Result<ElementRefMut<Move>, Error> {
        self.this_turn_moves
            .get_mut(mov)
            .wrap_error_with_format(format_args!("failed to access move from last turn {mov}"))
    }

    /// Move the registry to the next turn.
    ///
    /// All move objects from last turn are dropped. Moves from this turn are moved to the last turn
    /// registry.
    pub fn next_turn(&mut self) {
        mem::swap(&mut self.last_turn_moves, &mut self.this_turn_moves);
        self.this_turn_moves = MoveRegistry::new();
    }
}
