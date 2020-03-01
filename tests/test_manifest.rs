mod utils;
use crate::utils::get_command_path;

#[test]
fn invalid_manifest() {
    assert_cli::Assert::command(&[
        get_command_path("add").as_str(),
        "add",
        "foo",
        "--manifest-path=tests/fixtures/manifest-invalid/Cargo.toml.sample",
    ])
    .fails_with(1)
    .and()
    .stderr()
    .contains("Command failed due to unhandled error: Unable to parse Cargo.toml")
    .unwrap();
}
