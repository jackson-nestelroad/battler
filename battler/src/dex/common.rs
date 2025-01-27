use std::marker::PhantomData;

use zone_alloc::{
    BorrowError,
    ElementRef,
};

use crate::{
    common::Id,
    dex::{
        DataStore,
        DataTable,
    },
    error::{
        general_error,
        ConvertError,
        Error,
        NotFoundError,
    },
};

/// Trait for implementing custom logic for looking up and creating a resource instance by ID.
///
/// Lookup methods are only called once for a given input ID. Afterwards, the created resource
/// instance is cached for future lookups.
pub trait ResourceLookup<'d, T> {
    /// Creates a new instance of the [`ResourceLookup`] implementation.
    ///
    /// The lookup instance can store the [`DataStore`] reference for looking up data.
    fn new(data: &'d dyn DataStore) -> Self;

    /// Looks up a resource by ID.
    ///
    /// The ID is guaranteed to not be an alias.
    fn lookup(&self, id: &Id) -> Result<T, Error>;

    /// Looks up a resource by alias and its real ID.
    ///
    /// `alias` is the original input. `real_id` is the end of the alias chain, as defined in
    /// [`DataStore`].
    fn lookup_alias(&self, _alias: &Id, real_id: &Id) -> Result<T, Error> {
        self.lookup(real_id)
    }
}

/// Trait for wrapping a resource data type to create a resource instance.
pub trait ResourceWrapper<D, T> {
    /// Wraps the given resource data in a resource instance type.
    fn wrap(id: Id, data: D) -> T;
}

/// A resource cache that can be used internally by [`Dex`][`crate::dex::Dex`] for caching resource
/// instances.
pub struct ResourceCache<T> {
    cache: DataTable<T>,
}

impl<T> ResourceCache<T> {
    /// Creates a new resource cache.
    pub fn new() -> Self {
        Self {
            cache: DataTable::new(),
        }
    }

    /// Checks if the given ID is cached.
    pub fn is_cached(&self, id: &Id) -> bool {
        self.cache.contains_key(id)
    }

    /// Gets the data for a cached ID.
    pub fn get(&self, id: &Id) -> Result<ElementRef<T>, BorrowError> {
        self.cache.get(id)
    }

    /// Caches the given reference for future lookups.
    pub fn save(&self, id: &Id, data: T) -> bool {
        self.cache.register(id.clone(), data)
    }
}

/// A collection of resources indexed by ID.
pub struct ResourceDex<'d, D, T, L, W> {
    data: &'d dyn DataStore,
    /// Cache of resource instances, so each ID is only looked up once.
    cache: ResourceCache<T>,
    lookup: L,
    phantom_data: PhantomData<D>,
    phantom_wrapper: PhantomData<W>,
}

impl<'d, D, T, L, W> ResourceDex<'d, D, T, L, W>
where
    L: ResourceLookup<'d, D>,
    W: ResourceWrapper<D, T>,
{
    /// Creates a new collection of resources.
    pub fn new(data: &'d dyn DataStore) -> Self {
        Self {
            data,
            cache: ResourceCache::new().into(),
            lookup: L::new(data),
            phantom_data: PhantomData,
            phantom_wrapper: PhantomData,
        }
    }

    fn cache_data(&self, id: &Id) -> Result<(), Error> {
        let data = self.lookup_data_by_id(id)?;
        let resource = W::wrap(id.clone(), data);
        if !self.cache.save(id, resource) {
            Err(general_error(format!(
                "failed to save data for {id} in cache"
            )))
        } else {
            Ok(())
        }
    }

    /// Retrieves a resource by name.
    pub fn get(&self, name: &str) -> Result<ElementRef<T>, Error> {
        self.get_by_id(&Id::from(name))
    }

    /// Retrieves a resource by ID.
    pub fn get_by_id(&self, id: &Id) -> Result<ElementRef<T>, Error> {
        let id = self.resolve_alias(id.clone())?;
        // The borrow checker struggles if we use pattern matching here, so we have to do two
        // lookups.
        if self.cache.is_cached(&id) {
            return self
                .cache
                .get(&id)
                .map_err(|err| err.convert_error_with_message(format!("cached resource {id}")));
        }
        self.cache_data(&id)?;
        self.cache
            .get(&id)
            .map_err(|err| err.convert_error_with_message(format!("cached resource {id}")))
    }

    fn resolve_alias(&self, mut id: Id) -> Result<Id, Error> {
        loop {
            match self.data.translate_alias(&id) {
                Ok(alias) => id = alias,
                Err(error) => {
                    if error.as_ref().is::<NotFoundError>() {
                        return Ok(id);
                    } else {
                        return Err(error);
                    }
                }
            }
        }
    }

    /// Looks up a resource by ID using the internal [`ResourceLookup`] implementation.
    fn lookup_data_by_id(&self, id: &Id) -> Result<D, Error> {
        self.lookup.lookup(&id)
    }
}

