//! WIT to S-expression bindings.
//!
//! The generator emits type definitions, capability metadata for
//! introspection, and plain `defun` stubs that call one generic host function:
//!
//! ```lisp
//! (defun soil:records/scan (collection cursor limit)
//!   (host-call "soil:records" "scan" (list collection cursor limit)))
//! ```
//!
//! The host owns grants and dispatch. This crate does not contain a runtime
//! catalog or a per-capability host adapter.

pub mod generate;

pub use generate::{generate, Generated};
