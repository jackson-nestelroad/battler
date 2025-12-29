use std::str::FromStr;

use ahash::HashMap;
use anyhow::{
    Context,
    Error,
    Result,
};

use crate::ui::Effect;

/// The name of a Mon, part of a [`LogEntry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonName {
    pub name: String,
    pub player: String,
    pub position: Option<usize>,
}

impl FromStr for MonName {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse backwards, since the Mon name could have a comma.
        let reversed = s.chars().rev().collect::<String>();
        let mut parts = reversed.splitn(3, ',').peekable();
        let (name, player, position) = if parts
            .peek()
            .is_some_and(|part| part.chars().all(|c| c.is_digit(10)))
        {
            let position = parts
                .next()
                .ok_or_else(|| Error::msg("missing position"))?
                .chars()
                .rev()
                .collect::<String>()
                .parse()
                .context("invalid position")?;
            let player = parts
                .next()
                .ok_or_else(|| Error::msg("missing player"))?
                .chars()
                .rev()
                .collect();
            let name = parts
                .next()
                .ok_or_else(|| Error::msg("missing name"))?
                .chars()
                .rev()
                .collect();
            (name, player, Some(position))
        } else {
            let mut parts = reversed.splitn(2, ',').peekable();

            let player = parts
                .next()
                .ok_or_else(|| Error::msg("missing player"))?
                .chars()
                .rev()
                .collect();
            let name = parts
                .next()
                .ok_or_else(|| Error::msg("missing name"))?
                .chars()
                .rev()
                .collect();
            (name, player, None)
        };

        Ok(Self {
            name,
            player,
            position,
        })
    }
}

/// A list of Mons, part of a [`LogEntry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonNameList(pub Vec<MonName>);

impl FromStr for MonNameList {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            s.split(';')
                .map(|s| s.parse::<MonName>())
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

/// The name of an effect, part of a [`LogEntry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectName {
    pub effect_type: Option<String>,
    pub name: String,
}

impl Into<Effect> for EffectName {
    fn into(self) -> Effect {
        Effect {
            effect_type: self.effect_type,
            name: self.name,
        }
    }
}

impl FromStr for EffectName {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (effect_type, name) = s
            .split_once(':')
            .map(|(k, v)| (Some(k.to_owned()), v.to_owned()))
            .unwrap_or_else(|| (None, s.to_owned()));
        Ok(Self { effect_type, name })
    }
}

/// The entry of a battle [`Log`].
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LogEntry {
    title: String,
    values: HashMap<String, String>,
}

impl LogEntry {
    /// The log title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// All values in the log entry.
    pub fn values(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Returns a value out of the log entry.
    pub fn value_ref(&self, value: &str) -> Option<&str> {
        self.values.get(value).map(|s| s.as_str())
    }

    /// Parses a value out of the log entry.
    pub fn value<T>(&self, value: &str) -> Option<T>
    where
        T: FromStr,
    {
        self.values.get(value).map(|val| val.parse().ok()).flatten()
    }

    /// Parses a value out of the log entry.
    pub fn value_or_else<T>(&self, value: &str) -> Result<T>
    where
        T: FromStr,
    {
        self.value(value)
            .ok_or_else(|| Error::msg(format!("expected {value}")))
    }
}

impl FromStr for LogEntry {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut values = s.split('|');
        let title = values.next().unwrap_or_default().to_owned();
        let values = values
            .map(|val| {
                val.split_once(':')
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .unwrap_or((val.to_owned(), String::default()))
            })
            .collect();
        Ok(Self { title, values })
    }
}

#[derive(Debug)]
pub struct Log {
    entries: Vec<LogEntry>,
    turns: Vec<usize>,
    filled_up_to: usize,
    last_checked_for_turn: usize,
}

