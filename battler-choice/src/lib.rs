#![no_std]
#![feature(pattern)]

extern crate alloc;

use alloc::{
    borrow::ToOwned,
    collections::VecDeque,
    format,
    string::{
        String,
        ToString,
    },
    vec::Vec,
};
use core::{
    fmt::Display,
    str::{
        FromStr,
        pattern::Pattern,
    },
};

use anyhow::{
    Context,
    Error,
    Result,
};
use itertools::Itertools;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("invalid choice: {0}")]
pub struct InvalidChoiceError(String);

/// A choice to use a move.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MoveChoice {
    /// The move slot to use.
    pub slot: usize,
    /// The target of the move.
    pub target: Option<isize>,
    /// Mega Evolve?
    pub mega: bool,
    /// Dynamax?
    pub dyna: bool,
    /// Terastallize?
    pub tera: bool,
    /// Force a random target to be selected?
    pub random_target: bool,
}

impl Display for MoveChoice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.slot)?;
        if let Some(target) = self.target {
            write!(f, ",{target}")?;
        }
        if self.mega {
            write!(f, ",mega")?;
        }
        if self.dyna {
            write!(f, ",dyna")?;
        }
        if self.tera {
            write!(f, ",tera")?;
        }
        if self.random_target {
            write!(f, ",randomtarget")?;
        }
        Ok(())
    }
}

impl FromStr for MoveChoice {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut args = s
            .split(',')
            .map(|str| str.trim())
            .collect::<VecDeque<&str>>();
        let slot = args
            .pop_front()
            .context("missing move slot")?
            .parse()
            .context("invalid move slot")?;
        let mut choice = Self {
            slot,
            target: None,
            mega: false,
            dyna: false,
            tera: false,
            random_target: false,
        };

        if let Some(target) = args
            .front()
            .map(|target| target.parse::<isize>().ok())
            .flatten()
        {
            choice.target = Some(target);
            args.pop_front();
        }

        while let Some(arg) = args.pop_front() {
            match arg {
                "mega" => {
                    choice.mega = true;
                }
                "dyna" => {
                    choice.dyna = true;
                }
                "tera" => {
                    choice.tera = true;
                }
                "randomtarget" => {
                    choice.random_target = true;
                }
                _ => {
                    return Err(Error::msg(format!("invalid option in move choice: {arg}")));
                }
            }
        }

        Ok(choice)
    }
}

/// A choice to use an item.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ItemChoice {
    /// The item to use.
    pub item: String,
    /// The target of the item.
    pub target: Option<isize>,
    /// Any additional input.
    ///
    /// For example, when using a PP-healing move, the target move must be specified.
    pub additional_input: VecDeque<String>,
}

impl Display for ItemChoice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.item)?;
        if let Some(target) = self.target {
            write!(f, ",{target}")?;
        }
        for val in &self.additional_input {
            write!(f, ",{val}")?;
        }
        Ok(())
    }
}

impl FromStr for ItemChoice {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut args = s
            .split(',')
            .map(|str| str.trim())
            .collect::<VecDeque<&str>>();
        let item = args.pop_front().context("missing item")?.to_string();
        let target = args.pop_front().map(|target| target.parse().ok()).flatten();
        let additional_input = args.into_iter().map(|arg| arg.to_owned()).collect();
        Ok(Self {
            item,
            target,
            additional_input,
        })
    }
}

/// A team selection choice.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TeamSelectionChoice {
    /// The Mons to select for the team, in order.
    pub mons: Vec<usize>,
}

impl Display for TeamSelectionChoice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (i, mon) in self.mons.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{mon}")?;
        }
        Ok(())
    }
}

impl FromStr for TeamSelectionChoice {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self {
                mons: Vec::default(),
            });
        }
        let mons = s
            .split(" ")
            .map(|str| str.trim())
            .map(|str| str.parse::<usize>())
            .collect::<Result<_, _>>()?;
        Ok(Self { mons })
    }
}

/// A choice to switch in.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SwitchChoice {
    /// The Mon to switch in.
    ///
    /// If not specified, a random Mon will be selected.
    pub mon: Option<usize>,
}

impl Display for SwitchChoice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(mon) = self.mon {
            write!(f, "{}", mon)?;
        }
        Ok(())
    }
}

