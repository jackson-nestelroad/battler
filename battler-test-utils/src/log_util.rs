use std::{
    fmt,
    fs,
    io,
};

use battler::battle::{
    Battle,
    BattleOptions,
    PublicCoreBattle,
};
use itertools::Itertools;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum LogMatch {
    Exact(String),
    Substrings(Vec<String>),
}

impl fmt::Debug for LogMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(str) => write!(f, "\"{str}\""),
            Self::Substrings(strs) => write!(
                f,
                "substrings:{}",
                strs.iter().map(|str| format!("\"{str}\"")).join(";")
            ),
        }
    }
}

impl PartialEq<str> for LogMatch {
    fn eq(&self, other: &str) -> bool {
        match self {
            Self::Exact(str) => str.eq(&other),
            Self::Substrings(strs) => strs.iter().all(|str| other.contains(str)),
        }
    }
}

impl PartialEq<String> for LogMatch {
    fn eq(&self, other: &String) -> bool {
        self.eq(other.as_str())
    }
}

impl PartialEq<&str> for LogMatch {
    fn eq(&self, other: &&str) -> bool {
        self.eq(*other)
    }
}

/// Asserts that new logs in the battle are equal to the given logs.
#[track_caller]
pub fn assert_new_logs_eq<'d, B, O>(battle: &mut B, want: &[LogMatch])
where
    B: Battle<'d, O>,
    O: BattleOptions,
{
    let got = battle.new_logs().collect::<Vec<&str>>();
    let want = want.into_iter().collect::<Vec<_>>();
    pretty_assertions::assert_eq!(want, got)
}

/// Asserts that new logs in the battle are equal to the given logs.
#[track_caller]
pub fn assert_turn_logs_eq<'d, B, O>(battle: &mut B, turn: usize, want: &[LogMatch])
where
    B: Battle<'d, O>,
    O: BattleOptions,
{
    let got = battle.all_logs().collect::<Vec<_>>();
    let turn_log = format!("turn|turn:{turn}");
    let next_turn_log = format!("turn|turn:{}", turn + 1);
    let turn_log_index = got.iter().position(|log| log == &&turn_log).unwrap();
    // Skip turn logs that are always present.
    let turn_log_index = turn_log_index + 2;
    let next_turn_log_index = got
        .iter()
        .position(|log| log == &&next_turn_log)
        .unwrap_or(got.len());
    let got = &got[turn_log_index..next_turn_log_index];
    let want = want.into_iter().collect::<Vec<_>>();
    pretty_assertions::assert_eq!(want, got)
}

pub fn write_battle_logs_to_file(file: &str, battle: &PublicCoreBattle) -> Result<(), io::Error> {
    fs::write(file, battle.all_logs().join("\n"))
}
