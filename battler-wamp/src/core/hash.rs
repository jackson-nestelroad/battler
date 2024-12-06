/// Type alias for hash maps used across the library.
pub type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

/// Type alias for hash sets used across the library.
pub type HashSet<V> = std::collections::HashSet<V, ahash::RandomState>;
