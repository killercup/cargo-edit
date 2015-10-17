#[macro_use] extern crate assert_cli;

mod utils;
use utils::{clone_out_test, execute_command, get_toml};

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
fn no_argument() {
    assert_cli!("target/debug/cargo-rm", &["rm"] => Error 1,
                r"Invalid arguments.

Usage:
    cargo rm <crate> [--dev|--build] [options]
    cargo rm (-h|--help)
    cargo rm --version")
        .unwrap();
}

#[test]
fn unknown_flags() {
    assert_cli!("target/debug/cargo-rm", &["rm", "foo", "--flag"] => Error 1,
                r"Unknown flag: '--flag'

Usage:
    cargo rm <crate> [--dev|--build] [options]
    cargo rm (-h|--help)
    cargo rm --version")
        .unwrap();
}
