use std::fmt::Write as _;

use super::scan::EXCERPT_CHARS;
use super::*;
use crate::decode::{decode, line_at};
use crate::palette::{Role, Theme};

/// Every role at its built-in light value, one per line (lines 2 to 9),
/// then `series` from line 10, inside one rule.
fn file(series: &str) -> String {
    let light = Palette::built_in(Theme::Light);
    let mut text = String::from(":root {\n");
    for role in Neutral::ALL {
        let value = light.value(Role::Neutral(role));
        writeln!(text, "  {role}: {value};").expect("writes");
    }
    text + series + "}\n"
}

fn errors(text: &str) -> Vec<PaletteError> {
    parse(text).expect_err("refused")
}

/// Every role missing, then `--series-1`: what a file with no accepted
/// rule reports after its own errors.
fn all_missing() -> Vec<PaletteError> {
    Neutral::ALL
        .into_iter()
        .map(|role| PaletteError::MissingRole { role })
        .chain([PaletteError::MissingSeries { from: 1, to: 1 }])
        .collect()
}

fn then_all_missing(first: PaletteError) -> Vec<PaletteError> {
    std::iter::once(first).chain(all_missing()).collect()
}

#[test]
fn a_complete_rule_parses() {
    let p = parse(&file("  --series-1: #0072B2;\n")).expect("parses");
    assert_eq!(p.series_len(), 1);
    assert_eq!(p.value(Role::Series(0)), "#0072B2");
}

/// FR-87, SC-21: the count is read from the file, five or twelve.
#[test]
fn the_series_is_as_long_as_the_file_says() {
    for count in [5, 12] {
        let mut series = String::new();
        for i in 1..=count {
            writeln!(series, "  --series-{i}: #0000{i:02X};").expect("writes");
        }
        let p = parse(&file(&series)).expect("parses");
        assert_eq!(p.series_len(), count);
    }
}

#[test]
fn values_are_stored_uppercased() {
    let p = parse(&file("  --series-1: #00a1b2;\n")).expect("parses");
    assert_eq!(p.value(Role::Series(0)), "#00A1B2");
}

/// FR-102: comments anywhere, including between a value and its `;`,
/// and holding the characters the grammar reads.
#[test]
fn comments_count_as_whitespace() {
    let text = file("  --series-1: /* blue; } { */ #0072B2 /* dark: same */;\n");
    assert!(parse(&format!("/* head\n */{text}/* tail */")).is_ok());
    // A comment separates tokens, as CSS's does, so it cannot join them.
    assert_eq!(
        errors(&file(
            "  --series-1: #000001;\n  --ser/**/ies-2: #000002;\n"
        )),
        [PaletteError::Unknown {
            line: 11,
            name: "--ser    ies-2".into()
        }]
    );
}

#[test]
fn root_matches_in_any_ascii_case_and_the_last_semicolon_is_optional() {
    let text = file("  --series-1: #0072B2\n").replacen(":root", ":ROOT", 1);
    assert!(parse(&text).is_ok());
    assert!(
        parse(&file("  --series-1: #0072B2;;\n")).is_ok(),
        "an empty declaration is ignored"
    );
}

/// Review Focus 1: Notepad's CRLF and BOM, decoded as `read` decodes.
#[test]
fn crlf_and_a_bom_parse_and_count_lines() {
    let text = file("  --series-1: #0072B2;\n  --nope: #000000;\n").replace('\n', "\r\n");
    let bytes = [b"\xEF\xBB\xBF".as_slice(), text.as_bytes()].concat();
    let decoded = decode(&bytes).expect("UTF-8");
    assert_eq!(
        errors(&decoded),
        [PaletteError::Unknown {
            line: 11,
            name: "--nope".into()
        }]
    );
}

/// FR-102: each grammar error is named and located, and is the only
/// error when the rest of the file is sound.
#[test]
fn each_grammar_error_is_named_and_located() {
    let sound = file("  --series-1: #000001;\n");
    let cases = [
        (
            format!("{sound}/* open"),
            vec![PaletteError::UnclosedComment { line: 12 }],
        ),
        (
            format!("{sound}html {{ }}\n"),
            vec![PaletteError::OtherRule {
                line: 12,
                selector: "html".into(),
            }],
        ),
        (
            format!("{sound}:root{{}}\n"),
            vec![PaletteError::SecondRoot { line: 12 }],
        ),
        (
            format!("{sound}hello\n"),
            vec![PaletteError::TextOutside {
                line: 12,
                text: "hello".into(),
            }],
        ),
        (
            format!("{sound}}}\n"),
            vec![PaletteError::TextOutside {
                line: 12,
                text: "}".into(),
            }],
        ),
        (
            file("  --series-1: #000001;\n  oops\n"),
            vec![PaletteError::Malformed {
                line: 11,
                text: "oops".into(),
            }],
        ),
        (
            file("  --series-1: #000001;\n  : #FFFFFF;\n"),
            vec![PaletteError::Malformed {
                line: 11,
                text: ": #FFFFFF".into(),
            }],
        ),
        (
            format!("{sound}@media print {{ :root {{ --focus: #000000; }} }}\n"),
            vec![PaletteError::OtherRule {
                line: 12,
                selector: "@media print".into(),
            }],
        ),
    ];
    for (text, want) in cases {
        assert_eq!(errors(&text), want, "{text:?}");
    }
}

