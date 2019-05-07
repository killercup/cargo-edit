#[macro_use]
extern crate pretty_assertions;

use std::process;
mod utils;
use crate::utils::{clone_out_test, execute_command, get_toml};

/// Some of the tests need to have a crate name that does not exist on crates.io. Hence this rather
/// silly constant. Tests _will_ fail, though, if a crate is ever published with this name.
const BOGUS_CRATE_NAME: &str = "tests-will-break-if-there-is-ever-a-real-package-with-this-name";

/// Check 'failure' deps are not present
fn no_manifest_failures(manifest: &toml_edit::Item) -> bool {
    let no_failure_key_in = |section| manifest[section][BOGUS_CRATE_NAME].is_none();
    no_failure_key_in("dependencies")
        && no_failure_key_in("dev-dependencies")
        && no_failure_key_in("build-dependencies")
}

#[test]
fn adds_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(&["add", "my-package"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["my-package"];
    assert_eq!(val.as_str().unwrap(), "my-package--CURRENT_VERSION_TEST");
}

#[test]
fn adds_prerelease_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(&["add", "my-package", "--allow-prerelease"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["my-package"];
    assert_eq!(val.as_str().unwrap(), "my-package--PRERELEASE_VERSION_TEST");
}

fn upgrade_test_helper(upgrade_method: &str, expected_prefix: &str) {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    let upgrade_arg = format!("--upgrade={0}", upgrade_method);

    execute_command(&["add", "my-package", upgrade_arg.as_str()], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["my-package"];

    let expected_result = format!("{0}my-package--CURRENT_VERSION_TEST", expected_prefix);
    assert_eq!(val.as_str().unwrap(), expected_result);
}

#[test]
fn adds_dependency_with_upgrade_none() {
    upgrade_test_helper("none", "=");
}
#[test]
fn adds_dependency_with_upgrade_patch() {
    upgrade_test_helper("patch", "~");
}
#[test]
fn adds_dependency_with_upgrade_minor() {
    upgrade_test_helper("minor", "^");
}
#[test]
fn adds_dependency_with_upgrade_all() {
    upgrade_test_helper("all", ">=");
}

#[test]
fn adds_dependency_with_upgrade_bad() {
    upgrade_test_helper("an_invalid_string", "");
}

#[test]
fn adds_multiple_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(&["add", "my-package1", "my-package2"], &manifest);

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["my-package1"];
    assert_eq!(val.as_str().unwrap(), "my-package1--CURRENT_VERSION_TEST");
    let val = &toml["dependencies"]["my-package2"];
    assert_eq!(val.as_str().unwrap(), "my-package2--CURRENT_VERSION_TEST");
}

#[test]
fn adds_dev_build_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"].is_none());
    assert!(toml["build-dependencies"].is_none());

    execute_command(&["add", "my-dev-package", "--dev"], &manifest);
    execute_command(&["add", "my-build-package", "--build"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dev-dependencies"]["my-dev-package"];
    assert_eq!(
        val.as_str().unwrap(),
        "my-dev-package--CURRENT_VERSION_TEST"
    );
    let val = &toml["build-dependencies"]["my-build-package"];
    assert_eq!(
        val.as_str().unwrap(),
        "my-build-package--CURRENT_VERSION_TEST"
    );

    // cannot run with both --dev and --build at the same time
    let call = process::Command::new("target/debug/cargo-add")
        .args(&["add", BOGUS_CRATE_NAME, "--dev", "--build"])
        .arg(format!("--manifest-path={}", &manifest))
        .output()
        .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest).root));
}

#[test]
fn adds_multiple_dev_build_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"].is_none());
    assert!(toml["dev-dependencies"].is_none());
    assert!(toml["build-dependencies"].is_none());
    assert!(toml["build-dependencies"].is_none());

    execute_command(
        &["add", "my-dev-package1", "my-dev-package2", "--dev"],
        &manifest,
    );
    execute_command(
        &["add", "my-build-package1", "--build", "my-build-package2"],
        &manifest,
    );

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dev-dependencies"]["my-dev-package1"];
    assert_eq!(
        val.as_str().unwrap(),
        "my-dev-package1--CURRENT_VERSION_TEST"
    );
    let val = &toml["dev-dependencies"]["my-dev-package2"];
    assert_eq!(
        val.as_str().unwrap(),
        "my-dev-package2--CURRENT_VERSION_TEST"
    );
    let val = &toml["build-dependencies"]["my-build-package1"];
    assert_eq!(
        val.as_str().unwrap(),
        "my-build-package1--CURRENT_VERSION_TEST"
    );
    let val = &toml["build-dependencies"]["my-build-package2"];
    assert_eq!(
        val.as_str().unwrap(),
        "my-build-package2--CURRENT_VERSION_TEST"
    );
}

