use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use battler_data::{
    AbilityData,
    ConditionData,
    ItemData,
    MoveData,
    SpeciesData,
};
use battler_data_service::BattlerDataService;
pub use battler_data_service_schema::{
    BatchInput,
    BatchInputArgs,
    BatchQuery,
    BatchResult,
    BattlerDataServiceConsumer,
    ResourceInput,
    ResourceInputArgs,
    ResourceOptions,
};
use battler_wamprat::peer::CallOptions;

/// Client wrapper for [`battler_data_service::BattlerDataService`].
#[async_trait]
pub trait BattlerDataServiceClient: Send + Sync {
    /// Queries move data by name or ID.
    async fn get_move(&self, query: &str, options: ResourceOptions) -> Result<MoveData>;
    /// Queries ability data by name or ID.
    async fn get_ability(&self, query: &str, options: ResourceOptions) -> Result<AbilityData>;
    /// Queries item data by name or ID.
    async fn get_item(&self, query: &str, options: ResourceOptions) -> Result<ItemData>;
    /// Queries condition data by name or ID.
    async fn get_condition(&self, query: &str, options: ResourceOptions) -> Result<ConditionData>;
    /// Queries species data by name or ID.
    async fn get_species(&self, query: &str, options: ResourceOptions) -> Result<SpeciesData>;
    /// Queries multiple resources in a single batch request.
    async fn batch(&self, query: BatchQuery) -> Result<BatchResult>;
}

/// Client that forwards calls directly to an in-memory [`BattlerDataService`].
pub struct DirectBattlerDataServiceClient {
    service: Arc<BattlerDataService<'static>>,
}

impl DirectBattlerDataServiceClient {
    /// Creates a new direct data service client.
    pub fn new(service: Arc<BattlerDataService<'static>>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl BattlerDataServiceClient for DirectBattlerDataServiceClient {
    async fn get_move(&self, query: &str, options: ResourceOptions) -> Result<MoveData> {
        self.service.get_move(query, options)?.ok_or_else(|| {
            battler_data_service_schema::BattlerDataServiceError::NotFound(query.to_owned()).into()
        })
    }

    async fn get_ability(&self, query: &str, options: ResourceOptions) -> Result<AbilityData> {
        self.service.get_ability(query, options)?.ok_or_else(|| {
            battler_data_service_schema::BattlerDataServiceError::NotFound(query.to_owned()).into()
        })
    }

    async fn get_item(&self, query: &str, options: ResourceOptions) -> Result<ItemData> {
        self.service.get_item(query, options)?.ok_or_else(|| {
            battler_data_service_schema::BattlerDataServiceError::NotFound(query.to_owned()).into()
        })
    }

    async fn get_condition(&self, query: &str, options: ResourceOptions) -> Result<ConditionData> {
        self.service.get_condition(query, options)?.ok_or_else(|| {
            battler_data_service_schema::BattlerDataServiceError::NotFound(query.to_owned()).into()
        })
    }

    async fn get_species(&self, query: &str, options: ResourceOptions) -> Result<SpeciesData> {
        self.service.get_species(query, options)?.ok_or_else(|| {
            battler_data_service_schema::BattlerDataServiceError::NotFound(query.to_owned()).into()
        })
    }

    async fn batch(&self, query: BatchQuery) -> Result<BatchResult> {
        self.service.batch(query)
    }
}

/// Creates a new data service client that forwards requests directly to the service.
pub fn battler_data_service_client_over_direct_service(
    service: Arc<BattlerDataService<'static>>,
) -> DirectBattlerDataServiceClient {
    DirectBattlerDataServiceClient::new(service)
}

/// Implementation of [`BattlerDataServiceClient`] over a WAMP service consumer.
pub struct WampBattlerDataServiceClient<S> {
    consumer: Arc<BattlerDataServiceConsumer<S>>,
}

impl<S> WampBattlerDataServiceClient<S> {
    /// Creates a new client around a WAMP service consumer.
    pub fn new(consumer: Arc<BattlerDataServiceConsumer<S>>) -> Self {
        Self { consumer }
    }
}

/// Creates a new data service client over a WAMP consumer.
pub fn battler_data_service_client_over_wamp_consumer<S>(
    consumer: Arc<BattlerDataServiceConsumer<S>>,
) -> WampBattlerDataServiceClient<S> {
    WampBattlerDataServiceClient::new(consumer)
}

#[async_trait]
impl<S> BattlerDataServiceClient for WampBattlerDataServiceClient<S>
where
    S: Send + 'static,
{
    async fn get_move(&self, query: &str, options: ResourceOptions) -> Result<MoveData> {
        let output = self
            .consumer
            .r#move(
                ResourceInput(ResourceInputArgs {
                    query: query.to_owned(),
                    options,
                }),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(serde_json::from_str(&output.0.data_json)?)
    }

    async fn get_ability(&self, query: &str, options: ResourceOptions) -> Result<AbilityData> {
        let output = self
            .consumer
            .ability(
                ResourceInput(ResourceInputArgs {
                    query: query.to_owned(),
                    options,
                }),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(serde_json::from_str(&output.0.data_json)?)
    }

    async fn get_item(&self, query: &str, options: ResourceOptions) -> Result<ItemData> {
        let output = self
            .consumer
            .item(
                ResourceInput(ResourceInputArgs {
                    query: query.to_owned(),
                    options,
                }),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(serde_json::from_str(&output.0.data_json)?)
    }

    async fn get_condition(&self, query: &str, options: ResourceOptions) -> Result<ConditionData> {
        let output = self
            .consumer
            .condition(
                ResourceInput(ResourceInputArgs {
                    query: query.to_owned(),
                    options,
                }),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(serde_json::from_str(&output.0.data_json)?)
    }

    async fn get_species(&self, query: &str, options: ResourceOptions) -> Result<SpeciesData> {
        let output = self
            .consumer
            .species(
                ResourceInput(ResourceInputArgs {
                    query: query.to_owned(),
                    options,
                }),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(serde_json::from_str(&output.0.data_json)?)
    }

    async fn batch(&self, query: BatchQuery) -> Result<BatchResult> {
        let output = self
            .consumer
            .batch(
                BatchInput(BatchInputArgs {
                    query_json: serde_json::to_string(&query)?,
                }),
                CallOptions::default(),
            )
            .await?
            .result()
            .await?;
        Ok(serde_json::from_str(&output.0.result_json)?)
    }
}
