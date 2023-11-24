use std::collections::VecDeque;

use crate::{
    battle::{
        Action,
        Context,
    },
    common::Error,
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
    pub fn add_action(context: &mut Context, action: Action) -> Result<(), Error> {
        Self::add_sub_actions(context, action)
    }

    /// Adds multiple [`Action`]s to the queue.
    pub fn add_actions<I>(context: &mut Context, actions: I) -> Result<(), Error>
    where
        I: Iterator<Item = Action>,
    {
        for action in actions {
            Self::add_sub_actions(context, action)?;
        }
        Ok(())
    }

    fn add_sub_actions(context: &mut Context, mut action: Action) -> Result<(), Error> {
        if let Action::Pass = action {
            return Ok(());
        }

        if let Action::Move(action) = &action {
            if action.mega {
                context
                    .battle_queue_mut()
                    .actions
                    .push_back(Action::MegaEvo(action.mon_action.clone()))
            }
        }

        context.battle_mut().resolve_action(&mut action)?;

        context.battle_queue_mut().actions.push_back(action);
        Ok(())
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
