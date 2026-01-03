use std::{
    env,
    fs::File,
    path::Path,
};

use ahash::HashMap;
use anyhow::Result;
use battler::WrapResultError;
use serde::de::DeserializeOwned;

fn test_case_dir<'s>() -> Result<String> {
    Ok(format!("{}/battler/test_cases", env::var("CRATE_ROOT")?))
}

pub fn read_test_cases<T: DeserializeOwned>(file: &str) -> Result<HashMap<String, T>> {
    serde_json::from_reader(
        File::open(Path::new(&test_case_dir()?).join(file))
            .wrap_error_with_format(format_args!("failed to read test cases from {file}"))?,
    )
    .wrap_error_with_format(format_args!("failed to parse test cases from {file}"))
}
