use std::{
    env,
    fs::File,
    path::Path,
};

use ahash::HashMapExt;
use anyhow::Result;
use serde::de::DeserializeOwned;

use crate::{
    abilities::AbilityData,
    common::{
        FastHashMap,
        Id,
    },
    conditions::ConditionData,
    config::ClauseData,
    dex::{
        deserialize_aliases,
        Aliases,
    },
    error::{
        general_error,
        WrapOptionError,
        WrapResultError,
    },
    items::ItemData,
    mons::{
        SpeciesData,
        TypeChart,
    },
    moves::MoveData,
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
    fn translate_alias(&self, id: &Id) -> Result<Id>;
    /// Gets an ability by ID.
    fn get_ability(&self, id: &Id) -> Result<AbilityData>;
    /// Gets a clause by ID.
    fn get_clause(&self, id: &Id) -> Result<ClauseData>;
    /// Gets a condition by ID.
    fn get_condition(&self, id: &Id) -> Result<ConditionData>;
    /// Gets an item by ID.
    fn get_item(&self, id: &Id) -> Result<ItemData>;
    /// Gets a move by ID.
    fn get_move(&self, id: &Id) -> Result<MoveData>;
    /// Gets a species by ID.
    fn get_species(&self, id: &Id) -> Result<SpeciesData>;
}

