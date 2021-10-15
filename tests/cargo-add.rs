use assert_fs::prelude::*;

#[macro_use]
extern crate pretty_assertions;

mod utils;

use crate::utils::{
    clone_out_test, copy_workspace_test, execute_bad_command, execute_command,
    execute_command_for_pkg, get_toml, setup_alt_registry_config,
};
use assert_cmd::Command;

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
    assert_eq!(val.as_str().unwrap(), "99999.0.0");
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
    assert_eq!(val.as_str().unwrap(), "99999.0.0-alpha.1");
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

    let expected_result = format!("{}99999.0.0", expected_prefix);
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
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    let upgrade_arg = "--upgrade=an_invalid_string".to_string();
    execute_bad_command(&["add", "my-package", upgrade_arg.as_str()], &manifest);
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
    assert_eq!(val.as_str().unwrap(), "99999.0.0");
    let val = &toml["dependencies"]["my-package2"];
    assert_eq!(val.as_str().unwrap(), "99999.0.0");
}

#[test]
fn adds_renamed_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(&["add", "my-package1", "--rename", "renamed"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let renamed = &toml["dependencies"]["renamed"];
    assert_eq!(renamed["version"].as_str().unwrap(), "99999.0.0");
    assert_eq!(renamed["package"].as_str().unwrap(), "my-package1");
}

#[test]
fn adds_multiple_dependencies_conficts_with_rename() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_bad_command(
        &["add", "--rename", "rename", "my-package1", "my-package2"],
        &manifest,
    );
}

#[test]
fn adds_multiple_dependencies_with_conflicts_option() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_bad_command(
        &["add", "my-package1", "my-package2", "--vers", "0.1.0"],
        &manifest,
    );
    execute_bad_command(
        &[
            "add",
            "my-package1",
            "my-package2",
            "--git",
            "https://github.com/dcjanus/invalid",
        ],
        &manifest,
    );
    execute_bad_command(
        &["add", "my-package1", "my-package2", "--path", "./foo"],
        &manifest,
    );
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
    assert_eq!(val.as_str().unwrap(), "99999.0.0");
    let val = &toml["build-dependencies"]["my-build-package"];
    assert_eq!(val.as_str().unwrap(), "99999.0.0");

    // cannot run with both --dev and --build at the same time
    let call = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", BOGUS_CRATE_NAME, "--dev", "--build"])
        .arg(format!("--manifest-path={}", &manifest))
        .env("CARGO_IS_TEST", "1")
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
    assert_eq!(val.as_str().unwrap(), "99999.0.0");
    let val = &toml["dev-dependencies"]["my-dev-package2"];
    assert_eq!(val.as_str().unwrap(), "99999.0.0");
    let val = &toml["build-dependencies"]["my-build-package1"];
    assert_eq!(val.as_str().unwrap(), "99999.0.0");
    let val = &toml["build-dependencies"]["my-build-package2"];
    assert_eq!(val.as_str().unwrap(), "99999.0.0");
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
    let call = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", BOGUS_CRATE_NAME, "--vers", "invalid version string"])
        .arg(format!("--manifest-path={}", &manifest))
        .env("CARGO_IS_TEST", "1")
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
    assert_eq!(val.as_str().expect("not string"), "99999.0.0");
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
    assert_eq!(val["branch"].as_str(), None);

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
    assert_eq!(val["branch"].as_str(), None);
}

#[test]
fn adds_git_branch_using_flag() {
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
            "--branch",
            "master",
        ],
        &manifest,
    );

    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["git-package"];
    assert_eq!(
        val["git"].as_str(),
        Some("http://localhost/git-package.git")
    );

    assert_eq!(val["branch"].as_str(), Some("master"));

    // check this works with other flags (e.g. --dev) as well
    let toml = get_toml(&manifest);
    assert!(toml["dev-dependencies"].is_none());

    execute_command(
        &[
            "add",
            "git-dev-pkg",
            "--git",
            "http://site/gp.git",
            "--branch",
            "master",
            "--dev",
        ],
        &manifest,
    );

    let toml = get_toml(&manifest);
    let val = &toml["dev-dependencies"]["git-dev-pkg"];
    assert_eq!(val["git"].as_str(), Some("http://site/gp.git"));
    assert_eq!(val["branch"].as_str(), Some("master"));
}

