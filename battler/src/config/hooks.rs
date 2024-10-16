use std::str::FromStr;

use ahash::{
    HashMapExt,
    HashSetExt,
};
use lazy_static::lazy_static;

use crate::{
    battle::CoreBattleOptions,
    common::{
        FastHashMap,
        FastHashSet,
        Id,
        Identifiable,
    },
    config::{
        ClauseStaticHooks,
        RuleSet,
    },
    error::{
        general_error,
        Error,
        WrapOptionError,
        WrapResultError,
    },
    mons::Type,
    teams::{
        MonData,
        TeamValidationProblems,
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
                     -> TeamValidationProblems {
                        let mut result = TeamValidationProblems::default();
                        let limit = validator
                            .format
                            .rules
                            .numeric_value(&Id::from_known("abilityclause"))
                            .wrap_expectation("expected abilityclause to be defined")?
                            as usize;
                        let mut abilities = FastHashMap::<Id, usize>::new();
                        for mon in team {
                            let ability = validator
                                .dex
                                .abilities
                                .get(&mon.ability)
                                .wrap_error_with_message("expected ability to exist")?;
                            *abilities.entry(ability.id().clone()).or_default() += 1;
                        }
                        for (ability, _) in
                            abilities.into_iter().filter(|(_, count)| *count > limit)
                        {
                            let ability = validator
                                .dex
                                .abilities
                                .get_by_id(&ability)
                                .wrap_error_with_message("expected ability to exist")?;
                            result.add_problem(format!("Ability Clause: Ability {} appears more than {limit} times in your team.", ability.data.name));
                        }
                        result
                    }
                )),
                ..Default::default()
            },
        ),
        (
            Id::from_known("forcemonotype"),
            ClauseStaticHooks {
                on_validate_mon: Some(Box::new(
                    |validator: &TeamValidator, mon: &mut MonData| -> TeamValidationProblems {
                        let mut result = TeamValidationProblems::default();
                        let want = Type::from_str(
                            validator
                                .format
                                .rules
                                .value(&Id::from_known("forcemonotype"))
                                .wrap_expectation("expected forcemonotype value to be defined")?,
                        )
                        .map_err(|err| general_error(format!("invalid type: {err}")))
                        .wrap_error_with_message("expected forcemonotype to be a type")?;
                        let species = validator
                            .dex
                            .species
                            .get(&mon.species)
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
                     -> TeamValidationProblems {
                        let mut result = TeamValidationProblems::default();
                        let mut items_seen = FastHashSet::new();
                        for mon in team {
                            match &mon.item {
                                None => continue,
                                Some(item) => {
                                    let item = validator
                                        .dex
                                        .items
                                        .get(&item)
                                        .wrap_error_with_message("expected item to exist")?;
                                    if !items_seen.insert(item.id().clone()) {
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
                    |_: &TeamValidator, team: &mut [&mut MonData]| -> TeamValidationProblems {
                        let mut result = TeamValidationProblems::default();
                        let mut nicknames_seen = FastHashSet::new();
                        for mon in team {
                            if !nicknames_seen.insert(&mon.name) {
                                result.add_problem(format!("You are limited to one of each nickname by Nickname Clause (you have more than one Mon named {}).", mon.name));
                            }
                        }
                        result
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
                     -> TeamValidationProblems {
                        let mut result = TeamValidationProblems::default();
                        let mut team_types = FastHashMap::<Type, usize>::new();
                        for mon in team.iter() {
                            let species = validator
                                .dex
                                .species
                                .get(&mon.species)
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
                        result
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
                     -> TeamValidationProblems {
                        let mut result = TeamValidationProblems::default();
                        let mut species_seen = FastHashSet::new();
                        for mon in team {
                            let species = validator
                                .dex
                                .species
                                .get(&mon.species)
                                .wrap_error_with_message("expected species to exist")?;
                            if !species_seen.insert(Id::from(species.data.base_species.as_ref())) {
                                result.add_problem(format!("You are limited to one of each Mon by Species Clause (you have more than one {}).", species.data.base_species));
                            }
                        }
                        result
                    }
                )),
                ..Default::default()
            }
        ),
        (
            Id::from_known("playersperside"),
            ClauseStaticHooks {
                on_validate_core_battle_options: Some(Box::new(
                    |rules: &RuleSet, options: &mut CoreBattleOptions| -> Result<(), Error> {
                        let players_per_side = rules
                            .numeric_value(&Id::from_known("playersperside"))
                            .wrap_expectation("expected playersperside to be an integer")?
                            as usize;
                        let suffix = if players_per_side == 1 { "" } else { "s" };
                        if options.side_1.players.len() != players_per_side {
                            return Err(general_error(format!(
                                "{} must have exactly {players_per_side} player{suffix}.",
                                options.side_1.name,
                            )));
                        }
                        if options.side_2.players.len() != players_per_side {
                            return Err(general_error(format!(
                                "{} must have exactly {players_per_side} player{suffix}.",
                                options.side_2.name,
                            )));
                        }
                        Ok(())
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
