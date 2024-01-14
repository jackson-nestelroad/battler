use ahash::{
    HashMap,
    HashMapExt,
};
use battler::{
    abilities::AbilityData,
    common::{
        Error,
        Id,
    },
    config::ClauseData,
    dex::{
        DataLookupResult,
        DataStore,
        LocalDataStore,
    },
    items::ItemData,
    mons::{
        SpeciesData,
        TypeChart,
    },
    moves::MoveData,
};

/// Wrapper around a battle's [`LocalDataStore`] for testing.
///
/// Allows fake values to be inserted exclusively for the test. This is great for tests that focus
/// more on how the battle engine behaves than the battle itself.
pub struct TestDataStore {
    local: LocalDataStore,
    fake_moves: HashMap<Id, MoveData>,
}

impl TestDataStore {
    pub fn new_from_env(env_var: &str) -> Result<Self, Error> {
        let local = LocalDataStore::new_from_env(env_var)?;
        Ok(Self {
            local,
            fake_moves: HashMap::new(),
        })
    }

    pub fn add_fake_move(&mut self, id: Id, move_data: MoveData) {
        self.fake_moves.insert(id, move_data);
    }
}

impl DataStore for TestDataStore {
    fn get_type_chart(&self) -> DataLookupResult<TypeChart> {
        self.local.get_type_chart()
    }

    fn translate_alias(&self, id: &Id) -> DataLookupResult<Id> {
        self.local.translate_alias(id)
    }

    fn get_ability(&self, id: &Id) -> DataLookupResult<AbilityData> {
        self.local.get_ability(id)
    }

    fn get_clause(&self, id: &Id) -> DataLookupResult<ClauseData> {
        self.local.get_clause(id)
    }

    fn get_item(&self, id: &Id) -> DataLookupResult<ItemData> {
        self.local.get_item(id)
    }

    fn get_move(&self, id: &Id) -> DataLookupResult<MoveData> {
        Into::<DataLookupResult<MoveData>>::into(self.fake_moves.get(id).cloned())
            .or_else(|| self.local.get_move(id))
    }

    fn get_species(&self, id: &Id) -> DataLookupResult<SpeciesData> {
        self.local.get_species(id)
    }
}
