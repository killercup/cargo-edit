#![warn(rust_2018_idioms)]
#![allow(clippy::all)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::redundant_clone)]

#[macro_use]
extern crate cargo_test_macro;

pub fn cargo_exe() -> &'static std::path::Path {
    snapbox::cmd::cargo_bin!("cargo-add")
}

pub fn cargo_command() -> snapbox::cmd::Command {
    snapbox::cmd::Command::new(cargo_exe()).with_assert(assert())
}

pub fn project_from_template(template_path: impl AsRef<std::path::Path>) -> std::path::PathBuf {
    let root = cargo_test_support::paths::root();
    let project_root = root.join("case");
    snapbox::path::copy_template(template_path.as_ref(), &project_root).unwrap();
    project_root
}

pub fn assert() -> snapbox::Assert {
    let root = cargo_test_support::paths::root().display().to_string();

    let mut subs = snapbox::Substitutions::new();
    subs.extend([
        (
            "[EXE]",
            std::borrow::Cow::Borrowed(std::env::consts::EXE_SUFFIX),
        ),
        ("[ROOT]", std::borrow::Cow::Owned(root.into())),
    ])
    .unwrap();
    snapbox::Assert::new()
        .action_env(snapbox::DEFAULT_ACTION_ENV)
        .substitutions(subs)
}

#[cargo_test]
fn add_basic() {
    let project_root = project_from_template("tests/snapshots/add/add_basic.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/add_basic.stdout")
        .stderr_matches_path("tests/snapshots/add/add_basic.stderr");

    assert().subset_matches("tests/snapshots/add/add_basic.out", &project_root);
}

#[cargo_test]
fn add_multiple() {
    let project_root = project_from_template("tests/snapshots/add/add_multiple.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/add_multiple.stdout")
        .stderr_matches_path("tests/snapshots/add/add_multiple.stderr");

    assert().subset_matches("tests/snapshots/add/add_multiple.out", &project_root);
}