#[test]
fn adds_specified_version() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(
        &["add", "versioned-package", "--vers", ">=0.1.1"],
        &manifest,
    );

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["versioned-package"];
    assert_eq!(val.as_str().expect("not string"), ">=0.1.1");

    // cannot run with both --dev and --build at the same time
    let call = process::Command::new("target/debug/cargo-add")
        .args(&["add", BOGUS_CRATE_NAME, "--vers", "invalid version string"])
        .arg(format!("--manifest-path={}", &manifest))
        .output()
        .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest).root));
}

#[test]
fn adds_specified_version_with_inline_notation() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(&["add", "versioned-package@>=0.1.1"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["versioned-package"];
    assert_eq!(val.as_str().expect("not string"), ">=0.1.1");
}

#[test]
fn adds_multiple_dependencies_with_versions() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());
    assert!(toml["dependencies"].is_none());

    execute_command(
        &["add", "my-package1@>=0.1.1", "my-package2@0.2.3"],
        &manifest,
    );

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["my-package1"];
    assert_eq!(val.as_str().expect("not string"), ">=0.1.1");
    let val = &toml["dependencies"]["my-package2"];
    assert_eq!(val.as_str().expect("not string"), "0.2.3");
}

#[test]
fn adds_multiple_dependencies_with_some_versions() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());
    assert!(toml["dependencies"].is_none());

    execute_command(&["add", "my-package1", "my-package2@0.2.3"], &manifest);

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["my-package1"];
    assert_eq!(
        val.as_str().expect("not string"),
        "my-package1--CURRENT_VERSION_TEST"
    );
    let val = &toml["dependencies"]["my-package2"];
    assert_eq!(val.as_str().expect("not string"), "0.2.3");
}

#[test]
fn adds_git_source_using_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(
        &[
            "add",
            "git-package",
            "--git",
            "http://localhost/git-package.git",
        ],
        &manifest,
    );

    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["git-package"];
    assert_eq!(
        val["git"].as_str(),
        Some("http://localhost/git-package.git")
    );

    // check this works with other flags (e.g. --dev) as well
    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"].is_none());

    execute_command(
        &["add", "git-dev-pkg", "--git", "http://site/gp.git", "--dev"],
        &manifest,
    );

    let toml = get_toml(&manifest);
    let val = &toml["dev-dependencies"]["git-dev-pkg"];
    assert_eq!(val["git"].as_str(), Some("http://site/gp.git"));
}

#[test]
fn adds_local_source_using_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(&["add", "local", "--path", "/path/to/pkg"], &manifest);

    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["local"];
    assert_eq!(val["path"].as_str(), Some("/path/to/pkg"));

    // check this works with other flags (e.g. --dev) as well
    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"].is_none());

    execute_command(
        &["add", "local-dev", "--path", "/path/to/pkg-dev", "--dev"],
        &manifest,
    );

    let toml = get_toml(&manifest);
    let val = &toml["dev-dependencies"]["local-dev"];
    assert_eq!(val["path"].as_str(), Some("/path/to/pkg-dev"));
}

#[test]
#[cfg(feature = "test-external-apis")]
fn adds_git_source_without_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(
        &["add", "https://github.com/killercup/cargo-edit.git"],
        &manifest,
    );

    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["cargo-edit"];
    assert_eq!(
        val["git"].as_str(),
        Some("https://github.com/killercup/cargo-edit.git")
    );

    // check this works with other flags (e.g. --dev) as well
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");
    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"].is_none());

    execute_command(
        &[
            "add",
            "https://github.com/killercup/cargo-edit.git",
            "--dev",
        ],
        &manifest,
    );

    let toml = get_toml(&manifest);
    let val = &toml["dev-dependencies"]["cargo-edit"];
    assert_eq!(
        val["git"].as_str(),
        Some("https://github.com/killercup/cargo-edit.git")
    );
}