/// Plan RG-018A R7: what precedes `:root` is reported and the rule kept.
#[test]
fn a_preamble_does_not_swallow_the_rule() {
    let sound = file("  --series-1: #000001;\n");
    for (preamble, want) in [
        (
            "@charset \"UTF-8\";\n",
            PaletteError::OtherRule {
                line: 1,
                selector: "@charset \"UTF-8\";".into(),
            },
        ),
        (
            "@import url(\"x.css\");\n",
            PaletteError::OtherRule {
                line: 1,
                selector: "@import url(\"x.css\");".into(),
            },
        ),
        (
            "hello\n",
            PaletteError::TextOutside {
                line: 1,
                text: "hello".into(),
            },
        ),
    ] {
        assert_eq!(
            errors(&format!("{preamble}{sound}")),
            [want],
            "{preamble:?}"
        );
    }
}

/// FR-102: a rule that never closes takes its declarations with it.
#[test]
fn an_unclosed_rule_is_named_where_it_opens() {
    assert_eq!(
        errors("\n\n:root {\n  --focus: #000000;\n"),
        then_all_missing(PaletteError::UnclosedRule { line: 3 })
    );
}

#[test]
fn a_selector_list_is_another_rule() {
    assert_eq!(
        errors(":root, html { }\n"),
        then_all_missing(PaletteError::OtherRule {
            line: 1,
            selector: ":root, html".into()
        })
    );
}

#[test]
fn an_empty_file_defines_nothing() {
    assert_eq!(errors(""), all_missing());
    assert_eq!(errors("/* only a comment */\n"), all_missing());
}

#[test]
fn unknown_duplicate_and_malformed_values_are_named() {
    let all = errors(&file(
        "  --series-1: #0072B2;\n  --Surface-1: #FFFFFF;\n  --series-01: #000000;\n  --series-0: #000000;\n  --series-1: #111111;\n  --series-2: rgb(0 0 0);\n  --series-3: #0072B2 !important;\n",
    ));
    let first = 10;
    assert_eq!(
        all,
        [
            PaletteError::Unknown {
                line: first + 1,
                name: "--Surface-1".into()
            },
            PaletteError::Unknown {
                line: first + 2,
                name: "--series-01".into()
            },
            PaletteError::Unknown {
                line: first + 3,
                name: "--series-0".into()
            },
            PaletteError::Duplicate {
                line: first + 4,
                name: "--series-1".into(),
                first
            },
            PaletteError::NotHex {
                line: first + 5,
                name: "--series-2".into(),
                value: "rgb(0 0 0)".into()
            },
            PaletteError::NotHex {
                line: first + 6,
                name: "--series-3".into(),
                value: "#0072B2 !important".into()
            },
        ]
    );
}

/// SC-21: the old two-block sample is refused at its dark rule.
#[test]
fn a_second_block_is_refused_at_its_line() {
    let text = include_str!("../../../tests/fixtures/palette-two-block.css");
    assert_eq!(
        errors(text),
        [PaletteError::OtherRule {
            line: 59,
            selector: ":root[data-theme=\"dark\"]".into()
        }]
    );
}

/// A property declared with a bad value is reported once, not again as
/// missing.
#[test]
fn missing_roles_and_series_come_last_in_order() {
    let all = errors(":root {\n  --focus: nope;\n}\n");
    let mut want = vec![PaletteError::NotHex {
        line: 2,
        name: "--focus".into(),
        value: "nope".into(),
    }];
    want.extend(
        Neutral::ALL
            .into_iter()
            .filter(|role| *role != Neutral::Focus)
            .map(|role| PaletteError::MissingRole { role }),
    );
    want.push(PaletteError::MissingSeries { from: 1, to: 1 });
    assert_eq!(all, want);
}

/// NFR-16: sorted by line, though the grammar pass finds the later one
/// first.
#[test]
fn errors_are_sorted_by_line() {
    let text = format!("{}html {{ }}\n", file("  --series-1: nope;\n"));
    assert_eq!(
        errors(&text),
        [
            PaletteError::NotHex {
                line: 10,
                name: "--series-1".into(),
                value: "nope".into()
            },
            PaletteError::OtherRule {
                line: 12,
                selector: "html".into()
            },
        ]
    );
}

