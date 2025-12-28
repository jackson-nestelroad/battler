#![feature(closure_lifetime_binder)]

mod ai;
pub mod choice;
mod client;
pub mod gemini;
pub mod random;
pub mod trainer;

pub use ai::*;
pub use client::*;
