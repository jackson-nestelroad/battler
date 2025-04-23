pub use crate::common::Id;
use crate::{
    dex::{
        DataStore,
        ResourceDex,
        ResourceLookup,
        ResourceWrapper,
    },
    error::Error,
    moves::{
        Move,
        MoveData,
    },
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

    fn lookup(&self, id: &Id) -> Result<MoveData, Error> {
        self.data.get_move(id)
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
