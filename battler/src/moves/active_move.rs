use ahash::HashMapExt;

use crate::{
    common::{
        FastHashMap,
        Fraction,
        Id,
        Identifiable,
    },
    moves::Move,
};

/// An active move, being used by a Mon right now.
///
/// This object is mutable, so that different effects can modify the active move.
pub struct ActiveMove<'m> {
    mov: &'m Move,
    /// Dynamic values assigned by move effect callbacks.
    pub dynamic: FastHashMap<Id, String>,
    /// Force this move to have a same-type attack bonus (STAB) on its next use.
    pub force_stab: bool,
    /// The move was used externally, rather than directly by a Mon through its moveset.
    pub is_external: bool,
    /// Whether or not this move hit multiple targets.
    pub spread_hit: bool,
    /// STAB multiplier (default is 1.5).
    pub stab_multiplier: Fraction,
    /// Volatile status for the move (not the user).
    pub volatile_status: Option<Id>,
}

impl<'m> ActiveMove<'m> {
    pub fn new(mov: &'m Move) -> Self {
        Self {
            mov,
            dynamic: FastHashMap::new(),
            force_stab: false,
            is_external: false,
            spread_hit: false,
            stab_multiplier: Fraction::from(1.5),
            volatile_status: None,
        }
    }
}

impl Identifiable for ActiveMove<'_> {
    fn id(&self) -> &Id {
        self.mov.id()
    }
}
