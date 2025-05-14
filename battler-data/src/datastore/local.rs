use std::{
    env,
    fs::File,
    path::Path,
};

use ahash::HashMap;
use anyhow::{
    Context,
    Error,
    Result,
};
use serde::de::DeserializeOwned;

use crate::{
    AbilityData,
    Aliases,
    ClauseData,
    ConditionData,
    DataStore,
    Id,
    ItemData,
    MoveData,
    SpeciesData,
    TypeChart,
    deserialize_aliases,
};

/// An implementation of [`DataStore`] that reads all data locally from disk.
pub struct LocalDataStore {
    root: String,
    pub type_chart: TypeChart,
    pub abilities: HashMap<Id, AbilityData>,
    pub aliases: Aliases,
    pub clauses: HashMap<Id, ClauseData>,
    pub conditions: HashMap<Id, ConditionData>,
    pub items: HashMap<Id, ItemData>,
    pub moves: HashMap<Id, MoveData>,
    pub species: HashMap<Id, SpeciesData>,
}

impl LocalDataStore {
    /// Type chart file name.
    pub const TYPE_CHART_FILE: &str = "type-chart.json";
    /// Aliases file name.
    pub const ALIASES_FILE: &str = "aliases.json";
    /// Clauses file name.
    pub const CLAUSES_FILE: &str = "clauses.json";
    /// Conditions file name.
    pub const CONDITIONS_FILE: &str = "conditions.json";
    /// Abilities directory name.
    pub const ABILITIES_DIR: &str = "abilities";
    /// Items directory name.
    pub const ITEMS_DIR: &str = "items";
    /// Moves directory name.
    pub const MOVES_DIR: &str = "moves";
    /// Species directory name.
    pub const SPECIES_DIR: &str = "mons";

    /// Creates a new instance of [`LocalDataStore`] that reads from the given root directory.
    ///
    /// Fails if the path does not exist, does not point to a directory, or cannot be used to fill
    /// cached data.
    pub fn new(root: String) -> Result<Self> {
        if !Path::new(&root).is_dir() {
            return Err(Error::msg(format!(
                "Root directory for LocalDataStore ({root}) does not exist",
            )));
        }
        let mut store = Self {
            root,
            type_chart: TypeChart::new(),
            abilities: HashMap::default(),
            aliases: Aliases::default(),
            clauses: HashMap::default(),
            conditions: HashMap::default(),
            items: HashMap::default(),
            moves: HashMap::default(),
            species: HashMap::default(),
        };
        store.initialize()?;
        Ok(store)
    }

    /// Creates a new instance of [`LocalDataStore`] that reads from the root directory at the given
    /// environment variable.
    pub fn new_from_env(env_var: &str) -> Result<Self> {
        Self::new(env::var(env_var).context("DATA_DIR not defined")?)
    }

    fn initialize(&mut self) -> Result<()> {
        self.type_chart = serde_json::from_reader(
            File::open(Path::new(&self.root).join(Self::TYPE_CHART_FILE))
                .context("failed to read type chart")?,
        )
        .context("failed to parse type chart")?;

        self.aliases = deserialize_aliases(
            serde_json::from_reader(
                File::open(Path::new(&self.root).join(Self::ALIASES_FILE))
                    .context("failed to read type chart")?,
            )
            .context("failed to parse type chart")?,
        );

        let clauses: HashMap<Id, ClauseData> = serde_json::from_reader(
            File::open(Path::new(&self.root).join(Self::CLAUSES_FILE))
                .context("failed to read clauses")?,
        )
        .context("failed to parse clauses")?;
        self.clauses.extend(clauses);

        let conditions: HashMap<Id, ConditionData> = serde_json::from_reader(
            File::open(Path::new(&self.root).join(Self::CONDITIONS_FILE))
                .context("failed to read conditions")?,
        )
        .context("failed to parse conditions")?;
        self.conditions.extend(conditions);

        self.abilities = self.read_all_files_in_directory::<AbilityData>(Self::ABILITIES_DIR)?;
        self.items = self.read_all_files_in_directory::<ItemData>(Self::ITEMS_DIR)?;
        self.moves = self.read_all_files_in_directory::<MoveData>(Self::MOVES_DIR)?;
        self.species = self.read_all_files_in_directory::<SpeciesData>(Self::SPECIES_DIR)?;

        Ok(())
    }

    fn read_all_files_in_directory<T: DeserializeOwned>(
        &self,
        dir: &str,
    ) -> Result<HashMap<Id, T>> {
        let tables = Path::new(&self.root)
            .join(dir)
            .read_dir()
            .context(format!("failed to read {dir} directory"))?
            .filter_map(|entry| Some(entry.ok()?.path()))
            .filter(|path| path.is_file())
            .map(|path| {
                let path_name = path.to_string_lossy().to_string();
                serde_json::from_reader::<File, HashMap<String, T>>(
                    File::open(path).context(format!("{path_name} could not be opened"))?,
                )
                .context(format!("failed to read {dir} data from {path_name}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut map = HashMap::default();
        map.extend(
            tables
                .into_iter()
                .map(|table| table.into_iter())
                .flatten()
                .map(|(key, value)| (Id::from(key), value)),
        );
        Ok(map)
    }
}

impl DataStore for LocalDataStore {
    fn all_move_ids(&self, filter: &dyn Fn(&MoveData) -> bool) -> Result<Vec<Id>> {
        let mut move_ids = Vec::new();
        for (id, move_data) in self.moves.iter() {
            if filter(move_data) {
                move_ids.push(id.clone());
            }
        }
        Ok(move_ids)
    }

    fn get_type_chart(&self) -> Result<TypeChart> {
        Ok(self.type_chart.clone())
    }

    fn translate_alias(&self, id: &Id) -> Result<Option<Id>> {
        Ok(self.aliases.get(id).cloned())
    }

    fn get_ability(&self, id: &Id) -> Result<Option<AbilityData>> {
        Ok(self.abilities.get(id).cloned())
    }

    fn get_clause(&self, id: &Id) -> Result<Option<ClauseData>> {
        Ok(self.clauses.get(id).cloned())
    }

    fn get_condition(&self, id: &Id) -> Result<Option<ConditionData>> {
        Ok(self.conditions.get(id).cloned())
    }

    fn get_item(&self, id: &Id) -> Result<Option<ItemData>> {
        Ok(self.items.get(id).cloned())
    }

    fn get_move(&self, id: &Id) -> Result<Option<MoveData>> {
        Ok(self.moves.get(id).cloned())
    }

    fn get_species(&self, id: &Id) -> Result<Option<SpeciesData>> {
        Ok(self.species.get(id).cloned())
    }
}