impl FromStr for SwitchChoice {
    type Err = <usize as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mon = (!s.is_empty()).then(|| s.parse()).transpose()?;
        Ok(Self { mon })
    }
}

/// A choice to learn a move.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LearnMoveChoice {
    /// The index of the move slot to forget.
    pub forget_move_slot: usize,
}

impl Display for LearnMoveChoice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.forget_move_slot)
    }
}

impl FromStr for LearnMoveChoice {
    type Err = <usize as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let forget_move_slot = s.parse()?;
        Ok(Self { forget_move_slot })
    }
}

/// A choice, which controls how a player responds to a request in a battle.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Choice {
    /// Do nothing.
    #[default]
    Pass,
    /// Make a random choice, depending on the type of request.
    Random,
    /// Make as many random choices as required.
    RandomAll,
    /// Attempt to escape from the battle.
    Escape,
    /// Forfeit the battle.
    Forfeit,
    /// Shift to the center (Triples battle only).
    Shift,
    /// Select team during team preview.
    Team(TeamSelectionChoice),
    /// Switch a Mon in.
    Switch(SwitchChoice),
    /// Use a move.
    Move(MoveChoice),
    /// Use an item.
    Item(ItemChoice),
    /// Learn a move.
    LearnMove(LearnMoveChoice),
}

impl Display for Choice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Pass => write!(f, "pass"),
            Self::Random => write!(f, "random"),
            Self::RandomAll => write!(f, "randomall"),
            Self::Escape => write!(f, "escape"),
            Self::Forfeit => write!(f, "forfeit"),
            Self::Shift => write!(f, "shift"),
            Self::Team(choice) => {
                write!(f, "team {choice}")
            }
            Self::Switch(choice) => write!(f, "switch {choice}"),
            Self::Move(choice) => {
                write!(f, "move {choice}")
            }
            Self::Item(choice) => {
                write!(f, "item {choice}")
            }
            Self::LearnMove(choice) => write!(f, "learnmove {choice}"),
        }
    }
}

fn split_once_optional<'a, P>(input: &'a str, delimiter: P) -> (&'a str, Option<&'a str>)
where
    P: Pattern,
{
    match input.split_once(delimiter) {
        None => (input, None),
        Some((a, b)) => (a, Some(b)),
    }
}

impl FromStr for Choice {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (choice, data) = split_once_optional(s, " ");
        let data = data.unwrap_or_default().trim();
        match choice {
            "pass" => Ok(Self::Pass),
            "random" => Ok(Self::Random),
            "randomall" => Ok(Self::RandomAll),
            "escape" => Ok(Self::Escape),
            "forfeit" => Ok(Self::Forfeit),
            "shift" => Ok(Self::Shift),
            "team" => Ok(Self::Team(TeamSelectionChoice::from_str(data)?)),
            "switch" => Ok(Self::Switch(SwitchChoice::from_str(data)?)),
            "move" => Ok(Self::Move(MoveChoice::from_str(data)?)),
            "item" => Ok(Self::Item(ItemChoice::from_str(data)?)),
            "learnmove" => Ok(Self::LearnMove(LearnMoveChoice::from_str(data)?)),
            _ => Err(Error::new(InvalidChoiceError(choice.to_string()))),
        }
    }
}

/// Serializes multiple [`Choice`]s to a string.
pub fn choices_to_string<I>(choices: I) -> String
where
    I: IntoIterator<Item = Choice>,
{
    choices
        .into_iter()
        .map(|choice| choice.to_string())
        .join(";")
}

/// Deserializes multiple [`Choice`]s from a string.
pub fn choices_from_string<S>(choices: S) -> Result<Vec<Choice>>
where
    S: AsRef<str>,
{
    choices
        .as_ref()
        .split(";")
        .map(|str| str.trim())
        .map(|str| Choice::from_str(str))
        .collect()
}

/// Deserializes multiple [`Choice`]s from a string, returning the result of parsing for each
/// choice.
pub fn choice_results_from_string<S>(choices: S) -> Vec<Result<Choice>>
where
    S: AsRef<str>,
{
    choices
        .as_ref()
        .split(";")
        .map(|str| str.trim())
        .map(|str| Choice::from_str(str))
        .collect()
}

