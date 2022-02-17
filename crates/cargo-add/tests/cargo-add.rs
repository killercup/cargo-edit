#[test]
fn cli_tests() {
    let t = trycmd::TestCases::new();
    t.case("tests/cmd/add/*.toml");
    #[cfg(not(feature = "test-external-apis"))]
    {
        t.skip("tests/cmd/add/invalid_git_external.toml");
        t.skip("tests/cmd/add/invalid_name_external.toml");
        t.skip("tests/cmd/add/add_normalized_name_external.toml");
        t.skip("tests/cmd/add/git_external.toml");
    }
}
