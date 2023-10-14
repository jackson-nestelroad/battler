use std::fmt::Display;

use ahash::{
    HashMapExt,
    HashSetExt,
};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    abilities::Ability,
    common::{
        Error,
        FastHashMap,
        FastHashSet,
        Id,
        Identifiable,
    },
    config::{
        Format,
        ResourceCheck,
    },
    dex::{
        DataLookupResult,
        Dex,
    },
    items::Item,
    mons::{
        EventData,
        Gender,
        MoveSource,
        ShinyChance,
        Species,
    },
    moves::Move,
    teams::MonData,
};

/// The maximum length of a Mon name.
const MAX_NAME_LENGTH: usize = 30;

/// An error generated from team validation.
pub struct TeamValidationError {
    /// Reasons for why the team failed validation.
    pub problems: Vec<String>,
}

impl TeamValidationError {
    /// Creates a new error.
    pub fn new() -> Self {
        Self {
            problems: Vec::new(),
        }
    }

    /// Creates a new error with the given problem.
    pub fn problem(problem: String) -> Self {
        let mut error = Self::new();
        error.add_problem(problem);
        error
    }

    /// Is the team valid?
    pub fn valid(&self) -> bool {
        self.problems.is_empty()
    }

    /// Adds a new problem.
    pub fn add_problem(&mut self, problem: String) {
        self.problems.push(problem)
    }

    fn merge(&mut self, other: Result<(), TeamValidationError>) {
        if let Err(mut error) = other {
            self.problems.append(&mut error.problems);
        }
    }
}

impl Into<Result<(), TeamValidationError>> for TeamValidationError {
    fn into(self) -> Result<(), TeamValidationError> {
        if self.problems.is_empty() {
            Ok(())
        } else {
            Err(self)
        }
    }
}

impl Display for TeamValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.problems.join("; "))
    }
}

/// The state of Mon validation algorithms.
///
/// Some state must be persisted across validating a single Mon. All such state should be stored
/// here.
struct MonValidationState<'b> {
    /// Was this Mon obtained from a giveaway event?
    from_event: bool,
    /// Possible events the Mon may have been obtained from.
    possible_events: FastHashMap<&'b str, &'b EventData>,
}

impl<'d> MonValidationState<'d> {
    fn new() -> Self {
        Self {
            from_event: false,
            possible_events: FastHashMap::new(),
        }
    }

    fn add_possible_events(&mut self, events: FastHashMap<&'d str, &'d EventData>) {
        // If this Mon is not yet known to be from an event, then the first set of events is the
        // initial set. Otherwise, take the intersection to receive the new set of possible events
        // this Mon could have been from.
        if self.possible_events.is_empty() && !self.from_event {
            self.from_event = true;
            self.possible_events = events;
        } else {
            let mut intersection = FastHashMap::new();
            for (id, event) in &self.possible_events {
                if events.contains_key(id) {
                    intersection.insert(*id, *event);
                }
            }
            self.possible_events = intersection;
        }
    }
}

impl From<Error> for TeamValidationError {
    fn from(value: Error) -> Self {
        Self::problem(value.to_string())
    }
}

/// Whether or not a single move is known to be legal on a Mon.
enum MoveLegality {
    Illegal(String),
    Unknown,
    Legal,
}

impl MoveLegality {
    fn is_unknown(&self) -> bool {
        match self {
            Self::Unknown => true,
            _ => false,
        }
    }
}

/// An object used for validating teams for a new battle against its rules and format.
///
/// Each Mon on the team will be validated completely. Mons are determined to be legal against their
/// corresponding [`crate::mons::SpeciesData`].
pub struct TeamValidator<'b, 'd> {
    /// Battle format.
    pub format: &'b Format,
    /// Resource dex.
    pub dex: &'b Dex<'d>,
}

impl<'b, 'd> TeamValidator<'b, 'd> {
    /// Creates a new [`TeamValidator`].
    pub fn new(format: &'b Format, dex: &'b Dex<'d>) -> Self {
        Self { format, dex }
    }

