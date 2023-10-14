use std::str::FromStr;

use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::{
    battler_error,
    common::{
        Error,
        Id,
        Identifiable,
    },
    config::{
        hooks::clause_hooks,
        RuleSet,
        SerializedRuleSet,
    },
    mons::Type,
    teams::{
        MonData,
        TeamValidationError,
        TeamValidator,
    },
};

/// The type of a clause value.
#[derive(Debug, Clone, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum ClauseValueType {
    #[string = "type"]
    Type,
    #[string = "PositiveInteger"]
    PositiveInteger,
    #[string = "NonNegativeInteger"]
    NonNegativeInteger,
}

/// Data for an individual clause.
///
/// A clause is a generalization of a rule: a clause can be a compound rule made up of several more
/// rules, or it can be a simple rule with an assigned value.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ClauseData {
    /// Clause name.
    pub name: String,
    /// Clause description.
    pub description: String,
    /// Message added to the battle log on battle start.
    #[serde(default)]
    pub rule_log: Option<String>,
    /// Is a value required?
    #[serde(default)]
    pub requires_value: bool,
    /// Type of value enforced by validation.
    #[serde(default)]
    pub value_type: Option<ClauseValueType>,
    /// Nested rules added to the battle format.
    #[serde(default)]
    pub rules: SerializedRuleSet,
}

type ValidateRuleCallack = dyn Fn(&RuleSet, &str) -> Result<(), Error> + Send + Sync;
type ValidateMonCallback =
    dyn Fn(&TeamValidator, &mut MonData) -> Result<(), TeamValidationError> + Send + Sync;
type ValidateTeamCallback =
    dyn Fn(&TeamValidator, &mut [&mut MonData]) -> Result<(), TeamValidationError> + Send + Sync;

/// Static hooks for clauses.
///
/// These hooks are exclusive to clauses, so they are not represented in the same way as generic
/// battle effects.
#[derive(Default)]
pub(in crate::config) struct ClauseStaticHooks {
    /// Hook for rule validation (validating this rule in the context of all other rules).
    pub on_validate_rule: Option<Box<ValidateRuleCallack>>,
    /// Hook for Mon validaiton.
    pub on_validate_mon: Option<Box<ValidateMonCallback>>,
    /// Hook for team validation.
    pub on_validate_team: Option<Box<ValidateTeamCallback>>,
}

/// A rule that modifies the validation, start, or team preview stages of a battle.
///
/// A clause is a generalization of a rule: a clause can be a compound rule made up of several more
/// rules, or it can be a simple rule with an assigned value.
#[derive(Clone)]
pub struct Clause {
    /// Clause data.
    pub data: ClauseData,
    hooks: &'static ClauseStaticHooks,
    id: Id,
}

impl Clause {
    /// Creates a new [`Clause`] instance from [`ClauseData`].
    pub fn new(data: ClauseData) -> Self {
        let id = Id::from(data.name.as_ref());
        Self {
            data,
            hooks: clause_hooks(&id),
            id,
        }
    }

    /// Validates the given value according to clause's configuration.
    pub fn validate_value(&self, value: &str) -> Result<(), Error> {
        if value.is_empty() {
            if self.data.requires_value {
                return Err(battler_error!("missing value"));
            }
            Ok(())
        } else {
            match self.data.value_type {
                Some(ClauseValueType::Type) => Type::from_str(value)
                    .map_err(|_| battler_error!("\"{value}\" is not a type"))
                    .map(|_| ()),
                Some(ClauseValueType::PositiveInteger) => value
                    .parse::<u32>()
                    .map_err(|_| ())
                    .and_then(|val| if val > 0 { Ok(()) } else { Err(()) })
                    .map_err(|_| battler_error!("\"{value}\" is not a positive integer")),
                Some(ClauseValueType::NonNegativeInteger) => value
                    .parse::<u32>()
                    .map_err(|_| battler_error!("\"{value}\" is not a non-negative integer"))
                    .map(|_| ()),
                _ => Ok(()),
            }
        }
    }

    /// Runs the hook for rule validation.
    pub fn on_validate_rule(&self, rules: &RuleSet, value: &str) -> Result<(), Error> {
        self.hooks
            .on_validate_rule
            .as_ref()
            .map_or(Ok(()), |f| f(rules, value))
    }

