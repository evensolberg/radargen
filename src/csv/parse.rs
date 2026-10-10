//! Text to records (FR-1, FR-3, FR-4, FR-90).
//!
//! RFC 4180 by hand rather than through the `csv` crate, which numbered CRLF
//! lines wrongly and accepted an unterminated quote silently — NFR-10 and
//! NFR-15 both turn on exactly those (plan RG-005, Decisions).

use std::iter::Peekable;
use std::str::Chars;

use crate::diagnostics::{Fatal, RowKind, Warning, Warnings};
use crate::escape::displayable;
use crate::normalise;

/// One record and where it begins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The 1-based physical line the record begins on, the file's first
    /// line counting as 1 (NFR-10, D-37). A quoted newline makes this run ahead
    /// of the record's ordinal, which is the point: an editor can go to it.
    pub line: usize,
    /// The fields as written: unquoted, unescaped, untrimmed.
    pub fields: Vec<String>,
}

/// What ended a field: a comma, or the end of its record (a line break or
/// the end of the input; a record needs no difference between the two).
enum End {
    Comma,
    Record,
}

/// RFC 4180 records, read lazily so `parse` can judge the header before
/// it scans the rest, with a running physical line count.
struct Scanner<'a> {
    chars: Peekable<Chars<'a>>,
    line: usize,
    /// The character that followed a closing quote, when that was a
    /// [`Fatal::TextAfterQuote`]: a `;` there in the header means another
    /// separator, not bad quoting.
    after_quote: Option<char>,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            chars: text.chars().peekable(),
            line: 1,
            after_quote: None,
        }
    }

    /// Consume one line break — LF, CRLF or a lone CR — if one is next,
    /// and count it.
    fn line_break(&mut self) -> bool {
        match self.chars.peek() {
            Some('\n') => {
                self.chars.next();
            }
            Some('\r') => {
                self.chars.next();
                self.chars.next_if_eq(&'\n');
            }
            _ => return false,
        }
        self.line += 1;
        true
    }

    /// Consume whatever ends a field, if a field may end here.
    fn end(&mut self) -> Option<End> {
        if self.chars.next_if_eq(&',').is_some() {
            Some(End::Comma)
        } else if self.line_break() || self.chars.peek().is_none() {
            Some(End::Record)
        } else {
            None
        }
    }

    /// One field and what ended it. `column` is its 1-based position, for
    /// the error message.
    fn field(&mut self, column: usize) -> Result<(String, End), Fatal> {
        let mut text = String::new();
        if self.chars.next_if_eq(&'"').is_some() {
            let opened = self.line;
            loop {
                match self.chars.next() {
                    None => {
                        return Err(Fatal::UnterminatedQuote {
                            line: opened,
                            column,
                        });
                    }
                    Some('"') if self.chars.next_if_eq(&'"').is_some() => text.push('"'),
                    Some('"') => break,
                    Some(c) => {
                        // A break inside quotes is content, and still a
                        // physical line; CRLF is counted at its LF.
                        if c == '\n' || (c == '\r' && self.chars.peek() != Some(&'\n')) {
                            self.line += 1;
                        }
                        text.push(c);
                    }
                }
            }
            if let Some(end) = self.end() {
                return Ok((text, end));
            }
            self.after_quote = self.chars.peek().copied();
            return Err(Fatal::TextAfterQuote {
                line: self.line,
                column,
            });
        }
        loop {
            if let Some(end) = self.end() {
                return Ok((text, end));
            }
            // `end` returned `None`, so a character follows. A quote inside
            // an unquoted field is kept literally (Decisions).
            if let Some(c) = self.chars.next() {
                text.push(c);
            }
        }
    }
}

/// Decisions: a record whose every field is empty or only spaces and tabs
/// looks blank to the author — an empty line, or Excel's `,,,` for a
/// cleared row. Read before FR-90's removal, so U+0001 is not blank.
fn is_blank(fields: &[String]) -> bool {
    fields
        .iter()
        .all(|f| f.chars().all(|c| c == ' ' || c == '\t'))
}

