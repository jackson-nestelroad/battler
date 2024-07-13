use crate::{
    battle::{
        MonHandle,
        SpeedOrderable,
    },
    common::Id,
};

/// A Mon action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonAction {
    pub mon: MonHandle,
    pub speed: u32,
}

impl MonAction {
    pub fn new(mon: MonHandle) -> Self {
        Self { mon, speed: 0 }
    }
}

/// A Team Preview action input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamActionInput {
    pub mon: MonHandle,
    pub index: usize,
    pub priority: i32,
}

/// A Team Preview action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamAction {
    pub mon_action: MonAction,
    pub index: usize,
    pub priority: i32,
}

impl TeamAction {
    /// Creates a new [`TeamAction`] from [`TeamActionInput`].
    pub fn new(input: TeamActionInput) -> Self {
        Self {
            mon_action: MonAction::new(input.mon),
            index: input.index,
            priority: input.priority,
        }
    }
}

/// A switch action input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchActionInput {
    pub instant: bool,
    pub mon: MonHandle,
    pub switching_out: MonHandle,
    pub position: usize,
}

/// A switch action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchAction {
    pub instant: bool,
    pub mon_action: MonAction,
    pub switching_out: MonHandle,
    pub position: usize,
}

impl SwitchAction {
    /// Creates a new [`SwitchAction`] from [`SwitchActionInput`].
    pub fn new(input: SwitchActionInput) -> Self {
        Self {
            instant: input.instant,
            mon_action: MonAction::new(input.mon),
            switching_out: input.switching_out,
            position: input.position,
        }
    }
}

/// A move action input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveActionInput {
    pub id: Id,
    pub mon: MonHandle,
    pub target: Option<isize>,
    pub mega: bool,
}

/// A move action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveAction {
    pub id: Id,
    pub mon_action: MonAction,
    pub target: Option<isize>,
    pub original_target: Option<MonHandle>,
    pub mega: bool,
    pub priority: i32,
    pub sub_priority: u32,
}

impl MoveAction {
    /// Creates a new [`MoveAction`] from [`MoveActionInput`].
    pub fn new(input: MoveActionInput) -> Self {
        Self {
            id: input.id,
            mon_action: MonAction::new(input.mon),
            target: input.target,
            original_target: None,
            mega: input.mega,
            priority: 0,
            sub_priority: 0,
        }
    }
}

/// An experience action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceAction {
    pub mon: MonHandle,
    pub player_index: usize,
    pub mon_index: usize,
    pub active: bool,
    pub exp: u32,
}

/// A level up action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelUpAction {
    pub mon: MonHandle,
}

/// A learn move action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearnMoveAction {
    pub mon: MonHandle,
    pub forget_move_slot: usize,
}

/// An end action, which ends the battle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndAction {
    pub winning_side: Option<usize>,
}

/// An escape action input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscapeActionInput {
    pub mon: MonHandle,
}

/// An escape action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscapeAction {
    pub mon_action: MonAction,
}

impl EscapeAction {
    pub fn new(input: EscapeActionInput) -> Self {
        Self {
            mon_action: MonAction::new(input.mon),
        }
    }
}

/// An action during a battle.
///
/// Actions are the core of a battle. A turn of a battle consists of several actions running
/// sequentially. Actions can also be run outside of a turn for configuration and miscellaneous
/// purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Start,
    End(EndAction),
    Pass,
    BeforeTurn,
    BeforeTurnMove(MoveAction),
    Residual,
    Team(TeamAction),
    Switch(SwitchAction),
    Move(MoveAction),
    MegaEvo(MonAction),
    Experience(ExperienceAction),
    LevelUp(LevelUpAction),
    LearnMove(LearnMoveAction),
    Escape(EscapeAction),
}

impl Action {
    pub fn mon_action_mut(&mut self) -> Option<&mut MonAction> {
        match self {
            Self::Team(action) => Some(&mut action.mon_action),
            Self::Switch(action) => Some(&mut action.mon_action),
            Self::Move(action) => Some(&mut action.mon_action),
            Self::MegaEvo(action) => Some(action),
            Self::Escape(action) => Some(&mut action.mon_action),
            _ => None,
        }
    }
}

impl SpeedOrderable for Action {
    fn order(&self) -> u32 {
        match self {
            Self::Team(_) => 1,
            Self::Start => 2,
            Self::LearnMove(_) => 3,
            Self::LevelUp(_) => 4,
            Self::Experience(_) => 5,
            Self::Switch(action) => {
                if action.instant {
                    6
                } else {
                    100
                }
            }
            Self::End(_) => 7,
            Self::BeforeTurn => 8,
            Self::BeforeTurnMove(_) => 9,
            Self::Escape(_) => 101,
            Self::MegaEvo(_) => 102,
            Self::Move(_) => 200,
            Self::Pass => 200,
            Self::Residual => 300,
        }
    }

    fn priority(&self) -> i32 {
        match self {
            Self::Team(action) => action.priority,
            Self::Move(action) => action.priority,
            Self::Experience(action) => action.player_index as i32,
            _ => 0,
        }
    }

    fn speed(&self) -> u32 {
        match self {
            Self::Team(action) => action.mon_action.speed,
            Self::Switch(action) => action.mon_action.speed,
            Self::Move(action) => action.mon_action.speed,
            Self::MegaEvo(action) => action.speed,
            _ => 1,
        }
    }

    fn sub_order(&self) -> u32 {
        match self {
            Self::Move(action) => action.sub_priority,
            Self::Experience(action) => {
                // Active Mons should get experience before inactive Mons.
                if action.active {
                    action.mon_index as u32
                } else {
                    action.mon_index as u32 + 65535
                }
            }
            _ => 0,
        }
    }
}
