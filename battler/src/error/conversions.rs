use std::fmt::Display;

use anyhow::Error;
use zone_alloc::BorrowError;

use crate::error::{
    borrow_failed_error,
    not_found_error,
};

/// Trait for implementing manual conversions to [`Error`].
pub trait ConvertError {
    #[track_caller]
    fn convert_error(self) -> Error;

    #[track_caller]
    fn convert_error_with_message<M>(self, message: M) -> Error
    where
        M: Display;
}

impl ConvertError for BorrowError {
    #[track_caller]
    fn convert_error(self) -> Error {
        self.convert_error_with_message("[unknown target]")
    }

    #[track_caller]
    fn convert_error_with_message<M>(self, message: M) -> Error
    where
        M: Display,
    {
        match self {
            Self::OutOfBounds => not_found_error(message),
            Self::AlreadyBorrowed | Self::AlreadyBorrowedMutably => {
                borrow_failed_error(self, message)
            }
        }
    }
}
