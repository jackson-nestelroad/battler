use battler_data::{
    ConditionData,
    ConditionType,
    Id,
    Identifiable,
};

use crate::effect::fxlang;

/// An individual condition, which can affect a Mon in a variety of ways.
pub struct Condition {
    id: Id,
    pub data: ConditionData,
    pub condition: fxlang::Effect,
}

impl Condition {
    /// Creates a new condition.
    pub fn new(id: Id, data: ConditionData) -> Self {
        let condition = data.condition.clone().try_into().unwrap_or_default();
        Self {
            id,
            data,
            condition,
        }
    }

    /// The name of the condition type, for logging.
    pub fn condition_type_name(&self) -> &str {
        match self.data.condition_type {
            ConditionType::BuiltIn => "",
            ConditionType::Condition => "condition",
            ConditionType::Status => "status",
            ConditionType::Type => "type",
            ConditionType::Volatile => "",
            ConditionType::Weather => "weather",
        }
    }

    /// Similar to [`Self::condition_type_name`], except `"condition"` is used if the type name is
    /// empty.
    pub fn non_empty_condition_type_name(&self) -> &str {
        let type_name = self.condition_type_name();
        if type_name.is_empty() {
            "condition"
        } else {
            type_name
        }
    }
}

impl Identifiable for Condition {
    fn id(&self) -> &Id {
        &self.id
    }
}
