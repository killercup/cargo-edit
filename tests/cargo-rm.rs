extern crate assert_cli;

use assert_cli::assert_cli_output;

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
