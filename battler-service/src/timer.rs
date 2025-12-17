use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    time::Duration,
};

use serde::{
    Deserialize,
    Serialize,
};

/// Timer type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TimerType {
    /// Timer for the entire battle.
    Battle,
    /// Timer per player.
    Player(String),
    /// Timer per player per action.
    Action(String),
}

impl TimerType {
    /// Should the timer be reset when resumed?
    pub(crate) fn reset_on_resume(&self) -> bool {
        match self {
            Self::Action(_) => true,
            _ => false,
        }
    }

    /// The player the timer corresponds to.
    pub(crate) fn player(&self) -> Option<&str> {
        match self {
            Self::Player(player) | Self::Action(player) => Some(&player),
            _ => None,
        }
    }
}

/// State for a single timer.
#[derive(Debug, Clone)]
pub(crate) struct TimerState {
    /// The total duration of the timer.
    pub total: Duration,
    /// The remaining duration of the timer.
    pub remaining: Duration,
    /// Durations at which, when the timer has the specified amount of time remaining, warning logs
    /// are issued.
    pub warnings: BTreeSet<Duration>,
}

impl From<Timer> for TimerState {
    fn from(value: Timer) -> Self {
        Self {
            total: Duration::from_secs(value.secs),
            remaining: Duration::from_secs(value.secs),
            warnings: value
                .warnings
                .into_iter()
                .map(|val| Duration::from_secs(val))
                .collect(),
        }
    }
}

/// Configuration for a single timer.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Timer {
    /// Number of seconds.
    pub secs: u64,
    /// Second durations at which, when the timer has the specified amount of time remaining,
    /// warning logs are issued.
    pub warnings: BTreeSet<u64>,
}

/// Configuration for battle timers.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Timers {
    /// Timer for the entire battle.
    ///
    /// When this timer runs out, the battle automatically ends and a winner is decided based on
    /// Mons remaining.
    pub battle: Option<Timer>,
    /// Timer for each player.
    ///
    /// When this timer runs out, the player is forced to forfeit.
    pub player: Option<Timer>,
    /// Timer for each player per action.
    ///
    /// When this timer runs out, the player is forced to make a random action.
    pub action: Option<Timer>,
}

impl Timers {
    /// Converts the timer configuration to a map of states.
    ///
    /// [`BTreeMap`] is used for consistent ordering, which ensures timers are started in a stable
    /// order in the battle log.
    pub(crate) fn to_state<S>(self, players: &[S]) -> BTreeMap<TimerType, TimerState>
    where
        S: ToString,
    {
        let mut state = BTreeMap::default();
        if let Some(timer) = self.battle {
            state.insert(TimerType::Battle, timer.into());
        }
        if let Some(timer) = self.player {
            state.extend(
                players
                    .iter()
                    .map(|player| (TimerType::Player(player.to_string()), timer.clone().into())),
            );
        }
        if let Some(timer) = self.action {
            state.extend(
                players
                    .iter()
                    .map(|player| (TimerType::Action(player.to_string()), timer.clone().into())),
            );
        }
        state
    }
}