    /// Validates an entire team for a battle.
    pub fn validate_team(&self, team: &mut [&'b mut MonData]) -> Result<(), TeamValidationError> {
        let mut result = TeamValidationError::new();

        let team_size = team.len();
        let max_team_size = self.format.rules.numeric_rules.max_team_size as usize;
        if team_size > max_team_size {
            result.add_problem(format!(
                "You may only bring up to {max_team_size} Mons (your team has {team_size})."
            ));
            // Return early, since there's no point in validating a large team.
            return result.into();
        }

        let min_team_size = self.format.rules.numeric_rules.min_team_size as usize;
        if team_size < min_team_size {
            result.add_problem(format!(
                "You must bring at least {min_team_size} Mons (your team has {team_size})."
            ))
        }

        for mon in team.iter_mut() {
            result.merge(self.validate_mon(&mut *mon));
        }

        for clause in self.format.rules.clauses(self.dex) {
            result.merge(clause.on_validate_team(self, team.as_mut()));
        }

        result.into()
    }

    /// Validates a single Mon for a battle.
    pub fn validate_mon(&self, mon: &'b mut MonData) -> Result<(), TeamValidationError> {
        let mut result = TeamValidationError::new();

        let species = match self.dex.species.get(&mon.species) {
            DataLookupResult::Found(species) => species,
            DataLookupResult::NotFound => {
                result.add_problem(format!("Species {} does not exist.", mon.species));
                return result.into();
            }
            DataLookupResult::Error(error) => {
                result.add_problem(format!(
                    "Failed to lookup species {}: {error}.",
                    mon.species
                ));
                return result.into();
            }
        };
        let ability = match self.dex.abilities.get(&mon.ability) {
            DataLookupResult::Found(ability) => ability,
            DataLookupResult::NotFound => {
                result.add_problem(format!(
                    "Ability {} (on {}) does not exist.",
                    mon.ability, mon.name
                ));
                return result.into();
            }
            DataLookupResult::Error(error) => {
                result.add_problem(format!(
                    "Failed to lookup ability {}: {error}.",
                    mon.ability
                ));
                return result.into();
            }
        };
        let item = if let Some(item) = &mon.item {
            match self.dex.items.get(&item) {
                DataLookupResult::Found(item) => Some(item),
                DataLookupResult::NotFound => {
                    result.add_problem(format!("Item {} (on {}) does not exist.", item, mon.name));
                    return result.into();
                }
                DataLookupResult::Error(error) => {
                    result.add_problem(format!("Failed to lookup item {}: {error}.", item));
                    return result.into();
                }
            }
        } else {
            None
        };

        // Name validation.
        if mon.name.len() > MAX_NAME_LENGTH {
            result.add_problem(format!(
                "Nickname \"{}\" is too long (should be {MAX_NAME_LENGTH} characters or fewer).",
                mon.name,
            ));
        }
        lazy_static! {
            static ref NAME_PATTERN: Regex = Regex::new(r"^[^|]+$").unwrap();
        }
        if !NAME_PATTERN.is_match(&mon.name) {
            result.add_problem(format!(
                "Nickname \"{}\" contains illegal characters.",
                mon.name
            ));
        }

        // Level validation.
        if mon.level == u8::default() {
            mon.level = self.format.rules.numeric_rules.default_level as u8;
        }
        if let Some(force_level) = self.format.rules.numeric_rules.force_level {
            mon.level = force_level as u8;
        } else if let Some(adjust_level_down) = self.format.rules.numeric_rules.adjust_level_down {
            if mon.level > adjust_level_down as u8 {
                // For Mon validation purposes, we spike the level up mto the maximum to make all
                // moves obtainable.
                //
                // We adjust the level at the end of validation.
                mon.level = self.format.rules.numeric_rules.max_level as u8;
            }
        }
        let min_level = self.format.rules.numeric_rules.min_level as u8;
        let max_level = self.format.rules.numeric_rules.max_level as u8;
        if mon.level < min_level {
            result.add_problem(format!(
                "{} (level {}) is below the minimum level of {min_level}.",
                mon.name, mon.level
            ));
        }
        if mon.level > max_level {
            result.add_problem(format!(
                "{} (level {}) is above the maximum level of {max_level}.",
                mon.name, mon.level
            ));
        }

        // EV validation.
        if !mon.evs.values().all(|ev| ev <= 255) {
            result.add_problem(format!("{} has an EV over 255 in some stat.", mon.name));
        }
        let ev_limit = self.format.rules.numeric_rules.ev_limit;
        let ev_sum = mon.evs.sum();
        if ev_sum > ev_limit {
            result.add_problem(format!(
                "{} has {ev_sum} EVs, which exceeds the limit of {ev_limit}.",
                mon.name
            ));
        }

        // IV validation.
        if !mon.ivs.values().all(|iv| iv <= 31) {
            result.add_problem(format!("{} has an IV over 31 in some stat.", mon.name));
        }

        // Gender validation.
        if species.data.male_only() && mon.gender != Gender::Male {
            result.add_problem(format!("{} must be male.", mon.name));
        } else if species.data.female_only() && mon.gender != Gender::Female {
            result.add_problem(format!("{} must be female.", mon.name));
        } else if species.data.unknown_gender() && mon.gender != Gender::Unknown {
            result.add_problem(format!("{} must be genderless.", mon.name));
        }

        // Species validation.
        result.merge(self.validate_species(species));
        result.merge(self.validate_forme(mon, species, ability, item));

        // Forme may have changed, so look up the species again.
        let species = match self.dex.species.get(&mon.species) {
            DataLookupResult::Found(species) => species,
            DataLookupResult::NotFound => {
                result.add_problem(format!(
                    "Species {} ({} was forced into it) does not exist.",
                    mon.species, mon.name
                ));
                return result.into();
            }
            DataLookupResult::Error(error) => {
                result.add_problem(format!(
                    "Failed to lookup species {}: {error}.",
                    mon.species
                ));
                return result.into();
            }
        };

        let mut state = MonValidationState::new();
        if let Some(item) = item {
            result.merge(self.validate_item(mon, item));
        }
        result.merge(self.validate_moveset(mon, species, &mut state));
        result.merge(self.validate_ability(mon, species, ability, &mut state));
        // At this point, the moves and ability have informed us of a set of possible events this
        // Mon could have been received from. We must validate the rest of the Mon's properties to
        // make sure this Mon really did come from that event.
        result.merge(self.validate_event(mon, &mut state));

        if let Some(adjust_level_down) = self.format.rules.numeric_rules.adjust_level_down {
            if mon.level > adjust_level_down as u8 {
                mon.level = adjust_level_down as u8;
            }
        }

        for clause in self.format.rules.clauses(self.dex) {
            result.merge(clause.on_validate_mon(self, mon));
        }

        result.into()
    }

    fn check_if_resource_is_allowed<'a>(&self, ids: impl Iterator<Item = &'a Id>) -> ResourceCheck {
        let mut check = ResourceCheck::Unknown;
        for id in ids {
            check = check.and_then(|| self.format.rules.check_resource(id));
        }
        check
    }

