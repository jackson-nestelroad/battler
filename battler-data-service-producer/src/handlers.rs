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
                battler_data_service_schema::BattlerDataServiceError::NotFound(
                    input.0.query.clone(),
                )
            })?;
        let description = self.service.get_description(
            battler_data_service_schema::ResourceType::Move,
            &input.0.query,
        )?;
        let data_json = serde_json::to_string(&data)?;
        Ok(battler_data_service_schema::ResourceOutput(
            battler_data_service_schema::ResourceOutputArgs {
                data_json,
                description,
            },
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
                battler_data_service_schema::BattlerDataServiceError::NotFound(
                    input.0.query.clone(),
                )
            })?;
        let description = self.service.get_description(
            battler_data_service_schema::ResourceType::Ability,
            &input.0.query,
        )?;
        let data_json = serde_json::to_string(&data)?;
        Ok(battler_data_service_schema::ResourceOutput(
            battler_data_service_schema::ResourceOutputArgs {
                data_json,
                description,
            },
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
                battler_data_service_schema::BattlerDataServiceError::NotFound(
                    input.0.query.clone(),
                )
            })?;
        let description = self.service.get_description(
            battler_data_service_schema::ResourceType::Item,
            &input.0.query,
        )?;
        let data_json = serde_json::to_string(&data)?;
        Ok(battler_data_service_schema::ResourceOutput(
            battler_data_service_schema::ResourceOutputArgs {
                data_json,
                description,
            },
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
                battler_data_service_schema::BattlerDataServiceError::NotFound(
                    input.0.query.clone(),
                )
            })?;
        let description = self.service.get_description(
            battler_data_service_schema::ResourceType::Condition,
            &input.0.query,
        )?;
        let data_json = serde_json::to_string(&data)?;
        Ok(battler_data_service_schema::ResourceOutput(
            battler_data_service_schema::ResourceOutputArgs {
                data_json,
                description,
            },
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
                battler_data_service_schema::BattlerDataServiceError::NotFound(
                    input.0.query.clone(),
                )
            })?;
        let description = self.service.get_description(
            battler_data_service_schema::ResourceType::Species,
            &input.0.query,
        )?;
        let data_json = serde_json::to_string(&data)?;
        Ok(battler_data_service_schema::ResourceOutput(
            battler_data_service_schema::ResourceOutputArgs {
                data_json,
                description,
            },
        ))
    }

    fn options() -> battler_wamprat::procedure::ProcedureOptions {
        battler_wamprat::procedure::ProcedureOptions {
            disclose_caller: false,
            ..Default::default()
        }
    }
}

pub struct ResourceHandler {
    pub service: Arc<BattlerDataService<'static>>,
}

impl battler_data_service_schema::ResourceProcedure for ResourceHandler {}

impl battler_wamprat::procedure::TypedProcedure for ResourceHandler {
    type Input = battler_data_service_schema::ResourceLookupInput;
    type Output = battler_data_service_schema::ResourceOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        _: battler_wamprat::procedure::Invocation,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let data = self
            .service
            .get_resource(&input.0.query, input.0.options)?
            .ok_or_else(|| {
                battler_data_service_schema::BattlerDataServiceError::NotFound(
                    input.0.query.clone(),
                )
            })?;
        let resource_type = match &data {
            battler_data_service_schema::ResourceData::Condition(_) => {
                battler_data_service_schema::ResourceType::Condition
            }
            battler_data_service_schema::ResourceData::Move(_) => {
                battler_data_service_schema::ResourceType::Move
            }
            battler_data_service_schema::ResourceData::Ability(_) => {
                battler_data_service_schema::ResourceType::Ability
            }
            battler_data_service_schema::ResourceData::Item(_) => {
                battler_data_service_schema::ResourceType::Item
            }
            battler_data_service_schema::ResourceData::Species(_) => {
                battler_data_service_schema::ResourceType::Species
            }
        };
        let description = self
            .service
            .get_description(resource_type, &input.0.query)?;
        let data_json = serde_json::to_string(&data)?;
        Ok(battler_data_service_schema::ResourceOutput(
            battler_data_service_schema::ResourceOutputArgs {
                data_json,
                description,
            },
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

pub struct TypeChartHandler {
    pub service: Arc<BattlerDataService<'static>>,
}

impl battler_data_service_schema::TypeChartProcedure for TypeChartHandler {}

impl battler_wamprat::procedure::TypedProcedure for TypeChartHandler {
    type Input = battler_data_service_schema::TypeChartInput;
    type Output = battler_data_service_schema::TypeChartOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        _: battler_wamprat::procedure::Invocation,
        _: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let type_chart = self.service.get_type_chart()?;
        let data_json = serde_json::to_string(&type_chart)?;
        Ok(battler_data_service_schema::TypeChartOutput(
            battler_data_service_schema::TypeChartOutputArgs { data_json },
        ))
    }

    fn options() -> battler_wamprat::procedure::ProcedureOptions {
        battler_wamprat::procedure::ProcedureOptions {
            disclose_caller: false,
            ..Default::default()
        }
    }
}
