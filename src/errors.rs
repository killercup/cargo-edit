error_chain! {
    foreign_links {
        Io(::std::io::Error) #[doc = "An error from the std::io module"];
        Git(::git2::Error)#[doc = "An error from the git2 crate"];
    }

    errors {
        /// Failed to read home directory
        ReadHomeDirFailure {
            description("Failed to read home directory")
        }
        /// Invalid JSON in registry index
        InvalidSummaryJson {
            description("Invalid JSON in registry index")
        }
        /// Given crate name is empty
        EmptyCrateName{
            description("Found empty crate name")
        }
        /// No crate by that name exists
        NoCrate(name: String) {
            description("The crate could not be found in registry index.")
            display("The crate `{}` could not be found in registry index.", name)
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
        /// Config of cargo is invalid
        InvalidCargoConfig {
            description("Invalid cargo config")
        }
        /// Unable to find the source specified by 'replace-with'
        NoSuchSourceFound(name: String) {
            description("Unable to find the source specified by 'replace-with'")
            display("The source '{}' could not be found", name)
        }
    }
}
