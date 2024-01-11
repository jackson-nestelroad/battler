use std::{
    collections::hash_map::Entry,
    fmt,
    fmt::Display,
    mem,
};

use ahash::HashMapExt;
use itertools::Itertools;

use crate::common::FastHashMap;

/// Trait for objects that can be added directly to the event log.
pub trait EventLoggable {
    fn log(&self, event: &mut Event);
}

impl EventLoggable for &str {
    fn log(&self, event: &mut Event) {
        event.add_flag(*self)
    }
}

impl<T> EventLoggable for (&str, T)
where
    T: Display,
{
    fn log(&self, event: &mut Event) {
        event.set(self.0, &self.1)
    }
}

/// An event that is added to the [`EventLog`].
///
/// This object should not be constructed directly. Instead, use the [`log_event`] macro.
#[derive(Debug)]
pub struct Event {
    title: String,
    values: FastHashMap<String, String>,
    insertion_order: Vec<String>,
}

impl Event {
    /// Creates a new event with the given title.
    pub fn new<T>(title: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            title: title.into(),
            values: FastHashMap::new(),
            insertion_order: Vec::new(),
        }
    }

    pub fn extend<T>(&mut self, value: &T)
    where
        T: EventLoggable,
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

    /// Adds a new flag (a property with no value) to the event.
    pub fn add_flag<K>(&mut self, key: K)
    where
        K: Into<String>,
    {
        self.add_entry(key, "")
    }

    /// Sets the value of a property on the event.
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

impl Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.commit())
    }
}

/// Constructs an [`Event`] to be added to the [`EventLog`].
///
/// This macro enforces a common format for all messages in the event log.
#[macro_export]
macro_rules! log_event {
    ($title:expr) => {{
        $crate::log::Event::new($title)
    }};
    ($title:expr $(, $entries:expr)+ $(,)?) => {{
        let mut event = $crate::log::Event::new($title);
        $($crate::log::EventLoggable::log(&$entries, &mut event);)*
        event
    }};
}

/// Event log entry.
#[derive(Debug)]
pub enum EventLogEntry<'e> {
    Committed(&'e str),
    Uncommitted(&'e Event),
}

impl EventLogEntry<'_> {
    pub fn committed(&self) -> bool {
        match self {
            Self::Committed(_) => true,
            Self::Uncommitted(_) => false,
        }
    }
}

impl Display for EventLogEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Committed(event) => write!(f, "{event}"),
            Self::Uncommitted(event) => write!(f, "{event}"),
        }
    }
}

/// Mutable event log entry.
///
/// Only uncommitted logs are mutable.
#[derive(Debug)]
pub enum EventLogEntryMut<'e> {
    Committed(&'e str),
    Uncommitted(&'e mut Event),
}

impl EventLogEntryMut<'_> {
    pub fn committed(&self) -> bool {
        match self {
            Self::Committed(_) => true,
            Self::Uncommitted(_) => false,
        }
    }
}

impl Display for EventLogEntryMut<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Committed(event) => write!(f, "{event}"),
            Self::Uncommitted(event) => write!(f, "{event}"),
        }
    }
}

/// A log of events that can be exported.
///
/// When a new [`Event`] is added, it is considered uncommitted and is mutable (see
/// [`EventLogEntryMut`]). Logs must be manually committed using the [`Self::commit`] method.
/// Once a log is committed, it is considered immutable.
///
/// The log also keeps track of reads. Once a log is read out using [`Self::read_out`], it will not
/// be read out again.
pub struct EventLog {
    last_read: usize,
    committed_logs: Vec<String>,
    new_logs: Vec<Event>,
}

impl EventLog {
    /// Creates a new event log.
    pub fn new() -> Self {
        Self {
            last_read: 0,
            committed_logs: Vec::new(),
            new_logs: Vec::new(),
        }
    }

    /// Does the log contain new messages since the last call to [`Self::read_out`]?
    pub fn has_new_messages(&self) -> bool {
        !self.new_logs.is_empty()
    }

    /// Pushes a new event to the log.
    pub fn push(&mut self, event: Event) {
        self.new_logs.push(event)
    }

    /// Pushes multiple events to the log.
    pub fn push_extend<I>(&mut self, iterable: I)
    where
        I: IntoIterator<Item = Event>,
    {
        self.new_logs.extend(iterable.into_iter());
    }

