extern crate assert_cli;
#[macro_use]
extern crate pretty_assertions;
extern crate tempdir;
extern crate toml_edit;

use std::fs;

mod utils;
use utils::{clone_out_test, execute_command, get_toml};

/// Helper function that copies the workspace test into a temporary directory.
pub fn copy_workspace_test() -> (tempdir::TempDir, String, Vec<String>) {
    // Create a temporary directory and copy in the root manifest, the dummy rust file, and
    // workspace member manifests.
    let tmpdir = tempdir::TempDir::new("upgrade_workspace")
        .expect("failed to construct temporary directory");

    let (root_manifest_path, workspace_manifest_paths) = {
        // Helper to copy in files to the temporary workspace. The standard library doesn't have a
        // good equivalent of `cp -r`, hence this oddity.
        let copy_in = |dir, file| {
            let file_path = tmpdir
                .path()
                .join(dir)
                .join(file)
                .to_str()
                .unwrap()
                .to_string();

            fs::create_dir_all(tmpdir.path().join(dir)).unwrap();

            fs::copy(
                format!("tests/fixtures/workspace/{}/{}", dir, file),
                &file_path,
            ).unwrap_or_else(|err| panic!("could not copy test file: {}", err));

            file_path
        };

        let root_manifest_path = copy_in(".", "Cargo.toml");
        copy_in(".", "dummy.rs");

        let workspace_manifest_paths = ["one", "two", "implicit/three", "explicit/four"]
            .iter()
            .map(|member| copy_in(member, "Cargo.toml"))
            .collect::<Vec<_>>();

        (root_manifest_path, workspace_manifest_paths)
    };

    (
        tmpdir,
        root_manifest_path,
        workspace_manifest_paths.to_owned(),
    )
}

// Verify that an upgraded Cargo.toml matches what we expect.
#[test]
fn upgrade_as_expected() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/upgrade/Cargo.toml.source");

    execute_command(&["upgrade"], &manifest);

    let upgraded = get_toml(&manifest);
    let target = get_toml("tests/fixtures/upgrade/Cargo.toml.target");

    assert_eq!(target.to_string(), upgraded.to_string());
}

#[test]
fn upgrade_all() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Setup manifest with the dependency `docopt@0.8.0`
    execute_command(&["add", "docopt", "--vers", "0.8.0"], &manifest);

    // Now, upgrade `docopt` to the latest version
    execute_command(&["upgrade"], &manifest);

    // Verify that `docopt` has been updated successfully.
    assert_eq!(
        get_toml(&manifest)["dependencies"]["docopt"].as_str(),
        Some("docopt--CURRENT_VERSION_TEST")
    );
}

#[test]
fn upgrade_all_allow_prerelease() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Setup manifest with `docopt`
    execute_command(&["add", "docopt", "--vers", "0.8"], &manifest);

    // Now, upgrade `docopt` to the latest version
    execute_command(&["upgrade", "--allow-prerelease"], &manifest);

    // Verify that `docopt` has been updated successfully.
    assert_eq!(
        get_toml(&manifest)["dependencies"]["docopt"].as_str(),
        Some("docopt--PRERELEASE_VERSION_TEST")
    );
}

#[test]
fn upgrade_all_dry_run() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Setup manifest with the dependency `docopt@0.8`
    execute_command(&["add", "docopt", "--vers", "0.8"], &manifest);

    // Now, upgrade `docopt` to the latest version
    execute_command(&["upgrade", "--dry-run"], &manifest);

    // Verify that `docopt` has not been updated.
    assert_eq!(
        get_toml(&manifest)["dependencies"]["docopt"].as_str(),
        Some("0.8")
    );
}

#[test]
fn upgrade_all_allow_prerelease_dry_run() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Setup manifest with the dependency `docopt@0.8`
    execute_command(&["add", "docopt", "--vers", "0.8"], &manifest);

    // Now, upgrade `docopt` to the latest version
    execute_command(&["upgrade", "--allow-prerelease", "--dry-run"], &manifest);

    // Verify that `docopt` has been updated successfully.
    assert_eq!(
        get_toml(&manifest)["dependencies"]["docopt"].as_str(),
        Some("0.8")
    );
}

#[test]
fn upgrade_specified_only() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Setup manifest with the dependencies `env_proxy` and `docopt`
    execute_command(&["add", "docopt", "--vers", "0.8"], &manifest);
    execute_command(&["add", "env_proxy", "--vers", "0.1.1"], &manifest);

    // Update `docopt` to the latest version
    execute_command(&["upgrade", "docopt"], &manifest);

    // Verify that `docopt` was upgraded, but not `env_proxy`
    let dependencies = &get_toml(&manifest)["dependencies"];
    assert_eq!(
        dependencies["docopt"].as_str(),
        Some("docopt--CURRENT_VERSION_TEST")
    );
    assert_eq!(dependencies["env_proxy"].as_str(), Some("0.1.1"));
}

