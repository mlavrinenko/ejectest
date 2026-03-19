use crate::EjectError;

/// Byte range of the `#[cfg(test)] mod tests { ... }` block in source text.
#[derive(Debug)]
pub(crate) struct TestModuleRegion {
    /// Byte offset where `#[cfg(test)]` starts.
    pub(crate) outer_start: usize,
    /// Byte offset just past the closing `}` (and optional trailing newline).
    pub(crate) outer_end: usize,
    /// Byte offset of the first byte after the opening `{`.
    pub(crate) inner_start: usize,
    /// Byte offset of the closing `}`.
    pub(crate) inner_end: usize,
}

/// Locate the `#[cfg(test)] mod tests { ... }` block in source text.
///
/// Skips matches that appear inside comments or string literals.
///
/// # Errors
///
/// Returns [`EjectError::NoTestModule`] if no inline test module is found.
/// Returns [`EjectError::AlreadyExternal`] if the test module uses a path attribute.
/// Returns [`EjectError::RegionNotFound`] if boundaries cannot be determined.
pub(crate) fn find_test_module_region(source: &str) -> Result<TestModuleRegion, EjectError> {
    let cfg_test = "#[cfg(test)]";
    let code_positions = find_cfg_test_in_code(source, cfg_test);

    for cfg_pos in code_positions {
        let after_cfg = cfg_pos + cfg_test.len();
        let rest = source.get(after_cfg..).ok_or(EjectError::RegionNotFound)?;

        if let Some(mod_offset) = find_mod_tests_after_attrs(rest) {
            let mod_pos = after_cfg + mod_offset;
            let after_kw = mod_pos + "mod tests".len();
            let after_mod = source.get(after_kw..).ok_or(EjectError::RegionNotFound)?;
            let trimmed = after_mod.trim_start();

            if trimmed.starts_with('{') {
                let ws_len = after_mod.len() - trimmed.len();
                let open_brace = after_kw + ws_len;
                let close_brace = find_matching_close_brace(source, open_brace)?;

                let mut outer_end = close_brace + 1;
                if source.get(outer_end..outer_end + 1) == Some("\n") {
                    outer_end += 1;
                }

                return Ok(TestModuleRegion {
                    outer_start: cfg_pos,
                    outer_end,
                    inner_start: open_brace + 1,
                    inner_end: close_brace,
                });
            } else if trimmed.starts_with(';') {
                return Err(EjectError::AlreadyExternal);
            }
        }
    }

    Err(EjectError::NoTestModule)
}

/// Find all byte offsets of `needle` that appear in actual code (not in
/// comments or string literals).
fn find_cfg_test_in_code(source: &str, needle: &str) -> Vec<usize> {
    let mut results = Vec::new();
    let mut state = ScanState::Normal;
    let bytes = source.as_bytes();

    for (idx, ch) in source.char_indices() {
        let is_normal = matches!(state, ScanState::Normal);

        if is_normal && starts_with_at(bytes, needle.as_bytes(), idx) {
            results.push(idx);
        }

        let action = state.advance(ch);
        state = action.next;
    }

    results
}

/// Check if `haystack` starting at `offset` begins with `needle`.
fn starts_with_at(haystack: &[u8], needle: &[u8], offset: usize) -> bool {
    let Some(slice) = haystack.get(offset..offset + needle.len()) else {
        return false;
    };
    slice == needle
}

/// After `#[cfg(test)]`, skip whitespace and extra attributes, then check for `mod tests`.
/// Returns the byte offset (relative to input) of `mod tests` if found.
fn find_mod_tests_after_attrs(source: &str) -> Option<usize> {
    let mut pos: usize = 0;

    loop {
        let rest = source.get(pos..)?;
        let trimmed = rest.trim_start();
        let ws_skipped = rest.len() - trimmed.len();
        pos += ws_skipped;

        if trimmed.starts_with("mod tests") {
            let after = trimmed.get("mod tests".len()..)?;
            let next_ch = after.chars().next();
            match next_ch {
                Some('{' | ';' | ' ' | '\t' | '\n' | '\r') | None => return Some(pos),
                _ => return None,
            }
        } else if trimmed.starts_with("#[") {
            let close = trimmed.find(']')?;
            pos += close + 1;
        } else {
            return None;
        }
    }
}