pub struct SingleValueDex<'d, T> {
    #[allow(unused)]
    data: &'d dyn DataStore,
    value: T,
}

impl<'d, T> SingleValueDex<'d, T> {
    /// Creates a new single value dex, wrapping the given value.
    pub fn new(data: &'d dyn DataStore, value: T) -> Self {
        Self { data, value }
    }

    /// Retrieves the inner resource.
    pub fn get(&self) -> &T {
        &self.value
    }
}

#[cfg(test)]
mod resource_cache_tests {
    use crate::{
        common::Id,
        dex::ResourceCache,
    };

    #[derive(Debug, Clone, PartialEq)]
    struct Data {
        number: i32,
    }

    #[test]
    fn caches_resources() {
        let cache = ResourceCache::<Data>::new();
        let id = Id::from("first");
        assert!(!cache.is_cached(&id));
        cache.save(&id, Data { number: 123 });
        assert!(cache.is_cached(&id));
    }

    #[test]
    fn gets_reference_to_cached_resource() {
        let cache = ResourceCache::<Data>::new();
        let id = Id::from("first");
        let data = Data { number: 123 };
        let inserted = cache.save(&id, data.clone());
        assert!(inserted);
        let fetched = cache.get(&id);
        assert_eq!(fetched.unwrap().number, 123);
    }
}

#[cfg(test)]
mod dex_tests {
    use std::{
        cell::RefCell,
        ops::Deref,
    };

    use ahash::HashMapExt;
    use rand::random;

    use crate::{
        common::{
            FastHashMap,
            Id,
        },
        dex::{
            DataStore,
            FakeDataStore,
            ResourceDex,
            ResourceLookup,
            ResourceWrapper,
        },
        error::Error,
    };

    #[derive(Debug, Clone, PartialEq)]
    struct TestData {
        id: Id,
        numeric_id: u64,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TestResource {
        id: Id,
        data: TestData,
    }

    struct TestDataLookup {
        lookup_calls: RefCell<FastHashMap<Id, u64>>,
    }

    impl<'d> ResourceLookup<'d, TestData> for TestDataLookup {
        fn new(_: &'d dyn DataStore) -> Self {
            Self {
                lookup_calls: RefCell::new(FastHashMap::new()),
            }
        }

        fn lookup(&self, id: &Id) -> Result<TestData, Error> {
            *self
                .lookup_calls
                .borrow_mut()
                .entry(id.clone())
                .or_default() += 1;
            Ok(TestData {
                id: id.clone(),
                numeric_id: random(),
            })
        }
    }

    struct TestResourceWrapper;

    impl ResourceWrapper<TestData, TestResource> for TestResourceWrapper {
        fn wrap(id: Id, data: TestData) -> TestResource {
            TestResource { data, id }
        }
    }

    type TestDex<'d> = ResourceDex<'d, TestData, TestResource, TestDataLookup, TestResourceWrapper>;

    #[test]
    fn finds_and_caches_resource() {
        let data = FakeDataStore::new();
        let dex = TestDex::new(&data);
        let first_resource = dex.get("first").unwrap();
        let second_resource = dex.get_by_id(&Id::from("first")).unwrap();
        // Random integers should be the same.
        assert_eq!(first_resource.deref(), second_resource.deref());
        // Only a single lookup occurred.
        assert_eq!(
            *dex.lookup.lookup_calls.borrow(),
            FastHashMap::from_iter([(Id::from("first"), 1)])
        );
    }

    #[test]
    fn resolves_alias() {
        let mut data = FakeDataStore::new();
        data.aliases.insert(Id::from("alias3"), Id::from("alias2"));
        data.aliases.insert(Id::from("alias2"), Id::from("alias1"));
        data.aliases.insert(Id::from("alias1"), Id::from("native"));
        let dex = TestDex::new(&data);
        let a = dex.get("alias3");
        let b = dex.get("alias3");
        let c = dex.get("alias1");
        let d = dex.get("native");
        assert_eq!(a.unwrap().deref(), b.as_ref().unwrap().deref());
        assert_eq!(b.unwrap().data.id, c.as_ref().unwrap().data.id);
        assert_eq!(c.unwrap().data.id, d.unwrap().data.id);
        // Only a single lookup occurred for the resolved alias.
        assert_eq!(
            *dex.lookup.lookup_calls.borrow(),
            FastHashMap::from_iter([(Id::from("native"), 1)])
        );
    }
}
