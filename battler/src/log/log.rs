use std::{
    borrow::Cow,
    fmt::Display,
    mem,
};

use itertools::Itertools;

/// Trait for objects that can be added directly to the battle log.
///
/// Automatically implemented for types that implement [`Display`].
pub trait BattleLoggable {
    fn log<'s>(&'s self, items: &mut Vec<Cow<'s, str>>);
}

impl<T> BattleLoggable for T
where
    T: Display,
{
    fn log(&self, parts: &mut Vec<Cow<'_, str>>) {
        parts.push(Cow::Owned(format!("{self}")))
    }
}

/// A battle event that is added to the [`EventLog`].
///
/// This object should not be constructed directly. Instead, use the [`battle_event`] macro.
pub struct BattleEvent(String);

impl BattleEvent {
    pub fn from_parts(parts: &[&dyn BattleLoggable]) -> Self {
        let mut log_parts = Vec::with_capacity(parts.len());
        for part in parts {
            part.log(&mut log_parts);
        }
        Self(log_parts.into_iter().join("|"))
    }
}

/// Constructs a [`BattleEvent`] to be added to the [`EventLog`].
///
/// This macro enforces a common format for all messages in the event log.
#[macro_export]
macro_rules! battle_event {
    ($($arg:expr),* $(,)?) => {{
        $crate::log::BattleEvent::from_parts(&[$(&$arg),*])
    }};
}

/// A log of battle events that can be exported.
pub struct EventLog {
    logs: Vec<String>,
    last_read: usize,
}

impl EventLog {
    /// Creates a new event log.
    pub fn new() -> Self {
        Self {
            logs: Vec::new(),
            last_read: 0,
        }
    }

    /// Does the log contain new messages since the last call to [`Self::read_out`].
    pub fn has_new_messages(&self) -> bool {
        self.last_read < self.logs.len()
    }

    /// Pushes a new event to the log.
    pub fn push(&mut self, message: BattleEvent) {
        self.logs.push(message.0)
    }

    /// Pushes multiple events to the log.
    pub fn push_extend<I>(&mut self, iterable: I)
    where
        I: IntoIterator<Item = BattleEvent>,
    {
        self.logs.extend(iterable.into_iter().map(|event| event.0));
    }

    /// Returns an iterator over all logs.
    pub fn logs(&self) -> impl Iterator<Item = &str> {
        self.logs.iter().map(|s| s.as_ref())
    }

    /// Reads out any new logs that have been added since the last call to [`Self::read_out`].
    pub fn read_out(&mut self) -> impl Iterator<Item = &str> {
        let i = mem::replace(&mut self.last_read, self.logs.len());
        self.logs[i..].iter().map(|s| s.as_ref())
    }
}

#[cfg(test)]
mod event_log_tests {
    use std::{
        borrow::Cow,
        fmt,
        fmt::Display,
    };

    use crate::log::{
        BattleLoggable,
        EventLog,
    };

    fn last_log(log: &mut EventLog) -> &str {
        log.logs.last().unwrap()
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

        log.push(battle_event!("a", "b", "c"));
        assert_eq!(last_log(&mut log), "a|b|c");

        log.push(battle_event!("time", 100000i32));
        assert_eq!(last_log(&mut log), "time|100000");

        log.push(battle_event!(
            "customdata",
            3.1415926535f64,
            CustomData {
                a: 234,
                b: "bulbasaur".to_owned(),
            },
            0i32,
            1i32,
            0i32,
        ));
        assert_eq!(
            last_log(&mut log),
            "customdata|3.1415926535|234 => bulbasaur|0|1|0"
        );
    }

    struct CustomDataWithLogImplementation {
        a: u32,
        b: String,
    }

    impl BattleLoggable for CustomDataWithLogImplementation {
        fn log<'s>(&'s self, items: &mut Vec<Cow<'s, str>>) {
            items.push(format!("{}", self.a).into());
            items.push("other".into());
            items.push(self.b.as_str().into());
        }
    }

    #[test]
    fn allows_custom_implementation() {
        let mut log = EventLog::new();

        log.push(battle_event!(
            "customdata",
            "abc",
            CustomDataWithLogImplementation {
                a: 234,
                b: "bulbasaur".to_owned(),
            },
            0i32,
        ));
        assert_eq!(last_log(&mut log), "customdata|abc|234|other|bulbasaur|0");
    }
}
