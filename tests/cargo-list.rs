#[macro_use]
extern crate assert_cli;

#[test]
fn listing() {
    assert_cli!("target/debug/cargo-list",
                &["list", "--manifest-path=tests/fixtures/list/Cargo.toml"] =>
                Success, r"clippy          git
docopt          0.6
pad             0.1
rustc-serialize 0.3
semver          0.1
toml            0.1")
        .unwrap();
}

#[test]
fn tree() {
    assert_cli!("target/debug/cargo-list",
                &["list", "--tree", "--manifest-path=tests/fixtures/tree/Cargo.lock"] =>
                Success, r"├── clippy (0.0.5)
├── docopt (0.6.67)
│   ├── regex (0.1.38)
│   │   ├── aho-corasick (0.2.1)
│   │   │   └── memchr (0.1.3)
│   │   │       └── libc (0.1.8)
│   │   ├── memchr (0.1.3)
│   │   │   └── libc (0.1.8)
│   │   └── regex-syntax (0.1.2)
│   ├── rustc-serialize (0.3.15)
│   └── strsim (0.3.0)
├── pad (0.1.4)
│   └── unicode-width (0.1.1)
├── rustc-serialize (0.3.15)
├── semver (0.1.19)
└── toml (0.1.20)
    └── rustc-serialize (0.3.15)")
        .unwrap();
}

#[test]
fn unknown_flags() {
    assert_cli!("target/debug/cargo-list", &["list", "foo", "--flag"] => Error 1,
                r"Unknown flag: '--flag'

Usage:
    cargo list [<section>] [options]
    cargo list (-h|--help)
    cargo list --version")
        .unwrap();
}
