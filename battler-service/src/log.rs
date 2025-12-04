use tokio::sync::{
    broadcast,
    mpsc,
};
use uuid::Uuid;

/// A single entry of a [`Log`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    /// Index of the entry.
    pub index: usize,
    /// Content of the entry.
    pub content: String,
}

/// A global log entry, which corresponds to some side of some battle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalLogEntry {
    pub battle: Uuid,
    pub side: Option<usize>,
    pub entry: LogEntry,
}

/// A log of events for a battle.
pub struct Log {
    battle: Uuid,
    side: Option<usize>,
    entries: Vec<String>,
    entry_tx: broadcast::Sender<LogEntry>,
    global_log_tx: mpsc::UnboundedSender<GlobalLogEntry>,
}

impl Log {
    fn new(
        battle: Uuid,
        side: Option<usize>,
        global_log_tx: mpsc::UnboundedSender<GlobalLogEntry>,
    ) -> Self {
        let (entry_tx, _) = broadcast::channel(128);
        Self {
            battle,
            side,
            entries: Vec::new(),
            entry_tx,
            global_log_tx,
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
            let entry = LogEntry {
                index: i + index,
                content: entry.clone(),
            };
            self.entry_tx.send(entry.clone()).ok();

            self.global_log_tx
                .send(GlobalLogEntry {
                    battle: self.battle,
                    side: self.side,
                    entry,
                })
                .ok();
        }
    }

    /// Subscribes to new log entries.
    pub fn subscribe(&self) -> broadcast::Receiver<LogEntry> {
        self.entry_tx.subscribe()
    }

    /// All entries in the log.
    pub fn entries(&self) -> impl Iterator<Item = &str> + ExactSizeIterator + DoubleEndedIterator {
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
    pub fn new(
        battle: Uuid,
        sides: usize,
        global_log_tx: mpsc::UnboundedSender<GlobalLogEntry>,
    ) -> Self {
        Self {
            public_log: Log::new(battle, None, global_log_tx.clone()),
            per_side_logs: Vec::from_iter(
                (0..sides).map(|side| Log::new(battle, Some(side), global_log_tx.clone())),
            ),
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
    use std::usize;

    use tokio::sync::mpsc;
    use uuid::Uuid;

    use crate::{
        GlobalLogEntry,
        log::{
            LogEntry,
            SplitLogs,
        },
    };

    #[test]
    fn filters_split_logs() {
        let (global_log_tx, _) = mpsc::unbounded_channel();
        let mut logs = SplitLogs::new(Uuid::from_u64_pair(0, 128), 2, global_log_tx);
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
        let (global_log_tx, mut global_log_rx) = mpsc::unbounded_channel();
        let mut logs = SplitLogs::new(Uuid::from_u64_pair(0, 128), 2, global_log_tx);
        let mut public_log_rx = logs.public_log().subscribe();
        let mut side_1_log_rx = logs.side_log(0).unwrap().subscribe();
        let mut side_2_log_rx = logs.side_log(1).unwrap().subscribe();

        logs.append(["split|side:0", "ghi|hp:255/255", "ghi|hp:100/100", "public"]);

        assert_matches::assert_matches!(public_log_rx.recv().await, Ok(entry) => {
            assert_eq!(entry, LogEntry { index: 0, content: "ghi|hp:100/100".to_owned() });
        });
        assert_matches::assert_matches!(side_1_log_rx.recv().await, Ok(entry) => {
            assert_eq!(entry, LogEntry { index: 0, content: "ghi|hp:255/255".to_owned() });
        });
        assert_matches::assert_matches!(side_2_log_rx.recv().await, Ok(entry) => {
            assert_eq!(entry, LogEntry { index: 0, content: "ghi|hp:100/100".to_owned() });
        });

        let mut global_log = Vec::default();
        global_log_rx.recv_many(&mut global_log, usize::MAX).await;
        pretty_assertions::assert_eq!(
            global_log,
            Vec::from_iter([
                GlobalLogEntry {
                    battle: Uuid::from_u64_pair(0, 128),
                    side: None,
                    entry: LogEntry {
                        index: 0,
                        content: "ghi|hp:100/100".to_owned(),
                    },
                },
                GlobalLogEntry {
                    battle: Uuid::from_u64_pair(0, 128),
                    side: None,
                    entry: LogEntry {
                        index: 1,
                        content: "public".to_owned(),
                    },
                },
                GlobalLogEntry {
                    battle: Uuid::from_u64_pair(0, 128),
                    side: Some(0),
                    entry: LogEntry {
                        index: 0,
                        content: "ghi|hp:255/255".to_owned(),
                    },
                },
                GlobalLogEntry {
                    battle: Uuid::from_u64_pair(0, 128),
                    side: Some(0),
                    entry: LogEntry {
                        index: 1,
                        content: "public".to_owned(),
                    },
                },
                GlobalLogEntry {
                    battle: Uuid::from_u64_pair(0, 128),
                    side: Some(1),
                    entry: LogEntry {
                        index: 0,
                        content: "ghi|hp:100/100".to_owned(),
                    },
                },
                GlobalLogEntry {
                    battle: Uuid::from_u64_pair(0, 128),
                    side: Some(1),
                    entry: LogEntry {
                        index: 1,
                        content: "public".to_owned(),
                    },
                },
            ])
        );
    }
}
