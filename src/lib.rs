//! Show and Edit Cargo's Manifest Files

#![cfg_attr(test, allow(dead_code))]
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts, trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces)]

#[macro_use]
extern crate quick_error;
extern crate toml;

mod manifest;
mod dependency;

pub use dependency::Dependency;
pub use manifest::Manifest;
