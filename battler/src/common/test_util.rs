#![cfg(test)]

use std::{
    env,
    fmt::{
        Debug,
        Display,
    },
    fs::File,
    path::Path,
};

use ahash::HashMap;
use anyhow::Result;
use serde::{
    Serialize,
    de::DeserializeOwned,
};

use crate::error::WrapResultError;

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

fn test_case_dir<'s>() -> Result<String> {
    env::var("TEST_CASE_DIR").wrap_error_with_message("TEST_CASE_DIR is not defined")
}

pub fn read_test_json<T: DeserializeOwned>(file: &str) -> Result<T> {
    serde_json::from_reader(
        File::open(Path::new(&test_case_dir()?).join(""))
            .wrap_error_with_format(format_args!("failed to read from {file}"))?,
    )
    .wrap_error_with_format(format_args!("failed to read object from {file}"))
}

pub fn read_test_cases<T: DeserializeOwned>(file: &str) -> Result<HashMap<String, T>> {
    serde_json::from_reader(
        File::open(Path::new(&test_case_dir()?).join(file))
            .wrap_error_with_format(format_args!("failed to read test cases from {file}"))?,
    )
    .wrap_error_with_format(format_args!("failed to parse test cases from {file}"))
}
