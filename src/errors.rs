error_chain!{
    errors {
        /// Failed to fetch crate from crates.io
        FetchVersionFailure {
            description("Failed to fetch crate version from crates.io")
        }
        /// Invalid JSON from crates.io response
        InvalidCratesIoJson {
            description("Invalid JSON (the crate may not exist)")
        }
        /// No crate by that name exists
        NoCrate(name: String) {
            description("The crate could not be found on crates.io.")
            display("The crate `{}` could not be found on crates.io.", name)
        }
        /// No versions available
        NoVersionsAvailable {
            description("No available versions exist. Either all were yanked \
                         or only prerelease versions exist. Trying with the \
                         --allow-prerelease flag might solve the issue."
            )
        }
        /// Unable to parse external Cargo.toml
        ParseCargoToml {
            description("Unable to parse external Cargo.toml")
        }
        /// Cargo.toml could not be found.
        MissingManifest {
            description("Unable to find Cargo.toml")
        }
        /// Cargo.toml is valid toml, but doesn't contain the expected fields
        InvalidManifest {
            description("Cargo.toml missing expected `package` or `project` fields")
        }
        /// Found a workspace manifest when expecting a normal manifest
        UnexpectedRootManifest {
            description("Found virtual manifest, but this command requires running against an \
                         actual package in this workspace.")
        }
        /// The TOML table could not be found.
        NonExistentTable(table: String) {
            description("non existent table")
            display("The table `{}` could not be found.", table)
        }
        /// The dependency could not be found.
        NonExistentDependency(name: String, table: String) {
            description("non existent dependency")
            display("The dependency `{}` could not be found in `{}`.", name, table)
        }
        /// Failed to parse a version for a dependency
        ParseVersion(version: String, dep: String) {
            description("Failed to parse a version for a dependency")
            display("The version `{}` for the dependency `{}` couldn't be parsed", version, dep)
        }
    }
}
