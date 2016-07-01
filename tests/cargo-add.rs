#[macro_use]
extern crate assert_cli;
extern crate tempdir;
extern crate toml;

use std::process;
mod utils;
use utils::{clone_out_test, execute_command, get_toml, no_manifest_failures};

#[test]
fn adds_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.my-package").is_none());

    execute_command(&["add", "my-package"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.my-package").unwrap();
    assert_eq!(val.as_str().unwrap(), "CURRENT_VERSION_TEST");
}

fn upgrade_test_helper(upgrade_method : &str , expected_prefix : &str) {
	let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.my-package").is_none());

	let upgrade_arg = format!("--upgrade={0}", upgrade_method); 
	
    execute_command(&["add", "my-package", upgrade_arg.as_str()], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.my-package").unwrap();
    
	let expected_result = format!("{0}CURRENT_VERSION_TEST", expected_prefix);
    assert_eq!(val.as_str().unwrap(), expected_result);
}

#[test]
fn adds_dependency_with_upgrade_none() {
    upgrade_test_helper("none", "=");
}
#[test]
fn adds_dependency_with_upgrade_patch() {
    upgrade_test_helper("patch", "~");
}
#[test]
fn adds_dependency_with_upgrade_minor() {
    upgrade_test_helper("minor", "^");
}
#[test]
fn adds_dependency_with_upgrade_all() {
    upgrade_test_helper("all", ">=");
}

#[test]
fn adds_multiple_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.my-package1").is_none());
    assert!(toml.lookup("dependencies.my-package2").is_none());

    execute_command(&["add", "my-package1", "my-package2"], &manifest);

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.my-package1").unwrap();
    assert_eq!(val.as_str().unwrap(), "CURRENT_VERSION_TEST");
    let val = toml.lookup("dependencies.my-package2").unwrap();
    assert_eq!(val.as_str().unwrap(), "CURRENT_VERSION_TEST");
}

#[test]
fn adds_dev_build_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dev-dependencies.my-dev-package").is_none());
    assert!(toml.lookup("build-dependencies.my-build-package").is_none());

    execute_command(&["add", "my-dev-package", "--dev"], &manifest);
    execute_command(&["add", "my-build-package", "--build"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dev-dependencies.my-dev-package").unwrap();
    assert_eq!(val.as_str().unwrap(), "CURRENT_VERSION_TEST");
    let val = toml.lookup("build-dependencies.my-build-package").unwrap();
    assert_eq!(val.as_str().unwrap(), "CURRENT_VERSION_TEST");

    // cannot run with both --dev and --build at the same time
    let call = process::Command::new("target/debug/cargo-add")
                   .args(&["add", "failure", "--dev", "--build"])
                   .arg(format!("--manifest-path={}", &manifest))
                   .output()
                   .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest)));
}

#[test]
fn adds_multiple_dev_build_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dev-dependencies.my-dev-package1").is_none());
    assert!(toml.lookup("dev-dependencies.my-dev-package2").is_none());
    assert!(toml.lookup("build-dependencies.my-build-package1").is_none());
    assert!(toml.lookup("build-dependencies.my-build-package2").is_none());

    execute_command(&["add", "my-dev-package1", "my-dev-package2", "--dev"], &manifest);
    execute_command(&["add", "my-build-package1", "--build", "my-build-package2"], &manifest);

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dev-dependencies.my-dev-package1").unwrap();
    assert_eq!(val.as_str().unwrap(), "CURRENT_VERSION_TEST");
    let val = toml.lookup("dev-dependencies.my-dev-package2").unwrap();
    assert_eq!(val.as_str().unwrap(), "CURRENT_VERSION_TEST");
    let val = toml.lookup("build-dependencies.my-build-package1").unwrap();
    assert_eq!(val.as_str().unwrap(), "CURRENT_VERSION_TEST");
    let val = toml.lookup("build-dependencies.my-build-package2").unwrap();
    assert_eq!(val.as_str().unwrap(), "CURRENT_VERSION_TEST");
}

#[test]
fn adds_specified_version() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.versioned-package").is_none());

    execute_command(&["add", "versioned-package", "--vers", ">=0.1.1"],
                    &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.versioned-package").expect("not added");
    assert_eq!(val.as_str().expect("not string"), ">=0.1.1");

    // cannot run with both --dev and --build at the same time
    let call = process::Command::new("target/debug/cargo-add")
                   .args(&["add", "failure", "--vers", "invalid version string"])
                   .arg(format!("--manifest-path={}", &manifest))
                   .output()
                   .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest)));
}

#[test]
fn adds_specified_version_with_inline_notation() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.versioned-package").is_none());

    execute_command(&["add", "versioned-package@>=0.1.1"], &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.versioned-package").expect("not added");
    assert_eq!(val.as_str().expect("not string"), ">=0.1.1");
}

