//! Show and Edit Cargo's Manifest Files
#![recursion_limit = "128"]
#![cfg_attr(test, allow(dead_code))]
#![warn(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;

mod crate_name;
mod dependency;
mod errors;
mod fetch;
mod manifest;
mod registry;

pub use crate::crate_name::CrateName;
pub use crate::dependency::Dependency;
pub use crate::errors::*;
pub use crate::fetch::{
    get_crate_name_from_github, get_crate_name_from_gitlab, get_crate_name_from_path,
    get_latest_dependency, update_registry_index,
};
pub use crate::manifest::{find, LocalManifest, Manifest};
pub use crate::registry::registry_url;
