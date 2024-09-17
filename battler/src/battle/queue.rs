use std::{
    collections::VecDeque,
    mem,
};

use crate::{
    battle::{
        compare_priority,
        speed_sort,
        Action,
        BeforeMoveAction,
        BeforeMoveActionInput,
        Context,
        CoreBattle,
        CoreBattleEngineSpeedSortTieResolution,
        MonHandle,
    },
    common::Error,
    rng::{
        rand_util,
        PseudoRandomNumberGenerator,
    },
};

/// A queue of [`Action`]s to be run in a [`CoreBattle`][`crate::battle::CoreBattle`].
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
        let actions = Self::resolve_action(context, action)?;
        context.battle_mut().queue.actions.extend(actions);
        Ok(())
    }

    /// Adds multiple [`Action`]s to the queue.
    pub fn add_actions<I>(context: &mut Context, actions: I) -> Result<(), Error>
    where
        I: Iterator<Item = Action>,
    {
        for action in actions {
            Self::add_action(context, action)?;
        }
        Ok(())
    }

    fn sub_actions(action: &Action) -> Vec<Action> {
        match action {
            Action::Move(action) => {
                let mut actions = Vec::from_iter([
                    Action::BeforeTurnMove(BeforeMoveAction::new(BeforeMoveActionInput {
                        id: action.id.clone(),
                        mon: action.mon_action.mon,
                    })),
                    Action::PriorityChargeMove(BeforeMoveAction::new(BeforeMoveActionInput {
                        id: action.id.clone(),
                        mon: action.mon_action.mon,
                    })),
                ]);
                if action.mega {
                    actions.push(Action::MegaEvo(action.mon_action.clone()));
                }
                actions
            }
            _ => Vec::new(),
        }
    }

    fn resolve_action(context: &mut Context, action: Action) -> Result<Vec<Action>, Error> {
        match action {
            Action::Pass => Ok(Vec::new()),
            _ => {
                let mut actions = Self::sub_actions(&action);
                actions.push(action);
                for action in &mut actions {
                    CoreBattle::resolve_action(context, action)?;
                }
                Ok(actions)
            }
        }
    }

    /// Pushes a new [`Action`] to the queue.
    ///
    /// In general, [`Self::add_action`] should be preferred, since it will split actions into
    /// sub-actions as applicable.
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

    /// Returns a mutable reference to an [`Action`] already in the queue.
    pub fn find_action_mut<F>(&mut self, matcher: F) -> Option<&mut Action>
    where
        F: Fn(&Action) -> bool,
    {
        self.actions.iter_mut().find(|action| matcher(action))
    }

    /// Sorts all [`Action`]s in the queue.
    pub fn sort(context: &mut Context) {
        let prng = context.battle_mut().prng.as_mut();
        // SAFETY: PRNG and battle queue are completely disjoint.
        let prng = unsafe { mem::transmute(prng) };
        let tie_resolution = context.battle().engine_options.speed_sort_tie_resolution;
        context
            .battle_mut()
            .queue
            .sort_internal(prng, tie_resolution)
    }

    fn sort_internal(
        &mut self,
        prng: &mut dyn PseudoRandomNumberGenerator,
        tie_resolution: CoreBattleEngineSpeedSortTieResolution,
    ) {
        let actions = self.actions.make_contiguous();
        speed_sort(actions, prng, tie_resolution);
    }

    /// Checks if there is any move scheduled for this turn.
    pub fn any_move_this_turn(&self) -> bool {
        self.actions.iter().any(|action| match action {
            Action::Move(_) => true,
            _ => false,
        })
    }

    /// Checks if the given Mon will move this turn.
    pub fn will_move_this_turn(&self, mon: MonHandle) -> bool {
        self.actions.iter().any(|action| match action {
            Action::Move(move_action) => move_action.mon_action.mon == mon,
            _ => false,
        })
    }

    /// Cancels the move action to be made by the Mon.
    pub fn cancel_move(&mut self, mon: MonHandle) -> bool {
        let before = self.actions.len();

        let mut actions = VecDeque::new();
        mem::swap(&mut actions, &mut self.actions);
        actions = actions
            .into_iter()
            .filter(|action| {
                if let Action::Move(action) = action {
                    action.mon_action.mon != mon
                } else {
                    true
                }
            })
            .collect();
        mem::swap(&mut actions, &mut self.actions);

        let after = self.actions.len();
        before > after
    }

    /// Inserts an [`Action`] into the queue into the position it would have been had it been sorted
    /// originally.
    ///
    /// Assumes the queue is already sorted.
    pub fn insert_action_into_sorted_position(
        context: &mut Context,
        action: Action,
    ) -> Result<(), Error> {
        for action in Self::resolve_action(context, action)? {
            Self::insert_resolved_action_into_sorted_position(context, action)?;
        }
        Ok(())
    }

    fn insert_resolved_action_into_sorted_position(
        context: &mut Context,
        action: Action,
    ) -> Result<(), Error> {
        let prng = context.battle_mut().prng.as_mut();
        // SAFETY: PRNG and battle queue are completely disjoint.
        let prng = unsafe { mem::transmute(prng) };
        let tie_resolution = context.battle().engine_options.speed_sort_tie_resolution;
        context
            .battle_mut()
            .queue
            .insert_resolved_action_into_sorted_position_internal(action, prng, tie_resolution);
        Ok(())
    }

    fn insert_resolved_action_into_sorted_position_internal(
        &mut self,
        action: Action,
        prng: &mut dyn PseudoRandomNumberGenerator,
        tie_resolution: CoreBattleEngineSpeedSortTieResolution,
    ) {
        let mut min = None;
        let mut max = None;
        for (i, existing) in self.actions.iter().enumerate() {
            let order = compare_priority(&action, existing);
            if order.is_le() && min.is_none() {
                min = Some(i);
            }
            if order.is_lt() && max.is_none() {
                max = Some(i);
                break;
            }
        }
        match min {
            Some(min) => {
                let max = max.unwrap_or(self.actions.len());
                if min == max {
                    self.actions.insert(min, action);
                } else {
                    match tie_resolution {
                        CoreBattleEngineSpeedSortTieResolution::Keep => {
                            self.actions.insert(min, action)
                        }
                        CoreBattleEngineSpeedSortTieResolution::Reverse => {
                            self.actions.insert(max, action)
                        }
                        CoreBattleEngineSpeedSortTieResolution::Random => self.actions.insert(
                            rand_util::range(prng, min as u64, max as u64 + 1) as usize,
                            action,
                        ),
                    }
                }
            }
            None => {
                self.actions.push_back(action);
            }
        }
    }
}

