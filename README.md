# cargo edit

This tool extends the behavior of cargo to allow you to add and list dependencies by reading/writing to your `Cargo.toml` file from the command line. It is very simple to install and use.

[![Build Status](https://travis-ci.org/killercup/cargo-edit.svg?branch=master)](https://travis-ci.org/killercup/cargo-edit)

## Installation

You can build `cargo-edit` from the source, available through GitHub. Once you have an executable, you can move it to a directory in your PATH, and the `cargo edit` command will now work.

## Features

- Add any kind of dependency
- List dependencies
- Display a tree of dependencies and subdependencies

## Use

See `cargo edit -h` for a full description of use.

By default, `cargo edit deps add <name>` will add a wildcard dependency for the crate; you can add multiple such wildcard dependencies at once by listening them one by one (e.g. `cargo edit deps add serde libc time` will add all three crates). You can also use a flag to add a crate pegged to a specific version string, or dependent on another source accepted by cargo other than the registry (e.g. git or local dependencies).

Instead of just 'deps', you can specify each dependency list in your `Cargo.toml`(e.g. 'dev-dependencies'). Since 'dependencies' is a word with quite a lot of characters and the author of this tool types them in the wrong order surprisingly often, you can also use 'deps' (i.e., 'deps', 'dev-deps', 'build-deps').
