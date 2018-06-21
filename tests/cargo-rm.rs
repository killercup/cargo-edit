extern crate assert_cli;

mod utils;
use utils::{clone_out_test, execute_command, get_toml};

#[test]
fn remove_existing_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    let toml = get_toml(&manifest);
    assert!(!toml["dependencies"]["docopt"].is_none());
    execute_command(&["rm", "docopt"], &manifest);
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"]["docopt"].is_none());
}

#[test]
fn remove_existing_dependency_from_specific_section() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    // Test removing dev dependency.
    let toml = get_toml(&manifest);
    assert!(!toml["dev-dependencies"]["regex"].is_none());
    execute_command(&["rm", "--dev", "regex"], &manifest);
    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"].is_none());

    // Test removing build dependency.
    let toml = get_toml(&manifest);
    assert!(!toml["build-dependencies"]["semver"].is_none());
    execute_command(&["rm", "--build", "semver"], &manifest);
    let toml = get_toml(&manifest);
    assert!(toml["build-dependencies"].is_none());
}

#[test]
fn remove_section_after_removed_last_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    let toml = get_toml(&manifest);
    assert!(!toml["dev-dependencies"]["regex"].is_none());
    assert_eq!(toml["dev-dependencies"].as_table().unwrap().len(), 1);

    execute_command(&["rm", "--dev", "regex"], &manifest);

    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"].is_none());
}

// https://github.com/killercup/cargo-edit/issues/32
#[test]
fn issue_32() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    let toml = get_toml(&manifest);
    assert!(toml["dependencies"]["foo"].is_none());

    execute_command(&["add", "foo@1.0"], &manifest);
    execute_command(&["add", "bar@1.0.7"], &manifest);

    let toml = get_toml(&manifest);
    assert!(!toml["dependencies"]["foo"].is_none());
    assert!(!toml["dependencies"]["bar"].is_none());

    execute_command(&["rm", "foo"], &manifest);
    execute_command(&["rm", "bar"], &manifest);

    let toml = get_toml(&manifest);
    assert!(toml["dependencies"]["foo"].is_none());
    assert!(toml["dependencies"]["bar"].is_none());
}

#[test]
fn invalid_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    assert_cli::Assert::command(&[
        "target/debug/cargo-rm",
        "rm",
        "invalid_dependency_name",
        &format!("--manifest-path={}", manifest),
    ]).fails_with(1)
        .and()
        .stderr().is(
            "Command failed due to unhandled error: The dependency `invalid_dependency_name` could \
             not be found in `dependencies`.",
        )
        .unwrap();
}

#[test]
fn invalid_section() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    execute_command(&["rm", "semver", "--build"], &manifest);
    assert_cli::Assert::command(&[
        "target/debug/cargo-rm",
        "rm",
        "semver",
        "--build",
        &format!("--manifest-path={}", manifest),
    ]).fails_with(1)
        .and()
        .stderr()
        .is(
            "Command failed due to unhandled error: The table `build-dependencies` could not be \
             found.",
        )
        .unwrap();
}

#[test]
fn invalid_dependency_in_section() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    assert_cli::Assert::command(&[
        "target/debug/cargo-rm",
        "rm",
        "semver",
        "--dev",
        &format!("--manifest-path={}", manifest),
    ]).fails_with(1)
        .and()
        .stderr()
        .is(
            "Command failed due to unhandled error: The dependency `semver` could not be found in \
             `dev-dependencies`.",
        )
        .unwrap();
}

#[test]
fn no_argument() {
    assert_cli::Assert::command(&["target/debug/cargo-rm", "rm"])
        .fails_with(1)
        .and()
        .stderr()
        .is(r"Invalid arguments.

Usage:
    cargo rm <crate> [--dev|--build] [options]
    cargo rm (-h|--help)
    cargo rm --version")
        .unwrap();
}

#[test]
fn unknown_flags() {
    assert_cli::Assert::command(&["target/debug/cargo-rm", "rm", "foo", "--flag"])
        .fails_with(1)
        .and()
        .stderr()
        .is(r"Unknown flag: '--flag'

Usage:
    cargo rm <crate> [--dev|--build] [options]
    cargo rm (-h|--help)
    cargo rm --version")
        .unwrap();
}

#[test]
fn rm_prints_message() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    assert_cli::Assert::command(&[
        "target/debug/cargo-rm",
        "rm",
        "semver",
        &format!("--manifest-path={}", manifest),
    ]).succeeds()
        .and()
        .stdout()
        .is("Removing semver from dependencies")
        .unwrap();
}
