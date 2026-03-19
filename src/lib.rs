mod scanner;

use thiserror::Error;

/// Errors that can occur during test extraction.
#[derive(Debug, Error)]
pub enum EjectError {
    /// No inline `#[cfg(test)] mod tests { .. }` block was found.
    #[error("no inline #[cfg(test)] mod tests block found")]
    NoTestModule,

    /// The test module already references an external file via `#[path]`.
    #[error("tests already extracted to external file")]
    AlreadyExternal,

    /// Internal error: could not map the parsed module back to source bytes.
    #[error("could not locate test module boundaries in source")]
    RegionNotFound,

    /// The modified source or extracted test content failed to parse.
    #[error("generated output failed to parse: {reason}")]
    ValidationFailed {
        /// Description of the parse failure.
        reason: String,
    },
}

/// The result of extracting an inline test module.
pub struct EjectResult {
    /// The original source with the test module replaced by a `#[path]` reference.
    pub modified_source: String,
    /// The contents of the extracted test file (inner items only).
    pub test_content: String,
    /// The suggested file name for the test file (e.g. `foo_tests.rs`).
    pub test_file_name: String,
}

/// Extract an inline `#[cfg(test)] mod tests { ... }` block from Rust source
/// into a separate file's content.
///
/// `file_stem` is the base name without extension (e.g. `"foo"` for `foo.rs`),
/// used to derive the test file name `foo_tests.rs`.
///
/// Only the first `#[cfg(test)] mod tests` block is extracted. Files with
/// multiple test modules should be processed one at a time after renaming.
///
/// # Errors
///
/// Returns [`EjectError::NoTestModule`] if no inline test module is found.
/// Returns [`EjectError::AlreadyExternal`] if tests already use a `#[path]` attribute.
/// Returns [`EjectError::RegionNotFound`] if module boundaries cannot be determined.
/// Returns [`EjectError::ValidationFailed`] if the modified source fails to parse
/// (requires the `validate` feature, enabled by default).
pub fn eject_tests(source: &str, file_stem: &str) -> Result<EjectResult, EjectError> {
    let region = scanner::find_test_module_region(source)?;

    let inner = source
        .get(region.inner_start..region.inner_end)
        .ok_or(EjectError::RegionNotFound)?;

    let test_content = dedent(inner);
    let test_file_name = format!("{file_stem}_tests.rs");
    let replacement = format!("#[cfg(test)]\n#[path = \"{test_file_name}\"]\nmod tests;\n");

    let prefix = source
        .get(..region.outer_start)
        .ok_or(EjectError::RegionNotFound)?;
    let suffix = source
        .get(region.outer_end..)
        .ok_or(EjectError::RegionNotFound)?;
    let modified_source = normalize_trailing_newlines(&format!("{prefix}{replacement}{suffix}"));

    #[cfg(feature = "validate")]
    syn::parse_file(&modified_source).map_err(|err| EjectError::ValidationFailed {
        reason: err.to_string(),
    })?;

    Ok(EjectResult {
        modified_source,
        test_content,
        test_file_name,
    })
}

/// Ensure source ends with exactly one trailing newline and no trailing blank lines.
fn normalize_trailing_newlines(source: &str) -> String {
    let trimmed = source.trim_end();
    let mut result = trimmed.to_owned();
    result.push('\n');
    result
}

/// Remove the common leading whitespace from every line.
fn dedent(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();

    let min_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    if min_indent == 0 {
        return text.to_owned();
    }

    let mut result: String = lines
        .iter()
        .map(|line| {
            if line.trim().is_empty() {
                ""
            } else {
                line.get(min_indent..).unwrap_or("")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if text.ends_with('\n') {
        result.push('\n');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_extraction() {
        let source = concat!(
            "use std::collections::HashMap;\n",
            "\n",
            "pub fn add(aa: i32, bb: i32) -> i32 {\n",
            "    aa + bb\n",
            "}\n",
            "\n",
            "#[cfg(test)]\n",
            "mod tests {\n",
            "    use super::*;\n",
            "\n",
            "    #[test]\n",
            "    fn test_add() {\n",
            "        assert_eq!(add(1, 2), 3);\n",
            "    }\n",
            "}\n",
        );

        let result = eject_tests(source, "math").expect("should succeed");

        assert!(
            result
                .modified_source
                .contains("#[path = \"math_tests.rs\"]")
        );
        assert!(result.modified_source.contains("mod tests;"));
        assert!(!result.modified_source.contains("fn test_add"));
        assert!(result.test_content.contains("fn test_add"));
        assert!(result.test_content.contains("use super::*;"));
        assert_eq!(result.test_file_name, "math_tests.rs");
    }

    #[test]
    fn no_test_module() {
        let source = "pub fn add(aa: i32, bb: i32) -> i32 { aa + bb }\n";
        let result = eject_tests(source, "math");
        assert!(matches!(result, Err(EjectError::NoTestModule)));
    }

    #[test]
    fn already_external() {
        let source = "#[cfg(test)]\n#[path = \"math_tests.rs\"]\nmod tests;\n";
        let result = eject_tests(source, "math");
        assert!(matches!(result, Err(EjectError::AlreadyExternal)));
    }

    #[test]
    fn dedent_basic() {
        let input = "    use super::*;\n\n    #[test]\n    fn test_foo() {}\n";
        let result = dedent(input);
        assert!(result.starts_with("use super::*;"));
        assert!(result.contains("#[test]\nfn test_foo()"));
    }

    #[test]
    fn dedent_no_indent() {
        let input = "use super::*;\nfn test_foo() {}\n";
        let result = dedent(input);
        assert_eq!(result, input);
    }

    #[test]
    fn preserves_code_before_tests() {
        let source = concat!(
            "pub struct Foo;\n",
            "\n",
            "impl Foo {\n",
            "    pub fn bar(&self) -> i32 { 42 }\n",
            "}\n",
            "\n",
            "#[cfg(test)]\n",
            "mod tests {\n",
            "    use super::*;\n",
            "    #[test]\n",
            "    fn test_bar() {\n",
            "        assert_eq!(Foo.bar(), 42);\n",
            "    }\n",
            "}\n",
        );

        let result = eject_tests(source, "foo").expect("should succeed");
        assert!(result.modified_source.contains("pub struct Foo;"));
        assert!(result.modified_source.contains("impl Foo"));
        assert!(result.modified_source.contains("fn bar"));
    }

    #[test]
    fn no_trailing_blank_lines() {
        let source = concat!(
            "pub fn add(aa: i32, bb: i32) -> i32 {\n",
            "    aa + bb\n",
            "}\n",
            "\n",
            "\n",
            "#[cfg(test)]\n",
            "mod tests {\n",
            "    use super::*;\n",
            "    #[test]\n",
            "    fn test_add() {\n",
            "        assert_eq!(add(1, 2), 3);\n",
            "    }\n",
            "}\n",
        );

        let result = eject_tests(source, "math").expect("should succeed");
        assert!(result.modified_source.ends_with("mod tests;\n"));
        assert!(!result.modified_source.ends_with("\n\n"));
    }
}
