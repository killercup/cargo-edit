#![allow(clippy::all)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::redundant_clone)]

#[macro_use]
extern crate cargo_test_macro;

mod alt_registry;
mod dry_run;
mod exclude_dep;
mod exclude_renamed;
mod implicit_prerelease;
mod invalid_dep;
mod invalid_flag;
mod invalid_manifest;
mod invalid_workspace_root_manifest;
mod locked;
mod optional_dep;
mod pinned;
mod preserve_precision_major;
mod preserve_precision_minor;
mod preserve_precision_patch;
mod preserves_inline_table;
mod preserves_std_table;
mod single_dep;
mod skip_compatible;
mod specified;
mod to_lockfile;
mod to_version;
mod upgrade_all;
mod upgrade_everything;
mod upgrade_renamed;
mod upgrade_workspace;
mod virtual_manifest;
mod workspace_member_cwd;
mod workspace_member_manifest_path;

fn init_registry() {
    cargo_test_support::registry::init();
    add_registry_packages(false);
}

fn init_alt_registry() {
    cargo_test_support::registry::alt_init();
    add_registry_packages(true);
}

fn add_registry_packages(alt: bool) {
    for name in [
        "my-package",
        "my-package1",
        "my-package2",
        "my-dev-package1",
        "my-dev-package2",
        "my-build-package1",
        "my-build-package2",
        "versioned-package",
        "cargo-list-test-fixture-dependency",
        "unrelated-crate",
        // "Everything"
        "docopt",
        "pad",
        "serde_json",
        "syn",
        "tar",
        "ftp",
        "toml_edit",
        "semver",
        "renamed",
        "assert_cli",
        "tempdir",
        "toml",
        "openssl",
        "rget",
        "geo",
        // Operators
        "default",
        "exact",
        "lessthan",
        "lessorequal",
        "greaterthan",
        "greaterorequal",
        "wildcard",
        "caret",
        "tilde",
        "greaterthan",
        "greaterthan",
    ] {
        cargo_test_support::registry::Package::new(name, "0.1.1-alpha.1+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "0.1.1+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "0.2.0+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "0.2.3+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "0.4.1+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "20.0.0+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "99999.0.0-alpha.1+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "99999.0.0+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "99999.1.0-alpha.1+my-package")
            .alternative(alt)
            .publish();
    }

    cargo_test_support::registry::Package::new("prerelease_only", "0.2.0-alpha.1")
        .alternative(alt)
        .publish();
    cargo_test_support::registry::Package::new("test_breaking", "0.2.0")
        .alternative(alt)
        .publish();
    cargo_test_support::registry::Package::new("test_nonbreaking", "0.1.1")
        .alternative(alt)
        .publish();
    cargo_test_support::registry::Package::new("test_nonbreaking", "0.1.2")
        .alternative(alt)
        .publish();

    // Normalization
    cargo_test_support::registry::Package::new("linked-hash-map", "0.5.4")
        .alternative(alt)
        .feature("clippy", &[])
        .feature("heapsize", &[])
        .feature("heapsize_impl", &[])
        .feature("nightly", &[])
        .feature("serde", &[])
        .feature("serde_impl", &[])
        .feature("serde_test", &[])
        .publish();
    cargo_test_support::registry::Package::new("inflector", "0.11.4")
        .alternative(alt)
        .feature("default", &["heavyweight", "lazy_static", "regex"])
        .feature("heavyweight", &[])
        .feature("lazy_static", &[])
        .feature("regex", &[])
        .feature("unstable", &[])
        .publish();

    cargo_test_support::registry::Package::new("your-face", "99999.0.0+my-package")
        .alternative(alt)
        .feature("nose", &[])
        .feature("mouth", &[])
        .feature("eyes", &[])
        .feature("ears", &[])
        .publish();
}

pub fn cargo_exe() -> std::path::PathBuf {
    snapbox::cmd::cargo_bin("cargo-upgrade")
}

/// Test the cargo command
pub trait CargoCommand {
    fn cargo_ui() -> Self;
}

impl CargoCommand for snapbox::cmd::Command {
    fn cargo_ui() -> Self {
        use cargo_test_support::TestEnv;
        Self::new(cargo_exe())
            .with_assert(cargo_test_support::compare::assert_ui())
            .test_env()
    }
}
