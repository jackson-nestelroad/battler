use std::{
    fmt,
    fs,
    io,
};

use battler::PublicCoreBattle;
use itertools::Itertools;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
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

impl PartialEq for LogMatch {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LogMatch::Exact(s1), LogMatch::Exact(s2)) => s1 == s2,
            (LogMatch::Substrings(s1), LogMatch::Substrings(s2)) => {
                s1.iter().all(|sub| s2.contains(sub)) || s2.iter().all(|sub| s1.contains(sub))
            }
            (LogMatch::Exact(s), LogMatch::Substrings(subs))
            | (LogMatch::Substrings(subs), LogMatch::Exact(s)) => {
                subs.iter().all(|sub| s.contains(sub))
            }
        }
    }
}

/// Asserts that new logs in the battle are equal to the given logs.
#[track_caller]
pub fn assert_new_logs_eq(battle: &mut PublicCoreBattle, want: &[LogMatch]) {
    let got = battle.new_log_entries().collect::<Vec<&str>>();
    let want = want.into_iter().collect::<Vec<_>>();
    pretty_assertions::assert_eq!(want, got)
}

/// Asserts that logs since the start of the battle are equal to the given logs.
#[track_caller]
pub fn assert_logs_since_start_eq(battle: &PublicCoreBattle, want: &[LogMatch]) {
    let got = battle.full_log().collect::<Vec<&str>>();
    let start_log = "battlestart";
    let start_log_index = got.iter().position(|log| log == &start_log).unwrap();
    let start_log_index = start_log_index + 1;
    let got = &got[start_log_index..];
    let want = want.into_iter().collect::<Vec<_>>();
    pretty_assertions::assert_eq!(want, got)
}

/// Asserts that logs since the given turn in the battle are equal to the given logs.
#[track_caller]
pub fn assert_logs_since_turn_eq(battle: &PublicCoreBattle, turn: usize, want: &[LogMatch]) {
    let got = battle.full_log().collect::<Vec<&str>>();
    let turn_log = format!("turn|turn:{turn}");
    let turn_log_index = got.iter().position(|log| log == &&turn_log).unwrap();
    // Skip turn logs that are always present.
    let mut turn_log_index = turn_log_index + 1;
    if got[turn_log_index].starts_with("time") || got[turn_log_index].starts_with("continue") {
        turn_log_index = turn_log_index + 1;
    }
    let got = &got[turn_log_index..];
    let want = want.into_iter().collect::<Vec<_>>();
    pretty_assertions::assert_eq!(want, got)
}

/// Asserts that logs for the given turn in the battle are equal to the given logs.
#[track_caller]
pub fn assert_turn_logs_eq(battle: &PublicCoreBattle, turn: usize, want: &[LogMatch]) {
    let got = battle.full_log().collect::<Vec<_>>();
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

pub fn write_battle_log_to_file(file: &str, battle: &PublicCoreBattle) -> Result<(), io::Error> {
    fs::write(file, battle.full_log().join("\n"))
}
