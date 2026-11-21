//! River's first evaluator.
//!
//! The evaluator is deliberately small: it runs the wearable-presence
//! profiles in Instinct before River has a compiler. The S-expression reader
//! is the IR; the forms are the language; the host is still Rust.

pub mod program;
pub mod sexp;

pub use program::{
    load_program, Device, EvalError, Fusion, Link, PersonState, Profile, Program, Quality, Signal,
    WearableState,
};
pub use sexp::Value;