/// FR-102: only the accepted rule counts; a swallowed rule leaves every
/// role missing.
#[test]
fn a_rejected_rule_declares_nothing() {
    let all = errors(&format!("html {{ --series-1: #000000; }}\n{}", file("")));
    assert_eq!(
        all,
        [
            PaletteError::OtherRule {
                line: 1,
                selector: "html".into()
            },
            PaletteError::MissingSeries { from: 1, to: 1 },
        ]
    );
    assert_eq!(
        errors(&format!("/*{}", file("  --series-1: #000000;\n"))),
        then_all_missing(PaletteError::UnclosedComment { line: 1 })
    );
}

/// Review Focus 2, plan RG-018A Ruling 3.
#[test]
fn a_gap_is_one_error_per_run() {
    let all = errors(&file(
        "  --series-1: #000001;\n  --series-3: #000003;\n  --series-1000000: #000004;\n",
    ));
    assert_eq!(
        all,
        [
            PaletteError::MissingSeries { from: 2, to: 2 },
            PaletteError::MissingSeries {
                from: 4,
                to: 999_999
            },
        ]
    );
    assert_eq!(
        all.last().map(ToString::to_string).as_deref(),
        Some(
            "--series-4 to --series-999999 are missing; number series entries from --series-1 with no gaps"
        )
    );
}

/// Plan RG-018A R6.
#[test]
fn an_index_at_usize_max_does_not_panic() {
    let all = errors(&file(
        "  --series-1: #000001;\n  --series-18446744073709551615: #000002;\n",
    ));
    assert_eq!(
        all,
        [PaletteError::MissingSeries {
            from: 2,
            to: usize::MAX - 1
        }]
    );
}

/// Review Focus 5, plan RG-018A R5: every echoed field is cut to 40
/// characters on a character boundary.
#[test]
fn long_and_non_ascii_text_is_quoted_safely() {
    let long = "ä".repeat(100);
    let cut = format!("{}…", "ä".repeat(EXCERPT_CHARS));
    assert_eq!(
        errors(&format!(
            "{}{long} {{ }}\n",
            file("  --series-1: #000001;\n")
        )),
        [PaletteError::OtherRule {
            line: 12,
            selector: cut.clone()
        }]
    );
    let name = format!("--{long}");
    let cut_name = format!("--{}…", "ä".repeat(EXCERPT_CHARS - 2));
    assert_eq!(
        errors(&file(&format!(
            "  --series-1: #000001;\n  {name}: #000000;\n"
        ))),
        [PaletteError::Unknown {
            line: 11,
            name: cut_name
        }]
    );
    assert_eq!(
        errors(&file(&format!("  --series-1: {long};\n"))),
        [PaletteError::NotHex {
            line: 10,
            name: "--series-1".into(),
            value: cut
        }]
    );
}

/// The likeliest typo: a forgotten `;` quotes the value's first line,
/// not the next declaration.
#[test]
fn a_forgotten_semicolon_quotes_the_value_alone() {
    assert_eq!(
        errors(&file("  --series-1: #000001\n  --series-2: #000002;\n")),
        [PaletteError::NotHex {
            line: 10,
            name: "--series-1".into(),
            value: "#000001…".into()
        }]
    );
}

/// Plan RG-018A R3: a large file is linear, not quadratic.
#[test]
fn a_large_file_is_read_in_linear_time() {
    let text = [":root {", &";".repeat(1 << 20), "}"].concat();
    let start = std::time::Instant::now();
    assert_eq!(errors(&text), all_missing());
    assert!(
        start.elapsed() < std::time::Duration::from_secs(5),
        "{:?}",
        start.elapsed()
    );
}

/// `Lines` agrees with `decode::line_at`, the reference, at every
/// offset that is not itself a line break.
#[test]
fn line_index_agrees_with_line_at() {
    let text = "a\nb\r\nc\rd\r\n\r\ne\n\nf";
    let lines = Lines::of(text);
    for (at, c) in text.char_indices() {
        if c != '\n' && c != '\r' {
            assert_eq!(lines.at(at), line_at(text.as_bytes(), at), "{at}");
        }
    }
}

#[test]
fn the_property_list_names_every_role() {
    for role in Neutral::ALL {
        assert!(PROPERTIES.contains(role.as_str()), "{role}");
    }
}

/// Plan RG-018A R4: a file past the cap is refused before it is decoded.
#[test]
fn read_refuses_a_file_past_the_cap() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("big.css");
    std::fs::write(&path, vec![b' '; MAX_BYTES + 1]).expect("writes");
    assert!(matches!(read(&path), Err(ReadError::TooLarge)));
    std::fs::write(&path, vec![b' '; MAX_BYTES]).expect("writes");
    assert!(
        matches!(read(&path), Err(ReadError::Invalid(_))),
        "at the cap"
    );
}

#[test]
fn read_names_what_kept_it_from_the_text() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(matches!(
        read(&dir.path().join("missing.css")),
        Err(ReadError::Unreadable(_))
    ));
    let path = dir.path().join("latin1.css");
    std::fs::write(&path, b":root {\n  --focus: #\xFF;\n}\n").expect("writes");
    assert!(matches!(read(&path), Err(ReadError::NotUtf8 { line: 2 })));
}