#[test]
fn adds_local_source_using_flag() {
    let tmpdir = assert_fs::TempDir::new().expect("failed to construct temporary directory");
    let manifest_path = tmpdir.child("primary/Cargo.toml");
    tmpdir
        .child("Cargo.toml")
        .write_str(
            r#"[workspace]
members = ["primary", "dependency"]
"#,
        )
        .expect("Manifest is writeable");
    manifest_path
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");
    tmpdir
        .child("dependency/Cargo.toml")
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "0.0.0"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");

    let assert = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "cargo-list-test-fixture-dependency",
            "--path",
            "../dependency",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    println!("Succeeded: {}", assert);
    manifest_path.assert(
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { path = "../dependency" }
"#,
    );

    // check this works with other flags (e.g. --dev) as well
    let assert = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "cargo-list-test-fixture-dependency",
            "--path",
            "../dependency",
            "--dev",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    println!("Succeeded: {}", assert);
    manifest_path.assert(
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { path = "../dependency" }

[dev-dependencies]
cargo-list-test-fixture-dependency = { path = "../dependency" }
"#,
    );
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
    let tmpdir = assert_fs::TempDir::new().expect("failed to construct temporary directory");
    let manifest_path = tmpdir.child("primary/Cargo.toml");
    tmpdir
        .child("Cargo.toml")
        .write_str(
            r#"[workspace]
members = ["primary", "dependency"]
"#,
        )
        .expect("Manifest is writeable");
    manifest_path
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");
    tmpdir
        .child("dependency/Cargo.toml")
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "0.0.0"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");

    let assert = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", "../dependency"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    println!("Succeeded: {}", assert);
    manifest_path.assert(
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { version = "0.0.0", path = "../dependency" }
"#,
    );

    // check this works with other flags (e.g. --dev) as well
    let assert = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", "../dependency", "--dev"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    println!("Succeeded: {}", assert);
    manifest_path.assert(
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { version = "0.0.0", path = "../dependency" }

[dev-dependencies]
cargo-list-test-fixture-dependency = { path = "../dependency" }
"#,
    );
}

#[test]
fn adds_local_source_without_flag_without_workspace() {
    let tmpdir = assert_fs::TempDir::new().expect("failed to construct temporary directory");
    let manifest_path = tmpdir.child("primary/Cargo.toml");
    manifest_path
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");
    tmpdir
        .child("dependency/Cargo.toml")
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "0.0.0"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");

    let assert = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", "../dependency"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    println!("Succeeded: {}", assert);
    manifest_path.assert(
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { path = "../dependency" }
"#,
    );

    // check this works with other flags (e.g. --dev) as well
    let assert = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", "../dependency", "--dev"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    println!("Succeeded: {}", assert);
    manifest_path.assert(
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { path = "../dependency" }

[dev-dependencies]
cargo-list-test-fixture-dependency = { path = "../dependency" }
"#,
    );
}

#[test]
fn adds_local_source_with_version_flag() {
    let tmpdir = assert_fs::TempDir::new().expect("failed to construct temporary directory");
    let manifest_path = tmpdir.child("primary/Cargo.toml");
    tmpdir
        .child("Cargo.toml")
        .write_str(
            r#"[workspace]
members = ["primary", "dependency"]
"#,
        )
        .expect("Manifest is writeable");
    manifest_path
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");
    tmpdir
        .child("dependency/Cargo.toml")
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "0.4.3"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");

    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "cargo-list-test-fixture-dependency",
            "--vers=0.4.3",
            "--path",
            "../dependency",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    manifest_path.assert(
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { version = "0.4.3", path = "../dependency" }
"#,
    );

    // check this works with other flags (e.g. --dev) as well
    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "cargo-list-test-fixture-dependency",
            "--vers=0.4.3",
            "--path",
            "../dependency",
            "--dev",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    manifest_path.assert(
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { version = "0.4.3", path = "../dependency" }

[dev-dependencies]
cargo-list-test-fixture-dependency = { version = "0.4.3", path = "../dependency" }
"#,
    );
}

#[test]
fn adds_local_source_with_version_flag_and_semver_metadata() {
    let tmpdir = assert_fs::TempDir::new().expect("failed to construct temporary directory");
    let manifest_path = tmpdir.child("primary/Cargo.toml");
    tmpdir
        .child("Cargo.toml")
        .write_str(
            r#"[workspace]
members = ["primary", "dependency"]
"#,
        )
        .expect("Manifest is writeable");
    manifest_path
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");
    tmpdir
        .child("dependency/Cargo.toml")
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "0.4.3+useless-metadata.1.0.0"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");

    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "cargo-list-test-fixture-dependency",
            "--vers=0.4.3+useless-metadata.1.0.0",
            "--path",
            "../dependency",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    manifest_path.assert(
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { version = "0.4.3", path = "../dependency" }
"#,
    );

    // check this works with other flags (e.g. --dev) as well
    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "cargo-list-test-fixture-dependency",
            "--vers=0.4.3+useless-metadata.1.0.0",
            "--path",
            "../dependency",
            "--dev",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    manifest_path.assert(
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { version = "0.4.3", path = "../dependency" }

[dev-dependencies]
cargo-list-test-fixture-dependency = { version = "0.4.3", path = "../dependency" }
"#,
    );
}

