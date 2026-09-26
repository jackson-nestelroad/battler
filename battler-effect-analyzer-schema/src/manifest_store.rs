use anyhow::Result;
use battler_data::{
    DataStoreByName,
    Id,
};

use crate::{
    ability_manifest::AbilityManifest,
    condition_manifest::ConditionManifest,
    effect_manifest::EffectManifest,
    item_manifest::ItemManifest,
};

/// Trait for looking up precompiled manifests by ID.
pub trait ManifestStore: Send + Sync {
    /// Gets a move effect manifest by ID.
    fn get_move_manifest(&self, id: &Id) -> Result<Option<&EffectManifest>>;
    /// Gets a condition manifest by ID.
    fn get_condition_manifest(&self, id: &Id) -> Result<Option<&ConditionManifest>>;
    /// Gets an ability manifest by ID.
    fn get_ability_manifest(&self, id: &Id) -> Result<Option<&AbilityManifest>>;
    /// Gets an item manifest by ID.
    fn get_item_manifest(&self, id: &Id) -> Result<Option<&ItemManifest>>;
}

/// Extension trait for looking up precompiled manifests by name.
pub trait ManifestStoreByName: ManifestStore {
    /// Gets a move effect manifest by name.
    fn get_move_manifest_by_name(&self, name: &str) -> Result<Option<&EffectManifest>>;
    /// Gets a condition manifest by name.
    fn get_condition_manifest_by_name(&self, name: &str) -> Result<Option<&ConditionManifest>>;
    /// Gets an ability manifest by name.
    fn get_ability_manifest_by_name(&self, name: &str) -> Result<Option<&AbilityManifest>>;
    /// Gets an item manifest by name.
    fn get_item_manifest_by_name(&self, name: &str) -> Result<Option<&ItemManifest>>;
}

/// Combined trait implemented by any data source that provides both authentic Mon data and manifests.
pub trait CalcDataStore: DataStoreByName + ManifestStoreByName {}
impl<T: DataStoreByName + ManifestStoreByName> CalcDataStore for T {}
