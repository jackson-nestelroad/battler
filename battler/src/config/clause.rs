use std::str::FromStr;

use anyhow::Result;
use battler_data::{
    ClauseData,
    ClauseValueType,
    Id,
    Identifiable,
    Type,
};

use crate::{
    effect::fxlang,
    error::general_error,
};

/// A rule that modifies the validation, start, or team preview stages of a battle.
///
/// A clause is a generalization of a rule: a clause can be a compound rule made up of several more
/// rules, or it can be a simple rule with an assigned value.
#[derive(Clone)]
pub struct Clause {
    id: Id,
    pub data: ClauseData,
    pub effect: fxlang::Effect,
}

impl Clause {
    /// Creates a new clause.
    pub fn new(id: Id, data: ClauseData) -> Self {
        let effect = data.effect.clone().try_into().unwrap_or_default();
        Self { id, data, effect }
    }

    /// Validates the given value according to clause's configuration.
    pub fn validate_value(&self, value: &str) -> Result<()> {
        if value.is_empty() {
            if self.data.requires_value {
                return Err(general_error("missing value"));
            }
            Ok(())
        } else {
            match self.data.value_type {
                Some(ClauseValueType::Type) => {
                    Type::from_str(value).map_err(general_error).map(|_| ())
                }
                Some(ClauseValueType::PositiveInteger) => {
                    value.parse::<u32>().map_err(general_error).and_then(|val| {
                        if val > 0 {
                            Ok(())
                        } else {
                            Err(general_error("integer cannot be 0"))
                        }
                    })
                }
                Some(ClauseValueType::NonNegativeInteger) => {
                    value.parse::<u32>().map_err(general_error).map(|_| ())
                }
                _ => Ok(()),
            }
        }
    }
}

impl Identifiable for Clause {
    fn id(&self) -> &Id {
        &self.id
    }
}