    /// Runs the hook for Mon validation.
    pub fn on_validate_mon(
        &self,
        validator: &TeamValidator,
        mon: &mut MonData,
    ) -> Result<(), TeamValidationError> {
        self.hooks
            .on_validate_mon
            .as_ref()
            .map_or(Ok(()), |f| f(validator, mon))
    }

    /// Runs the hook for team validation.
    pub fn on_validate_team(
        &self,
        validator: &TeamValidator,
        team: &mut [&mut MonData],
    ) -> Result<(), TeamValidationError> {
        self.hooks
            .on_validate_team
            .as_ref()
            .map_or(Ok(()), |f| f(validator, team))
    }
}

impl Identifiable for Clause {
    fn id(&self) -> &Id {
        &self.id
    }
}

#[cfg(test)]
mod clause_tests {
    use lazy_static::lazy_static;

    use crate::{
        battle::BattleType,
        battler_error,
        common::{
            Error,
            Id,
        },
        config::{
            Clause,
            ClauseData,
            ClauseStaticHooks,
            ClauseValueType,
            Format,
            RuleSet,
            SerializedRuleSet,
        },
        dex::{
            Dex,
            LocalDataStore,
        },
        teams::{
            MonData,
            TeamValidationError,
            TeamValidator,
        },
    };

    #[test]
    fn validates_type_value() {
        let clause = Clause::new(ClauseData {
            name: "Test Clause".to_owned(),
            requires_value: true,
            value_type: Some(ClauseValueType::Type),
            ..Default::default()
        });
        assert!(clause
            .validate_value("")
            .err()
            .unwrap()
            .to_string()
            .contains("missing value"));
        assert!(clause
            .validate_value("bird")
            .err()
            .unwrap()
            .to_string()
            .contains("is not a type"));
        assert!(clause.validate_value("grass").is_ok());
    }

    #[test]
    fn validates_positive_integer() {
        let clause = Clause::new(ClauseData {
            name: "Test Clause".to_owned(),
            requires_value: false,
            value_type: Some(ClauseValueType::PositiveInteger),
            ..Default::default()
        });
        assert!(clause.validate_value("").is_ok());
        assert!(clause
            .validate_value("bad")
            .err()
            .unwrap()
            .to_string()
            .contains("is not a positive integer"));
        assert!(clause
            .validate_value("-1")
            .err()
            .unwrap()
            .to_string()
            .contains("is not a positive integer"));
        assert!(clause
            .validate_value("0")
            .err()
            .unwrap()
            .to_string()
            .contains("is not a positive integer"));
        assert!(clause.validate_value("10").is_ok());
    }

    #[test]
    fn validates_non_negative_integer() {
        let clause = Clause::new(ClauseData {
            name: "Test Clause".to_owned(),
            requires_value: false,
            value_type: Some(ClauseValueType::NonNegativeInteger),
            ..Default::default()
        });
        assert!(clause.validate_value("").is_ok());
        assert!(clause
            .validate_value("bad")
            .err()
            .unwrap()
            .to_string()
            .contains("is not a non-negative integer"));
        assert!(clause
            .validate_value("-20")
            .err()
            .unwrap()
            .to_string()
            .contains("is not a non-negative integer"));
        assert!(clause.validate_value("0").is_ok());
        assert!(clause.validate_value("10").is_ok());
    }

    fn construct_ruleset(
        serialized: &str,
        battle_type: &BattleType,
        dex: &Dex,
    ) -> Result<RuleSet, Error> {
        let ruleset = serde_json::from_str::<SerializedRuleSet>(serialized).unwrap();
        RuleSet::new(ruleset, battle_type, dex)
    }

