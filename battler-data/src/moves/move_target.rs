use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// The acceptable target(s) of a move.
///
/// In this enum, the following terms are used:
/// - "Adjacent" = A reachable Mon.
/// - "Ally" - A Mon on the same side.
/// - "Foe" - A Mon on the opposite side.
/// - "Side" - The side of a battle, not any particular Mon on that side.
/// - "Team" - All unfainted Mons on a team.
/// - "User" - The user of a move.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerializeLabeledStringEnum,
    DeserializeLabeledStringEnum,
)]
pub enum MoveTarget {
    /// An adjacent ally.
    #[string = "AdjacentAlly"]
    AdjacentAlly,
    /// The user or its ally.
    #[string = "AdjacentAllyOrUser"]
    AdjacentAllyOrUser,
    /// An adjacent foe.
    #[string = "AdjacentFoe"]
    AdjacentFoe,
    /// All Mons at once.
    #[string = "All"]
    All,
    /// All adjacent mons (including allies).
    #[string = "AllAdjacent"]
    AllAdjacent,
    /// All adjacent foes.
    ///
    /// Also known as a spread move.
    #[string = "AllAdjacentFoes"]
    AllAdjacentFoes,
    /// All active Mons on the user's team.
    #[string = "Allies"]
    Allies,
    /// The user's side.
    #[string = "AllySide"]
    AllySide,
    /// All unfainted Mons on the user's team.
    #[string = "AllyTeam"]
    AllyTeam,
    /// Any other active Mon.
    #[string = "Any"]
    Any,
    /// The field.
    #[string = "Field"]
    Field,
    /// The foe's side.
    #[string = "FoeSide"]
    FoeSide,
    /// One adjacent Mon of the user's choice.
    ///
    /// Could also be called "Adjacent."
    #[string = "Normal"]
    #[default]
    Normal,
    /// Any adjacent foe chosen at random.
    #[string = "RandomNormal"]
    RandomNormal,
    /// The for that damaged the user.
    #[string = "Scripted"]
    Scripted,
    /// The user of the move.
    #[string = "User"]
    User,
}

impl MoveTarget {
    /// Is the move target choosable?
    pub fn choosable(&self) -> bool {
        match self {
            Self::Normal
            | Self::Any
            | Self::AdjacentAlly
            | Self::AdjacentAllyOrUser
            | Self::AdjacentFoe => true,
            _ => false,
        }
    }

    /// Does the move require a single target?
    pub fn requires_target(&self) -> bool {
        match self {
            Self::All
            | Self::AllAdjacent
            | Self::AllAdjacentFoes
            | Self::Allies
            | Self::AllySide
            | Self::AllyTeam
            | Self::Field
            | Self::FoeSide
            | Self::Scripted => false,
            _ => true,
        }
    }

    /// Does the move have a single target?
    pub fn has_single_target(&self) -> bool {
        match self {
            Self::All
            | Self::AllAdjacent
            | Self::AllAdjacentFoes
            | Self::Allies
            | Self::AllySide
            | Self::AllyTeam
            | Self::Field
            | Self::FoeSide => false,
            _ => true,
        }
    }

    /// Does the move affect Mons directly?
    pub fn affects_mons_directly(&self) -> bool {
        match self {
            Self::AllySide | Self::AllyTeam | Self::Field | Self::FoeSide => false,
            _ => true,
        }
    }

    /// Can the move target the user?
    pub fn can_target_user(&self) -> bool {
        match self {
            Self::User
            | Self::All
            | Self::Allies
            | Self::AllySide
            | Self::AllyTeam
            | Self::AdjacentAllyOrUser => true,
            _ => false,
        }
    }

    /// Can the move target foes?
    pub fn can_target_foes(&self) -> bool {
        match self {
            Self::AdjacentAlly
            | Self::AdjacentAllyOrUser
            | Self::Allies
            | Self::AllySide
            | Self::AllyTeam => false,
            _ => true,
        }
    }

