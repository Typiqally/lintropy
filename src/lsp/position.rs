//! Byte-offset ↔ LSP [`Position`] conversion.
//!
//! lintropy diagnostics carry 0-based UTF-8 byte offsets (`byte_start` /
//! `byte_end`); LSP [`Position`] fields are 0-based line + UTF-16 code-unit
//! column. The conversion has to:
//!
//! 1. Count `\n` bytes up to `offset` to find the line,
//! 2. Walk that line char-by-char to turn the remaining UTF-8 bytes into
//!    a UTF-16 code-unit count.
//!
//! Both functions assume `src` is valid UTF-8 (which it must be — we
//! received it from the LSP client as a `String` / from `fs::read` and
//! tree-sitter already parsed it).

use tower_lsp::lsp_types::{Position, Range};

/// Convert a 0-based UTF-8 byte offset into an LSP [`Position`].
///
/// If `offset` is past the end of `src`, clamps to the final position.
pub fn byte_to_position(src: &str, offset: usize) -> Position {
    let offset = offset.min(src.len());
    let prefix = &src[..offset];

    let line_starts_iter = prefix.match_indices('\n');
    let (line, line_start) = line_starts_iter
        .enumerate()
        .last()
        .map(|(idx, (pos, _))| (idx as u32 + 1, pos + 1))
        .unwrap_or((0, 0));

    let column_slice = &src[line_start..offset];
    let character: u32 = column_slice.encode_utf16().count() as u32;

    Position { line, character }
}

/// Convert a UTF-8 byte range into an LSP [`Range`].
pub fn byte_range_to_range(src: &str, byte_start: usize, byte_end: usize) -> Range {
    Range {
        start: byte_to_position(src, byte_start),
        end: byte_to_position(src, byte_end),
    }
}

/// Convert an LSP [`Position`] (line + UTF-16 column) into a UTF-8 byte
/// offset into `src`. Clamps out-of-range positions to `src.len()` — the
/// LSP spec lets clients send positions past end-of-line/file and
/// requires the server to behave as if they landed at EOL/EOF.
pub fn position_to_byte(src: &str, pos: Position) -> usize {
    let mut byte = 0usize;
    let mut line_remaining = pos.line;
    for (idx, ch) in src.char_indices() {
        if line_remaining == 0 {
            byte = idx;
            break;
        }
        if ch == '\n' {
            line_remaining -= 1;
            byte = idx + 1;
        }
    }
    if line_remaining > 0 {
        return src.len();
    }

    let mut utf16_remaining = pos.character as usize;
    let line_tail = &src[byte..];
    for (idx, ch) in line_tail.char_indices() {
        if ch == '\n' || utf16_remaining == 0 {
            return byte + idx;
        }
        let units = ch.len_utf16();
        if utf16_remaining < units {
            return byte + idx;
        }
        utf16_remaining -= units;
    }
    src.len()
}

/// Apply a single LSP content-change to `text` in place.
///
/// `range == None` means full replace (older clients may still send this
/// even with incremental sync negotiated). `range == Some(..)` means
/// replace exactly that UTF-16 range with `new_text`.
pub fn apply_change(text: &mut String, range: Option<Range>, new_text: &str) {
    match range {
        None => *text = new_text.to_string(),
        Some(range) => {
            let start = position_to_byte(text, range.start);
            let end = position_to_byte(text, range.end).max(start);
            text.replace_range(start..end, new_text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_offsets_map_to_utf16_columns() {
        let src = "fn main() {\n    let x = 1;\n}\n";
        let pos = byte_to_position(src, src.find("let").unwrap());
        assert_eq!(
            pos,
            Position {
                line: 1,
                character: 4
            }
        );
    }

    #[test]
    fn unicode_column_counts_utf16_units() {
        // "let π = 3;" — π is 2 UTF-8 bytes, 1 UTF-16 code unit.
        let src = "let π = 3;\n";
        let pos = byte_to_position(src, src.find('=').unwrap());
        assert_eq!(
            pos,
            Position {
                line: 0,
                character: 6
            }
        );
    }

    #[test]
    fn offset_at_eof_is_clamped() {
        let src = "abc";
        let pos = byte_to_position(src, 9999);
        assert_eq!(
            pos,
            Position {
                line: 0,
                character: 3
            }
        );
    }

    #[test]
    fn position_round_trips_with_byte_to_position() {
        let src = "fn main() {\n    let π = 3;\n}\n";
        for target in [0usize, 1, 12, 18, src.len()] {
            let pos = byte_to_position(src, target);
            assert_eq!(position_to_byte(src, pos), target, "target={target}");
        }
    }

    #[test]
    fn position_past_line_end_clamps_to_line_end() {
        let src = "ab\ncd\n";
        let pos = Position {
            line: 0,
            character: 99,
        };
        assert_eq!(position_to_byte(src, pos), 2);
    }

    #[test]
    fn apply_change_replaces_range_in_place() {
        let mut text = String::from("let x = 1;\nlet y = 2;\n");
        let range = Range {
            start: Position {
                line: 0,
                character: 4,
            },
            end: Position {
                line: 0,
                character: 5,
            },
        };
        apply_change(&mut text, Some(range), "xx");
        assert_eq!(text, "let xx = 1;\nlet y = 2;\n");
    }

    #[test]
    fn apply_change_inserts_multiline() {
        let mut text = String::from("abc");
        let range = Range {
            start: Position {
                line: 0,
                character: 1,
            },
            end: Position {
                line: 0,
                character: 1,
            },
        };
        apply_change(&mut text, Some(range), "XY\n");
        assert_eq!(text, "aXY\nbc");
    }

    #[test]
    fn apply_change_none_replaces_whole_buffer() {
        let mut text = String::from("old");
        apply_change(&mut text, None, "new");
        assert_eq!(text, "new");
    }
}
