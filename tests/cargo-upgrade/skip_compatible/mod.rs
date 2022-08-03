use cargo_test_support::compare::assert_ui;
use cargo_test_support::Project;

use crate::CargoCommand;
use cargo_test_support::curr_dir;

#[cargo_test]
fn case() {
    cargo_test_support::registry::init();
    crate::add_breaking_registry_packages(false);
    let project = Project::from_template(curr_dir!().join("in"));
    let project_root = project.root();
    let cwd = &project_root;

    snapbox::cmd::Command::cargo_ui()
        .arg("upgrade")
        .args(["--verbose"])
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path(curr_dir!().join("stdout.log"))
        .stderr_matches_path(curr_dir!().join("stderr.log"));

    assert_ui().subset_matches(curr_dir!().join("out"), &project_root);
}
