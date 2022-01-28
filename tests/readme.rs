#[test]
fn cli_tests() {
    let t = trycmd::TestCases::new();
    t.case("README.md");
}
