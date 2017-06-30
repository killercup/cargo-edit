#[macro_use]
extern crate assert_cli;

#[test]
fn invalid_manifest() {
    assert_cli!("target/debug/cargo-list",
                &["list", "--manifest-path=tests/fixtures/manifest-invalid/Cargo.toml.sample"] =>
                Error 1, r#"failed to parse datetime for key `invalid-section.key`"#)
        .unwrap();
}
