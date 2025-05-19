use anyhow::Result;

use crate::{
    AbilityData,
    ClauseData,
    ConditionData,
    Id,
    ItemData,
    MoveData,
    SpeciesData,
    TypeChart,
};

/// Collection of tables for all resource data.
///
/// This trait can be implemented for different data sources, such as an external database or disk.
///
/// This collection is used for "raw lookup" of resources by ID. Individual dexes may implement
/// specialized lookup rules over this table, such as resolving aliases or special names.
pub trait DataStore: Send + Sync {
    /// Gets all move IDs, applying the given filter on the underlying data.
    fn all_move_ids(&self, filter: &dyn Fn(&MoveData) -> bool) -> Result<Vec<Id>>;

    /// Gets the type chart.
    fn get_type_chart(&self) -> Result<TypeChart>;

    /// Translates the given alias to another ID, if the alias mapping exists.
    fn translate_alias(&self, id: &Id) -> Result<Option<Id>>;

    /// Gets an ability by ID.
    fn get_ability(&self, id: &Id) -> Result<Option<AbilityData>>;
    /// Gets a clause by ID.
    fn get_clause(&self, id: &Id) -> Result<Option<ClauseData>>;
    /// Gets a condition by ID.
    fn get_condition(&self, id: &Id) -> Result<Option<ConditionData>>;
    /// Gets an item by ID.
    fn get_item(&self, id: &Id) -> Result<Option<ItemData>>;
    /// Gets a move by ID.
    fn get_move(&self, id: &Id) -> Result<Option<MoveData>>;
    /// Gets a species by ID.
    fn get_species(&self, id: &Id) -> Result<Option<SpeciesData>>;
}

/// An extension of [`DataStore`] for looking up resources by name.
pub trait DataStoreByName: DataStore {
    /// Gets an ability by name.
    fn get_ability_by_name(&self, name: &str) -> Result<Option<AbilityData>>;
    /// Gets a clause by name.
    fn get_clause_by_name(&self, name: &str) -> Result<Option<ClauseData>>;
    /// Gets a condition by name.
    fn get_condition_by_name(&self, name: &str) -> Result<Option<ConditionData>>;
    /// Gets an item by name.
    fn get_item_by_name(&self, name: &str) -> Result<Option<ItemData>>;
    /// Gets a move by name.
    fn get_move_by_name(&self, name: &str) -> Result<Option<MoveData>>;
    /// Gets a species by name.
    fn get_species_by_name(&self, name: &str) -> Result<Option<SpeciesData>>;
}
