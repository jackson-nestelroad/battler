use std::{
    cell::UnsafeCell,
    mem,
};

use ahash::HashMap;
use anyhow::Result;
use zone_alloc::ElementRefMut;

use crate::{
    battle::{
        CoreBattle,
        Mon,
        MonHandle,
        MoveHandle,
    },
    error::WrapOptionError,
    moves::Move,
};

/// Cache of resources borrowed by a [`Context`][`crate::battle::Context`] chain.
///
/// Resources are borrowed for the lifetime of the context chain. For instance, if a
/// [`MonContext`][`crate::battle::MonContext`] borrows a [`Mon`], that [`Mon`] will stay borrowed
/// as long as the parent context lives, even if the original
/// [`MonContext`][`crate::battle::MonContext`] is dropped.
///
/// Borrowing resources at the root of the chain allows multiple contexts to borrow the same
/// resource at different parts in the chain.
pub struct ContextCache<'borrow> {
    mons: UnsafeCell<HashMap<MonHandle, ElementRefMut<'borrow, Mon>>>,
    active_moves: UnsafeCell<HashMap<MoveHandle, ElementRefMut<'borrow, Move>>>,
}

impl<'borrow> ContextCache<'borrow> {
    /// Creates a new context cache.
    pub fn new() -> Self {
        Self {
            mons: UnsafeCell::new(HashMap::default()),
            active_moves: UnsafeCell::new(HashMap::default()),
        }
    }

    /// Clears all borrowed references.
    pub fn clear(&mut self) {
        self.mons.get_mut().clear();
        self.active_moves.get_mut().clear();
    }

    /// Borrows a new [`Mon`] by handle.
    pub fn mon(&self, battle: &CoreBattle, mon_handle: MonHandle) -> Result<&mut Mon> {
        // SAFETY: This is the only method that accesses this map.
        let mons = unsafe { &mut *self.mons.get() };
        // Multiple map look ups because the borrow checker cannot handle otherwise.
        if mons.contains_key(&mon_handle) {
            return mons
                .get_mut(&mon_handle)
                .wrap_expectation_with_format(format_args!(
                    "expected Mon {mon_handle} to exist in cache"
                ))
                .map(|mon| mon.as_mut());
        }
        // SAFETY: This should always succeed, assuming that a Mon was not incorrectly borrowed
        // outside of this context.
        let mon = unsafe { battle.mon_mut(mon_handle)? };
        // SAFETY: We extend the lifetime of the ElementRef (dynamic borrow checking) so that the
        // borrow lives for the lifetime of the context chain.
        //
        // SAFETY: This is safe across multiple insertions because ElementRef does not own any data.
        // The underlying reference is not invalidated.
        let mon = unsafe { mem::transmute(mon) };
        mons.insert(mon_handle, mon);
        let mon = mons
            .get_mut(&mon_handle)
            .wrap_expectation_with_format(format_args!(
                "expected Mon {mon_handle} to have been inserted"
            ))?;
        Ok(mon.as_mut())
    }

    /// Borrows a new [`Move`] by handle.
    pub fn active_move(&self, battle: &CoreBattle, move_handle: MoveHandle) -> Result<&mut Move> {
        // SAFETY: This is the only method that accesses this map.
        let moves = unsafe { &mut *self.active_moves.get() };
        // Multiple map look ups because the borrow checker cannot handle otherwise.
        if moves.contains_key(&move_handle) {
            return moves
                .get_mut(&move_handle)
                .wrap_expectation_with_format(format_args!(
                    "expected active move {move_handle} to exist in cache"
                ))
                .map(|mov| mov.as_mut());
        }
        // SAFETY: This should always succeed, assuming that a move was not incorrectly borrowed
        // outside of this context.
        let mov = unsafe { battle.active_move_mut(move_handle)? };
        // SAFETY: We extend the lifetime of the ElementRef (dynamic borrow checking) so that the
        // borrow lives for the lifetime of the context chain.
        //
        // SAFETY: This is safe across multiple insertions because ElementRef does not own any data.
        // The underlying reference is not invalidated.
        let mov = unsafe { mem::transmute(mov) };
        moves.insert(move_handle, mov);
        let mov = moves
            .get_mut(&move_handle)
            .wrap_expectation_with_format(format_args!(
                "expected active move {move_handle} to have been inserted"
            ))?;
        Ok(mov.as_mut())
    }
}