    /// Can the move only target adjacent Mons?
    pub fn is_adjacent_only(&self) -> bool {
        match self {
            Self::AdjacentAlly
            | Self::AdjacentAllyOrUser
            | Self::AdjacentFoe
            | Self::AllAdjacent
            | Self::AllAdjacentFoes
            | Self::Normal
            | Self::RandomNormal => true,
            _ => false,
        }
    }

    /// Is the target randomly selected?
    pub fn is_random(&self) -> bool {
        match self {
            Self::RandomNormal => true,
            _ => false,
        }
    }

    /// Validates the relative target position.
    pub fn valid_target(&self, relative_target: isize, adjacency_reach: u8) -> bool {
        match self {
            Self::AdjacentAlly
            | Self::AdjacentAllyOrUser
            | Self::AdjacentFoe
            | Self::Any
            | Self::Normal
            | Self::RandomNormal
            | Self::Scripted
            | Self::User => self.is_affected(relative_target, adjacency_reach),
            _ => false,
        }
    }

    /// Checks if the Mon at the relative target is affected by the move.
    pub fn is_affected(&self, relative_target: isize, adjacency_reach: u8) -> bool {
        let is_self = relative_target == 0;
        let is_foe = relative_target > 0;
        let is_adjacent = if relative_target > 0 {
            // Foe side, at most two steps away.
            relative_target <= adjacency_reach as isize
        } else {
            // Same side, at most one step away.
            relative_target == -(adjacency_reach as isize) + 1
        };

        match self {
            Self::AdjacentAlly => is_adjacent && !is_foe,
            Self::AdjacentAllyOrUser => (is_adjacent && !is_foe) || is_self,
            Self::AdjacentFoe | Self::AllAdjacentFoes => is_adjacent && is_foe,
            Self::All => true,
            Self::AllAdjacent => is_adjacent,
            Self::Allies => !is_foe && !is_self,
            Self::AllySide | Self::AllyTeam => !is_foe,
            Self::Any => !is_self,
            Self::Field => true,
            Self::FoeSide => is_foe,
            Self::Normal | Self::RandomNormal | Self::Scripted => is_adjacent,
            Self::User => is_self,
        }
    }
}

#[cfg(test)]
mod move_target_test {
    use crate::{
        moves::MoveTarget,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(MoveTarget::AdjacentAlly, "AdjacentAlly");
        test_string_serialization(MoveTarget::AllAdjacentFoes, "AllAdjacentFoes");
        test_string_serialization(MoveTarget::RandomNormal, "RandomNormal");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("normal", MoveTarget::Normal);
        test_string_deserialization("allyTeam", MoveTarget::AllyTeam);
        test_string_deserialization("foeside", MoveTarget::FoeSide);
    }

    #[test]
    fn choosable() {
        assert!(MoveTarget::Normal.choosable());
        assert!(MoveTarget::Any.choosable());
        assert!(MoveTarget::AdjacentAlly.choosable());
        assert!(MoveTarget::AdjacentAllyOrUser.choosable());
        assert!(MoveTarget::AdjacentFoe.choosable());
        assert!(!MoveTarget::RandomNormal.choosable());
        assert!(!MoveTarget::All.choosable());
        assert!(!MoveTarget::AllAdjacentFoes.choosable());
    }

    #[test]
    fn valid_target_any_adjacent() {
        assert!(MoveTarget::RandomNormal.valid_target(1, 2));
        assert!(MoveTarget::Scripted.valid_target(1, 2));
        assert!(MoveTarget::Normal.valid_target(1, 2));
        assert!(MoveTarget::RandomNormal.valid_target(2, 2));
        assert!(MoveTarget::Scripted.valid_target(2, 2));
        assert!(MoveTarget::Normal.valid_target(2, 2));
        assert!(MoveTarget::RandomNormal.valid_target(-1, 2));
        assert!(MoveTarget::Scripted.valid_target(-1, 2));
        assert!(MoveTarget::Normal.valid_target(-1, 2));

        assert!(!MoveTarget::Normal.valid_target(0, 2));
        assert!(!MoveTarget::Normal.valid_target(3, 2));
        assert!(!MoveTarget::Normal.valid_target(-2, 2));

        assert!(MoveTarget::Normal.valid_target(3, 3));
        assert!(MoveTarget::Normal.valid_target(-2, 3));
        assert!(!MoveTarget::Normal.valid_target(4, 3));
        assert!(!MoveTarget::Normal.valid_target(-3, 3));
    }

