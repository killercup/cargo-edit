mod utils;
use crate::utils::{
    clone_out_test, copy_workspace_test, execute_command, execute_command_for_pkg,
    get_command_path, get_toml,
};

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
fn remove_multiple_existing_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    let toml = get_toml(&manifest);
    assert!(!toml["dependencies"]["docopt"].is_none());
    assert!(!toml["dependencies"]["semver"].is_none());
    execute_command(&["rm", "docopt", "semver"], &manifest);
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"]["docopt"].is_none());
    assert!(toml["dependencies"]["semver"].is_none());
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

    assert_cli::Assert::command(&[
        get_command_path("rm").as_str(),
        "rm",
        "invalid_dependency_name",
        &format!("--manifest-path={}", manifest),
    ])
    .fails_with(1)
    .and()
    .stderr()
    .contains(
        "Command failed due to unhandled error: The resource `invalid_dependency_name` could \
         not be found in `dependencies`.",
    )
    .unwrap();
}

#[test]
fn invalid_section() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    execute_command(&["rm", "semver", "--build"], &manifest);
    assert_cli::Assert::command(&[
        get_command_path("rm").as_str(),
        "rm",
        "semver",
        "--build",
        &format!("--manifest-path={}", manifest),
    ])
    .fails_with(1)
    .and()
    .stderr()
    .contains(
        "Command failed due to unhandled error: The resource `build-dependencies` could not be \
         found.",
    )
    .unwrap();
}

#[test]
fn invalid_dependency_in_section() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    assert_cli::Assert::command(&[
        get_command_path("rm").as_str(),
        "rm",
        "semver",
        "regex",
        "--dev",
        &format!("--manifest-path={}", manifest),
    ])
    .fails_with(1)
    .and()
    .stderr()
    .contains(
        "Command failed due to unhandled error: The resource `semver` could not be found in \
         `dev-dependencies`.",
    )
    .unwrap();
}

#[test]
fn no_argument() {
    assert_cli::Assert::command(&[get_command_path("rm").as_str(), "rm"])
        .fails_with(1)
        .and()
        .stderr()
        .is(r"error: The following required arguments were not provided:
    <crates>...

USAGE:
    cargo rm [FLAGS] [OPTIONS] <crates>...

For more information try --help")
        .unwrap();
}

#[test]
fn unknown_flags() {
    assert_cli::Assert::command(&[get_command_path("rm").as_str(), "rm", "foo", "--flag"])
        .fails_with(1)
        .and()
        .stderr()
        .is(
            r"error: Found argument '--flag' which wasn't expected, or isn't valid in this context

USAGE:
    cargo rm [FLAGS] [OPTIONS] <crates>...

For more information try --help",
        )
        .unwrap();
}

#[test]
fn rm_prints_message() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    assert_cli::Assert::command(&[
        get_command_path("rm").as_str(),
        "rm",
        "semver",
        &format!("--manifest-path={}", manifest),
    ])
    .succeeds()
    .and()
    .stdout()
    .is("Removing semver from dependencies")
    .unwrap();
}

#[test]
fn rm_prints_messages_for_multiple() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml.sample");

    assert_cli::Assert::command(&[
        get_command_path("rm").as_str(),
        "rm",
        "semver",
        "docopt",
        &format!("--manifest-path={}", manifest),
    ])
    .succeeds()
    .and()
    .stdout()
    .is("Removing semver from dependencies\n    Removing docopt from dependencies")
    .unwrap();
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
