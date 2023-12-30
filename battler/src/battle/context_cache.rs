use std::{
    cell::UnsafeCell,
    mem,
};

use ahash::HashMapExt;
use zone_alloc::ElementRefMut;

use crate::{
    battle::{
        CoreBattle,
        Mon,
        MonHandle,
        MoveHandle,
    },
    common::{
        Error,
        FastHashMap,
        WrapResultError,
    },
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
///
/// SAFETY: Never remove elements from these containers. We could use a
/// [`KeyedRegistry`][`zone_alloc::KeyedRegistry`] to help make this guarantee, but that is slightly
/// overkill.
pub struct ContextCache<'borrow> {
    mons: UnsafeCell<FastHashMap<MonHandle, ElementRefMut<'borrow, Mon>>>,
    active_moves: UnsafeCell<FastHashMap<MoveHandle, ElementRefMut<'borrow, Move>>>,
}

impl<'borrow> ContextCache<'borrow> {
    pub fn new() -> Self {
        Self {
            mons: UnsafeCell::new(FastHashMap::new()),
            active_moves: UnsafeCell::new(FastHashMap::new()),
        }
    }

    pub fn mon(&self, battle: &CoreBattle, mon_handle: MonHandle) -> Result<&mut Mon, Error> {
        // SAFETY: This is the only method that accesses this map.
        let mons = unsafe { &mut *self.mons.get() };
        // Multiple map look ups because the borrow checker cannot handle otherwise.
        if mons.contains_key(&mon_handle) {
            return mons
                .get_mut(&mon_handle)
                .wrap_error_with_format(format_args!("expected Mon {mon_handle} to exist in cache"))
                .map(|mon| mon.as_mut());
        }
        let mon = unsafe { battle.mon_mut(mon_handle)? };
        let mon = unsafe { mem::transmute(mon) };
        mons.insert(mon_handle, mon);
        let mon = mons
            .get_mut(&mon_handle)
            .wrap_error_with_format(format_args!(
                "expected Mon {mon_handle} to have been inserted"
            ))?;
        Ok(mon.as_mut())
    }

    pub fn active_move(
        &self,
        battle: &CoreBattle,
        move_handle: MoveHandle,
    ) -> Result<&mut Move, Error> {
        // SAFETY: This is the only method that accesses this map.
        let moves = unsafe { &mut *self.active_moves.get() };
        // Multiple map look ups because the borrow checker cannot handle otherwise.
        if moves.contains_key(&move_handle) {
            return moves
                .get_mut(&move_handle)
                .wrap_error_with_format(format_args!(
                    "expected active move {move_handle} to exist in cache"
                ))
                .map(|mov| mov.as_mut());
        }
        let mov = unsafe { battle.this_turn_move_mut(move_handle)? };
        let mov = unsafe { mem::transmute(mov) };
        moves.insert(move_handle, mov);
        let mov = moves
            .get_mut(&move_handle)
            .wrap_error_with_format(format_args!(
                "expected active move {move_handle} to have been inserted"
            ))?;
        Ok(mov.as_mut())
    }
}
