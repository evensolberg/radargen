//! FR-102: a `--palette` file, read into a [`Palette`] (D-92).
//!
//! Not a CSS parser. It accepts the one shape the sample file has and an
//! author edits — comments, and one `:root` rule of custom-property
//! declarations — and names everything else with its line, so a file is
//! fixed in one pass (NFR-16 tier one). It knows nothing of flags or
//! `Fatal`: the pipeline says where the file came from (plan RG-018A, R1).

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read as _};
use std::path::Path;

use super::{Hex, Neutral, Palette};
use crate::decode::{NotUtf8, decode};
use scan::{Lines, block_len, css_space, css_trim, excerpt, uncommented};

/// The largest file read (plan RG-018A, R4): the sample is 2 kB, and ten
/// thousand series entries fit in 300 kB.
pub const MAX_BYTES: usize = 1 << 20;

/// The properties a palette may declare, as an unknown-property message
/// lists them.
const PROPERTIES: &str = "--series-1, --series-2 and on, --surface-1, --surface-2, --text-primary, --text-muted, --stroke-ring, --stroke-divider, --marker-hollow and --focus";

/// One problem with a `--palette` file. Messages leave out the line, which
/// the caller prefixes from [`PaletteError::line`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PaletteError {
    /// A `/*` with no `*/` after it.
    #[error("a comment opens here and never closes; add */")]
    UnclosedComment {
        /// Where it opens.
        line: usize,
    },
    /// A rule with no closing `}`.
    #[error("this rule never closes; add }}")]
    UnclosedRule {
        /// Where its selector starts.
        line: usize,
    },
    /// Any rule but the first `:root`: another selector, or an at-rule.
    #[error("'{selector}' is not allowed; a palette is one :root rule and nothing else")]
    OtherRule {
        /// Where it starts.
        line: usize,
        /// The selector or at-rule, quoted by [`excerpt`].
        selector: String,
    },
    /// A second `:root` rule.
    #[error("a second :root rule; a palette is one :root rule, so merge them")]
    SecondRoot {
        /// Where its selector starts.
        line: usize,
    },
    /// Text that is neither a comment nor part of a rule.
    #[error("'{text}' is outside the :root rule; delete it")]
    TextOutside {
        /// Where it starts.
        line: usize,
        /// The text, quoted by [`excerpt`].
        text: String,
    },
    /// A declaration with no `:`, or an empty name.
    #[error("'{text}' is not a declaration; write --name: #RRGGBB;")]
    Malformed {
        /// Where it starts.
        line: usize,
        /// The declaration, quoted by [`excerpt`].
        text: String,
    },
    /// A property that is not one of FR-82's roles or `--series-N`.
    #[error("{name} is not a palette property; the properties are {}", PROPERTIES)]
    Unknown {
        /// Where it is declared.
        line: usize,
        /// The name, quoted by [`excerpt`].
        name: String,
    },
    /// A property declared again; both lines are named (FR-102).
    #[error("{name} repeats line {first}; keep one")]
    Duplicate {
        /// The repeat's line.
        line: usize,
        /// The name, quoted by [`excerpt`].
        name: String,
        /// The first declaration's line.
        first: usize,
    },
    /// FR-86: a value that is not `#RRGGBB`.
    #[error("{name}: '{value}' is not a #RRGGBB colour")]
    NotHex {
        /// Where it is declared.
        line: usize,
        /// The property.
        name: String,
        /// The value, quoted by [`excerpt`].
        value: String,
    },
    /// FR-86: a role the accepted rule does not declare.
    #[error("{role} is missing; a palette defines all eight roles")]
    MissingRole {
        /// The role.
        role: Neutral,
    },
    /// FR-87: series indices `from..=to` are not declared (plan RG-018A,
    /// Ruling 3).
    #[error(
        "{} missing; number series entries from --series-1 with no gaps",
        missing_series(*from, *to)
    )]
    MissingSeries {
        /// The first missing 1-based index.
        from: usize,
        /// The last missing 1-based index.
        to: usize,
    },
}

impl PaletteError {
    /// The line the problem is on, as the file is written; `None` for a
    /// missing role or series, which has no line.
    #[must_use]
    pub const fn line(&self) -> Option<usize> {
        match self {
            Self::UnclosedComment { line }
            | Self::UnclosedRule { line }
            | Self::OtherRule { line, .. }
            | Self::SecondRoot { line }
            | Self::TextOutside { line, .. }
            | Self::Malformed { line, .. }
            | Self::Unknown { line, .. }
            | Self::Duplicate { line, .. }
            | Self::NotHex { line, .. } => Some(*line),
            Self::MissingRole { .. } | Self::MissingSeries { .. } => None,
        }
    }
}

fn missing_series(from: usize, to: usize) -> String {
    if from == to {
        format!("--series-{from} is")
    } else {
        format!("--series-{from} to --series-{to} are")
    }
}

