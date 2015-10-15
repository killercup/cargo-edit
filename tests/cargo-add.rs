extern crate tempdir;
extern crate toml;

use std::{fs, process};
use std::io::prelude::*;
use std::ffi::OsStr;

fn clone_out_test(source: &str) -> (tempdir::TempDir, String) {
    let tmpdir = tempdir::TempDir::new("cargo-add-test")
        .ok().expect("failed to construct temporary directory");
    fs::copy(source, tmpdir.path().join("Cargo.toml"))
        .unwrap_or_else(|err| panic!("could not copy test manifest: {}", err));
    let path = tmpdir.path().join("Cargo.toml").to_str().unwrap().to_string().clone();

    (tmpdir, path)
}

fn execute_command<S>(command: &[S], manifest: &str) where S: AsRef<OsStr> {
    let call = process::Command::new("target/debug/cargo-add")
        .args(command)
        .arg(format!("--manifest-path={}", manifest))
        .env("CARGO_IS_TEST", "1")
        .output().unwrap();

    if !call.status.success() {
        println!("Status code: {:?}", call.status);
        println!("STDOUT: {}", String::from_utf8_lossy(&call.stdout));
        println!("STDERR: {}", String::from_utf8_lossy(&call.stderr));
        panic!("cargo-add failed to execute")
    }
}

fn get_toml(manifest_path: &str) -> toml::Value {
    let mut f = fs::File::open(manifest_path).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    toml::Value::Table(toml::Parser::new(&s).parse().unwrap())
}

#[test]
fn adds_dependencies() {
    let (tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

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
    let (tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

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
        .output().unwrap();
    assert!(!call.status.success());

    // 'failure' dep not present
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.failure").is_none());
    assert!(toml.lookup("dev-dependencies.failure").is_none());
    assert!(toml.lookup("build-dependencies.failure").is_none());
}

#[test]
fn adds_fixed_version() {
    let (tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

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
        .output().unwrap();
    assert!(!call.status.success());

    // 'failure' dep not present
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.failure").is_none());
    assert!(toml.lookup("dev-dependencies.failure").is_none());
    assert!(toml.lookup("build-dependencies.failure").is_none());
}

#[test]
fn adds_git_source() {
    let (tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.git-package").is_none());

    execute_command(
        &["add", "git-package", "--git", "http://localhost/git-package.git"],
        &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.git-package").unwrap();
    assert_eq!(val.as_table().unwrap().get("git").unwrap().as_str().unwrap(),
        "http://localhost/git-package.git");

    // check this works with other flags (e.g. --dev) as well
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dev-dependencies.git-dev-pkg").is_none());

    execute_command(
        &["add", "git-dev-pkg", "--git", "http://site/gp.git", "--dev"],
        &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dev-dependencies.git-dev-pkg").unwrap();
    assert_eq!(val.as_table().unwrap().get("git").unwrap().as_str().unwrap(),
        "http://site/gp.git");
}

#[test]
fn adds_local_source() {
    let (tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

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

    execute_command(&["add", "local-dev", "--path", "/path/to/pkg-dev", "--dev"], &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dev-dependencies.local-dev").unwrap();
    assert_eq!(val.as_table().unwrap().get("path").unwrap().as_str().unwrap(),
        "/path/to/pkg-dev");
}

#[test]
fn package_kinds_are_mutually_exclusive() {
    let (tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let call = process::Command::new("target/debug/cargo-add")
        .args(&["add", "failure"])
        .args(&["--ver", "0.4.3"])
        .args(&["--git", "git://git.git"])
        .args(&["--path", "/path/here"])
        .arg(format!("--manifest-path={}", &manifest))
        .output().unwrap();
    assert!(!call.status.success());

    // 'failure' dep not present
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.failure").is_none());
    assert!(toml.lookup("dev-dependencies.failure").is_none());
    assert!(toml.lookup("build-dependencies.failure").is_none());
}
