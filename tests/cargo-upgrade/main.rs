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
mod lockfile;
mod optional_dep;
mod pinned;
mod preserve_op;
mod preserve_precision_major;
mod preserve_precision_minor;
mod preserve_precision_patch;
mod preserves_inline_table;
mod preserves_std_table;
mod single_dep;
mod skip_compatible;
mod specified;
mod to_version;
mod upgrade_all;
mod upgrade_everything;
mod upgrade_renamed;
mod upgrade_verbose;
mod upgrade_workspace;
mod virtual_manifest;
mod workspace_inheritance;
mod workspace_member_cwd;
mod workspace_member_manifest_path;

fn init_registry() {
    cargo_test_support::registry::init();
    add_fake_registry_packages(false);
}

fn init_alt_registry() {
    cargo_test_support::registry::alt_init();
    add_fake_registry_packages(true);
}

fn add_fake_registry_packages(alt: bool) {
    for name in [
        "my-package",
        "my-package1",
        "my-package2",
        "unrelated-crate",
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
}

fn add_breaking_registry_packages(alt: bool) {
    cargo_test_support::registry::Package::new("test_breaking", "0.2.0")
        .alternative(alt)
        .publish();
    cargo_test_support::registry::Package::new("test_nonbreaking", "0.1.1")
        .alternative(alt)
        .publish();
    cargo_test_support::registry::Package::new("test_nonbreaking", "0.1.2")
        .alternative(alt)
        .publish();
}

fn add_everything_registry_packages(alt: bool) {
    for name in [
        // "Everything"
        "docopt",
        "pad",
        "serde",
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
}

fn add_git_registry_packages() {
    cargo_test_support::git::new("serde", |project| {
        project
            .file(
                "Cargo.toml",
                &cargo_test_support::basic_manifest("serde", "1.0.99999"),
            )
            .file("src/lib.rs", r#"pub fn hello() { println!("it works"); }"#)
    });
}

fn add_op_registry_packages(alt: bool) {
    for name in [
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
