# serde-struct-tuple

[![Latest Version]][crates.io]

[Latest Version]: https://img.shields.io/crates/v/serde-struct-tuple.svg
[crates.io]: https://crates.io/crates/serde-struct-tuple

**serde-struct-tuple** is a utility crate, built initially for [`battler-wamp`](https://crates.io/crates/battler-wamp). It provides procedural macros to automatically derive [`serde`](https://serde.rs/)'s `Serialize` and `Deserialize` traits for struct types that should be encoded as a tuple (list) of its fields.
