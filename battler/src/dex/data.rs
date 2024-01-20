use std::{
    env,
    fs::File,
    ops::Deref,
    path::Path,
};

use ahash::HashMapExt;
use serde::de::DeserializeOwned;
use zone_alloc::{
    BorrowError,
    KeyedRegistry,
};

use crate::{
    abilities::AbilityData,
    battler_error,
    common::{
        Error,
        FastHashMap,
        Id,
        LookupResult,
        WrapResultError,
    },
    conditions::ConditionData,
    config::ClauseData,
    dex::{
        deserialize_aliases,
        Aliases,
    },
    items::ItemData,
    mons::{
        SpeciesData,
        TypeChart,
    },
    moves::MoveData,
};

/// A user-defined table of resource data of a particular type.
pub type SerializedDataTable<T> = FastHashMap<String, T>;

/// Table for all resource data of a particular type.
pub type DataTable<T> = KeyedRegistry<Id, T>;

/// The result of a data lookup request.
pub type DataLookupResult<T> = LookupResult<T, Error>;

impl<T> From<Result<T, BorrowError>> for DataLookupResult<T> {
    fn from(value: Result<T, BorrowError>) -> Self {
        match value {
            Ok(value) => Self::Found(value),
            Err(BorrowError::OutOfBounds) => Self::NotFound,
            Err(error) => Self::Error(Error::new(error)),
        }
    }
}

/// Collection of tables for all resource data.
///
/// This trait can be implemented for different data sources, such as an external database or disk.
///
/// This collection is used for "raw lookup" of resources by ID. Individual dexes may implement
/// specialized lookup rules over this table, such as resolving aliases or special names.
pub trait DataStore {
    /// Gets the type char.
    fn get_type_chart(&self) -> DataLookupResult<TypeChart>;
    /// Translates the given alias to another ID, if the alias mapping exists.
    fn translate_alias(&self, id: &Id) -> DataLookupResult<Id>;
    /// Gets an ability by ID.
    fn get_ability(&self, id: &Id) -> DataLookupResult<AbilityData>;
    /// Gets a clause by ID.
    fn get_clause(&self, id: &Id) -> DataLookupResult<ClauseData>;
    /// Gets a condition by ID.
    fn get_condition(&self, id: &Id) -> DataLookupResult<ConditionData>;
    /// Gets an item by ID.
    fn get_item(&self, id: &Id) -> DataLookupResult<ItemData>;
    /// Gets a move by ID.
    fn get_move(&self, id: &Id) -> DataLookupResult<MoveData>;
    /// Gets a species by ID.
    fn get_species(&self, id: &Id) -> DataLookupResult<SpeciesData>;
}

/// An implementation of [`DataStore`] that reads all data locally from disk.
pub struct LocalDataStore {
    root: String,
    type_chart: TypeChart,
    aliases: Aliases,
    clauses: DataTable<ClauseData>,
    conditions: DataTable<ConditionData>,
    items: DataTable<ItemData>,
    species: DataTable<SpeciesData>,
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
    pub fn new(root: String) -> Result<Self, Error> {
        if !Path::new(&root).is_dir() {
            return Err(battler_error!(
                "Root directory for LocalDataStore ({root}) does not exist"
            ));
        }
        let mut store = Self {
            root,
            type_chart: TypeChart::new(),
            aliases: Aliases::new(),
            clauses: DataTable::new(),
            conditions: DataTable::new(),
            items: DataTable::new(),
            species: DataTable::new(),
        };
        store.initialize()?;
        Ok(store)
    }

    /// Creates a new instance of [`LocalDataStore`] that reads from the root directory at the given
    /// environment variable.
    pub fn new_from_env(env_var: &str) -> Result<Self, Error> {
        Self::new(env::var(env_var).wrap_error_with_message("DATA_DIR not defined")?)
    }

    fn initialize(&mut self) -> Result<(), Error> {
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
        for (id, clause) in clauses {
            self.clauses.register(id, clause);
        }

        let conditions: FastHashMap<Id, ConditionData> = serde_json::from_reader(
            File::open(Path::new(&self.root).join(Self::CONDITIONS_FILE))
                .wrap_error_with_message("failed to read conditions")?,
        )
        .wrap_error_with_message("failed to parse conditions")?;
        for (id, condition) in conditions {
            self.conditions.register(id, condition);
        }

        self.species = self.read_all_files_in_directory::<SpeciesData>(Self::SPECIES_DIR)?;
        self.items = self.read_all_files_in_directory::<ItemData>(Self::ITEMS_DIR)?;

        Ok(())
    }

    fn read_resource<T: DeserializeOwned>(&self, dir: &str, id: &Id) -> DataLookupResult<T> {
        match File::open(Path::new(&self.root).join(dir).join(format!("{id}.json"))) {
            Err(_) => DataLookupResult::NotFound,
            Ok(reader) => serde_json::from_reader(reader)
                .wrap_error_with_format(format_args!("found {id}, but data was invalid"))
                .into(),
        }
    }

