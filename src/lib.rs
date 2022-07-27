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

pub use crate_spec::CrateSpec;
pub use dependency::Dependency;
pub use dependency::PathSource;
pub use dependency::RegistrySource;
pub use dependency::Source;
pub use errors::*;
pub use fetch::{get_latest_dependency, update_registry_index};
pub use manifest::{find, get_dep_version, set_dep_version, LocalManifest, Manifest};
pub use metadata::{manifest_from_pkgid, resolve_manifests, workspace_members};
pub use registry::registry_url;
pub use util::{
    colorize_stderr, shell_note, shell_print, shell_status, shell_warn, shell_write_stderr, Color,
    ColorChoice,
};
pub use version::{upgrade_requirement, VersionExt};
