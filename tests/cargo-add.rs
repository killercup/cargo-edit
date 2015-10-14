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