    fn read_all_files_in_directory<T: DeserializeOwned>(
        &self,
        dir: &str,
    ) -> Result<DataTable<T>, Error> {
        let tables = Path::new(&self.root)
            .join(dir)
            .read_dir()
            .wrap_error_with_format(format_args!("failed to read {dir} directory"))?
            .filter_map(|entry| Some(entry.ok()?.path()))
            .filter(|path| path.is_file())
            .map(|path| {
                let path_name = path.to_string_lossy().to_string();
                match serde_json::from_reader::<File, SerializedDataTable<T>>(
                    File::open(path)
                        .wrap_error_with_format(format_args!("{path_name} could not be opened"))?,
                ) {
                    Err(err) => Err(battler_error!(
                        "failed to read {dir} data from {path_name}: {err}"
                    )),
                    Ok(table) => Ok(table),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let registry = KeyedRegistry::new();
        registry.register_extend(
            tables
                .into_iter()
                .map(|table| table.into_iter())
                .flatten()
                .map(|(key, value)| (Id::from(key), value)),
        );
        Ok(registry)
    }
}

impl DataStore for LocalDataStore {
    fn get_type_chart(&self) -> DataLookupResult<TypeChart> {
        DataLookupResult::Found(self.type_chart.clone())
    }

    fn translate_alias(&self, id: &Id) -> DataLookupResult<Id> {
        self.aliases.get(id).cloned().into()
    }

    fn get_ability(&self, id: &Id) -> DataLookupResult<AbilityData> {
        self.read_resource(Self::ABILITIES_DIR, id)
    }

    fn get_clause(&self, id: &Id) -> DataLookupResult<ClauseData> {
        self.clauses.get(id).map(|data| data.deref().clone()).into()
    }

    fn get_condition(&self, id: &Id) -> DataLookupResult<ConditionData> {
        self.conditions
            .get(id)
            .map(|data| data.deref().clone())
            .into()
    }

    fn get_item(&self, id: &Id) -> DataLookupResult<ItemData> {
        self.items.get(id).map(|data| data.deref().clone()).into()
    }

    fn get_move(&self, id: &Id) -> DataLookupResult<MoveData> {
        self.read_resource(Self::MOVES_DIR, id)
    }

    fn get_species(&self, id: &Id) -> DataLookupResult<SpeciesData> {
        self.species.get(id).map(|data| data.deref().clone()).into()
    }
}

#[cfg(test)]
pub mod fake_data_store {
    use std::ops::Deref;

    use ahash::HashMapExt;

    use crate::{
        abilities::AbilityData,
        common::Id,
        conditions::ConditionData,
        config::ClauseData,
        dex::{
            Aliases,
            DataLookupResult,
            DataStore,
            DataTable,
        },
        items::ItemData,
        mons::{
            SpeciesData,
            TypeChart,
        },
        moves::MoveData,
    };

    /// A fake implementation of [`DataStore`] used for unit testing.
    pub struct FakeDataStore {
        pub type_chart: TypeChart,
        pub aliases: Aliases,
        pub abilities: DataTable<AbilityData>,
        pub clauses: DataTable<ClauseData>,
        pub conditions: DataTable<ConditionData>,
        pub items: DataTable<ItemData>,
        pub moves: DataTable<MoveData>,
        pub species: DataTable<SpeciesData>,
    }

    impl FakeDataStore {
        pub fn new() -> Self {
            Self {
                type_chart: TypeChart::new(),
                aliases: Aliases::new(),
                abilities: DataTable::new(),
                clauses: DataTable::new(),
                conditions: DataTable::new(),
                items: DataTable::new(),
                moves: DataTable::new(),
                species: DataTable::new(),
            }
        }
    }

    impl DataStore for FakeDataStore {
        fn get_type_chart(&self) -> DataLookupResult<TypeChart> {
            DataLookupResult::Found(self.type_chart.clone())
        }

        fn translate_alias(&self, id: &Id) -> DataLookupResult<Id> {
            self.aliases.get(id).cloned().into()
        }

        fn get_ability(&self, id: &Id) -> DataLookupResult<AbilityData> {
            self.abilities
                .get(id)
                .map(|data| data.deref().clone())
                .into()
        }

        fn get_clause(&self, id: &Id) -> DataLookupResult<ClauseData> {
            self.clauses.get(id).map(|data| data.deref().clone()).into()
        }

        fn get_condition(&self, id: &Id) -> DataLookupResult<ConditionData> {
            self.conditions
                .get(id)
                .map(|data| data.deref().clone())
                .into()
        }

        fn get_item(&self, id: &Id) -> DataLookupResult<ItemData> {
            self.items.get(id).map(|data| data.deref().clone()).into()
        }

        fn get_move(&self, id: &Id) -> DataLookupResult<MoveData> {
            self.moves.get(id).map(|data| data.deref().clone()).into()
        }

        fn get_species(&self, id: &Id) -> DataLookupResult<SpeciesData> {
            self.species.get(id).map(|data| data.deref().clone()).into()
        }
    }
}