/// Each record with the physical line it begins on. A blank record is
/// skipped, though its lines are still counted. An `Err` is malformed
/// quoting — [`Fatal::UnterminatedQuote`] or [`Fatal::TextAfterQuote`] —
/// after which nothing parses (NFR-15, NFR-16 tier one).
impl Iterator for Scanner<'_> {
    type Item = Result<Record, Fatal>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.chars.peek().is_some() {
            let line = self.line;
            let mut fields = Vec::new();
            loop {
                let (field, end) = match self.field(fields.len() + 1) {
                    Ok(field) => field,
                    Err(e) => return Some(Err(e)),
                };
                fields.push(field);
                if !matches!(end, End::Comma) {
                    break;
                }
            }
            if !is_blank(&fields) {
                return Some(Ok(Record { line, fields }));
            }
        }
        None
    }
}

/// The parsed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Column names as written, after FR-90's removal, in declared order.
    pub header: Vec<String>,
    /// The header's physical line: 1, unless the file opens with blank lines.
    pub header_line: usize,
    /// Every well-formed data record; each has exactly `header.len()` fields.
    pub records: Vec<Record>,
}

/// FR-3, FR-4 and FR-90: the header and records, with undisplayable
/// characters removed — and warned about — before anything reads a field.
///
/// The `Vec<Fatal>` beside the table is NFR-16 tier two: every duplicate
/// header and every field-count mismatch, in line order. A row that fails
/// its field count is left out of `records`; the rest still reach PR 06, so
/// its row errors join these in the same run.
///
/// # Errors
///
/// Tier one, alone, since nothing after it parses: malformed quoting, an
/// empty file, or a header that shows another separator was used.
pub fn parse(text: &str, w: &mut Warnings) -> Result<(Table, Vec<Fatal>), Fatal> {
    let mut scanner = Scanner::new(text);
    let head = match scanner.next() {
        None => return Err(Fatal::NoHeader),
        // A quoted header cell followed by `;` or a tab: `"name";"ring"`.
        Some(Err(Fatal::TextAfterQuote { line, .. }))
            if let Some(separator) = scanner.after_quote.and_then(separator_name) =>
        {
            return Err(Fatal::NotCommaSeparated { line, separator });
        }
        Some(head) => head?,
    };
    let (head, removed) = cleaned(head);
    let header = head.fields;
    // Judged before the rest is scanned: a quoted first-column cell in a
    // `;` export would otherwise surface as a quoting error.
    if let Some(separator) = other_separator(&header) {
        return Err(Fatal::NotCommaSeparated {
            line: head.line,
            separator,
        });
    }
    warn(w, head.line, &removed, &header);

    let mut errors = duplicate_headers(&header, head.line);
    let mut records = Vec::new();
    for row in scanner {
        let row = row?;
        let (row, removed) = cleaned(row);
        warn(w, row.line, &removed, &header);
        if row.fields.len() == header.len() {
            records.push(row);
        } else {
            errors.push(Fatal::FieldCount {
                line: row.line,
                found: row.fields.len(),
                expected: header.len(),
            });
        }
    }
    let table = Table {
        header,
        header_line: head.line,
        records,
    };
    Ok((table, errors))
}

/// FR-1: the separator a one-column header shows was used instead of a
/// comma, if any. A one-column header can never satisfy FR-5, so naming
/// the real cause loses nothing (Decisions).
fn other_separator(header: &[String]) -> Option<&'static str> {
    let [only] = header else {
        return None;
    };
    [';', '\t']
        .into_iter()
        .find(|&c| only.contains(c))
        .and_then(separator_name)
}

/// How a message names a separator other than a comma.
const fn separator_name(c: char) -> Option<&'static str> {
    match c {
        ';' => Some("semicolons"),
        '\t' => Some("tabs"),
        _ => None,
    }
}

/// FR-90: `record` with every undisplayable character removed, and for each
/// field that lost any, its index, the first removed character and how many.
fn cleaned(record: Record) -> (Record, Vec<(usize, char, usize)>) {
    let mut removed = Vec::new();
    let fields = record
        .fields
        .into_iter()
        .enumerate()
        .map(|(i, field)| {
            let Some(first) = field.chars().find(|&c| !displayable(c)) else {
                return field;
            };
            let count = field.chars().filter(|&c| !displayable(c)).count();
            removed.push((i, first, count));
            field.chars().filter(|&c| displayable(c)).collect()
        })
        .collect();
    (
        Record {
            line: record.line,
            fields,
        },
        removed,
    )
}

