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
use battler_data::{
    AbilityData,
    ClauseData,
    ConditionData,
    DataStore,
    DataStoreByName,
    Id,
    ItemData,
    MoveData,
    PartitionedAliases,
    ResourceType,
    SpeciesData,
    TypeChart,
    deserialize_aliases,
};
use battler_data_service_schema::{
    DescriptionData,
    DescriptionStore,
    ResourceType as ServiceResourceType,
};
use serde::de::DeserializeOwned;

/// An implementation of [`DataStore`] that reads all data locally from disk.
pub struct LocalDataStore {
    pub type_chart: TypeChart,
    pub abilities: HashMap<Id, AbilityData>,
    pub aliases: PartitionedAliases,
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
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref();
        if !root.is_dir() {
            return Err(Error::msg(format!(
                "Root directory for LocalDataStore ({}) does not exist",
                root.display()
            )));
        }

        let type_chart = serde_json::from_reader(
            File::open(root.join(Self::TYPE_CHART_FILE)).context("failed to read type chart")?,
        )
        .context("failed to parse type chart")?;

        let aliases = deserialize_aliases(
            serde_json::from_reader(
                File::open(root.join(Self::ALIASES_FILE)).context("failed to read aliases")?,
            )
            .context("failed to parse aliases")?,
        );

        let clauses: HashMap<Id, ClauseData> = serde_json::from_reader(
            File::open(root.join(Self::CLAUSES_FILE)).context("failed to read clauses")?,
        )
        .context("failed to parse clauses")?;

        let mut conditions: HashMap<Id, ConditionData> = serde_json::from_reader(
            File::open(root.join(Self::CONDITIONS_FILE)).context("failed to read conditions")?,
        )
        .context("failed to parse conditions")?;

        for base_file in [
            Self::MOVE_BASES_FILE,
            Self::ABILITY_BASES_FILE,
            Self::ITEM_BASES_FILE,
        ] {
            let bases: HashMap<Id, ConditionData> = serde_json::from_reader(
                File::open(root.join(base_file)).context(format!("failed to read {base_file}"))?,
            )
            .context(format!("failed to parse {base_file}"))?;
            conditions.extend(bases);
        }

