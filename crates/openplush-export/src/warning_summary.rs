//! The warning summary as lines of text: each unresolved warning's row, wrapped to a width in
//! characters and encoded for the PDF's standard fonts.

use crate::ExportError;

/// One unresolved warning as the pipeline renders it: its code, where it is, what is wrong, and
/// the fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRow {
    code: String,
    location: String,
    explanation: String,
    remedy: String,
}

impl SummaryRow {
    /// A row from its four texts.
    pub fn new(code: String, location: String, explanation: String, remedy: String) -> Self {
        Self {
            code,
            location,
            explanation,
            remedy,
        }
    }
}

/// One printed line, in WinAnsi bytes; `indent` when it continues its row.
#[derive(Debug, Clone)]
pub(crate) struct Line {
    pub(crate) indent: bool,
    pub(crate) text: Vec<u8>,
}

impl Line {
    /// An unindented line of ASCII text.
    pub(crate) fn plain(text: &str) -> Self {
        Self {
            indent: false,
            text: text.as_bytes().to_vec(),
        }
    }
}

/// Columns a continuing line is set in by.
pub(crate) const INDENT_COLUMNS: usize = 3;

/// Every row as lines at most `columns` wide: `N. code: location`, then the explanation and
/// `Fix: remedy`, each indented. Runs of whitespace print as one space.
///
/// # Errors
/// `Unprintable` for the first text with a character outside WinAnsi's Latin-1 range.
pub(crate) fn lines(rows: &[SummaryRow], columns: usize) -> Result<Vec<Line>, ExportError> {
    let mut out = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        for text in [&row.code, &row.location, &row.explanation, &row.remedy] {
            printable(text)?;
        }
        let first = format!("{}. {}: {}", i + 1, row.code, row.location);
        let narrow = columns - INDENT_COLUMNS;
        let fix = format!("Fix: {}", row.remedy);
        let parts = [
            (first.as_str(), columns),
            (&row.explanation, narrow),
            (&fix, narrow),
        ];
        for (k, (text, width)) in parts.into_iter().enumerate() {
            for (j, part) in wrap(text, width, narrow).into_iter().enumerate() {
                out.push(encoded(part, k > 0 || j > 0));
            }
        }
    }
    Ok(out)
}

/// Printable ASCII, Latin-1 above 0xA0 (WinAnsi's bytes for them are their code points), or
/// whitespace, which wrapping turns into single spaces.
fn printable(text: &str) -> Result<(), ExportError> {
    let ok = |c: char| (' '..='~').contains(&c) || ('\u{a0}'..='\u{ff}').contains(&c);
    match text.chars().find(|&c| !ok(c) && !c.is_whitespace()) {
        Some(character) => Err(ExportError::Unprintable {
            text: text.to_owned(),
            character,
        }),
        None => Ok(()),
    }
}

/// Greedy word wrap, the first line `first` columns wide and the rest `rest` (no wider); a word
/// longer than `rest` is split across lines.
fn wrap(text: &str, first: usize, rest: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let chars: Vec<char> = word.chars().collect();
        for piece in chars.chunks(rest) {
            let width = if out.is_empty() { first } else { rest };
            let fits = line.chars().count() + 1 + piece.len() <= width;
            if !line.is_empty() && !fits {
                out.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.extend(piece);
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

/// INVARIANT: `printable` passed, and wrapping dropped all whitespace but spaces, so every
/// character is one byte.
fn encoded(text: String, indent: bool) -> Line {
    Line {
        indent,
        text: text.chars().map(|c| c as u8).collect(),
    }
}
