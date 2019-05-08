//! Handle `cargo rm` arguments

use crate::errors::*;

#[derive(Debug, Deserialize)]
/// Docopts input args.
pub struct Args {
    /// Crate name (usage 1)
    pub arg_crate: String,
    /// Crate names (usage 2)
    pub arg_crates: Vec<String>,
    /// dev-dependency
    pub flag_dev: bool,
    /// build-dependency
    pub flag_build: bool,
    /// `Cargo.toml` path
    pub flag_manifest_path: Option<String>,
    /// `--version`
    pub flag_version: bool,
    /// '--quiet'
    pub flag_quiet: bool,
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

    /// Build dependencies from arguments
    pub fn parse_dependencies(&self) -> Result<Vec<String>> {
        if !self.arg_crates.is_empty() {
            return Ok(self.arg_crates.to_owned());
        }

        Ok(vec![self.arg_crate.to_owned()])
    }
}

impl Default for Args {
    fn default() -> Args {
        Args {
            arg_crate: "demo".to_owned(),
            arg_crates: vec![],
            flag_dev: false,
            flag_build: false,
            flag_manifest_path: None,
            flag_version: false,
            flag_quiet: false,
        }
    }
}