/// Why [`read`] has no palette for the caller.
#[derive(Debug)]
pub enum ReadError {
    /// The file does not exist, is a directory, or cannot be read (NFR-15).
    Unreadable(io::Error),
    /// The file is larger than [`MAX_BYTES`].
    TooLarge,
    /// The file is not UTF-8, or holds a NUL, first on `line` (FR-102).
    NotUtf8 {
        /// The physical line holding the first bad byte.
        line: usize,
    },
    /// The text breaks FR-102, FR-86 or FR-87: every problem, in
    /// [`parse`]'s order.
    Invalid(Vec<PaletteError>),
}

/// FR-102: the palette in the file at `path`.
///
/// Read through a cap rather than whole, so `--palette /dev/zero` ends; not
/// checked to be a regular file, so a process substitution still works.
///
/// # Errors
///
/// A [`ReadError`] saying what kept the file from being a palette.
pub fn read(path: &Path) -> Result<Palette, ReadError> {
    let mut bytes = Vec::new();
    let cap = u64::try_from(MAX_BYTES + 1).unwrap_or(u64::MAX);
    File::open(path)
        .and_then(|file| file.take(cap).read_to_end(&mut bytes))
        .map_err(ReadError::Unreadable)?;
    if bytes.len() > MAX_BYTES {
        return Err(ReadError::TooLarge);
    }
    let text = decode(&bytes).map_err(|NotUtf8 { line }| ReadError::NotUtf8 { line })?;
    parse(&text).map_err(ReadError::Invalid)
}

/// FR-102: `text`, decoded, as a palette; or every problem in it, sorted by
/// line, the missing roles and series last.
///
/// # Errors
///
/// Every [`PaletteError`] in the text.
pub fn parse(text: &str) -> Result<Palette, Vec<PaletteError>> {
    let mut errors = Vec::new();
    let (src, unclosed) = uncommented(text);
    let lines = Lines::of(&src);
    if let Some(open) = unclosed {
        errors.push(PaletteError::UnclosedComment {
            line: lines.at(open),
        });
    }
    let declared = rules(&src, &lines, &mut errors).unwrap_or_default();
    let palette = resolve(&declared, &mut errors);
    errors.sort_by_key(|e| e.line().unwrap_or(usize::MAX));
    match palette {
        Some(palette) if errors.is_empty() => Ok(palette),
        _ => Err(errors),
    }
}

/// One declaration in the accepted rule, trimmed.
struct Declaration<'a> {
    line: usize,
    name: &'a str,
    value: &'a str,
}

/// The accepted `:root` rule's declarations, every grammar error pushed to
/// `errors`; `None` when the file has no `:root` rule.
fn rules<'a>(
    src: &'a str,
    lines: &Lines,
    errors: &mut Vec<PaletteError>,
) -> Option<Vec<Declaration<'a>>> {
    let mut root = None;
    let mut pos = 0;
    loop {
        let rest = src.get(pos..).unwrap_or_default();
        pos += rest.len() - rest.trim_start_matches(css_space).len();
        let rest = src.get(pos..).unwrap_or_default();
        if rest.is_empty() {
            return root;
        }
        let line = lines.at(pos);
        let Some(end) = rest.find(['{', '}', ';']) else {
            errors.push(PaletteError::TextOutside {
                line,
                text: excerpt(rest),
            });
            return root;
        };
        let (prelude, from_end) = rest.split_at(end);
        if !from_end.starts_with('{') {
            // R7: a statement ended by `;` (`@charset`, `@import`), or a
            // stray `}`, is reported, and scanning goes on.
            let statement = rest.get(..=end).unwrap_or_default();
            errors.push(stray(line, statement));
            pos += end + 1;
            continue;
        }
        let body_start = pos + end + 1;
        let Some(len) = block_len(src.get(body_start..).unwrap_or_default()) else {
            errors.push(PaletteError::UnclosedRule { line });
            return root;
        };
        let selector = selector(prelude, pos, lines, errors);
        if !selector.eq_ignore_ascii_case(":root") {
            errors.push(stray_rule(line, selector));
        } else if root.is_some() {
            errors.push(PaletteError::SecondRoot { line });
        } else {
            let body = src.get(body_start..body_start + len).unwrap_or_default();
            root = Some(declarations(body, body_start, lines, errors));
        }
        pos = body_start + len + 1;
    }
}

/// A statement outside every rule: an at-rule when it starts with `@`.
fn stray(line: usize, statement: &str) -> PaletteError {
    if statement.starts_with('@') {
        PaletteError::OtherRule {
            line,
            selector: excerpt(statement),
        }
    } else {
        PaletteError::TextOutside {
            line,
            text: excerpt(statement),
        }
    }
}

/// A rule's selector, from `prelude`, which starts at byte `start`. When its
/// last line is `:root` and text precedes it, that text is reported and the
/// rule kept (R7).
fn selector<'a>(
    prelude: &'a str,
    start: usize,
    lines: &Lines,
    errors: &mut Vec<PaletteError>,
) -> &'a str {
    let whole = css_trim(prelude);
    let Some(split) = whole.rfind(['\n', '\r']) else {
        return whole;
    };
    let (head, tail) = whole.split_at(split);
    let tail = css_trim(tail);
    if tail.eq_ignore_ascii_case(":root") {
        errors.push(stray(lines.at(start), head));
        tail
    } else {
        whole
    }
}

