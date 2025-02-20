//! Show and Edit Cargo's Manifest Files
//!
//! # Semver Compatibility
//!
//! cargo-edit's versioning tracks compatibility for the binaries, not the API.  We upload to
//! crates.io to distribute the binary.  If using this as a library, be sure to pin the version
//! with a `=` version requirement operator.  Note though that our goal is for `cargo-edit` to go
//! away as we move things into cargo.
#![recursion_limit = "256"]
#![cfg_attr(test, allow(dead_code))]
#![warn(
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
mod index;
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
pub use fetch::{RustVersion, get_compatible_dependency, get_latest_dependency};
pub use index::*;
pub use manifest::{LocalManifest, Manifest, find, get_dep_version, set_dep_version};
pub use metadata::manifest_from_pkgid;
pub use registry::registry_url;
pub use util::{
    Color, ColorChoice, colorize_stderr, shell_note, shell_print, shell_status, shell_warn,
    shell_write_stderr, shell_write_stdout,
};
pub use version::{VersionExt, upgrade_requirement};
