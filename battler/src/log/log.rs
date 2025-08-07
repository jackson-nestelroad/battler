use std::{
    collections::hash_map::Entry,
    fmt,
    fmt::Display,
    mem,
};

use ahash::HashMap;
use itertools::Itertools;

/// Trait for objects that can be added directly to the battle log.
pub trait BattleLoggable {
    /// Logs the object into the entry.
    fn log(&self, entry: &mut UncommittedBattleLogEntry);
}

impl BattleLoggable for &str {
    fn log(&self, entry: &mut UncommittedBattleLogEntry) {
        entry.add_flag(*self)
    }
}

impl<T> BattleLoggable for (&str, T)
where
    T: Display,
{
    fn log(&self, entry: &mut UncommittedBattleLogEntry) {
        entry.set(self.0, &self.1)
    }
}

/// An uncommitted, mutable entry that is added to the [`BattleLog`] and can be modified after the
/// fact.
///
/// This object should not be constructed directly. Instead, use the
/// [`battle_log_entry`][`crate::battle_log_entry`] macro.
#[derive(Debug, Clone)]
pub struct UncommittedBattleLogEntry {
    title: String,
    values: HashMap<String, String>,
    insertion_order: Vec<String>,
}

impl UncommittedBattleLogEntry {
    /// Creates a new entry with the given title.
    pub fn new<T>(title: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            title: title.into(),
            values: HashMap::default(),
            insertion_order: Vec::new(),
        }
    }

    /// Adds the given value to the entry.
    pub fn extend<T>(&mut self, value: &T)
    where
        T: BattleLoggable,
    {
        value.log(self)
    }

    fn add_entry<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Display,
    {
        match self.values.entry(key.into()) {
            Entry::Occupied(mut entry) => *entry.get_mut() = format!("{value}"),
            Entry::Vacant(entry) => {
                let entry = entry.insert_entry(format!("{value}"));
                self.insertion_order.push(entry.key().clone());
            }
        }
    }

    /// Adds a new flag (a property with no value) to the entry.
    pub fn add_flag<K>(&mut self, key: K)
    where
        K: Into<String>,
    {
        self.add_entry(key, "")
    }

    /// Sets the value of a property on the entry.
    ///
    /// If the property did not exist before, the pair is added. If the property did exist, the
    /// value is updated.
    pub fn set<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Display,
    {
        self.add_entry(key, value)
    }

    /// Removes the property.
    pub fn remove(&mut self, key: &str) {
        self.values.remove(key);
    }

    fn commit(&self) -> String {
        let values = self
            .insertion_order
            .iter()
            .filter_map(|key| self.values.get_key_value(key))
            .map(|(key, value)| {
                if value.is_empty() {
                    key.clone()
                } else {
                    format!("{key}:{value}")
                }
            })
            .join("|");

        if values.is_empty() {
            self.title.clone()
        } else {
            format!("{}|{}", self.title, values)
        }
    }
}

impl Display for UncommittedBattleLogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.commit())
    }
}

/// Constructs a [`UncommittedBattleLogEntry`] to be added to the [`BattleLog`].
///
/// This macro enforces a common format for all messages in the battle log.
#[macro_export]
macro_rules! battle_log_entry {
    ($title:expr) => {{
        $crate::log::UncommittedBattleLogEntry::new($title)
    }};
    ($title:expr $(, $entries:expr)+ $(,)?) => {{
        let mut entry = $crate::log::UncommittedBattleLogEntry::new($title);
        $($crate::log::BattleLoggable::log(&$entries, &mut entry);)*
        entry
    }};
}

/// Battle log entry.
#[derive(Debug)]
pub enum BattleLogEntry<'e> {
    Committed(&'e str),
    Uncommitted(&'e UncommittedBattleLogEntry),
}

impl BattleLogEntry<'_> {
    /// Is the log entry committed and published for clients?
    pub fn committed(&self) -> bool {
        match self {
            Self::Committed(_) => true,
            Self::Uncommitted(_) => false,
        }
    }
}

impl Display for BattleLogEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Committed(entry) => write!(f, "{entry}"),
            Self::Uncommitted(entry) => write!(f, "{entry}"),
        }
    }
}

