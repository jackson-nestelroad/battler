use std::{
    fs,
    io,
};

use battler::battle::{
    Battle,
    BattleOptions,
    PublicCoreBattle,
};
use itertools::Itertools;

fn log_is_random(log: &str) -> bool {
    log.starts_with("time")
}

/// Asserts that new logs in the battle are equal to the given logs.
#[track_caller]
pub fn assert_new_logs_eq<'d, B, O>(battle: &mut B, want: &[&str])
where
    O: BattleOptions,
    B: Battle<'d, O>,
{
    let got = battle
        .new_logs()
        .filter(|log| !log_is_random(log))
        .collect::<Vec<&str>>();
    let want = want
        .into_iter()
        .filter(|log| !log_is_random(log))
        .map(|log| *log)
        .collect::<Vec<_>>();
    pretty_assertions::assert_eq!(got, want)
}

pub fn write_battle_logs_to_file(file: &str, battle: &PublicCoreBattle) -> Result<(), io::Error> {
    fs::write(file, battle.all_logs().join("\n"))
}