#[test]
fn adds_local_source_with_inline_version_notation() {
    let tmpdir = assert_fs::TempDir::new().expect("failed to construct temporary directory");
    let manifest_path = tmpdir.child("primary/Cargo.toml");
    tmpdir
        .child("Cargo.toml")
        .write_str(
            r#"[workspace]
members = ["primary", "dependency"]
"#,
        )
        .expect("Manifest is writeable");
    manifest_path
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");
    tmpdir
        .child("dependency/Cargo.toml")
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "0.4.3"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");

    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "cargo-list-test-fixture-dependency@0.4.3",
            "--path",
            "../dependency",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    manifest_path.assert(
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { version = "0.4.3", path = "../dependency" }
"#,
    );

    // check this works with other flags (e.g. --dev) as well
    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "cargo-list-test-fixture-dependency@0.4.3",
            "--path",
            "../dependency",
            "--dev",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    manifest_path.assert(
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { version = "0.4.3", path = "../dependency" }

[dev-dependencies]
cargo-list-test-fixture-dependency = { version = "0.4.3", path = "../dependency" }
"#,
    );
}

#[test]
fn local_path_is_self() {
    let (tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let assert = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", BOGUS_CRATE_NAME])
        .args(&["--path", "."])
        .arg(format!("--manifest-path={}", &manifest))
        .env("CARGO_IS_TEST", "1")
        .current_dir(tmpdir.path())
        .assert()
        .failure();
    println!("Succeeded: {}", assert);

    assert!(no_manifest_failures(&get_toml(&manifest).root));
}

#[test]
fn git_and_version_flags_are_mutually_exclusive() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let call = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", BOGUS_CRATE_NAME])
        .args(&["--vers", "0.4.3"])
        .args(&["--git", "git://git.git"])
        .arg(format!("--manifest-path={}", &manifest))
        .env("CARGO_IS_TEST", "1")
        .output()
        .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest).root));
}

#[test]
fn git_flag_and_inline_version_are_mutually_exclusive() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let call = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", &format!("{}@0.4.3", BOGUS_CRATE_NAME)])
        .args(&["--git", "git://git.git"])
        .arg(format!("--manifest-path={}", &manifest))
        .env("CARGO_IS_TEST", "1")
        .output()
        .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest).root));
}

#[test]
fn git_and_path_are_mutually_exclusive() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let call = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", BOGUS_CRATE_NAME])
        .args(&["--git", "git://git.git"])
        .args(&["--path", "/path/here"])
        .arg(format!("--manifest-path={}", &manifest))
        .env("CARGO_IS_TEST", "1")
        .output()
        .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest).root));
}

#[test]
fn git_and_registry_are_mutually_exclusive() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let call = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", BOGUS_CRATE_NAME])
        .args(&["--git", "git://git.git"])
        .args(&["--registry", "alternative"])
        .arg(format!("--manifest-path={}", &manifest))
        .env("CARGO_IS_TEST", "1")
        .output()
        .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest).root));
}

