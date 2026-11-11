//! Find and replace for the code editor, engine-free: every match of a query in a
//! [`CodeBuffer`](crate::CodeBuffer), by plain text or by regular expression, with case and
//! whole-word options, and the edits a replace-all is.
//!
//! A match is inside one line (a query is not matched across a line break). The regular expressions
//! are `regex-lite`'s: the usual syntax (`.`, `*`, `+`, `?`, `[a-z]`, `\d`, `\w`, `\s`, `(..)`, `|`,
//! `^`, `$`, `\b`) with no look-around and no back-references; a replacement may use `$1` and
//! `${name}`. Plain text is matched as typed, and a whole word is a match with no letter, digit or
//! underscore on either side of it.

use std::ops::Range;

use crate::code_buffer::{CodeBuffer, CodeEdit, CodePos};

/// How many matches a search keeps; a text with more is searched no further, so a one-letter query
/// over a huge file stays quick.
pub const FIND_LIMIT: usize = 100_000;

/// What a search matches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindOptions {
    /// Letters match only the same case. Off (the default) ignores it.
    pub case_sensitive: bool,
    /// A match is a whole word: no letter, digit or underscore touches either end.
    pub whole_word: bool,
    /// The query is a regular expression.
    pub regex: bool,
}

/// Why a search could not run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindError {
    /// The regular expression did not compile; the reason, as the engine says it.
    Regex(String),
}

impl std::fmt::Display for FindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Regex(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for FindError {}

/// One match: where it starts and ends, and what a replacement makes of it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Hit {
    start: CodePos,
    end: CodePos,
    replacement: Option<String>,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn same_char(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

/// Every match of plain `query` in `text`, as byte ranges.
fn plain_in(text: &str, query: &str, options: FindOptions) -> Vec<Range<usize>> {
    let needle: Vec<char> = query.chars().collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let hay: Vec<(usize, char)> = text.char_indices().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        let window = &hay[i..i + needle.len()];
        let equal = window.iter().zip(&needle).all(|(&(_, h), &n)| {
            if options.case_sensitive {
                h == n
            } else {
                same_char(h, n)
            }
        });
        if equal {
            let start = hay[i].0;
            let end = hay
                .get(i + needle.len())
                .map_or(text.len(), |&(byte, _)| byte);
            let word = !options.whole_word
                || (i == 0 || !is_word(hay[i - 1].1))
                    && hay.get(i + needle.len()).is_none_or(|&(_, c)| !is_word(c));
            if word {
                found.push(start..end);
                i += needle.len();
                continue;
            }
        }
        i += 1;
    }
    found
}

fn build_regex(query: &str, options: FindOptions) -> Result<regex_lite::Regex, FindError> {
    let source = if options.whole_word {
        format!(r"\b(?:{query})\b")
    } else {
        query.to_owned()
    };
    regex_lite::RegexBuilder::new(&source)
        .case_insensitive(!options.case_sensitive)
        .build()
        .map_err(|error| FindError::Regex(error.to_string()))
}

/// Every hit, with the replacement each makes when `replacement` is given.
fn scan(
    buffer: &CodeBuffer,
    query: &str,
    options: FindOptions,
    replacement: Option<&str>,
) -> Result<Vec<Hit>, FindError> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let regex = options.regex.then(|| build_regex(query, options)).transpose()?;
    let mut hits = Vec::new();
    'lines: for line in 0..buffer.len_lines() {
        let text = buffer.line(line);
        if text.is_empty() && regex.is_none() {
            continue;
        }
        let at = |start: usize, end: usize, replaced: Option<String>| Hit {
            start: CodePos::new(line, start),
            end: CodePos::new(line, end),
            replacement: replaced,
        };
        match &regex {
            Some(regex) => {
                for caps in regex.captures_iter(text) {
                    let Some(whole) = caps.get(0) else {
                        continue;
                    };
                    if whole.is_empty() {
                        continue;
                    }
                    let replaced = replacement.map(|template| {
                        let mut out = String::new();
                        caps.expand(template, &mut out);
                        out
                    });
                    hits.push(at(whole.start(), whole.end(), replaced));
                    if hits.len() >= FIND_LIMIT {
                        break 'lines;
                    }
                }
            }
            None => {
                for range in plain_in(text, query, options) {
                    hits.push(at(
                        range.start,
                        range.end,
                        replacement.map(str::to_owned),
                    ));
                    if hits.len() >= FIND_LIMIT {
                        break 'lines;
                    }
                }
            }
        }
    }
    Ok(hits)
}

