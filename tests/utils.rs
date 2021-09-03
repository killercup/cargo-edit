#![allow(unused)]
use std::ffi::{OsStr, OsString};
use std::io::prelude::*;
use std::{env, fs, path::Path, path::PathBuf, process};

/// Helper function that copies the workspace test into a temporary directory.
pub fn copy_workspace_test() -> (tempfile::TempDir, String, Vec<String>) {
    // Create a temporary directory and copy in the root manifest, the dummy rust file, and
    // workspace member manifests.
    let tmpdir = tempfile::tempdir().expect("failed to construct temporary directory");

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
pub fn clone_out_test(source: &str) -> (tempfile::TempDir, String) {
    let tmpdir = tempfile::tempdir().expect("failed to construct temporary directory");
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
    let subcommand_name = &command[0].as_ref();

    let call = process::Command::new(&get_command_path(subcommand_name))
        .args(command)
        .arg(format!("--manifest-path={}", manifest))
        .env("CARGO_IS_TEST", "1")
        .output()
        .unwrap();

    if call.status.success() {
        println!("Status code: {:?}", call.status);
        println!("STDOUT: {}", String::from_utf8_lossy(&call.stdout));
        println!("STDERR: {}", String::from_utf8_lossy(&call.stderr));
        panic!(
            "cargo-{} success to execute",
            subcommand_name.to_string_lossy()
        )
    }
}

/// Execute local cargo command, includes `--package`
pub fn execute_command_for_pkg<S, P>(command: &[S], pkgid: &str, cwd: P)
where
    S: AsRef<OsStr>,
    P: AsRef<Path>,
{
    let subcommand_name = &command[0].as_ref();
    let cwd = cwd.as_ref();

    let call = process::Command::new(&get_command_path(subcommand_name))
        .args(command)
        .arg("--package")
        .arg(pkgid)
        .current_dir(&cwd)
        .env("CARGO_IS_TEST", "1")
        .output()
        .expect("call to test command failed");

    if !call.status.success() {
        println!("Status code: {:?}", call.status);
        println!("STDOUT: {}", String::from_utf8_lossy(&call.stdout));
        println!("STDERR: {}", String::from_utf8_lossy(&call.stderr));
        panic!(
            "cargo-{} failed to execute",
            subcommand_name.to_string_lossy()
        )
    }
}

/// Execute local cargo command, includes `--manifest-path`
pub fn execute_command<S>(command: &[S], manifest: &str)
where
    S: AsRef<OsStr>,
{
    let subcommand_name = &command[0].as_ref();

    let call = process::Command::new(&get_command_path(subcommand_name))
        .args(command)
        .arg(format!("--manifest-path={}", manifest))
        .env("CARGO_IS_TEST", "1")
        .output()
        .expect("call to test build failed");

    if !call.status.success() {
        println!("Status code: {:?}", call.status);
        println!("STDOUT: {}", String::from_utf8_lossy(&call.stdout));
        println!("STDERR: {}", String::from_utf8_lossy(&call.stderr));
        panic!(
            "cargo-{} failed to execute",
            subcommand_name.to_string_lossy()
        )
    }
}

/// Execute local cargo command in a given directory
pub fn execute_command_in_dir<S>(command: &[S], dir: &Path)
where
    S: AsRef<OsStr>,
{
    let subcommand_name = &command[0].as_ref();

    let call = process::Command::new(&get_command_path(subcommand_name))
        .args(command)
        .env("CARGO_IS_TEST", "1")
        .current_dir(dir)
        .output()
        .expect("call to test build failed");

    if !call.status.success() {
        println!("Status code: {:?}", call.status);
        println!("STDOUT: {}", String::from_utf8_lossy(&call.stdout));
        println!("STDERR: {}", String::from_utf8_lossy(&call.stderr));
        panic!(
            "cargo-{} failed to execute",
            subcommand_name.to_string_lossy()
        )
    }
}

/// Parse a manifest file as TOML
pub fn get_toml(manifest_path: &str) -> toml_edit::Document {
    let mut f = fs::File::open(manifest_path).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    s.parse().expect("toml parse error")
}

pub fn get_command_path(s: impl AsRef<OsStr>) -> &'static str {
    match s.as_ref().to_str() {
        Some("add") => env!("CARGO_BIN_EXE_cargo-add"),
        Some("rm") => env!("CARGO_BIN_EXE_cargo-rm"),
        Some("upgrade") => env!("CARGO_BIN_EXE_cargo-upgrade"),
        _ => panic!("Unsupported subcommand"),
    }
}
