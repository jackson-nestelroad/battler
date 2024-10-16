use ahash::{
    HashMap,
    HashMapExt,
};
use battler::{
    abilities::AbilityData,
    common::Id,
    conditions::ConditionData,
    config::ClauseData,
    dex::{
        DataStore,
        LocalDataStore,
    },
    error::Error,
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
    fn all_move_ids(&self, filter: &dyn Fn(&MoveData) -> bool) -> Result<Vec<Id>, Error> {
        let mut all_moves = self.local.all_move_ids(filter)?;
        let mut fake_moves = self
            .fake_moves
            .iter()
            .filter_map(|(id, move_data)| filter(move_data).then(|| id.clone()))
            .collect::<Vec<_>>();
        all_moves.append(&mut fake_moves);
        Ok(all_moves)
    }

    fn get_type_chart(&self) -> Result<TypeChart, Error> {
        self.local.get_type_chart()
    }

    fn translate_alias(&self, id: &Id) -> Result<Id, Error> {
        self.local.translate_alias(id)
    }

    fn get_ability(&self, id: &Id) -> Result<AbilityData, Error> {
        self.local.get_ability(id)
    }

    fn get_clause(&self, id: &Id) -> Result<ClauseData, Error> {
        self.local.get_clause(id)
    }

    fn get_condition(&self, id: &Id) -> Result<ConditionData, Error> {
        self.local.get_condition(id)
    }

    fn get_item(&self, id: &Id) -> Result<ItemData, Error> {
        self.local.get_item(id)
    }

    fn get_move(&self, id: &Id) -> Result<MoveData, Error> {
        match self.fake_moves.get(id) {
            Some(fake_move) => Ok(fake_move.clone()),
            None => self.local.get_move(id),
        }
    }

    fn get_species(&self, id: &Id) -> Result<SpeciesData, Error> {
        self.local.get_species(id)
    }
}
