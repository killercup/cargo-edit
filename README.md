# cargo edit

This tool extends [Cargo](http://doc.crates.io/) to allow you to add, remove, and upgrade dependencies by modifying your `Cargo.toml` file from the command line.

Currently available subcommands:

- [`cargo add`](#cargo-add)
- [`cargo rm`](#cargo-rm)
- [`cargo upgrade`](#cargo-upgrade)

[![Build Status](https://travis-ci.org/killercup/cargo-edit.svg?branch=master)](https://travis-ci.org/killercup/cargo-edit)
[![Build status](https://ci.appveyor.com/api/projects/status/m23rnkaxhipb23i9/branch/master?svg=true)](https://ci.appveyor.com/project/killercup/cargo-edit/branch/master)
[![Coverage Status](https://coveralls.io/repos/killercup/cargo-edit/badge.svg?branch=master&service=github)](https://coveralls.io/github/killercup/cargo-edit?branch=master)
[![crates.io](https://img.shields.io/crates/v/cargo-edit.svg)](https://crates.io/crates/cargo-edit)
[![Join the chat at https://gitter.im/cargo-edit/Lobby](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/cargo-edit/Lobby)

## Contribution

Thanks for your interest - we gratefully welcome contributions.

Questions can be asked in [issues](https://github.com/killercup/cargo-edit/issues), or on [Gitter](https://gitter.im/cargo-edit/Lobby).

To help us help you get pull requests merged quickly and smoothly, open an issue before submitted large changes. Please keep the contents of pull requests and commits short. Commit messages should include the intent of the commit.

`cargo-edit` has a moderately comprehensive test suite. Contributions that add/improve tests are awesome. Please add tests for every change.

`cargo-edit` uses [`rustfmt-nightly`](https://github.com/rust-lang-nursery/rustfmt) for formatting and [`clippy`](https://github.com/rust-lang-nursery/rust-clippy) for linting.

## Installation

```sh
$ cargo install cargo-edit
```

(Please check [`cargo`'s documentation](http://doc.crates.io/) to learn how `cargo install` works and how to set up your system so it finds binaries installed by `cargo`.)

Install a sub-set of the commands with `cargo install -f --no-default-features --features "<COMMANDS>"`, where `<COMMANDS>` is a space-separated list of commands; i.e. `add rm upgrade` for the full set.

## Available Subcommands

### `cargo add`

Add new dependencies to your `Cargo.toml`. When no version is specified, `cargo add` will try to query the latest version's number from [crates.io](https://crates.io).

#### Examples

```sh
$ # Add a specific version
$ cargo add regex@0.1.41 --dev
$ # Query the latest version from crates.io and adds it as build dependency
$ cargo add gcc --build
$ # Add a non-crates.io crate
$ cargo add local_experiment --path=lib/trial-and-error/
$ # Add a non-crates.io crate; the crate name will be found automatically
$ cargo add lib/trial-and-error/
```

#### Usage

```plain
$ cargo add --help
Usage:
    cargo add <crate> [--dev|--build|--optional] [--vers=<ver>|--git=<uri>|--path=<uri>] [options]
    cargo add <crates>... [--dev|--build|--optional] [options]
    cargo add (-h|--help)
    cargo add --version

Specify what crate to add:
    --vers <ver>            Specify the version to grab from the registry (crates.io).
                            You can also specify versions as part of the name, e.g
                            `cargo add bitflags@0.3.2`.
    --git <uri>             Specify a git repository to download the crate from.
    --path <uri>            Specify the path the crate should be loaded from.

Specify where to add the crate:
    -D --dev                Add crate as development dependency.
    -B --build              Add crate as build dependency.
    --optional              Add as an optional dependency (for use in features). This does not work
                            for `dev-dependencies` or `build-dependencies`.
    --target <target>       Add as dependency to the given target platform. This does not work
                            for `dev-dependencies` or `build-dependencies`.

Options:
    --upgrade=<method>      Choose method of semantic version upgrade. Must be one of
                            "none" (exact version), "patch" (`~` modifier), "minor"
                            (`^` modifier, default), or "all" (`>=`).
    --manifest-path=<path>  Path to the manifest to add a dependency to.
    --allow-prerelease      Include prerelease versions when fetching from crates.io (e.g.
                            '0.6.0-alpha'). Defaults to false.
    -q --quiet              Do not print any output in case of success.
    -h --help               Show this help page.
    -V --version            Show version.

This command allows you to add a dependency to a Cargo.toml manifest file. If <crate> is a github
or gitlab repository URL, or a local path, `cargo add` will try to automatically get the crate name
and set the appropriate `--git` or `--path` value.

Please note that Cargo treats versions like "1.2.3" as "^1.2.3" (and that "^1.2.3" is specified
as ">=1.2.3 and <2.0.0"). By default, `cargo add` will use this format, as it is the one that the
crates.io registry suggests. One goal of `cargo add` is to prevent you from using wildcard
dependencies (version set to "*").
```

### `cargo rm`

Remove dependencies from your `Cargo.toml`.

#### Examples

```sh
$ # Remove a dependency
$ cargo rm regex
$ # Remove a development dependency
$ cargo rm regex --dev
$ # Remove a build dependency
$ cargo rm regex --build
```

#### Usage

```plain
$ cargo rm --help
Usage:
    cargo rm <crate> [--dev|--build] [options]
    cargo rm (-h|--help)
    cargo rm --version

Options:
    -D --dev                Remove crate as development dependency.
    -B --build              Remove crate as build dependency.
    --manifest-path=<path>  Path to the manifest to remove a dependency from.
    -q --quiet              Do not print any output in case of success.
    -h --help               Show this help page.
    -V --version            Show version.

Remove a dependency from a Cargo.toml manifest file.
```

### `cargo upgrade`

Upgrade dependencies in your `Cargo.toml` to their latest versions.

This command differs from `cargo update`, which updates the dependency versions recorded in the
local lock file (Cargo.lock).

#### Examples

```sh
# Upgrade all dependencies for the current crate
$ cargo upgrade
# Upgrade libc (to the latest version) and serde (to v1.0.0)
$ cargo upgrade libc serde@1.0.0
# Upgrade regex across all crates in the workspace
$ cargo upgrade regex --all
```

#### Usage

```plain
Upgrade dependencies as specified in the local manifest file (i.e. Cargo.toml).

Usage:
    cargo upgrade [options]
    cargo upgrade [options] <dependency>... [--precise <PRECISE>]
    cargo upgrade (-h | --help)
    cargo upgrade (-V | --version)            

Options:
    --all                   Upgrade all packages in the workspace.
    --precise PRECISE       Upgrade the dependencies to exactly PRECISE.
    --manifest-path PATH    Path to the manifest to upgrade.
    --allow-prerelease      Include prerelease versions when fetching from crates.io (e.g.
                            '0.6.0-alpha'). Defaults to false.
    --dry-run               Print changes to be made without making them. Defaults to false.
    -h --help               Show this help page.
    -V --version            Show version.

This command differs from `cargo update`, which updates the dependency versions recorded in the
local lock file (Cargo.lock).

If `<dependency>`(s) are provided, only the specified dependencies will be upgraded. The version to 
upgrade to for each can be specified with e.g. `docopt@0.8.0`.

Dev, build, and all target dependencies will also be upgraded. Only dependencies from crates.io are
supported. Git/path dependencies will be ignored.

All packages in the workspace will be upgraded if the `--all` flag is supplied. The `--all` flag may
be supplied in the presence of a virtual manifest.
```

## License

Apache-2.0/MIT
