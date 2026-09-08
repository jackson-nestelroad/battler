use std::sync::Arc;

use battler_data_service::BattlerDataService;

pub struct MoveHandler {
    pub service: Arc<BattlerDataService<'static>>,
}

impl battler_data_service_schema::MoveProcedure for MoveHandler {}

impl battler_wamprat::procedure::TypedProcedure for MoveHandler {
    type Input = battler_data_service_schema::ResourceInput;
    type Output = battler_data_service_schema::ResourceOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        _: battler_wamprat::procedure::Invocation,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let data = self
            .service
            .get_move(&input.0.query, input.0.options)?
            .ok_or_else(|| {
                battler_data_service_schema::BattlerDataServiceError::NotFound(input.0.query)
            })?;
        let data_json = serde_json::to_string(&data)?;
        Ok(battler_data_service_schema::ResourceOutput(
            battler_data_service_schema::ResourceOutputArgs { data_json },
        ))
    }

    fn options() -> battler_wamprat::procedure::ProcedureOptions {
        battler_wamprat::procedure::ProcedureOptions {
            disclose_caller: false,
            ..Default::default()
        }
    }
}

pub struct AbilityHandler {
    pub service: Arc<BattlerDataService<'static>>,
}

impl battler_data_service_schema::AbilityProcedure for AbilityHandler {}

impl battler_wamprat::procedure::TypedProcedure for AbilityHandler {
    type Input = battler_data_service_schema::ResourceInput;
    type Output = battler_data_service_schema::ResourceOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        _: battler_wamprat::procedure::Invocation,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let data = self
            .service
            .get_ability(&input.0.query, input.0.options)?
            .ok_or_else(|| {
                battler_data_service_schema::BattlerDataServiceError::NotFound(input.0.query)
            })?;
        let data_json = serde_json::to_string(&data)?;
        Ok(battler_data_service_schema::ResourceOutput(
            battler_data_service_schema::ResourceOutputArgs { data_json },
        ))
    }

    fn options() -> battler_wamprat::procedure::ProcedureOptions {
        battler_wamprat::procedure::ProcedureOptions {
            disclose_caller: false,
            ..Default::default()
        }
    }
}

pub struct ItemHandler {
    pub service: Arc<BattlerDataService<'static>>,
}

impl battler_data_service_schema::ItemProcedure for ItemHandler {}

impl battler_wamprat::procedure::TypedProcedure for ItemHandler {
    type Input = battler_data_service_schema::ResourceInput;
    type Output = battler_data_service_schema::ResourceOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        _: battler_wamprat::procedure::Invocation,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let data = self
            .service
            .get_item(&input.0.query, input.0.options)?
            .ok_or_else(|| {
                battler_data_service_schema::BattlerDataServiceError::NotFound(input.0.query)
            })?;
        let data_json = serde_json::to_string(&data)?;
        Ok(battler_data_service_schema::ResourceOutput(
            battler_data_service_schema::ResourceOutputArgs { data_json },
        ))
    }

    fn options() -> battler_wamprat::procedure::ProcedureOptions {
        battler_wamprat::procedure::ProcedureOptions {
            disclose_caller: false,
            ..Default::default()
        }
    }
}

pub struct ConditionHandler {
    pub service: Arc<BattlerDataService<'static>>,
}

impl battler_data_service_schema::ConditionProcedure for ConditionHandler {}

impl battler_wamprat::procedure::TypedProcedure for ConditionHandler {
    type Input = battler_data_service_schema::ResourceInput;
    type Output = battler_data_service_schema::ResourceOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        _: battler_wamprat::procedure::Invocation,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let data = self
            .service
            .get_condition(&input.0.query, input.0.options)?
            .ok_or_else(|| {
                battler_data_service_schema::BattlerDataServiceError::NotFound(input.0.query)
            })?;
        let data_json = serde_json::to_string(&data)?;
        Ok(battler_data_service_schema::ResourceOutput(
            battler_data_service_schema::ResourceOutputArgs { data_json },
        ))
    }

    fn options() -> battler_wamprat::procedure::ProcedureOptions {
        battler_wamprat::procedure::ProcedureOptions {
            disclose_caller: false,
            ..Default::default()
        }
    }
}

pub struct SpeciesHandler {
    pub service: Arc<BattlerDataService<'static>>,
}

impl battler_data_service_schema::SpeciesProcedure for SpeciesHandler {}

impl battler_wamprat::procedure::TypedProcedure for SpeciesHandler {
    type Input = battler_data_service_schema::ResourceInput;
    type Output = battler_data_service_schema::ResourceOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        _: battler_wamprat::procedure::Invocation,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let data = self
            .service
            .get_species(&input.0.query, input.0.options)?
            .ok_or_else(|| {
                battler_data_service_schema::BattlerDataServiceError::NotFound(input.0.query)
            })?;
        let data_json = serde_json::to_string(&data)?;
        Ok(battler_data_service_schema::ResourceOutput(
            battler_data_service_schema::ResourceOutputArgs { data_json },
        ))
    }

    fn options() -> battler_wamprat::procedure::ProcedureOptions {
        battler_wamprat::procedure::ProcedureOptions {
            disclose_caller: false,
            ..Default::default()
        }
    }
}

pub struct BatchHandler {
    pub service: Arc<BattlerDataService<'static>>,
}

impl battler_data_service_schema::BatchProcedure for BatchHandler {}

impl battler_wamprat::procedure::TypedProcedure for BatchHandler {
    type Input = battler_data_service_schema::BatchInput;
    type Output = battler_data_service_schema::BatchOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        _: battler_wamprat::procedure::Invocation,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let query: battler_data_service_schema::BatchQuery =
            serde_json::from_str(&input.0.query_json).map_err(|err| {
                battler_data_service_schema::BattlerDataServiceError::InvalidQuery(err.to_string())
            })?;
        let result = self.service.batch(query)?;
        let result_json = serde_json::to_string(&result)?;
        Ok(battler_data_service_schema::BatchOutput(
            battler_data_service_schema::BatchOutputArgs { result_json },
        ))
    }

    fn options() -> battler_wamprat::procedure::ProcedureOptions {
        battler_wamprat::procedure::ProcedureOptions {
            disclose_caller: false,
            ..Default::default()
        }
    }
}