        Ok(Self {
            type_chart,
            aliases,
            clauses,
            conditions,
            abilities: Self::read_all_files_in_directory(root, Self::ABILITIES_DIR)?,
            items: Self::read_all_files_in_directory(root, Self::ITEMS_DIR)?,
            moves: Self::read_all_files_in_directory(root, Self::MOVES_DIR)?,
            species: Self::read_all_files_in_directory(root, Self::SPECIES_DIR)?,
            abilities_by_name: RwLock::new(HashMap::default()),
            clauses_by_name: RwLock::new(HashMap::default()),
            conditions_by_name: RwLock::new(HashMap::default()),
            items_by_name: RwLock::new(HashMap::default()),
            moves_by_name: RwLock::new(HashMap::default()),
            species_by_name: RwLock::new(HashMap::default()),
        })
    }

    /// Creates a new instance of [`LocalDataStore`] that reads from the root directory at the given
    /// environment variable.
    pub fn new_from_env(env_var: &str) -> Result<Self> {
        Self::new(env::var(env_var).context(format!("{env_var} not defined"))?)
    }

    fn read_all_files_in_directory<T: DeserializeOwned>(
        root: &Path,
        dir: &str,
    ) -> Result<HashMap<Id, T>> {
        let tables = root
            .join(dir)
            .read_dir()
            .context(format!("failed to read {dir} directory"))?
            .filter_map(|entry| Some(entry.ok()?.path()))
            .filter(|path| path.is_file())
            .map(|path| {
                let path_name = path.to_string_lossy().to_string();
                serde_json::from_reader::<File, HashMap<String, T>>(
                    File::open(&path).context(format!("{path_name} could not be opened"))?,
                )
                .context(format!("failed to read {dir} data from {path_name}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tables
            .into_iter()
            .flatten()
            .map(|(key, value)| (Id::from(key), value))
            .collect())
    }
}

fn filter_ids<T>(map: &HashMap<Id, T>, filter: &dyn Fn(&T) -> bool) -> Vec<Id> {
    map.iter()
        .filter(|(_, data)| filter(data))
        .map(|(id, _)| id.clone())
        .collect()
}

impl DataStore for LocalDataStore {
    fn all_ability_ids(&self, filter: &dyn Fn(&AbilityData) -> bool) -> Result<Vec<Id>> {
        Ok(filter_ids(&self.abilities, filter))
    }

    fn all_item_ids(&self, filter: &dyn Fn(&ItemData) -> bool) -> Result<Vec<Id>> {
        Ok(filter_ids(&self.items, filter))
    }

    fn all_move_ids(&self, filter: &dyn Fn(&MoveData) -> bool) -> Result<Vec<Id>> {
        Ok(filter_ids(&self.moves, filter))
    }

    fn all_species_ids(&self, filter: &dyn Fn(&SpeciesData) -> bool) -> Result<Vec<Id>> {
        Ok(filter_ids(&self.species, filter))
    }

    fn get_type_chart(&self) -> Result<TypeChart> {
        Ok(self.type_chart.clone())
    }

    fn translate_alias(&self, resource_type: ResourceType, id: &Id) -> Result<Option<Id>> {
        Ok(self.aliases.get(resource_type, id).cloned())
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

fn get_by_name<T: Clone>(
    cache: &RwLock<HashMap<String, Id>>,
    map: &HashMap<Id, T>,
    name: &str,
    get_name: impl Fn(&T) -> &str,
) -> Option<T> {
    if let Ok(c) = cache.read()
        && let Some(id) = c.get(name)
    {
        return map.get(id).cloned();
    }

    let (id, val) = map.iter().find(|(_, val)| get_name(val) == name)?;
    cache
        .write()
        .unwrap_or_else(|mut err| {
            **err.get_mut() = HashMap::default();
            cache.clear_poison();
            err.into_inner()
        })
        .insert(name.to_owned(), id.clone());
    Some(val.clone())
}

impl DataStoreByName for LocalDataStore {
    fn get_ability_by_name(&self, name: &str) -> Result<Option<AbilityData>> {
        Ok(get_by_name(
            &self.abilities_by_name,
            &self.abilities,
            name,
            |a| &a.name,
        ))
    }

    fn get_clause_by_name(&self, name: &str) -> Result<Option<ClauseData>> {
        Ok(get_by_name(
            &self.clauses_by_name,
            &self.clauses,
            name,
            |c| &c.name,
        ))
    }

    fn get_condition_by_name(&self, name: &str) -> Result<Option<ConditionData>> {
        Ok(get_by_name(
            &self.conditions_by_name,
            &self.conditions,
            name,
            |c| &c.name,
        ))
    }

    fn get_item_by_name(&self, name: &str) -> Result<Option<ItemData>> {
        Ok(get_by_name(&self.items_by_name, &self.items, name, |i| {
            &i.name
        }))
    }

    fn get_move_by_name(&self, name: &str) -> Result<Option<MoveData>> {
        Ok(get_by_name(&self.moves_by_name, &self.moves, name, |m| {
            &m.name
        }))
    }

    fn get_species_by_name(&self, name: &str) -> Result<Option<SpeciesData>> {
        Ok(get_by_name(
            &self.species_by_name,
            &self.species,
            name,
            |s| &s.name,
        ))
    }
}

/// An implementation of [`DescriptionStore`] that reads all descriptions locally from disk.
pub struct LocalDescriptionStore {
    pub moves: HashMap<Id, DescriptionData>,
    pub abilities: HashMap<Id, DescriptionData>,
    pub items: HashMap<Id, DescriptionData>,
    pub species: HashMap<Id, DescriptionData>,
    pub conditions: HashMap<Id, DescriptionData>,
}

impl LocalDescriptionStore {
    /// Moves descriptions file name.
    pub const MOVES_FILE: &str = "moves.json";
    /// Abilities descriptions file name.
    pub const ABILITIES_FILE: &str = "abilities.json";
    /// Items descriptions file name.
    pub const ITEMS_FILE: &str = "items.json";
    /// Species descriptions file name.
    pub const SPECIES_FILE: &str = "species.json";
    /// Conditions descriptions file name.
    pub const CONDITIONS_FILE: &str = "conditions.json";

    /// Creates a new instance of [`LocalDescriptionStore`] that reads from the given root
    /// directory.
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref();
        if !root.is_dir() {
            return Err(Error::msg(format!(
                "Root directory for LocalDescriptionStore ({}) does not exist",
                root.display()
            )));
        }
        Ok(Self {
            moves: Self::read_file(root, Self::MOVES_FILE)?,
            abilities: Self::read_file(root, Self::ABILITIES_FILE)?,
            items: Self::read_file(root, Self::ITEMS_FILE)?,
            species: Self::read_file(root, Self::SPECIES_FILE)?,
            conditions: Self::read_file(root, Self::CONDITIONS_FILE)?,
        })
    }

    /// Creates a new instance of [`LocalDescriptionStore`] that reads from the root directory at
    /// the given environment variable.
    pub fn new_from_env(env_var: &str) -> Result<Self> {
        Self::new(env::var(env_var).context(format!("{env_var} not defined"))?)
    }

    fn read_file(root: &Path, file: &str) -> Result<HashMap<Id, DescriptionData>> {
        let path = root.join(file);
        if !path.exists() {
            return Ok(HashMap::default());
        }
        let file_handle =
            File::open(&path).context(format!("failed to read {}", path.display()))?;
        serde_json::from_reader(file_handle).context(format!("failed to parse {}", path.display()))
    }
}

impl DescriptionStore for LocalDescriptionStore {
    fn get_description(
        &self,
        resource_type: ServiceResourceType,
        id: &Id,
    ) -> Result<Option<DescriptionData>> {
        let entry = match resource_type {
            ServiceResourceType::Move => self.moves.get(id),
            ServiceResourceType::Ability => self.abilities.get(id),
            ServiceResourceType::Item => self.items.get(id),
            ServiceResourceType::Species => self.species.get(id),
            ServiceResourceType::Condition => self.conditions.get(id),
        };
        Ok(entry.cloned())
    }
}

#[cfg(test)]
mod tests {
    use battler_data::{
        DataStore,
        Id,
        ResourceType,
    };
    use battler_data_service_schema::{
        DescriptionStore,
        ResourceType as ServiceResourceType,
    };

    use super::{
        LocalDataStore,
        LocalDescriptionStore,
    };

    #[test]
    fn translates_partitioned_aliases() {
        let store = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        // Species alias
        assert_eq!(
            store
                .translate_alias(ResourceType::Species, &Id::from("alcremierainbowswirl"))
                .unwrap(),
            Some(Id::from("alcremie"))
        );

        // Item alias
        assert_eq!(
            store
                .translate_alias(ResourceType::Item, &Id::from("healthfeather"))
                .unwrap(),
            Some(Id::from("healthwing"))
        );

        // Condition alias
        assert_eq!(
            store
                .translate_alias(ResourceType::Condition, &Id::from("burn"))
                .unwrap(),
            Some(Id::from("brn"))
        );
        assert_eq!(
            store
                .translate_alias(ResourceType::Condition, &Id::from("badpoison"))
                .unwrap(),
            Some(Id::from("tox"))
        );

        // Prove partition isolation: move "burn" does NOT resolve to condition alias "brn"
        assert_eq!(
            store
                .translate_alias(ResourceType::Move, &Id::from("burn"))
                .unwrap(),
            None
        );
    }

    #[test]
    fn loads_descriptions_and_verifies_coverage() {
        let data_store = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let desc_store = LocalDescriptionStore::new_from_env("DESCRIPTIONS_DIR").unwrap();

        // Check specific resources
        let tackle = desc_store
            .get_description(ServiceResourceType::Move, &Id::from("tackle"))
            .unwrap()
            .expect("tackle description missing");
        assert!(!tackle.description.is_empty());

        let leftovers = desc_store
            .get_description(ServiceResourceType::Item, &Id::from("leftovers"))
            .unwrap()
            .expect("leftovers description missing");
        assert_eq!(leftovers.source, "Scarlet / Violet");

        let pikachu = desc_store
            .get_description(ServiceResourceType::Species, &Id::from("pikachu"))
            .unwrap()
            .expect("pikachu description missing");
        assert_eq!(pikachu.source, "Scarlet");

        let par = desc_store
            .get_description(ServiceResourceType::Condition, &Id::from("par"))
            .unwrap()
            .expect("par description missing");
        assert!(!par.description.is_empty());

        // Verify 100% species coverage across authentic ROM dumps
        for id in data_store.species.keys() {
            assert!(
                desc_store.species.contains_key(id),
                "missing description for species {id}"
            );
        }

        // Verify representative species across all 5 games in cascade
        let charizard = desc_store
            .get_description(ServiceResourceType::Species, &Id::from("charizard"))
            .unwrap()
            .expect("charizard description missing");
        assert_eq!(charizard.source, "Scarlet");

        let mewtwo = desc_store
            .get_description(ServiceResourceType::Species, &Id::from("mewtwo"))
            .unwrap()
            .expect("mewtwo description missing");
        assert_eq!(mewtwo.source, "Brilliant Diamond");

        let zacian = desc_store
            .get_description(ServiceResourceType::Species, &Id::from("zacian"))
            .unwrap()
            .expect("zacian description missing");
        assert_eq!(zacian.source, "Sword");

        let tapukoko = desc_store
            .get_description(ServiceResourceType::Species, &Id::from("tapukoko"))
            .unwrap()
            .expect("tapukoko description missing");
        assert_eq!(tapukoko.source, "Ultra Sun");

        let volcanion = desc_store
            .get_description(ServiceResourceType::Species, &Id::from("volcanion"))
            .unwrap()
            .expect("volcanion description missing");
        assert_eq!(volcanion.source, "Omega Ruby");

        // Verify LGPE move
        let zippyzap = desc_store
            .get_description(ServiceResourceType::Move, &Id::from("zippyzap"))
            .unwrap()
            .expect("zippyzap description missing");
        assert_eq!(zippyzap.source, "Let’s Go, Pikachu! / Let’s Go, Eevee!");

        // Verify moves coverage (except internal pseudomoves)
        let excluded_moves = ["recharge", "pass"];
        for id in data_store.moves.keys() {
            if excluded_moves.contains(&id.as_ref()) {
                continue;
            }
            assert!(
                desc_store.moves.contains_key(id),
                "missing description for move {id}"
            );
        }

        // Verify abilities and items coverage
        assert!(desc_store.abilities.len() >= 300);
        assert!(desc_store.items.len() >= 1700);
    }
}