    fn validate_species(&self, species: &'d Species) -> Result<(), TeamValidationError> {
        let mut result = TeamValidationError::new();

        let tags = species
            .data
            .tags
            .iter()
            .map(|tag| Id::from(tag.to_string()))
            .collect::<Vec<_>>();
        let check = self.check_if_resource_is_allowed(
            [species.id(), &Id::from(species.data.base_species.as_ref())]
                .into_iter()
                .chain(tags.iter())
                .chain([&Id::from_known("allmons")].into_iter()),
        );
        match check {
            ResourceCheck::Banned => {
                result.add_problem(format!("{} is not allowed.", species.data.display_name()));
                return result.into();
            }
            ResourceCheck::Allowed => {
                return result.into();
            }
            ResourceCheck::Unknown => (),
        }

        result.into()
    }

    fn validate_forme(
        &self,
        mon: &'b mut MonData,
        species: &'d Species,
        _: &'d Ability,
        item: Option<&Item>,
    ) -> Result<(), TeamValidationError> {
        let mut result = TeamValidationError::new();

        if species.data.battle_only_forme {
            result.add_problem(format!(
                "{} is only available via in-battle transformation, so your team may not start with one.",
                species.data.display_name()
            ));
        }

        if !species.data.required_items.is_empty()
            && (item.is_none()
                || !species
                    .data
                    .required_items
                    .contains(item.unwrap().id().as_ref()))
        {
            result.add_problem(format!(
                "{} is only available when holding one of the following items: {}.",
                species.data.display_name(),
                species.data.required_items.iter().join(", ")
            ));
        }

        // The item forces this base species into some forme, so modify the Mon's species.
        if let Some(Some(force_forme)) = &item.map(|item| &item.data.force_forme) {
            if let DataLookupResult::Found(force_forme_species) = self.dex.species.get(&force_forme)
            {
                if species.data.name == force_forme_species.data.base_species {
                    mon.species = force_forme.clone();
                }
            }
        }

        result.into()
    }

