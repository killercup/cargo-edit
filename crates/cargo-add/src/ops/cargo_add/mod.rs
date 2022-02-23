//! Core of cargo-add command

mod crate_spec;
mod dependency;
mod errors;
mod fetch;
mod manifest;
mod registry;
mod util;
mod version;

pub use crate_spec::CrateSpec;
pub use dependency::Dependency;
pub use dependency::GitSource;
pub use dependency::PathSource;
pub use dependency::RegistrySource;
pub use dependency::Source;
pub use errors::*;
pub use fetch::{
    get_features_from_registry, get_latest_dependency, get_manifest_from_path,
    get_manifest_from_url, update_registry_index,
};
pub use manifest::LocalManifest;
pub use registry::registry_url;
pub use util::colorize_stderr;

use manifest::Manifest;
use version::VersionExt;
