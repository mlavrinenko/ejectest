/// Scanning state for tracking context (comments, strings, etc.) in Rust source.
#[derive(Clone, Copy)]
pub(crate) enum ScanState {
    Normal,
    /// Seen `/`, waiting for `/` or `*` to enter a comment.
    Slash,
    LineComment,
    /// Inside a `/* */` block comment at a given nesting depth (≥1).
    BlockComment {
        depth: u32,
    },
    /// Seen `*` inside a block comment, may close with `/`.
    BlockCommentStar {
        depth: u32,
    },
    /// Seen `/` inside a block comment, may open nested `/*`.
    BlockCommentSlash {
        depth: u32,
    },
    InString,
    InStringEscape,
    /// Seen `'` — could be a char literal or a lifetime.
    Tick,
    /// Inside a char literal, consumed first char, expecting closing `'`.
    InChar,
    /// Inside a char literal after `\`, consuming escaped char next.
    InCharEscape,
    /// Seen `r` in code — might start a raw string literal.
    SeenR,
    /// Seen `r` followed by one or more `#`s, counting them.
    RawStringHashes {
        count: u32,
    },
    /// Inside a raw string literal with a known closing hash count.
    InRawString {
        hashes: u32,
    },
    /// Seen `"` inside a raw string, counting trailing `#`s.
    RawStringClosing {
        hashes: u32,
        seen: u32,
    },
}

pub(crate) struct StateAction {
    pub(crate) next: ScanState,
    pub(crate) brace: BraceAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BraceAction {
    None,
    Open,
    Close,
}

impl ScanState {
    pub(crate) fn advance(self, ch: char) -> StateAction {
        match self {
            Self::Normal => Self::advance_normal(ch),
            Self::Slash => Self::advance_slash(ch),
            Self::LineComment => Self::advance_line_comment(ch),
            Self::BlockComment { depth } => Self::advance_block_comment(depth, ch),
            Self::BlockCommentStar { depth } => Self::advance_block_comment_star(depth, ch),
            Self::BlockCommentSlash { depth } => Self::advance_block_comment_slash(depth, ch),
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
            Self::SeenR => Self::advance_seen_r(ch),
            Self::RawStringHashes { count } => Self::advance_raw_string_hashes(count, ch),
            Self::InRawString { hashes } => Self::advance_in_raw_string(hashes, ch),
            Self::RawStringClosing { hashes, seen } => {
                Self::advance_raw_string_closing(hashes, seen, ch)
            }
        }
    }

    fn advance_normal(ch: char) -> StateAction {
        let (next, brace) = match ch {
            '{' => (Self::Normal, BraceAction::Open),
            '}' => (Self::Normal, BraceAction::Close),
            '/' => (Self::Slash, BraceAction::None),
            '"' => (Self::InString, BraceAction::None),
            '\'' => (Self::Tick, BraceAction::None),
            'r' => (Self::SeenR, BraceAction::None),
            _ => (Self::Normal, BraceAction::None),
        };
        StateAction { next, brace }
    }

