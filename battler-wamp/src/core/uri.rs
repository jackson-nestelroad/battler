use std::{
    fmt::Display,
    sync::LazyLock,
};

use regex::Regex;
use serde::{
    de::{
        Unexpected,
        Visitor,
    },
    Deserialize,
    Serialize,
};
use thiserror::Error;

/// Error for an invalid URI.
#[derive(Debug, Error)]
#[error("invalid URI")]
pub struct InvalidUri;

/// Validates a strict URI.
pub fn validate_strict_uri<S>(uri: S) -> Result<(), InvalidUri>
where
    S: AsRef<str>,
{
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^([0-9a-z_]+\.)*([0-9a-z_]+)$").unwrap());
    if !RE.is_match(uri.as_ref()) {
        return Err(InvalidUri);
    }
    Ok(())
}

/// Validates a URI with wildcards.
pub fn validate_wildcard_uri<S>(uri: S) -> Result<(), InvalidUri>
where
    S: AsRef<str>,
{
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^([0-9a-z_]*\.)*([0-9a-z_]*)$").unwrap());
    if !RE.is_match(uri.as_ref()) {
        return Err(InvalidUri);
    }
    Ok(())
}

/// A uniform resource identifier, used in many aspects of WAMP messaging for identifying resources,
/// such as realms, topics, and procedures.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Uri(String);

impl Uri {
    /// Constructs a URI directly from a value known to be valid, skipping validation.
    pub(crate) fn from_known<S>(value: S) -> Self
    where
        S: Into<String>,
    {
        Self(value.into())
    }

    /// Splits the URI into its components.
    pub fn split(&self) -> impl Iterator<Item = &str> {
        self.0.split('.')
    }
}

impl Display for Uri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for Uri {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Uri {
    type Error = InvalidUri;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_strict_uri(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&str> for Uri {
    type Error = InvalidUri;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        validate_strict_uri(value)?;
        Ok(Self(value.to_owned()))
    }
}

impl TryFrom<WildcardUri> for Uri {
    type Error = InvalidUri;

    fn try_from(value: WildcardUri) -> Result<Self, Self::Error> {
        validate_strict_uri(&value.0)?;
        Ok(Self(value.0))
    }
}

impl TryFrom<&WildcardUri> for Uri {
    type Error = InvalidUri;

    fn try_from(value: &WildcardUri) -> Result<Self, Self::Error> {
        validate_strict_uri(&value.0)?;
        Ok(Self(value.0.clone()))
    }
}

impl Into<String> for Uri {
    fn into(self) -> String {
        self.0
    }
}

struct UriVisitor;

impl<'de> Visitor<'de> for UriVisitor {
    type Value = Uri;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a URI")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Uri::try_from(v.to_owned()).map_err(|_| E::invalid_value(Unexpected::Str(&v), &self))
    }
}

impl<'de> Deserialize<'de> for Uri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(UriVisitor)
    }
}

/// [`Uri`], but with wildcards allowed.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct WildcardUri(String);

impl WildcardUri {
    /// Splits the URI into its components.
    pub fn split(&self) -> impl Iterator<Item = &str> {
        self.0.split('.')
    }
}

impl Display for WildcardUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for WildcardUri {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WildcardUri {
    type Error = InvalidUri;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_wildcard_uri(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&str> for WildcardUri {
    type Error = InvalidUri;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        validate_wildcard_uri(value)?;
        Ok(Self(value.to_owned()))
    }
}

impl Into<String> for WildcardUri {
    fn into(self) -> String {
        self.0
    }
}

impl From<Uri> for WildcardUri {
    fn from(value: Uri) -> Self {
        Self(value.0)
    }
}
struct WildcardUriVisitor;

impl<'de> Visitor<'de> for WildcardUriVisitor {
    type Value = WildcardUri;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a wildcard URI")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        WildcardUri::try_from(v.to_owned())
            .map_err(|_| E::invalid_value(Unexpected::Str(&v), &self))
    }
}

impl<'de> Deserialize<'de> for WildcardUri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(WildcardUriVisitor)
    }
}

#[cfg(test)]
mod uri_test {
    use crate::core::uri::{
        validate_strict_uri,
        validate_wildcard_uri,
        Uri,
        WildcardUri,
    };

    #[test]
    fn validates_strict_uris() {
        assert_matches::assert_matches!(validate_strict_uri("com"), Ok(()));
        assert_matches::assert_matches!(validate_strict_uri("com123"), Ok(()));
        assert_matches::assert_matches!(validate_strict_uri("com.battler.topic"), Ok(()));
        assert_matches::assert_matches!(validate_strict_uri("com.battler.TOPIC"), Err(_));
        assert_matches::assert_matches!(validate_strict_uri("com.battler.topic_123-@!!"), Err(_));
        assert_matches::assert_matches!(validate_strict_uri("com.1"), Ok(()));
        assert_matches::assert_matches!(validate_strict_uri("."), Err(_));
        assert_matches::assert_matches!(validate_strict_uri(".."), Err(_));
        assert_matches::assert_matches!(validate_strict_uri(".com.battler.topic1"), Err(_));
        assert_matches::assert_matches!(validate_strict_uri("com.battler#"), Err(_));
    }

    #[test]
    fn fails_deserialization_invalid_uri() {
        assert_matches::assert_matches!(serde_json::from_str::<Uri>(r#""com.battler.TOPIC""#), Err(err) => {
            assert!(err.to_string().contains("expected a URI"));
        });
    }

    #[test]
    fn validates_wildcard_uris() {
        assert_matches::assert_matches!(validate_wildcard_uri("com"), Ok(()));
        assert_matches::assert_matches!(validate_wildcard_uri("com123"), Ok(()));
        assert_matches::assert_matches!(validate_wildcard_uri("com.battler.topic"), Ok(()));
        assert_matches::assert_matches!(validate_wildcard_uri("com.battler..topic"), Ok(()));
        assert_matches::assert_matches!(
            validate_wildcard_uri("com.battler..topic..a.b...c"),
            Ok(())
        );
        assert_matches::assert_matches!(validate_wildcard_uri("com.battler.TOPIC"), Err(_));
        assert_matches::assert_matches!(validate_wildcard_uri("com.battler.topic_123-@!!"), Err(_));
        assert_matches::assert_matches!(validate_wildcard_uri("com.1"), Ok(()));
        assert_matches::assert_matches!(validate_wildcard_uri("."), Ok(()));
        assert_matches::assert_matches!(validate_wildcard_uri(".."), Ok(()));
        assert_matches::assert_matches!(validate_wildcard_uri(".com.battler.topic1"), Ok(()));
        assert_matches::assert_matches!(validate_wildcard_uri("com.battler#"), Err(_));
    }

    #[test]
    fn fails_deserialization_invalid_wildcard_uri() {
        assert_matches::assert_matches!(serde_json::from_str::<WildcardUri>(r#""com.battler..TOPIC""#), Err(err) => {
            assert!(err.to_string().contains("expected a wildcard URI"));
        });
    }
}
