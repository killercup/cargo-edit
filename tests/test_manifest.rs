extern crate assert_cli;

#[test]
fn invalid_manifest() {
    assert_cli::Assert::command(&[
        "target/debug/cargo-add",
        "add",
        "foo",
        "--manifest-path=tests/fixtures/manifest-invalid/Cargo.toml.sample",
    ]).fails_with(1)
        .prints_error_exactly(
            r"Command failed due to unhandled error: Unable to parse Cargo.toml

Caused by: Manifest not valid TOML
Caused by: failed to parse datetime for key `invalid-section.key`",
        )
        .unwrap();
}
