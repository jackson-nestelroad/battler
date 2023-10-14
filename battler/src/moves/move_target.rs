use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// The acceptable target(s) of a move.
///
/// In this enum, the following terms are used:
///     - "Adjacent" = A reachable Mon.
///     - "Ally" - A Mon on the same side.
///     - "Foe" - A Mon on the opposite side.
///     - "Side" - The side of a battle, not any particular Mon on that side.
///     - "Team" - All unfainted Mons on a team.
///     - "User" - The user of a move.
#[derive(Debug, Clone, PartialEq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
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
    /// The field or all Mons at once.
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
    /// The foe's side.
    #[string = "FoeSide"]
    FoeSide,
    /// One adjacent Mon of the user's choice.
    ///
    /// Could also be called "Adjacent."
    #[string = "Normal"]
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

#[cfg(test)]
mod move_target_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        moves::MoveTarget,
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
}
