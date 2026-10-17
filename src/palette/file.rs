//! FR-102: a `--palette` file, read into a [`Palette`] (D-92).
//!
//! Not a CSS parser. It accepts the one shape the sample file has and an
//! author edits — comments, and one `:root` rule of custom-property
//! declarations — and names everything else with its line, so a file is
//! fixed in one pass (NFR-16 tier one). It knows nothing of flags or
//! `Fatal`: the pipeline says where the file came from (plan RG-018A, R1).

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fs::File;
use std::io::{self, Read as _};
use std::path::Path;

use super::{Hex, Neutral, Palette};
use crate::decode::{NotUtf8, decode};
use scan::{Lines, block_len, css_space, css_trim, excerpt, uncommented};

/// The largest file read (plan RG-018A, R4): the sample is 2 kB, and ten
/// thousand series entries fit in 300 kB.
pub const MAX_BYTES: usize = 1 << 20;
/// What [`read`] reads at most: one byte past the cap, to tell a file at
/// the cap from one beyond it.
const READ_CAP: u64 = (1 << 20) + 1;

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
    /// A value runs onto a later line that declares a property: a forgotten
    /// `;` (rg-8xm). The value ends at that line, which is read as the next
    /// declaration, so it is checked in the same pass (rg-p9a).
    #[error("{name}: its value runs onto the next line; add ; at its end")]
    MissingSemicolon {
        /// Where the declaration missing its `;` is.
        line: usize,
        /// Its property.
        name: String,
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
            | Self::NotHex { line, .. }
            | Self::MissingSemicolon { line, .. } => Some(*line),
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
    /// The file is not UTF-8, or holds a NUL (FR-102).
    NotUtf8(NotUtf8),
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
    File::open(path)
        .and_then(|file| file.take(READ_CAP).read_to_end(&mut bytes))
        .map_err(ReadError::Unreadable)?;
    if bytes.len() > MAX_BYTES {
        return Err(ReadError::TooLarge);
    }
    let text = decode(&bytes).map_err(ReadError::NotUtf8)?;
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
    let declared = rules(&src, &lines, &mut errors);
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

/// The accepted `:root` rule's declarations, none when the file has no
/// `:root` rule; every grammar error is pushed to `errors`.
fn rules<'a>(src: &'a str, lines: &Lines, errors: &mut Vec<PaletteError>) -> Vec<Declaration<'a>> {
    let mut root = None;
    let mut pos = 0;
    loop {
        let rest = src.get(pos..).unwrap_or_default();
        let trimmed = rest.trim_start_matches(css_space);
        pos += rest.len() - trimmed.len();
        let rest = trimmed;
        if rest.is_empty() {
            break;
        }
        let line = lines.at(pos);
        let Some(end) = rest.find(['{', '}', ';']) else {
            errors.push(PaletteError::TextOutside {
                line,
                text: excerpt(rest),
            });
            break;
        };
        let (prelude, from_end) = rest.split_at(end);
        if !from_end.starts_with('{') {
            // R7: a statement ended by `;` (`@charset`, `@import`), or a
            // stray `}`, is reported, and scanning goes on.
            errors.push(stray(line, rest.get(..=end).unwrap_or_default()));
            pos += end + 1;
            continue;
        }
        let body_start = pos + end + 1;
        let Some(len) = block_len(src.get(body_start..).unwrap_or_default()) else {
            errors.push(PaletteError::UnclosedRule { line });
            break;
        };
        let selector = selector(prelude, pos, lines, errors);
        if !selector.eq_ignore_ascii_case(":root") {
            errors.push(other_rule(line, selector));
        } else if root.is_some() {
            errors.push(PaletteError::SecondRoot { line });
        } else {
            let body = src.get(body_start..body_start + len).unwrap_or_default();
            root = Some(declarations(body, body_start, lines, errors));
        }
        pos = body_start + len + 1;
    }
    root.unwrap_or_default()
}

/// A rule other than the accepted `:root`, named by its prelude.
fn other_rule(line: usize, prelude: &str) -> PaletteError {
    let shown = if prelude.is_empty() { "{" } else { prelude };
    PaletteError::OtherRule {
        line,
        selector: excerpt(shown),
    }
}

/// A statement outside every rule: an at-rule when it starts with `@`.
fn stray(line: usize, statement: &str) -> PaletteError {
    if statement.starts_with('@') {
        other_rule(line, statement)
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
        let (part, from_end) = rest.split_at(end);
        let lead = part.len() - part.trim_start_matches(css_space).len();
        let at = start + offset + lead;
        let text = css_trim(part);
        if from_end.starts_with('{') {
            errors.push(other_rule(lines.at(at), text));
            // The outer rule closed, so every block inside it closes too.
            let inner = from_end.get(1..).unwrap_or_default();
            offset += end + 1 + block_len(inner).unwrap_or(inner.len()) + 1;
            continue;
        }
        offset += end + 1;
        segment(text, at, lines, errors, &mut out);
    }
    out
}

