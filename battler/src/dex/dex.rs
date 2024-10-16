use crate::{
    common::Id,
    dex::{
        AbilityDex,
        ClauseDex,
        ConditionDex,
        DataStore,
        ItemDex,
        MoveDex,
        SingleValueDex,
        SpeciesDex,
    },
    error::Error,
    mons::TypeChart,
    moves::MoveData,
};

/// Collection of all resources indexed by ID.
pub struct Dex<'d> {
    /// Collection of abilities.
    pub abilities: AbilityDex<'d>,
    /// Collection of clauses.
    pub clauses: ClauseDex<'d>,
    /// Collection of conditions.
    pub conditions: ConditionDex<'d>,
    /// Collection of items.
    pub items: ItemDex<'d>,
    /// Collection of moves.
    pub moves: MoveDex<'d>,
    /// Collection of species.
    pub species: SpeciesDex<'d>,
    /// Type chart.
    type_chart: SingleValueDex<'d, TypeChart>,

    data: &'d dyn DataStore,
}

impl<'d> Dex<'d> {
    /// Creates a new [`Dex`], backed by the given [`DataStore`].
    pub fn new(data: &'d dyn DataStore) -> Result<Self, Error> {
        let type_chart = SingleValueDex::new(data, data.get_type_chart()?);
        Ok(Self {
            abilities: AbilityDex::new(data),
            clauses: ClauseDex::new(data),
            conditions: ConditionDex::new(data),
            items: ItemDex::new(data),
            moves: MoveDex::new(data),
            species: SpeciesDex::new(data),
            type_chart,
            data,
        })
    }

    /// Returns the cached type chart.
    pub fn type_chart(&self) -> &TypeChart {
        self.type_chart.get()
    }

    /// Gets all move IDs, applying the given filter on the underlying data.
    pub fn all_move_ids(&self, filter: &dyn Fn(&MoveData) -> bool) -> Result<Vec<Id>, Error> {
        self.data.all_move_ids(filter)
    }
}
