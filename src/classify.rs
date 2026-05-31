//! Read-only classification of a Rust source file's test module.

use crate::EjectError;
use crate::scanner;

/// How a source file's `mod tests` relates to the sibling-test-file convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// Carries an inline `#[cfg(test)] mod tests { ... }` block that
    /// [`crate::eject_tests`] would extract.
    Inline,
    /// Test module is already external via `#[path]` (nothing to do).
    External,
    /// No `#[cfg(test)] mod tests` module at all.
    NoTests,
}

/// Classify `source` without modifying it.
///
/// Mirrors the detection [`crate::eject_tests`] performs: an inline module is
/// reported as [`Classification::Inline`], an already-`#[path]` module as
/// [`Classification::External`], anything else as [`Classification::NoTests`].
#[must_use]
pub fn classify_source(source: &str) -> Classification {
    match scanner::find_test_module_region(source) {
        Ok(_) => Classification::Inline,
        Err(EjectError::AlreadyExternal) => Classification::External,
        Err(_) => Classification::NoTests,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inline_source() -> &'static str {
        concat!(
            "pub fn foo() -> i32 { 42 }\n",
            "\n",
            "#[cfg(test)]\n",
            "mod tests {\n",
            "    use super::*;\n",
            "    #[test]\n",
            "    fn test_foo() { assert_eq!(foo(), 42); }\n",
            "}\n",
        )
    }

    #[test]
    fn classifies_inline() {
        assert_eq!(classify_source(inline_source()), Classification::Inline);
    }

    #[test]
    fn classifies_external() {
        let src = "#[cfg(test)]\n#[path = \"foo_tests.rs\"]\nmod tests;\n";
        assert_eq!(classify_source(src), Classification::External);
    }

    #[test]
    fn classifies_no_tests() {
        assert_eq!(
            classify_source("pub fn foo() -> i32 { 42 }\n"),
            Classification::NoTests
        );
    }
}
