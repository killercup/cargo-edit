#!/bin/sh

run_tests() {
    run test_basic &&
        run test_basic_dev &&
        run test_multiple &&
        run test_semver &&
        run test_bad_semver &&
        run test_git &&
        run test_git_dev &&
        run test_path
}

# Basic cargo edit deps add
test_basic() {
    cargo edit deps add foo &&
        assert_added "foo = \"*\""
}

# Add dev-dependency
test_basic_dev() {
    cargo edit dev-deps add foo_dev &&
        assert_added "\[dev-dependencies\]" &&
        assert_added "foo_dev = \"*\""
}

# Add build-dependency
test_basic_build() {
    cargo edit build-deps add foo_build &&
        assert_added "\[build-dependencies\]" &&
        assert_added "foo_build = \"*\""
}

# Add multiple crates
test_multiple() {
    cargo edit deps add bar baz &&
        assert_added "bar = \"*\"" &&
        assert_added "baz = \"*\""
}

# Add specific version
test_semver() {
    cargo edit deps add quux_sem --version ">=0.5.0" &&
        assert_added "quux_sem = \">=0.5.0\""
}

# Add invalid semver (should fail).
test_bad_semver() {
     ! cargo edit deps add quux_sem_bad --version "garglesnout" 2>&1 > /dev/null &&
       ! assert_added "quux_sem_bad" 2>&1 > /dev/null
}

# Add git source
test_git() {
    cargo edit deps add quux_git --git "https://localhost/quux_git" &&
        assert_added "\[dependencies.quux_git\]" &&
        assert_added "git = \"https://localhost/quux_git\""
}

# Add git source
test_git_dev() {
    cargo edit dev-deps add quux_dev --git "https://localhost/quux_dev_git" &&
        assert_added "\[dev\-dependencies.quux_dev\]" &&
        assert_added "git = \"https://localhost/quux_dev_git\""
}

# Add local source
test_path() {
    cargo edit deps add quux_pat --path "/path/to/quux_pat" &&
        assert_added "\[dependencies.quux_pat\]" &&
        assert_added "path = \"/path/to/quux_pat\""
}

# Executes a test and printes a passed message.
run() {
    echo Running $1 && ($1) && echo Passed. || (echo FAILED!! && cat Cargo.toml)
}


# Check that cargo-add actually worked.
assert_added() {
    test $(grep "$1" $OUTSIDE_ENV/__cargo_edit_test/Cargo.toml | wc -l) = 1
}

# Set up testing environment (a new crate)
setup_env() {
    echo "Beginning shell tests..\n" &&
        OUTSIDE_ENV=$(pwd | sed "s/\(^.*cargo-edit\/\).*\$/\1/g") &&
        cd $OUTSIDE_ENV &&
        cargo build &&
        PATH=$(echo $OUTSIDE_ENV/target/debug/:$PATH) &&
        cargo new __cargo_edit_test &&
        cd __cargo_edit_test
}

# Delete testing environment from the system
teardown_env() {
    cd $OUTSIDE_ENV &&
        rm -rf __cargo_edit_test
}

main() {
    setup_env && run_tests
    teardown_env && echo "\nFIN."
}

(main)
