use std::str::FromStr;

use ahash::{
    HashMapExt,
    HashSetExt,
};
use lazy_static::lazy_static;

use crate::{
    common::{
        FastHashMap,
        FastHashSet,
        Id,
        Identifiable,
        WrapResultError,
    },
    config::ClauseStaticHooks,
    mons::Type,
    teams::{
        MonData,
        TeamValidationError,
        TeamValidator,
    },
};

lazy_static! {
    static ref DEFAULT_CLAUSE_HOOKS: ClauseStaticHooks = ClauseStaticHooks::default();
    static ref CLAUSE_HOOKS: FastHashMap<Id, ClauseStaticHooks> = FastHashMap::from_iter([
        (
            Id::from_known("abilityclause"),
            ClauseStaticHooks {
                on_validate_team: Some(Box::new(
                    |validator: &TeamValidator,
                     team: &mut [&mut MonData]|
                     -> Result<(), TeamValidationError> {
                        let mut result = TeamValidationError::new();
                        let limit = validator
                            .format
                            .rules
                            .numeric_value(&Id::from_known("abilityclause"))
                            .wrap_error_with_message("expected abilityclause to be defined")?
                            as usize;
                        let mut abilities = FastHashMap::<&Id, usize>::new();
                        for mon in team {
                            let ability = validator
                                .dex
                                .abilities
                                .get(&mon.ability)
                                .into_result()
                                .wrap_error_with_message("expected ability to exist")?;
                            *abilities.entry(ability.id()).or_default() += 1;
                        }
                        for (ability, _) in
                            abilities.into_iter().filter(|(_, count)| *count > limit)
                        {
                            let ability = validator
                                .dex
                                .abilities
                                .get_by_id(&ability)
                                .into_result()
                                .wrap_error_with_message("expected ability to exist")?;
                            result.add_problem(format!("Ability Clause: Ability {} appears more than {limit} times in your team.", ability.data.name));
                        }
                        result.into()
                    }
                )),
                ..Default::default()
            },
        ),
        (
            Id::from_known("forcemonotype"),
            ClauseStaticHooks {
                on_validate_mon: Some(Box::new(
                    |validator: &TeamValidator,
                     mon: &mut MonData|
                     -> Result<(), TeamValidationError> {
                        let mut result = TeamValidationError::new();
                        let want = Type::from_str(
                            validator
                                .format
                                .rules
                                .value(&Id::from_known("forcemonotype"))
                                .wrap_error_with_message(
                                    "expected forcemonotype value to be defined",
                                )?,
                        )
                        .wrap_error_with_message("expected forcemonotype to be a type")?;
                        let species = validator
                            .dex
                            .species
                            .get(&mon.species)
                            .into_result()
                            .wrap_error_with_message("expected species to exist")?;
                        if !(species.data.primary_type == want
                            || species.data.secondary_type.is_some_and(|typ| typ == want))
                        {
                            result.add_problem(format!("{} is not {want} type.", mon.name));
                        }
                        result.into()
                    }
                )),
                ..Default::default()
            },
        ),
        (
            Id::from_known("itemclause"),
            ClauseStaticHooks {
                on_validate_team: Some(Box::new(
                    |validator: &TeamValidator,
                     team: &mut [&mut MonData]|
                     -> Result<(), TeamValidationError> {
                        let mut result = TeamValidationError::new();
                        let mut items_seen = FastHashSet::new();
                        for mon in team {
                            match &mon.item {
                                None => continue,
                                Some(item) => {
                                    let item = validator
                                        .dex
                                        .items
                                        .get(&item)
                                        .into_result()
                                        .wrap_error_with_message("expected item to exist")?;
                                    if !items_seen.insert(item.id()) {
                                        result.add_problem(format!("You are limited to one of each item by Item Clause (you have more than one {}).", item.data.name));
                                    }
                                }
                            }
                        }
                        result.into()
                    }
                )),
                ..Default::default()
            }
        ),
        (
            Id::from_known("nicknameclause"),
            ClauseStaticHooks {
                on_validate_team: Some(Box::new(
                    |_: &TeamValidator,
                     team: &mut [&mut MonData]|
                     -> Result<(), TeamValidationError> {
                        let mut result = TeamValidationError::new();
                        let mut nicknames_seen = FastHashSet::new();
                        for mon in team {
                            if !nicknames_seen.insert(&mon.name) {
                                result.add_problem(format!("You are limited to one of each nickname by Nickname Clause (you have more than one Mon named {}).", mon.name));
                            }
                        }
                        result.into()
                    }
                )),
                ..Default::default()
            }
        ),
        (
            Id::from_known("sametypeclause"),
            ClauseStaticHooks {
                on_validate_team: Some(Box::new(
                    |validator: &TeamValidator,
                     team: &mut [&mut MonData]|
                     -> Result<(), TeamValidationError> {
                        let mut result = TeamValidationError::new();
                        let mut team_types = FastHashMap::<Type, usize>::new();
                        for mon in team.iter() {
                            let species = validator
                                .dex
                                .species
                                .get(&mon.species)
                                .into_result()
                                .wrap_error_with_message("expected species to exist")?;
                            *team_types.entry(species.data.primary_type).or_default() += 1;
                            if let Some(secondary_type) = species.data.secondary_type {
                                *team_types.entry(secondary_type).or_default() += 1;
                            }
                        }
                        let team_size = team.len();
                        if team_types.into_iter().all(|(_, count)| count < team_size) {
                            result.add_problem(format!("Your team does not share a common type to satisfy Same Type Clause."));
                        }
                        result.into()
                    }
                )),
                ..Default::default()
            },
        ),
        (
            Id::from_known("speciesclause"),
            ClauseStaticHooks {
                on_validate_team: Some(Box::new(
                    |validator: &TeamValidator,
                     team: &mut [&mut MonData]|
                     -> Result<(), TeamValidationError> {
                        let mut result = TeamValidationError::new();
                        let mut species_seen = FastHashSet::new();
                        for mon in team {
                            let species = validator
                                .dex
                                .species
                                .get(&mon.species)
                                .into_result()
                                .wrap_error_with_message("expected species to exist")?;
                            if !species_seen.insert(Id::from(species.data.base_species.as_ref())) {
                                result.add_problem(format!("You are limited to one of each Mon by Species Clause (you have more than one {}).", species.data.base_species));
                            }
                        }
                        result.into()
                    }
                )),
                ..Default::default()
            }
        )
    ]);
}

/// Returns the static hooks for the given clause.
pub(in crate::config) fn clause_hooks(clause: &Id) -> &'static ClauseStaticHooks {
    CLAUSE_HOOKS.get(clause).unwrap_or(&*DEFAULT_CLAUSE_HOOKS)
}