#[cfg(test)]
mod battler_choice_test {
    use alloc::{
        borrow::ToOwned,
        collections::VecDeque,
        string::ToString,
        vec::Vec,
    };
    use core::str::FromStr;

    use crate::{
        Choice,
        ItemChoice,
        LearnMoveChoice,
        MoveChoice,
        SwitchChoice,
        TeamSelectionChoice,
        choice_results_from_string,
        choices_from_string,
        choices_to_string,
    };

    #[test]
    fn serializes_to_string() {
        assert_eq!(Choice::Pass.to_string(), "pass");
        assert_eq!(Choice::Random.to_string(), "random");
        assert_eq!(Choice::RandomAll.to_string(), "randomall");
        assert_eq!(Choice::Escape.to_string(), "escape");
        assert_eq!(Choice::Forfeit.to_string(), "forfeit");
        assert_eq!(Choice::Shift.to_string(), "shift");
        assert_eq!(
            Choice::Team(TeamSelectionChoice {
                mons: Vec::from_iter([0, 2, 4]),
            })
            .to_string(),
            "team 0 2 4"
        );
        assert_eq!(
            Choice::Switch(SwitchChoice { mon: Some(1) }).to_string(),
            "switch 1"
        );
        assert_eq!(
            Choice::Switch(SwitchChoice { mon: None }).to_string(),
            "switch "
        );
        assert_eq!(
            Choice::Move(MoveChoice {
                slot: 0,
                target: None,
                mega: false,
                dyna: false,
                tera: false,
                random_target: false,
            })
            .to_string(),
            "move 0"
        );
        assert_eq!(
            Choice::Move(MoveChoice {
                slot: 1,
                target: Some(-1),
                mega: false,
                dyna: false,
                tera: false,
                random_target: false,
            })
            .to_string(),
            "move 1,-1"
        );
        assert_eq!(
            Choice::Move(MoveChoice {
                slot: 2,
                target: Some(2),
                mega: true,
                dyna: false,
                tera: false,
                random_target: false,
            })
            .to_string(),
            "move 2,2,mega"
        );
        assert_eq!(
            Choice::Move(MoveChoice {
                slot: 3,
                target: None,
                mega: true,
                dyna: true,
                tera: true,
                random_target: false,
            })
            .to_string(),
            "move 3,mega,dyna,tera"
        );
        assert_eq!(
            Choice::Item(ItemChoice {
                item: "ball".to_owned(),
                target: None,
                additional_input: VecDeque::default(),
            })
            .to_string(),
            "item ball"
        );
        assert_eq!(
            Choice::Item(ItemChoice {
                item: "potion".to_owned(),
                target: Some(-1),
                additional_input: VecDeque::from_iter(["abc".to_owned(), "def".to_owned()]),
            })
            .to_string(),
            "item potion,-1,abc,def"
        );
        assert_eq!(
            Choice::LearnMove(LearnMoveChoice {
                forget_move_slot: 5
            })
            .to_string(),
            "learnmove 5"
        );
    }

