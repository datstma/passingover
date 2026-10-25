//! Core logic for Passing Over: where the planes above you are going.
//!
//! This crate has no Flutter dependency. It is used by the CLI
//! (`passingover-cli`), by the Flutter app through a thin
//! flutter_rust_bridge wrapper (`app/rust`), and by tests.

pub mod aircraft;
pub mod geo;
pub mod position;

pub use aircraft::Aircraft;
pub use geo::{Coordinate, GeoError, QueryPoint, Radius};

/// Version of the core crate, as set in Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A greeting used to check that a front end is wired to the core.
pub fn hello() -> String {
    format!("Hello from passingover-core {VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_names_the_core_and_its_version() {
        let greeting = hello();
        assert!(greeting.starts_with("Hello from passingover-core"));
        assert!(greeting.ends_with(VERSION));
    }
}
