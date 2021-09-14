mod utils;
use crate::utils::get_command_path;

#[test]
fn invalid_manifest() {
    assert_cli::Assert::command(&[
        get_command_path("add"),
        "add",
        "foo",
        "--manifest-path=tests/fixtures/manifest-invalid/Cargo.toml.sample",
    ])
    .fails_with(1)
    .and()
    .stderr()
    .is(
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
While parsing a Float"#,
    )
    .unwrap();
}
