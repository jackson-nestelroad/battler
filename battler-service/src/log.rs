use tokio::sync::broadcast;

/// A single entry of a [`Log`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    /// Index of the entry.
    pub index: usize,
    /// Content of the entry.
    pub content: String,
}

/// A log of events for a battle.
pub struct Log {
    entries: Vec<String>,
    entry_tx: broadcast::Sender<LogEntry>,
}

impl Log {
    fn new() -> Self {
        let (entry_tx, _) = broadcast::channel(16);
        Self {
            entries: Vec::new(),
            entry_tx,
        }
    }

    fn append<I, S>(&mut self, entries: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let new_entries = entries.into_iter().map(|s| s.into()).collect::<Vec<_>>();
        let last_index = self.entries.len();
        self.entries.extend(new_entries.clone());
        self.publish_from(last_index);
    }

    fn publish_from(&self, index: usize) {
        for (i, entry) in self.entries[index..].iter().enumerate() {
            // If send fails, there is no receiver, which is OK.
            self.entry_tx
                .send(LogEntry {
                    index: i + index,
                    content: entry.clone(),
                })
                .ok();
        }
    }

    /// Subscribes to new log entries.
    pub fn subscribe(&self) -> broadcast::Receiver<LogEntry> {
        self.entry_tx.subscribe()
    }

    /// All entries in the log.
    pub fn entries(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|s| s.as_str())
    }
}

/// A set of battle logs filtered according to side splits.
pub struct SplitLogs {
    public_log: Log,
    per_side_logs: Vec<Log>,
}

impl SplitLogs {
    /// Creates a new set of split logs with a given number of sides.
    pub fn new(sides: usize) -> Self {
        Self {
            public_log: Log::new(),
            per_side_logs: Vec::from_iter(std::iter::repeat_with(|| Log::new()).take(sides)),
        }
    }

    fn split_next_logs_for_side(entry: &str) -> Option<usize> {
        let mut values = entry.split('|');
        match values.next() {
            Some("split") => (),
            _ => return None,
        }
        let side = values
            .filter_map(|value| value.split_once(':'))
            .find_map(|(key, value)| (key == "side").then_some(value))?;
        side.parse::<usize>().ok()
    }

    /// Appends new entries to the log, splitting them accordingly.
    pub fn append<I, S>(&mut self, entries: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut public_entries = Vec::new();
        let mut per_side_entries = self
            .per_side_logs
            .iter()
            .map(|_| Vec::new())
            .collect::<Vec<_>>();

        let mut entries = entries.into_iter().map(|s| Into::<String>::into(s));
        while let Some(entry) = entries.next() {
            match Self::split_next_logs_for_side(&entry) {
                Some(side) => {
                    let (private, public) = (entries.next(), entries.next());
                    if let Some(public) = public.clone() {
                        public_entries.push(public);
                    }
                    for (i, entries) in per_side_entries.iter_mut().enumerate() {
                        if i == side {
                            if let Some(private) = private.clone() {
                                entries.push(private);
                            }
                        } else if let Some(public) = public.clone() {
                            entries.push(public);
                        }
                    }
                }
                None => {
                    public_entries.push(entry.clone());
                    per_side_entries
                        .iter_mut()
                        .for_each(|log| log.push(entry.clone()));
                }
            }
        }

        self.public_log.append(public_entries);
        for (i, entries) in per_side_entries.iter().enumerate() {
            if let Some(log) = self.per_side_logs.get_mut(i) {
                log.append(entries);
            }
        }
    }

    /// The public log.
    pub fn public_log(&self) -> &Log {
        &self.public_log
    }

    /// The log for an individual side.
    pub fn side_log(&self, side: usize) -> Option<&Log> {
        self.per_side_logs.get(side)
    }
}

#[cfg(test)]
mod log_test {
    use crate::log::{
        LogEntry,
        SplitLogs,
    };

    #[test]
    fn filters_split_logs() {
        let mut logs = SplitLogs::new(2);
        logs.append([
            "time|time:123",
            "abc|def",
            "split|side:0",
            "ghi|hp:255/255",
            "ghi|hp:100/100",
            "jkl|mno",
            "split|side:1",
            "pqr|move:stu|ability:vwx",
            "pqr|move:stu",
        ]);
        pretty_assertions::assert_eq!(
            logs.public_log().entries().collect::<Vec<_>>(),
            Vec::from_iter([
                "time|time:123",
                "abc|def",
                "ghi|hp:100/100",
                "jkl|mno",
                "pqr|move:stu",
            ])
        );
        pretty_assertions::assert_eq!(
            logs.side_log(0).unwrap().entries().collect::<Vec<_>>(),
            Vec::from_iter([
                "time|time:123",
                "abc|def",
                "ghi|hp:255/255",
                "jkl|mno",
                "pqr|move:stu",
            ])
        );
        pretty_assertions::assert_eq!(
            logs.side_log(1).unwrap().entries().collect::<Vec<_>>(),
            Vec::from_iter([
                "time|time:123",
                "abc|def",
                "ghi|hp:100/100",
                "jkl|mno",
                "pqr|move:stu|ability:vwx",
            ])
        );
        assert!(logs.side_log(2).is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn publishes_filtered_logs() {
        let mut logs = SplitLogs::new(2);
        let mut public_log_rx = logs.public_log().subscribe();
        let mut side_1_log_rx = logs.side_log(0).unwrap().subscribe();
        let mut side_2_log_rx = logs.side_log(1).unwrap().subscribe();

        logs.append(["split|side:0", "ghi|hp:255/255", "ghi|hp:100/100"]);

        assert_matches::assert_matches!(public_log_rx.recv().await, Ok(entry) => {
            assert_eq!(entry, LogEntry { index: 0, content: "ghi|hp:100/100".to_owned() });
        });
        assert_matches::assert_matches!(side_1_log_rx.recv().await, Ok(entry) => {
            assert_eq!(entry, LogEntry { index: 0, content: "ghi|hp:255/255".to_owned() });
        });
        assert_matches::assert_matches!(side_2_log_rx.recv().await, Ok(entry) => {
            assert_eq!(entry, LogEntry { index: 0, content: "ghi|hp:100/100".to_owned() });
        });
    }
}