/// The declarations in `text`, one `;`-ended segment, trimmed, starting at
/// byte `at` of the file.
///
/// A later line of a value that declares a custom property is a forgotten
/// `;` (rg-8xm): the value ends there, and reading resumes at that line, so
/// what it declares is checked in this pass like any other declaration
/// (rg-p9a). Each part of the segment is read once, so a file of forgotten
/// `;`s stays linear (plan RG-018A R3).
fn segment<'a>(
    mut text: &'a str,
    mut at: usize,
    lines: &Lines,
    errors: &mut Vec<PaletteError>,
    out: &mut Vec<Declaration<'a>>,
) {
    while !text.is_empty() {
        let line = lines.at(at);
        let Some((name, name_len, value)) = text
            .split_once(':')
            .map(|(name, value)| (css_trim(name), name.len(), value))
            .filter(|(name, _, _)| !name.is_empty())
        else {
            errors.push(PaletteError::Malformed {
                line,
                text: excerpt(text),
            });
            return;
        };
        // `value` starts one past the colon, which is ASCII.
        let value_at = at + name_len + 1;
        let next = lines
            .starting_in(value_at..value_at + value.len())
            .map(|(_, from)| from)
            .find(|&from| value.get(from - value_at..).is_some_and(declares_property));
        let Some(from) = next else {
            out.push(Declaration {
                line,
                name,
                value: css_trim(value),
            });
            return;
        };
        errors.push(PaletteError::MissingSemicolon {
            line,
            name: excerpt(name),
        });
        let (before, after) = value.split_at(from - value_at);
        out.push(Declaration {
            line,
            name,
            value: css_trim(before),
        });
        let lead = after.len() - after.trim_start_matches(css_space).len();
        at = from + lead;
        text = after.get(lead..).unwrap_or_default();
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
    // Every series index declared, its colour `None` when the value is bad,
    // so the gaps are those of the declarations and not of the good values.
    let mut series: BTreeMap<usize, Option<Hex>> = BTreeMap::new();
    // Every property declared, valid or not, so one with a bad value is
    // reported once, as `NotHex`, and not again as missing.
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for d in declared {
        if !first_declaration(&mut seen, d.line, d.name, errors) {
            continue;
        }
        let Some(slot) = property(d.name) else {
            errors.push(PaletteError::Unknown {
                line: d.line,
                name: excerpt(d.name),
            });
            continue;
        };
        let hex = Hex::parse(d.value);
        if hex.is_none() {
            errors.push(PaletteError::NotHex {
                line: d.line,
                name: d.name.into(),
                value: excerpt(d.value),
            });
        }
        match slot {
            Property::Neutral(at) => {
                if let Some(cell) = neutrals.get_mut(at) {
                    *cell = hex;
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
    missing_series_runs(series.keys().copied(), errors);
    let neutrals: [Hex; Neutral::ALL.len()] = neutrals
        .into_iter()
        .collect::<Option<Vec<_>>>()?
        .try_into()
        .ok()?;
    Palette::new(neutrals, series.into_values().collect::<Option<_>>()?)
}

/// Records `name` at `line` in `seen`; whether it is the first, a repeat
/// being pushed to `errors` (FR-102).
fn first_declaration<'a>(
    seen: &mut BTreeMap<&'a str, usize>,
    line: usize,
    name: &'a str,
    errors: &mut Vec<PaletteError>,
) -> bool {
    match seen.entry(name) {
        Entry::Occupied(first) => {
            errors.push(PaletteError::Duplicate {
                line,
                name: excerpt(name),
                first: *first.get(),
            });
            false
        }
        Entry::Vacant(slot) => {
            slot.insert(line);
            true
        }
    }
}

/// Whether `line`, a line of a value running to the value's end, declares
/// a custom property (`--name:`), which ends the value before it.
fn declares_property(line: &str) -> bool {
    line.split(['\n', '\r'])
        .next()
        .and_then(|first| first.split_once(':'))
        .is_some_and(|(name, _)| css_trim(name).starts_with("--"))
}

/// FR-87's gaps in the declared series indices, in ascending order: one
/// error per run (Ruling 3), and `--series-1` missing when none is declared.
fn missing_series_runs(indices: impl Iterator<Item = usize>, errors: &mut Vec<PaletteError>) {
    let mut next = 1_usize;
    for index in indices {
        if index > next {
            errors.push(PaletteError::MissingSeries {
                from: next,
                to: index - 1,
            });
        }
        next = index.saturating_add(1);
    }
    if next == 1 {
        errors.push(PaletteError::MissingSeries { from: 1, to: 1 });
    }
}

mod scan;
#[cfg(test)]
mod tests;
