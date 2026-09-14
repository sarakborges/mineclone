// The canonical Asteria version lives outside Cargo.toml on purpose.
//
// Normal development version bumps must not touch the Cargo manifest because
// changing package metadata invalidates Cargo fingerprints beyond the binary
// crate and defeats fast incremental `cargo run` builds.
pub(crate) const VERSION: &str = include_str!("../../VERSION");
