# cargo edit

This tool extends [Cargo](http://doc.crates.io/) to allow you to add, remove, and upgrade dependencies by modifying your `Cargo.toml` file from the command line.

Currently available subcommands:

- [`cargo upgrade`](#cargo-upgrade)
- [`cargo set-version`](#cargo-set-version)

[![Build Status](https://github.com/killercup/cargo-edit/workflows/build/badge.svg)](https://github.com/killercup/cargo-edit/actions)
[![Build Status](https://travis-ci.org/killercup/cargo-edit.svg?branch=master)](https://travis-ci.org/killercup/cargo-edit)
[![Build status](https://ci.appveyor.com/api/projects/status/m23rnkaxhipb23i9/branch/master?svg=true)](https://ci.appveyor.com/project/killercup/cargo-edit/branch/master)
[![Coverage Status](https://coveralls.io/repos/killercup/cargo-edit/badge.svg?branch=master&service=github)](https://coveralls.io/github/killercup/cargo-edit?branch=master)
[![crates.io](https://img.shields.io/crates/v/cargo-edit.svg)](https://crates.io/crates/cargo-edit)
[![Join the chat at https://gitter.im/cargo-edit/Lobby](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/cargo-edit/Lobby)

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

`cargo add` is now integrated into `cargo` as of v1.62.  If you want access in older versions of `cargo`, you'll need to install `cargo-edit` v0.9 or earlier.

Known differences from `cargo-edit` v0.9.1
- `cargo add <path>` is unsupported, instead use `cargo add --path <path>`
- `cargo add <crate> +<feature>` is unsupported, instead use `cargo add <crate> -F <feature>`
  - If adding multiple crates, qualify the feature like `cargo add serde -F serde/derive serde_json`
  - See [rust-lang/cargo#10809](https://github.com/rust-lang/cargo/issues/10809)

### `cargo rm`

`cargo rm` is now integrated into `cargo` as of v1.66.  If you want access in older versions of `cargo`, you'll need to install `cargo-edit` v0.11 or earlier.

### `cargo upgrade`

Upgrade dependencies in your `Cargo.toml` to their latest versions.

To specify a version to upgrade to, provide the dependencies in the `<crate name>@<version>` format,
e.g. `cargo upgrade -p docopt@~0.9.0 -p serde@>=0.9,<2.0`.

This command differs from `cargo update`, which updates the dependency versions recorded in the
local lock file (Cargo.lock).

#### Examples

```console,ignore
# Upgrade all dependencies for the current crate
$ cargo upgrade
# Upgrade docopt (to ~0.9) and serde (to >=0.9,<2.0)
$ cargo upgrade -p docopt@~0.9 -p serde@>=0.9,<2.0
# Upgrade all dependencies except docopt and serde
$ cargo upgrade --exclude docopt --exclude serde
```

#### Usage

```console
$ cargo-upgrade upgrade --help
Upgrade dependency version requirements in Cargo.toml manifest files

Usage: cargo upgrade [OPTIONS]

Options:
      --dry-run               Print changes to be made without making them
      --manifest-path <PATH>  Path to the manifest to upgrade
      --rust-version <VER>    Override `rust-version`
      --ignore-rust-version   Ignore `rust-version` specification in packages
      --offline               Run without accessing the network
      --locked                Require `Cargo.toml` to be up to date
  -v, --verbose...            Use verbose output
  -Z <FLAG>                   Unstable (nightly-only) flags
  -h, --help                  Print help
  -V, --version               Print version

Version:
      --compatible [<allow|ignore>]    Upgrade to latest compatible version [default: allow]
  -i, --incompatible [<allow|ignore>]  Upgrade to latest incompatible version [default: ignore]
      --pinned [<allow|ignore>]        Upgrade pinned to latest incompatible version [default:
                                       ignore]

Dependencies:
  -p, --package <PKGID[@<VERSION>]>  Crate to be upgraded
      --exclude <PKGID>              Crates to exclude and not upgrade
      --recursive [<true|false>]     Recursively update locked dependencies

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
Change a package's version in the local manifest file (i.e. Cargo.toml)

Usage: cargo set-version [OPTIONS] [TARGET]

Arguments:
  [TARGET]  Version to change manifests to

Options:
      --bump <BUMP>           Increment manifest version
  -m, --metadata <METADATA>   Specify the version metadata field (e.g. a wrapped libraries version)
      --manifest-path <PATH>  Path to the manifest to upgrade
  -p, --package <PKGID>       Package id of the crate to change the version of
      --all                   [deprecated in favor of `--workspace`]
      --workspace             Modify all packages in the workspace
      --dry-run               Print changes to be made without making them
      --exclude <EXCLUDE>     Crates to exclude and not modify
      --offline               Run without accessing the network
      --locked                Require `Cargo.toml` to be up to date
  -Z <FLAG>                   Unstable (nightly-only) flags
      --allow-downgrade       Allow version to be set to a lower version than the current one
  -h, --help                  Print help
  -V, --version               Print version

```

For more on `metadata`, see the
[semver crate's documentation](https://docs.rs/semver/1.0.4/semver/struct.BuildMetadata.html).

## Related Cargo Commands

- [`cargo feature`](https://github.com/Riey/cargo-feature)

## Contribution

Thanks for your interest - we gratefully welcome contributions.

Questions can be asked in [issues](https://github.com/killercup/cargo-edit/issues), or on [Gitter](https://gitter.im/cargo-edit/Lobby).

To help us help you get pull requests merged quickly and smoothly, open an issue before submitted large changes. Please keep the contents of pull requests and commits short. Commit messages should include the intent of the commit.

`cargo-edit` has a moderately comprehensive test suite. Contributions that add/improve tests are awesome. Please add tests for every change.

`cargo-edit` uses [`rustfmt`](https://github.com/rust-lang-nursery/rustfmt) for formatting and [`clippy`](https://github.com/rust-lang-nursery/rust-clippy) for linting.

## License

Apache-2.0/MIT