    fn validate_item(&self, _: &'b MonData, item: &'d Item) -> Result<(), TeamValidationError> {
        let mut result = TeamValidationError::new();

        // Check if item is allowed.
        let tags = item
            .data
            .flags
            .iter()
            .map(|tag| Id::from(format!("itemtag{tag}")))
            .collect::<Vec<_>>();
        let check = self.check_if_resource_is_allowed(
            [item.id()]
                .into_iter()
                .chain(tags.iter())
                .chain([&Id::from_known("allitems")].into_iter()),
        );
        match check {
            ResourceCheck::Banned => {
                result.add_problem(format!("Item {} is not allowed.", item.data.name));
                return result.into();
            }
            ResourceCheck::Allowed => {
                return result.into();
            }
            ResourceCheck::Unknown => (),
        }

        result.into()
    }

    fn validate_moveset(
        &self,
        mon: &'b MonData,
        species: &'d Species,
        state: &mut MonValidationState<'d>,
    ) -> Result<(), TeamValidationError> {
        let mut result = TeamValidationError::new();

        let max_move_count = self.format.rules.numeric_rules.max_move_count as usize;
        if mon.moves.len() > max_move_count {
            result.add_problem(format!(
                "{} has {} moves, which is more than the limit of {max_move_count}.",
                mon.name,
                mon.moves.len()
            ));
            return result.into();
        }

        for move_name in &mon.moves {
            let mov = match self.dex.moves.get(move_name) {
                DataLookupResult::Found(mov) => mov,
                DataLookupResult::NotFound => {
                    result.add_problem(format!(
                        "Move {} (on {}) does not exist.",
                        move_name, mon.name
                    ));
                    return result.into();
                }
                DataLookupResult::Error(error) => {
                    result.add_problem(format!("Failed to lookup move {}: {error}", move_name,));
                    return result.into();
                }
            };
            result.merge(self.validate_move(mon, species, mov, state));
        }
        result.into()
    }

    fn validate_move(
        &self,
        mon: &'b MonData,
        species: &'d Species,
        mov: &'d Move,
        state: &mut MonValidationState<'d>,
    ) -> Result<(), TeamValidationError> {
        let mut result = TeamValidationError::new();

        // Check if move is allowed.
        let tags = mov
            .data
            .flags
            .iter()
            .map(|tag| Id::from(format!("movetag{tag}")))
            .collect::<Vec<_>>();
        let check = self.check_if_resource_is_allowed(
            [mov.id()]
                .into_iter()
                .chain(tags.iter())
                .chain([&Id::from_known("allmoves")].into_iter()),
        );
        match check {
            ResourceCheck::Banned => {
                result.add_problem(format!("Move {} is not allowed.", mov.data.name));
                return result.into();
            }
            ResourceCheck::Allowed => {
                return result.into();
            }
            ResourceCheck::Unknown => (),
        }

        match self.validate_can_learn(mon, species, mov, state) {
            MoveLegality::Legal => (),
            MoveLegality::Illegal(reason) => {
                result.add_problem(format!(
                    "{} cannot learn {}, because {} {reason}",
                    mon.name, mov.data.name, mov.data.name
                ));
            }
            // This should not happen.
            MoveLegality::Unknown => {
                result.add_problem(format!(
                    "It is unknown if {} can learn {}. This is a bug in the validation algorithm.",
                    mon.name, mov.data.name
                ));
            }
        }

        result.into()
    }

