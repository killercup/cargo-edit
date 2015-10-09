extern crate assert_cli;

use assert_cli::assert_cli_output;

#[test]
fn listing() {
    assert_cli_output("target/debug/cargo-list",
                      &["list", "--manifest-path=tests/fixtures/list/Cargo.toml"],
                      r"clippy          git
docopt          0.6
pad             0.1
rustc-serialize 0.3
semver          0.1
toml            0.1")
        .unwrap();
}

#[test]
fn tree() {
    assert_cli_output("target/debug/cargo-list",
                      &["list", "--tree", "--manifest-path=tests/fixtures/tree/Cargo.lock"],
                      r"‣ clippy (0.0.5)
‣ docopt (0.6.67)
    ‣ regex (0.1.38)
        ‣ aho-corasick (0.2.1)
            ‣ memchr (0.1.3)
                ‣ libc (0.1.8)
        ‣ memchr (0.1.3)
            ‣ libc (0.1.8)
        ‣ regex-syntax (0.1.2)
    ‣ rustc-serialize (0.3.15)
    ‣ strsim (0.3.0)
‣ pad (0.1.4)
    ‣ unicode-width (0.1.1)
‣ rustc-serialize (0.3.15)
‣ semver (0.1.19)
‣ toml (0.1.20)
    ‣ rustc-serialize (0.3.15)")
        .unwrap();
}
