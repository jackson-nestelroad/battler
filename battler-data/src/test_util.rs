#![cfg(test)]

use alloc::format;
use core::fmt::{
    Debug,
    Display,
};

use serde::{
    Serialize,
    de::DeserializeOwned,
};

#[track_caller]
pub fn test_deserialization<'a, T>(s: &str, expected: T)
where
    T: Debug + PartialEq + DeserializeOwned,
{
    let got = serde_json::from_str::<T>(s).unwrap();
    assert_eq!(got, expected);
}

#[track_caller]
pub fn test_serialization<'a, T, S>(v: T, expected: S)
where
    T: Debug + PartialEq + Serialize + DeserializeOwned,
    S: Display,
{
    let expected_str = format!("{expected}");
    let got = serde_json::to_string(&v).unwrap();
    assert_eq!(got, expected_str);
    test_deserialization(&got, v);
}

#[track_caller]
pub fn test_string_deserialization<'a, T>(s: &str, expected: T)
where
    T: Debug + PartialEq + DeserializeOwned,
{
    test_deserialization(&format!("\"{s}\""), expected)
}

#[track_caller]
pub fn test_string_serialization<'a, T>(v: T, expected: &str)
where
    T: Debug + PartialEq + Serialize + DeserializeOwned,
{
    test_serialization(v, format!("\"{expected}\""))
}
