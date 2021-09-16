mod utils;

#[test]
fn invalid_manifest() {
    assert_cmd::Command::cargo_bin("cargo-add")
        .expect("can find bin")
        .args(&[
            "add",
            "foo",
            "--manifest-path=tests/fixtures/manifest-invalid/Cargo.toml.sample",
        ])
        .assert()
        .code(1)
        .stderr(
            r#"Command failed due to unhandled error: Unable to parse Cargo.toml

Caused by: Manifest not valid TOML
Caused by: TOML parse error at line 6, column 7
  |
6 | key = invalid-value
  |       ^
Unexpected `i`
Expected `-`, `+`, `inf`, `nan`, `0x`, `0o` or `0b`
invalid digit found in string
invalid digit found in string
While parsing a hexadecimal Integer
While parsing an octal Integer
While parsing a binary Integer
While parsing an Integer
While parsing a Float

"#,
        );
}
