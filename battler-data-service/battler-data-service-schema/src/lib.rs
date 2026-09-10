use ahash::HashMap;

use battler_data::{
    AbilityData,
    ConditionData,
    ItemData,
    MoveData,
    SpeciesData,
};
use battler_wamp_values::{
    WampDeserialize,
    WampDeserializeError,
    WampDictionary,
    WampList,
    WampSerialize,
    WampSerializeError,
};
use battler_wamprat_message::WampApplicationMessage;
use battler_wamprat_schema::WampSchema;
use serde::{
    Deserialize,
    Serialize,
};

mod error;
pub use error::BattlerDataServiceError;

/// A type of resource that can be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    Condition,
    Move,
    Ability,
    Item,
    Species,
}

impl WampSerialize for ResourceType {
    fn wamp_serialize(self) -> Result<battler_wamp_values::Value, WampSerializeError> {
        match self {
            Self::Condition => "condition",
            Self::Move => "move",
            Self::Ability => "ability",
            Self::Item => "item",
            Self::Species => "species",
        }
        .to_owned()
        .wamp_serialize()
    }
}

impl WampDeserialize for ResourceType {
    fn wamp_deserialize(value: battler_wamp_values::Value) -> Result<Self, WampDeserializeError> {
        let s = String::wamp_deserialize(value)?;
        match s.to_ascii_lowercase().as_str() {
            "condition" => Ok(Self::Condition),
            "move" => Ok(Self::Move),
            "ability" => Ok(Self::Ability),
            "item" => Ok(Self::Item),
            "species" => Ok(Self::Species),
            _ => Err(WampDeserializeError::new(format!("invalid resource type: {s}"))),
        }
    }
}

/// Options for querying a generic resource across multiple types.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, WampDictionary)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct ResourceLookupOptions {
    /// Whether to include raw fxlang code. Defaults to false.
    #[battler_wamp_values(default)]
    #[serde(default)]
    pub include_fxlang: bool,
}

/// Arguments for querying a generic resource.
#[derive(Debug, Default, Clone, PartialEq, Eq, WampList)]
pub struct ResourceLookupInputArgs {
    /// Query string (name or ID).
    pub query: String,
    /// Options for querying the resource.
    #[battler_wamp_values(default)]
    pub options: ResourceLookupOptions,
}

/// Input for querying a generic resource.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ResourceLookupInput(#[arguments] pub ResourceLookupInputArgs);

/// A resolved resource tagged with its type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum ResourceData {
    Condition(ConditionData),
    Move(MoveData),
    Ability(AbilityData),
    Item(ItemData),
    Species(SpeciesData),
}

/// Options for querying a resource.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, WampDictionary)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct ResourceOptions {
    /// Whether to include raw fxlang code. Defaults to false.
    #[battler_wamp_values(default)]
    #[serde(default)]
    pub include_fxlang: bool,
}

/// Arguments for querying a single resource.
#[derive(Debug, Default, Clone, PartialEq, Eq, WampList)]
pub struct ResourceInputArgs {
    /// Query string (name or ID).
    pub query: String,
    /// Options for querying the resource.
    #[battler_wamp_values(default)]
    pub options: ResourceOptions,
}

/// Input for querying a single resource.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ResourceInput(#[arguments] pub ResourceInputArgs);

/// Arguments for single resource data output.
#[derive(Debug, Default, Clone, WampList)]
pub struct ResourceOutputArgs {
    /// JSON-serialized resource data.
    pub data_json: String,
}

/// Output for querying a single resource.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ResourceOutput(#[arguments] pub ResourceOutputArgs);

/// Arguments for batch resource lookup.
#[derive(Debug, Default, Clone, WampList)]
pub struct BatchInputArgs {
    /// JSON-serialized [`BatchQuery`].
    pub query_json: String,
}

/// Input for batch resource lookup.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct BatchInput(#[arguments] pub BatchInputArgs);

/// Arguments for batch resource lookup output.
#[derive(Debug, Default, Clone, WampList)]
pub struct BatchOutputArgs {
    /// JSON-serialized [`BatchResult`].
    pub result_json: String,
}

/// Output for batch resource lookup.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct BatchOutput(#[arguments] pub BatchOutputArgs);

/// Query for batch resource lookup.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct BatchQuery {
    #[serde(default)]
    pub moves: Vec<String>,
    #[serde(default)]
    pub abilities: Vec<String>,
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default)]
    pub conditions: Vec<String>,
    #[serde(default)]
    pub species: Vec<String>,
    #[serde(default)]
    pub options: ResourceOptions,
}

/// Result of a batch resource lookup.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct BatchResult {
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(type = "Record<string, MoveData | null>"))]
    pub moves: HashMap<String, Option<MoveData>>,
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(type = "Record<string, AbilityData | null>"))]
    pub abilities: HashMap<String, Option<AbilityData>>,
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(type = "Record<string, ItemData | null>"))]
    pub items: HashMap<String, Option<ItemData>>,
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(type = "Record<string, ConditionData | null>"))]
    pub conditions: HashMap<String, Option<ConditionData>>,
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(type = "Record<string, SpeciesData | null>"))]
    pub species: HashMap<String, Option<SpeciesData>>,
}

#[cfg(test)]
#[cfg(feature = "typescript")]
mod typescript_tests {
    use ts_rs::TS;

    use super::*;

    #[test]
    fn export_types() {
        ResourceType::export().unwrap();
        ResourceLookupOptions::export().unwrap();
        ResourceData::export().unwrap();
        ResourceOptions::export().unwrap();
        BatchQuery::export().unwrap();
        BatchResult::export().unwrap();
    }
}

/// Service for querying game data from the `battler` data store.
#[derive(Debug, Clone, WampSchema)]
#[realm("com.battler")]
pub enum BattlerDataService {
    /// Queries move data by name or ID.
    #[rpc(uri = "com.battler.data_service.move", input = ResourceInput, output = ResourceOutput)]
    Move,

    /// Queries ability data by name or ID.
    #[rpc(uri = "com.battler.data_service.ability", input = ResourceInput, output = ResourceOutput)]
    Ability,

    /// Queries item data by name or ID.
    #[rpc(uri = "com.battler.data_service.item", input = ResourceInput, output = ResourceOutput)]
    Item,

    /// Queries condition data by name or ID.
    #[rpc(uri = "com.battler.data_service.condition", input = ResourceInput, output = ResourceOutput)]
    Condition,

    /// Queries species data by name or ID.
    #[rpc(uri = "com.battler.data_service.species", input = ResourceInput, output = ResourceOutput)]
    Species,

    /// Queries a resource across multiple resource types by name or ID.
    #[rpc(uri = "com.battler.data_service.resource", input = ResourceLookupInput, output = ResourceOutput)]
    Resource,

    /// Batch queries multiple resources in a single RPC round-trip.
    #[rpc(uri = "com.battler.data_service.batch", input = BatchInput, output = BatchOutput)]
    Batch,
}
