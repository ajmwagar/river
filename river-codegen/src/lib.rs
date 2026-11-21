//! Compile River's frozen numeric core to WebAssembly.
//!
//! This is the first backend, intentionally narrow: `i64` values, `defun`,
//! `let`, `if`, `cond`, arithmetic, comparisons, `and`/`or`/`not`, and calls
//! between numeric functions. Lists, strings, records, JSON, and host calls are
//! rejected until the value representation is designed.

pub mod core;
pub mod wasm;

pub use core::{parse, Expr, Function, Program};
pub use wasm::compile;