#[cfg(test)]
mod queue_tests {
    use crate::{
        battle::{
            Action,
            BattleQueue,
            CoreBattleEngineSpeedSortTieResolution,
            ExperienceAction,
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

    fn experience_action(mon: MonHandle, exp: u32) -> Action {
        Action::Experience(ExperienceAction {
            mon,
            player_index: 0,
            mon_index: 0,
            active: true,
            exp,
        })
    }

    fn sort(queue: &mut BattleQueue, seed: Option<u64>) {
        let mut prng = RealPseudoRandomNumberGenerator::new(seed);
        queue.actions.make_contiguous();
        queue.sort_internal(&mut prng, CoreBattleEngineSpeedSortTieResolution::Random);
    }

    fn insert_resolved_action_into_sorted_position(
        queue: &mut BattleQueue,
        action: Action,
        seed: Option<u64>,
    ) {
        let mut prng = RealPseudoRandomNumberGenerator::new(seed);
        queue.insert_resolved_action_into_sorted_position_internal(
            action,
            &mut prng,
            CoreBattleEngineSpeedSortTieResolution::Random,
        );
    }

    fn battle_queue_actions_to_string_for_test(queue: &BattleQueue) -> Vec<String> {
        queue
            .actions
            .iter()
            .map(|action| match action {
                Action::Start => "start".to_owned(),
                Action::End(_) => "end".to_owned(),
                Action::Pass => "pass".to_owned(),
                Action::BeforeTurn => "beforeturn".to_owned(),
                Action::Residual => "residual".to_owned(),
                Action::Team(action) => format!("team {}", action.mon_action.mon),
                Action::Switch(action) => format!("switch {}", action.mon_action.mon),
                Action::SwitchEvents(action) => format!("switchevents {}", action.mon_action.mon),
                Action::Move(action) => format!("move {}", action.id),
                Action::BeforeTurnMove(action) => {
                    format!("beforeturnmove {}", action.mon_action.mon)
                }
                Action::PriorityChargeMove(action) => {
                    format!("prioritychargemove {}", action.mon_action.mon)
                }
                Action::MegaEvo(action) => format!("megaevo {}", action.mon),
                Action::Experience(action) => format!("experience {}", action.mon),
                Action::LevelUp(action) => format!("levelup {}", action.mon),
                Action::LearnMove(action) => format!("learnmove {}", action.mon),
                Action::Escape(action) => format!("escape {}", action.mon_action.mon),
                Action::Forfeit(action) => format!("forfeit {}", action.player),
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
                "move m5",
                "move m4",
                "move m1",
                "move m2",
                "move m3",
                "move m6",
                "move m7",
            ]
        );

        sort(&mut queue, Some(1234567));
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 2",
                "switch 1",
                "megaevo 4",
                "megaevo 3",
                "move m4",
                "move m5",
                "move m3",
                "move m1",
                "move m2",
                "move m6",
                "move m7",
            ]
        );

        sort(&mut queue, Some(987654321));
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 2",
                "switch 1",
                "megaevo 3",
                "megaevo 4",
                "move m4",
                "move m5",
                "move m3",
                "move m1",
                "move m2",
                "move m7",
                "move m6",
            ]
        );

        sort(&mut queue, Some(1902372845324));
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 2",
                "switch 1",
                "megaevo 4",
                "megaevo 3",
                "move m5",
                "move m4",
                "move m1",
                "move m3",
                "move m2",
                "move m7",
                "move m6",
            ]
        );

        sort(&mut queue, Some(897234982374932874));
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 1",
                "switch 2",
                "megaevo 3",
                "megaevo 4",
                "move m5",
                "move m4",
                "move m2",
                "move m3",
                "move m1",
                "move m6",
                "move m7",
            ]
        );
    }

    #[test]
    fn inserts_action_into_sorted_position_with_random_ties() {
        let mut source_queue = BattleQueue::new();
        source_queue.push(move_action(Id::from("m1"), 0, 100, 0));
        source_queue.push(move_action(Id::from("m2"), 0, 100, 0));
        source_queue.push(move_action(Id::from("m3"), 0, 100, 0));
        source_queue.push(move_action(Id::from("m4"), 1, 100, 0));
        source_queue.push(move_action(Id::from("m5"), 1, 100, 0));
        source_queue.push(move_action(Id::from("m6"), -1, 100, 0));
        source_queue.push(move_action(Id::from("m7"), -1, 100, 0));

        source_queue.push(switch_action(MonHandle::from(1), false, 10));
        source_queue.push(switch_action(MonHandle::from(2), false, 10));

        source_queue.push(mega_evo_action(MonHandle::from(3), 10));
        source_queue.push(mega_evo_action(MonHandle::from(4), 10));

        sort(&mut source_queue, Some(0));
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&source_queue),
            vec![
                "switch 1",
                "switch 2",
                "megaevo 3",
                "megaevo 4",
                "move m5",
                "move m4",
                "move m1",
                "move m2",
                "move m3",
                "move m6",
                "move m7",
            ]
        );

        let mut queue = source_queue.clone();
        insert_resolved_action_into_sorted_position(
            &mut queue,
            move_action(Id::from("m8"), 0, 200, 0),
            None,
        );
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 1",
                "switch 2",
                "megaevo 3",
                "megaevo 4",
                "move m5",
                "move m4",
                "move m8",
                "move m1",
                "move m2",
                "move m3",
                "move m6",
                "move m7",
            ]
        );

        queue = source_queue.clone();
        insert_resolved_action_into_sorted_position(
            &mut queue,
            move_action(Id::from("m8"), 0, 100, 0),
            Some(0),
        );
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 1",
                "switch 2",
                "megaevo 3",
                "megaevo 4",
                "move m5",
                "move m4",
                "move m8",
                "move m1",
                "move m2",
                "move m3",
                "move m6",
                "move m7",
            ]
        );

        queue = source_queue.clone();
        insert_resolved_action_into_sorted_position(
            &mut queue,
            move_action(Id::from("m8"), 0, 100, 0),
            Some(1),
        );
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 1",
                "switch 2",
                "megaevo 3",
                "megaevo 4",
                "move m5",
                "move m4",
                "move m1",
                "move m8",
                "move m2",
                "move m3",
                "move m6",
                "move m7",
            ]
        );

        queue = source_queue.clone();
        insert_resolved_action_into_sorted_position(
            &mut queue,
            move_action(Id::from("m8"), 0, 100, 0),
            Some(2),
        );
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 1",
                "switch 2",
                "megaevo 3",
                "megaevo 4",
                "move m5",
                "move m4",
                "move m1",
                "move m2",
                "move m8",
                "move m3",
                "move m6",
                "move m7",
            ]
        );

        queue = source_queue.clone();
        insert_resolved_action_into_sorted_position(
            &mut queue,
            move_action(Id::from("m8"), 0, 100, 0),
            Some(5),
        );
        pretty_assertions::assert_eq!(
            battle_queue_actions_to_string_for_test(&queue),
            vec![
                "switch 1",
                "switch 2",
                "megaevo 3",
                "megaevo 4",
                "move m5",
                "move m4",
                "move m1",
                "move m2",
                "move m3",
                "move m8",
                "move m6",
                "move m7",
            ]
        );
    }

    #[test]
    fn finds_existing_action() {
        let mut queue = BattleQueue::new();
        queue.push(move_action(Id::from("m1"), 0, 100, 0));
        queue.push(move_action(Id::from("m2"), 0, 100, 0));
        queue.push(experience_action(MonHandle::from(0), 100));
        queue.push(experience_action(MonHandle::from(1), 100));

        sort(&mut queue, Some(0));

        match queue.find_action_mut(|action| match action {
            Action::Experience(action) => action.mon == MonHandle::from(0),
            _ => false,
        }) {
            Some(Action::Experience(action)) => action.exp += 200,
            _ => assert!(
                false,
                "find_action_mut did not produce the correct experience action"
            ),
        }

        assert_matches::assert_matches!(queue.find_action_mut(|action| match action {
            Action::Experience(action) => action.mon == MonHandle::from(0),
            _ => false,
        }), Some(Action::Experience(action)) => assert_eq!(action.exp, 300));
    }
}
