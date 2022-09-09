#![allow(clippy::all)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::redundant_clone)]

mod avoid_empty_tables;
mod build;
mod dev;
mod dry_run;
mod existing;
mod invalid_arg;
mod invalid_dep;
mod invalid_package;
mod invalid_package_multiple;
mod invalid_section;
mod invalid_section_dep;
mod invalid_target;
mod invalid_target_dep;
mod multiple_deps;
mod multiple_dev;
mod no_arg;
mod offline;
mod optional_dep_feature;
mod optional_feature;
mod package;
mod target;
mod target_build;
mod target_dev;
mod update_lock_file;

fn init_registry() {
    cargo_test_support::registry::init();
    add_registry_packages(false);
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
        "clippy",
        "dbus",
        "docopt",
        "ncurses",
        "pad",
        "regex",
        "rustc-serialize",
        "toml",
        "versioned-package",
        "cargo-list-test-fixture-dependency",
        "unrelateed-crate",
    ] {
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
        cargo_test_support::registry::Package::new(name, "0.6.2+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "0.9.9+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "1.0.90+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "20.0.0+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "99999.0.0+my-package")
            .alternative(alt)
            .publish();
        cargo_test_support::registry::Package::new(name, "99999.0.0-alpha.1+my-package")
            .alternative(alt)
            .publish();
    }

    for name in ["semver", "serde"] {
        cargo_test_support::registry::Package::new(name, "0.1.1")
            .alternative(alt)
            .feature("std", &[])
            .publish();
        cargo_test_support::registry::Package::new(name, "0.9.0")
            .alternative(alt)
            .feature("std", &[])
            .publish();
        cargo_test_support::registry::Package::new(name, "1.0.90")
            .alternative(alt)
            .feature("std", &[])
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
    snapbox::cmd::cargo_bin("cargo-remove")
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