    fn validate_can_learn(
        &self,
        mon: &'b MonData,
        species: &'d Species,
        mov: &'d Move,
        state: &mut MonValidationState<'d>,
    ) -> MoveLegality {
        let mut seen = FastHashSet::<Id>::new();
        let mut current_species = DataLookupResult::Found(species);
        let mut possible_events = FastHashMap::new();

        loop {
            let species = match current_species {
                DataLookupResult::NotFound => break,
                DataLookupResult::Found(species) => species,
                DataLookupResult::Error(error) => {
                    return MoveLegality::Illegal(format!("could not be looked up: {error}."));
                }
            };

            // Loop ends if we have already checked this species for legality.
            if seen.contains(species.id()) {
                break;
            }
            seen.insert(species.id().clone());

            // This forme does not have its own learnset.
            //
            // Check the base forme.
            if species.data.learnset.is_empty() {
                if let Some(changes_from) = species
                    .data
                    .changes_from
                    .as_ref()
                    .or(species.data.base_forme.as_ref())
                {
                    current_species = self.dex.species.get(changes_from);
                }
            }

            // Look through all move sources to learn about how the Mon may have gotten this move.
            //
            // For events, we must consider all possible events the Mon may have been received from.
            // This forms a set of possible events, which is stored on the shared MonValidationState
            // and verified in the end.
            //
            // If the move is obtainable from other means aside from event, then there is no need to
            // enforce that the move comes from one of those events, so they can all be ignored.
            let mut legality = MoveLegality::Unknown;

            // At this point, we have a learnset to check.
            match species.data.learnset.get(mov.id().as_ref()) {
                Some(sources) => {
                    for source in sources {
                        match source {
                            MoveSource::Level(level) => {
                                if mon.level < *level {
                                    legality = MoveLegality::Illegal(format!(
                                        "is learned at level {level}."
                                    ));
                                } else {
                                    legality = MoveLegality::Legal;
                                }
                            }
                            _ => {
                                // No validation for other move sources. We assume the
                                // move is automatically valid in these cases.
                                legality = MoveLegality::Legal;
                                // Break out of the loop here, since we know the move is
                                // valid, so all other move source possibilities don't
                                // matter.
                                break;
                            }
                        }
                    }
                }
                None => {
                    // Sketch can be relearned, so the Mon can effectively learn any move.
                    if species.data.learnset.contains_key("sketch") {
                        legality = MoveLegality::Legal;
                    }
                }
            }

            // We have our answer.
            if !legality.is_unknown() {
                return legality;
            }

            // The move may have come from an event giveaway.
            //
            // However, the move may also come from a different forme or pre-evolution. Thus, we
            // need to store the possible events for this move separately from the Mon state until
            // we find that there is no other way to obtain this move outside of a giveaway event.
            let events_with_this_move = species
                .data
                .events
                .iter()
                .filter(|(_, event)| event.moves.contains(mov.id().as_ref()))
                .map(|(id, event)| (id.as_ref(), event))
                .collect::<FastHashMap<_, _>>();
            if possible_events.is_empty() {
                possible_events = events_with_this_move;
            } else {
                possible_events = possible_events
                    .into_iter()
                    .filter(|(id, _)| events_with_this_move.contains_key(id))
                    .collect()
            }

            // We are not sure if the move is legal or not yet.
            //
            // First, check for a base species. Some formes have learnset extensions from their base
            // forme.
            //
            // Next, check pre-evolutions, which may contain moves unobtainable after evolution.
            if let Some(changes_from) = species
                .data
                .changes_from
                .as_ref()
                .or(species.data.base_forme.as_ref())
            {
                current_species = self.dex.species.get(changes_from);
                continue;
            } else if let Some(prevo) = &species.data.prevo {
                current_species = self.dex.species.get(prevo);
            }
        }

        // There is some giveaway event that allows this move.
        //
        // Notice that if the move was found to be obtainable any other way, these events do not
        // need to be saved here.
        if !possible_events.is_empty() {
            state.add_possible_events(possible_events);
            return MoveLegality::Legal;
        }

        // If we finish the loop without an answer, the default answer is that the move is illegal.
        return MoveLegality::Illegal(format!("is unobtainable on {}.", species.data.name));
    }

