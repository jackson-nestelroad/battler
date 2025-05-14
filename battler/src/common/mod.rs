mod captures;
mod lru;
mod maybe_owned;
mod reference;
mod strings;
mod test_util;

pub use captures::Captures;
pub use lru::{
    Iter,
    IterMut,
    LruCache,
};
pub use maybe_owned::{
    MaybeOwned,
    MaybeOwnedMut,
};
pub use reference::{
    UnsafelyDetachBorrow,
    UnsafelyDetachBorrowMut,
};
pub use strings::split_once_optional;
#[cfg(test)]
pub use test_util::{
    read_test_cases,
    read_test_json,
    test_deserialization,
    test_serialization,
    test_string_deserialization,
    test_string_serialization,
};