impl Log {
    /// Creates a new log over an iterator of entries.
    pub fn new<I, T>(iter: I) -> Result<Self>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let entries = iter
            .into_iter()
            .map(|entry| entry.as_ref().parse())
            .collect::<Result<Vec<_>>>()?;
        let mut log = Self {
            entries,
            turns: Vec::from_iter([0]),
            filled_up_to: 0,
            last_checked_for_turn: 0,
        };
        log.update();
        Ok(log)
    }

    /// The number of entries in the log.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    fn update(&mut self) {
        self.check_filled();
        self.check_next_turn();
    }

    fn check_filled(&mut self) {
        for i in self.filled_up_to..self.entries.len() {
            // SAFETY: i iterates up to the length of self.entries.
            if self.entries.get(i).unwrap().title().is_empty() {
                break;
            }
            self.filled_up_to = i;
        }
    }

    fn check_next_turn(&mut self) {
        for i in self.last_checked_for_turn..self.entries.len() {
            // SAFETY: i iterates up to the length of self.entries.
            let entry = self.entries.get(i).unwrap();
            if entry.title() == "turn" {
                if let Some(turn) = entry.value::<usize>("turn") {
                    self.turns.resize_with(turn + 1, usize::default);
                    // SAFETY: Resize above ensures this index is valid.
                    *self.turns.get_mut(turn).unwrap() = i;
                }
            }
            self.last_checked_for_turn = i;
        }
    }

    /// Adds an entry to the log at the given index.
    pub fn add<S>(&mut self, index: usize, content: S) -> Result<()>
    where
        S: AsRef<str>,
    {
        if index + 1 > self.entries.len() {
            self.entries.resize_with(index + 1, LogEntry::default);
        }
        // SAFETY: Resize above ensures this index is valid.
        *self.entries.get_mut(index).unwrap() = content.as_ref().parse()?;
        self.update();
        Ok(())
    }

    /// Extends the log.
    pub fn extend<I, S>(&mut self, iter: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.entries.extend(
            iter.into_iter()
                .map(|content| content.as_ref().parse())
                .collect::<Result<Vec<_>>>()?,
        );
        self.update();
        Ok(())
    }

    /// Checks if all entries are filled.
    ///
    /// If false, the log requires reconstructing for middle values.
    pub fn filled(&self) -> bool {
        self.entries.is_empty()
            || (self.filled_up_to == self.entries.len() - 1
                && self
                    .entries
                    .iter()
                    .all(|entry| *entry != LogEntry::default()))
    }

    /// Returns all log entries.
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// Returns log entries for a given turn.
    pub fn entries_for_turn(&self, turn: usize, min_index: Option<usize>) -> &[LogEntry] {
        match self.turns.get(turn).cloned() {
            Some(i) => {
                let begin = match min_index {
                    Some(min) => min.max(i),
                    None => i,
                };
                let end = self
                    .turns
                    .get(turn + 1)
                    .cloned()
                    .unwrap_or_else(|| self.entries.len());
                let end = end.max(begin);
                &self.entries[begin..end]
            }
            None => &[],
        }
    }

    /// The current turn according to the log.
    ///
    /// This turn is not finished; more entries corresponding to this turn may be coming in.
    pub fn current_turn(&self) -> usize {
        self.turns.len().saturating_sub(1)
    }
}

#[cfg(test)]
mod log_test {
    use crate::log::{
        Log,
        LogEntry,
    };

    #[test]
    fn constructs_from_full_log() {
        let log = Log::new(&[
            "info|battletype:Singles",
            "info|environment:Normal|time:Day",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:1",
            "teamsize|player:player-2|size:1",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5",
            "turn|turn:1",
            "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
            "damage|mon:Charmander,player-2,1|health:86/100",
            "residual",
            "turn|turn:2",
            "move|mon:Charmander,player-2,1|name:Scratch|target:Squirtle,player-1,1",
            "damage|mon:Squirtle,player-1,1|health:86/100",
            "residual",
            "turn|turn:3",
        ])
        .unwrap();

        assert!(log.filled());

        assert_matches::assert_matches!(log.entries_for_turn(0, None).first(), Some(entry) => {
            assert_eq!(entry.title(), "info");
        });
        assert_matches::assert_matches!(log.entries_for_turn(0, None).last(), Some(entry) => {
            assert_eq!(entry.title(), "switch");
        });

        pretty_assertions::assert_eq!(
            log.entries_for_turn(1, None),
            [
                "turn|turn:1",
                "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
                "damage|mon:Charmander,player-2,1|health:86/100",
                "residual",
            ]
            .into_iter()
            .map(|s| s.parse::<LogEntry>())
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        );

        pretty_assertions::assert_eq!(
            log.entries_for_turn(2, None),
            [
                "turn|turn:2",
                "move|mon:Charmander,player-2,1|name:Scratch|target:Squirtle,player-1,1",
                "damage|mon:Squirtle,player-1,1|health:86/100",
                "residual",
            ]
            .into_iter()
            .map(|s| s.parse::<LogEntry>())
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        );

        pretty_assertions::assert_eq!(
            log.entries_for_turn(3, None),
            ["turn|turn:3"]
                .into_iter()
                .map(|s| s.parse::<LogEntry>())
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        );

        pretty_assertions::assert_eq!(log.entries_for_turn(4, None), []);
    }

