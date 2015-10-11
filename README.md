# cargo edit

This tool extends [Cargo](http://doc.crates.io/) to allow you to add and list dependencies by reading/writing to your `Cargo.toml` file from the command line.

Currently available subcommands:

- [`cargo add`](#cargo-add)
- [`cargo list`](#cargo-list)

[![Build Status](https://travis-ci.org/killercup/cargo-edit.svg?branch=master)](https://travis-ci.org/killercup/cargo-edit)
[![Coverage Status](https://coveralls.io/repos/killercup/cargo-edit/badge.svg?branch=master&service=github)](https://coveralls.io/github/killercup/cargo-edit?branch=master)


## Installation

You can build all commands of `cargo-edit` from the source available on GitHub:

```sh
$ git clone https://github.com/killercup/cargo-edit.git
$ cd cargo-edit
$ cargo build --release
```

Once you have the executables, you can move them to a directory in your `$PATH`, e.g.

```sh
$ cp target/release/cargo-* ~/.bin/
```

You should be able to use the new Cargo subcommands now.

## Available Subcommands

### `cargo add`

Add new dependencies to your `Cargo.toml`.

#### Examples

```sh
$ cargo add regex@0.1.41 --dev
$ cargo add gcc --ver=0.3.18 --build
$ cargo add local_experiment --path=lib/trial-and-error/
```

#### Usage

```plain
$ cargo add --help
Usage:
    cargo add <crate> [--dev|--build] [--ver=<semver>|--git=<uri>|--path=<uri>] [options]
    cargo add (-h|--help)
    cargo add --version

Options:
    -D --dev                Add crate as development dependency.
    -B --build              Add crate as build dependency.
    --ver=<semver>          Specify the version to grab from the registry (crates.io).
                            You can also specify versions as part of the name, e.g
                            `cargo add bitflags@0.3.2`.
    --git=<uri>             Specify a git repository to download the crate from.
    --path=<uri>            Specify the path the crate should be loaded from.
    --manifest-path=<path>  Path to the manifest to add a dependency to.
    -h --help               Show this help page.
    --version               Show version.

Add a dependency to a Cargo.toml manifest file.
```

### `cargo list`

#### Examples

```plain
$ cargo list
clippy          0.0.19
docopt          0.6
pad             0.1
rustc-serialize 0.3
semver          0.1
toml            0.1
```

```plain
$ cargo list --tree
‣ assert_cli (0.1.0)
    ‣ ansi_term (0.6.3)
    ‣ difference (0.4.1)
        ‣ getopts (0.2.14)
‣ clippy (0.0.19)
    ‣ unicode-normalization (0.1.1)
‣ docopt (0.6.73)
    ‣ regex (0.1.41)
        ‣ aho-corasick (0.3.2)
            ‣ memchr (0.1.6)
                ‣ libc (0.1.10)
        ‣ memchr (0.1.6)
            ‣ libc (0.1.10)
        ‣ regex-syntax (0.2.2)
    ‣ rustc-serialize (0.3.16)
    ‣ strsim (0.3.0)
‣ pad (0.1.4)
    ‣ unicode-width (0.1.3)
‣ rustc-serialize (0.3.16)
‣ semver (0.1.20)
‣ toml (0.1.23)
    ‣ rustc-serialize (0.3.16)
```

#### Usage

```plain
$ cargo list --help
Usage:
    cargo list [<section>] [options]
    cargo list (-h|--help)
    cargo list --version

Options:
    --manifest-path=<path>  Path to the manifest to add a dependency to.
    --tree                  List dependencies recursively as tree.
    -h --help               Show this help page.

Display a crate's dependencies using its Cargo.toml file.
```

## License

Apache-2.0/MIT