// ---------------------------------------------------------------------------
// Brace-matching scanner with basic string/comment awareness
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum ScanState {
    Normal,
    /// Seen `/`, waiting for `/` or `*` to enter a comment.
    Slash,
    LineComment,
    BlockComment,
    /// Seen `*` inside a block comment, waiting for `/` to close.
    BlockCommentStar,
    InString,
    InStringEscape,
    /// Seen `'` — could be a char literal or a lifetime.
    Tick,
    /// Inside a char literal, consumed first char, expecting closing `'`.
    InChar,
    /// Inside a char literal after `\`, consuming escaped char next.
    InCharEscape,
}

struct StateAction {
    next: ScanState,
    brace: BraceAction,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BraceAction {
    None,
    Open,
    Close,
}

impl ScanState {
    fn advance(self, ch: char) -> StateAction {
        match self {
            Self::Normal => Self::advance_normal(ch),
            Self::Slash => Self::advance_slash(ch),
            Self::LineComment => Self::advance_line_comment(ch),
            Self::BlockComment => Self::advance_block_comment(ch),
            Self::BlockCommentStar => Self::advance_block_comment_star(ch),
            Self::InString => Self::advance_in_string(ch),
            Self::InStringEscape => StateAction {
                next: Self::InString,
                brace: BraceAction::None,
            },
            Self::Tick => Self::advance_tick(ch),
            Self::InChar => Self::advance_in_char(ch),
            Self::InCharEscape => StateAction {
                next: Self::InChar,
                brace: BraceAction::None,
            },
        }
    }

    fn advance_normal(ch: char) -> StateAction {
        let (next, brace) = match ch {
            '{' => (Self::Normal, BraceAction::Open),
            '}' => (Self::Normal, BraceAction::Close),
            '/' => (Self::Slash, BraceAction::None),
            '"' => (Self::InString, BraceAction::None),
            '\'' => (Self::Tick, BraceAction::None),
            _ => (Self::Normal, BraceAction::None),
        };
        StateAction { next, brace }
    }

    fn advance_slash(ch: char) -> StateAction {
        let (next, brace) = match ch {
            '/' => (Self::LineComment, BraceAction::None),
            '*' => (Self::BlockComment, BraceAction::None),
            '{' => (Self::Normal, BraceAction::Open),
            '}' => (Self::Normal, BraceAction::Close),
            '"' => (Self::InString, BraceAction::None),
            '\'' => (Self::Tick, BraceAction::None),
            _ => (Self::Normal, BraceAction::None),
        };
        StateAction { next, brace }
    }

    fn advance_line_comment(ch: char) -> StateAction {
        let next = if ch == '\n' {
            Self::Normal
        } else {
            Self::LineComment
        };
        StateAction {
            next,
            brace: BraceAction::None,
        }
    }

    fn advance_block_comment(ch: char) -> StateAction {
        let next = if ch == '*' {
            Self::BlockCommentStar
        } else {
            Self::BlockComment
        };
        StateAction {
            next,
            brace: BraceAction::None,
        }
    }

    fn advance_block_comment_star(ch: char) -> StateAction {
        let next = match ch {
            '/' => Self::Normal,
            '*' => Self::BlockCommentStar,
            _ => Self::BlockComment,
        };
        StateAction {
            next,
            brace: BraceAction::None,
        }
    }

    fn advance_in_string(ch: char) -> StateAction {
        let next = match ch {
            '\\' => Self::InStringEscape,
            '"' => Self::Normal,
            _ => Self::InString,
        };
        StateAction {
            next,
            brace: BraceAction::None,
        }
    }

    /// After `'`: could be a char literal (`'x'`, `'\n'`) or a lifetime (`'a`).
    fn advance_tick(ch: char) -> StateAction {
        let next = match ch {
            '\\' => Self::InCharEscape,
            '\'' => Self::Normal,
            _ => Self::InChar,
        };
        StateAction {
            next,
            brace: BraceAction::None,
        }
    }