#[cargo_test]
#[cfg(feature = "test-external-apis")]
fn add_normalized_name_external() {
    let project_root = project_from_template("tests/snapshots/add/add_normalized_name_external.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["linked_hash_map", "Inflector"])
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/add_normalized_name_external.stdout")
        .stderr_matches_path("tests/snapshots/add/add_normalized_name_external.stderr");

    assert().subset_matches(
        "tests/snapshots/add/add_normalized_name_external.out",
        &project_root,
    );
}

#[cargo_test]
fn build() {
    let project_root = project_from_template("tests/snapshots/add/build.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["--build", "my-build-package1", "my-build-package2"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/build.stdout")
        .stderr_matches_path("tests/snapshots/add/build.stderr");

    assert().subset_matches("tests/snapshots/add/build.out", &project_root);
}

#[cargo_test]
fn build_prefer_existing_version() {
    let project_root =
        project_from_template("tests/snapshots/add/build_prefer_existing_version.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["cargo-list-test-fixture-dependency", "--build"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/build_prefer_existing_version.stdout")
        .stderr_matches_path("tests/snapshots/add/build_prefer_existing_version.stderr");

    assert().subset_matches(
        "tests/snapshots/add/build_prefer_existing_version.out",
        &project_root,
    );
}

#[cargo_test]
fn cargo_config_source_empty() {
    let project_root = project_from_template("tests/snapshots/add/cargo_config_source_empty.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/cargo_config_source_empty.stdout")
        .stderr_matches_path("tests/snapshots/add/cargo_config_source_empty.stderr");

    assert().subset_matches(
        "tests/snapshots/add/cargo_config_source_empty.out",
        &project_root,
    );
}

#[cargo_test]
fn default_features() {
    let project_root = project_from_template("tests/snapshots/add/default_features.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2@0.4.1", "--default-features"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/default_features.stdout")
        .stderr_matches_path("tests/snapshots/add/default_features.stderr");

    assert().subset_matches("tests/snapshots/add/default_features.out", &project_root);
}

#[cargo_test]
fn dev() {
    let project_root = project_from_template("tests/snapshots/add/dev.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["--dev", "my-dev-package1", "my-dev-package2"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/dev.stdout")
        .stderr_matches_path("tests/snapshots/add/dev.stderr");

    assert().subset_matches("tests/snapshots/add/dev.out", &project_root);
}

#[cargo_test]
fn dev_build_conflict() {
    let project_root = project_from_template("tests/snapshots/add/dev_build_conflict.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package", "--dev", "--build"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(2)
        .stdout_matches_path("tests/snapshots/add/dev_build_conflict.stdout")
        .stderr_matches_path("tests/snapshots/add/dev_build_conflict.stderr");

    assert().subset_matches("tests/snapshots/add/dev_build_conflict.out", &project_root);
}

#[cargo_test]
fn dev_prefer_existing_version() {
    let project_root = project_from_template("tests/snapshots/add/dev_prefer_existing_version.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["cargo-list-test-fixture-dependency", "--dev"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/dev_prefer_existing_version.stdout")
        .stderr_matches_path("tests/snapshots/add/dev_prefer_existing_version.stderr");

    assert().subset_matches(
        "tests/snapshots/add/dev_prefer_existing_version.out",
        &project_root,
    );
}

#[cargo_test]
fn dry_run() {
    let project_root = project_from_template("tests/snapshots/add/dry_run.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package", "--dry-run"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/dry_run.stdout")
        .stderr_matches_path("tests/snapshots/add/dry_run.stderr");

    assert().subset_matches("tests/snapshots/add/dry_run.out", &project_root);
}

#[cargo_test]
fn features() {
    let project_root = project_from_template("tests/snapshots/add/features.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["your-face", "--features", "eyes"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/features.stdout")
        .stderr_matches_path("tests/snapshots/add/features.stderr");

    assert().subset_matches("tests/snapshots/add/features.out", &project_root);
}

#[cargo_test]
fn features_empty() {
    let project_root = project_from_template("tests/snapshots/add/features_empty.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["your-face", "--features", ""])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/features_empty.stdout")
        .stderr_matches_path("tests/snapshots/add/features_empty.stderr");

    assert().subset_matches("tests/snapshots/add/features_empty.out", &project_root);
}

#[cargo_test]
fn features_multiple_occurrences() {
    let project_root =
        project_from_template("tests/snapshots/add/features_multiple_occurrences.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["your-face", "--features", "eyes", "--features", "nose"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/features_multiple_occurrences.stdout")
        .stderr_matches_path("tests/snapshots/add/features_multiple_occurrences.stderr");

    assert().subset_matches(
        "tests/snapshots/add/features_multiple_occurrences.out",
        &project_root,
    );
}

#[cargo_test]
fn features_preserve() {
    let project_root = project_from_template("tests/snapshots/add/features_preserve.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["your-face"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/features_preserve.stdout")
        .stderr_matches_path("tests/snapshots/add/features_preserve.stderr");

    assert().subset_matches("tests/snapshots/add/features_preserve.out", &project_root);
}

#[cargo_test]
fn features_spaced_values() {
    let project_root = project_from_template("tests/snapshots/add/features_spaced_values.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["your-face", "--features", "eyes nose"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/features_spaced_values.stdout")
        .stderr_matches_path("tests/snapshots/add/features_spaced_values.stderr");

    assert().subset_matches(
        "tests/snapshots/add/features_spaced_values.out",
        &project_root,
    );
}

#[cargo_test]
fn features_unknown() {
    let project_root = project_from_template("tests/snapshots/add/features_unknown.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["your-face", "--features", "noze"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/features_unknown.stdout")
        .stderr_matches_path("tests/snapshots/add/features_unknown.stderr");

    assert().subset_matches("tests/snapshots/add/features_unknown.out", &project_root);
}

#[cargo_test]
fn git() {
    let project_root = project_from_template("tests/snapshots/add/git.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "git-package",
            "--git",
            "http://localhost/git-package.git",
            "-Zgit",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/git.stdout")
        .stderr_matches_path("tests/snapshots/add/git.stderr");

    assert().subset_matches("tests/snapshots/add/git.out", &project_root);
}

#[cargo_test]
fn git_branch() {
    let project_root = project_from_template("tests/snapshots/add/git_branch.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "git-package",
            "--git",
            "http://localhost/git-package.git",
            "--branch",
            "main",
            "-Zgit",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/git_branch.stdout")
        .stderr_matches_path("tests/snapshots/add/git_branch.stderr");

    assert().subset_matches("tests/snapshots/add/git_branch.out", &project_root);
}

#[cargo_test]
fn git_conflicts_namever() {
    let project_root = project_from_template("tests/snapshots/add/git_conflicts_namever.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "my-package@0.4.3",
            "--git",
            "https://github.com/dcjanus/invalid",
            "-Zgit",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(1)
        .stdout_matches_path("tests/snapshots/add/git_conflicts_namever.stdout")
        .stderr_matches_path("tests/snapshots/add/git_conflicts_namever.stderr");

    assert().subset_matches(
        "tests/snapshots/add/git_conflicts_namever.out",
        &project_root,
    );
}

#[cargo_test]
fn git_conflicts_registry() {
    let project_root = project_from_template("tests/snapshots/add/git_conflicts_registry.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "my-package",
            "--git",
            "https://github.com/dcjanus/invalid",
            "--registry",
            "alternative",
            "-Zgit",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(2)
        .stdout_matches_path("tests/snapshots/add/git_conflicts_registry.stdout")
        .stderr_matches_path("tests/snapshots/add/git_conflicts_registry.stderr");

    assert().subset_matches(
        "tests/snapshots/add/git_conflicts_registry.out",
        &project_root,
    );
}

#[cargo_test]
fn git_dev() {
    let project_root = project_from_template("tests/snapshots/add/git_dev.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "git-package",
            "--git",
            "http://localhost/git-package.git",
            "--dev",
            "-Zgit",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/git_dev.stdout")
        .stderr_matches_path("tests/snapshots/add/git_dev.stderr");

    assert().subset_matches("tests/snapshots/add/git_dev.out", &project_root);
}

#[cargo_test]
#[cfg(feature = "test-external-apis")]
fn git_external() {
    let project_root = project_from_template("tests/snapshots/add/git_external.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "cargo-edit",
            "--git",
            "https://github.com/killercup/cargo-edit.git",
            "--tag",
            "v0.8.0",
            "-Zgit",
        ])
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/git_external.stdout")
        .stderr_matches_path("tests/snapshots/add/git_external.stderr");

    assert().subset_matches("tests/snapshots/add/git_external.out", &project_root);
}

#[cargo_test]
fn git_rev() {
    let project_root = project_from_template("tests/snapshots/add/git_rev.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "git-package",
            "--git",
            "http://localhost/git-package.git",
            "--rev",
            "423a3",
            "-Zgit",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/git_rev.stdout")
        .stderr_matches_path("tests/snapshots/add/git_rev.stderr");

    assert().subset_matches("tests/snapshots/add/git_rev.out", &project_root);
}

#[cargo_test]
fn git_tag() {
    let project_root = project_from_template("tests/snapshots/add/git_tag.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "git-package",
            "--git",
            "http://localhost/git-package.git",
            "--tag",
            "v1.0.0",
            "-Zgit",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/git_tag.stdout")
        .stderr_matches_path("tests/snapshots/add/git_tag.stderr");

    assert().subset_matches("tests/snapshots/add/git_tag.out", &project_root);
}

#[cargo_test]
fn inline_path() {
    let project_root = project_from_template("tests/snapshots/add/inline_path.in");
    let cwd = project_root.join("primary");

    cargo_command()
        .arg("add")
        .args(["../dependency"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(&cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/inline_path.stdout")
        .stderr_matches_path("tests/snapshots/add/inline_path.stderr");

    assert().subset_matches("tests/snapshots/add/inline_path.out", &project_root);
}

#[cargo_test]
fn inline_path_dev() {
    let project_root = project_from_template("tests/snapshots/add/inline_path_dev.in");
    let cwd = project_root.join("primary");

    cargo_command()
        .arg("add")
        .args(["../dependency", "--dev"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(&cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/inline_path_dev.stdout")
        .stderr_matches_path("tests/snapshots/add/inline_path_dev.stderr");

    assert().subset_matches("tests/snapshots/add/inline_path_dev.out", &project_root);
}

#[cargo_test]
fn invalid_arg() {
    let project_root = project_from_template("tests/snapshots/add/invalid_arg.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package", "--flag"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(2)
        .stdout_matches_path("tests/snapshots/add/invalid_arg.stdout")
        .stderr_matches_path("tests/snapshots/add/invalid_arg.stderr");

    assert().subset_matches("tests/snapshots/add/invalid_arg.out", &project_root);
}

#[cargo_test]
#[cfg(feature = "test-external-apis")]
fn invalid_git_external() {
    let project_root = project_from_template("tests/snapshots/add/invalid_git_external.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "fake-git",
            "--git",
            "https://github.com/killercup/fake-git-repo.git",
            "-Zgit",
        ])
        .current_dir(cwd)
        .assert()
        .code(1)
        .stdout_matches_path("tests/snapshots/add/invalid_git_external.stdout")
        .stderr_matches_path("tests/snapshots/add/invalid_git_external.stderr");

    assert().subset_matches(
        "tests/snapshots/add/invalid_git_external.out",
        &project_root,
    );
}

#[cargo_test]
fn invalid_git_no_unstable() {
    let project_root = project_from_template("tests/snapshots/add/invalid_git_no_unstable.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["git-package", "--git", "http://localhost/git-package.git"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(1)
        .stdout_matches_path("tests/snapshots/add/invalid_git_no_unstable.stdout")
        .stderr_matches_path("tests/snapshots/add/invalid_git_no_unstable.stderr");

    assert().subset_matches(
        "tests/snapshots/add/invalid_git_no_unstable.out",
        &project_root,
    );
}

#[cargo_test]
fn invalid_inline_path() {
    let project_root = project_from_template("tests/snapshots/add/invalid_inline_path.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["./tests/fixtures/local"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(1)
        .stdout_matches_path("tests/snapshots/add/invalid_inline_path.stdout")
        .stderr_matches_path("tests/snapshots/add/invalid_inline_path.stderr");

    assert().subset_matches("tests/snapshots/add/invalid_inline_path.out", &project_root);
}

#[cargo_test]
fn invalid_inline_path_self() {
    let project_root = project_from_template("tests/snapshots/add/invalid_inline_path_self.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["."])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(1)
        .stdout_matches_path("tests/snapshots/add/invalid_inline_path_self.stdout")
        .stderr_matches_path("tests/snapshots/add/invalid_inline_path_self.stderr");

    assert().subset_matches(
        "tests/snapshots/add/invalid_inline_path_self.out",
        &project_root,
    );
}

#[cargo_test]
fn invalid_manifest() {
    let project_root = project_from_template("tests/snapshots/add/invalid_manifest.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(1)
        .stdout_matches_path("tests/snapshots/add/invalid_manifest.stdout")
        .stderr_matches_path("tests/snapshots/add/invalid_manifest.stderr");

    assert().subset_matches("tests/snapshots/add/invalid_manifest.out", &project_root);
}

#[cargo_test]
#[cfg(feature = "test-external-apis")]
fn invalid_name_external() {
    let project_root = project_from_template("tests/snapshots/add/invalid_name_external.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["lets_hope_nobody_ever_publishes_this_crate"])
        .current_dir(cwd)
        .assert()
        .code(1)
        .stdout_matches_path("tests/snapshots/add/invalid_name_external.stdout")
        .stderr_matches_path("tests/snapshots/add/invalid_name_external.stderr");

    assert().subset_matches(
        "tests/snapshots/add/invalid_name_external.out",
        &project_root,
    );
}

#[cargo_test]
fn invalid_target_empty() {
    let project_root = project_from_template("tests/snapshots/add/invalid_target_empty.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package", "--target", ""])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(2)
        .stdout_matches_path("tests/snapshots/add/invalid_target_empty.stdout")
        .stderr_matches_path("tests/snapshots/add/invalid_target_empty.stderr");

    assert().subset_matches(
        "tests/snapshots/add/invalid_target_empty.out",
        &project_root,
    );
}

#[cargo_test]
fn invalid_vers() {
    let project_root = project_from_template("tests/snapshots/add/invalid_vers.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package@invalid version string"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(1)
        .stdout_matches_path("tests/snapshots/add/invalid_vers.stdout")
        .stderr_matches_path("tests/snapshots/add/invalid_vers.stderr");

    assert().subset_matches("tests/snapshots/add/invalid_vers.out", &project_root);
}

#[cargo_test]
fn list_features() {
    let project_root = project_from_template("tests/snapshots/add/list_features.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["your-face"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/list_features.stdout")
        .stderr_matches_path("tests/snapshots/add/list_features.stderr");

    assert().subset_matches("tests/snapshots/add/list_features.out", &project_root);
}

#[cargo_test]
fn list_features_path() {
    let project_root = project_from_template("tests/snapshots/add/list_features_path.in");
    let cwd = project_root.join("primary");

    cargo_command()
        .arg("add")
        .args(["cargo-list-test-fixture-dependency", "../dependency"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(&cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/list_features_path.stdout")
        .stderr_matches_path("tests/snapshots/add/list_features_path.stderr");

    assert().subset_matches("tests/snapshots/add/list_features_path.out", &project_root);
}

#[cargo_test]
fn list_features_path_no_default() {
    let project_root =
        project_from_template("tests/snapshots/add/list_features_path_no_default.in");
    let cwd = project_root.join("primary");

    cargo_command()
        .arg("add")
        .args([
            "cargo-list-test-fixture-dependency",
            "../dependency",
            "--no-default-features",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(&cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/list_features_path_no_default.stdout")
        .stderr_matches_path("tests/snapshots/add/list_features_path_no_default.stderr");

    assert().subset_matches(
        "tests/snapshots/add/list_features_path_no_default.out",
        &project_root,
    );
}

#[cargo_test]
fn manifest_path_package() {
    let project_root = project_from_template("tests/snapshots/add/manifest_path_package.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "--manifest-path",
            "Cargo.toml",
            "--package",
            "cargo-list-test-fixture",
            "cargo-list-test-fixture-dependency",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/manifest_path_package.stdout")
        .stderr_matches_path("tests/snapshots/add/manifest_path_package.stderr");

    assert().subset_matches(
        "tests/snapshots/add/manifest_path_package.out",
        &project_root,
    );
}

#[cargo_test]
fn multiple_conflicts_with_features() {
    let project_root =
        project_from_template("tests/snapshots/add/multiple_conflicts_with_features.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "your-face", "--features", "nose"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(1)
        .stdout_matches_path("tests/snapshots/add/multiple_conflicts_with_features.stdout")
        .stderr_matches_path("tests/snapshots/add/multiple_conflicts_with_features.stderr");

    assert().subset_matches(
        "tests/snapshots/add/multiple_conflicts_with_features.out",
        &project_root,
    );
}

#[cargo_test]
fn multiple_conflicts_with_git() {
    let project_root = project_from_template("tests/snapshots/add/multiple_conflicts_with_git.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "my-package1",
            "my-package2",
            "--git",
            "https://github.com/dcjanus/invalid",
            "-Zgit",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(1)
        .stdout_matches_path("tests/snapshots/add/multiple_conflicts_with_git.stdout")
        .stderr_matches_path("tests/snapshots/add/multiple_conflicts_with_git.stderr");

    assert().subset_matches(
        "tests/snapshots/add/multiple_conflicts_with_git.out",
        &project_root,
    );
}

#[cargo_test]
fn multiple_conflicts_with_rename() {
    let project_root =
        project_from_template("tests/snapshots/add/multiple_conflicts_with_rename.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2", "--rename", "renamed"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(1)
        .stdout_matches_path("tests/snapshots/add/multiple_conflicts_with_rename.stdout")
        .stderr_matches_path("tests/snapshots/add/multiple_conflicts_with_rename.stderr");

    assert().subset_matches(
        "tests/snapshots/add/multiple_conflicts_with_rename.out",
        &project_root,
    );
}

#[cargo_test]
fn namever() {
    let project_root = project_from_template("tests/snapshots/add/namever.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1@>=0.1.1", "my-package2@0.2.3", "my-package"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/namever.stdout")
        .stderr_matches_path("tests/snapshots/add/namever.stderr");

    assert().subset_matches("tests/snapshots/add/namever.out", &project_root);
}

#[cargo_test]
fn no_args() {
    let project_root = project_from_template("tests/snapshots/add/no_args.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .code(2)
        .stdout_matches_path("tests/snapshots/add/no_args.stdout")
        .stderr_matches_path("tests/snapshots/add/no_args.stderr");

    assert().subset_matches("tests/snapshots/add/no_args.out", &project_root);
}

#[cargo_test]
fn no_default_features() {
    let project_root = project_from_template("tests/snapshots/add/no_default_features.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2@0.4.1", "--no-default-features"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/no_default_features.stdout")
        .stderr_matches_path("tests/snapshots/add/no_default_features.stderr");

    assert().subset_matches("tests/snapshots/add/no_default_features.out", &project_root);
}

#[cargo_test]
fn no_optional() {
    let project_root = project_from_template("tests/snapshots/add/no_optional.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2@0.4.1", "--no-optional"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/no_optional.stdout")
        .stderr_matches_path("tests/snapshots/add/no_optional.stderr");

    assert().subset_matches("tests/snapshots/add/no_optional.out", &project_root);
}

#[cargo_test]
fn optional() {
    let project_root = project_from_template("tests/snapshots/add/optional.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2@0.4.1", "--optional"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/optional.stdout")
        .stderr_matches_path("tests/snapshots/add/optional.stderr");

    assert().subset_matches("tests/snapshots/add/optional.out", &project_root);
}

#[cargo_test]
fn overwrite_default_features() {
    let project_root = project_from_template("tests/snapshots/add/overwrite_default_features.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2@0.4.1", "--default-features"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_default_features.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_default_features.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_default_features.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_default_features_with_no_default_features() {
    let project_root = project_from_template(
        "tests/snapshots/add/overwrite_default_features_with_no_default_features.in",
    );
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2@0.4.1", "--no-default-features"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path(
            "tests/snapshots/add/overwrite_default_features_with_no_default_features.stdout",
        )
        .stderr_matches_path(
            "tests/snapshots/add/overwrite_default_features_with_no_default_features.stderr",
        );

    assert().subset_matches(
        "tests/snapshots/add/overwrite_default_features_with_no_default_features.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_features() {
    let project_root = project_from_template("tests/snapshots/add/overwrite_features.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["your-face", "--features", "nose"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_features.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_features.stderr");

    assert().subset_matches("tests/snapshots/add/overwrite_features.out", &project_root);
}

#[cargo_test]
fn overwrite_git_with_inline_path() {
    let project_root =
        project_from_template("tests/snapshots/add/overwrite_git_with_inline_path.in");
    let cwd = project_root.join("primary");

    cargo_command()
        .arg("add")
        .args(["../dependency"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(&cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_git_with_inline_path.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_git_with_inline_path.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_git_with_inline_path.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_inline_features() {
    let project_root = project_from_template("tests/snapshots/add/overwrite_inline_features.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "unrelateed-crate",
            "your-face",
            "+nose,mouth",
            "+ears",
            "-Zinline-add",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_inline_features.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_inline_features.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_inline_features.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_name_dev_noop() {
    let project_root = project_from_template("tests/snapshots/add/overwrite_name_dev_noop.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["cargo-list-test-fixture-dependency", "--dev"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_name_dev_noop.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_name_dev_noop.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_name_dev_noop.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_name_noop() {
    let project_root = project_from_template("tests/snapshots/add/overwrite_name_noop.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["cargo-list-test-fixture-dependency"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_name_noop.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_name_noop.stderr");

    assert().subset_matches("tests/snapshots/add/overwrite_name_noop.out", &project_root);
}

#[cargo_test]
fn overwrite_no_default_features() {
    let project_root =
        project_from_template("tests/snapshots/add/overwrite_no_default_features.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2@0.4.1", "--no-default-features"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_no_default_features.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_no_default_features.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_no_default_features.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_no_default_features_with_default_features() {
    let project_root = project_from_template(
        "tests/snapshots/add/overwrite_no_default_features_with_default_features.in",
    );
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2@0.4.1", "--default-features"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path(
            "tests/snapshots/add/overwrite_no_default_features_with_default_features.stdout",
        )
        .stderr_matches_path(
            "tests/snapshots/add/overwrite_no_default_features_with_default_features.stderr",
        );

    assert().subset_matches(
        "tests/snapshots/add/overwrite_no_default_features_with_default_features.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_no_optional() {
    let project_root = project_from_template("tests/snapshots/add/overwrite_no_optional.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2@0.4.1", "--no-optional"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_no_optional.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_no_optional.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_no_optional.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_no_optional_with_optional() {
    let project_root =
        project_from_template("tests/snapshots/add/overwrite_no_optional_with_optional.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2@0.4.1", "--optional"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_no_optional_with_optional.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_no_optional_with_optional.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_no_optional_with_optional.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_optional() {
    let project_root = project_from_template("tests/snapshots/add/overwrite_optional.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2@0.4.1", "--optional"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_optional.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_optional.stderr");

    assert().subset_matches("tests/snapshots/add/overwrite_optional.out", &project_root);
}

#[cargo_test]
fn overwrite_optional_with_no_optional() {
    let project_root =
        project_from_template("tests/snapshots/add/overwrite_optional_with_no_optional.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2@0.4.1", "--no-optional"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_optional_with_no_optional.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_optional_with_no_optional.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_optional_with_no_optional.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_path_noop() {
    let project_root = project_from_template("tests/snapshots/add/overwrite_path_noop.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["./dependency"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_path_noop.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_path_noop.stderr");

    assert().subset_matches("tests/snapshots/add/overwrite_path_noop.out", &project_root);
}

#[cargo_test]
fn overwrite_path_with_version() {
    let project_root = project_from_template("tests/snapshots/add/overwrite_path_with_version.in");
    let cwd = project_root.join("primary");

    cargo_command()
        .arg("add")
        .args(["cargo-list-test-fixture-dependency@20.0"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(&cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_path_with_version.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_path_with_version.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_path_with_version.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_rename_with_no_rename() {
    let project_root =
        project_from_template("tests/snapshots/add/overwrite_rename_with_no_rename.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["versioned-package"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_rename_with_no_rename.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_rename_with_no_rename.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_rename_with_no_rename.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_rename_with_rename() {
    let project_root = project_from_template("tests/snapshots/add/overwrite_rename_with_rename.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["versioned-package", "--rename", "a2"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_rename_with_rename.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_rename_with_rename.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_rename_with_rename.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_rename_with_rename_noop() {
    let project_root =
        project_from_template("tests/snapshots/add/overwrite_rename_with_rename_noop.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["versioned-package", "--rename", "a1"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_rename_with_rename_noop.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_rename_with_rename_noop.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_rename_with_rename_noop.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_version_with_git() {
    let project_root = project_from_template("tests/snapshots/add/overwrite_version_with_git.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["versioned-package", "--git", "git://git.git", "-Zgit"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_version_with_git.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_version_with_git.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_version_with_git.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_version_with_inline_path() {
    let project_root =
        project_from_template("tests/snapshots/add/overwrite_version_with_inline_path.in");
    let cwd = project_root.join("primary");

    cargo_command()
        .arg("add")
        .args(["../dependency"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(&cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_version_with_inline_path.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_version_with_inline_path.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_version_with_inline_path.out",
        &project_root,
    );
}

#[cargo_test]
fn overwrite_with_rename() {
    let project_root = project_from_template("tests/snapshots/add/overwrite_with_rename.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["versioned-package", "--rename", "renamed"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/overwrite_with_rename.stdout")
        .stderr_matches_path("tests/snapshots/add/overwrite_with_rename.stderr");

    assert().subset_matches(
        "tests/snapshots/add/overwrite_with_rename.out",
        &project_root,
    );
}

#[cargo_test]
fn preserve_sorted() {
    let project_root = project_from_template("tests/snapshots/add/preserve_sorted.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["toml"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/preserve_sorted.stdout")
        .stderr_matches_path("tests/snapshots/add/preserve_sorted.stderr");

    assert().subset_matches("tests/snapshots/add/preserve_sorted.out", &project_root);
}

#[cargo_test]
fn preserve_unsorted() {
    let project_root = project_from_template("tests/snapshots/add/preserve_unsorted.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["toml"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/preserve_unsorted.stdout")
        .stderr_matches_path("tests/snapshots/add/preserve_unsorted.stderr");

    assert().subset_matches("tests/snapshots/add/preserve_unsorted.out", &project_root);
}

#[cargo_test]
fn registry() {
    let project_root = project_from_template("tests/snapshots/add/registry.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2", "--registry", "alternative"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/registry.stdout")
        .stderr_matches_path("tests/snapshots/add/registry.stderr");

    assert().subset_matches("tests/snapshots/add/registry.out", &project_root);
}

#[cargo_test]
fn rename() {
    let project_root = project_from_template("tests/snapshots/add/rename.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package", "--rename", "renamed"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/rename.stdout")
        .stderr_matches_path("tests/snapshots/add/rename.stderr");

    assert().subset_matches("tests/snapshots/add/rename.out", &project_root);
}

#[cargo_test]
fn target() {
    let project_root = project_from_template("tests/snapshots/add/target.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args([
            "my-package1",
            "my-package2",
            "--target",
            "i686-unknown-linux-gnu",
        ])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/target.stdout")
        .stderr_matches_path("tests/snapshots/add/target.stderr");

    assert().subset_matches("tests/snapshots/add/target.out", &project_root);
}

#[cargo_test]
fn target_cfg() {
    let project_root = project_from_template("tests/snapshots/add/target_cfg.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package1", "my-package2", "--target", "cfg(unix)"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/target_cfg.stdout")
        .stderr_matches_path("tests/snapshots/add/target_cfg.stderr");

    assert().subset_matches("tests/snapshots/add/target_cfg.out", &project_root);
}

#[cargo_test]
fn vers() {
    let project_root = project_from_template("tests/snapshots/add/vers.in");
    let cwd = &project_root;

    cargo_command()
        .arg("add")
        .args(["my-package@>=0.1.1"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/vers.stdout")
        .stderr_matches_path("tests/snapshots/add/vers.stderr");

    assert().subset_matches("tests/snapshots/add/vers.out", &project_root);
}

#[cargo_test]
fn workspace_inline_path() {
    let project_root = project_from_template("tests/snapshots/add/workspace_inline_path.in");
    let cwd = project_root.join("primary");

    cargo_command()
        .arg("add")
        .args(["../dependency"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(&cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/workspace_inline_path.stdout")
        .stderr_matches_path("tests/snapshots/add/workspace_inline_path.stderr");

    assert().subset_matches(
        "tests/snapshots/add/workspace_inline_path.out",
        &project_root,
    );
}

#[cargo_test]
fn workspace_inline_path_dev() {
    let project_root = project_from_template("tests/snapshots/add/workspace_inline_path_dev.in");
    let cwd = project_root.join("primary");

    cargo_command()
        .arg("add")
        .args(["../dependency", "--dev"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(&cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/workspace_inline_path_dev.stdout")
        .stderr_matches_path("tests/snapshots/add/workspace_inline_path_dev.stderr");

    assert().subset_matches(
        "tests/snapshots/add/workspace_inline_path_dev.out",
        &project_root,
    );
}

#[cargo_test]
fn workspace_name() {
    let project_root = project_from_template("tests/snapshots/add/workspace_name.in");
    let cwd = project_root.join("primary");

    cargo_command()
        .arg("add")
        .args(["cargo-list-test-fixture-dependency"])
        .env("CARGO_IS_TEST", "1")
        .current_dir(&cwd)
        .assert()
        .success()
        .stdout_matches_path("tests/snapshots/add/workspace_name.stdout")
        .stderr_matches_path("tests/snapshots/add/workspace_name.stderr");

    assert().subset_matches("tests/snapshots/add/workspace_name.out", &project_root);
}
