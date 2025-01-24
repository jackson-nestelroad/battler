//! # battler-wamprat-error
//!
//! **battler-wamprat-error** is a utility crate for [`battler-wamprat`](https://crates.io/crates/battler-wamprat). It provides a procedural macro for custom WAMP errors.
//!
//! `battler_wamp` and `battler_wamprat` use the `battler_wamp::core::error::WampError` type for
//! errors transmitted between peers and routers. Errors are categorized by their URI and may be
//! contextualized by an error message.
//!
//! Custom WAMP error types can implement conversions to and from the core
//! `battler_wamp::core::error::WampError` type for use in application code (and as a part of the
//! `battler_wamprat`) framework. The [`WampError`] derive macro generates these conversions for
//! you. It works for structs or enums. All structs and enum variants must have the following
//! properties:
//!
//! 1. Have a valid URI attribute for identification.
//! 1. Be a unit struct/variant or be constructible from a single [`&str`][`str`] (the error
//!    message).
//!
//! ## Example
//!
//! ```
//! use battler_wamp::core::{
//!     error::WampError,
//!     uri::Uri,
//! };
//! use battler_wamprat_error::WampError;
//!
//! #[derive(Debug, PartialEq, WampError)]
//! enum CustomError {
//!     #[uri("com.test.missing_input")]
//!     MissingInput,
//!     #[uri("com.test.failed")]
//!     Failed(String),
//! }
//!
//! impl std::fmt::Display for CustomError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         match self {
//!             Self::MissingInput => write!(f, "missing input!"),
//!             Self::Failed(msg) => write!(f, "{msg}"),
//!         }
//!     }
//! }
//!
//! fn main() {
//!     // Into.
//!     assert_eq!(
//!         Into::<WampError>::into(CustomError::MissingInput),
//!         WampError::new(
//!             Uri::try_from("com.test.missing_input").unwrap(),
//!             "missing input!"
//!         )
//!     );
//!     assert_eq!(
//!         Into::<WampError>::into(CustomError::Failed("connection lost".to_owned())),
//!         WampError::new(Uri::try_from("com.test.failed").unwrap(), "connection lost")
//!     );
//!
//!     // From.
//!     assert_eq!(
//!         TryInto::<CustomError>::try_into(WampError::new(
//!             Uri::try_from("com.test.missing_input").unwrap(),
//!             ""
//!         ))
//!         .unwrap(),
//!         CustomError::MissingInput
//!     );
//!     assert_eq!(
//!         TryInto::<CustomError>::try_into(WampError::new(
//!             Uri::try_from("com.test.failed").unwrap(),
//!             "deadline exceeded"
//!         ))
//!         .unwrap(),
//!         CustomError::Failed("deadline exceeded".to_owned())
//!     );
//! }
//! ```

pub use battler_wamprat_error_proc_macro::WampError;
