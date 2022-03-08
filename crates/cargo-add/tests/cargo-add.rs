#[test]
fn cli_tests() {
    let t = trycmd::TestCases::new();
    t.case("tests/snapshots/add/*.toml");
    #[cfg(not(feature = "test-external-apis"))]
    {
        t.skip("tests/snapshots/add/invalid_git_external.toml");
        t.skip("tests/snapshots/add/invalid_name_external.toml");
        t.skip("tests/snapshots/add/add_normalized_name_external.toml");
        t.skip("tests/snapshots/add/git_external.toml");
    }
}
