use std::{
    borrow::Cow,
    fmt,
    fmt::{
        Debug,
        Display,
    },
    hash,
    hash::Hash,
    str::FromStr,
};

use lazy_static::lazy_static;
use regex::Regex;
use serde::{
    de::Visitor,
    Deserialize,
    Serialize,
};

use crate::common::Error;

/// A string that may or may not be owned.
///
/// An optimization that allows the [`Id`] type to directly store string references that are known
/// to already be valid IDs.
#[derive(Clone)]
enum MaybeOwnedString {
    Owned(String),
    Unowned(&'static str),
}

impl AsRef<str> for MaybeOwnedString {
    fn as_ref(&self) -> &str {
        match self {
            Self::Owned(str) => str.as_ref(),
            Self::Unowned(str) => str,
        }
    }
}

impl PartialEq for MaybeOwnedString {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(self.as_ref(), other.as_ref())
    }
}

impl Eq for MaybeOwnedString {}

impl Hash for MaybeOwnedString {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        Hash::hash(self.as_ref(), state)
    }
}

impl Display for MaybeOwnedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl Debug for MaybeOwnedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

/// An ID for a resource.
///
/// Resources of the same type should have a unique ID.
///
/// A futher optimization would be to allocate strings in an arena for memory proximity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Id(MaybeOwnedString);

/// A reference to an ID for a resource.
///
/// This type is primarily for optimization purposes. Some code needs IDs but doesn't necessarily
/// need to own them. Thus, this type provides ID comparisons for unowned strings.
struct IdRef<'s>(&'s str);

impl Id {
    pub(crate) fn from_known(value: &'static str) -> Self {
        Self(MaybeOwnedString::Unowned(value))
    }

    #[allow(unused)]
    fn as_id_ref(&self) -> IdRef {
        IdRef(self.0.as_ref())
    }
}

impl<'s> IdRef<'s> {
    fn considered_chars(&'s self) -> impl Iterator<Item = char> + 's {
        self.0.chars().filter_map(|c| match c {
            '0'..='9' => Some(c),
            'a'..='z' => Some(c),
            'A'..='Z' => Some(c.to_ascii_lowercase()),
            _ => None,
        })
    }
}

impl PartialEq for IdRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.considered_chars().eq(other.considered_chars())
    }
}

impl Eq for IdRef<'_> {}

impl Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Display for IdRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self.0, f)
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl From<String> for Id {
    fn from(value: String) -> Self {
        normalize_id(&value)
    }
}

impl From<&str> for Id {
    fn from(value: &str) -> Self {
        normalize_id(value)
    }
}

impl FromStr for Id {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Id::from(s))
    }
}

impl From<IdRef<'_>> for Id {
    fn from(value: IdRef) -> Self {
        Id::from(value.0.to_owned())
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}

struct IdVisitor;

impl<'de> Visitor<'de> for IdVisitor {
    type Value = Id;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v))
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(IdVisitor)
    }
}

/// A trait that provides a common way of identifying resources.
///
/// Resources of the same type should have a unique ID.
pub trait Identifiable {
    fn id(&self) -> &Id;
}

/// Normalizes the given ID.
///
/// IDs must have lowercase alphanumeric characters. Non-alphanumeric characters are removed.
fn normalize_id(id: &str) -> Id {
    lazy_static! {
        static ref PATTERN: Regex = Regex::new(r"[^a-z0-9]").unwrap();
    }
    match PATTERN.replace_all(&id.to_ascii_lowercase(), "") {
        // There is an optimization to be done here. If this is a &'static str, we can save it
        // without owning it. However, this code is shared for all &str, so we cannot make the
        // distinction as is.
        Cow::Borrowed(str) => Id(MaybeOwnedString::Owned(str.to_owned())),
        Cow::Owned(str) => Id(MaybeOwnedString::Owned(str)),
    }
}

#[cfg(test)]
mod id_from_tests {
    use crate::common::Id;

    fn assert_normalize_id(input: &str, output: &str) {
        assert_eq!(Id::from(input), Id::from(output));
    }

    #[test]
    fn removes_non_alphanumeric_characters() {
        assert_normalize_id("Bulbasaur", "bulbasaur");
        assert_normalize_id("CHARMANDER", "charmander");
        assert_normalize_id("Porygon-Z", "porygonz");
        assert_normalize_id("Flabébé", "flabb");
        assert_normalize_id("Giratina (Origin)", "giratinaorigin");
    }
}
