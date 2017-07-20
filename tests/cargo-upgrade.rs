extern crate assert_cli;
#[macro_use]
extern crate pretty_assertions;
extern crate tempdir;
extern crate toml;

mod utils;
use utils::{clone_out_test, execute_command, get_toml};

// Verify that an upgraded Cargo.toml matches what we expect.
#[test]
fn upgrade_as_expected() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/upgrade/Cargo.toml.source");

    execute_command(&["upgrade"], &manifest);

    let upgraded = get_toml(&manifest);
    let target = get_toml("tests/fixtures/upgrade/Cargo.toml.target");

    assert_eq!(target, upgraded);
}

#[test]
fn upgrade_all() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Setup manifest with the dependency `versioned-package@0.1.1`
    execute_command(&["add", "versioned-package", "--vers", "0.1.1"], &manifest);

    // Now, upgrade `versioned-package` to the latest version
    execute_command(&["upgrade"], &manifest);

    // Verify that `versioned-package` has been updated successfully.
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["versioned-package"];
    assert_eq!(
        val.as_str().expect("not string"),
        "versioned-package--CURRENT_VERSION_TEST"
    );
}

#[test]
fn upgrade_specified_only() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Setup manifest with the dependencies `versioned-package` and `versioned-package-2`
    execute_command(&["add", "versioned-package", "--vers", "0.1.1"], &manifest);
    execute_command(
        &["add", "versioned-package-2", "--vers", "0.1.1"],
        &manifest,
    );

    // Update `versioned-package` to the latest version
    execute_command(&["upgrade", "-d", "versioned-package"], &manifest);

    // Verify that `versioned-package` was upgraded, but not `versioned-package-2`
    assert_eq!(
        get_toml(&manifest)["dependencies"]["versioned-package"]
            .as_str()
            .expect("not string"),
        "versioned-package--CURRENT_VERSION_TEST"
    );
    assert_eq!(
        get_toml(&manifest)["dependencies"]["versioned-package-2"]
            .as_str()
            .expect("not string"),
        "0.1.1"
    );
}

#[test]
#[should_panic(expected = "not added")]
fn fails_to_upgrade_missing_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Update the non-existent `failure` to the latest version
    execute_command(&["upgrade", "-d", "failure"], &manifest);

    // Verify that `failure` has not been added
    get_toml(&manifest).get("dependencies").expect("not added");
}

#[test]
fn upgrade_optional_dependency() {
    // Set up a Cargo.toml with an optional dependency `test_optional_dependency` verifies that this
    // is correct.
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");
    execute_command(
        &[
            "add",
            "versioned-package",
            "--vers",
            ">=0.1.1",
            "--optional",
        ],
        &manifest,
    );

    // Now, update without including the `optional` flag.
    execute_command(&["upgrade"], &manifest);

    // Dependency present afterwards - correct version, and still optional.
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["versioned-package"];
    assert_eq!(
        val["version"].as_str().expect("not string"),
        "versioned-package--CURRENT_VERSION_TEST"
    );
    assert_eq!(
        val["optional"].as_bool().expect("optional not a bool"),
        true
    );
}

#[test]
fn unknown_flags() {
    assert_cli::Assert::command(&["target/debug/cargo-upgrade", "upgrade", "foo", "--flag"])
        .fails_with(1)
        .prints_error_exactly(
            r"Unknown flag: '--flag'

Usage:
    cargo upgrade [--dependency <dep>...] [--manifest-path <path>]
    cargo upgrade (-h | --help)
    cargo upgrade (-V | --version)",
        )
        .unwrap();
}
