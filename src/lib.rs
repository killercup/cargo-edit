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

#[cfg_attr(feature = "upgrade", macro_use)]
#[cfg(feature = "upgrade")]
extern crate serde_derive;

mod crate_spec;
#[cfg(feature = "upgrade")]
mod dependency;
mod errors;
#[cfg(feature = "upgrade")]
mod fetch;
#[cfg(feature = "upgrade")]
mod index;
mod manifest;
mod metadata;
#[cfg(feature = "upgrade")]
mod registry;
mod util;
mod version;

pub use crate_spec::CrateSpec;
#[cfg(feature = "upgrade")]
pub use dependency::{Dependency, PathSource, RegistrySource, Source};
pub use errors::*;
#[cfg(feature = "upgrade")]
pub use fetch::{get_compatible_dependency, get_latest_dependency, RustVersion};
#[cfg(feature = "upgrade")]
pub use index::*;
pub use manifest::{find, get_dep_version, set_dep_version, LocalManifest, Manifest};
pub use metadata::manifest_from_pkgid;
#[cfg(feature = "upgrade")]
pub use registry::registry_url;
#[cfg(feature = "upgrade")]
pub use util::{
    colorize_stderr, shell_note, shell_print, shell_write_stderr, shell_write_stdout, Color,
    ColorChoice,
};
pub use util::{shell_status, shell_warn};
pub use version::{upgrade_requirement, VersionExt};