/// Every match of `query` in `buffer`, in order, as the places it starts and ends. An empty query
/// matches nothing; a regular expression that does not compile is a [`FindError`].
///
/// ```
/// use bevy_gooey::{CodeBuffer, CodePos, FindOptions, find_all};
/// let buffer = CodeBuffer::new("Foo foo\nfoofoo");
/// let found = find_all(&buffer, "foo", FindOptions::default()).unwrap();
/// assert_eq!(found.len(), 4);
/// let words = FindOptions { whole_word: true, ..FindOptions::default() };
/// assert_eq!(find_all(&buffer, "foo", words).unwrap().len(), 2);
/// assert_eq!(found[1], (CodePos::new(0, 4), CodePos::new(0, 7)));
/// ```
pub fn find_all(
    buffer: &CodeBuffer,
    query: &str,
    options: FindOptions,
) -> Result<Vec<(CodePos, CodePos)>, FindError> {
    Ok(scan(buffer, query, options, None)?
        .into_iter()
        .map(|hit| (hit.start, hit.end))
        .collect())
}

/// The edits a replace-all of `query` by `replacement` is, over byte offsets of the whole text:
/// give them to [`CodeBuffer::replace_ranges`] for one undo step. In a regular expression the
/// replacement may name groups (`$1`, `${name}`).
///
/// ```
/// use bevy_gooey::{CodeBuffer, FindOptions, replace_edits};
/// let mut buffer = CodeBuffer::new("let a = 1;\nlet b = 2;");
/// let options = FindOptions { regex: true, ..FindOptions::default() };
/// let edits = replace_edits(&buffer, r"let (\w+)", "var $1", options).unwrap();
/// buffer.replace_ranges(&edits, 0.0);
/// assert_eq!(buffer.text(), "var a = 1;\nvar b = 2;");
/// assert!(buffer.undo());
/// assert_eq!(buffer.text(), "let a = 1;\nlet b = 2;");
/// ```
pub fn replace_edits(
    buffer: &CodeBuffer,
    query: &str,
    replacement: &str,
    options: FindOptions,
) -> Result<Vec<CodeEdit>, FindError> {
    let hits = scan(buffer, query, options, Some(replacement))?;
    // The start of each line in the whole text, once, so a hit costs a lookup.
    let mut starts = Vec::with_capacity(buffer.len_lines());
    let mut at = 0;
    for line in 0..buffer.len_lines() {
        starts.push(at);
        at += buffer.line_with_break(line).len();
    }
    Ok(hits
        .into_iter()
        .map(|hit| {
            CodeEdit::new(
                starts[hit.start.line] + hit.start.col..starts[hit.end.line] + hit.end.col,
                hit.replacement.unwrap_or_default(),
            )
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(text: &str, query: &str, options: FindOptions) -> usize {
        find_all(&CodeBuffer::new(text), query, options)
            .expect("a query")
            .len()
    }

    #[test]
    fn case_whole_word_and_regex_options() {
        let text = "Foo foo FOO\nfoofoo\n_foo foo_";
        let plain = FindOptions::default();
        assert_eq!(count(text, "foo", plain), 7);
        let case = FindOptions { case_sensitive: true, ..plain };
        assert_eq!(count(text, "foo", case), 5);
        let word = FindOptions { whole_word: true, ..plain };
        assert_eq!(count(text, "foo", word), 3, "Foo, foo and FOO; not foofoo, _foo or foo_");
        let both = FindOptions { case_sensitive: true, whole_word: true, ..plain };
        assert_eq!(count(text, "foo", both), 1);
        let regex = FindOptions { regex: true, ..plain };
        assert_eq!(count(text, r"fo+", regex), 7);
        assert_eq!(count(text, r"^foo", regex), 2);
        assert_eq!(count("", "foo", plain), 0);
        assert_eq!(count(text, "", plain), 0);
        // A bad expression is an error with the engine's reason.
        let bad = find_all(&CodeBuffer::new("x"), "(", regex);
        assert!(matches!(bad, Err(FindError::Regex(_))));
    }

    #[test]
    fn matches_do_not_overlap_and_unicode_offsets_are_bytes() {
        assert_eq!(count("aaaa", "aa", FindOptions::default()), 2);
        let buffer = CodeBuffer::new("héllo hé");
        let found = find_all(&buffer, "hé", FindOptions::default()).expect("found");
        assert_eq!(found[1].0.col, "héllo ".len());
        let edits = replace_edits(&buffer, "é", "e", FindOptions::default()).expect("edits");
        assert_eq!(edits.len(), 2);
        let mut buffer = buffer;
        buffer.replace_ranges(&edits, 0.0);
        assert_eq!(buffer.text(), "hello he");
    }

    #[test]
    fn replace_all_is_one_undo_step_across_lines_and_crlf() {
        let mut buffer = CodeBuffer::new("a\r\nb a\r\na");
        let edits = replace_edits(&buffer, "a", "xyz", FindOptions::default()).expect("edits");
        assert!(buffer.replace_ranges(&edits, 0.0));
        assert_eq!(buffer.text(), "xyz\r\nb xyz\r\nxyz");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "a\r\nb a\r\na");
        assert!(!buffer.undo());
    }
}
