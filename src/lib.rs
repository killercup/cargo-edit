//! Show and Edit Cargo's Manifest Files

#![cfg_attr(test, allow(dead_code))]
#![deny(missing_docs)]

#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

#[macro_use]
extern crate quick_error;
extern crate toml;

mod manifest;

pub use manifest::{Dependency, Manifest};
