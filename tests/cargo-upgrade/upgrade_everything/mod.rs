use cargo_test_support::compare::assert_ui;
use cargo_test_support::file;
use cargo_test_support::Project;

use crate::CargoCommand;
use cargo_test_support::current_dir;

#[cargo_test]
fn case() {
    cargo_test_support::registry::init();
    crate::add_everything_registry_packages(false);
    crate::add_git_registry_packages();
    let project = Project::from_template(current_dir!().join("in"));
    let project_root = project.root();
    let cwd = &project_root;

    snapbox::cmd::Command::cargo_ui()
        .arg("upgrade")
        .args(["--pinned", "--incompatible"])
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_eq(file!["stdout.term.svg"])
        .stderr_eq(file!["stderr.term.svg"]);

    assert_ui().subset_matches(current_dir!().join("out"), &project_root);
}
