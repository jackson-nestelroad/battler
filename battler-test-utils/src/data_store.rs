use ahash::{
    HashMap,
    HashMapExt,
};
use anyhow::Result;
use battler::{
    AbilityData,
    ClauseData,
    ConditionData,
    DataStore,
    Id,
    ItemData,
    LocalDataStore,
    MoveData,
    SpeciesData,
    TypeChart,
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
    pub fn new_from_env(env_var: &str) -> Result<Self> {
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
    fn all_move_ids(&self, filter: &dyn Fn(&MoveData) -> bool) -> Result<Vec<Id>> {
        let mut all_moves = self.local.all_move_ids(filter)?;
        let mut fake_moves = self
            .fake_moves
            .iter()
            .filter_map(|(id, move_data)| filter(move_data).then(|| id.clone()))
            .collect::<Vec<_>>();
        all_moves.append(&mut fake_moves);
        Ok(all_moves)
    }

    fn get_type_chart(&self) -> Result<TypeChart> {
        self.local.get_type_chart()
    }

    fn translate_alias(&self, id: &Id) -> Result<Option<Id>> {
        self.local.translate_alias(id)
    }

    fn get_ability(&self, id: &Id) -> Result<Option<AbilityData>> {
        self.local.get_ability(id)
    }

    fn get_clause(&self, id: &Id) -> Result<Option<ClauseData>> {
        self.local.get_clause(id)
    }

    fn get_condition(&self, id: &Id) -> Result<Option<ConditionData>> {
        self.local.get_condition(id)
    }

    fn get_item(&self, id: &Id) -> Result<Option<ItemData>> {
        self.local.get_item(id)
    }

    fn get_move(&self, id: &Id) -> Result<Option<MoveData>> {
        match self.fake_moves.get(id) {
            Some(fake_move) => Ok(Some(fake_move.clone())),
            None => self.local.get_move(id),
        }
    }

    fn get_species(&self, id: &Id) -> Result<Option<SpeciesData>> {
        self.local.get_species(id)
    }
}
