#![allow(unused)]
use assert_cmd::Command;
use std::ffi::{OsStr, OsString};
use std::io::prelude::*;
use std::{env, fs, path::Path, path::PathBuf, process};

/// Helper function that copies the workspace test into a temporary directory.
pub fn copy_workspace_test() -> (assert_fs::TempDir, String, Vec<String>) {
    // Create a temporary directory and copy in the root manifest, the dummy rust file, and
    // workspace member manifests.
    let tmpdir = assert_fs::TempDir::new().expect("failed to construct temporary directory");

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
            )
            .unwrap_or_else(|err| panic!("could not copy test file: {}", err));

            file_path
        };

        let root_manifest_path = copy_in(".", "Cargo.toml");
        copy_in(".", "dummy.rs");
        copy_in(".", "Cargo.lock");

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

/// Create temporary working directory with Cargo.toml manifest
pub fn clone_out_test(source: &str) -> (assert_fs::TempDir, String) {
    let tmpdir = assert_fs::TempDir::new().expect("failed to construct temporary directory");
    fs::copy(source, tmpdir.path().join("Cargo.toml"))
        .unwrap_or_else(|err| panic!("could not copy test manifest: {}", err));
    let path = tmpdir
        .path()
        .join("Cargo.toml")
        .to_str()
        .unwrap()
        .to_string()
        .clone();

    (tmpdir, path)
}

/// Add directory
pub fn setup_alt_registry_config(path: &std::path::Path) {
    fs::create_dir(path.join(".cargo")).expect("failed to create .cargo directory");
    fs::copy(
        "tests/fixtures/alt-registry-cargo-config",
        path.join(".cargo").join("config"),
    )
    .unwrap_or_else(|err| panic!("could not copy test cargo config: {}", err));
}

/// Execute local cargo command, includes `--manifest-path`, expect command failed
pub fn execute_bad_command<S>(command: &[S], manifest: &str)
where
    S: AsRef<OsStr>,
{
    let subcommand_name = format!("cargo-{}", command[0].as_ref().to_str().unwrap());

    let call = Command::cargo_bin(&subcommand_name)
        .expect("can find bin")
        .args(command)
        .arg(format!("--manifest-path={}", manifest))
        .env("CARGO_IS_TEST", "1")
        .assert()
        .failure();
}

/// Execute local cargo command, includes `--package`
pub fn execute_command_for_pkg<S, P>(command: &[S], pkgid: &str, cwd: P)
where
    S: AsRef<OsStr>,
    P: AsRef<Path>,
{
    let subcommand_name = format!("cargo-{}", command[0].as_ref().to_str().unwrap());
    let cwd = cwd.as_ref();

    let assert = Command::cargo_bin(&subcommand_name)
        .expect("can find bin")
        .args(command)
        .arg("--package")
        .arg(pkgid)
        .current_dir(&cwd)
        .env("CARGO_IS_TEST", "1")
        .assert()
        .success();
    println!("Succeeded: {}", assert);
}

/// Execute local cargo command, includes `--manifest-path`
pub fn execute_command<S, P>(command: &[S], manifest: P)
where
    S: AsRef<OsStr>,
    P: AsRef<Path>,
{
    let subcommand_name = format!("cargo-{}", command[0].as_ref().to_str().unwrap());

    let assert = Command::cargo_bin(&subcommand_name)
        .expect("can find bin")
        .args(command)
        .arg("--manifest-path")
        .arg(manifest.as_ref())
        .env("CARGO_IS_TEST", "1")
        .assert()
        .success();
    println!("Succeeded: {}", assert);
}

/// Execute local cargo command in a given directory
pub fn execute_command_in_dir<S>(command: &[S], dir: &Path)
where
    S: AsRef<OsStr>,
{
    let subcommand_name = format!("cargo-{}", command[0].as_ref().to_str().unwrap());

    let call = Command::cargo_bin(&subcommand_name)
        .expect("can find bin")
        .args(command)
        .env("CARGO_IS_TEST", "1")
        .current_dir(dir)
        .assert()
        .success();
}

/// Parse a manifest file as TOML
pub fn get_toml(manifest_path: &str) -> toml_edit::Document {
    let mut f = fs::File::open(manifest_path).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    s.parse().expect("toml parse error")
}
