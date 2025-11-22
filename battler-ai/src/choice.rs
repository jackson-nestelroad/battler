use std::sync::LazyLock;

use anyhow::{
    Context,
    Error,
    Result,
};
use battler_choice::{
    Choice,
    ItemChoice,
    MoveChoice,
};
use regex::Regex;
use serde::Serialize;

/// A failure that occurred when making a choice.
#[derive(Serialize)]
pub struct MakeChoiceFailure {
    pub choice: String,
    pub reason: String,
}

/// A failure that occurred when making a switch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SwitchChoiceFailure {
    /// The Mon is trapped.
    Trapped { position: usize },
}

/// A failure that occurred when making a move.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MoveChoiceFailure {
    /// The target is not allowed.
    InvalidTarget { slot: usize, target: isize },
}

/// A failure that occurred when using an item.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemChoiceFailure {
    /// The target is not allowed.
    InvalidTarget { item: String, target: isize },
}

/// A failure that occurred when choosing to escape.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EscapeChoiceFailure {
    /// Cannot escape.
    CannotEscape,
}

/// A failure that occurred when choosing to forfeit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ForfeitChoiceFailure {
    /// Cannot forfeit.
    CannotForfeit,
}

/// A parsed version of [`MakeChoiceFailure`], which can be used in decision making.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChoiceFailure {
    Switch(SwitchChoiceFailure),
    Move(MoveChoiceFailure),
    Item(ItemChoiceFailure),
    Escape(EscapeChoiceFailure),
    Forfeit(ForfeitChoiceFailure),
}

impl ChoiceFailure {
    /// Parses an error that occurred when attempting to make the given choices.
    pub fn new(error: Error, choices: &[Choice]) -> Result<Self> {
        Self::from_message(&format!("{error:#}"), choices)
            .map_err(|err| err.context("unrecoverable error"))
    }

    fn from_message(message: &str, choices: &[Choice]) -> Result<Self> {
        static INVALID_CHOICE_PATTERN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^invalid choice (\d+):(.*)"#).unwrap());

        let captures = INVALID_CHOICE_PATTERN
            .captures(&message)
            .ok_or_else(|| Error::msg("missing invalid choice prefix"))?;

        // SAFETY: Regex has 1 capture group.
        let index = captures.get(1).unwrap();
        let index = index
            .as_str()
            .parse::<usize>()
            .with_context(|| "invalid choice index")?;

        // SAFETY: Regex has 2 capture groups.
        let message = captures.get(2).unwrap().as_str();

        let choice = choices
            .get(index)
            .ok_or_else(|| Error::msg("choice index is out of bounds"))?;

        match choice {
            Choice::Switch { .. } => {
                if message.contains("is trapped") {
                    Ok(Self::Switch(SwitchChoiceFailure::Trapped {
                        position: index,
                    }))
                } else {
                    Err(Error::msg("unrecoverable switch choice"))
                }
            }
            Choice::Move(MoveChoice { slot, target, .. }) => {
                if message.contains("invalid target") {
                    Ok(Self::Move(MoveChoiceFailure::InvalidTarget {
                        slot: *slot,
                        target: target.ok_or_else(|| {
                            Error::msg(
                                "error said choice had invalid target, but no target was chosen",
                            )
                        })?,
                    }))
                } else {
                    Err(Error::msg("unrecoverable move choice"))
                }
            }
            Choice::Item(ItemChoice { item, target, .. }) => {
                if message.contains("cannot be used on") {
                    Ok(Self::Item(ItemChoiceFailure::InvalidTarget {
                        item: item.clone(),
                        target: target.ok_or_else(|| {
                            Error::msg(
                                "error said choice had invalid target, but no target was chosen",
                            )
                        })?,
                    }))
                } else {
                    Err(Error::msg("unrecoverable item choice"))
                }
            }
            Choice::Escape => {
                if message.contains("you cannot escape") {
                    Ok(Self::Escape(EscapeChoiceFailure::CannotEscape))
                } else {
                    Err(Error::msg("unrecoverable escape choice"))
                }
            }
            Choice::Forfeit => {
                if message.contains("you cannot forfeit") {
                    Ok(Self::Forfeit(ForfeitChoiceFailure::CannotForfeit))
                } else {
                    Err(Error::msg("unrecoverable forfeit choice"))
                }
            }
            _ => Err(Error::msg("choice has no recoverable failure messages")),
        }
    }
}
