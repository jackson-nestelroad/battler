use std::{
    borrow::Borrow,
    hash::{
        Hash,
        Hasher,
    },
    iter::FusedIterator,
    marker::PhantomData,
    mem,
    ptr::{
        self,
        NonNull,
    },
};

use ahash::{
    HashMap,
    HashMapExt,
};

/// A reference to a key.
#[derive(Eq)]
#[repr(transparent)]
struct KeyRef<K>(*const K);

impl<K: Hash> Hash for KeyRef<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { &*self.0 }.hash(state)
    }
}

impl<K: PartialEq> PartialEq for KeyRef<K> {
    fn eq(&self, other: &KeyRef<K>) -> bool {
        unsafe { &*self.0 }.eq(unsafe { &*other.0 })
    }
}

#[derive(PartialEq, Eq, Hash)]
#[repr(transparent)]
struct KeyValue<K: ?Sized>(K);

impl<K> KeyValue<K>
where
    K: ?Sized,
{
    fn from_ref(key: &K) -> &Self {
        // Transparent representation makes this cast valid.
        unsafe { &*(key as *const K as *const KeyValue<K>) }
    }
}

impl<K, L> Borrow<KeyValue<L>> for KeyRef<K>
where
    K: Borrow<L>,
    L: ?Sized,
{
    fn borrow(&self) -> &KeyValue<L> {
        let key = unsafe { &*self.0 }.borrow();
        KeyValue::from_ref(key)
    }
}

/// An entry in an LRU cache.
///
/// Holds a key-value pair, and a reference to the previous and next entry for linked list ordering.
struct LruEntry<K, V> {
    key: mem::MaybeUninit<K>,
    value: mem::MaybeUninit<V>,
    prev: *mut LruEntry<K, V>,
    next: *mut LruEntry<K, V>,
}

impl<K, V> LruEntry<K, V> {
    fn new(key: K, value: V) -> Self {
        Self {
            key: mem::MaybeUninit::new(key),
            value: mem::MaybeUninit::new(value),
            prev: ptr::null_mut(),
            next: ptr::null_mut(),
        }
    }

    fn new_empty() -> Self {
        Self {
            key: mem::MaybeUninit::uninit(),
            value: mem::MaybeUninit::uninit(),
            prev: ptr::null_mut(),
            next: ptr::null_mut(),
        }
    }
}

/// An LRU (least-recently-used) cache.
///
/// Implemented by maintaining a doubly-linked list of cache entries. On access, entries are moved
/// to the head of list. Once the cache reaches capacity, entries at the back of the list will be
/// evicted first to make room for newer entries.
pub struct LruCache<K, V> {
    map: HashMap<KeyRef<K>, NonNull<LruEntry<K, V>>>,
    capacity: usize,
    head: *mut LruEntry<K, V>,
    tail: *mut LruEntry<K, V>,
}

impl<K, V> Clone for LruCache<K, V>
where
    K: PartialEq + Eq + Hash + Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        let mut cloned = Self::new(self.capacity());
        for (key, value) in self.iter().rev() {
            cloned.push(key.clone(), value.clone());
        }
        cloned
    }
}

