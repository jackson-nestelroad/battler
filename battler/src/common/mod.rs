mod captures;
mod error;
mod fraction;
mod hash;
mod id;
mod lookup_result;
mod maybe_owned;
mod reference;
mod strings;
mod test_util;

pub use captures::Captures;
#[cfg(test)]
pub use error::{
    assert_error_message,
    assert_error_message_contains,
};
pub use error::{
    Error,
    WrapResultError,
};
pub use fraction::{
    Fraction,
    FractionInteger,
};
pub use hash::{
    FastHashMap,
    FastHashSet,
};
pub use id::{
    Id,
    Identifiable,
};
pub use lookup_result::LookupResult;
pub use maybe_owned::{
    MaybeOwned,
    MaybeOwnedMut,
};
pub use reference::UnsafelyDetachBorrow;
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