#[test]
fn adds_local_source_without_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let (tmpdir, _) = clone_out_test("tests/fixtures/add/local/Cargo.toml.sample");
    let tmppath = tmpdir.into_path();
    let tmpdirstr = tmppath.to_str().unwrap();

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(&["add", tmpdirstr], &manifest);

    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["foo-crate"];
    assert_eq!(val["path"].as_str(), Some(tmpdirstr));

    // check this works with other flags (e.g. --dev) as well
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");
    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"].is_none());

    execute_command(&["add", tmpdirstr, "--dev"], &manifest);

    let toml = get_toml(&manifest);
    let val = &toml["dev-dependencies"]["foo-crate"];
    assert_eq!(val["path"].as_str(), Some(tmpdirstr));
}

#[test]
fn adds_local_source_with_version_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(
        &["add", "local", "--vers", "0.4.3", "--path", "/path/to/pkg"],
        &manifest,
    );

    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["local"];
    assert_eq!(val["path"].as_str(), Some("/path/to/pkg"));
    assert_eq!(val["version"].as_str(), Some("0.4.3"));

    // check this works with other flags (e.g. --dev) as well
    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"].is_none());

    execute_command(
        &[
            "add",
            "local-dev",
            "--vers",
            "0.4.3",
            "--path",
            "/path/to/pkg-dev",
            "--dev",
        ],
        &manifest,
    );

    let toml = get_toml(&manifest);
    let val = &toml["dev-dependencies"]["local-dev"];
    assert_eq!(val["path"].as_str(), Some("/path/to/pkg-dev"));
    assert_eq!(val["version"].as_str(), Some("0.4.3"));
}

#[test]
fn adds_local_source_with_inline_version_notation() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(&["add", "local@0.4.3", "--path", "/path/to/pkg"], &manifest);

    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["local"];
    assert_eq!(val["path"].as_str(), Some("/path/to/pkg"));
    assert_eq!(val["version"].as_str(), Some("0.4.3"));

    // check this works with other flags (e.g. --dev) as well
    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"].is_none());

    execute_command(
        &[
            "add",
            "local-dev@0.4.3",
            "--path",
            "/path/to/pkg-dev",
            "--dev",
        ],
        &manifest,
    );

    let toml = get_toml(&manifest);
    let val = &toml["dev-dependencies"]["local-dev"];
    assert_eq!(val["path"].as_str(), Some("/path/to/pkg-dev"));
    assert_eq!(val["version"].as_str(), Some("0.4.3"));
}

#[test]
fn git_and_version_flags_are_mutually_exclusive() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let call = process::Command::new("target/debug/cargo-add")
        .args(&["add", BOGUS_CRATE_NAME])
        .args(&["--vers", "0.4.3"])
        .args(&["--git", "git://git.git"])
        .arg(format!("--manifest-path={}", &manifest))
        .output()
        .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest).root));
}

#[test]
fn git_flag_and_inline_version_are_mutually_exclusive() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let call = process::Command::new("target/debug/cargo-add")
        .args(&["add", &format!("{}@0.4.3", BOGUS_CRATE_NAME)])
        .args(&["--git", "git://git.git"])
        .arg(format!("--manifest-path={}", &manifest))
        .output()
        .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest).root));
}

#[test]
fn git_and_path_are_mutually_exclusive() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let call = process::Command::new("target/debug/cargo-add")
        .args(&["add", BOGUS_CRATE_NAME])
        .args(&["--git", "git://git.git"])
        .args(&["--path", "/path/here"])
        .arg(format!("--manifest-path={}", &manifest))
        .output()
        .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest).root));
}

#[test]
fn adds_optional_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

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

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["versioned-package"]["optional"];
    assert_eq!(val.as_bool().expect("optional not a bool"), true);
}

#[test]
fn adds_multiple_optional_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(
        &["add", "--optional", "my-package1", "my-package2"],
        &manifest,
    );

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    assert!(&toml["dependencies"]["my-package1"]["optional"]
        .as_bool()
        .expect("optional not a bool"));
    assert!(&toml["dependencies"]["my-package2"]["optional"]
        .as_bool()
        .expect("optional not a bool"));
}

#[test]
fn adds_no_default_features_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(
        &[
            "add",
            "versioned-package",
            "--vers",
            ">=0.1.1",
            "--no-default-features",
        ],
        &manifest,
    );

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["versioned-package"]["default-features"];
    assert_eq!(val.as_bool().expect("default-features not a bool"), false);
}

