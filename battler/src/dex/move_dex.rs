use anyhow::Result;
use battler_data::{
    DataStore,
    Id,
    MoveData,
};

use crate::{
    dex::{
        ResourceDex,
        ResourceLookup,
        ResourceWrapper,
    },
    moves::Move,
    WrapOptionError,
};

/// Lookup type for [`MoveDex`].
#[derive(Clone)]
pub struct MoveLookup<'d> {
    data: &'d dyn DataStore,
}

impl<'d> ResourceLookup<'d, MoveData> for MoveLookup<'d> {
    fn new(data: &'d dyn DataStore) -> Self {
        Self { data }
    }

    fn lookup(&self, id: &Id) -> Result<MoveData> {
        self.data
            .get_move(id)?
            .wrap_not_found_error_with_format(format_args!("move {id}"))
    }
}

/// Wrapper type for [`MoveDex`].
pub struct MoveWrapper;

impl ResourceWrapper<MoveData, Move> for MoveWrapper {
    fn wrap(id: Id, data: MoveData) -> Move {
        Move::new(id, data)
    }
}

/// Indexed collection of moves.
pub type MoveDex<'d> = ResourceDex<'d, MoveData, Move, MoveLookup<'d>, MoveWrapper>;
