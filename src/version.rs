/// Additional version functionality
pub trait VersionExt {
    /// Checks to see if the current Version is in pre-release status
    fn is_prerelease(&self) -> bool;
}

impl VersionExt for semver::Version {
    fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}