impl<K, V> LruCache<K, V>
where
    K: Eq + Hash,
{
    /// Creates a new LRU cache with the given capacity.
    pub fn new(capacity: usize) -> Self {
        let cache = Self {
            map: HashMap::with_capacity(capacity),
            capacity,
            head: Box::into_raw(Box::new(LruEntry::new_empty())),
            tail: Box::into_raw(Box::new(LruEntry::new_empty())),
        };

        unsafe {
            (*cache.head).next = cache.tail;
            (*cache.tail).prev = cache.head;
        }
        cache
    }

    /// The capacity of the cache.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// The length of the map.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Puts a key-value pair into the cache.
    ///
    /// If the key already exists in the cache, it is updated and the old value is returned.
    /// Otherwise, `None` is returned.
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        self.capturing_put(key, value, false).map(|(_, v)| v)
    }

    /// Pushes a key-value pair into the cache.
    ///
    /// If the key already exists in the cache or another entry is removed (due to capacity),
    /// then the old key-value pair is returned. Otherwise, returns `None`.
    pub fn push(&mut self, key: K, value: V) -> Option<(K, V)> {
        self.capturing_put(key, value, true)
    }

    fn capturing_put(&mut self, key: K, mut value: V, capture: bool) -> Option<(K, V)> {
        let entry = self.map.get_mut(&KeyRef(&key));
        match entry {
            Some(entry) => {
                // The key is already in the cache, so just update it and move it to the front of
                // the list.
                let entry_ptr = entry.as_ptr();
                let stored_value = unsafe { &mut (*(*entry_ptr).value.as_mut_ptr()) };
                mem::swap(&mut value, stored_value);
                self.detach(entry_ptr);
                self.attach(entry_ptr);
                Some((key, value))
            }
            None => {
                let (replaced, entry) = self.replace_or_create_entry(key, value);
                let entry_ptr = entry.as_ptr();
                self.attach(entry_ptr);
                let key = unsafe { &*entry_ptr }.key.as_ptr();
                self.map.insert(KeyRef(key), entry);
                replaced.filter(|_| capture)
            }
        }
    }

    fn replace_or_create_entry(
        &mut self,
        key: K,
        value: V,
    ) -> (Option<(K, V)>, NonNull<LruEntry<K, V>>) {
        if self.len() == self.capacity() {
            // Cache is full, remove the last entry.
            let old_key = KeyRef(unsafe { &(*(*(*self.tail).prev).key.as_ptr()) });
            let old_entry = self.map.remove(&old_key).unwrap();
            let entry_ptr = old_entry.as_ptr();
            let replaced = unsafe {
                (
                    mem::replace(&mut (*entry_ptr).key, mem::MaybeUninit::new(key)).assume_init(),
                    mem::replace(&mut (*entry_ptr).value, mem::MaybeUninit::new(value))
                        .assume_init(),
                )
            };
            self.detach(entry_ptr);
            (Some(replaced), old_entry)
        } else {
            (None, unsafe {
                NonNull::new_unchecked(Box::into_raw(Box::new(LruEntry::new(key, value))))
            })
        }
    }

    fn detach(&mut self, entry: *mut LruEntry<K, V>) {
        unsafe {
            (*(*entry).prev).next = (*entry).next;
            (*(*entry).next).prev = (*entry).prev;
        }
    }

    fn attach(&mut self, entry: *mut LruEntry<K, V>) {
        unsafe {
            (*entry).next = (*self.head).next;
            (*entry).prev = self.head;
            (*self.head).next = entry;
            (*(*entry).next).prev = entry;
        }
    }

    /// Checks if the given key is contained in the cache.
    pub fn contains_key<'a, L>(&'a self, key: &L) -> bool
    where
        K: Borrow<L>,
        L: Eq + Hash + ?Sized,
    {
        self.map.contains_key(KeyValue::from_ref(key))
    }

    /// Returns a reference to the value associated with the given key.
    ///
    /// Moves the key to the head of the LRU list if it exists. Otherwise, returns [`None`].
    pub fn get<'a, L>(&'a mut self, key: &L) -> Option<&'a V>
    where
        K: Borrow<L>,
        L: Eq + Hash + ?Sized,
    {
        if let Some(entry) = self.map.get_mut(KeyValue::from_ref(key)) {
            let entry_ptr = entry.as_ptr();
            self.detach(entry_ptr);
            self.attach(entry_ptr);
            Some(unsafe { &*(*entry_ptr).value.as_ptr() })
        } else {
            None
        }
    }

    /// Returns a mutable reference to the value associated with the given key.
    ///
    /// Moves the key to the head of the LRU list if it exists. Otherwise, returns [`None`].
    pub fn get_mut<'a, L>(&'a mut self, key: &L) -> Option<&'a mut V>
    where
        K: Borrow<L>,
        L: Eq + Hash + ?Sized,
    {
        if let Some(entry) = self.map.get_mut(KeyValue::from_ref(key)) {
            let entry_ptr = entry.as_ptr();
            self.detach(entry_ptr);
            self.attach(entry_ptr);
            Some(unsafe { &mut *(*entry_ptr).value.as_mut_ptr() })
        } else {
            None
        }
    }

    /// Returns an iterator visiting all entries in most-recently used order.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            len: self.len(),
            ptr: unsafe { (*self.head).next },
            end: unsafe { (*self.tail).prev },
            phantom: PhantomData,
        }
    }

    /// Returns an iterator visiting all entries in most-recently used order, with a mutable
    /// reference to the value.
    pub fn iter_mut(&self) -> IterMut<'_, K, V> {
        IterMut {
            len: self.len(),
            ptr: unsafe { (*self.head).next },
            end: unsafe { (*self.tail).prev },
            phantom: PhantomData,
        }
    }
}

impl<K, V> Drop for LruCache<K, V> {
    fn drop(&mut self) {
        self.map.drain().for_each(|(_, entry)| unsafe {
            let mut entry = *Box::from_raw(entry.as_ptr());
            ptr::drop_in_place(entry.key.as_mut_ptr());
            ptr::drop_in_place(entry.value.as_mut_ptr());
        });
        unsafe { drop(Box::from_raw(self.head)) };
        unsafe { drop(Box::from_raw(self.tail)) };
    }
}

impl<'a, K, V> IntoIterator for &'a LruCache<K, V>
where
    K: Eq + Hash,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut LruCache<K, V>
where
    K: Eq + Hash,
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

unsafe impl<K: Send, V: Send> Send for LruCache<K, V> {}
unsafe impl<K: Sync, V: Sync> Sync for LruCache<K, V> {}

