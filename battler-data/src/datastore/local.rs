use std::{
    env,
    fs::File,
    path::Path,
    sync::RwLock,
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
    DataStoreByName,
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

    abilities_by_name: RwLock<HashMap<String, Id>>,
    clauses_by_name: RwLock<HashMap<String, Id>>,
    conditions_by_name: RwLock<HashMap<String, Id>>,
    items_by_name: RwLock<HashMap<String, Id>>,
    moves_by_name: RwLock<HashMap<String, Id>>,
    species_by_name: RwLock<HashMap<String, Id>>,
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
    /// Move bases file name.
    pub const MOVE_BASES_FILE: &str = "move-bases.json";
    /// Ability bases file name.
    pub const ABILITY_BASES_FILE: &str = "ability-bases.json";
    /// Item bases file name.
    pub const ITEM_BASES_FILE: &str = "item-bases.json";
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
            abilities_by_name: RwLock::new(HashMap::default()),
            clauses_by_name: RwLock::new(HashMap::default()),
            conditions_by_name: RwLock::new(HashMap::default()),
            items_by_name: RwLock::new(HashMap::default()),
            moves_by_name: RwLock::new(HashMap::default()),
            species_by_name: RwLock::new(HashMap::default()),
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

        let move_bases: HashMap<Id, ConditionData> = serde_json::from_reader(
            File::open(Path::new(&self.root).join(Self::MOVE_BASES_FILE))
                .context("failed to read move bases")?,
        )
        .context("failed to parse move bases")?;
        self.conditions.extend(move_bases);

        let ability_bases: HashMap<Id, ConditionData> = serde_json::from_reader(
            File::open(Path::new(&self.root).join(Self::ABILITY_BASES_FILE))
                .context("failed to read ability bases")?,
        )
        .context("failed to parse ability bases")?;
        self.conditions.extend(ability_bases);

        let item_bases: HashMap<Id, ConditionData> = serde_json::from_reader(
            File::open(Path::new(&self.root).join(Self::ITEM_BASES_FILE))
                .context("failed to read item bases")?,
        )
        .context("failed to parse item bases")?;
        self.conditions.extend(item_bases);

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

impl DataStoreByName for LocalDataStore {
    fn get_ability_by_name(&self, name: &str) -> Result<Option<AbilityData>> {
        if let Ok(cache) = self.abilities_by_name.read()
            && let Some(id) = cache.get(name)
        {
            return self.get_ability(id);
        }

        let (id, ability) = match self
            .abilities
            .iter()
            .find(|(_, ability)| ability.name == name)
        {
            Some((id, ability)) => (id.clone(), ability.clone()),
            None => return Ok(None),
        };
        self.abilities_by_name
            .write()
            .unwrap_or_else(|mut err| {
                **err.get_mut() = HashMap::default();
                self.abilities_by_name.clear_poison();
                err.into_inner()
            })
            .insert(name.to_owned(), id);
        Ok(Some(ability))
    }

    fn get_clause_by_name(&self, name: &str) -> Result<Option<ClauseData>> {
        if let Ok(cache) = self.clauses_by_name.read()
            && let Some(id) = cache.get(name)
        {
            return self.get_clause(id);
        }

        let (id, clause) = match self.clauses.iter().find(|(_, clause)| clause.name == name) {
            Some((id, clause)) => (id.clone(), clause.clone()),
            None => return Ok(None),
        };
        self.clauses_by_name
            .write()
            .unwrap_or_else(|mut err| {
                **err.get_mut() = HashMap::default();
                self.clauses_by_name.clear_poison();
                err.into_inner()
            })
            .insert(name.to_owned(), id);
        Ok(Some(clause))
    }

    fn get_condition_by_name(&self, name: &str) -> Result<Option<ConditionData>> {
        if let Ok(cache) = self.conditions_by_name.read()
            && let Some(id) = cache.get(name)
        {
            return self.get_condition(id);
        }

        let (id, condition) = match self
            .conditions
            .iter()
            .find(|(_, condition)| condition.name == name)
        {
            Some((id, condition)) => (id.clone(), condition.clone()),
            None => return Ok(None),
        };
        self.conditions_by_name
            .write()
            .unwrap_or_else(|mut err| {
                **err.get_mut() = HashMap::default();
                self.conditions_by_name.clear_poison();
                err.into_inner()
            })
            .insert(name.to_owned(), id);
        Ok(Some(condition))
    }

    fn get_item_by_name(&self, name: &str) -> Result<Option<ItemData>> {
        if let Ok(cache) = self.items_by_name.read()
            && let Some(id) = cache.get(name)
        {
            return self.get_item(id);
        }

        let (id, item) = match self.items.iter().find(|(_, item)| item.name == name) {
            Some((id, item)) => (id.clone(), item.clone()),
            None => return Ok(None),
        };
        self.items_by_name
            .write()
            .unwrap_or_else(|mut err| {
                **err.get_mut() = HashMap::default();
                self.items_by_name.clear_poison();
                err.into_inner()
            })
            .insert(name.to_owned(), id);
        Ok(Some(item))
    }

    fn get_move_by_name(&self, name: &str) -> Result<Option<MoveData>> {
        if let Ok(cache) = self.moves_by_name.read()
            && let Some(id) = cache.get(name)
        {
            return self.get_move(id);
        }

        let (id, mov) = match self.moves.iter().find(|(_, mov)| mov.name == name) {
            Some((id, mov)) => (id.clone(), mov.clone()),
            None => return Ok(None),
        };
        self.moves_by_name
            .write()
            .unwrap_or_else(|mut err| {
                **err.get_mut() = HashMap::default();
                self.moves_by_name.clear_poison();
                err.into_inner()
            })
            .insert(name.to_owned(), id);
        Ok(Some(mov))
    }

    fn get_species_by_name(&self, name: &str) -> Result<Option<SpeciesData>> {
        if let Ok(cache) = self.species_by_name.read()
            && let Some(id) = cache.get(name)
        {
            return self.get_species(id);
        }

        let (id, species) = match self
            .species
            .iter()
            .find(|(_, species)| species.name == name)
        {
            Some((id, species)) => (id.clone(), species.clone()),
            None => return Ok(None),
        };
        self.species_by_name
            .write()
            .unwrap_or_else(|mut err| {
                **err.get_mut() = HashMap::default();
                self.species_by_name.clear_poison();
                err.into_inner()
            })
            .insert(name.to_owned(), id);
        Ok(Some(species))
    }
}