    fn validate_ability(
        &self,
        mon: &'b MonData,
        species: &Species,
        ability: &Ability,
        _: &mut MonValidationState,
    ) -> Result<(), TeamValidationError> {
        let mut result = TeamValidationError::new();

        // Check if ability is allowed.
        let tags = ability
            .data
            .flags
            .iter()
            .map(|tag| Id::from(format!("abilitytag{tag}")))
            .collect::<Vec<_>>();
        let check = self.check_if_resource_is_allowed(
            [ability.id()]
                .into_iter()
                .chain(tags.iter())
                .chain([&Id::from_known("allabilities")].into_iter()),
        );
        match check {
            ResourceCheck::Banned => {
                result.add_problem(format!("Ability {} is not allowed.", ability.data.name));
                return result.into();
            }
            ResourceCheck::Allowed => {
                return result.into();
            }
            ResourceCheck::Unknown => (),
        }

        // Normal ability.
        if species.data.abilities.contains(&ability.data.name) {
            return result.into();
        }

        // Hidden ability.
        if let Some(hidden_ability) = &species.data.hidden_ability {
            if &ability.data.name == hidden_ability {
                return result.into();
            }
        }

        // Otherwise, this ability may be exclusive to some giveaway event. This does not really
        // happen, but we allow it.
        result.add_problem(format!(
            "{} cannot have the ability {} because it is unobtainable.",
            mon.name, ability.data.name
        ));
        result.into()
    }

    fn validate_event(
        &self,
        mon: &'b MonData,
        state: &mut MonValidationState,
    ) -> Result<(), TeamValidationError> {
        let mut result = TeamValidationError::new();

        // Nothing to check.
        if !state.from_event {
            return result.into();
        }

        // Check if events are banned.
        if self.check_if_resource_is_allowed([&Id::from_known("allevents")].into_iter())
            == ResourceCheck::Banned
        {
            result.add_problem(format!("All Mons obtained from events are banned."));
            return result.into();
        }

        if state.possible_events.is_empty() {
            result.add_problem(format!(
                "{} is unobtainable (no single giveaway event allows its moveset).",
                mon.name,
            ));
            return result.into();
        }

        // At this point, the set of possible events is made up using the Mon's moveset and ability.
        // We validate everything else here.
        let number_of_possible_events = state
            .possible_events
            .iter()
            .filter(|(_, event)| mon.level >= event.level.unwrap_or(0))
            .filter(|(_, event)| match event.shiny {
                ShinyChance::Always => mon.shiny,
                ShinyChance::Never => !mon.shiny,
                _ => true,
            })
            .filter(|(_, event)| match &event.gender {
                Some(gender) => mon.gender == *gender,
                None => true,
            })
            .filter(|(_, event)| match &event.nature {
                Some(nature) => mon.nature == *nature,
                None => true,
            })
            .filter(|(_, event)| match &event.ball {
                Some(ball) => mon.ball == *ball,
                None => true,
            })
            .filter(|(_, event)| {
                event
                    .ivs
                    .iter()
                    .all(|(stat, iv)| mon.ivs.get(*stat) == *iv as u16)
            })
            .count();

        if number_of_possible_events == 0 {
            result.add_problem(format!(
                "{} is unobtainable (no matching giveaway event).",
                mon.name,
            ));
        }

        result.into()
    }
}

#[cfg(test)]
mod team_validator_tests {
    use serde::Deserialize;

    use crate::{
        common::read_test_cases,
        config::{
            Format,
            FormatData,
        },
        dex::{
            Dex,
            LocalDataStore,
        },
        teams::{
            MonData,
            TeamValidator,
        },
    };

    #[derive(Deserialize)]
    struct TeamValidatorTestCase {
        format: FormatData,
        team: Vec<MonData>,
        expected_problems: Vec<String>,
        want_team: Option<Vec<MonData>>,
    }

    #[test]
    fn team_validator_test_cases() {
        let test_cases =
            read_test_cases::<TeamValidatorTestCase>("team_validator_tests.json").unwrap();
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let dex = Dex::new(&data);
        for (test_name, mut test_case) in test_cases {
            let format = Format::new(test_case.format, &dex).unwrap();
            let validator = TeamValidator::new(&format, &dex);
            let result =
                validator.validate_team(test_case.team.iter_mut().collect::<Vec<_>>().as_mut());
            let problems = match result {
                Ok(_) => Vec::new(),
                Err(error) => error.problems,
            };
            assert_eq!(
                problems, test_case.expected_problems,
                "Problems with {test_name}"
            );
            if let Some(want_team) = test_case.want_team {
                assert_eq!(
                    test_case.team, want_team,
                    "Team after validation of {test_name}"
                );
            }
        }
    }
}