    #[test]
    fn valid_target_adjacent_ally() {
        assert!(MoveTarget::AdjacentAlly.valid_target(-1, 2));

        assert!(!MoveTarget::AdjacentAlly.valid_target(0, 2));
        assert!(!MoveTarget::AdjacentAlly.valid_target(1, 2));
        assert!(!MoveTarget::AdjacentAlly.valid_target(2, 2));
        assert!(!MoveTarget::AdjacentAlly.valid_target(3, 2));
        assert!(!MoveTarget::AdjacentAlly.valid_target(-2, 2));

        assert!(MoveTarget::AdjacentAlly.valid_target(-2, 3));
        assert!(!MoveTarget::AdjacentAlly.valid_target(-3, 3));
    }

    #[test]
    fn valid_target_adjacent_ally_or_user() {
        assert!(MoveTarget::AdjacentAllyOrUser.valid_target(-1, 2));
        assert!(MoveTarget::AdjacentAllyOrUser.valid_target(0, 2));

        assert!(!MoveTarget::AdjacentAllyOrUser.valid_target(1, 2));
        assert!(!MoveTarget::AdjacentAllyOrUser.valid_target(2, 2));
        assert!(!MoveTarget::AdjacentAllyOrUser.valid_target(3, 2));
        assert!(!MoveTarget::AdjacentAllyOrUser.valid_target(-2, 2));

        assert!(MoveTarget::AdjacentAllyOrUser.valid_target(-2, 3));
        assert!(!MoveTarget::AdjacentAllyOrUser.valid_target(-3, 3));
    }

    #[test]
    fn valid_target_adjacent_foe() {
        assert!(MoveTarget::AdjacentFoe.valid_target(1, 2));
        assert!(MoveTarget::AdjacentFoe.valid_target(2, 2));

        assert!(!MoveTarget::AdjacentFoe.valid_target(0, 2));
        assert!(!MoveTarget::AdjacentFoe.valid_target(3, 2));
        assert!(!MoveTarget::AdjacentFoe.valid_target(-1, 2));
        assert!(!MoveTarget::AdjacentFoe.valid_target(-2, 2));

        assert!(MoveTarget::AdjacentFoe.valid_target(3, 3));
        assert!(!MoveTarget::AdjacentFoe.valid_target(-2, 3));
        assert!(!MoveTarget::AdjacentFoe.valid_target(4, 3));
        assert!(!MoveTarget::AdjacentFoe.valid_target(-3, 3));
    }

    #[test]
    fn valid_target_any_but_user() {
        assert!(MoveTarget::Any.valid_target(1, 2));
        assert!(MoveTarget::Any.valid_target(2, 2));
        assert!(MoveTarget::Any.valid_target(3, 2));
        assert!(MoveTarget::Any.valid_target(-1, 2));
        assert!(MoveTarget::Any.valid_target(-2, 2));

        assert!(!MoveTarget::Any.valid_target(0, 2));
    }

    #[test]
    fn valid_target_user() {
        assert!(MoveTarget::User.valid_target(0, 2));

        assert!(!MoveTarget::User.valid_target(1, 2));
        assert!(!MoveTarget::User.valid_target(2, 2));
        assert!(!MoveTarget::User.valid_target(3, 2));
        assert!(!MoveTarget::User.valid_target(-1, 2));
        assert!(!MoveTarget::User.valid_target(-2, 2));
    }
}
