#cargo add

This tool extends the behavior of cargo to allow you to add dependencies to
your Cargo.toml from the command line. It is very simple to install and use.

###Installation

You can build cargo-add from the source, available through GitHub or in the
repository at crates.io. Once you have an executable, you can move it to a
directory in your PATH, and the `cargo add` command will now work.

###Use

See `cargo add -h` for a full description of use. By default, `cargo add` will
add a wildcard dependency for the crate; you can add multiple such wildcard
dependencies at once by listening them one by one (e.g. `cargo add serde libc
time` will add all three crates). You can also use a flag to add a crate pegged
to a specific version string, or dependent on another source accepted by cargo
other than the registry (e.g. git or local dependencies).
