use alloc::string::String;

use hashbrown::HashMap;

use crate::Id;

/// A user-defined set of aliases.
pub type SerializedAliases = HashMap<String, String>;

/// Map of one ID to another.
///
/// The key represents the alias. The value should be a non-alias ID.
pub type Aliases = HashMap<Id, Id>;

/// Converts [`SerializedAliases`] into [`Aliases`].
pub fn deserialize_aliases(aliases: SerializedAliases) -> Aliases {
    aliases
        .into_iter()
        .map(|(a, b)| (Id::from(a), Id::from(b)))
        .collect()
}
