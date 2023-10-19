use std::{
    fmt,
    fmt::{
        Debug,
        Display,
    },
    hash,
    hash::Hash,
};

/// A string that may or may not be owned.
///
/// An optimization that allows the [`Id`][`crate::common::Id`] type to directly store string
/// references that are known to already be valid IDs.
#[derive(Clone)]
pub enum MaybeOwnedString<'s> {
    Owned(String),
    Unowned(&'s str),
}

impl From<String> for MaybeOwnedString<'_> {
    fn from(value: String) -> Self {
        Self::Owned(value)
    }
}

impl<'s> From<&'s str> for MaybeOwnedString<'s> {
    fn from(value: &'s str) -> Self {
        Self::Unowned(value)
    }
}

impl AsRef<str> for MaybeOwnedString<'_> {
    fn as_ref(&self) -> &str {
        match self {
            Self::Owned(str) => str.as_ref(),
            Self::Unowned(str) => str,
        }
    }
}

impl PartialEq for MaybeOwnedString<'_> {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(self.as_ref(), other.as_ref())
    }
}

impl Eq for MaybeOwnedString<'_> {}

impl Hash for MaybeOwnedString<'_> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        Hash::hash(self.as_ref(), state)
    }
}

impl Display for MaybeOwnedString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl Debug for MaybeOwnedString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}
