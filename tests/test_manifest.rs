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
            "Command failed due to unhandled error: \
             failed to parse datetime for key `invalid-section.key`",
        )
        .unwrap();
}
