quick_error! {
    #[derive(Debug)]
    pub enum ListError {
        SectionMissing(section: String) {
            description("Couldn't read section")
            display("Couldn't read section `{}`.", section)
        }
        VersionMissing(dep: String, section: String) {
            description("Couldn't read version")
            display("Couldn't read version of `{}` in section `{}`.", dep, section)
        }
        PackagesMissing {
            description("Couldn't read list of packages in `Cargo.lock` file.")
        }
        PackageInvalid {
            description("Invalid package record")
            display("Invalid package record in `Cargo.lock`")
        }
        PackageFieldMissing(field: &'static str) {
            description("Field missing in package record")
            display("Field `{}` missing in package record in `Cargo.lock`.", field)
        }
    }
}
