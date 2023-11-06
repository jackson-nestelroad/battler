use std::cmp::Ordering;

use crate::{
    battle::MonHandle,
    common::Id,
};

/// A Team Preview action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamAction {
    pub mon: MonHandle,
    pub index: usize,
    pub priority: i32,
}

/// A switch action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchAction {
    pub instant: bool,
    pub mon: MonHandle,
    pub position: usize,
}

/// A move action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveAction {
    pub id: Id,
    pub mon: MonHandle,
    pub target: Option<isize>,
    pub mega: bool,
}

/// An action during a battle.
///
/// Actions are the core of a battle. A turn of a battle consists of several actions running
/// sequentially. Actions can also be run outside of a turn for configuration and miscellaneous
/// purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Start,
    Pass,
    BeforeTurn,
    Residual,
    Team(TeamAction),
    Switch(SwitchAction),
    Move(MoveAction),
}

impl Action {
    pub fn order(&self) -> u32 {
        match self {
            Self::Team(_) => 1,
            Self::Start => 2,
            Self::Switch(action) => {
                if action.instant {
                    3
                } else {
                    200
                }
            }
            Self::BeforeTurn => 4,
            Self::Move(_) => 200,
            Self::Pass => 200,
            Self::Residual => 300,
        }
    }

    pub fn priority(&self) -> i32 {
        match self {
            Self::Team(action) => action.priority,
            _ => 0,
        }
    }

    pub fn speed(&self) -> u32 {
        match self {
            _ => 0,
        }
    }

    pub fn sub_order(&self) -> u32 {
        match self {
            _ => 0,
        }
    }
}

impl PartialOrd for Action {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Action {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower order first.
        self.order().cmp(&other.order()).then_with(|| {
            // Higher priority first.
            other.priority().cmp(&self.priority()).then_with(|| {
                // Higher speed first.
                other
                    .speed()
                    .cmp(&self.speed())
                    .then_with(|| self.sub_order().cmp(&other.sub_order()))
            })
        })
    }
}
