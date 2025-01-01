# serde-struct-tuple-enum-proc-macro

[![Latest Version]][crates.io]

[Latest Version]: https://img.shields.io/crates/v/serde-struct-tuple-enum-proc-macro.svg
[crates.io]: https://crates.io/crates/serde-struct-tuple-enum-proc-macro

**serde-struct-tuple-proc-macro-enum** is a utility crate for [`serde-struct-tuple-enum`](https://crates.io/crates/serde-struct-tuple-enum). It provides procedural macros to automatically derive [`serde`](https://serde.rs/)'s `Serialize` and `Deserialize` traits for enum types, where each variant of the enum if a struct that is encoded as a tuple of its fields (specifically using [`serde-struct-tuple`](https://crates.io/crates/serde-struct-tuple)).
