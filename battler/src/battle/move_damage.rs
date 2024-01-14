/// Damage dealt by an individual hit of a move.
pub enum MoveDamage {
    Failure,
    None,
    Damage(u16),
}

impl MoveDamage {
    pub fn hit(&self) -> bool {
        match self {
            Self::Failure => false,
            Self::None => false,
            Self::Damage(_) => true,
        }
    }

    pub fn failed(&self) -> bool {
        match self {
            Self::Failure => true,
            Self::None => false,
            Self::Damage(_) => false,
        }
    }
}

impl Into<Option<u16>> for MoveDamage {
    fn into(self) -> Option<u16> {
        match self {
            Self::Failure => None,
            Self::None => None,
            Self::Damage(damage) => Some(damage),
        }
    }
}