#[test]
fn registry_and_path_are_mutually_exclusive() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let call = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", BOGUS_CRATE_NAME])
        .args(&["--registry", "alternative"])
        .args(&["--path", "/path/here"])
        .arg(format!("--manifest-path={}", &manifest))
        .env("CARGO_IS_TEST", "1")
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
    assert!(val.as_bool().expect("optional not a bool"));
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
    assert!(!val.as_bool().expect("default-features not a bool"));
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
fn adds_alternative_registry_dependency() {
    let (tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");
    setup_alt_registry_config(tmpdir.path());

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(
        &[
            "add",
            "versioned-package",
            "--vers",
            ">=0.1.1",
            "--registry",
            "alternative",
        ],
        &manifest,
    );

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["versioned-package"]["registry"];
    assert_eq!(val.as_str().expect("registry not a string"), "alternative");
}

#[test]
fn adds_multiple_alternative_registry_dependencies() {
    let (tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");
    setup_alt_registry_config(tmpdir.path());

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(
        &[
            "add",
            "--registry",
            "alternative",
            "my-package1",
            "my-package2",
        ],
        &manifest,
    );

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    assert_eq!(
        toml["dependencies"]["my-package1"]["registry"]
            .as_str()
            .expect("registry not a string"),
        "alternative"
    );
    assert_eq!(
        toml["dependencies"]["my-package2"]["registry"]
            .as_str()
            .expect("registry not a string"),
        "alternative"
    );
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
    assert_eq!(val.as_str().unwrap(), "99999.0.0");
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

    assert_eq!(val.as_str().unwrap(), "99999.0.0");
}

#[test]
fn adds_features_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_command(
        &[
            "add",
            "https://github.com/killercup/cargo-edit.git",
            "--features",
            "jui",
        ],
        &manifest,
    );

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = toml["dependencies"]["cargo-edit"]["features"][0].as_str();
    assert_eq!(val, Some("jui"));
}

#[test]
fn overrides_existing_features() {
    overwrite_dependency_test(
        &["add", "your-face", "--features", "nose"],
        &["add", "your-face", "--features", "mouth"],
        r#"
[dependencies]
your-face = { version = "99999.0.0", features = ["mouth"] }
"#,
    )
}

#[test]
fn keeps_existing_features_by_default() {
    overwrite_dependency_test(
        &["add", "your-face", "--features", "nose"],
        &["add", "your-face"],
        r#"
[dependencies]
your-face = { version = "99999.0.0", features = ["nose"] }
"#,
    )
}

#[test]
fn handles_specifying_features_option_multiple_times() {
    overwrite_dependency_test(
        &["add", "your-face"],
        &[
            "add",
            "your-face",
            "--features",
            "nose",
            "--features",
            "mouth",
        ],
        r#"
[dependencies]
your-face = { version = "99999.0.0", features = ["nose", "mouth"] }
"#,
    )
}

#[test]
fn can_be_forced_to_provide_an_empty_features_list() {
    overwrite_dependency_test(
        &["add", "your-face"],
        &["add", "your-face", "--features", ""],
        r#"
[dependencies]
your-face = { version = "99999.0.0", features = [] }
"#,
    )
}

#[test]
fn parses_space_separated_argument_to_features() {
    overwrite_dependency_test(
        &["add", "your-face", "--features", "nose"],
        &["add", "your-face", "--features", "mouth ears"],
        r#"
[dependencies]
your-face = { version = "99999.0.0", features = ["mouth", "ears"] }
"#,
    )
}

#[test]
fn forbids_multiple_crates_with_features_option() {
    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", "your-face", "--features", "mouth", "nose"])
        .env("CARGO_IS_TEST", "1")
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "Cannot specify multiple crates with features",
        ));
}

#[test]
fn adds_dependency_with_custom_target() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    execute_command(
        &["add", "--target", "windows.json", "my-package1"],
        &manifest,
    );

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    // Get package by hand because toml-rs does not currently handle escaping dots in get()
    let val = &toml["target"]["windows.json"]["dependencies"]["my-package1"];
    assert_eq!(val.as_str(), Some("99999.0.0"));
}

#[test]
#[cfg(feature = "test-external-apis")]
fn adds_dependency_normalized_name() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "linked_hash_map",
            "Inflector",
            &format!("--manifest-path={}", manifest),
        ])
        .assert()
        .success()
        .stderr(predicates::str::contains(
            "WARN: Added `linked-hash-map` instead of `linked_hash_map`",
        ));

    // dependency present afterwards
    let toml = get_toml(&manifest);
    assert!(!toml["dependencies"]["linked-hash-map"].is_none());
}

#[test]
fn fails_to_add_dependency_with_empty_target() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Fails because target parameter must be a valid target
    execute_bad_command(&["add", "--target", "", "my-package1"], &manifest);
}

#[test]
fn fails_to_add_optional_dev_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    // Fails because optional dependencies must be in `dependencies` table.
    execute_bad_command(
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
fn fails_to_add_multiple_optional_dev_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    // Fails because optional dependencies must be in `dependencies` table.
    execute_bad_command(
        &["add", "--optional", "my-package1", "my-package2", "--dev"],
        &manifest,
    );
}

#[test]
#[cfg(feature = "test-external-apis")]
fn fails_to_add_inexistent_git_source_without_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_bad_command(
        &["add", "https://github.com/killercup/fake-git-repo.git"],
        &manifest,
    );
}