    /// Inside a char literal after the first character, expecting closing `'`.
    /// If `'` → char literal closed. Otherwise it was a lifetime → back to Normal.
    /// Either way, we return to Normal. But we must check for braces in the
    /// lifetime-fallback case (the current char is real code).
    fn advance_in_char(ch: char) -> StateAction {
        let brace = match ch {
            '{' => BraceAction::Open,
            '}' => BraceAction::Close,
            _ => BraceAction::None,
        };
        StateAction {
            next: Self::Normal,
            brace,
        }
    }
}

/// Find the byte offset of the `}` that matches the `{` at `open_pos`.
fn find_matching_close_brace(source: &str, open_pos: usize) -> Result<usize, EjectError> {
    let rest = source
        .get(open_pos + 1..)
        .ok_or(EjectError::RegionNotFound)?;
    let base = open_pos + 1;

    let mut depth: u32 = 1;
    let mut state = ScanState::Normal;

    for (offset, ch) in rest.char_indices() {
        let action = state.advance(ch);
        state = action.next;
        match action.brace {
            BraceAction::Open => depth += 1,
            BraceAction::Close => {
                depth -= 1;
                if depth == 0 {
                    return Ok(base + offset);
                }
            }
            BraceAction::None => {}
        }
    }

    Err(EjectError::RegionNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_region() {
        let src = "fn main() {}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n}\n";
        let rg = find_test_module_region(src).expect("should find region");
        let outer = src.get(rg.outer_start..rg.outer_end).expect("valid range");
        assert!(outer.starts_with("#[cfg(test)]"));
        assert!(outer.contains("mod tests"));
    }

    #[test]
    fn already_external() {
        let src = "#[cfg(test)]\n#[path = \"t.rs\"]\nmod tests;\n";
        let err = find_test_module_region(src).expect_err("should fail");
        assert!(matches!(err, EjectError::AlreadyExternal));
    }

    #[test]
    fn no_test_module() {
        let src = "fn main() {}\n";
        let err = find_test_module_region(src).expect_err("should fail");
        assert!(matches!(err, EjectError::NoTestModule));
    }

    #[test]
    fn braces_in_string() {
        let src = concat!(
            "#[cfg(test)]\nmod tests {\n",
            "    fn t() { let s = \"}\"; }\n",
            "}\n"
        );
        let rg = find_test_module_region(src).expect("should handle string braces");
        let inner = src.get(rg.inner_start..rg.inner_end).expect("valid range");
        assert!(inner.contains("let s"));
    }

    #[test]
    fn braces_in_comments() {
        let src = concat!(
            "#[cfg(test)]\nmod tests {\n",
            "    // }\n",
            "    /* } */\n",
            "    fn t() {}\n",
            "}\n"
        );
        let rg = find_test_module_region(src).expect("should handle comment braces");
        let inner = src.get(rg.inner_start..rg.inner_end).expect("valid range");
        assert!(inner.contains("fn t()"));
    }

    #[test]
    fn same_line_cfg() {
        let src = "fn main() {}\n#[cfg(test)] mod tests {\n    fn t() {}\n}\n";
        let rg = find_test_module_region(src).expect("should find same-line cfg");
        assert!(
            src.get(rg.outer_start..rg.outer_end)
                .expect("valid")
                .starts_with("#[cfg(test)]")
        );
    }

    #[test]
    fn cfg_test_in_doc_comment_skipped() {
        let src = concat!(
            "/// No inline `#[cfg(test)] mod tests` here.\n",
            "pub fn foo() {}\n",
            "\n",
            "#[cfg(test)]\n",
            "mod tests {\n",
            "    fn real_test() {}\n",
            "}\n"
        );
        let rg = find_test_module_region(src).expect("should skip doc comment");
        let inner = src.get(rg.inner_start..rg.inner_end).expect("valid range");
        assert!(inner.contains("real_test"));
    }

    #[test]
    fn char_literal_with_quote() {
        let src = concat!(
            "fn foo() { let _c = '\"'; }\n",
            "\n",
            "#[cfg(test)]\n",
            "mod tests {\n",
            "    fn real_test() {}\n",
            "}\n"
        );
        let rg = find_test_module_region(src).expect("should handle char literal with quote");
        let inner = src.get(rg.inner_start..rg.inner_end).expect("valid range");
        assert!(inner.contains("real_test"));
    }

    #[test]
    fn cfg_test_in_string_literal_skipped() {
        let src = concat!(
            "fn foo() { let _s = \"#[cfg(test)] mod tests { }\"; }\n",
            "\n",
            "#[cfg(test)]\n",
            "mod tests {\n",
            "    fn real_test() {}\n",
            "}\n"
        );
        let rg = find_test_module_region(src).expect("should skip string literal");
        let inner = src.get(rg.inner_start..rg.inner_end).expect("valid range");
        assert!(inner.contains("real_test"));
    }
}
