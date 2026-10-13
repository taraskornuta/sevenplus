//! SevenPlus calculator engine: a pure state machine with no UI or OS dependency
//! (ENG-008). Feed it [`Key`]s with [`Calculator::press`] and render the returned
//! [`DisplayState`] with `calc-format`.

#![forbid(unsafe_code)]

pub mod calculator;
pub mod datecalc;
pub mod keys;
pub mod number;
pub mod units;
pub mod worksheets;

pub use calculator::*;
pub use number::{CalcError, Num};
