//! # battler-wamprat-uri
//!
//! **battler-wamprat-uri** is a utility crate for [`battler-wamprat`](https://crates.io/crates/battler-wamprat). It provides a procedural macro for dynamically matching URIs for WAMP subscriptions and procedure registrations.
//!
//! Pattern-based subscriptions and procedure registrations can be complex. They provide an
//! additional avenue of input for WAMP messages, outside the `arguments` and `arguments_keyword`
//! fields expressed by [`battler_wamprat_message::WampApplicationMessage`](https://docs.rs/battler-wamprat-message/latest/battler_wamprat_message/) types.
//!
//! For example, a callee can register a procedure on the wildcard URI `com.test.add..v2`. A caller
//! can then call this procedure using any URI that matches this pattern. One caller may call
//! `com.test.add.integers.v2` while another may call `com.test.add.strings.v2`. The wildcard URI
//! component (in this case, `integers` or `strings`) is likely very important to the callee!
//!
//! This crate provides runtime type checking and pattern matching to incoming and outgoing WAMP URI
//! patterns through the [`WampUriMatcher`] derive macro. A URI matcher can be thought of as a
//! *glorified regular expression*: each wildcard component is a *named capture group* that can
//! be referenced after matching. The end result is that URIs going out from a peer can be formatted
//! automatically, and URIs coming into a peer can be pattern matched, allowing the extracted URI
//! components to be easily read by application logic.
//!
//! ## Basic Usage
//!
//! A struct using the [`WampUriMatcher`] derive macro must have a `uri` attribute. The URI
//! describes the wildcard pattern, which is used for formatting outgoing URIs and matching incoming
//! URIs.
//!
//! In the simplest case, a struct with no fields can match a static URI:
//!
//! ```
//! use battler_wamprat_uri::WampUriMatcher;
//!
//! #[derive(WampUriMatcher)]
//! #[uri("com.test.add")]
//! struct AddUri {}
//! ```
//!
//! For each field you add, that field *must* be represented in the URI pattern. Otherwise, the
//! struct would be impossible to construct for incoming URIs.
//!
//! ```
//! use battler_wamprat_uri::WampUriMatcher;
//!
//! #[derive(WampUriMatcher)]
//! #[uri("com.test.add.{name}.v2")]
//! struct AddUri {
//!     name: String,
//! }
//! ```
//!
//! Each struct field must be convertible to and from a string, using
//! [`Display`][`core::fmt::Display`] and [`FromStr`][`core::str::FromStr`] respectively.
//! ```
//! use battler_wamprat_uri::WampUriMatcher;
//!
//! #[derive(WampUriMatcher)]
//! #[uri("com.test.add.{name}.{count}")]
//! struct AddUri {
//!     name: String,
//!     count: u64,
//! }
//! ```
//! ## Advanced Features
//!
//! ### Prefix Matching
//!
//! In some cases, a URI pattern may need to match in prefix form. Prefix matching is only possible
//! if the last field in the struct is marked with the `rest` attribute. This attribute requires
//! that an iterator of [`String`]s can be collected into its type.
//!
//! ```
//! use battler_wamprat_uri::WampUriMatcher;
//!
//! #[derive(WampUriMatcher)]
//! #[uri("com.test.fn.{a}.{rest}")]
//! struct PrefixUri {
//!     a: String,
//!     #[rest]
//!     rest: Vec<String>,
//! }
//! ```
//!
//! ### Field Repetition
//!
//! Fields can be repeated in the URI pattern. The first use of the field will be treated as the
//! source of truth, and all later uses of the field must match this first value.
//!
//! ```
//! use battler_wamprat_uri::WampUriMatcher;
//!
//! #[derive(WampUriMatcher)]
//! #[uri("com.test.fn.{a}.{b}.{a}")]
//! struct RepeatedUri {
//!     a: u64,
//!     b: u64,
//! }
//! ```
//!
//! ### Regular Expression Component Matching
//!
//! In most cases, struct fields should be isolated to their own URI components for simplicity.
//! However, this is not a strict requirement of the URI matcher macro.
//!
//! Strings and struct fields can be combined into the same URI component. If this is done, the URI
//! pattern matching is less enforceable on the router, but the peer will still validate and match
//! the URI as expected.
//!
//! ```
//! use battler_wamprat_uri::WampUriMatcher;
//!
//! #[derive(WampUriMatcher)]
//! #[uri("com.test.math.{a}log{b}")]
//! struct RegExUri {
//!     a: u64,
//!     b: u64,
//! }
//! ```
//!
//! Note that in the above case, the URI registered on the router will be the wildcard
//! `com.test.math.`, so the last component can be matched by *any* string. However, the `RegExUri`
//! type will enforce the additional restrictions. For instance, the URI `com.test.math.abc` will
//! be rejected by the peer, while `com.test.math.2log12` will be accepted.
//!
//! *Note: URI such as the one above will generate a dependency on the `regex` crate.*
//!
//! ### Generators
//!
//! It can sometimes be useful to generate partial-wildcard URIs based on a URI pattern. For
//! example, one peer may be publishing messages to `com.chat.{version}.{channel}.{author}`, and
//! another peer may want to subscribe to messages for a specific channel using the
//! `com.chat.1.main.{author}`. Rather than requiring the subscriber to generate this
//! wildcard URI manually, it can use a **derived generator struct** from base URI pattern.
//!
//! A generator struct implements the [`WampWildcardUriGenerator`] trait and can be generated
//! automatically with the `generator` macro attribute. You can generate multiple generators for a
//! single matcher.
//!
//! ```
//! use std::marker::PhantomData;
//!
//! use battler_wamprat_uri::{
//!     WampUriMatcher,
//!     Wildcard,
//!     WampWildcardUriGenerator,
//! };
//!
//! #[derive(Clone, WampUriMatcher)]
//! #[uri("com.chat.{version}.{channel}.{author}")]
//! #[generator(ChannelMessageUri, fixed(version = 1u64), require(channel), derive(Clone))]
//! struct MessageUri {
//!     version: u64,
//!     channel: String,
//!     author: String,
//! }
//!
//! fn main() {
//!     assert_matches::assert_matches!(ChannelMessageUri {
//!         version: PhantomData,
//!         channel: "main".to_owned(),
//!         author: Wildcard::Wildcard,
//!     }.wamp_generate_wildcard_uri(), Ok(uri) => {
//!         assert_eq!(uri.as_ref(), "com.chat.1.main.");
//!     });
//! }
//! ```
//!
//! You can also create generators for patterns with unnamed fields, but indices must be prefixed
//! with an underscore (e.g., `_0`, `_1`) in the `generator` attribute.
//!
//! **Note:** Generators are limited to wildcard URI patterns only. URI patterns with prefix
//! matching **cannot** use the `generator` attribute. You may, however, create your own generator
//! structs manually.

