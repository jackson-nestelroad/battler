use serde::{
    Deserialize,
    Serialize,
};

/// The relational role of an effect target relative to the acting Mon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetRole {
    /// The user itself.
    User,
    /// An ally partner on the same side of the field.
    Ally,
    /// An opposing Mon on the opposing side of the field.
    Foe,
    /// The user's side of the field (e.g., for Screens, Tailwind, Safeguard).
    AllySide,
    /// The opponent's side of the field (e.g., for Spikes, Stealth Rock, Sticky Web).
    FoeSide,
    /// The entire field or environment (e.g., Weather, Terrain, Trick Room).
    Field,
}

impl TargetRole {
    /// Checks if the target is on the user's team (User, Ally, or AllySide).
    pub fn is_friendly(&self) -> bool {
        matches!(self, Self::User | Self::Ally | Self::AllySide)
    }

    /// Checks if the target is an opponent (Foe or FoeSide).
    pub fn is_opposing(&self) -> bool {
        matches!(self, Self::Foe | Self::FoeSide)
    }

    /// Checks if the target is an individual Mon (User, Ally, or Foe).
    pub fn is_mon(&self) -> bool {
        matches!(self, Self::User | Self::Ally | Self::Foe)
    }

    /// Checks if the target is a side of the field.
    pub fn is_side(&self) -> bool {
        matches!(self, Self::AllySide | Self::FoeSide)
    }
}
