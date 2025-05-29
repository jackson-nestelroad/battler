use std::{
    fmt::Display,
    sync::LazyLock,
};

use anyhow::{
    Context,
    Error,
    Result,
};
use regex::Regex;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MoveChoice {
    pub slot: usize,
    pub target: Option<isize>,
    pub mega: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ItemChoice {
    pub item: String,
    pub target: Option<isize>,
    pub additional: Vec<isize>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Choice {
    #[default]
    Pass,
    Escape,
    Forfeit,
    Team {
        mons: Vec<usize>,
    },
    Switch {
        mon: usize,
    },
    Move(MoveChoice),
    Item(ItemChoice),
    LearnMove {
        forget: usize,
    },
}

impl Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "pass"),
            Self::Escape => write!(f, "escape"),
            Self::Forfeit => write!(f, "forfeit"),
            Self::Team { mons } => {
                write!(f, "team")?;
                for (i, mon) in mons.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, " {mon}")?;
                }
                Ok(())
            }
            Self::Switch { mon } => write!(f, "switch {mon}"),
            Self::Move(MoveChoice { slot, target, mega }) => {
                write!(f, "move {slot}")?;
                if let Some(target) = target {
                    write!(f, ",{target}")?;
                }
                if *mega {
                    write!(f, ",mega")?;
                }
                Ok(())
            }
            Self::Item(ItemChoice {
                item,
                target,
                additional,
            }) => {
                write!(f, "item {item}")?;
                if let Some(target) = target {
                    write!(f, ",{target}")?;
                }
                for (i, val) in additional.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{val}")?;
                }
                Ok(())
            }
            Self::LearnMove { forget } => write!(f, "learnmove {forget}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SwitchChoiceFailure {
    Trapped { position: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MoveChoiceFailure {
    InvalidTarget { slot: usize, target: isize },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemChoiceFailure {
    InvalidTarget { item: String, target: isize },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EscapeChoiceFailure {
    CannotEscape,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ForfeitChoiceFailure {
    CannotForfeit,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChoiceFailure {
    Switch(SwitchChoiceFailure),
    Move(MoveChoiceFailure),
    Item(ItemChoiceFailure),
    Escape(EscapeChoiceFailure),
    Forfeit(ForfeitChoiceFailure),
}

impl ChoiceFailure {
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