/// Mutable battle log entry.
///
/// Only uncommitted logs are mutable.
#[derive(Debug)]
pub enum BattleLogEntryMut<'e> {
    Committed(&'e str),
    Uncommitted(&'e mut UncommittedBattleLogEntry),
}

impl BattleLogEntryMut<'_> {
    pub fn committed(&self) -> bool {
        match self {
            Self::Committed(_) => true,
            Self::Uncommitted(_) => false,
        }
    }
}

impl Display for BattleLogEntryMut<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Committed(entry) => write!(f, "{entry}"),
            Self::Uncommitted(entry) => write!(f, "{entry}"),
        }
    }
}

/// A log of battle events that can be exported.
///
/// When a new [`BattleLogEntry`] is added, it is considered uncommitted and is mutable (see
/// [`BattleLogEntryMut`]). Logs must be manually committed using the [`Self::commit`] method in
/// order to appear for clients. Once a log is committed, it is considered immutable.
///
/// The log also keeps track of reads. Once a log is read out using [`Self::read_out`], it will not
/// be read out again.
pub struct BattleLog {
    last_read: usize,
    committed_logs: Vec<String>,
    new_log_entries: Vec<UncommittedBattleLogEntry>,
}

impl BattleLog {
    /// Creates a new battle log.
    pub fn new() -> Self {
        Self {
            last_read: 0,
            committed_logs: Vec::new(),
            new_log_entries: Vec::new(),
        }
    }

    /// Does the log contain new messages since the last call to [`Self::read_out`]?
    pub fn has_new_messages(&self) -> bool {
        !self.new_log_entries.is_empty()
    }

    /// Pushes a new entry to the log.
    pub fn push(&mut self, entry: UncommittedBattleLogEntry) {
        self.new_log_entries.push(entry)
    }

    /// Pushes multiple entries to the log.
    pub fn push_extend<I>(&mut self, iterable: I)
    where
        I: IntoIterator<Item = UncommittedBattleLogEntry>,
    {
        self.new_log_entries.extend(iterable.into_iter());
    }

    /// Commits all uncommitted logs.
    pub fn commit(&mut self) {
        let new_log_entries = mem::replace(&mut self.new_log_entries, Vec::new());
        self.committed_logs
            .extend(new_log_entries.into_iter().map(|entry| entry.commit()))
    }

    /// Returns an iterator over all committed logs.
    pub fn logs(&self) -> impl Iterator<Item = &str> {
        self.committed_logs.iter().map(|s| s.as_ref())
    }

    /// Reads out any new, committed logs that have been added since the last call to
    /// [`Self::read_out`].
    pub fn read_out(&mut self) -> impl Iterator<Item = &str> {
        let i = self.last_read;
        self.last_read = self.committed_logs.len();
        self.committed_logs[i..].iter().map(|s| s.as_ref())
    }

    /// Returns the total number of log entries.
    pub fn len(&self) -> usize {
        self.committed_logs.len() + self.new_log_entries.len()
    }

    /// Returns a reference to the log entry at the given index.
    pub fn get(&self, index: usize) -> Option<BattleLogEntry<'_>> {
        self.committed_logs
            .get(index)
            .map(|s| BattleLogEntry::Committed(s.as_ref()))
            .or_else(|| {
                index
                    .checked_sub(self.committed_logs.len())
                    .and_then(|index| {
                        self.new_log_entries
                            .get(index)
                            .map(|entry| BattleLogEntry::Uncommitted(entry))
                    })
            })
    }

    /// Returns a mutable reference to the log entry at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<BattleLogEntryMut<'_>> {
        self.committed_logs
            .get(index)
            .map(|s| BattleLogEntryMut::Committed(s.as_ref()))
            .or_else(|| {
                index
                    .checked_sub(self.committed_logs.len())
                    .and_then(|index| {
                        self.new_log_entries
                            .get_mut(index)
                            .map(|entry| BattleLogEntryMut::Uncommitted(entry))
                    })
            })
    }
}

#[cfg(test)]
mod battle_log_test {
    use std::{
        fmt,
        fmt::Display,
    };

    use crate::log::{
        BattleLog,
        BattleLogEntryMut,
        BattleLoggable,
        UncommittedBattleLogEntry,
    };

    fn last_log(log: &mut BattleLog) -> String {
        log.get(log.len() - 1).unwrap().to_string()
    }

    struct CustomData {
        a: u32,
        b: String,
    }

