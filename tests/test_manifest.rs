extern crate assert_cli;

#[test]
fn invalid_manifest() {
    assert_cli::Assert::command(&[
        "target/debug/cargo-add",
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
Expected `digit`, `-` or `+`
expected 4 more elements
expected 2 more elements
While parsing a Time
While parsing a Date-Time
While parsing a Float
While parsing an Integer"#,
    )
    .unwrap();
}
