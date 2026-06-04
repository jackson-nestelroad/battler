use alloc::string::String;
use core::str::FromStr;

use anyhow::Result;
use hashbrown::{
    HashMap,
    hash_map::Entry,
};
use itertools::Itertools;

use crate::{
    battle::SpeedOrderable,
    effect::fxlang::{
        BattleEvent,
        BattleEventModifier,
        Callback,
        Callbacks,
        ConditionAttributes,
        LocalData,
        ParsedProgram,
        ProgramMetadata,
    },
    error::WrapResultError,
    general_error,
};

/// Parsed version of [`Callback`][`crate::effect::fxlang::Callback`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCallback {
    pub program: ParsedProgram,
    pub order: u32,
    pub priority: i32,
    pub sub_order: u32,
    pub metadata: ProgramMetadata,
}

impl ParsedCallback {
    /// Extends the callback with another callback, overriding data if applicable.
    fn extend(&mut self, other: Self) {
        if !other.program.block.is_empty() {
            self.program = other.program;
        }

        // Always override order numbers.
        //
        // The previous program may be reused at a different priority.
        self.order = other.order;
        self.priority = other.priority;
        self.sub_order = other.sub_order;

        if other.metadata != ProgramMetadata::default() {
            self.metadata = other.metadata;
        }
    }
}

impl SpeedOrderable for ParsedCallback {
    fn order(&self) -> u32 {
        self.order
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn sub_priority(&self) -> i32 {
        0
    }

    fn speed(&self) -> u32 {
        0
    }

    fn sub_order(&self) -> u32 {
        self.sub_order
    }
}

/// Parsed version of [`Effect`][`crate::effect::fxlang::Effect`].
#[derive(Debug, Default, Clone)]
pub struct ParsedEffect {
    callbacks: HashMap<(BattleEvent, BattleEventModifier), ParsedCallback>,
    condition: ConditionAttributes,
    local_data: LocalData,
}

impl ParsedEffect {
    fn parse_and_save(&mut self, name: &str, callback: &Callback) -> Result<()> {
        let (event, modifier) = Self::callback_name_to_event_key(name)
            .wrap_error_with_format(format_args!("invalid callback {name}"))?;

        let program = match callback.program() {
            Some(program) => ParsedProgram::from(program)
                .wrap_error_with_format(format_args!("error parsing {event} callback"))?,
            None => ParsedProgram::default(),
        };
        self.callbacks.insert(
            (event, modifier),
            ParsedCallback {
                program,
                order: callback.order(),
                priority: callback.priority(),
                sub_order: callback.sub_order(),
                metadata: callback.metadata().cloned().unwrap_or_default(),
            },
        );
        Ok(())
    }

