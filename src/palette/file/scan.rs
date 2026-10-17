//! The text a `--palette` file is read as: comments blanked, lines
//! indexed, blocks matched and quotes cut short (FR-102).

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

/// `text`'s first line, trimmed, at most [`EXCERPT_CHARS`] characters, with
/// `…` when anything was left out (Review Focus 5).
pub(super) fn excerpt(text: &str) -> String {
    // Split by hand rather than with `lines()`, which ignores a lone CR,
    // and FR-102 ends a line there too.
    let text = css_trim(text);
    let (first, rest) = text.split_once(['\n', '\r']).unwrap_or((text, ""));
    let mut chars = css_trim(first).chars();
    let head: String = chars.by_ref().take(EXCERPT_CHARS).collect();
    if chars.next().is_some() || !css_trim(rest).is_empty() {
        head + "…"
    } else {
        head
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
