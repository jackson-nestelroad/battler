use crate::{
    common::Error,
    dex::{
        AbilityDex,
        ClauseDex,
        DataStore,
        ItemDex,
        MoveDex,
        SingleValueDex,
        SpeciesDex,
    },
    mons::TypeChart,
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
    /// Type chart.
    type_chart: SingleValueDex<'d, TypeChart>,
}

impl<'d> Dex<'d> {
    /// Creates a new [`Dex`], backed by the given [`DataStore`].
    pub fn new(data: &'d dyn DataStore) -> Result<Self, Error> {
        let type_chart = SingleValueDex::new(data, data.get_type_chart()?);
        Ok(Self {
            abilities: AbilityDex::new(data),
            clauses: ClauseDex::new(data),
            items: ItemDex::new(data),
            moves: MoveDex::new(data),
            species: SpeciesDex::new(data),
            type_chart,
        })
    }

    /// Returns the cached type chart.
    pub fn type_chart(&self) -> &TypeChart {
        self.type_chart.get()
    }
}