    #[test]
    fn avoids_old_log_entries_with_min_index() {
        let log = Log::new(&[
            "info|battletype:Singles",
            "info|environment:Normal|time:Day",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:1",
            "teamsize|player:player-2|size:1",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5",
            "turn|turn:1",
            "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
            "damage|mon:Charmander,player-2,1|health:86/100",
            "residual",
            "turn|turn:2",
            "move|mon:Charmander,player-2,1|name:Scratch|target:Squirtle,player-1,1",
            "damage|mon:Squirtle,player-1,1|health:86/100",
            "residual",
            "turn|turn:3",
        ])
        .unwrap();

        assert!(log.filled());

        assert_matches::assert_matches!(log.entries_for_turn(0, Some(2)).first(), Some(entry) => {
            assert_eq!(entry.title(), "side");
        });
    }

    #[test]
    fn returns_log_entries_for_turn_0_before_turn_1() {
        let log = Log::new(&[
            "info|battletype:Singles",
            "info|environment:Normal|time:Day",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:1",
            "teamsize|player:player-2|size:1",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5",
        ])
        .unwrap();

        assert!(log.filled());

        assert_matches::assert_matches!(log.entries_for_turn(0, None).first(), Some(entry) => {
            assert_eq!(entry.title(), "info");
        });
        assert_matches::assert_matches!(log.entries_for_turn(0, None).last(), Some(entry) => {
            assert_eq!(entry.title(), "switch");
        });
    }

    #[test]
    fn adds_new_sequential_log() {
        let mut log = Log::new(&["turn|turn:1"]).unwrap();

        assert!(log.filled());
        pretty_assertions::assert_eq!(
            log.entries_for_turn(1, None),
            ["turn|turn:1"]
                .into_iter()
                .map(|s| s.parse::<LogEntry>())
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        );

        assert_matches::assert_matches!(
            log.add(
                1,
                "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
            ),
            Ok(())
        );
        assert!(log.filled());
        pretty_assertions::assert_eq!(
            log.entries_for_turn(1, None),
            [
                "turn|turn:1",
                "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
            ]
            .into_iter()
            .map(|s| s.parse::<LogEntry>())
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        );
    }

    #[test]
    fn adds_new_non_sequential_log() {
        let mut log = Log::new(&["turn|turn:1"]).unwrap();

        assert!(log.filled());
        pretty_assertions::assert_eq!(
            log.entries_for_turn(1, None),
            ["turn|turn:1"]
                .into_iter()
                .map(|s| s.parse::<LogEntry>())
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        );

        assert_matches::assert_matches!(
            log.add(2, "damage|mon:Charmander,player-2,1|health:86/100",),
            Ok(())
        );
        assert!(!log.filled());
        pretty_assertions::assert_eq!(
            log.entries_for_turn(1, None),
            [
                "turn|turn:1",
                "",
                "damage|mon:Charmander,player-2,1|health:86/100",
            ]
            .into_iter()
            .map(|s| s.parse::<LogEntry>())
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        );

        // Fill in the missing log.
        assert_matches::assert_matches!(
            log.add(
                1,
                "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
            ),
            Ok(())
        );
        assert!(log.filled());
        pretty_assertions::assert_eq!(
            log.entries_for_turn(1, None),
            [
                "turn|turn:1",
                "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
                "damage|mon:Charmander,player-2,1|health:86/100",
            ]
            .into_iter()
            .map(|s| s.parse::<LogEntry>())
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        );
    }
}
