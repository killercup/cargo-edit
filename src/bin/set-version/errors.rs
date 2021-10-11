error_chain! {
    errors {
        /// We do not support this version requirement at this time
        UnsupportedVersionReq(req: String) {
            display("Support for modifying {} is currently unsupported", req)
        }
        /// User requested to downgrade a crate
        VersionDowngrade(current: semver::Version, requested: semver::Version) {
            display("Cannot downgrade from {} to {}", current, requested)
        }
        /// User sets version to current
        VersionDoesNotIncrease(current: semver::Version) {
            display("Version is already {}", current)
        }
    }
    links {
        CargoEditLib(::cargo_edit::Error, ::cargo_edit::ErrorKind);
    }
    foreign_links {
        CargoMetadata(::cargo_metadata::Error)#[doc = "An error from the cargo_metadata crate"];
        Version(::semver::Error)#[doc = "An error from the semver crate"];
    }
}
