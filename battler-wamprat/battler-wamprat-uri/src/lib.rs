use battler_wamp::core::{
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

/// A dynamic WAMP URI matcher, which configures a URI pattern for incoming and outgoing URIs.
///
/// This type can receive a WAMP URI as input and parse it to this type (e.g., for callee-side
/// invocations), or it can generate a URI based on itself (e.g., for caller-side invocations).
pub trait WampUriMatcher: Sized {
    /// The wildcard URI for the router.
    fn uri_for_router(&self) -> WildcardUri;

    /// The match style of the URI matcher.
    fn match_style(&self) -> MatchStyle;

    /// Matches an incoming URI to the configured pattern.
    fn wamp_match_uri(uri: &str) -> Result<Self, WampUriMatchError>;

    /// Generates an outgoing URI for the configured values.
    fn wamp_generate_uri(&self) -> Result<Uri, InvalidUri>;
}
