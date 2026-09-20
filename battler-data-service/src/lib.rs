use anyhow::Result;
use battler_data::{
    AbilityData,
    ConditionData,
    DataStore,
    Id,
    ItemData,
    MoveData,
    ResourceType,
    SpeciesData,
    TypeChart,
};
pub use battler_data_service_schema::{
    BatchQuery,
    BatchResult,
    DescriptionData,
    DescriptionStore,
    ResourceData,
    ResourceLookupOptions,
    ResourceOptions,
    ResourceType as SchemaResourceType,
};

/// Sanitizes a [`MoveData`] by stripping fxlang AST bytecode fields.
pub fn sanitize_move(mut data: MoveData) -> MoveData {
    data.effect = serde_json::Value::Null;
    data.condition = serde_json::Value::Null;
    for secondary in &mut data.secondary_effects {
        secondary.effect = serde_json::Value::Null;
    }
    data
}

/// Sanitizes an [`AbilityData`] by stripping fxlang AST bytecode fields.
pub fn sanitize_ability(mut data: AbilityData) -> AbilityData {
    data.effect = serde_json::Value::Null;
    data.condition = serde_json::Value::Null;
    data
}

/// Sanitizes an [`ItemData`] by stripping fxlang AST bytecode fields.
pub fn sanitize_item(mut data: ItemData) -> ItemData {
    data.effect = serde_json::Value::Null;
    data.condition = serde_json::Value::Null;
    data
}

/// Sanitizes a [`ConditionData`] by stripping fxlang AST bytecode fields.
pub fn sanitize_condition(mut data: ConditionData) -> ConditionData {
    data.condition = serde_json::Value::Null;
    data
}

/// Service for querying game data from the `battler` data store.
pub struct BattlerDataService<'d> {
    data: &'d dyn DataStore,
    descriptions: Option<&'d dyn DescriptionStore>,
}

impl<'d> BattlerDataService<'d> {
    /// Creates a new data service over the given data store.
    pub fn new(data: &'d dyn DataStore) -> Self {
        Self {
            data,
            descriptions: None,
        }
    }

    /// Creates a new data service with descriptions support.
    pub fn new_with_descriptions(
        data: &'d dyn DataStore,
        descriptions: &'d dyn DescriptionStore,
    ) -> Self {
        Self {
            data,
            descriptions: Some(descriptions),
        }
    }

    /// Sets the description store for the service.
    pub fn set_descriptions(&mut self, descriptions: &'d dyn DescriptionStore) {
        self.descriptions = Some(descriptions);
    }

    /// Queries the description for a resource.
    pub fn get_description(
        &self,
        resource_type: SchemaResourceType,
        query: &str,
    ) -> Result<Option<DescriptionData>> {
        let Some(descriptions) = self.descriptions else {
            return Ok(None);
        };
        let id = self.resolve_id(resource_type.into(), query)?;
        descriptions.get_description(resource_type, &id)
    }

    fn resolve_id(&self, resource_type: ResourceType, query: &str) -> Result<Id> {
        let mut id = Id::from(query);
        while let Some(alias) = self.data.translate_alias(resource_type, &id)? {
            id = alias;
        }
        Ok(id)
    }

    /// Queries a move by name or ID.
    pub fn get_move(&self, query: &str, options: ResourceOptions) -> Result<Option<MoveData>> {
        let id = self.resolve_id(ResourceType::Move, query)?;
        let data = self.data.get_move(&id)?;
        Ok(data.map(|d| {
            if options.include_fxlang {
                d
            } else {
                sanitize_move(d)
            }
        }))
    }

    /// Queries an ability by name or ID.
    pub fn get_ability(
        &self,
        query: &str,
        options: ResourceOptions,
    ) -> Result<Option<AbilityData>> {
        let id = self.resolve_id(ResourceType::Ability, query)?;
        let data = self.data.get_ability(&id)?;
        Ok(data.map(|d| {
            if options.include_fxlang {
                d
            } else {
                sanitize_ability(d)
            }
        }))
    }

    /// Queries an item by name or ID.
    pub fn get_item(&self, query: &str, options: ResourceOptions) -> Result<Option<ItemData>> {
        let id = self.resolve_id(ResourceType::Item, query)?;
        let data = self.data.get_item(&id)?;
        Ok(data.map(|d| {
            if options.include_fxlang {
                d
            } else {
                sanitize_item(d)
            }
        }))
    }

    /// Queries a condition by name or ID.
    pub fn get_condition(
        &self,
        query: &str,
        options: ResourceOptions,
    ) -> Result<Option<ConditionData>> {
        let id = self.resolve_id(ResourceType::Condition, query)?;
        let data = self.data.get_condition(&id)?;
        Ok(data.map(|d| {
            if options.include_fxlang {
                d
            } else {
                sanitize_condition(d)
            }
        }))
    }

    /// Queries a species by name or ID.
    pub fn get_species(
        &self,
        query: &str,
        _options: ResourceOptions,
    ) -> Result<Option<SpeciesData>> {
        let id = self.resolve_id(ResourceType::Species, query)?;
        self.data.get_species(&id)
    }

    /// Queries a resource across multiple resource types by name or ID.
    pub fn get_resource(
        &self,
        query: &str,
        options: ResourceLookupOptions,
    ) -> Result<Option<ResourceData>> {
        let resource_options = ResourceOptions {
            include_fxlang: options.include_fxlang,
        };

        if let Some(data) = self.get_condition(query, resource_options)? {
            return Ok(Some(ResourceData::Condition(data)));
        }
        if let Some(data) = self.get_move(query, resource_options)? {
            return Ok(Some(ResourceData::Move(data)));
        }
        if let Some(data) = self.get_ability(query, resource_options)? {
            return Ok(Some(ResourceData::Ability(data)));
        }
        if let Some(data) = self.get_item(query, resource_options)? {
            return Ok(Some(ResourceData::Item(data)));
        }
        if let Some(data) = self.get_species(query, resource_options)? {
            return Ok(Some(ResourceData::Species(data)));
        }

        Ok(None)
    }

    /// Queries multiple resources in a batch.
    pub fn batch(&self, query: BatchQuery) -> Result<BatchResult> {
        let mut result = BatchResult::default();
        for m in query.moves {
            let data = self.get_move(&m, query.options)?;
            let desc = self.get_description(SchemaResourceType::Move, &m)?;
            result.moves.insert(m.clone(), data);
            result.descriptions.insert(m, desc);
        }
        for a in query.abilities {
            let data = self.get_ability(&a, query.options)?;
            let desc = self.get_description(SchemaResourceType::Ability, &a)?;
            result.abilities.insert(a.clone(), data);
            result.descriptions.insert(a, desc);
        }
        for i in query.items {
            let data = self.get_item(&i, query.options)?;
            let desc = self.get_description(SchemaResourceType::Item, &i)?;
            result.items.insert(i.clone(), data);
            result.descriptions.insert(i, desc);
        }
        for c in query.conditions {
            let data = self.get_condition(&c, query.options)?;
            let desc = self.get_description(SchemaResourceType::Condition, &c)?;
            result.conditions.insert(c.clone(), data);
            result.descriptions.insert(c, desc);
        }
        for s in query.species {
            let data = self.get_species(&s, query.options)?;
            let desc = self.get_description(SchemaResourceType::Species, &s)?;
            result.species.insert(s.clone(), data);
            result.descriptions.insert(s, desc);
        }
        Ok(result)
    }

    /// Queries the full type chart.
    pub fn get_type_chart(&self) -> Result<TypeChart> {
        self.data.get_type_chart()
    }
}
