mod utils;

use crate::utils::{
    clone_out_test, copy_workspace_test, execute_command, execute_command_for_pkg, get_toml,
};
use assert_cmd::Command;

#[test]
fn remove_existing_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    let toml = get_toml(&manifest);
    assert!(!toml["dependencies"]["docopt"].is_none());
    execute_command(&["rm", "docopt"], &manifest);
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"]["docopt"].is_none());

    // Activated features should not be changed:
    assert_eq!(toml["features"]["std"].as_array().unwrap().len(), 2);
}

#[test]
fn remove_existing_dependency_does_not_create_empty_tables() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.no_features.sample");

    let toml = get_toml(&manifest);
    assert!(toml["features"].is_none());
    assert!(toml["build-dependencies"].is_none());
    execute_command(&["rm", "docopt"], &manifest);
    let toml = get_toml(&manifest);
    assert!(toml["features"].is_none());
    assert!(toml["build-dependencies"].is_none());
}

#[test]
fn remove_existing_optional_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    let toml = get_toml(&manifest);
    assert!(!toml["dependencies"]["clippy"].is_none());
    assert_eq!(toml["features"]["annoy"].as_array().unwrap().len(), 1);

    execute_command(&["rm", "clippy"], &manifest);
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"]["clippy"].is_none());
    // Also check that exact match feature activations are removed:
    assert_eq!(toml["features"]["annoy"].as_array().unwrap().len(), 0);
}

#[test]
fn remove_multiple_existing_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    let toml = get_toml(&manifest);
    assert!(!toml["dependencies"]["docopt"].is_none());
    assert!(!toml["dependencies"]["semver"].is_none());
    execute_command(&["rm", "docopt", "semver"], &manifest);
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"]["docopt"].is_none());
    assert!(toml["dependencies"]["semver"].is_none());

    // "semver/std" activated feature should NOT have been dropped as
    // there's still a build-dep on the crate:
    assert_eq!(toml["features"]["std"].as_array().unwrap().len(), 2);

    // Let's remove the last semver dependency and expect the associated feature to be dropped.
    execute_command(&["rm", "--build", "semver"], &manifest);
    let toml = get_toml(&manifest);
    assert_eq!(toml["features"]["std"].as_array().unwrap().len(), 1);
}

#[test]
fn remove_existing_dependency_from_specific_section() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    // Test removing dev dependency.
    let toml = get_toml(&manifest);
    assert!(!toml["dev-dependencies"]["regex"].is_none());
    execute_command(&["rm", "--dev", "regex"], &manifest);
    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"]["regex"].is_none());

    // Test removing build dependency.
    let toml = get_toml(&manifest);
    assert!(!toml["build-dependencies"]["semver"].is_none());
    execute_command(&["rm", "--build", "semver"], &manifest);
    let toml = get_toml(&manifest);
    assert!(toml["build-dependencies"].is_none());
}

#[test]
fn remove_multiple_existing_dependencies_from_specific_section() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    // Test removing dev dependency.
    let toml = get_toml(&manifest);
    assert!(!toml["dev-dependencies"]["regex"].is_none());
    assert!(!toml["dev-dependencies"]["serde"].is_none());
    execute_command(&["rm", "--dev", "regex", "serde"], &manifest);
    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"].is_none());
}

#[test]
fn remove_section_after_removed_last_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    let toml = get_toml(&manifest);
    assert!(!toml["dev-dependencies"]["regex"].is_none());
    assert_eq!(toml["dev-dependencies"].as_table().unwrap().len(), 2);

    execute_command(&["rm", "--dev", "regex", "serde"], &manifest);

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

    Command::cargo_bin("cargo-rm")
        .expect("can find bin")
        .args(&[
        "rm",
        "invalid_dependency_name",
        &format!("--manifest-path={}", manifest),
    ])
    .assert()
    .code(1)
        .stderr(predicates::str::contains(
        "Command failed due to unhandled error: The dependency `invalid_dependency_name` could \
         not be found in `dependencies`.",
    ));
}

#[test]
fn invalid_section() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    execute_command(&["rm", "semver", "--build"], &manifest);
    Command::cargo_bin("cargo-rm")
        .expect("can find bin")
        .args(&[
            "rm",
            "semver",
            "--build",
            &format!("--manifest-path={}", manifest),
        ])
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "The table `build-dependencies` could not be found.",
        ));
}

#[test]
fn invalid_dependency_in_section() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    Command::cargo_bin("cargo-rm")
        .expect("can find bin")
        .args(&[
            "rm",
            "semver",
            "regex",
            "--dev",
            &format!("--manifest-path={}", manifest),
        ])
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "Command failed due to unhandled error: The dependency `semver` could not be found in \
         `dev-dependencies`.",
        ));
}

#[test]
fn no_argument() {
    Command::cargo_bin("cargo-rm")
        .expect("can find bin")
        .args(&["rm"])
        .assert()
        .code(1)
        .stderr(
            r"error: The following required arguments were not provided:
    <crates>...

USAGE:
    cargo rm [FLAGS] [OPTIONS] <crates>...

For more information try --help
",
        );
}

#[test]
fn unknown_flags() {
    Command::cargo_bin("cargo-rm")
        .expect("can find bin")
        .args(&["rm", "foo", "--flag"])
        .assert()
        .code(1)
        .stderr(
            r"error: Found argument '--flag' which wasn't expected, or isn't valid in this context

USAGE:
    cargo rm [FLAGS] [OPTIONS] <crates>...

For more information try --help
",
        );
}

#[test]
fn rm_prints_message() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    Command::cargo_bin("cargo-rm")
        .expect("can find bin")
        .args(&["rm", "semver", &format!("--manifest-path={}", manifest)])
        .assert()
        .success()
        .stdout("    Removing semver from dependencies\n");
}

#[test]
fn rm_prints_messages_for_multiple() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    Command::cargo_bin("cargo-rm")
        .expect("can find bin")
        .args(&[
            "rm",
            "semver",
            "docopt",
            &format!("--manifest-path={}", manifest),
        ])
        .assert()
        .success()
        .stdout("    Removing semver from dependencies\n    Removing docopt from dependencies\n");
}

#[test]
fn rm_dependency_from_workspace_member() {
    let (tmpdir, _root_manifest, workspace_manifests) = copy_workspace_test();
    execute_command_for_pkg(&["rm", "libc"], "one", &tmpdir);

    let one = workspace_manifests
        .iter()
        .map(|manifest| get_toml(manifest))
        .find(|manifest| manifest["package"]["name"].as_str() == Some("one"))
        .expect("Couldn't find workspace member `one'");

    assert!(one["dependencies"]["libc"].as_str().is_none());
}
