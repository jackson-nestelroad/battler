# serde-struct-tuple-proc-macro

[![Latest Version]][crates.io]

[Latest Version]: https://img.shields.io/crates/v/serde-struct-tuple-proc-macro.svg
[crates.io]: https://crates.io/crates/serde-struct-tuple-proc-macro

**serde-struct-tuple-proc-macro** is a utility crate for [`serde-struct-tuple`](https://crates.io/crates/serde-struct-tuple). It provides procedural macros to automatically derive [`serde`](https://serde.rs/)'s `Serialize` and `Deserialize` traits for struct types that should be encoded as a tuple (list) of its fields.
