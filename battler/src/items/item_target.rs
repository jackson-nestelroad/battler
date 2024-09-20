use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// The acceptable target of an item.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum ItemTarget {
    /// A Mon in the player's party.
    #[string = "Party"]
    Party,
    /// The active Mon that the player is controlling.
    #[string = "Active"]
    Active,
    /// A foe on the battle field.
    #[string = "Foe"]
    Foe,
    /// An isolated foe on the battle field.
    #[string = "IsolatedFoe"]
    IsolatedFoe,
}

impl ItemTarget {
    /// Does the item require a single target?
    pub fn requires_target(&self) -> bool {
        match self {
            Self::Party | Self::Foe => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod item_target_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        items::ItemTarget,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(ItemTarget::Party, "Party");
        test_string_serialization(ItemTarget::Foe, "Foe");
        test_string_serialization(ItemTarget::IsolatedFoe, "IsolatedFoe");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("party", ItemTarget::Party);
        test_string_deserialization("foe", ItemTarget::Foe);
        test_string_deserialization("isolatedfoe", ItemTarget::IsolatedFoe);
    }
}
