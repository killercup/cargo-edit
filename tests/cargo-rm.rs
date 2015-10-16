extern crate assert_cli;

use assert_cli::assert_cli_output;

mod utils;
use utils::*;

// https://github.com/killercup/cargo-edit/issues/32
#[test]
fn issue_32() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/rm/Cargo.toml");

    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.foo").is_none());

    execute_command(&["add", "foo@1.0"], &manifest);
    execute_command(&["add", "bar@1.0.7"], &manifest);

    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.foo").is_some());
    assert!(toml.lookup("dependencies.bar").is_some());

    execute_command(&["rm", "foo"], &manifest);
    execute_command(&["rm", "bar"], &manifest);

    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.foo").is_none());
    assert!(toml.lookup("dependencies.bar").is_none());
}





#[test]
#[ignore]
fn no_argument() {
    assert_cli_output("target/debug/cargo-rm",
                      &["rm"],
                      r"Invalid arguments.
Usage:
    cargo rm <crate> [--dev|--build] [options]
    cargo rm (-h|--help)
    cargo rm --version")
        .unwrap();
}

#[test]
#[ignore]
fn unknown_flags() {
    assert_cli_output("target/debug/cargo-rm",
                      &["rm", "foo", "--flag"],
                      r"Unknown flag: '--flag'

Usage:
    cargo rm <crate> [--dev|--build] [options]
    cargo rm (-h|--help)
    cargo rm --version")
        .unwrap();
}
