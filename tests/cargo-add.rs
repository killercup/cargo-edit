#[macro_use]
extern crate assert_cli;
extern crate tempdir;
extern crate toml;

use std::process;
mod utils;
use utils::{clone_out_test, execute_command, get_toml, no_manifest_failures};

#[test]
fn adds_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.my-package").is_none());

    execute_command(&["add", "my-package"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.my-package").unwrap();
    assert_eq!(val.as_str().unwrap(), "*");
}

#[test]
fn adds_dev_build_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dev-dependencies.my-dev-package").is_none());
    assert!(toml.lookup("build-dependencies.my-build-package").is_none());

    execute_command(&["add", "my-dev-package", "--dev"], &manifest);
    execute_command(&["add", "my-build-package", "--build"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dev-dependencies.my-dev-package").unwrap();
    assert_eq!(val.as_str().unwrap(), "*");
    let val = toml.lookup("build-dependencies.my-build-package").unwrap();
    assert_eq!(val.as_str().unwrap(), "*");

    // cannot run with both --dev and --build at the same time
    let call = process::Command::new("target/debug/cargo-add")
                   .args(&["add", "failure", "--dev", "--build"])
                   .arg(format!("--manifest-path={}", &manifest))
                   .output()
                   .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest)));
}

#[test]
fn adds_fixed_version() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.versioned-package").is_none());

    execute_command(&["add", "versioned-package", "--ver", ">=0.1.1"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.versioned-package").expect("not added");
    assert_eq!(val.as_str().expect("not string"), ">=0.1.1");

    // cannot run with both --dev and --build at the same time
    let call = process::Command::new("target/debug/cargo-add")
                   .args(&["add", "failure", "--ver", "invalid version string"])
                   .arg(format!("--manifest-path={}", &manifest))
                   .output()
                   .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest)));
}

#[test]
fn adds_git_source() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.git-package").is_none());

    execute_command(&["add", "git-package", "--git", "http://localhost/git-package.git"],
                    &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.git-package").unwrap();
    assert_eq!(val.as_table().unwrap().get("git").unwrap().as_str().unwrap(),
               "http://localhost/git-package.git");

    // check this works with other flags (e.g. --dev) as well
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dev-dependencies.git-dev-pkg").is_none());

    execute_command(&["add", "git-dev-pkg", "--git", "http://site/gp.git", "--dev"],
                    &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dev-dependencies.git-dev-pkg").unwrap();
    assert_eq!(val.as_table().unwrap().get("git").unwrap().as_str().unwrap(),
               "http://site/gp.git");
}

#[test]
fn adds_local_source() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.local").is_none());

    execute_command(&["add", "local", "--path", "/path/to/pkg"], &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.local").unwrap();
    assert_eq!(val.as_table().unwrap().get("path").unwrap().as_str().unwrap(),
               "/path/to/pkg");

    // check this works with other flags (e.g. --dev) as well
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dev-dependencies.local-dev").is_none());

    execute_command(&["add", "local-dev", "--path", "/path/to/pkg-dev", "--dev"],
                    &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dev-dependencies.local-dev").unwrap();
    assert_eq!(val.as_table().unwrap().get("path").unwrap().as_str().unwrap(),
               "/path/to/pkg-dev");
}

#[test]
fn package_kinds_are_mutually_exclusive() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let call = process::Command::new("target/debug/cargo-add")
                   .args(&["add", "failure"])
                   .args(&["--ver", "0.4.3"])
                   .args(&["--git", "git://git.git"])
                   .args(&["--path", "/path/here"])
                   .arg(format!("--manifest-path={}", &manifest))
                   .output()
                   .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest)));
}

#[test]
fn no_argument() {
    assert_cli!("target/debug/cargo-add", &["add"] => Error 1,
                r"Invalid arguments.

Usage:
    cargo add <crate> [--dev|--build] [--ver=<semver>|--git=<uri>|--path=<uri>] [options]
    cargo add (-h|--help)
    cargo add --version")
        .unwrap();
}

#[test]
fn unknown_flags() {
    assert_cli!("target/debug/cargo-add", &["add", "foo", "--flag"] => Error 1,
                r"Unknown flag: '--flag'

Usage:
    cargo add <crate> [--dev|--build] [--ver=<semver>|--git=<uri>|--path=<uri>] [options]
    cargo add (-h|--help)
    cargo add --version")
        .unwrap();
}
