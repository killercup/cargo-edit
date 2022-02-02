# cargo edit

This tool extends [Cargo](http://doc.crates.io/) to allow you to add, remove, and upgrade dependencies by modifying your `Cargo.toml` file from the command line.

Currently available subcommands:

- [`cargo add`](#cargo-add)
- [`cargo rm`](#cargo-rm)
- [`cargo upgrade`](#cargo-upgrade)
- [`cargo set-version`](#cargo-set-version)

[![Build Status](https://github.com/killercup/cargo-edit/workflows/build/badge.svg)](https://github.com/killercup/cargo-edit/actions)
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

`cargo-edit` uses [`rustfmt`](https://github.com/rust-lang-nursery/rustfmt) for formatting and [`clippy`](https://github.com/rust-lang-nursery/rust-clippy) for linting.

## Related Cargo Commands

- [`cargo feature`](https://github.com/Riey/cargo-feature)

## Installation

<a href="https://repology.org/project/cargo-edit/versions"><img align="right" src="https://repology.org/badge/vertical-allrepos/cargo-edit.svg" alt="Packaging status"></a>

Ensure that you have a fairly recent version of rust/cargo installed. On Ubuntu you would also need to install `libssl-dev` and `pkg-config` packages.

```console,ignore
$ cargo install cargo-edit
```

If you wish to use a bundled version of `openssl`:

```console,ignore
$ cargo install cargo-edit --features vendored-openssl
```

*Compiler support: requires rustc 1.44+*

(Please check [`cargo`'s documentation](http://doc.crates.io/) to learn how `cargo install` works and how to set up your system so it finds binaries installed by `cargo`.)

Install a sub-set of the commands with `cargo install -f --no-default-features --features "<COMMANDS>"`, where `<COMMANDS>` is a space-separated list of commands; i.e. `add rm upgrade` for the full set.

## Available Subcommands

### `cargo add`

Add new dependencies to your `Cargo.toml`. When no version is specified, `cargo add` will try to query the latest version's number from [crates.io](https://crates.io).

#### Examples

```console,ignore
$ # Add a specific version
$ cargo add regex@0.1.41 --dev
$ # Query the latest version from crates.io and adds it as build dependency
$ cargo add gcc --build
$ # Add a non-crates.io crate
$ cargo add local_experiment --path=lib/trial-and-error/
$ # Add a non-crates.io crate; the crate name will be found automatically
$ cargo add lib/trial-and-error/
$ # Add a crates.io crate with a local development path
$ cargo add my_helper --vers=1.3.1 --path=lib/my-helper/
$ # Add a renamed dependency
$ cargo add thiserror --rename error
```

#### Usage

```console
$ cargo-add add -h
cargo-add [..]
Add dependencies to a Cargo.toml manifest file

USAGE:
    cargo add [OPTIONS] <DEP>[@<VERSION>] [+<FEATURE>,...] ...
    cargo add [OPTIONS] <DEP_PATH> [+<FEATURE>,...] ...

ARGS:
    <DEP_ID>...    Reference to a package to add as a dependency

OPTIONS:
        --no-default-features     Disable the default features
        --default-features        Re-enable the default features
    -F, --features <FEATURES>     Space-separated list of features to add
        --optional                Mark the dependency as optional
        --no-optional             Mark the dependency as required
    -r, --rename <RENAME>         Rename the dependency
        --registry <REGISTRY>     Package registry for this dependency
        --manifest-path <PATH>    Path to `Cargo.toml`
    -p, --package <PKGID>         Package to modify
        --offline                 Run without accessing the network
        --quiet                   Do not print any output in case of success
    -h, --help                    Print help information
    -V, --version                 Print version information

SECTION:
    -D, --dev                Add as development dependency
    -B, --build              Add as build dependency
        --target <TARGET>    Add as dependency to the given target platform

UNSTABLE:
    -Z <FLAG>                Unstable (nightly-only) flags [possible values: git, inline-add]
        --git <URI>          Git repository location
        --branch <BRANCH>    Git branch to download the crate from
        --tag <TAG>          Git tag to download the crate from
        --rev <REV>          Git reference to download the crate from

Examples:
  $ cargo add regex --build
  $ cargo add trycmd --dev
  $ cargo add ./crate/parser/
  $ cargo add serde +derive serde_json

```

### `cargo rm`

Remove dependencies from your `Cargo.toml`.

#### Examples

```console,ignore
$ # Remove a dependency
$ cargo rm regex
$ # Remove a development dependency
$ cargo rm regex --dev
$ # Remove a build dependency
$ cargo rm regex --build
```

#### Usage

```console
$ cargo-rm rm --help
cargo-rm [..]
Remove a dependency from a Cargo.toml manifest file

USAGE:
    cargo rm [OPTIONS] <CRATE>...

ARGS:
    <CRATE>...    Crates to be removed

OPTIONS:
    -B, --build                   Remove crate as build dependency
    -D, --dev                     Remove crate as development dependency
    -h, --help                    Print help information
        --manifest-path <PATH>    Path to the manifest to remove a dependency from
    -p, --package <PKGID>         Package id of the crate to remove this dependency from
    -q, --quiet                   Do not print any output in case of success
    -V, --version                 Print version information
    -Z <FLAG>                     Unstable (nightly-only) flags

```

### `cargo upgrade`

Upgrade dependencies in your `Cargo.toml` to their latest versions.

To specify a version to upgrade to, provide the dependencies in the `<crate name>@<version>` format,
e.g. `cargo upgrade docopt@~0.9.0 serde@>=0.9,<2.0`.

This command differs from `cargo update`, which updates the dependency versions recorded in the
local lock file (Cargo.lock).

#### Examples

```console,ignore
# Upgrade all dependencies for the current crate
$ cargo upgrade
# Upgrade docopt (to ~0.9) and serde (to >=0.9,<2.0)
$ cargo upgrade docopt@~0.9 serde@>=0.9,<2.0
# Upgrade regex (to the latest version) across all crates in the workspace
$ cargo upgrade regex --workspace
# Upgrade all dependencies except docopt and serde
$ cargo upgrade --exclude docopt serde
```

#### Usage

```console
$ cargo-upgrade upgrade --help
cargo-upgrade [..]
Upgrade dependencies as specified in the local manifest file (i.e. Cargo.toml)

USAGE:
    cargo upgrade [OPTIONS] [DEPENDENCY]...

ARGS:
    <DEPENDENCY>...    Crates to be upgraded

OPTIONS:
        --all                     [deprecated in favor of `--workspace`]
        --allow-prerelease        Include prerelease versions when fetching from crates.io (e.g.
                                  0.6.0-alpha')
        --dry-run                 Print changes to be made without making them
        --exclude <EXCLUDE>       Crates to exclude and not upgrade
    -h, --help                    Print help information
        --manifest-path <PATH>    Path to the manifest to upgrade
        --offline                 Run without accessing the network
    -p, --package <PKGID>         Package id of the crate to add this dependency to
        --skip-compatible         Only update a dependency if the new version is semver incompatible
        --to-lockfile             Upgrade all packages to the version in the lockfile
    -V, --version                 Print version information
        --workspace               Upgrade all packages in the workspace
    -Z <FLAG>                     Unstable (nightly-only) flags [possible values: preserve-
                                  precision]

This command differs from `cargo update`, which updates the dependency versions recorded in the
local lock file (Cargo.lock).

If `<dependency>`(s) are provided, only the specified dependencies will be upgraded. The version to
upgrade to for each can be specified with e.g. `docopt@0.8.0` or `serde@>=0.9,<2.0`.

Dev, build, and all target dependencies will also be upgraded. Only dependencies from crates.io are
supported. Git/path dependencies will be ignored.

All packages in the workspace will be upgraded if the `--workspace` flag is supplied. The
`--workspace` flag may be supplied in the presence of a virtual manifest.

If the '--to-lockfile' flag is supplied, all dependencies will be upgraded to the currently locked
version as recorded in the Cargo.lock file. This flag requires that the Cargo.lock file is up-to-
date. If the lock file is missing, or it needs to be updated, cargo-upgrade will exit with an error.
If the '--to-lockfile' flag is supplied then the network won't be accessed.

```

### `cargo set-version`

Set the version in your `Cargo.toml`.

#### Examples

```console,ignore
# Set the version to the version 1.0.0
$ cargo set-version 1.0.0
# Bump the version to the next major
$ cargo set-version --bump major
# Bump version to the next minor
$ cargo set-version --bump minor
# Bump version to the next patch
$ cargo set-version --bump patch
```

#### Usage

```console
$ cargo-set-version set-version --help
cargo-set-version [..]
Change a package's version in the local manifest file (i.e. Cargo.toml)

USAGE:
    cargo set-version [OPTIONS] [TARGET]

ARGS:
    <TARGET>    Version to change manifests to

OPTIONS:
        --all                     [deprecated in favor of `--workspace`]
        --bump <BUMP>             Increment manifest version [possible values: major, minor, patch,
                                  release, rc, beta, alpha]
        --dry-run                 Print changes to be made without making them
        --exclude <EXCLUDE>       Crates to exclude and not modify
    -h, --help                    Print help information
    -m, --metadata <METADATA>     Specify the version metadata field (e.g. a wrapped libraries
                                  version)
        --manifest-path <PATH>    Path to the manifest to upgrade
    -p, --package <PKGID>         Package id of the crate to change the version of
    -V, --version                 Print version information
        --workspace               Modify all packages in the workspace
    -Z <FLAG>                     Unstable (nightly-only) flags

```

For more on `metadata`, see the
[semver crate's documentation](https://docs.rs/semver/1.0.4/semver/struct.BuildMetadata.html).

## License

Apache-2.0/MIT
