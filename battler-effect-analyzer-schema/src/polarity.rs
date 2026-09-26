use serde::{
    Deserialize,
    Serialize,
};

/// The semantic intent of an effect action on its recipient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectPolarity {
    /// Beneficial to the recipient (e.g., Heal, Stat Buff, Screen, Tailwind).
    Beneficial,
    /// Harmful to the recipient (e.g., Damage, Stat Debuff, Status, Hazard, Taunt).
    Harmful,
    /// Contextual or Neutral (e.g., Trick Room, Weather, Haze, Transform).
    Neutral,
}

impl EffectPolarity {
    /// Checks if the polarity is beneficial.
    pub fn is_beneficial(&self) -> bool {
        matches!(self, Self::Beneficial)
    }

    /// Checks if the polarity is harmful.
    pub fn is_harmful(&self) -> bool {
        matches!(self, Self::Harmful)
    }

    /// Checks if the polarity is neutral.
    pub fn is_neutral(&self) -> bool {
        matches!(self, Self::Neutral)
    }

    /// Inverts the polarity (e.g., for inverted stat changes).
    pub fn invert(&self) -> Self {
        match self {
            Self::Beneficial => Self::Harmful,
            Self::Harmful => Self::Beneficial,
            Self::Neutral => Self::Neutral,
        }
    }
}