#[test]
fn adds_multiple_dependencies_with_versions() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.my-package1").is_none());
    assert!(toml.lookup("dependencies.my-package2").is_none());

    execute_command(&["add", "my-package1@>=0.1.1", "my-package2@0.2.3"], &manifest);

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.my-package1").expect("not added");
    assert_eq!(val.as_str().expect("not string"), ">=0.1.1");
    let val = toml.lookup("dependencies.my-package2").expect("not added");
    assert_eq!(val.as_str().expect("not string"), "0.2.3");
}

#[test]
fn adds_multiple_dependencies_with_some_versions() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.my-package1").is_none());
    assert!(toml.lookup("dependencies.my-package2").is_none());

    execute_command(&["add", "my-package1", "my-package2@0.2.3"], &manifest);

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.my-package1").expect("not added");
    assert_eq!(val.as_str().expect("not string"), "CURRENT_VERSION_TEST");
    let val = toml.lookup("dependencies.my-package2").expect("not added");
    assert_eq!(val.as_str().expect("not string"), "0.2.3");
}

#[test]
fn adds_git_source_using_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.git-package").is_none());

    execute_command(&["add", "git-package", "--git", "http://localhost/git-package.git"],
                    &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.git-package").unwrap();
    assert_eq!(val.as_table().unwrap().get("git").unwrap().as_str().unwrap(),
               "http://localhost/git-package.git");

    // check this works with other flags (e.g. --dev) as well
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dev-dependencies.git-dev-pkg").is_none());

    execute_command(&["add", "git-dev-pkg", "--git", "http://site/gp.git", "--dev"],
                    &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dev-dependencies.git-dev-pkg").unwrap();
    assert_eq!(val.as_table().unwrap().get("git").unwrap().as_str().unwrap(),
               "http://site/gp.git");
}

#[test]
fn adds_local_source_using_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.local").is_none());

    execute_command(&["add", "local", "--path", "/path/to/pkg"], &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.local").unwrap();
    assert_eq!(val.as_table().unwrap().get("path").unwrap().as_str().unwrap(),
               "/path/to/pkg");

    // check this works with other flags (e.g. --dev) as well
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dev-dependencies.local-dev").is_none());

    execute_command(&["add", "local-dev", "--path", "/path/to/pkg-dev", "--dev"],
                    &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dev-dependencies.local-dev").unwrap();
    assert_eq!(val.as_table().unwrap().get("path").unwrap().as_str().unwrap(),
               "/path/to/pkg-dev");
}

#[test]
#[cfg(feature="test-external-apis")]
fn adds_git_source_without_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.cargo-edit").is_none());
    
    execute_command(&["add", "https://github.com/killercup/cargo-edit.git"],
                    &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.cargo-edit").unwrap();
    assert_eq!(val.as_table().unwrap().get("git").unwrap().as_str().unwrap(),
            "https://github.com/killercup/cargo-edit.git");

    // check this works with other flags (e.g. --dev) as well
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dev-dependencies.cargo-edit").is_none());

    execute_command(&["add", "https://github.com/killercup/cargo-edit.git", "--dev"],
                    &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dev-dependencies.cargo-edit").unwrap();
    assert_eq!(val.as_table().unwrap().get("git").unwrap().as_str().unwrap(),
            "https://github.com/killercup/cargo-edit.git");
}

#[test]
fn adds_local_source_without_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");
    
    let (tmpdir, _) = clone_out_test("tests/fixtures/add/local/Cargo.toml.sample");
    let tmppath = tmpdir.into_path();
    let tmpdirstr = tmppath.to_str().unwrap();
    
    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.foo-crate").is_none());

    execute_command(&["add", tmpdirstr], &manifest);
    
    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.foo-crate").unwrap();
    assert_eq!(val.as_table().unwrap().get("path").unwrap().as_str().unwrap(),
               tmpdirstr);

    // check this works with other flags (e.g. --dev) as well
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dev-dependencies.foo-crate").is_none());

    execute_command(&["add",  tmpdirstr, "--dev"],
                    &manifest);

    let toml = get_toml(&manifest);
    let val = toml.lookup("dev-dependencies.foo-crate").unwrap();
    assert_eq!(val.as_table().unwrap().get("path").unwrap().as_str().unwrap(),
                tmpdirstr);
}

#[test]
fn package_kinds_are_mutually_exclusive() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    let call = process::Command::new("target/debug/cargo-add")
                   .args(&["add", "failure"])
                   .args(&["--vers", "0.4.3"])
                   .args(&["--git", "git://git.git"])
                   .args(&["--path", "/path/here"])
                   .arg(format!("--manifest-path={}", &manifest))
                   .output()
                   .unwrap();

    assert!(!call.status.success());
    assert!(no_manifest_failures(&get_toml(&manifest)));
}

#[test]
fn adds_optional_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.versioned-package").is_none());

    execute_command(&["add", "versioned-package", "--vers", ">=0.1.1", "--optional"],
                    &manifest);

    // dependency present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.versioned-package.optional").expect("not added optionally");
    assert_eq!(val.as_bool().expect("optional not a bool"), true);
}

