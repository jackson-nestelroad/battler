#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "typescript")]
extern crate std;

mod abilities;
mod clauses;
mod common;
mod conditions;
mod datastore;
mod items;
mod mons;
mod moves;

#[cfg(test)]
pub mod test_util;

pub use abilities::*;
pub use clauses::*;
pub use common::*;
pub use conditions::*;
pub use datastore::*;
pub use items::*;
pub use mons::*;
pub use moves::*;
