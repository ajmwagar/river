//! River's reader and first evaluator.
//!
//! The S-expression reader is the IR. Domain policy — devices, signals,
//! profiles, presence — belongs in `.river` programs, not in Rust types here.

pub mod sexp;

pub use sexp::Value;