#[test]
fn fails_to_add_inexistent_local_source_without_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml["dependencies"].is_none());

    execute_bad_command(&["add", "./tests/fixtures/local"], &manifest);
}

fn overwrite_dependency_test(first_command: &[&str], second_command: &[&str], expected: &str) {
    let orig_manifest = r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"
"#;
    let expected = orig_manifest.to_owned() + expected;
    expected
        .parse::<toml_edit::Document>()
        .expect("expected is valid toml");

    let tmpdir = assert_fs::TempDir::new().expect("failed to construct temporary directory");
    let manifest_path = tmpdir.child("primary/Cargo.toml");
    tmpdir
        .child("Cargo.toml")
        .write_str(
            r#"[workspace]
members = ["primary", "dependency"]
"#,
        )
        .expect("Manifest is writeable");
    manifest_path
        .write_str(orig_manifest)
        .expect("Manifest is writeable");
    tmpdir
        .child("dependency/Cargo.toml")
        .write_str(
            r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "0.0.0"

[lib]
path = "dummy.rs"
"#,
        )
        .expect("Manifest is writeable");

    // First, add a dependency.
    let assert = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(first_command)
        .arg("--manifest-path")
        .arg(manifest_path.path())
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    println!("Succeeded: {}", assert);

    // Then, overwrite with the latest version
    let assert = Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(second_command)
        .arg("--manifest-path")
        .arg(manifest_path.path())
        .env("CARGO_IS_TEST", "1")
        .current_dir(manifest_path.path().parent().expect("there is a parent"))
        .assert()
        .success();
    println!("Succeeded: {}", assert);

    // Verify that the dependency is as expected.
    manifest_path.assert(&expected);

    tmpdir.close().expect("No stray file handles");
}

#[test]
fn overwrite_version_with_version() {
    overwrite_dependency_test(
        &["add", "versioned-package", "--vers", "0.1.1", "--optional"],
        &["add", "versioned-package"],
        r#"
[dependencies]
versioned-package = { version = "99999.0.0", optional = true }
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
        &["add", "versioned-package", "--path", "../dependency"],
        r#"
[dependencies]
versioned-package = { optional = true, path = "../dependency" }
"#,
    )
}

#[test]
fn overwrite_renamed() {
    overwrite_dependency_test(
        &["add", "versioned-package", "--vers", "0.1"],
        &["add", "versioned-package", "--rename", "renamed"],
        r#"
[dependencies]
renamed = { version = "99999.0.0", package = "versioned-package" }
"#,
    )
}

#[test]
fn overwrite_renamed_optional() {
    overwrite_dependency_test(
        &["add", "versioned-package", "--vers", "0.1", "--optional"],
        &["add", "versioned-package", "--rename", "renamed"],
        r#"
[dependencies]
renamed = { version = "99999.0.0", optional = true, package = "versioned-package" }
"#,
    )
}

#[test]
fn overwrite_differently_renamed() {
    overwrite_dependency_test(
        &["add", "a", "--vers", "0.1", "--rename", "a1"],
        &["add", "a", "--vers", "0.2", "--rename", "a2"],
        r#"
[dependencies]
a2 = { version = "0.2", package = "a" }
"#,
    )
}

#[test]
fn overwrite_previously_renamed() {
    overwrite_dependency_test(
        &["add", "a", "--vers", "0.1", "--rename", "a1"],
        &["add", "a", "--vers", "0.2"],
        r#"
[dependencies]
a = "0.2"
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
        &["add", "versioned-package", "--path", "../dependency"],
        r#"
[dependencies]
versioned-package = { optional = true, path = "../dependency" }
"#,
    )
}

#[test]
fn overwrite_path_with_version() {
    overwrite_dependency_test(
        &["add", "versioned-package", "--path", "../dependency"],
        &["add", "versioned-package"],
        r#"
[dependencies]
versioned-package = "99999.0.0"
"#,
    )
}

#[test]
fn no_argument() {
    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add"])
        .env("CARGO_IS_TEST", "1")
        .assert()
        .code(1)
        .stderr(
            r"error: The following required arguments were not provided:
    <crate>...

USAGE:
    cargo add <crate>... --upgrade <method>

For more information try --help
",
        );
}

#[test]
fn unknown_flags() {
    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&["add", "foo", "--flag"])
        .env("CARGO_IS_TEST", "1")
        .assert()
        .code(1)
        .stderr(
            r"error: Found argument '--flag' which wasn't expected, or isn't valid in this context

USAGE:
    cargo add [FLAGS] [OPTIONS] <crate>...

For more information try --help
",
        );
}