#[test]
fn adds_multiple_no_default_features_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(
        &["add", "--no-default-features", "my-package1", "my-package2"],
        &manifest,
    );

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    assert!(!&toml["dependencies"]["my-package1"]["default-features"]
        .as_bool()
        .expect("default-features not a bool"));
    assert!(!&toml["dependencies"]["my-package2"]["default-features"]
        .as_bool()
        .expect("default-features not a bool"));
}

#[test]
fn adds_dependency_with_target_triple() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["target"].is_none());

    execute_command(
        &["add", "--target", "i686-unknown-linux-gnu", "my-package1"],
        &manifest,
    );

    // dependencies present afterwards
    let toml = get_toml(&manifest);

    let val = &toml["target"]["i686-unknown-linux-gnu"]["dependencies"]["my-package1"];
    assert_eq!(val.as_str().unwrap(), "my-package1--CURRENT_VERSION_TEST");
}

#[test]
fn adds_dependency_with_target_cfg() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["target"].is_none());

    execute_command(&["add", "--target", "cfg(unix)", "my-package1"], &manifest);

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["target"]["cfg(unix)"]["dependencies"]["my-package1"];

    assert_eq!(val.as_str().unwrap(), "my-package1--CURRENT_VERSION_TEST");
}

#[test]
fn adds_dependency_with_custom_target() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    execute_command(
        &["add", "--target", "x86_64/windows.json", "my-package1"],
        &manifest,
    );

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    // Get package by hand because toml-rs does not currently handle escaping dots in get()
    let val = &toml["target"]["x86_64/windows.json"]["dependencies"]["my-package1"];
    assert_eq!(val.as_str(), Some("my-package1--CURRENT_VERSION_TEST"));
}

#[test]
#[cfg(feature = "test-external-apis")]
fn adds_dependency_normalized_name() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    assert_cli::Assert::command(&[
        "target/debug/cargo-add",
        "add",
        "linked_hash_map",
        &format!("--manifest-path={}", manifest),
    ])
    .succeeds()
    .and()
    .stdout()
    .contains("WARN: Added `linked-hash-map` instead of `linked_hash_map`")
    .unwrap();

    // dependency present afterwards
    let toml = get_toml(&manifest);
    assert!(!toml["dependencies"]["linked-hash-map"].is_none());
}

#[test]
#[should_panic]
fn fails_to_add_dependency_with_empty_target() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Fails because target parameter must be a valid target
    execute_command(&["add", "--target", "", "my-package1"], &manifest);
}

#[test]
#[should_panic]
fn fails_to_add_optional_dev_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    // Fails because optional dependencies must be in `dependencies` table.
    execute_command(
        &[
            "add",
            "versioned-package",
            "--vers",
            ">=0.1.1",
            "--dev",
            "--optional",
        ],
        &manifest,
    );
}

#[test]
#[should_panic]
fn fails_to_add_multiple_optional_dev_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    // Fails because optional dependencies must be in `dependencies` table.
    execute_command(
        &["add", "--optional", "my-package1", "my-package2", "--dev"],
        &manifest,
    );
}

#[test]
#[should_panic]
#[cfg(feature = "test-external-apis")]
fn fails_to_add_inexistent_git_source_without_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(
        &["add", "https://github.com/killercup/fake-git-repo.git"],
        &manifest,
    );
}

#[test]
#[should_panic]
fn fails_to_add_inexistent_local_source_without_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(&["add", "./tests/fixtures/local"], &manifest);
}

fn overwrite_dependency_test(first_command: &[&str], second_command: &[&str], expected: &str) {
    // First, add a dependency.
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");
    execute_command(first_command, &manifest);

    // Then, overwite with the latest version
    execute_command(second_command, &manifest);

    // Verify that the dependency is as expected.
    let toml = get_toml(&manifest);
    let expected = r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"
"#
    .to_string()
        + expected;
    let expected_dep: toml_edit::Document = expected.parse().expect("toml parse error");
    assert_eq!(expected_dep.to_string(), toml.to_string());
}

#[test]
fn overwrite_version_with_version() {
    overwrite_dependency_test(
        &["add", "versioned-package", "--vers", "0.1.1", "--optional"],
        &["add", "versioned-package"],
        r#"
[dependencies]
versioned-package = { version = "versioned-package--CURRENT_VERSION_TEST", optional = true }
"#,
    )
}