    #[test]
    fn validates_rules() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let dex = Dex::new(&data);
        let ruleset = construct_ruleset(
            r#"[
                "Other Rule = value"
            ]"#,
            &BattleType::Singles,
            &dex,
        )
        .unwrap();
        lazy_static! {
            static ref HOOKS: ClauseStaticHooks = ClauseStaticHooks {
                on_validate_rule: Some(Box::new(|rules, value| {
                    if rules
                        .value(&Id::from_known("otherrule"))
                        .is_some_and(|other_value| other_value == value)
                    {
                        return Err(battler_error!("expected error"));
                    }
                    Ok(())
                })),
                ..Default::default()
            };
        }
        let clause = Clause {
            id: Id::from("testclause"),
            data: ClauseData::default(),
            hooks: &HOOKS,
        };
        assert!(clause.on_validate_rule(&ruleset, "other").is_ok());
        assert!(clause
            .on_validate_rule(&ruleset, "value")
            .err()
            .unwrap()
            .to_string()
            .contains("expected error"));
    }

    fn construct_format(dex: &Dex) -> Format {
        Format::new(
            serde_json::from_str(
                r#"{
                    "battle_type": "Singles",
                    "rules": []
                }"#,
            )
            .unwrap(),
            &dex,
        )
        .unwrap()
    }

    #[test]
    fn validates_mon() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let dex = Dex::new(&data);
        let format = construct_format(&dex);
        let validator = TeamValidator::new(&format, &dex);
        lazy_static! {
            static ref HOOKS: ClauseStaticHooks = ClauseStaticHooks {
                on_validate_mon: Some(Box::new(|_, mon| {
                    if mon.level != 1 {
                        return Err(TeamValidationError::problem("level 1 required".to_owned()));
                    }
                    Ok(())
                })),
                ..Default::default()
            };
        }
        let clause = Clause {
            id: Id::from("testclause"),
            data: ClauseData::default(),
            hooks: &HOOKS,
        };
        let mut mon = serde_json::from_str(
            r#"{
                "name": "Bulba Fett",
                "species": "Bulbasaur",
                "ability": "Overgrow",
                "moves": [],
                "nature": "Adamant",
                "gender": "M",
                "ball": "Normal",
                "level": 50
              }"#,
        )
        .unwrap();
        assert!(clause
            .on_validate_mon(&validator, &mut mon)
            .err()
            .unwrap()
            .to_string()
            .contains("level 1 required"));

        mon.level = 1;
        assert!(clause.on_validate_mon(&validator, &mut mon).is_ok());
    }

    #[test]
    fn validates_team() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let dex = Dex::new(&data);
        let format = construct_format(&dex);
        let validator = TeamValidator::new(&format, &dex);
        lazy_static! {
            static ref HOOKS: ClauseStaticHooks = ClauseStaticHooks {
                on_validate_team: Some(Box::new(|_, team| {
                    if team.len() <= 1 {
                        return Err(TeamValidationError::problem(
                            "must have more than 1 Mon".to_owned(),
                        ));
                    }
                    Ok(())
                })),
                ..Default::default()
            };
        }
        let clause = Clause {
            id: Id::from("testclause"),
            data: ClauseData::default(),
            hooks: &HOOKS,
        };
        let mut mon = serde_json::from_str::<MonData>(
            r#"{
                "name": "Bulba Fett",
                "species": "Bulbasaur",
                "ability": "Overgrow",
                "moves": [],
                "nature": "Adamant",
                "gender": "M",
                "ball": "Normal",
                "level": 50
              }"#,
        )
        .unwrap();
        assert!(clause
            .on_validate_team(&validator, &mut [&mut mon])
            .err()
            .unwrap()
            .to_string()
            .contains("must have more than 1 Mon"));

        let mut mon2 = mon.clone();
        assert!(clause
            .on_validate_team(&validator, &mut [&mut mon, &mut mon2])
            .is_ok());
    }

    #[test]
    fn hooks_do_nothing_by_default() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let dex = Dex::new(&data);
        let format = construct_format(&dex);
        let validator = TeamValidator::new(&format, &dex);
        lazy_static! {
            static ref HOOKS: ClauseStaticHooks = ClauseStaticHooks::default();
        }
        let clause = Clause {
            id: Id::from("testclause"),
            data: ClauseData::default(),
            hooks: &HOOKS,
        };
        let mut mon = serde_json::from_str::<MonData>(
            r#"{
                "name": "Bulba Fett",
                "species": "Bulbasaur",
                "ability": "Overgrow",
                "moves": [],
                "nature": "Adamant",
                "gender": "M",
                "ball": "Normal",
                "level": 50
              }"#,
        )
        .unwrap();
        assert!(clause.validate_value("value").is_ok());
        assert!(clause.on_validate_rule(&format.rules, "value").is_ok());
        assert!(clause.on_validate_mon(&validator, &mut mon).is_ok());
        assert!(clause.on_validate_team(&validator, &mut [&mut mon]).is_ok());
    }
}
