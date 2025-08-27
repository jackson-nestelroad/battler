use std::sync::{
    Mutex,
    MutexGuard,
};

use ahash::HashSet;
use battler_data::Id;

/// State for the evaluation of an event.
#[derive(Debug, Default)]
pub struct EventState {
    effect_ids_to_skip: Mutex<HashSet<Id>>,
}

impl EventState {
    fn effect_ids_to_skip(&self) -> MutexGuard<'_, HashSet<Id>> {
        self.effect_ids_to_skip.lock().unwrap_or_else(|mut e| {
            e.get_mut().clear();
            self.effect_ids_to_skip.clear_poison();
            e.into_inner()
        })
    }

    /// Marks the effect's callback to be skipped for the event.
    pub fn skip_effect(&self, effect: Id) {
        let mut skip = self.effect_ids_to_skip();
        skip.insert(effect);
    }

    /// Checks if the effect's callback for the event should run.
    pub fn effect_should_run(&self, effect: &str) -> bool {
        !self.effect_ids_to_skip().contains(effect)
    }
}
