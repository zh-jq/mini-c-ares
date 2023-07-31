//! Minimal Low-level bindings for the c-ares library.
//!
//! In most cases this crate should not be used directly.  The mini-c-ares crate provides a safe wrapper
//! and should be preferred wherever possible.

mod constants;
mod ffi;

pub use crate::constants::*;
pub use crate::ffi::*;
