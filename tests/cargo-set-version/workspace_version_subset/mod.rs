use cargo_test_support::Project;
use cargo_test_support::compare::assert_ui;
use cargo_test_support::file;

use crate::CargoCommand;
use crate::init_registry;
use cargo_test_support::current_dir;

#[cargo_test]
fn case() {
    init_registry();
    let project = Project::from_template(current_dir!().join("in"));
    let project_root = project.root();
    let cwd = &project_root;

    snapbox::cmd::Command::cargo_ui()
        .arg("set-version")
        .args(["2.0.0", "--package", "root"])
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_eq(file!["stdout.term.svg"])
        .stderr_eq(file!["stderr.term.svg"]);

    assert_ui().subset_matches(current_dir!().join("out"), &project_root);
}
