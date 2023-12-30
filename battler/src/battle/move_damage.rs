/// Damage dealt by an individual hit of a move.
pub enum MoveDamage {
    Failure,
    Damage(u16),
}

impl MoveDamage {
    pub fn hit(&self) -> bool {
        match self {
            Self::Failure => false,
            Self::Damage(_) => true,
        }
    }

    pub fn failed(&self) -> bool {
        match self {
            Self::Failure => true,
            Self::Damage(_) => false,
        }
    }
}

impl Into<Option<u16>> for MoveDamage {
    fn into(self) -> Option<u16> {
        match self {
            Self::Failure => None,
            Self::Damage(damage) => Some(damage),
        }
    }
}
