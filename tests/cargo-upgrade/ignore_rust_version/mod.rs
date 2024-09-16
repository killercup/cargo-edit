use cargo_test_support::compare::assert_ui;
use cargo_test_support::file;
use cargo_test_support::Project;

use crate::CargoCommand;
use cargo_test_support::current_dir;

#[cargo_test]
fn case() {
    cargo_test_support::registry::Package::new("my-package", "0.1.1")
        .rust_version("1.60.0")
        .publish();
    cargo_test_support::registry::Package::new("my-package", "0.1.2")
        .rust_version("1.64.0")
        .publish();
    cargo_test_support::registry::Package::new("my-package", "0.1.3")
        .rust_version("1.68.0")
        .publish();
    cargo_test_support::registry::Package::new("my-package", "0.2.0")
        .rust_version("1.68.0")
        .publish();

    let project = Project::from_template(current_dir!().join("in"));
    let project_root = project.root();
    let cwd = &project_root;

    snapbox::cmd::Command::cargo_ui()
        .arg("upgrade")
        .args([
            "--verbose",
            "--verbose",
            "--pinned",
            "--incompatible",
            "--ignore-rust-version",
        ])
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_eq(file!["stdout.term.svg"])
        .stderr_eq(file!["stderr.term.svg"]);

    assert_ui().subset_matches(current_dir!().join("out"), &project_root);
}