/// An iterator over entries in an [`LruCache`].
pub struct Iter<'a, K, V>
where
    K: 'a,
    V: 'a,
{
    len: usize,
    ptr: *const LruEntry<K, V>,
    end: *const LruEntry<K, V>,
    phantom: PhantomData<&'a K>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        let key = unsafe { &(*(*self.ptr).key.as_ptr()) as &K };
        let value = unsafe { &(*(*self.ptr).value.as_ptr()) as &V };
        self.len -= 1;
        self.ptr = unsafe { (*self.ptr).next };
        Some((key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }

    fn count(self) -> usize {
        self.len
    }
}

impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        let key = unsafe { &(*(*self.end).key.as_ptr()) };
        let value = unsafe { &(*(*self.end).value.as_ptr()) };
        self.len -= 1;
        self.end = unsafe { (*self.end).prev };
        Some((key, value))
    }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {}
impl<'a, K, V> FusedIterator for Iter<'a, K, V> {}

unsafe impl<'a, K: Send, V: Send> Send for Iter<'a, K, V> {}
unsafe impl<'a, K: Sync, V: Sync> Sync for Iter<'a, K, V> {}

/// A mutable iterator over entries in an [`LruCache`].
pub struct IterMut<'a, K, V>
where
    K: 'a,
    V: 'a,
{
    len: usize,
    ptr: *mut LruEntry<K, V>,
    end: *mut LruEntry<K, V>,
    phantom: PhantomData<&'a K>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        let key = unsafe { &(*(*self.ptr).key.as_ptr()) };
        let value = unsafe { &mut (*(*self.ptr).value.as_mut_ptr()) };
        self.len -= 1;
        self.ptr = unsafe { (*self.ptr).next };
        Some((key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }

    fn count(self) -> usize {
        self.len
    }
}

impl<'a, K, V> DoubleEndedIterator for IterMut<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        let key = unsafe { &(*(*self.end).key.as_ptr()) };
        let value = unsafe { &mut (*(*self.end).value.as_mut_ptr()) };
        self.len -= 1;
        self.end = unsafe { (*self.end).prev };
        Some((key, value))
    }
}

impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {}
impl<'a, K, V> FusedIterator for IterMut<'a, K, V> {}

unsafe impl<'a, K: Send, V: Send> Send for IterMut<'a, K, V> {}
unsafe impl<'a, K: Sync, V: Sync> Sync for IterMut<'a, K, V> {}

#[cfg(test)]
mod lru_cache_test {
    use crate::common::LruCache;

    #[test]
    fn removes_least_recently_used_by_capacity() {
        let mut cache = LruCache::new(2);
        assert_eq!(cache.capacity(), 2);
        assert_eq!(cache.len(), 0);

        assert!(!cache.contains_key("a"));
        assert_eq!(cache.push("a", 1), None);
        assert!(cache.contains_key("a"));
        assert_eq!(cache.len(), 1);
        assert!(!cache.contains_key("b"));
        assert_eq!(cache.push("b", 2), None);
        assert!(cache.contains_key("b"));
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get("a"), Some(&1));
        assert_eq!(cache.get("b"), Some(&2));

        assert_eq!(cache.push("b", 3), Some(("b", 2)));
        assert_eq!(cache.push("b", 4), Some(("b", 3)));
        assert_eq!(cache.get("a"), Some(&1));
        assert_eq!(cache.get("b"), Some(&4));
        assert_eq!(
            cache.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
            vec![("b", 4), ("a", 1)]
        );

        assert_eq!(cache.push("c", 5), Some(("a", 1)));
        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.get("b"), Some(&4));
        assert_eq!(cache.get("c"), Some(&5));
        assert_eq!(
            cache.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
            vec![("c", 5), ("b", 4)]
        );
    }

    #[test]
    fn iterates_in_most_recently_used_order() {
        let mut cache = LruCache::new(5);
        assert_eq!(cache.put(1, "a"), None);
        assert_eq!(cache.put(2, "b"), None);
        assert_eq!(cache.put(3, "c"), None);
        assert_eq!(cache.put(4, "d"), None);
        assert_eq!(cache.put(5, "e"), None);
        assert_eq!(
            cache.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
            vec![(5, "e"), (4, "d"), (3, "c"), (2, "b"), (1, "a")]
        );

        assert_eq!(cache.put(3, "f"), Some("c"));
        assert_eq!(cache.put(6, "g"), None);
        assert_eq!(
            cache.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
            vec![(6, "g"), (3, "f"), (5, "e"), (4, "d"), (2, "b"),]
        );
    }

    #[test]
    fn mutably_iterates_in_most_recently_used_order() {
        let mut cache = LruCache::new(5);
        assert_eq!(cache.put(1, 1), None);
        assert_eq!(cache.put(2, 2), None);
        assert_eq!(cache.put(3, 3), None);
        assert_eq!(cache.put(4, 4), None);
        assert_eq!(cache.put(5, 5), None);
        for (_, v) in cache.iter_mut() {
            *v *= 2;
        }
        assert_eq!(
            cache.iter_mut().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
            vec![(5, 10), (4, 8), (3, 6), (2, 4), (1, 2)]
        );
    }
}
