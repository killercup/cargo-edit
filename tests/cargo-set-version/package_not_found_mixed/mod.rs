use cargo_test_support::Project;
use cargo_test_support::compare::assert_ui;
use cargo_test_support::current_dir;
use cargo_test_support::file;
use cargo_test_support::prelude::*;

use crate::CargoCommand;

#[cargo_test]
fn case() {
    let project = Project::from_template(current_dir!().join("in"));
    let project_root = project.root();

    snapbox::cmd::Command::cargo_ui()
        .arg("set-version")
        .args(["2.0.0", "--package", "sample", "--package", "missing"])
        .current_dir(&project_root)
        .assert()
        .code(0)
        .stdout_eq(file!["stdout.term.svg"])
        .stderr_eq(file!["stderr.term.svg"]);

    assert_ui().subset_matches(current_dir!().join("out"), &project_root);
}
