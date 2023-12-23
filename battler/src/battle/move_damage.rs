/// Damage dealt by an individual hit of a move.
pub enum MoveDamage {
    Failure,
    SilentFailure,
    Damage(u16),
}

impl MoveDamage {
    pub fn hit(&self) -> bool {
        match self {
            Self::Failure | Self::SilentFailure => false,
            Self::Damage(_) => true,
        }
    }
}

impl Into<Option<u16>> for MoveDamage {
    fn into(self) -> Option<u16> {
        match self {
            Self::Failure | Self::SilentFailure => None,
            Self::Damage(damage) => Some(damage),
        }
    }
}
