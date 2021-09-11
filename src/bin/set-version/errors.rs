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
        CargoMetadata(::failure::Compat<::cargo_metadata::Error>);
        Version(::semver::Error)#[doc = "An error from the semver crate"];
    }
}