/// `body`'s declarations, `body` starting at byte `start` of the file. A
/// malformed one is pushed to `errors` and left out, and so is a block
/// nested inside the rule, which is another rule (FR-102).
fn declarations<'a>(
    body: &'a str,
    start: usize,
    lines: &Lines,
    errors: &mut Vec<PaletteError>,
) -> Vec<Declaration<'a>> {
    let mut out = Vec::new();
    let mut offset = 0;
    while let Some(rest) = body.get(offset..).filter(|rest| !rest.is_empty()) {
        let end = rest.find([';', '{']).unwrap_or(rest.len());
        let part = rest.get(..end).unwrap_or_default();
        let lead = part.len() - part.trim_start_matches(css_space).len();
        let line = lines.at(start + offset + lead);
        let text = css_trim(part);
        if rest.get(end..).is_some_and(|from| from.starts_with('{')) {
            errors.push(stray_rule(line, text));
            // The outer rule closed, so every block inside it closes too.
            let inner = rest.get(end + 1..).unwrap_or_default();
            offset += end + 1 + block_len(inner).unwrap_or(inner.len()) + 1;
            continue;
        }
        offset += end + 1;
        if text.is_empty() {
            continue;
        }
        match text.split_once(':') {
            Some((name, value)) if !css_trim(name).is_empty() => out.push(Declaration {
                line,
                name: css_trim(name),
                value: css_trim(value),
            }),
            _ => errors.push(PaletteError::Malformed {
                line,
                text: excerpt(text),
            }),
        }
    }
    out
}

/// A rule other than the accepted `:root`, named by its prelude.
fn stray_rule(line: usize, prelude: &str) -> PaletteError {
    let shown = if prelude.is_empty() { "{" } else { prelude };
    PaletteError::OtherRule {
        line,
        selector: excerpt(shown),
    }
}

/// What a property names.
enum Property {
    /// [`Neutral::ALL`]'s index.
    Neutral(usize),
    /// The 1-based series index.
    Series(usize),
}

/// `name` as FR-102 reads it: exact, and `--series-N` with no leading zero.
fn property(name: &str) -> Option<Property> {
    if let Some(at) = Neutral::ALL.iter().position(|n| n.as_str() == name) {
        return Some(Property::Neutral(at));
    }
    let digits = name.strip_prefix("--series-")?;
    if digits.starts_with('0') || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().map(Property::Series)
}

/// FR-86 and FR-87 over the accepted rule's declarations; the palette when
/// they hold.
fn resolve(declared: &[Declaration<'_>], errors: &mut Vec<PaletteError>) -> Option<Palette> {
    let mut neutrals: [Option<Hex>; Neutral::ALL.len()] = Default::default();
    let mut series: BTreeMap<usize, Hex> = BTreeMap::new();
    // Every property declared, valid or not, so one with a bad value is
    // reported once, as `NotHex`, and not again as missing.
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    let mut indices: Vec<usize> = Vec::new();
    for d in declared {
        if let Some(&first) = seen.get(d.name) {
            errors.push(PaletteError::Duplicate {
                line: d.line,
                name: excerpt(d.name),
                first,
            });
            continue;
        }
        seen.insert(d.name, d.line);
        let Some(slot) = property(d.name) else {
            errors.push(PaletteError::Unknown {
                line: d.line,
                name: excerpt(d.name),
            });
            continue;
        };
        if let Property::Series(index) = slot {
            indices.push(index);
        }
        let Some(hex) = Hex::parse(d.value) else {
            errors.push(PaletteError::NotHex {
                line: d.line,
                name: d.name.into(),
                value: excerpt(d.value),
            });
            continue;
        };
        match slot {
            Property::Neutral(at) => {
                if let Some(cell) = neutrals.get_mut(at) {
                    *cell = Some(hex);
                }
            }
            Property::Series(index) => {
                series.insert(index, hex);
            }
        }
    }
    for role in Neutral::ALL {
        if !seen.contains_key(role.as_str()) {
            errors.push(PaletteError::MissingRole { role });
        }
    }
    missing_series_runs(&mut indices, errors);
    let neutrals: [Hex; Neutral::ALL.len()] = neutrals
        .into_iter()
        .collect::<Option<Vec<_>>>()?
        .try_into()
        .ok()?;
    Palette::new(neutrals, series.into_values().collect())
}

/// FR-87's gaps in the declared series indices, one error per run (Ruling
/// 3); `--series-1` missing when none is declared.
fn missing_series_runs(indices: &mut [usize], errors: &mut Vec<PaletteError>) {
    indices.sort_unstable();
    let mut next = 1_usize;
    for &index in indices.iter() {
        if index > next {
            errors.push(PaletteError::MissingSeries {
                from: next,
                to: index - 1,
            });
        }
        next = index.saturating_add(1);
    }
    if indices.is_empty() {
        errors.push(PaletteError::MissingSeries { from: 1, to: 1 });
    }
}

mod scan;
#[cfg(test)]
mod tests;
