use std::collections::{
    HashMap,
    HashSet,
};

/// Type alias for hash maps used across the library.
pub type FastHashMap<K, V> = HashMap<K, V, ahash::RandomState>;
/// Type alias for hash sets used across the library.
pub type FastHashSet<V> = HashSet<V, ahash::RandomState>;
