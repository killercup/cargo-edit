//! Core of cargo-add command

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
pub use errors::*;
pub use fetch::{
    get_features_from_registry, get_latest_dependency, get_manifest_from_path,
    get_manifest_from_url, update_registry_index,
};
pub use manifest::{find, LocalManifest, Manifest};
pub use metadata::{manifest_from_pkgid, workspace_members};
pub use registry::registry_url;
pub use util::{colorize_stderr, ColorChoice};
pub use version::{upgrade_requirement, VersionExt};