    fn advance_slash(ch: char) -> StateAction {
        let (next, brace) = match ch {
            '/' => (Self::LineComment, BraceAction::None),
            '*' => (Self::BlockComment { depth: 1 }, BraceAction::None),
            '{' => (Self::Normal, BraceAction::Open),
            '}' => (Self::Normal, BraceAction::Close),
            '"' => (Self::InString, BraceAction::None),
            '\'' => (Self::Tick, BraceAction::None),
            'r' => (Self::SeenR, BraceAction::None),
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

    fn advance_block_comment(depth: u32, ch: char) -> StateAction {
        let next = match ch {
            '*' => Self::BlockCommentStar { depth },
            '/' => Self::BlockCommentSlash { depth },
            _ => Self::BlockComment { depth },
        };
        StateAction {
            next,
            brace: BraceAction::None,
        }
    }

    fn advance_block_comment_star(depth: u32, ch: char) -> StateAction {
        let next = match ch {
            '/' if depth == 1 => Self::Normal,
            '/' => Self::BlockComment { depth: depth - 1 },
            '*' => Self::BlockCommentStar { depth },
            _ => Self::BlockComment { depth },
        };
        StateAction {
            next,
            brace: BraceAction::None,
        }
    }

    fn advance_block_comment_slash(depth: u32, ch: char) -> StateAction {
        let next = match ch {
            '*' => Self::BlockComment { depth: depth + 1 },
            '/' => Self::BlockCommentSlash { depth },
            _ => Self::BlockComment { depth },
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
    /// If `'` → char literal closed. Otherwise it was a lifetime → back to Normal,
    /// processing the current character as code.
    fn advance_in_char(ch: char) -> StateAction {
        if ch == '\'' {
            return StateAction {
                next: Self::Normal,
                brace: BraceAction::None,
            };
        }
        // Was a lifetime; process ch as normal code.
        Self::advance_normal(ch)
    }

    /// Seen `r` in code — check for raw string prefix.
    fn advance_seen_r(ch: char) -> StateAction {
        match ch {
            '"' => StateAction {
                next: Self::InRawString { hashes: 0 },
                brace: BraceAction::None,
            },
            '#' => StateAction {
                next: Self::RawStringHashes { count: 1 },
                brace: BraceAction::None,
            },
            // Not a raw string; process ch as normal code.
            '{' => StateAction {
                next: Self::Normal,
                brace: BraceAction::Open,
            },
            '}' => StateAction {
                next: Self::Normal,
                brace: BraceAction::Close,
            },
            '/' => StateAction {
                next: Self::Slash,
                brace: BraceAction::None,
            },
            '\'' => StateAction {
                next: Self::Tick,
                brace: BraceAction::None,
            },
            'r' => StateAction {
                next: Self::SeenR,
                brace: BraceAction::None,
            },
            _ => StateAction {
                next: Self::Normal,
                brace: BraceAction::None,
            },
        }
    }

    /// Counting `#`s after `r` — waiting for `"` to open the raw string.
    fn advance_raw_string_hashes(count: u32, ch: char) -> StateAction {
        match ch {
            '#' => StateAction {
                next: Self::RawStringHashes { count: count + 1 },
                brace: BraceAction::None,
            },
            '"' => StateAction {
                next: Self::InRawString { hashes: count },
                brace: BraceAction::None,
            },
            // Not a raw string after all; process ch as normal code.
            '{' => StateAction {
                next: Self::Normal,
                brace: BraceAction::Open,
            },
            '}' => StateAction {
                next: Self::Normal,
                brace: BraceAction::Close,
            },
            '/' => StateAction {
                next: Self::Slash,
                brace: BraceAction::None,
            },
            '\'' => StateAction {
                next: Self::Tick,
                brace: BraceAction::None,
            },
            'r' => StateAction {
                next: Self::SeenR,
                brace: BraceAction::None,
            },
            _ => StateAction {
                next: Self::Normal,
                brace: BraceAction::None,
            },
        }
    }

    fn advance_in_raw_string(hashes: u32, ch: char) -> StateAction {
        let next = if ch == '"' {
            if hashes == 0 {
                Self::Normal
            } else {
                Self::RawStringClosing { hashes, seen: 0 }
            }
        } else {
            Self::InRawString { hashes }
        };
        StateAction {
            next,
            brace: BraceAction::None,
        }
    }

    fn advance_raw_string_closing(hashes: u32, seen: u32, ch: char) -> StateAction {
        if ch == '#' {
            let new_seen = seen + 1;
            if new_seen == hashes {
                return StateAction {
                    next: Self::Normal,
                    brace: BraceAction::None,
                };
            }
            return StateAction {
                next: Self::RawStringClosing {
                    hashes,
                    seen: new_seen,
                },
                brace: BraceAction::None,
            };
        }
        if ch == '"' {
            // New potential closing sequence.
            return StateAction {
                next: Self::RawStringClosing { hashes, seen: 0 },
                brace: BraceAction::None,
            };
        }
        StateAction {
            next: Self::InRawString { hashes },
            brace: BraceAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: run the state machine over source and collect brace actions at Normal positions.
    fn scan_braces(source: &str) -> Vec<(usize, BraceAction)> {
        let mut state = ScanState::Normal;
        let mut out = Vec::new();
        for (idx, ch) in source.char_indices() {
            let action = state.advance(ch);
            state = action.next;
            if action.brace != BraceAction::None {
                out.push((idx, action.brace));
            }
        }
        out
    }

    #[test]
    fn raw_string_braces_ignored() {
        // r#"}"# — brace inside a raw string should be ignored.
        let src = r##"let ss = r#"}"#; fn foo() {}"##;
        let braces = scan_braces(src);
        // Only the braces in `fn foo() {}` should be detected.
        assert_eq!(braces.len(), 2);
        assert!(braces.first().is_some_and(|bb| bb.1 == BraceAction::Open));
        assert!(braces.last().is_some_and(|bb| bb.1 == BraceAction::Close));
    }

    #[test]
    fn raw_string_no_hashes() {
        let src = "let ss = r\"}\"; fn foo() {}";
        let braces = scan_braces(src);
        assert_eq!(braces.len(), 2);
    }

    #[test]
    fn raw_string_multiple_hashes() {
        let src = r###"let ss = r##"}"##; fn foo() {}"###;
        let braces = scan_braces(src);
        assert_eq!(braces.len(), 2);
    }

    #[test]
    fn nested_block_comment() {
        let src = "/* /* } */ } */ fn foo() {}";
        let braces = scan_braces(src);
        // Only braces in `fn foo() {}` after the outer comment closes.
        assert_eq!(braces.len(), 2);
    }

    #[test]
    fn lifetime_followed_by_brace() {
        let src = "fn foo<'a>() {}";
        let braces = scan_braces(src);
        assert_eq!(braces.len(), 2);
        assert!(braces.first().is_some_and(|bb| bb.1 == BraceAction::Open));
        assert!(braces.last().is_some_and(|bb| bb.1 == BraceAction::Close));
    }

    #[test]
    fn byte_raw_string_handled() {
        // br#"}"# — brace inside a byte raw string should be ignored.
        // Constructed at runtime to avoid raw-string nesting issues.
        let hash = '#';
        let quote = '\"';
        let src = format!("let ss = br{hash}{quote}}}{quote}{hash}; fn foo() {{}}");
        let braces = scan_braces(&src);
        assert_eq!(braces.len(), 2);
    }
}
