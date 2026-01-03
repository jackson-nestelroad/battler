use alloc::format;
use core::{
    fmt,
    fmt::Display,
};

use anyhow::Result;
use zone_alloc::{
    BorrowError,
    ElementRef,
    ElementRefMut,
    Handle,
    KeyedRegistry,
    StrongRegistry,
};
use zone_alloc_strong_handle_derive::StrongHandle;

use crate::{
    battle::Mon,
    error::{
        ConvertError,
        general_error,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, StrongHandle)]
pub struct MoveHandle(Handle);

impl Display for MoveHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A [`Mon`] registry, which is a main memory allocation area for [`Mon`]s in a single
/// [`CoreBattle`][`crate::battle::CoreBattle`].
pub type MonRegistry = StrongRegistry<MonHandle, Mon>;

/// A [`Move`] registry, which is used for storing all move objects for a single turn of a
/// [`CoreBattle`][`crate::battle::CoreBattle`].
pub type MoveRegistry = KeyedRegistry<MoveHandle, Move>;

/// A centralized place for objects that must be accessed by reference all across the different
/// modules of a [`CoreBattle`][`crate::battle::CoreBattle`].
///
/// [`Mon`]s are guaranteed to live as long as the battle itself.
///
/// [`Move`]s are guaranteed to live at least two turns. A move will always live for the duration of
/// the turn it was used in. Once the battle progresses to the next turn, the move will be saved for
/// one more turn. If a [`Mon`] is still referencing a move that will be dropped at the end of the
/// turn, it is up to the battle to "save" the move using
/// [`Self::save_active_move_from_next_turn`] (which copies the move to the current turn).
pub struct BattleRegistry {
    mons: MonRegistry,
    last_turn_moves: MoveRegistry,
    this_turn_moves: MoveRegistry,
    next_active_move_handle: usize,
}

impl BattleRegistry {
    /// Creates a new [`BattleRegistry`].
    pub fn new() -> Self {
        Self {
            mons: MonRegistry::with_capacity(12),
            last_turn_moves: MoveRegistry::new(),
            this_turn_moves: MoveRegistry::new(),
            next_active_move_handle: 0,
        }
    }

    /// Registers a new [`Mon`], returning out the associated [`MonHandle`].
    pub fn register_mon(&self, mon: Mon) -> MonHandle {
        self.mons.register(mon)
    }

    /// Returns a reference to the [`Mon`] by [`MonHandle`].
    pub fn mon(&self, mon: MonHandle) -> Result<ElementRef<'_, Mon>> {
        self.mons
            .get(mon)
            .map_err(|err| err.convert_error_with_message(format!("mon {mon}")))
    }

    /// Returns a mutable reference to the [`Mon`] by [`MonHandle`].
    pub fn mon_mut(&self, mon: MonHandle) -> Result<ElementRefMut<'_, Mon>> {
        self.mons
            .get_mut(mon)
            .map_err(|err| err.convert_error_with_message(format!("mon {mon}")))
    }

    fn next_active_move_handle(&mut self) -> MoveHandle {
        let handle = MoveHandle::from(self.next_active_move_handle);
        self.next_active_move_handle += 1;
        handle
    }

    /// Registers a new [`Move`], returning out the associated [`MoveHandle`].
    pub fn register_move(&mut self, mov: Move) -> MoveHandle {
        let handle = self.next_active_move_handle();
        self.this_turn_moves.register(handle, mov);
        handle
    }

    /// Returns a reference to the [`Move`] by [`MoveHandle`].
    ///
    /// The move must be from this turn or last turn.
    pub fn active_move(&self, mov: MoveHandle) -> Result<ElementRef<'_, Move>> {
        match self.this_turn_moves.get(&mov) {
            Ok(active_move) => Ok(active_move),
            _ => match self.last_turn_moves.get(&mov) {
                Ok(active_move) => Ok(active_move),
                _ => Err(general_error(format!(
                    "active move {mov} does not exist in this turn or last turn",
                ))),
            },
        }
    }

    /// Returns a mutable reference to the [`Move`] by [`MoveHandle`].
    ///
    /// The move must be from this turn or last turn.
    pub fn active_move_mut(&self, mov: MoveHandle) -> Result<ElementRefMut<'_, Move>> {
        match self.this_turn_moves.get_mut(&mov) {
            Ok(active_move) => Ok(active_move),
            _ => match self.last_turn_moves.get_mut(&mov) {
                Ok(active_move) => Ok(active_move),
                _ => Err(general_error(format!(
                    "active move {mov} does not exist in this turn or last turn",
                ))),
            },
        }
    }

    /// Saves the given [`Move`] at [`MoveHandle`] from being dropped at the next turn.
    pub fn save_active_move_from_next_turn(&self, mov: MoveHandle) -> Result<()> {
        match self.last_turn_moves.get_mut(&mov) {
            Ok(active_move) => {
                self.this_turn_moves.register(mov, active_move.clone());
            }
            Err(BorrowError::OutOfBounds) => (),
            result @ _ => {
                return result
                    .map(|_| ())
                    .map_err(|err| err.convert_error_with_message(format!("active move {mov}")));
            }
        }
        Ok(())
    }

    /// Move the registry to the next turn.
    ///
    /// All move objects from last turn are dropped. Moves from this turn are moved to the last turn
    /// registry.
    pub fn next_turn(&mut self) -> Result<()> {
        // We detach element references in context chains, so we must check at runtime that no
        // dangling references will exist.
        if !self.last_turn_moves.safe_to_drop() {
            return Err(general_error(
                "cannot advance battle registry to the next turn: last_turn_moves is not safe to drop",
            ));
        }
        core::mem::swap(&mut self.last_turn_moves, &mut self.this_turn_moves);
        self.this_turn_moves = MoveRegistry::new();
        Ok(())
    }
}
