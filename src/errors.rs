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
        /// No versions available
        NoVersionsAvailable {
            description("No available versions exist. Either all were yanked \
                         or only prerelease versions exist. Trying with the \
                         --fetch-prereleases flag might solve the issue."
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
    }
}
