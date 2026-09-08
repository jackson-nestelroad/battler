use alloc::string::String;

use hashbrown::HashMap;
use serde::{
    Deserialize,
    Serialize,
};

use crate::Id;

/// A type of resource that can be aliased.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Ability,
    Clause,
    Condition,
    Item,
    Move,
    Species,
}

/// A serialized map of aliases for a single resource type.
type SerializedResourceAliases = HashMap<String, String>;

/// A user-defined set of partitioned aliases.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializedAliases {
    #[serde(default)]
    abilities: SerializedResourceAliases,
    #[serde(default)]
    clauses: SerializedResourceAliases,
    #[serde(default)]
    conditions: SerializedResourceAliases,
    #[serde(default)]
    items: SerializedResourceAliases,
    #[serde(default)]
    moves: SerializedResourceAliases,
    #[serde(default)]
    species: SerializedResourceAliases,
}

/// Map of one ID to another.
///
/// The key represents the alias. The value should be a non-alias ID.
pub type Aliases = HashMap<Id, Id>;

/// A partitioned set of aliases by [`ResourceType`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PartitionedAliases {
    pub abilities: Aliases,
    pub clauses: Aliases,
    pub conditions: Aliases,
    pub items: Aliases,
    pub moves: Aliases,
    pub species: Aliases,
}

impl PartitionedAliases {
    /// Translates the given alias for the specified resource type.
    pub fn get(&self, resource_type: ResourceType, id: &Id) -> Option<&Id> {
        match resource_type {
            ResourceType::Ability => self.abilities.get(id),
            ResourceType::Clause => self.clauses.get(id),
            ResourceType::Condition => self.conditions.get(id),
            ResourceType::Item => self.items.get(id),
            ResourceType::Move => self.moves.get(id),
            ResourceType::Species => self.species.get(id),
        }
    }
}

fn deserialize_resource_aliases(aliases: SerializedResourceAliases) -> Aliases {
    aliases
        .into_iter()
        .map(|(a, b)| (Id::from(a), Id::from(b)))
        .collect()
}

/// Converts [`SerializedAliases`] into [`PartitionedAliases`].
pub fn deserialize_aliases(aliases: SerializedAliases) -> PartitionedAliases {
    PartitionedAliases {
        abilities: deserialize_resource_aliases(aliases.abilities),
        clauses: deserialize_resource_aliases(aliases.clauses),
        conditions: deserialize_resource_aliases(aliases.conditions),
        items: deserialize_resource_aliases(aliases.items),
        moves: deserialize_resource_aliases(aliases.moves),
        species: deserialize_resource_aliases(aliases.species),
    }
}
