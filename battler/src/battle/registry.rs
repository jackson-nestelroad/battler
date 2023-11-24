use std::{
    fmt,
    fmt::Display,
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
};

/// A [`Mon`] handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, StrongHandle)]
pub struct MonHandle(Handle);

impl Display for MonHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A [`Mon`] registry, which is a main memory allocation area for [`Mon`]s in a single
/// [`Battle`][`crate::battle::Battle`].
pub type MonRegistry = StrongRegistry<MonHandle, Mon>;

/// A centralized place for objects that must be accessed by reference all across the different
/// modules of a [`Battle`][`crate::battle::Battle`]. These objects are guaranteed to live as long
/// as the battle itself.
pub struct BattleRegistry {
    /// Registry of [`Mon`]s.
    mons: MonRegistry,
}

impl BattleRegistry {
    /// Creates a new [`BattleRegistry`].
    pub fn new() -> Self {
        Self {
            mons: MonRegistry::with_capacity(12),
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
}
