use std::{
    collections::VecDeque,
    mem,
};

use crate::{
    battle::{
        speed_sort,
        Action,
        Context,
        CoreBattle,
    },
    common::Error,
    rng::PseudoRandomNumberGenerator,
};

/// A queue of [`Action`]s to be run in a [`Battle`][`crate::battle::Battle`].
///
/// Actions are ordered in complex ways, so this queue type encapsulates all ordering logic.
#[derive(Clone)]
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
                    .battle_mut()
                    .queue
                    .push(Action::MegaEvo(action.mon_action.clone()))
            }
        }

        CoreBattle::resolve_action(context, &mut action)?;

        context.battle_mut().queue.push(action);
        Ok(())
    }

    pub fn push(&mut self, action: Action) {
        self.actions.push_back(action);
    }

    /// Pops the front [`Action`] to be executed.
    ///
    /// [`Self::sort`] should be called first.
    pub fn pop_front(&mut self) -> Option<Action> {
        self.actions.pop_front()
    }

    /// Peeks at the front [`Action`] on the queue, which is the next action to be executed.
    pub fn peek(&self) -> Option<&Action> {
        self.actions.front()
    }

    /// Checks if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Sorts all [`Action`]s in the queue.
    pub fn sort(context: &mut Context) {
        let prng = context.battle_mut().prng.as_mut();
        // SAFETY: PRNG and battle queue are completely disjoint.
        let prng = unsafe { mem::transmute(prng) };
        context.battle_mut().queue.sort_internal(prng)
    }

    fn sort_internal(&mut self, prng: &mut dyn PseudoRandomNumberGenerator) {
        let actions = self.actions.make_contiguous();
        speed_sort(actions, prng);
    }
}

#[cfg(test)]
mod queue_tests {
    use crate::{
        battle::{
            Action,
            BattleQueue,
            MonAction,
            MonHandle,
            MoveAction,
            SwitchAction,
            TeamAction,
        },
        common::Id,
        rng::RealPseudoRandomNumberGenerator,
    };

    fn team_action(mon: MonHandle, priority: i32) -> Action {
        Action::Team(TeamAction {
            mon_action: MonAction { mon, speed: 0 },
            index: 0,
            priority,
        })
    }

    fn switch_action(switching_out: MonHandle, instant: bool, speed: u32) -> Action {
        Action::Switch(SwitchAction {
            instant,
            mon_action: MonAction {
                mon: switching_out,
                speed,
            },
            switching_out,
            position: 0,
        })
    }

    fn move_action(id: Id, priority: i32, speed: u32, sub_priority: u32) -> Action {
        Action::Move(MoveAction {
            id,
            mon_action: MonAction {
                mon: MonHandle::from(0),
                speed,
            },
            target: None,
            original_target: None,
            mega: false,
            priority,
            sub_priority,
        })
    }

    fn mega_evo_action(mon: MonHandle, speed: u32) -> Action {
        Action::MegaEvo(MonAction { mon, speed })
    }

    fn sort(queue: &mut BattleQueue, seed: Option<u64>) {
        let mut prng = RealPseudoRandomNumberGenerator::new(seed);
        let items = queue.actions.make_contiguous();
        queue.sort_internal(&mut prng);
    }

    fn battle_queue_actions_to_string_for_test(queue: &BattleQueue) -> Vec<String> {
        queue
            .actions
            .iter()
            .map(|action| match action {
                Action::Start => "start".to_owned(),
                Action::Pass => "pass".to_owned(),
                Action::BeforeTurn => "beforeturn".to_owned(),
                Action::Residual => "residual".to_owned(),
                Action::Team(action) => format!("team {}", action.mon_action.mon),
                Action::Switch(action) => format!("switch {}", action.mon_action.mon),
                Action::Move(action) => format!("move {}", action.id),
                Action::MegaEvo(action) => format!("megaevo {}", action.mon),
            })
            .collect()
    }

    #[test]
    fn sorts_actions_with_no_ties() {
        let mut queue = BattleQueue::new();
        queue.push(Action::Start);
        queue.push(Action::BeforeTurn);
        queue.push(Action::Residual);
        queue.push(Action::Pass);
        queue.push(move_action(Id::from("m1"), 0, 100, 0));
        queue.push(move_action(Id::from("m2"), 5, 100, 0));
        queue.push(move_action(Id::from("m3"), 0, 200, 0));
        queue.push(move_action(Id::from("m4"), -1, 400, 0));
        queue.push(switch_action(MonHandle::from(1), false, 10));
        queue.push(switch_action(MonHandle::from(7), true, 10));
        queue.push(switch_action(MonHandle::from(2), false, 20));
        queue.push(team_action(MonHandle::from(3), -5));
        queue.push(team_action(MonHandle::from(4), -1));
        queue.push(mega_evo_action(MonHandle::from(5), 10));
        queue.push(mega_evo_action(MonHandle::from(6), 20));

        sort(&mut queue, None);
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "team 4",
                "team 3",
                "start",
                "switch 7",
                "beforeturn",
                "switch 2",
                "switch 1",
                "megaevo 6",
                "megaevo 5",
                "move m2",
                "move m3",
                "move m1",
                "pass",
                "move m4",
                "residual",
            ]
        );
    }

    #[test]
    fn sorts_actions_with_random_ties() {
        let mut queue = BattleQueue::new();
        queue.push(move_action(Id::from("m1"), 0, 100, 0));
        queue.push(move_action(Id::from("m2"), 0, 100, 0));
        queue.push(move_action(Id::from("m3"), 0, 100, 0));
        queue.push(move_action(Id::from("m4"), 1, 100, 0));
        queue.push(move_action(Id::from("m5"), 1, 100, 0));
        queue.push(move_action(Id::from("m6"), -1, 100, 0));
        queue.push(move_action(Id::from("m7"), -1, 100, 0));

        queue.push(switch_action(MonHandle::from(1), false, 10));
        queue.push(switch_action(MonHandle::from(2), false, 10));

        queue.push(mega_evo_action(MonHandle::from(3), 10));
        queue.push(mega_evo_action(MonHandle::from(4), 10));

        sort(&mut queue, Some(0));
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 1",
                "switch 2",
                "megaevo 3",
                "megaevo 4",
                "move m4",
                "move m5",
                "move m1",
                "move m2",
                "move m3",
                "move m7",
                "move m6",
            ]
        );

        sort(&mut queue, Some(123456));
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 2",
                "switch 1",
                "megaevo 3",
                "megaevo 4",
                "move m5",
                "move m4",
                "move m2",
                "move m1",
                "move m3",
                "move m7",
                "move m6",
            ]
        );

        sort(&mut queue, Some(987654321));
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 2",
                "switch 1",
                "megaevo 4",
                "megaevo 3",
                "move m5",
                "move m4",
                "move m2",
                "move m1",
                "move m3",
                "move m6",
                "move m7",
            ]
        );

        sort(&mut queue, Some(1902372845324));
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 2",
                "switch 1",
                "megaevo 3",
                "megaevo 4",
                "move m4",
                "move m5",
                "move m1",
                "move m2",
                "move m3",
                "move m6",
                "move m7",
            ]
        );

        sort(&mut queue, Some(897234982374932874));
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 1",
                "switch 2",
                "megaevo 4",
                "megaevo 3",
                "move m4",
                "move m5",
                "move m3",
                "move m2",
                "move m1",
                "move m7",
                "move m6",
            ]
        );
    }
}
