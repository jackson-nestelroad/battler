use std::{
    fmt::Display,
    sync::LazyLock,
};

use anyhow::Error;
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

use crate::core::error::{
    BasicError,
    InteractionError,
};

#[derive(Debug, Error)]
#[error("invalid URI")]
pub struct InvalidUri;

/// Vaidates a strict URI.
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Uri(String);

impl Uri {
    pub fn from_known<S>(value: S) -> Self
    where
        S: Into<String>,
    {
        Self(value.into())
    }

    pub fn for_error(error: &Error) -> Self {
        if error.is::<InvalidUri>() {
            return Self::from_known("wamp.error.invalid_uri");
        } else if let Some(error) = error.downcast_ref::<BasicError>() {
            return Self::from_known(format!("wamp.error.{}", error.uri_component()));
        } else if let Some(error) = error.downcast_ref::<InteractionError>() {
            return Self::from_known(format!("wamp.error.{}", error.uri_component()));
        } else {
            return Self::from_known("wamp.error.unknown_error");
        }
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

#[cfg(test)]
mod uri_test {
    use crate::core::uri::{
        validate_strict_uri,
        Uri,
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
}
