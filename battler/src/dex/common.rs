use alloc::format;
use core::marker::PhantomData;

use anyhow::Result;
use battler_data::{
    DataStore,
    Id,
};
use zone_alloc::{
    BorrowError,
    ElementRef,
    KeyedRegistry,
};

use crate::error::{
    ConvertError,
    general_error,
};

type DataTable<T> = KeyedRegistry<Id, T>;

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
    fn lookup(&self, id: &Id) -> Result<T>;

    /// Looks up a resource by alias and its real ID.
    ///
    /// `alias` is the original input. `real_id` is the end of the alias chain, as defined in
    /// [`DataStore`].
    fn lookup_alias(&self, _alias: &Id, real_id: &Id) -> Result<T> {
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
    pub fn get(&self, id: &Id) -> Result<ElementRef<'_, T>, BorrowError> {
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

    fn cache_data(&self, id: &Id, real_id: &Id) -> Result<()> {
        let data = self.lookup_data_by_id(id, real_id)?;
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
    pub fn get(&self, name: &str) -> Result<ElementRef<'_, T>> {
        self.get_by_id(&Id::from(name))
    }

    /// Retrieves a resource by ID.
    pub fn get_by_id(&self, id: &Id) -> Result<ElementRef<'_, T>> {
        let real_id = self.resolve_alias(id.clone())?;
        // The borrow checker struggles if we use pattern matching here, so we have to do two
        // lookups.
        if self.cache.is_cached(id) {
            return self
                .cache
                .get(id)
                .map_err(|err| err.convert_error_with_message(format!("cached resource {id}")));
        }
        self.cache_data(id, &real_id)?;
        self.cache
            .get(id)
            .map_err(|err| err.convert_error_with_message(format!("cached resource {id}")))
    }

    fn resolve_alias(&self, mut id: Id) -> Result<Id> {
        loop {
            match self.data.translate_alias(&id) {
                Ok(Some(alias)) => id = alias,
                Ok(None) => return Ok(id),
                Err(error) => {
                    return Err(error);
                }
            }
        }
    }

    /// Looks up a resource by ID using the internal [`ResourceLookup`] implementation.
    fn lookup_data_by_id(&self, id: &Id, real_id: &Id) -> Result<D> {
        self.lookup.lookup_alias(&id, real_id)
    }
}

impl<'d, D, T, L, W> Clone for ResourceDex<'d, D, T, L, W>
where
    L: Clone,
{
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            cache: ResourceCache::new(),
            lookup: self.lookup.clone(),
            phantom_data: PhantomData,
            phantom_wrapper: PhantomData,
        }
    }
}

#[derive(Clone)]
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
mod resource_cache_test {
    use battler_data::Id;

    use crate::dex::ResourceCache;

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
mod dex_test {
    use core::{
        cell::RefCell,
        ops::Deref,
    };

    use anyhow::Result;
    use battler_data::{
        DataStore,
        Id,
    };
    use battler_test_utils::{
        local_data_store,
        static_local_data_store,
    };
    use hashbrown::HashMap;
    use rand::random;

    use crate::dex::{
        ResourceDex,
        ResourceLookup,
        ResourceWrapper,
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
        lookup_calls: RefCell<HashMap<Id, u64>>,
    }

    impl<'d> ResourceLookup<'d, TestData> for TestDataLookup {
        fn new(_: &'d dyn DataStore) -> Self {
            Self {
                lookup_calls: RefCell::new(HashMap::default()),
            }
        }

        fn lookup(&self, id: &Id) -> Result<TestData> {
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
        let dex = TestDex::new(static_local_data_store());
        let first_resource = dex.get("first").unwrap();
        let second_resource = dex.get_by_id(&Id::from("first")).unwrap();
        // Random integers should be the same.
        assert_eq!(first_resource.deref(), second_resource.deref());
        // Only a single lookup occurred.
        assert_eq!(
            *dex.lookup.lookup_calls.borrow(),
            HashMap::from_iter([(Id::from("first"), 1)])
        );
    }

    #[test]
    fn resolves_alias() {
        let mut data = local_data_store();
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
        // Multiple lookups for each alias.
        assert_eq!(
            *dex.lookup.lookup_calls.borrow(),
            HashMap::from_iter([(Id::from("native"), 3)])
        );
    }
}
