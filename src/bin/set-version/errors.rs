use std::fmt::Display;

pub(crate) use cargo_edit::CargoResult;

pub(crate) use cargo_edit::Error;

/// User requested to downgrade a crate
pub(crate) fn version_downgrade_err(current: impl Display, requested: impl Display) -> Error {
    anyhow::format_err!("Cannot downgrade from {current} to {requested}")
}
