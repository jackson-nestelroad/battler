use crate::common::{
    FastHashMap,
    Id,
};

/// A user-defined set of aliases.
pub type SerializedAliases = FastHashMap<String, String>;

/// Map of one ID to another.
///
/// The key represents the alias. The value should be a non-alias ID.
pub type Aliases = FastHashMap<Id, Id>;

/// Converts [`SerializedAliases`] into [`Aliases`].
pub fn deserialize_aliases(aliases: SerializedAliases) -> Aliases {
    aliases
        .into_iter()
        .map(|(a, b)| (Id::from(a), Id::from(b)))
        .collect()
}
