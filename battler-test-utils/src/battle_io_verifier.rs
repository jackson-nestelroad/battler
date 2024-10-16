use std::{
    env,
    fs::File,
    path::Path,
};

use ahash::HashMap;
use battler::{
    battle::{
        PublicCoreBattle,
        Request,
    },
    error::{
        Error,
        WrapResultError,
    },
};
use serde::Deserialize;

use crate::{
    assert_new_logs_eq,
    LogMatch,
};

fn integration_test_expected_io_dir<'s>() -> Result<String, Error> {
    env::var("INTEGRATION_TEST_EXPECTED_IO_DIR")
        .wrap_error_with_message("INTEGRATION_TEST_EXPECTED_IO_DIR is not defined")
}

type ExpectedBattleRequests = HashMap<String, Request>;
type ExpectedBattleLogs = Vec<LogMatch>;

#[derive(Deserialize)]
struct ExpectedBattleIo {
    requests: Vec<ExpectedBattleRequests>,
    logs: Vec<ExpectedBattleLogs>,
}

#[derive(Deserialize)]
pub struct BattleIoVerifier {
    expected: ExpectedBattleIo,
    requests_index: usize,
    logs_index: usize,
}

impl BattleIoVerifier {
    pub fn new(file: &str) -> Result<BattleIoVerifier, Error> {
        let expected = serde_json::from_reader(
            File::open(Path::new(&integration_test_expected_io_dir()?).join(file))
                .wrap_error_with_format(format_args!(
                    "failed to read expected battle io from file {file}"
                ))?,
        )
        .wrap_error_with_format(format_args!(
            "failed to parse expected battle io from {file}"
        ))?;
        Ok(Self {
            expected,
            requests_index: 0,
            logs_index: 0,
        })
    }

    fn next_expected_requests(&mut self) -> Option<&ExpectedBattleRequests> {
        let expected = self.expected.requests.get(self.requests_index);
        if expected.is_some() {
            self.requests_index += 1;
        }
        expected
    }

    #[track_caller]
    pub fn verify_next_request_set(&mut self, battle: &mut PublicCoreBattle) {
        match self.next_expected_requests() {
            None => assert!(false, "battle io verifier has no more expected requests"),
            Some(requests) => pretty_assertions_sorted::assert_eq_sorted!(
                &battle.active_requests().collect::<HashMap<_, _>>(),
                requests
            ),
        }
    }

    fn next_expected_logs(&mut self) -> Option<&ExpectedBattleLogs> {
        let expected = self.expected.logs.get(self.logs_index);
        if expected.is_some() {
            self.logs_index += 1;
        }
        expected
    }

    #[track_caller]
    pub fn verify_new_logs(&mut self, battle: &mut PublicCoreBattle) {
        match self.next_expected_logs() {
            None => assert!(false, "battle io verifier has no more expected logs"),
            Some(logs) => assert_new_logs_eq(battle, logs.as_slice()),
        }
    }
}
