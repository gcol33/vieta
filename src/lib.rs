//! Vieta is a strict, expression-oriented functional-rewrite language for
//! computer algebra.
//!
//! Ordinary computation uses lexically scoped functional programming. Symbolic
//! computation uses first-class guarded rewrite rules controlled by explicit
//! strategies. Syntax, terms, and runtime values are distinct but interoperable.
//! Mathematical domains, assumptions, rule sets, and sessions are values.
//! Effects and dynamic lookup are explicit. Vieta code compiles against
//! immutable versioned worlds.
//!
//! This release carries no implementation. The design is recorded in
//! `docs/architecture.md` (architectural assessment) and `docs/decisions.md`
//! (irreversible-decision register), both in the repository.

#![forbid(unsafe_code)]

/// The version of this crate, as recorded in `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn version_is_populated() {
        assert!(!VERSION.is_empty());
    }
}
