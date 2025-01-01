# serde-struct-tuple-enum

[![Latest Version]][crates.io]

[Latest Version]: https://img.shields.io/crates/v/serde-struct-tuple-enum.svg
[crates.io]: https://crates.io/crates/serde-struct-tuple-enum

**serde-struct-tuple-enum** is a utility crate, built initially for [`battler-wamp`](https://crates.io/crates/battler-wamp). It provides procedural macros to automatically derive [`serde`](https://serde.rs/)'s `Serialize` and `Deserialize` traits for enum types, where each variant of the enum if a struct that is encoded as a tuple of its fields (specifically using [`serde-struct-tuple`](https://crates.io/crates/serde-struct-tuple)).

[`battler-wamp`](https://crates.io/crates/battler-wamp) uses this macro for all WAMP messages, since WAMP messages are encoded as a list, where the first element determines the message variant.
