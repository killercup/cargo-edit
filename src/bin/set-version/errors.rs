error_chain! {
    errors {
        /// We do not support this version requirement at this time
        UnsupportedVersionReq(req: String) {
            display("Support for modifying {} is currently unsupported", req)
        }
        /// User requested to downgrade a crate
        VersionDowngreade(current: semver::Version, requested: semver::Version) {
            display("Cannot downgrade from {} to {}", current, requested)
        }
    }
    links {
        CargoEditLib(::cargo_edit::Error, ::cargo_edit::ErrorKind);
    }
    foreign_links {
        Io(::std::io::Error) #[doc = "An error from the std::io module"];
        CargoMetadata(::cargo_metadata::Error)#[doc = "An error from the cargo_metadata crate"];
        Version(::semver::Error)#[doc = "An error from the semver crate"];
    }
}
