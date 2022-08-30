#![warn(rust_2018_idioms)]
#![allow(clippy::all)]
#![cfg_attr(feature = "deny-warnings", deny(warnings))]

#[macro_use]
extern crate cargo_test_macro;

mod cargo_remove;

#[macro_export]
macro_rules! curr_dir {
    () => {
        $crate::_curr_dir(std::path::Path::new(file!()))
    };
}

#[doc(hidden)]
pub fn _curr_dir(mut file_path: &'static std::path::Path) -> &'static std::path::Path {
    if !file_path.exists() {
        // HACK: temporary fix while in subdirectory, based on similar hack from rust-lang/rust
        let prefix = std::path::PathBuf::from("crates").join("cargo-remove");
        if let Ok(crate_relative) = file_path.strip_prefix(prefix) {
            file_path = crate_relative
        }
    }
    assert!(file_path.exists(), "{} does not exist", file_path.display());
    file_path.parent().unwrap()
}
