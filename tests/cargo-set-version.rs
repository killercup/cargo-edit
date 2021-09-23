#[macro_use]
extern crate pretty_assertions;

mod utils;

use assert_fs::prelude::*;

use crate::utils::{clone_out_test, execute_bad_command, execute_command, get_toml};

#[test]
fn set_absolute_version() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/set-version/Cargo.toml.sample");

    let toml = get_toml(&manifest);
    let val = &toml["package"]["version"];
    assert_eq!(val.as_str().unwrap(), "0.1.0");

    execute_command(&["set-version", "2.0.0"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["package"]["version"];
    assert_eq!(val.as_str().unwrap(), "2.0.0");
}

#[test]
fn set_relative_version() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/set-version/Cargo.toml.sample");

    let toml = get_toml(&manifest);
    let val = &toml["package"]["version"];
    assert_eq!(val.as_str().unwrap(), "0.1.0");

    execute_command(&["set-version", "--bump", "major"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = &toml["package"]["version"];
    assert_eq!(val.as_str().unwrap(), "1.0.0");
}

#[test]
fn relative_absolute_conflict() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/set-version/Cargo.toml.sample");

    execute_bad_command(&["set-version", "1.0.0", "--bump", "major"], &manifest);
}

#[test]
fn downgrade_error() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/set-version/Cargo.toml.sample");

    let toml = get_toml(&manifest);
    let val = &toml["package"]["version"];
    assert_eq!(val.as_str().unwrap(), "0.1.0");

    execute_bad_command(&["set-version", "0.0.1"], &manifest);
}

#[test]
fn test_version_dependency_ignored() {
    let tmpdir = assert_fs::TempDir::new().expect("failed to construct temporary directory");
    tmpdir
        .child("Cargo.toml")
        .write_str(
            r#"[workspace]
members = ["primary", "dependency"]
"#,
        )
        .expect("Manifest is writeable");
    let primary_manifest_path = tmpdir.child("primary/Cargo.toml");
    let primary_manifest = r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = "0.4.3"
"#;
    primary_manifest_path
        .write_str(primary_manifest)
        .expect("Manifest is writeable");
    let dependency_manifest_path = tmpdir.child("dependency/Cargo.toml");
    let dependency_manifest = r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "0.4.3"

[lib]
path = "dummy.rs"
"#;
    dependency_manifest_path
        .write_str(dependency_manifest)
        .expect("Manifest is writeable");

    execute_command(&["set-version", "2.0.0"], &dependency_manifest_path);
    let dependency_manifest = r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "2.0.0"

[lib]
path = "dummy.rs"
"#;

    primary_manifest_path.assert(primary_manifest);
    dependency_manifest_path.assert(dependency_manifest);
}

#[test]
fn test_compatible_dependency_ignored() {
    let tmpdir = assert_fs::TempDir::new().expect("failed to construct temporary directory");
    tmpdir
        .child("Cargo.toml")
        .write_str(
            r#"[workspace]
members = ["primary", "dependency"]
"#,
        )
        .expect("Manifest is writeable");
    let primary_manifest_path = tmpdir.child("primary/Cargo.toml");
    let primary_manifest = r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { version = "0.4", path = "../dependency" }
"#;
    primary_manifest_path
        .write_str(primary_manifest)
        .expect("Manifest is writeable");
    let dependency_manifest_path = tmpdir.child("dependency/Cargo.toml");
    let dependency_manifest = r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "0.4.3"

[lib]
path = "dummy.rs"
"#;
    dependency_manifest_path
        .write_str(dependency_manifest)
        .expect("Manifest is writeable");

    execute_command(&["set-version", "0.4.5"], &dependency_manifest_path);
    let dependency_manifest = r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "0.4.5"

[lib]
path = "dummy.rs"
"#;

    primary_manifest_path.assert(primary_manifest);
    dependency_manifest_path.assert(dependency_manifest);
}

#[test]
fn test_dependency_upgraded() {
    let tmpdir = assert_fs::TempDir::new().expect("failed to construct temporary directory");
    tmpdir
        .child("Cargo.toml")
        .write_str(
            r#"[workspace]
members = ["primary", "dependency"]
"#,
        )
        .expect("Manifest is writeable");
    let primary_manifest_path = tmpdir.child("primary/Cargo.toml");
    let primary_manifest = r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { version = "0.4", path = "../dependency" }
"#;
    primary_manifest_path
        .write_str(primary_manifest)
        .expect("Manifest is writeable");
    let dependency_manifest_path = tmpdir.child("dependency/Cargo.toml");
    let dependency_manifest = r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "0.4.3"

[lib]
path = "dummy.rs"
"#;
    dependency_manifest_path
        .write_str(dependency_manifest)
        .expect("Manifest is writeable");

    execute_command(&["set-version", "2.0.0"], &dependency_manifest_path);
    let primary_manifest = r#"[package]
name = "cargo-list-test-fixture"
version = "0.0.0"

[lib]
path = "dummy.rs"

[dependencies]
cargo-list-test-fixture-dependency = { version = "2.0", path = "../dependency" }
"#;
    let dependency_manifest = r#"[package]
name = "cargo-list-test-fixture-dependency"
version = "2.0.0"

[lib]
path = "dummy.rs"
"#;

    primary_manifest_path.assert(primary_manifest);
    dependency_manifest_path.assert(dependency_manifest);
}