    pub fn callback_name_to_event_key(name: &str) -> Result<(BattleEvent, BattleEventModifier)> {
        let mut parts = name.split('_').multipeek();

        match parts.peek().map(|s| *s) {
            Some("on") => {
                parts.next();
            }
            Some(_) => (),
            None => return Err(general_error("callback name cannot be empty")),
        }

        parts.reset_peek();

        let modifier = match parts.peek().map(|s| *s) {
            Some(part) => match BattleEventModifier::from_str(part) {
                Ok(modifier) => match parts.peek().map(|s| *s) {
                    // Some events use modifiers but are distinct.
                    Some("end" | "residual" | "restart" | "start") => {
                        BattleEventModifier::default()
                    }
                    Some(_) | None => {
                        parts.next();
                        modifier
                    }
                },
                Err(_) => BattleEventModifier::default(),
            },
            None => BattleEventModifier::default(),
        };

        let event = parts
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(char) => char.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::default(),
                }
            })
            .join("");

        let event = BattleEvent::from_str(&event).map_err(general_error)?;
        Ok((event, modifier))
    }

    /// Creates a new [`ParsedEffect`].
    pub fn new(
        callbacks: &Callbacks,
        condition: ConditionAttributes,
        local_data: LocalData,
    ) -> Result<Self> {
        let mut parsed = Self {
            callbacks: HashMap::default(),
            condition,
            local_data,
        };

        for (name, callback) in callbacks {
            parsed.parse_and_save(name, callback)?;
        }

        Ok(parsed)
    }

    /// Extends the callbacks for this effect.
    pub fn extend(&mut self, other: Self) {
        for (key, callback) in other.callbacks {
            match self.callbacks.entry(key) {
                Entry::Occupied(mut entry) => entry.get_mut().extend(callback),
                Entry::Vacant(entry) => {
                    entry.insert(callback);
                }
            }
        }
        self.condition.extend(other.condition);
        self.local_data.extend(other.local_data);
    }

    /// Returns the [`ParsedCallback`] for the given event and modifier.
    pub fn event(
        &self,
        event: BattleEvent,
        modifier: BattleEventModifier,
    ) -> Option<&ParsedCallback> {
        self.callbacks.get(&(event, modifier))
    }

    /// Sets the [`ParsedCallback`] for the given event and modifier.
    pub(in crate::effect) fn set_event(
        &mut self,
        event: BattleEvent,
        modifier: BattleEventModifier,
        callback: ParsedCallback,
    ) {
        self.callbacks.insert((event, modifier), callback);
    }

    /// Takes the [`ParsedCallback`] for the given event and modifier.
    pub(in crate::effect) fn take_event(
        &mut self,
        event: BattleEvent,
        modifier: BattleEventModifier,
    ) -> Option<ParsedCallback> {
        self.callbacks.remove(&(event, modifier))
    }

    /// The associated condition attributes.
    pub fn condition(&self) -> &ConditionAttributes {
        &self.condition
    }

    /// The associated local data.
    pub fn local_data(&self) -> &LocalData {
        &self.local_data
    }
}

#[cfg(test)]
mod parsed_effect_test {
    use crate::effect::fxlang::{
        BattleEvent,
        BattleEventModifier,
        ParsedEffect,
    };

    #[test]
    fn parses_callback_name_to_event_and_modifier() {
        assert_matches::assert_matches!(
            ParsedEffect::callback_name_to_event_key("on_set_status"),
            Ok((BattleEvent::SetStatus, BattleEventModifier::None))
        );
        assert_matches::assert_matches!(
            ParsedEffect::callback_name_to_event_key("on_ally_set_status"),
            Ok((BattleEvent::SetStatus, BattleEventModifier::Ally))
        );
        assert_matches::assert_matches!(
            ParsedEffect::callback_name_to_event_key("on_any_set_status"),
            Ok((BattleEvent::SetStatus, BattleEventModifier::Any))
        );
        assert_matches::assert_matches!(
            ParsedEffect::callback_name_to_event_key("on_foe_set_status"),
            Ok((BattleEvent::SetStatus, BattleEventModifier::Foe))
        );

        assert_matches::assert_matches!(
            ParsedEffect::callback_name_to_event_key("on_side_modify_atk"),
            Ok((BattleEvent::ModifyAtk, BattleEventModifier::Side))
        );
        assert_matches::assert_matches!(
            ParsedEffect::callback_name_to_event_key("on_field_modify_atk"),
            Ok((BattleEvent::ModifyAtk, BattleEventModifier::Field))
        );

        assert_matches::assert_matches!(
            ParsedEffect::callback_name_to_event_key("on_source_modify_damage"),
            Ok((BattleEvent::ModifyDamage, BattleEventModifier::Source))
        );

        assert_matches::assert_matches!(
            ParsedEffect::callback_name_to_event_key("on_residual"),
            Ok((BattleEvent::Residual, BattleEventModifier::None))
        );
        assert_matches::assert_matches!(
            ParsedEffect::callback_name_to_event_key("on_field_residual"),
            Ok((BattleEvent::FieldResidual, BattleEventModifier::None))
        );
        assert_matches::assert_matches!(
            ParsedEffect::callback_name_to_event_key("on_side_start"),
            Ok((BattleEvent::SideStart, BattleEventModifier::None))
        );
    }
}
