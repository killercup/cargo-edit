//! Show and Edit Cargo's Manifest Files
#![recursion_limit = "256"]
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

mod crate_spec;
mod dependency;
mod errors;
mod fetch;
mod manifest;
mod metadata;
mod registry;
mod util;
mod version;

pub use crate::crate_spec::CrateSpec;
pub use crate::dependency::Dependency;
pub use crate::errors::*;
pub use crate::fetch::{
    get_features_from_registry, get_latest_dependency, get_manifest_from_path,
    get_manifest_from_url, update_registry_index,
};
pub use crate::manifest::{find, LocalManifest, Manifest};
pub use crate::metadata::{manifest_from_pkgid, workspace_members};
pub use crate::registry::registry_url;
pub use crate::util::{colorize_stderr, ColorChoice};
pub use crate::version::{upgrade_requirement, VersionExt};
