use crate::dex::{
    AbilityDex,
    ClauseDex,
    DataStore,
    ItemDex,
    MoveDex,
    SpeciesDex,
};

/// Collection of all resources indexed by ID.
pub struct Dex<'d> {
    /// Collection of abilities.
    pub abilities: AbilityDex<'d>,
    /// Collection of clauses.
    pub clauses: ClauseDex<'d>,
    /// Collection of items.
    pub items: ItemDex<'d>,
    /// Collection of moves.
    pub moves: MoveDex<'d>,
    /// Collection of species.
    pub species: SpeciesDex<'d>,
}

impl<'d> Dex<'d> {
    /// Creates a new [`Dex`], backed by the given [`DataStore`].
    pub fn new(data: &'d dyn DataStore) -> Self {
        Self {
            abilities: AbilityDex::new(data),
            clauses: ClauseDex::new(data),
            items: ItemDex::new(data),
            moves: MoveDex::new(data),
            species: SpeciesDex::new(data),
        }
    }
}