    #[test]
    fn deserializes_from_string() {
        assert_matches::assert_matches!(Choice::from_str("pass"), Ok(Choice::Pass));
        assert_matches::assert_matches!(Choice::from_str("random"), Ok(Choice::Random));
        assert_matches::assert_matches!(Choice::from_str("randomall"), Ok(Choice::RandomAll));
        assert_matches::assert_matches!(Choice::from_str("escape"), Ok(Choice::Escape));
        assert_matches::assert_matches!(Choice::from_str("forfeit"), Ok(Choice::Forfeit));
        assert_matches::assert_matches!(Choice::from_str("shift"), Ok(Choice::Shift));
        assert_matches::assert_matches!(
            Choice::from_str("team 0 2 4"),
            Ok(Choice::Team(choice)) => {
                assert_eq!(choice, TeamSelectionChoice {
                    mons: Vec::from_iter([0, 2, 4]),
                });
            }
        );
        assert_matches::assert_matches!(
            Choice::from_str("switch 1"),
            Ok(Choice::Switch(choice)) => {
                assert_eq!(choice, SwitchChoice {
                    mon: Some(1),
                });
            }
        );
        assert_matches::assert_matches!(
            Choice::from_str("switch"),
            Ok(Choice::Switch(choice)) => {
                assert_eq!(choice, SwitchChoice {
                    mon: None,
                });
            }
        );
        assert_matches::assert_matches!(
            Choice::from_str("move 0"),
            Ok(Choice::Move(choice)) => {
                assert_eq!(choice, MoveChoice {
                    slot: 0,
                    target: None,
                    mega: false,
                    dyna: false,
                    tera: false,
                    random_target: false,
                });
            }
        );
        assert_matches::assert_matches!(
            Choice::from_str("move 1,-1"),
            Ok(Choice::Move(choice)) => {
                assert_eq!(choice, MoveChoice {
                    slot: 1,
                    target: Some(-1),
                    mega: false,
                    dyna: false,
                    tera: false,
                    random_target: false,
                });
            }
        );
        assert_matches::assert_matches!(
            Choice::from_str("move 2,2,mega"),
            Ok(Choice::Move(choice)) => {
                assert_eq!(choice, MoveChoice {
                    slot: 2,
                    target: Some(2),
                    mega: true,
                    dyna: false,
                    tera: false,
                    random_target: false,
                });
            }
        );
        assert_matches::assert_matches!(
            Choice::from_str("move 3,mega,dyna,tera"),
            Ok(Choice::Move(choice)) => {
                assert_eq!(choice, MoveChoice {
                    slot: 3,
                    target: None,
                    mega: true,
                    dyna: true,
                    tera: true,
                    random_target: false,
                });
            }
        );
        assert_matches::assert_matches!(
            Choice::from_str("item ball"),
            Ok(Choice::Item(choice)) => {
                assert_eq!(choice, ItemChoice {
                    item: "ball".to_owned(),
                    target: None,
                    additional_input: VecDeque::default(),
                });
            }
        );
        assert_matches::assert_matches!(
            Choice::from_str("item potion,-1,abc,def"),
            Ok(Choice::Item(choice)) => {
                assert_eq!(choice, ItemChoice {
                    item: "potion".to_owned(),
                    target: Some(-1),
                    additional_input: VecDeque::from_iter(["abc".to_owned(), "def".to_owned()]),
                });
            }
        );
        assert_matches::assert_matches!(
            Choice::from_str("learnmove 5"),
            Ok(Choice::LearnMove(choice)) => {
                assert_eq!(choice, LearnMoveChoice {
                    forget_move_slot: 5,
                });
            }
        );
    }

    #[test]
    fn serializes_multiple_to_string() {
        assert_eq!(
            choices_to_string([Choice::Move(MoveChoice {
                slot: 1,
                target: Some(2),
                ..Default::default()
            })]),
            "move 1,2"
        );
        assert_eq!(
            choices_to_string([
                Choice::Move(MoveChoice {
                    slot: 1,
                    target: Some(2),
                    ..Default::default()
                }),
                Choice::Switch(SwitchChoice { mon: Some(3) }),
                Choice::Forfeit,
            ]),
            "move 1,2;switch 3;forfeit"
        );
    }

    #[test]
    fn deserializes_multiple_from_string() {
        assert_matches::assert_matches!(choices_from_string("move 1,2"), Ok(choices) => {
            pretty_assertions::assert_eq!(choices, Vec::from_iter([Choice::Move(MoveChoice {
                slot: 1,
                target: Some(2),
                ..Default::default()
            })]));
        });
        assert_matches::assert_matches!(choices_from_string("move 1,2;switch 3;forfeit"), Ok(choices) => {
            pretty_assertions::assert_eq!(choices, Vec::from_iter([
                Choice::Move(MoveChoice {
                    slot: 1,
                    target: Some(2),
                    ..Default::default()
                }),
                Choice::Switch(SwitchChoice { mon: Some(3) }),
                Choice::Forfeit,
            ]));
        });
    }

    #[test]
    fn deserializes_multiple_results_from_string() {
        let choices = choice_results_from_string("move 1,2;switch abc");
        assert_eq!(choices.len(), 2);
        assert_matches::assert_matches!(&choices[0], Ok(choice) => {
            pretty_assertions::assert_eq!(choice, &Choice::Move(MoveChoice {
                slot: 1,
                target: Some(2),
                ..Default::default()
            }));
        });
        assert_matches::assert_matches!(&choices[1], Err(_));
    }
}
