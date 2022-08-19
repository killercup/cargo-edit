//! Core of cargo-rm command

use cargo::{CargoResult, Config};

#[derive(Clone, Debug)]
pub struct RmOptions<'a> {
    /// Configuration information for Cargo operations
    pub config: &'a Config,
}

pub fn rm(options: &RmOptions<'_>) -> CargoResult<()> {
    Ok(())
}
