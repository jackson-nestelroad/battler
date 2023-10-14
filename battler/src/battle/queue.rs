use std::collections::VecDeque;

use crate::battle::{
    Action,
    Context,
};

/// A queue of [`Action`]s to be run in a [`Battle`][`crate::battle::Battle`].
///
/// Actions are ordered in complex ways, so this queue type encapsulates all ordering logic.
pub struct BattleQueue {
    actions: VecDeque<Action>,
}

impl BattleQueue {
    /// Creates a new [`BattleQueue`].
    pub fn new() -> Self {
        Self {
            actions: VecDeque::new(),
        }
    }

    /// Adds a new [`Action`] to the queue.
    pub fn add_action(context: &mut Context, action: Action) {
        Self::add_sub_actions(context, action)
    }

    /// Adds multiple [`Action`]s to the queue.
    pub fn add_actions<I>(context: &mut Context, actions: I)
    where
        I: Iterator<Item = Action>,
    {
        for action in actions {
            Self::add_sub_actions(context, action);
        }
    }

    fn add_sub_actions(context: &mut Context, action: Action) {
        if let Action::Pass = action {
            return;
        }

        let queue = context.battle_queue_mut();
        queue.actions.push_back(action);
    }

    /// Sorts all [`Action`]s in the queue.
    pub fn sort(&mut self) {
        self.actions.make_contiguous().sort()
    }

    /// Pops the front [`Action`] to be executed.
    ///
    /// [`Self::sort`] should be called first.
    pub fn pop_front(&mut self) -> Option<Action> {
        self.actions.pop_front()
    }
}