#![feature(slice_concat_trait)]
use battler_wamp::core::{
    error::WampError,
    match_style::MatchStyle,
    uri::{
        InvalidUri,
        Uri,
        WildcardUri,
    },
};
pub use battler_wamprat_uri_proc_macro::WampUriMatcher;
use thiserror::Error;

/// An error resulting from attempting to match a [`Uri`] using a [`WampUriMatcher`].
#[derive(Debug, Error)]
#[error("{msg}")]
pub struct WampUriMatchError {
    msg: String,
}

impl WampUriMatchError {
    pub fn new<S>(msg: S) -> Self
    where
        S: Into<String>,
    {
        Self { msg: msg.into() }
    }
}

impl Into<WampError> for WampUriMatchError {
    fn into(self) -> WampError {
        WampError::new(
            Uri::try_from("com.battler_wamprat.uri_match_error").unwrap(),
            self.to_string(),
        )
    }
}

impl TryFrom<WampError> for WampUriMatchError {
    type Error = WampError;
    fn try_from(value: WampError) -> Result<Self, Self::Error> {
        if value.reason().as_ref() == "com.battler_wamprat.uri_match_error" {
            Ok(Self {
                msg: value.message().to_owned(),
            })
        } else {
            Err(value)
        }
    }
}

/// A dynamic WAMP URI matcher, which configures a URI pattern for incoming and outgoing URIs.
///
/// This type can receive a WAMP URI as input and parse it to this type (e.g., for callee-side
/// invocations), or it can generate a URI based on itself (e.g., for caller-side invocations).
pub trait WampUriMatcher: Sized {
    /// The wildcard URI for the router.
    fn uri_for_router() -> WildcardUri;

    /// The match style of the URI matcher.
    fn match_style() -> Option<MatchStyle>;

    /// Matches an incoming URI to the configured pattern.
    fn wamp_match_uri(uri: &str) -> Result<Self, WampUriMatchError>;

    /// Generates an outgoing URI for the configured values.
    fn wamp_generate_uri(&self) -> Result<Uri, InvalidUri>;
}

/// A field of a custom generator of a [`WampUriMatcher`] implementation (a.k.a., an implementation
/// of [`WampWildcardUriGenerator`]) where wildcard is allowed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Wildcard<T> {
    #[default]
    Wildcard,
    Value(T),
}

impl<T> std::fmt::Display for Wildcard<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wildcard => write!(f, ""),
            Self::Value(val) => val.fmt(f),
        }
    }
}

impl<T> std::str::FromStr for Wildcard<T>
where
    T: std::str::FromStr,
{
    type Err = <T as std::str::FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Self::Wildcard)
        } else {
            Ok(Self::Value(T::from_str(s)?))
        }
    }
}

impl<T> From<T> for Wildcard<T> {
    fn from(value: T) -> Self {
        Wildcard::Value(value)
    }
}

/// A custom generator of a [`WampUriMatcher`] implementation where wildcards may be used to
/// generate [`WildcardUri`]s.
pub trait WampWildcardUriGenerator<T> {
    /// Generates an outgoing URI for the configured values.
    fn wamp_generate_wildcard_uri(&self) -> Result<WildcardUri, InvalidUri>;
}

impl<T> WampWildcardUriGenerator<T> for T
where
    T: WampUriMatcher,
{
    fn wamp_generate_wildcard_uri(&self) -> Result<WildcardUri, InvalidUri> {
        Ok(WildcardUri::from(self.wamp_generate_uri()?))
    }
}