    impl Display for CustomData {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{} => {}", self.a, self.b)
        }
    }

    #[test]
    fn formats_entries() {
        let mut log = BattleLog::new();

        log.push(battle_log_entry!("a", ("b", "c")));
        assert_eq!(last_log(&mut log), "a|b:c");

        log.push(battle_log_entry!("time", ("time", 100000i32), "real"));
        assert_eq!(last_log(&mut log), "time|time:100000|real");

        log.push(battle_log_entry!(
            "customdata",
            ("pi", 3.1415926535f64),
            (
                "data",
                CustomData {
                    a: 234,
                    b: "bulbasaur".to_owned(),
                },
            ),
            ("a", 0i32),
            ("b", 1i32),
            ("c", 0i32),
        ));
        assert_eq!(
            last_log(&mut log),
            "customdata|pi:3.1415926535|data:234 => bulbasaur|a:0|b:1|c:0"
        );
    }

    struct CustomDataWithLogImplementation {
        a: u32,
        b: String,
    }

    impl BattleLoggable for CustomDataWithLogImplementation {
        fn log(&self, log: &mut UncommittedBattleLogEntry) {
            log.set("a", format!("{}", self.a));
            log.set("b", &self.b);
        }
    }

    #[test]
    fn allows_custom_implementation() {
        let mut log = BattleLog::new();

        log.push(battle_log_entry!(
            "customdata",
            "abc",
            CustomDataWithLogImplementation {
                a: 234,
                b: "bulbasaur".to_owned(),
            },
            ("val", 0i32),
        ));
        assert_eq!(last_log(&mut log), "customdata|abc|a:234|b:bulbasaur|val:0");
    }

    #[test]
    fn records_length() {
        let mut log = BattleLog::new();
        assert_eq!(log.len(), 0);
        log.push(battle_log_entry!("one"));
        assert_eq!(log.len(), 1);
        log.push(battle_log_entry!("two", "three", "four"));
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn returns_entry_by_index() {
        let mut log = BattleLog::new();
        assert!(log.get(0).is_none());
        log.push(battle_log_entry!("one"));
        assert_eq!(log.get(0).unwrap().to_string(), "one");
        assert!(log.get(1).is_none());
        log.push(battle_log_entry!("two", "three", "four"));
        assert_eq!(log.get(1).unwrap().to_string(), "two|three|four");
    }

    #[test]
    fn reads_out_new_committed_logs() {
        let mut log = BattleLog::new();
        assert!(log.read_out().collect::<Vec<_>>().is_empty());
        log.push(battle_log_entry!("one"));
        log.push(battle_log_entry!("two", "three", "four"));
        assert!(log.read_out().collect::<Vec<_>>().is_empty());
        log.commit();
        assert_eq!(
            log.read_out().collect::<Vec<_>>(),
            vec!["one", "two|three|four"]
        );
        assert!(log.read_out().collect::<Vec<_>>().is_empty());
        log.push(battle_log_entry!("five", "six"));
        log.push(battle_log_entry!("seven", "eight"));
        assert!(log.read_out().collect::<Vec<_>>().is_empty());
        log.commit();
        assert_eq!(
            log.read_out().collect::<Vec<_>>(),
            vec!["five|six", "seven|eight"]
        );
        assert!(log.read_out().collect::<Vec<_>>().is_empty());
    }

    #[test]
    fn returns_entry_type() {
        let mut log = BattleLog::new();
        log.push(battle_log_entry!("move", "tackle"));
        assert!(log.get(0).is_some_and(|entry| !entry.committed()));
        log.commit();
        assert!(log.get(0).is_some_and(|entry| entry.committed()));

        log.push(battle_log_entry!("move", "name:tackle", "bad"));
        if let Some(BattleLogEntryMut::Uncommitted(entry)) = log.get_mut(1) {
            entry.add_flag("noanim");
            entry.remove("bad");
            entry.set("damage", 12);
            entry.extend(&CustomDataWithLogImplementation {
                a: 1,
                b: "2".to_owned(),
            });
        }
        assert!(log.get(1).is_some_and(|entry| {
            entry
                .to_string()
                .eq("move|name:tackle|noanim|damage:12|a:1|b:2")
        }));

        log.commit();
        assert_eq!(
            log.read_out().collect::<Vec<_>>(),
            vec!["move|tackle", "move|name:tackle|noanim|damage:12|a:1|b:2"]
        );
    }
}
