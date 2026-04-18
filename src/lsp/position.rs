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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_offsets_map_to_utf16_columns() {
        let src = "fn main() {\n    let x = 1;\n}\n";
        let pos = byte_to_position(src, src.find("let").unwrap());
        assert_eq!(pos, Position { line: 1, character: 4 });
    }

    #[test]
    fn unicode_column_counts_utf16_units() {
        // "let π = 3;" — π is 2 UTF-8 bytes, 1 UTF-16 code unit.
        let src = "let π = 3;\n";
        let pos = byte_to_position(src, src.find('=').unwrap());
        assert_eq!(pos, Position { line: 0, character: 6 });
    }

    #[test]
    fn offset_at_eof_is_clamped() {
        let src = "abc";
        let pos = byte_to_position(src, 9999);
        assert_eq!(pos, Position { line: 0, character: 3 });
    }
}
