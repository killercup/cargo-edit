#[macro_use]
extern crate assert_cli;

#[test]
fn invalid_manifest() {
    assert_cli!("target/debug/cargo-add",
                &["add", "foo", "--manifest-path=tests/fixtures/manifest-invalid/Cargo.toml.sample"] =>
                Error 1, "Command failed due to unhandled error: failed to parse datetime for key `invalid-section.key`")
        .unwrap();
}