#[test]
fn adds_multiple_optional_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.my-package1").is_none());
    assert!(toml.lookup("dependencies.my-package2").is_none());

    execute_command(&["add", "--optional", "my-package1", "my-package2"],
                    &manifest);

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    let val = toml.lookup("dependencies.my-package1.optional").expect("not added optionally");
    assert_eq!(val.as_bool().expect("optional not a bool"), true);
    let val = toml.lookup("dependencies.my-package2.optional").expect("not added optionally");
    assert_eq!(val.as_bool().expect("optional not a bool"), true);
}

#[test]
fn adds_dependency_with_target_triple() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("target.i686-unknown-linux-gnu.dependencies.my-package1").is_none());

    execute_command(&["add", "--target", "i686-unknown-linux-gnu", "my-package1"],
                    &manifest);

    // dependencies present afterwards
    let toml = get_toml(&manifest);

    let val = toml.lookup("target.i686-unknown-linux-gnu.dependencies.my-package1")
                  .expect("target dependency not added");
    assert_eq!(val.as_str().unwrap(), "CURRENT_VERSION_TEST");
}

#[test]
fn adds_dependency_with_target_cfg() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("target.i686-unknown-linux-gnu.dependencies.my-package1").is_none());

    execute_command(&["add", "--target", "'cfg(unix)'", "my-package1"],
                    &manifest);

    // dependencies present afterwards
    let toml = get_toml(&manifest);

    let val = toml.lookup("target.'cfg(unix)'.dependencies.my-package1")
                  .expect("target dependency not added");
    assert_eq!(val.as_str().unwrap(), "CURRENT_VERSION_TEST");
}

#[test]
fn adds_dependency_with_custom_target() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    execute_command(&["add", "--target", "x86_64/windows.json", "my-package1"],
                    &manifest);

    // dependencies present afterwards
    let toml = get_toml(&manifest);
    // Get package by hand because toml-rs does not currently handle escaping dots in lookup()
    let target = toml.lookup("target").expect("target dependency not added");
    if let &toml::Value::Table(ref table) = target {
        let win_target = table.get("x86_64/windows.json").expect("target spec not found");
        let val = win_target.lookup("dependencies.my-package1")
                      .expect("target dependency not added");
        assert_eq!(val.as_str().unwrap(), "CURRENT_VERSION_TEST");
    } else {
        panic!("target is not a table");
    }

}


#[test]
#[should_panic]
fn fails_to_add_dependency_with_empty_target() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Fails because target parameter must be a valid target
    execute_command(&["add", "--target", "", "my-package1"],
                    &manifest);
}



#[test]
#[should_panic]
fn fails_to_add_optional_dev_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.versioned-package").is_none());

    // Fails because optional dependencies must be in `dependencies` table.
    execute_command(&["add", "versioned-package", "--vers", ">=0.1.1", "--dev", "--optional"],
                    &manifest);
}

#[test]
#[should_panic]
fn fails_to_add_multiple_optional_dev_dependencies() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependencies not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.my-package1").is_none());
    assert!(toml.lookup("dependencies.my-package2").is_none());

    // Fails because optional dependencies must be in `dependencies` table.
    execute_command(&["add", "--optional", "my-package1", "my-package2", "--dev"],
                    &manifest);
}

#[test]
#[should_panic]
#[cfg(feature="test-external-apis")]
fn fails_to_add_inexistent_git_source_without_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.cargo-edit").is_none());
    
    execute_command(&["add", "https://github.com/killercup/fake-git-repo.git"],
                    &manifest);
}

#[test]
#[should_panic]
fn fails_to_add_inexistent_local_source_without_flag() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // dependency not present beforehand
    let toml = get_toml(&manifest);
    assert!(toml.lookup("dependencies.foo-crate").is_none());

    execute_command(&["add", "./tests/fixtures/local"], &manifest);
}

#[test]
fn no_argument() {
    assert_cli!("target/debug/cargo-add", &["add"] => Error 1,
                r"Invalid arguments.

Usage:
    cargo add <crate> [--dev|--build|--optional] [--vers=<ver>|--git=<uri>|--path=<uri>] [options]
    cargo add <crates>... [--dev|--build|--optional] [options]
    cargo add (-h|--help)
    cargo add --version")
        .unwrap();
}

#[test]
fn unknown_flags() {
    assert_cli!("target/debug/cargo-add", &["add", "foo", "--flag"] => Error 1,
                r"Unknown flag: '--flag'

Usage:
    cargo add <crate> [--dev|--build|--optional] [--vers=<ver>|--git=<uri>|--path=<uri>] [options]
    cargo add <crates>... [--dev|--build|--optional] [options]
    cargo add (-h|--help)
    cargo add --version")
        .unwrap();
}
