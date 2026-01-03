/// A clock used to read the current time to attach to the battle log.
pub trait Clock: Send + Sync {
    /// The current timestamp.
    fn now(&self) -> u128;
}

#[cfg(feature = "std")]
pub mod system_time_clock {
    use std::time::{
        SystemTime,
        UNIX_EPOCH,
    };

    use crate::common::Clock;

    pub struct SystemTimeClock;

    impl Clock for SystemTimeClock {
        fn now(&self) -> u128 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        }
    }
}
