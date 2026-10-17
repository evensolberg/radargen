//! The text a `--palette` file is read as: comments blanked, lines
//! indexed, blocks matched and quotes cut short (FR-102).

use std::ops::Range;

/// CSS's comment delimiters.
const COMMENT_OPEN: &str = "/*";
const COMMENT_CLOSE: &str = "*/";

/// How much of an unexpected selector, name or value a message quotes.
pub(super) const EXCERPT_CHARS: usize = 40;

/// CSS whitespace (CSS Syntax §4.2).
pub(super) const fn css_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0C')
}

pub(super) fn css_trim(s: &str) -> &str {
    s.trim_matches(css_space)
}

/// `text`'s first line, trimmed, at most [`EXCERPT_CHARS`] characters
/// counting the escapes made here, with `…` when anything was left out
/// (Review Focus 5).
///
/// Whitespace CSS does not trim, such as a no-break space, is escaped
/// (`\u{a0}`), as NFR-10 shows an invisible character: printed raw, it
/// looks like a space the trim removed, and the value looks valid (rg-8xm).
/// Control and format (Cf) characters are not escaped here: every stderr
/// line passes through `diagnostics::escape_controls`, which escapes them,
/// so this covers only the whitespace CSS does not trim.
pub(super) fn excerpt(text: &str) -> String {
    // Split by hand rather than with `lines()`, which ignores a lone CR,
    // and FR-102 ends a line there too.
    let text = css_trim(text);
    let (first, rest) = text.split_once(['\n', '\r']).unwrap_or((text, ""));
    // Counted as shown, escapes included, so a run of them is cut too.
    let mut head = String::new();
    let mut shown = 0;
    for c in css_trim(first).chars() {
        let piece: String = if c.is_whitespace() && !css_space(c) {
            c.escape_default().collect()
        } else {
            c.into()
        };
        shown += piece.chars().count();
        if shown > EXCERPT_CHARS {
            return head + "…";
        }
        head.push_str(&piece);
    }
    if css_trim(rest).is_empty() {
        head
    } else {
        head + "…"
    }
}

/// `text` with each comment blanked byte for byte, line breaks kept, so
/// offsets and lines stay the file's (FR-102); and the offset of a comment
/// that never closes, past which nothing is read.
pub(super) fn uncommented(text: &str) -> (String, Option<usize>) {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find(COMMENT_OPEN) {
        let (before, from) = rest.split_at(open);
        out.push_str(before);
        let Some(close) = from
            .get(COMMENT_OPEN.len()..)
            .and_then(|after| after.find(COMMENT_CLOSE))
        else {
            return (out, Some(text.len() - from.len()));
        };
        let (comment, after) = from.split_at(close + COMMENT_OPEN.len() + COMMENT_CLOSE.len());
        for c in comment.chars() {
            if c == '\n' || c == '\r' {
                out.push(c);
            } else {
                out.extend(std::iter::repeat_n(' ', c.len_utf8()));
            }
        }
        rest = after;
    }
    out.push_str(rest);
    (out, None)
}

/// Where each line starts, so an offset's line is a binary search rather
/// than a rescan from the top (plan RG-018A, R3). It counts as
/// [`crate::decode::line_at`] does: LF, CRLF and a lone CR once each.
pub(super) struct Lines(Vec<usize>);

impl Lines {
    pub(super) fn of(text: &str) -> Self {
        let bytes = text.as_bytes();
        let mut starts = vec![0];
        for (at, &b) in bytes.iter().enumerate() {
            if b == b'\n' || (b == b'\r' && bytes.get(at + 1) != Some(&b'\n')) {
                starts.push(at + 1);
            }
        }
        Self(starts)
    }

    /// The 1-based line holding byte `offset`.
    pub(super) fn at(&self, offset: usize) -> usize {
        self.0.partition_point(|&start| start <= offset)
    }

    /// Each line that starts inside `span` after its first byte: its 1-based
    /// number and its start.
    pub(super) fn starting_in(&self, span: Range<usize>) -> impl Iterator<Item = (usize, usize)> {
        let first = self.at(span.start);
        let last = self.0.partition_point(|&start| start < span.end);
        let starts = self.0.get(first..last).unwrap_or_default();
        (first + 1..).zip(starts.iter().copied())
    }
}

/// The length of the block whose body is `body` (after its `{`), up to its
/// matching `}`; `None` when it never closes.
pub(super) fn block_len(body: &str) -> Option<usize> {
    let mut depth = 1_usize;
    for (at, c) in body.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}