#[test]
fn overwrite_version_with_git() {
    overwrite_dependency_test(
        &["add", "versioned-package", "--vers", "0.1.1", "--optional"],
        &["add", "versioned-package", "--git", "git://git.git"],
        r#"
[dependencies]
versioned-package = { optional = true, git = "git://git.git" }
"#,
    )
}

#[test]
fn overwrite_version_with_path() {
    overwrite_dependency_test(
        &["add", "versioned-package", "--vers", "0.1.1", "--optional"],
        &["add", "versioned-package", "--path", "../foo"],
        r#"
[dependencies]
versioned-package = { optional = true, path = "../foo" }
"#,
    )
}

#[test]
fn overwrite_git_with_path() {
    overwrite_dependency_test(
        &[
            "add",
            "versioned-package",
            "--git",
            "git://git.git",
            "--optional",
        ],
        &["add", "versioned-package", "--path", "../foo"],
        r#"
[dependencies]
versioned-package = { optional = true, path = "../foo" }
"#,
    )
}

#[test]
fn overwrite_path_with_version() {
    overwrite_dependency_test(
        &["add", "versioned-package", "--path", "../foo"],
        &["add", "versioned-package"],
        r#"
[dependencies]
versioned-package = "versioned-package--CURRENT_VERSION_TEST"
"#,
    )
}

#[test]
fn no_argument() {
    assert_cli::Assert::command(&["target/debug/cargo-add", "add"])
        .fails_with(1)
        .and()
        .stderr()
        .is(r"Invalid arguments.

Usage:
    cargo add <crate> [--dev|--build|--optional] [options]
    cargo add <crates>... [--dev|--build|--optional] [options]
    cargo add (-h|--help)
    cargo add --version")
        .unwrap();
}

#[test]
fn unknown_flags() {
    assert_cli::Assert::command(&["target/debug/cargo-add", "add", "foo", "--flag"])
        .fails_with(1)
        .and()
        .stderr()
        .is(r"Unknown flag: '--flag'

Usage:
    cargo add <crate> [--dev|--build|--optional] [options]
    cargo add <crates>... [--dev|--build|--optional] [options]
    cargo add (-h|--help)
    cargo add --version")
        .unwrap();
}

#[test]
fn add_prints_message() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    assert_cli::Assert::command(&[
        "target/debug/cargo-add",
        "add",
        "docopt",
        "--vers=0.6.0",
        &format!("--manifest-path={}", manifest),
    ])
    .succeeds()
    .and()
    .stdout()
    .is("Adding docopt v0.6.0 to dependencies")
    .unwrap();
}

#[test]
fn add_prints_message_with_section() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    assert_cli::Assert::command(&[
        "target/debug/cargo-add",
        "add",
        "clap",
        "--optional",
        "--target=mytarget",
        "--vers=0.1.0",
        &format!("--manifest-path={}", manifest),
    ])
    .succeeds()
    .and()
    .stdout()
    .is("Adding clap v0.1.0 to optional dependencies for target `mytarget`")
    .unwrap();
}

#[test]
fn add_prints_message_for_dev_deps() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    assert_cli::Assert::command(&[
        "target/debug/cargo-add",
        "add",
        "docopt",
        "--dev",
        "--vers",
        "0.8.0",
        &format!("--manifest-path={}", manifest),
    ])
    .succeeds()
    .and()
    .stdout()
    .is("Adding docopt v0.8.0 to dev-dependencies")
    .unwrap();
}

#[test]
fn add_prints_message_for_build_deps() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    assert_cli::Assert::command(&[
        "target/debug/cargo-add",
        "add",
        "hello-world",
        "--build",
        "--vers",
        "0.1.0",
        &format!("--manifest-path={}", manifest),
    ])
    .succeeds()
    .and()
    .stdout()
    .is("Adding hello-world v0.1.0 to build-dependencies")
    .unwrap();
}

#[test]
#[cfg(feature = "test-external-apis")]
fn add_typo() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    assert_cli::Assert::command(&[
        "target/debug/cargo-add",
        "add",
        "lets_hope_nobody_ever_publishes_this_crate",
        &format!("--manifest-path={}", manifest),
    ])
    .fails_with(1)
    .and()
    .stderr()
    .contains(
        "The crate `lets_hope_nobody_ever_publishes_this_crate` could not be found \
         on crates.io.",
    )
    .unwrap();
}
