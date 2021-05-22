use std::path::PathBuf;

error_chain! {
    foreign_links {
        Io(::std::io::Error) #[doc = "An error from the std::io module"];
        Git(::git2::Error)#[doc = "An error from the git2 crate"];
        CargoMetadata(::failure::Compat<::cargo_metadata::Error>)#[doc = "An error from the cargo_metadata crate"];
    }

    errors {
        /// A crate contains invalid symbol
        CrateNameContainsInvalidCharacter(crate_name: String, symbol: char) {
            description("Specified crate name(s) contains invalid symbol(s)")
            display("Crate name \"{}\" is invalid, contains symbol '{}' (byte values: {})", crate_name, &symbol, symbol.escape_unicode())
        }
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
        /// Unable to find the specified registry
        NoSuchRegistryFound(name: String) {
            display("The registry '{}' could not be found", name)
        }
        /// Failed to parse a version for a dependency
        ParseVersion(version: String, dep: String) {
            description("Failed to parse a version for a dependency")
            display("The version `{}` for the dependency `{}` couldn't be parsed", version, dep)
        }
        /// Missing registry checkout in the cargo registry
        MissingRegistraryCheckout(path: PathBuf) {
            description("Missing registry checkout in the cargo registry")
            display("Looks like ({}) is empty", path.display())
        }
        /// Non Unicode git path
        NonUnicodeGitPath {
            // this is because git2 function takes &str instead of something like AsRef<Path>
            description("Path to cargos registry contains non unicode characters")
        }
    }
}
