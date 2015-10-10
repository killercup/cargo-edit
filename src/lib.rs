//! Show and Edit Cargo's Manifest Files

#![cfg_attr(test, allow(dead_code))]
#![deny(missing_docs)]

#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate semver;
extern crate toml;
extern crate pad;
#[cfg(test)] extern crate rustc_serialize;

#[macro_use] mod utils;
mod manifest;
mod list;
mod list_error;
mod tree;
#[cfg(test)] mod args;
#[cfg(test)] mod manifest_test;

pub use manifest::{Dependency, Manifest};
pub use list::list_section;
pub use tree::parse_lock_file as list_tree;
