///! Handle `cargo rm` arguments

#[derive(Debug, RustcDecodable)]
/// Docopts input args.
pub struct Args {
    /// Crate name
    pub arg_crate: String,
    /// dev-dependency
    pub flag_dev: bool,
    /// build-dependency
    pub flag_build: bool,
    /// `Cargo.toml` path
    pub flag_manifest_path: Option<String>,
    /// `--version`
    pub flag_version: bool,
}

impl Args {
    /// Get depenency section
    pub fn get_section(&self) -> &'static str {
        if self.flag_dev {
            "dev-dependencies"
        } else if self.flag_build {
            "build-dependencies"
        } else {
            "dependencies"
        }
    }
}

impl Default for Args {
    fn default() -> Args {
        Args {
            arg_crate: "demo".to_owned(),
            flag_dev: false,
            flag_build: false,
            flag_manifest_path: None,
            flag_version: false,
        }
    }
}
