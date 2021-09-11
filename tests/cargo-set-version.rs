#[macro_use]
extern crate pretty_assertions;

mod utils;
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