    /// Commits all uncommitted logs.
    pub fn commit(&mut self) {
        let new_logs = mem::replace(&mut self.new_logs, Vec::new());
        self.committed_logs
            .extend(new_logs.into_iter().map(|event| event.commit()))
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
        self.committed_logs.len() + self.new_logs.len()
    }

    /// Returns a reference to the log entry at the given index.
    pub fn get(&self, index: usize) -> Option<EventLogEntry> {
        self.committed_logs
            .get(index)
            .map(|s| EventLogEntry::Committed(s.as_ref()))
            .or_else(|| {
                index
                    .checked_sub(self.committed_logs.len())
                    .and_then(|index| {
                        self.new_logs
                            .get(index)
                            .map(|event| EventLogEntry::Uncommitted(event))
                    })
            })
    }

    /// Returns a mutable reference to the log entry at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<EventLogEntryMut> {
        self.committed_logs
            .get(index)
            .map(|s| EventLogEntryMut::Committed(s.as_ref()))
            .or_else(|| {
                index
                    .checked_sub(self.committed_logs.len())
                    .and_then(|index| {
                        self.new_logs
                            .get_mut(index)
                            .map(|event| EventLogEntryMut::Uncommitted(event))
                    })
            })
    }
}

#[cfg(test)]
mod event_log_tests {
    use std::{
        fmt,
        fmt::Display,
    };

    use crate::log::{
        Event,
        EventLog,
        EventLogEntryMut,
        EventLoggable,
    };

    fn last_log(log: &mut EventLog) -> String {
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
    fn formats_events() {
        let mut log = EventLog::new();

        log.push(log_event!("a", ("b", "c")));
        assert_eq!(last_log(&mut log), "a|b:c");

        log.push(log_event!("time", ("time", 100000i32), "real"));
        assert_eq!(last_log(&mut log), "time|time:100000|real");

        log.push(log_event!(
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

    impl EventLoggable for CustomDataWithLogImplementation {
        fn log(&self, log: &mut Event) {
            log.set("a", format!("{}", self.a));
            log.set("b", &self.b);
        }
    }

    #[test]
    fn allows_custom_implementation() {
        let mut log = EventLog::new();

        log.push(log_event!(
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
        let mut log = EventLog::new();
        assert_eq!(log.len(), 0);
        log.push(log_event!("one"));
        assert_eq!(log.len(), 1);
        log.push(log_event!("two", "three", "four"));
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn returns_entry_by_index() {
        let mut log = EventLog::new();
        assert!(log.get(0).is_none());
        log.push(log_event!("one"));
        assert_eq!(log.get(0).unwrap().to_string(), "one");
        assert!(log.get(1).is_none());
        log.push(log_event!("two", "three", "four"));
        assert_eq!(log.get(1).unwrap().to_string(), "two|three|four");
    }

    #[test]
    fn reads_out_new_committed_logs() {
        let mut log = EventLog::new();
        assert!(log.read_out().collect::<Vec<_>>().is_empty());
        log.push(log_event!("one"));
        log.push(log_event!("two", "three", "four"));
        assert!(log.read_out().collect::<Vec<_>>().is_empty());
        log.commit();
        assert_eq!(
            log.read_out().collect::<Vec<_>>(),
            vec!["one", "two|three|four"]
        );
        assert!(log.read_out().collect::<Vec<_>>().is_empty());
        log.push(log_event!("five", "six"));
        log.push(log_event!("seven", "eight"));
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
        let mut log = EventLog::new();
        log.push(log_event!("move", "tackle"));
        assert!(log.get(0).is_some_and(|event| !event.committed()));
        log.commit();
        assert!(log.get(0).is_some_and(|event| event.committed()));

        log.push(log_event!("move", "name:tackle", "bad"));
        if let Some(EventLogEntryMut::Uncommitted(event)) = log.get_mut(1) {
            event.add_flag("noanim");
            event.remove("bad");
            event.set("damage", 12);
        }
        assert!(log
            .get(1)
            .is_some_and(|event| event.to_string().eq("move|name:tackle|noanim|damage:12")));

        log.commit();
        assert_eq!(
            log.read_out().collect::<Vec<_>>(),
            vec!["move|tackle", "move|name:tackle|noanim|damage:12"]
        );
    }
}
