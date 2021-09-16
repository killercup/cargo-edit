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

Ensure that you have a fairly recent version of rust/cargo installed. On Ubuntu you would also need to install `libssl-dev` and `pkg-config` packages.

```sh
$ cargo install cargo-edit
```

If you wish to use a bundled version of `openssl`:

```sh
$ cargo install cargo-edit --features vendored-openssl
```

*Compiler support: requires rustc 1.44+*

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
$ # Add a crates.io crate with a local development path
$ cargo add my_helper --vers=1.3.1 --path=lib/my-helper/
$ # Add a renamed dependency
$ cargo add thiserror --rename error
```

#### Usage

```plain
$ cargo add -h
cargo-add
Add dependency to a Cargo.toml manifest file

USAGE:
    cargo add [FLAGS] [OPTIONS] <crate>...

FLAGS:
        --allow-prerelease       Include prerelease versions when fetching from crates.io (e.g. '0.6.0-alpha')
    -B, --build                  Add crate as build dependency
    -D, --dev                    Add crate as development dependency
    -h, --help                   Prints help information
        --no-default-features    Set `default-features = false` for the added dependency
        --offline                Run without accessing the network
        --optional               Add as an optional dependency (for use in features)
    -q, --quiet                  Do not print any output in case of success
    -s, --sort                   Sort dependencies even if currently unsorted
    -V, --version                Prints version information

OPTIONS:
        --branch <branch>           Specify a git branch to download the crate from
        --features <features>...    Space-separated list of features to add. For an alternative approach to enabling
                                    features, consider installing the `cargo-feature` utility
        --git <uri>                 Specify a git repository to download the crate from
        --manifest-path <path>      Path to the manifest to add a dependency to
        --path <path>               Specify the path the crate should be loaded from
    -p, --package <pkgid>           Package id of the crate to add this dependency to
        --registry <registry>       Registry to use
    -r, --rename <rename>           Rename a dependency in Cargo.toml, https://doc.rust-
                                    lang.org/cargo/reference/specifying-
                                    dependencies.html#renaming-dependencies-in-cargotoml Only works
                                    when specifying a single dependency
        --target <target>           Add as dependency to the given target platform
        --upgrade <method>          Choose method of semantic version upgrade.  Must be one of "none" (exact version,
                                    `=` modifier), "patch" (`~` modifier), "minor" (`^` modifier), "all" (`>=`), or
                                    "default" (no modifier) [default: default]  [possible values: none, patch, minor,
                                    all, default]
        --vers <uri>                Specify the version to grab from the registry(crates.io). You can also specify
                                    version as part of name, e.g `cargo add bitflags@0.3.2`

ARGS:
    <crate>...    Crates to be added


This command allows you to add a dependency to a Cargo.toml manifest file. If <crate> is a github
or gitlab repository URL, or a local path, `cargo add` will try to automatically get the crate name
and set the appropriate `--git` or `--path` value.

Please note that Cargo treats versions like '1.2.3' as '^1.2.3' (and that '^1.2.3' is specified
as '>=1.2.3 and <2.0.0'). By default, `cargo add` will use this format, as it is the one that the
crates.io registry suggests. One goal of `cargo add` is to prevent you from using wildcard
dependencies (version set to '*').
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
$ cargo rm -h
cargo-rm
Remove a dependency from a Cargo.toml manifest file

USAGE:
    cargo rm [FLAGS] [OPTIONS] <crates>...

FLAGS:
    -B, --build      Remove crate as build dependency
    -D, --dev        Remove crate as development dependency
    -h, --help       Prints help information
    -q, --quiet      Do not print any output in case of success
    -V, --version    Prints version information

OPTIONS:
        --manifest-path <path>    Path to the manifest to remove a dependency from
    -p, --package <package>       Specify the package in the workspace to add a dependency to (see `cargo help pkgid`)

ARGS:
    <crates>...    Crates to be removed
```

### `cargo upgrade`

Upgrade dependencies in your `Cargo.toml` to their latest versions.

To specify a version to upgrade to, provide the dependencies in the `<crate name>@<version>` format,
e.g. `cargo upgrade docopt@~0.9.0 serde@>=0.9,<2.0`.

This command differs from `cargo update`, which updates the dependency versions recorded in the
local lock file (Cargo.lock).

#### Examples

```sh
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

```plain
$ cargo upgrade -h
cargo-upgrade
Upgrade dependencies as specified in the local manifest file (i.e. Cargo.toml)

USAGE:
    cargo upgrade [FLAGS] [OPTIONS] [dependency]...

FLAGS:
        --workspace           Upgrade all packages in the workspace
        --allow-prerelease    Include prerelease versions when fetching from crates.io (e.g. 0.6.0-alpha')
        --dry-run             Print changes to be made without making them
    -h, --help                Prints help information
        --offline             Run without accessing the network
        --skip-compatible     Only update a dependency if the new version is semver incompatible
        --to-lockfile         Upgrade all packages to the version in the lockfile
    -V, --version             Prints version information

OPTIONS:
        --exclude <exclude>...    Crates to exclude and not upgrade
        --manifest-path <path>    Path to the manifest to upgrade
    -p, --package <package>       Specify the package in the workspace to add a dependency to (see `cargo help pkgid`)

ARGS:
    <dependency>...    Crates to be upgraded

This command differs from `cargo update`, which updates the dependency versions recorded in the
local lock file (Cargo.lock).

If `<dependency>`(s) are provided, only the specified dependencies will be upgraded. The version to
upgrade to for each can be specified with e.g. `docopt@0.8.0` or `serde@>=0.9,<2.0`.

Dev, build, and all target dependencies will also be upgraded. Only dependencies from crates.io are
supported. Git/path dependencies will be ignored.

All packages in the workspace will be upgraded if the `--workspace` flag is supplied.
The `--workspace` flag may be supplied in the presence of a virtual manifest.

If the '--to-lockfile' flag is supplied, all dependencies will be upgraded to the currently locked
version as recorded in the Cargo.lock file. This flag requires that the Cargo.lock file is
up-to-date. If the lock file is missing, or it needs to be updated, cargo-upgrade will exit with an
error. If the '--to-lockfile' flag is supplied then the network won't be accessed.
```

### `cargo set-version`

Set the version in your `Cargo.toml`.

#### Examples

```sh
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

```plain
cargo-set-version 0.7.0
Change a package's version in the local manifest file (i.e. Cargo.toml)

USAGE:
    cargo set-version [FLAGS] [OPTIONS] [--] [target]

FLAGS:
        --all          [deprecated in favor of `--workspace`]
        --dry-run      Print changes to be made without making them
    -h, --help         Prints help information
    -V, --version      Prints version information
        --workspace    Modify all packages in the workspace

OPTIONS:
        --bump <bump>             Increment manifest version [possible values: major, minor, patch,
                                  release, rc, beta, alpha]
        --exclude <exclude>...    Crates to exclude and not modify
        --manifest-path <path>    Path to the manifest to upgrade
    -m, --metadata <metadata>     Specify the version metadata field (e.g. a wrapped libraries version)
    -p, --package <pkgid>         Package id of the crate to change the version of

ARGS:
    <target>    Version to change manifests to
```

For more on `metadata`, see the
[semver crate's documentation](https://docs.rs/semver/1.0.4/semver/struct.BuildMetadata.html).

## License

Apache-2.0/MIT