#[test]
fn add_prints_message() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "docopt",
            "--vers=0.6.0",
            &format!("--manifest-path={}", manifest),
        ])
        .env("CARGO_IS_TEST", "1")
        .assert()
        .success()
        .stderr(predicates::str::contains(
            "Adding docopt v0.6.0 to dependencies",
        ));
}

#[test]
fn add_prints_message_with_section() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "clap",
            "--optional",
            "--target=mytarget",
            "--vers=0.1.0",
            &format!("--manifest-path={}", manifest),
        ])
        .env("CARGO_IS_TEST", "1")
        .assert()
        .success()
        .stderr(predicates::str::contains(
            "Adding clap v0.1.0 to optional dependencies for target `mytarget`",
        ));
}

#[test]
fn add_prints_message_for_dev_deps() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "docopt",
            "--dev",
            "--vers",
            "0.8.0",
            &format!("--manifest-path={}", manifest),
        ])
        .env("CARGO_IS_TEST", "1")
        .assert()
        .success()
        .stderr(predicates::str::contains(
            "Adding docopt v0.8.0 to dev-dependencies",
        ));
}

#[test]
fn add_prints_message_for_build_deps() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "hello-world",
            "--build",
            "--vers",
            "0.1.0",
            &format!("--manifest-path={}", manifest),
        ])
        .env("CARGO_IS_TEST", "1")
        .assert()
        .success()
        .stderr(predicates::str::contains(
            "Adding hello-world v0.1.0 to build-dependencies",
        ));
}

#[test]
#[cfg(feature = "test-external-apis")]
fn add_typo() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "lets_hope_nobody_ever_publishes_this_crate",
            &format!("--manifest-path={}", manifest),
        ])
        .env("CARGO_IS_TEST", "1")
        .assert()
        .code(1)
    .stderr(predicates::str::contains(
        "The crate `lets_hope_nobody_ever_publishes_this_crate` could not be found in registry index.",
    ))
    ;
}

#[test]
fn sorts_unsorted_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.unsorted");

    // adds one dependency
    execute_command(&["add", "--sort", "toml"], &manifest);

    // and all the dependencies in the output get sorted
    let toml = get_toml(&manifest);
    assert_eq!(
        toml.to_string().replace("\r\n", "\n"),
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[dependencies]
atty = "0.2.13"
toml = "99999.0.0"
toml_edit = "0.1.5"
"#
    );
}

#[test]
fn adds_unsorted_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.unsorted");

    // adds one dependency
    execute_command(&["add", "toml"], &manifest);

    // and unsorted dependencies stay unsorted
    let toml = get_toml(&manifest);
    assert_eq!(
        toml.to_string().replace("\r\n", "\n"),
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[dependencies]
toml_edit = "0.1.5"
atty = "0.2.13"
toml = "99999.0.0"
"#
    );
}

#[test]
fn keeps_sorted_dependencies_sorted() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sorted");

    // adds one dependency
    execute_command(&["add", "toml"], &manifest);

    // and all the dependencies in the output get sorted
    let toml = get_toml(&manifest);
    assert_eq!(
        toml.to_string().replace("\r\n", "\n"),
        r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[dependencies]
atty = "0.2.13"
toml = "99999.0.0"
toml_edit = "0.1.5"
"#
    );
}

#[test]
fn add_dependency_to_workspace_member() {
    let (tmpdir, _root_manifest, workspace_manifests) = copy_workspace_test();
    execute_command_for_pkg(&["add", "toml"], "one", &tmpdir);

    let one = workspace_manifests
        .iter()
        .map(|manifest| get_toml(manifest))
        .find(|manifest| manifest["package"]["name"].as_str() == Some("one"))
        .expect("Couldn't find workspace member `one'");

    assert_eq!(
        one["dependencies"]["toml"]
            .as_str()
            .expect("toml dependency did not exist"),
        "99999.0.0",
    );
}
#[test]
fn add_prints_message_for_features_deps() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    Command::cargo_bin("cargo-add")
        .unwrap()
        .args(&[
            "add",
            "hello-world",
            "--vers",
            "0.1.0",
            "--features",
            "jui",
            &format!("--manifest-path={}", manifest),
        ])
        .env("CARGO_IS_TEST", "1")
        .assert()
        .success()
        .stderr(predicates::str::contains(
            r#"Adding hello-world v0.1.0 to dependencies with features: ["jui"]"#,
        ));
}
