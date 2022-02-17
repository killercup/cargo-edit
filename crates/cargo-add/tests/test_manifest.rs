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
            r#"Error: Unable to parse Cargo.toml

Caused by:
    0: Manifest not valid TOML
    1: TOML parse error at line 6, column 7
         |
       6 | key = invalid-value
         |       ^
       Unexpected `v`
       
"#,
        );
}