/// An implementation of [`DataStore`] that reads all data locally from disk.
pub struct LocalDataStore {
    root: String,
    type_chart: TypeChart,
    abilities: FastHashMap<Id, AbilityData>,
    aliases: Aliases,
    clauses: FastHashMap<Id, ClauseData>,
    conditions: FastHashMap<Id, ConditionData>,
    items: FastHashMap<Id, ItemData>,
    moves: FastHashMap<Id, MoveData>,
    species: FastHashMap<Id, SpeciesData>,
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
            return Err(general_error(format!(
                "Root directory for LocalDataStore ({root}) does not exist",
            )));
        }
        let mut store = Self {
            root,
            type_chart: TypeChart::new(),
            abilities: FastHashMap::new(),
            aliases: Aliases::new(),
            clauses: FastHashMap::new(),
            conditions: FastHashMap::new(),
            items: FastHashMap::new(),
            moves: FastHashMap::new(),
            species: FastHashMap::new(),
        };
        store.initialize()?;
        Ok(store)
    }

    /// Creates a new instance of [`LocalDataStore`] that reads from the root directory at the given
    /// environment variable.
    pub fn new_from_env(env_var: &str) -> Result<Self> {
        Self::new(env::var(env_var).wrap_error_with_message("DATA_DIR not defined")?)
    }

    fn initialize(&mut self) -> Result<()> {
        self.type_chart = serde_json::from_reader(
            File::open(Path::new(&self.root).join(Self::TYPE_CHART_FILE))
                .wrap_error_with_message("failed to read type chart")?,
        )
        .wrap_error_with_message("failed to parse type chart")?;

        self.aliases = deserialize_aliases(
            serde_json::from_reader(
                File::open(Path::new(&self.root).join(Self::ALIASES_FILE))
                    .wrap_error_with_message("failed to read type chart")?,
            )
            .wrap_error_with_message("failed to parse type chart")?,
        );

        let clauses: FastHashMap<Id, ClauseData> = serde_json::from_reader(
            File::open(Path::new(&self.root).join(Self::CLAUSES_FILE))
                .wrap_error_with_message("failed to read clauses")?,
        )
        .wrap_error_with_message("failed to parse clauses")?;
        self.clauses.extend(clauses);

        let conditions: FastHashMap<Id, ConditionData> = serde_json::from_reader(
            File::open(Path::new(&self.root).join(Self::CONDITIONS_FILE))
                .wrap_error_with_message("failed to read conditions")?,
        )
        .wrap_error_with_message("failed to parse conditions")?;
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
    ) -> Result<FastHashMap<Id, T>> {
        let tables = Path::new(&self.root)
            .join(dir)
            .read_dir()
            .wrap_error_with_format(format_args!("failed to read {dir} directory"))?
            .filter_map(|entry| Some(entry.ok()?.path()))
            .filter(|path| path.is_file())
            .map(|path| {
                let path_name = path.to_string_lossy().to_string();
                serde_json::from_reader::<File, FastHashMap<String, T>>(
                    File::open(path)
                        .wrap_error_with_format(format_args!("{path_name} could not be opened"))?,
                )
                .wrap_error_with_format(format_args!("failed to read {dir} data from {path_name}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut map = FastHashMap::new();
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

    fn translate_alias(&self, id: &Id) -> Result<Id> {
        self.aliases
            .get(id)
            .cloned()
            .wrap_not_found_error_with_format(format_args!("alias {id}"))
    }

    fn get_ability(&self, id: &Id) -> Result<AbilityData> {
        self.abilities
            .get(id)
            .cloned()
            .wrap_not_found_error_with_format(format_args!("ability {id}"))
    }

    fn get_clause(&self, id: &Id) -> Result<ClauseData> {
        self.clauses
            .get(id)
            .cloned()
            .wrap_not_found_error_with_format(format_args!("clause {id}"))
    }

    fn get_condition(&self, id: &Id) -> Result<ConditionData> {
        self.conditions
            .get(id)
            .cloned()
            .wrap_not_found_error_with_format(format_args!("condition {id}"))
    }

    fn get_item(&self, id: &Id) -> Result<ItemData> {
        self.items
            .get(id)
            .cloned()
            .wrap_not_found_error_with_format(format_args!("item {id}"))
    }

    fn get_move(&self, id: &Id) -> Result<MoveData> {
        self.moves
            .get(id)
            .cloned()
            .wrap_not_found_error_with_format(format_args!("move {id}"))
    }

    fn get_species(&self, id: &Id) -> Result<SpeciesData> {
        self.species
            .get(id)
            .cloned()
            .wrap_not_found_error_with_format(format_args!("species {id}"))
    }
}

#[cfg(test)]
pub mod fake_data_store {

    use ahash::HashMapExt;
    use anyhow::Result;

    use crate::{
        abilities::AbilityData,
        common::Id,
        conditions::ConditionData,
        config::ClauseData,
        dex::{
            Aliases,
            DataStore,
        },
        error::WrapOptionError,
        items::ItemData,
        mons::{
            SpeciesData,
            TypeChart,
        },
        moves::MoveData,
        FastHashMap,
    };

    /// A fake implementation of [`DataStore`] used for unit testing.
    pub struct FakeDataStore {
        pub type_chart: TypeChart,
        pub aliases: Aliases,
        pub abilities: FastHashMap<Id, AbilityData>,
        pub clauses: FastHashMap<Id, ClauseData>,
        pub conditions: FastHashMap<Id, ConditionData>,
        pub items: FastHashMap<Id, ItemData>,
        pub moves: FastHashMap<Id, MoveData>,
        pub species: FastHashMap<Id, SpeciesData>,
    }

    impl FakeDataStore {
        pub fn new() -> Self {
            Self {
                type_chart: TypeChart::new(),
                aliases: Aliases::new(),
                abilities: FastHashMap::new(),
                clauses: FastHashMap::new(),
                conditions: FastHashMap::new(),
                items: FastHashMap::new(),
                moves: FastHashMap::new(),
                species: FastHashMap::new(),
            }
        }
    }

    impl DataStore for FakeDataStore {
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

        fn translate_alias(&self, id: &Id) -> Result<Id> {
            self.aliases
                .get(id)
                .cloned()
                .wrap_not_found_error_with_format(format_args!("alias {id}"))
        }

        fn get_ability(&self, id: &Id) -> Result<AbilityData> {
            self.abilities
                .get(id)
                .cloned()
                .wrap_not_found_error_with_format(format_args!("ability {id}"))
        }

        fn get_clause(&self, id: &Id) -> Result<ClauseData> {
            self.clauses
                .get(id)
                .cloned()
                .wrap_not_found_error_with_format(format_args!("clause {id}"))
        }

        fn get_condition(&self, id: &Id) -> Result<ConditionData> {
            self.conditions
                .get(id)
                .cloned()
                .wrap_not_found_error_with_format(format_args!("condition {id}"))
        }

        fn get_item(&self, id: &Id) -> Result<ItemData> {
            self.items
                .get(id)
                .cloned()
                .wrap_not_found_error_with_format(format_args!("item {id}"))
        }

        fn get_move(&self, id: &Id) -> Result<MoveData> {
            self.moves
                .get(id)
                .cloned()
                .wrap_not_found_error_with_format(format_args!("move {id}"))
        }

        fn get_species(&self, id: &Id) -> Result<SpeciesData> {
            self.species
                .get(id)
                .cloned()
                .wrap_not_found_error_with_format(format_args!("species {id}"))
        }
    }
}