#[test]
fn upgrade_major_only() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    execute_command(&["add", "test_breaking", "--vers", "0.1"], &manifest);
    execute_command(&["add", "test_nonbreaking", "--vers", "0.1"], &manifest);

    execute_command(&["upgrade", "--major-only"], &manifest);

    // Verify that `docopt` was upgraded, but not `env_proxy`
    let dependencies = &get_toml(&manifest)["dependencies"];
    assert_eq!(dependencies["test_breaking"].as_str(), Some("0.2.0"));
    assert_eq!(dependencies["test_nonbreaking"].as_str(), Some("0.1"));
}

#[test]
fn fails_to_upgrade_missing_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // `failure` is not a dependency. Try to upgrade it anyway.
    execute_command(&["upgrade", "failure"], &manifest);

    // Verify that `failure` has not been added
    assert!(get_toml(&manifest)["dependencies"]["failure"].is_none());
}

#[test]
fn upgrade_optional_dependency() {
    // Set up a Cargo.toml with an optional dependency `test_optional_dependency` verifies that this
    // is correct.
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");
    execute_command(
        &["add", "docopt", "--vers", ">=0.1.1", "--optional"],
        &manifest,
    );

    // Now, update without including the `optional` flag.
    execute_command(&["upgrade"], &manifest);

    // Dependency present afterwards - correct version, and still optional.
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["docopt"];
    assert_eq!(
        val["version"].as_str(),
        Some("docopt--CURRENT_VERSION_TEST")
    );
    assert_eq!(val["optional"].as_bool(), Some(true));
}

#[test]
fn upgrade_at() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Setup manifest
    execute_command(&["add", "docopt", "--vers", "0.8"], &manifest);

    // Now, upgrade `docopt` to a version that seems unlikely to ever get published.
    execute_command(&["upgrade", "docopt@1000000.0.0"], &manifest);

    // Verify that `docopt` has been updated to the specified version.
    assert_eq!(
        get_toml(&manifest)["dependencies"]["docopt"].as_str(),
        Some("1000000.0.0")
    );
}

#[test]
fn upgrade_workspace() {
    let (_tmpdir, root_manifest, workspace_manifests) = copy_workspace_test();

    execute_command(&["upgrade", "--all"], &root_manifest);

    // All of the workspace members have `libc` as a dependency.
    for workspace_member in workspace_manifests {
        assert_eq!(
            get_toml(&workspace_member)["dependencies"]["libc"].as_str(),
            Some("libc--CURRENT_VERSION_TEST")
        );
    }
}

/// Detect if attempting to run against a workspace root and give a helpful warning.
#[test]
fn detect_workspace() {
    let (_tmpdir, root_manifest, _workspace_manifests) = copy_workspace_test();

    assert_cli::Assert::command(&[
        "target/debug/cargo-upgrade",
        "upgrade",
        "--manifest-path",
        &root_manifest,
    ]).fails_with(1)
        .and()
        .stderr()
        .is(
            "Command failed due to unhandled error: Found virtual manifest, but this command \
             requires running against an actual package in this workspace. Try adding `--all`.",
        )
        .unwrap();
}

#[test]
fn invalid_manifest() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/upgrade/Cargo.toml.invalid");

    assert_cli::Assert::command(&[
        "target/debug/cargo-upgrade",
        "upgrade",
        "--manifest-path",
        &manifest,
    ]).fails_with(1)
        .and()
        .stderr()
        .is(
            "Command failed due to unhandled error: Unable to parse Cargo.toml

Caused by: Manifest not valid TOML
Caused by: TOML parse error at line 1, column 6
  |
1 | This is clearly not a valid Cargo.toml.
  |      ^
Unexpected `i`
Expected `=`",
        )
        .unwrap();
}

#[test]
fn invalid_root_manifest() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/upgrade/Cargo.toml.invalid");

    assert_cli::Assert::command(&[
        "target/debug/cargo-upgrade",
        "upgrade",
        "--all",
        "--manifest-path",
        &manifest,
    ]).fails_with(1)
        .and()
        .stderr()
        .contains("Command failed due to unhandled error: Failed to get workspace metadata")
        .unwrap();
}

#[test]
fn unknown_flags() {
    assert_cli::Assert::command(&["target/debug/cargo-upgrade", "upgrade", "foo", "--flag"])
        .fails_with(1)
        .and()
        .stderr()
        .is("Unknown flag: '--flag'

Usage:
    cargo upgrade [options] [<dependency>]...
    cargo upgrade (-h | --help)
    cargo upgrade (-V | --version)")
        .unwrap();
}

#[test]
fn upgrade_prints_messages() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/upgrade/Cargo.toml.source");

    assert_cli::Assert::command(&[
        "target/debug/cargo-upgrade",
        "upgrade",
        "docopt",
        &format!("--manifest-path={}", manifest),
    ]).succeeds()
        .and()
        .stdout()
        .contains("docopt v0.8 -> v")
        .unwrap();
}