/// One FR-90 warning per field that lost characters. The column is named
/// in canonical form, as every row warning names it (PR 02), or by
/// position when the header has no name for it.
fn warn(w: &mut Warnings, line: usize, removed: &[(usize, char, usize)], header: &[String]) {
    for &(column, first, count) in removed {
        let name = header
            .get(column)
            .map(String::as_str)
            .map(normalise)
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("column {}", column + 1));
        w.push(Warning::Row {
            line,
            column,
            kind: RowKind::Undisplayable {
                column: name,
                first,
                count,
            },
        });
    }
}

/// FR-4: every header name repeated under FR-76, naming the column it
/// repeats. An empty cell names nothing, so it cannot repeat a name; Excel
/// pads the header with these (Decisions).
fn duplicate_headers(header: &[String], line: usize) -> Vec<Fatal> {
    let mut seen: Vec<(String, usize, &str)> = Vec::new();
    let mut errors = Vec::new();
    for (i, name) in header.iter().enumerate() {
        let key = normalise(name);
        if key.is_empty() {
            continue;
        }
        if let Some((_, first, first_name)) = seen.iter().find(|(k, ..)| *k == key) {
            errors.push(Fatal::DuplicateHeader {
                line,
                index: i + 1,
                name: name.trim().to_string(),
                first_index: *first,
                first_name: (*first_name).to_string(),
            });
        } else {
            seen.push((key, i + 1, name.trim()));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every record, or the first quoting error.
    fn scan(text: &str) -> Result<Vec<Record>, Fatal> {
        Scanner::new(text).collect()
    }

    /// `(line, fields)` for every record, so a test reads like the file.
    fn lines(text: &str) -> Vec<(usize, Vec<String>)> {
        scan(text)
            .expect("scans")
            .into_iter()
            .map(|r| (r.line, r.fields))
            .collect()
    }

    fn rec(line: usize, fields: &[&str]) -> (usize, Vec<String>) {
        (line, fields.iter().map(|f| (*f).to_string()).collect())
    }

    #[test]
    fn plain_records_with_their_lines() {
        assert_eq!(
            lines("name,ring\nKafka,Adopt\n"),
            [rec(1, &["name", "ring"]), rec(2, &["Kafka", "Adopt"])]
        );
    }

    #[test]
    fn quoted_fields_hold_commas_and_doubled_quotes() {
        assert_eq!(
            lines("a,b\n\"x, y\",\"say \"\"hi\"\"\"\n"),
            [rec(1, &["a", "b"]), rec(2, &["x, y", "say \"hi\""])]
        );
    }

    /// NFR-10, D-37: the physical line, not the ordinal.
    #[test]
    fn a_quoted_newline_advances_later_line_numbers() {
        assert_eq!(
            lines("a,b\n\"one\ntwo\nthree\",x\nnext,y\n"),
            [
                rec(1, &["a", "b"]),
                rec(2, &["one\ntwo\nthree", "x"]),
                rec(5, &["next", "y"])
            ]
        );
    }

    /// Review focus 1: Excel writes CRLF. A lone CR (classic Mac) counts too.
    #[test]
    fn crlf_lines_are_counted_like_an_editor_counts_them() {
        assert_eq!(
            lines("a,b\r\n1,2\r\n3,4\r\n"),
            [
                rec(1, &["a", "b"]),
                rec(2, &["1", "2"]),
                rec(3, &["3", "4"])
            ]
        );
        assert_eq!(
            lines("a\r1\r2"),
            [rec(1, &["a"]), rec(2, &["1"]), rec(3, &["2"])]
        );
        // A CRLF inside quotes is content, and still one physical break.
        assert_eq!(
            lines("a,b\r\n\"x\r\ny\",z\r\nq,r\r\n"),
            [
                rec(1, &["a", "b"]),
                rec(2, &["x\r\ny", "z"]),
                rec(4, &["q", "r"])
            ]
        );
        // Excel's commonest row: a quoted last field, then CRLF.
        assert_eq!(
            lines("a,b\r\nKafka,\"desc\"\r\n"),
            [rec(1, &["a", "b"]), rec(2, &["Kafka", "desc"])]
        );
        // A lone CR inside quotes is one break too.
        assert_eq!(
            lines("a\n\"x\ry\"\nq\n"),
            [rec(1, &["a"]), rec(2, &["x\ry"]), rec(4, &["q"])]
        );
    }

    /// Decisions: blank lines, whitespace-only lines and Excel's cleared
    /// `,,,` rows are skipped, wherever they sit, but their lines count.
    #[test]
    fn blank_records_are_skipped_but_counted() {
        assert_eq!(
            lines("a\n\n1\n\n\n2\n\n"),
            [rec(1, &["a"]), rec(3, &["1"]), rec(6, &["2"])]
        );
        assert_eq!(
            lines("a,b\r\n\r\n1,2\r\n"),
            [rec(1, &["a", "b"]), rec(3, &["1", "2"])]
        );
        assert_eq!(
            lines(",,\na,b,c\n,,\n  \n1,2,3\n\t,\"\"\n"),
            [rec(2, &["a", "b", "c"]), rec(5, &["1", "2", "3"])]
        );
        // Tested before FR-90: a cell of only U+0001 is not blank (SC-10).
        assert_eq!(lines("a\n\u{1}\n"), [rec(1, &["a"]), rec(2, &["\u{1}"])]);
    }

    #[test]
    fn edge_fields_are_kept() {
        assert_eq!(lines("a,b,\n"), [rec(1, &["a", "b", ""])]);
        assert_eq!(
            lines("a,b,"),
            [rec(1, &["a", "b", ""])],
            "trailing comma at EOF"
        );
        // A quoted CR before the closing quote is content; the CR after it ends the record.
        assert_eq!(
            lines("a\n\"x\r\"\rq"),
            [rec(1, &["a"]), rec(2, &["x\r"]), rec(4, &["q"])]
        );
        let err = scan("\"").expect_err("a lone quote");
        assert!(
            matches!(err, Fatal::UnterminatedQuote { line: 1, column: 1 }),
            "{err:?}"
        );
        assert_eq!(lines("a,b"), [rec(1, &["a", "b"])], "no final newline");
        assert_eq!(
            lines("a\n\"x\""),
            [rec(1, &["a"]), rec(2, &["x"])],
            "quoted, no final newline"
        );
        assert_eq!(
            lines(" a , b \n"),
            [rec(1, &[" a ", " b "])],
            "not trimmed here"
        );
        assert_eq!(lines("\"\",x\n"), [rec(1, &["", "x"])]);
    }

    /// Lenient where Excel never writes: a quote inside an unquoted field.
    #[test]
    fn a_quote_inside_an_unquoted_field_is_literal() {
        assert_eq!(lines("5\" screen,x\n"), [rec(1, &["5\" screen", "x"])]);
    }

    /// Decisions: RFC 4180 counts spaces around quotes as field content, so
    /// a space before the opening quote makes the field unquoted, and one
    /// after the closing quote is text after it.
    #[test]
    fn spaces_around_quotes_are_not_ignored() {
        assert_eq!(lines("x, \"y\"\n"), [rec(1, &["x", " \"y\""])]);
        let err = scan("a,b\n\"x\" ,y\n").expect_err("text after quote");
        assert!(
            matches!(err, Fatal::TextAfterQuote { line: 2, column: 1 }),
            "{err:?}"
        );
    }

    /// Review focus 3, NFR-15: it would swallow the rest of the file.
    #[test]
    fn an_unterminated_quote_names_the_line_it_opened_on() {
        let err = scan("a,b\nx,y\n\"open,2\n3,4\n").expect_err("unterminated");
        assert!(
            matches!(err, Fatal::UnterminatedQuote { line: 3, column: 1 }),
            "{err:?}"
        );
        assert_eq!(
            err.render(),
            "error: line 3, column 1: a quote opens this value and never closes; add the closing quote, and write any quote inside the value as \"\""
        );
        // The opening quote's line, even when its record began earlier.
        let err = scan("a,b,c\n1,\"two\nlines\",\"open\n").expect_err("unterminated");
        assert!(
            matches!(err, Fatal::UnterminatedQuote { line: 3, column: 3 }),
            "{err:?}"
        );
    }

    #[test]
    fn text_after_a_closing_quote_is_an_error() {
        let err = scan("a,b\n1,\"x\"y\n").expect_err("text after quote");
        assert!(
            matches!(err, Fatal::TextAfterQuote { line: 2, column: 2 }),
            "{err:?}"
        );
        assert_eq!(
            err.render(),
            "error: line 2, column 2: text follows a closing quote; inside quotes, write a quote as \"\""
        );
        // The closing quote's line, not the record's (Global Constraints).
        let err = scan("a,b\n\"x\ny\"z,1\n").expect_err("text after quote");
        assert!(
            matches!(err, Fatal::TextAfterQuote { line: 3, column: 1 }),
            "{err:?}"
        );
    }

    /// Every string of up to `max_len` characters over `alphabet`, shortest
    /// first. Exhaustive rather than random: no dependency, no flaky seed.
    fn all_texts(alphabet: &[char], max_len: u32) -> impl Iterator<Item = String> + '_ {
        (0..=max_len).flat_map(move |len| {
            (0..alphabet.len().pow(len)).map(move |mut i| {
                (0..len)
                    .map(|_| {
                        let c = alphabet[i % alphabet.len()];
                        i /= alphabet.len();
                        c
                    })
                    .collect()
            })
        })
    }

    /// About 19,500 inputs: scan never panics and fails only on quoting;
    /// lines only rise; without quotes, lines are an editor's split on LF,
    /// CRLF and CR, with blank records (here, those without an `a`) skipped.
    #[test]
    fn small_inputs_scan_cleanly_and_lines_only_rise() {
        for text in all_texts(&['a', ',', '"', '\n', '\r'], 6) {
            match scan(&text) {
                Ok(records) => {
                    assert!(
                        records.windows(2).all(|p| p[0].line < p[1].line),
                        "{text:?}"
                    );
                    if !text.contains('"') {
                        let norm = text.replace("\r\n", "\n").replace('\r', "\n");
                        let expected: Vec<usize> = norm
                            .split('\n')
                            .enumerate()
                            .filter(|(_, l)| l.contains('a'))
                            .map(|(i, _)| i + 1)
                            .collect();
                        let got: Vec<usize> = records.iter().map(|r| r.line).collect();
                        assert_eq!(got, expected, "{text:?}");
                    }
                }
                Err(e) => assert!(
                    matches!(
                        e,
                        Fatal::UnterminatedQuote { .. } | Fatal::TextAfterQuote { .. }
                    ),
                    "{text:?}: {e:?}"
                ),
            }
        }
    }

    /// RFC 4180 round trip: any field, quoted with its quotes doubled as
    /// Excel writes it, scans back unchanged.
    #[test]
    fn excel_quoted_fields_round_trip() {
        for field in all_texts(&['a', ',', '"', '\n', '\r', ' '], 4) {
            let written = format!("\"{}\",x\r\n", field.replace('"', "\"\""));
            let records = scan(&written).expect("scans");
            assert_eq!(records.len(), 1, "{written:?}");
            assert_eq!(
                records[0].fields,
                [field.clone(), "x".to_string()],
                "{written:?}"
            );
        }
    }

    /// `parse`'s outputs, rendered: the table or its tier-one error, the
    /// tier-two errors, and the warnings.
    fn parsed(text: &str) -> (Result<Table, Fatal>, Vec<String>, Vec<String>) {
        let mut w = Warnings::default();
        match parse(text, &mut w) {
            Ok((table, errs)) => (
                Ok(table),
                errs.iter().map(Fatal::render).collect(),
                w.render(),
            ),
            Err(e) => (Err(e), Vec::new(), w.render()),
        }
    }

    /// Every error, of either tier.
    fn errors(text: &str) -> Vec<String> {
        match parsed(text) {
            (Err(e), _, _) => vec![e.render()],
            (Ok(_), errs, _) => errs,
        }
    }

    /// A table that parsed with no error of either tier.
    fn table(text: &str) -> Table {
        let (table, errs, _) = parsed(text);
        assert!(errs.is_empty(), "{errs:?}");
        table.expect("parses")
    }

    const MATCH: &str =
        "; header names match ignoring case and surrounding spaces, so rename or remove one";
    const FEWER: &str = "; a value is missing, or an unquoted line break split the row";
    const MORE: &str = "; a comma inside a value needs the value in double quotes";

    #[test]
    fn a_table_keeps_the_header_and_every_record() {
        let (table, errs, warnings) = parsed("name,ring\nKafka,Adopt\nRedis,Trial\n");
        let table = table.expect("parses");
        assert_eq!(table.header, ["name", "ring"]);
        assert_eq!(table.header_line, 1);
        assert_eq!(table.records.len(), 2);
        assert_eq!(table.records[1].line, 3);
        assert!(
            errs.is_empty() && warnings.is_empty(),
            "{errs:?} {warnings:?}"
        );
    }

    #[test]
    fn a_header_only_file_parses_to_no_records() {
        assert_eq!(table("name,ring\n").records.len(), 0);
    }

    /// Decisions: the header is the first non-blank record, on its physical line.
    #[test]
    fn leading_blank_lines_move_the_header_line() {
        let t = table("\n,,\nname,ring\nKafka,Adopt\n");
        assert_eq!((t.header_line, t.records[0].line), (3, 4));
        assert_eq!(
            errors("\n\na,a\n"),
            [format!(
                "error: line 3: column 2 'a' repeats column 1 'a'{MATCH}"
            )]
        );
    }

    #[test]
    fn an_empty_file_has_no_header() {
        for text in ["", "\n\n", "\r\n", ",,,\n", "  \n"] {
            assert_eq!(
                errors(text),
                [
                    "error: the CSV has no header row (it is empty or blank); the header must name at least name, quadrant, ring and description"
                ],
                "{text:?}"
            );
        }
    }

    /// FR-1, Review focus 6: a decimal-comma locale's Excel writes `;`.
    #[test]
    fn a_semicolon_export_is_named_as_such() {
        assert_eq!(
            errors("name;ring\nKafka;Adopt, mostly\n"),
            [
                "error: line 1: the header is one column holding semicolons; radargen reads comma-separated files only, so export the file with commas as the separator"
            ]
        );
        assert_eq!(
            errors("name\tring\nKafka\tAdopt\n"),
            [
                "error: line 1: the header is one column holding tabs; radargen reads comma-separated files only, so export the file with commas as the separator"
            ]
        );
        // Excel quotes a first-column cell holding the separator, and some
        // exports quote every cell; neither may turn this into a quoting error.
        for (text, separator) in [
            ("name;ring\n\"Kafka; streams\";Adopt\n", "semicolons"),
            ("\"name\";\"ring\"\nKafka;Adopt\n", "semicolons"),
            ("\n\"name\"\t\"ring\"\n", "tabs"),
            ("name\tring\n\"a\tb\"\tAdopt\n", "tabs"),
        ] {
            let line = if text.starts_with('\n') { 2 } else { 1 };
            assert_eq!(
                errors(text),
                [format!(
                    "error: line {line}: the header is one column holding {separator}; radargen reads comma-separated files only, so export the file with commas as the separator"
                )],
                "{text:?}"
            );
        }
        // A quoting error in a comma-separated header is still one.
        assert_eq!(
            errors("\"name\"x,ring\n"),
            [
                "error: line 1, column 1: text follows a closing quote; inside quotes, write a quote as \"\""
            ]
        );
        assert!(
            errors("name\nKafka\n").is_empty(),
            "one plain column is PR 06's FR-5 to judge"
        );
    }

    /// FR-4 under FR-76: `Name` and `name ` collide. Every one is reported.
    #[test]
    fn every_duplicate_header_is_reported() {
        assert_eq!(
            errors("name,ring,Name ,RING,x\n"),
            [
                format!("error: line 1: column 3 'Name' repeats column 1 'name'{MATCH}"),
                format!("error: line 1: column 4 'RING' repeats column 2 'ring'{MATCH}"),
            ]
        );
    }

    /// Decisions, Review focus 6: Excel pads the header to its used range;
    /// empty cells name nothing.
    #[test]
    fn empty_header_cells_are_not_duplicates() {
        assert_eq!(
            table("name,ring,,\nKafka,Adopt,,\n").header,
            ["name", "ring", "", ""]
        );
    }

    /// FR-4, NFR-16 tier two: every bad row at once, in line order.
    #[test]
    fn every_field_count_mismatch_is_reported() {
        assert_eq!(
            errors("a,b\n1,2\n1\n1,2,3\n\"multi\nline\",2,3\n"),
            [
                format!("error: line 3: the row's field count is 1 but the header's is 2{FEWER}"),
                format!("error: line 4: the row's field count is 3 but the header's is 2{MORE}"),
                format!("error: line 5: the row's field count is 3 but the header's is 2{MORE}"),
            ]
        );
    }

    /// A tier-one error stands alone: no warning precedes it (NFR-16).
    #[test]
    fn a_refused_separator_leaves_no_warnings() {
        let (table, _, warnings) = parsed("name;ring\u{1}\nx\n");
        assert!(table.is_err());
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// `decode::line_at`'s promise, exhaustively: a bad byte where a record
    /// starts is reported on the line `parse` gives that record.
    #[test]
    fn decode_and_parse_agree_on_every_small_input() {
        for body in all_texts(&['a', ',', '"', '\n', '\r'], 6) {
            if !body.ends_with(['\n', '\r']) {
                continue;
            }
            let text = format!("h\n{body}w");
            let Ok((table, _)) = parse(&text, &mut Warnings::default()) else {
                continue;
            };
            let Some(last) = table.records.last().map(|r| r.line) else {
                continue;
            };
            let mut bytes = format!("h\n{body}").into_bytes();
            bytes.push(0xE9);
            assert_eq!(
                crate::csv::decode::decode(&bytes).map_err(|e| e.line),
                Err(last),
                "{text:?}"
            );
        }
    }

    /// Review focus 7: a bad row is reported, and the good rows still reach
    /// PR 06, so its FR-5/FR-6 errors arrive in the same run.
    #[test]
    fn a_bad_row_does_not_hide_the_good_ones() {
        let (table, errs, _) = parsed("a,b\n1\n2,3\n");
        assert_eq!(
            errs,
            [format!(
                "error: line 2: the row's field count is 1 but the header's is 2{FEWER}"
            )]
        );
        let table = table.expect("structural");
        assert_eq!(table.records.len(), 1);
        assert_eq!(table.records[0].line, 3);
    }

    #[test]
    fn duplicate_headers_and_bad_rows_are_reported_together() {
        assert_eq!(
            errors("a,A\n1\n"),
            [
                format!("error: line 1: column 2 'A' repeats column 1 'a'{MATCH}"),
                format!("error: line 2: the row's field count is 1 but the header's is 2{FEWER}"),
            ],
            "a duplicate header does not hide a bad row, and comes first"
        );
    }

    /// Malformed quoting stops the run: nothing after it can be trusted.
    #[test]
    fn malformed_quoting_is_the_only_error_reported() {
        assert_eq!(
            errors("a,a\n1\n\"open\n"),
            [
                "error: line 3, column 1: a quote opens this value and never closes; add the closing quote, and write any quote inside the value as \"\""
            ]
        );
    }

    /// FR-90, D-55: removed, with one warning per field naming line and column.
    #[test]
    fn undisplayable_characters_are_removed_and_warned() {
        let (table, _, warnings) = parsed("name,description\nKaf\u{1}ka,ok\n");
        assert_eq!(table.expect("parses").records[0].fields, ["Kafka", "ok"]);
        assert_eq!(
            warnings,
            ["warning: line 2, name: removed U+0001, which the output cannot display"]
        );
        // Several fields group into one line (NFR-20), each naming its column.
        let (table, _, warnings) = parsed("name,description\nKaf\u{1}ka,a\u{7f}b\u{85}c\n");
        assert_eq!(table.expect("parses").records[0].fields, ["Kafka", "abc"]);
        assert_eq!(
            warnings,
            [
                "warning: 2 fields had characters the output cannot display — name on line 2 (U+0001), description on line 2 (2, first U+007F); each was removed"
            ]
        );
    }

    /// PR 02's prefix rule: a row warning names its column in canonical
    /// form, so ` Name ` reads `name`, as FR-6 and FR-28 will.
    #[test]
    fn the_warning_names_the_column_in_canonical_form() {
        let (_, _, warnings) = parsed(" Name ,ring\nKaf\u{1}ka,Adopt\n");
        assert_eq!(
            warnings,
            ["warning: line 2, name: removed U+0001, which the output cannot display"]
        );
    }

    /// FR-90's whole set: NUL and the two noncharacters go too.
    #[test]
    fn nul_and_noncharacters_are_removed() {
        let (table, _, warnings) = parsed("a\n\u{0}x\u{fffe}\u{ffff}y\n");
        assert_eq!(table.expect("parses").records[0].fields, ["xy"]);
        assert_eq!(
            warnings,
            [
                "warning: line 2, a: removed 3 characters the output cannot display, the first U+0000"
            ]
        );
    }

    /// NFR-11 orders a line's warnings by column, so the index must be the
    /// field's, 0-based as `Position::Cell` documents.
    #[test]
    fn a_warning_carries_its_line_and_column_index() {
        let mut w = Warnings::default();
        parse("a,b,c\n1,2\u{1},3\n", &mut w).expect("parses");
        assert!(
            matches!(
                w.as_slice(),
                [Warning::Row {
                    line: 2,
                    column: 1,
                    ..
                }]
            ),
            "{:?}",
            w.as_slice()
        );
    }

    /// SC-10: a cell of only such characters is empty, for FR-6 to reject.
    #[test]
    fn a_cell_of_only_undisplayable_characters_becomes_empty() {
        let (table, _, _) = parsed("name,ring\n\u{1}\u{2},Adopt\n");
        assert_eq!(table.expect("parses").records[0].fields, ["", "Adopt"]);
    }

    /// Review focus 5: removal comes first, so the duplicate is the one the
    /// author sees in any editor that hides the character.
    #[test]
    fn removal_runs_before_the_duplicate_header_check() {
        let (_, errs, warnings) = parsed("name,name\u{1}\nx,y\n");
        assert_eq!(
            errs,
            [format!(
                "error: line 1: column 2 'name' repeats column 1 'name'{MATCH}"
            )]
        );
        assert_eq!(
            warnings,
            ["warning: line 1, name: removed U+0001, which the output cannot display"]
        );
    }

    /// Tab, LF and CR are displayable; they survive.
    #[test]
    fn tab_and_line_breaks_survive_removal() {
        let (table, _, warnings) = parsed("a\n\"x\ty\r\nz\"\n");
        assert_eq!(table.expect("parses").records[0].fields, ["x\ty\r\nz"]);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// A field past the header's end, or under an empty header cell, has no
    /// name to give; it gets its position instead.
    #[test]
    fn an_unnamed_column_is_named_by_position() {
        let (_, _, warnings) = parsed("name,\nx,\u{1}\n");
        assert_eq!(
            warnings,
            ["warning: line 2, column 2: removed U+0001, which the output cannot display"]
        );
    }

    /// Decisions: a field-count error does not hide the row's FR-90
    /// warnings, and a field past the header's end is named by position.
    #[test]
    fn a_short_or_long_row_still_warns_about_removal() {
        let (_, errs, warnings) = parsed("a,b\n\u{1}\nx,\u{2},\u{3}\n");
        assert_eq!(
            errs,
            [
                format!("error: line 2: the row's field count is 1 but the header's is 2{FEWER}"),
                format!("error: line 3: the row's field count is 3 but the header's is 2{MORE}"),
            ]
        );
        assert_eq!(
            warnings,
            [
                "warning: 3 fields had characters the output cannot display — a on line 2 (U+0001), b on line 3 (U+0002), column 3 on line 3 (U+0003); each was removed"
            ]
        );
    }
}
