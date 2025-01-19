//! # battler-wamprat-message
//!
//! **battler-wamprat-message** is a utility crate for [`battler-wamprat`](https://crates.io/crates/battler-wamprat). It provides a procedural macro for serializing and deserializing Rust structs as WAMP application messages that can be used for RPCs and pub/sub events over the WAMP protocol.
//!
//! A WAMP application message is simply a message consisting of `arguments` (a
//! [`List`][`battler_wamp_values::List`] field) and `arguments_keyword` (a
//! [`Dictionary`][`battler_wamp_values::Dictionary`]). The [`WampApplicationMessage`] derive macro
//! simply adds runtime-enforced type checking on incoming WAMP messages, such as pub/sub events,
//! incoming RPC invocations, and RPC results.
//!
//! An application message can have one field marked `arguments` (whose type should use the
//! [`WampList`][`battler_wamp_values::WampList`] macro) and one field marked `arguments_keyword`
//! (whose type should use the [`WampDictionary`][`battler_wamp_values::WampDictionary`] macro). You
//! may omit either of these fields as needed.
//!
//! *Note: This trait is used within the `battler-wamprat` framework. It is not intended to be
//! called directly.*
//!
//! ## Example
//!
//! ```
//! use battler_wamp_values::{
//!     Dictionary,
//!     Integer,
//!     List,
//!     Value,
//!     WampDictionary,
//!     WampList,
//! };
//! use battler_wamprat_message::WampApplicationMessage;
//!
//! #[derive(Debug, PartialEq, Eq, WampList)]
//! struct Args {
//!     a: Integer,
//!     b: Integer,
//! }
//!
//! #[derive(Debug, PartialEq, Eq, WampDictionary)]
//! struct Options {
//!     dry_run: bool,
//!     name: String,
//! }
//!
//! #[derive(Debug, PartialEq, Eq, WampApplicationMessage)]
//! struct Input {
//!     #[arguments]
//!     args: Args,
//!     #[arguments_keyword]
//!     options: Options,
//! }
//!
//! fn main() {
//!     // Serialization.
//!     assert_eq!(
//!         Input {
//!             args: Args { a: 123, b: 456 },
//!             options: Options {
//!                 dry_run: true,
//!                 name: "foo".to_owned(),
//!             }
//!         }
//!         .wamp_serialize_application_message()
//!         .unwrap(),
//!         (
//!             List::from_iter([Value::Integer(123), Value::Integer(456)]),
//!             Dictionary::from_iter([
//!                 ("dry_run".to_owned(), Value::Bool(true)),
//!                 ("name".to_owned(), Value::String("foo".to_owned())),
//!             ]),
//!         )
//!     );
//!
//!     // Deserialization.
//!     assert_eq!(
//!         Input::wamp_deserialize_application_message(
//!             List::from_iter([Value::Integer(1), Value::Integer(2)]),
//!             Dictionary::from_iter([
//!                 ("dry_run".to_owned(), Value::Bool(false)),
//!                 ("name".to_owned(), Value::String("bar".to_owned())),
//!             ]),
//!         )
//!         .unwrap(),
//!         Input {
//!             args: Args { a: 1, b: 2 },
//!             options: Options {
//!                 dry_run: false,
//!                 name: "bar".to_owned(),
//!             }
//!         }
//!     );
//! }
//! ```

use battler_wamp_values::{
    Dictionary,
    List,
    WampDeserializeError,
    WampSerializeError,
};
pub use battler_wamprat_message_proc_macro::WampApplicationMessage;

/// Trait for a WAMP application message, which can be passed between applications using pub/sub or
/// RPCs.
pub trait WampApplicationMessage: Sized {
    /// Serializes the object into arguments and keyword arguments.
    fn wamp_serialize_application_message(self) -> Result<(List, Dictionary), WampSerializeError>;

    /// Deserializes the object from arguments and keyword arguments.
    fn wamp_deserialize_application_message(
        arguments: List,
        arguments_keyword: Dictionary,
    ) -> Result<Self, WampDeserializeError>;
}
