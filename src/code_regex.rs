//! A small regular-expression engine for the code editor's find and replace, engine-free and with
//! no dependency (this crate's dependency list is closed, and a find bar needs the common syntax,
//! not a whole engine).
//!
//! **Syntax:** literals, `.` (any character but a line break), classes (`[abc]`, `[a-z]`, `[^x]`,
//! with `\d`, `\w`, `\s` inside), the escapes `\d \D \w \W \s \S \b \B \t` and an escaped
//! punctuation mark, the anchors `^` and `$`, groups `(..)` and `(?:..)`, alternation `|`, and the
//! quantifiers `*`, `+`, `?`, `{n}`, `{n,}` and `{n,m}`, each lazy with a trailing `?`. There is no
//! look-around and no back-reference. A replacement may use `$1` (a group), `${1}` and `$$`.
//! **A backslash before any other letter or digit is an error**, never a literal: `\1`, `\p`, `\A`,
//! `\x41` are not supported, and `\n` and `\r` are refused because a line is matched without its
//! break, so they could never match.
//!
//! It backtracks, over one line at a time. Work is bounded three ways, and each is a
//! [`MatchError`], never a quiet "no match": one attempt may take [`STEP_LIMIT`] steps, a whole
//! scan has a budget the caller passes, and the recursion may go [`DEPTH_LIMIT`] deep, so a minified
//! line cannot overflow the stack. A repeat of one character (`.*`, `\d+`, `[a-z]*`) is counted,
//! not recursed.

/// How many steps one attempt at one start position may take before it is given up.
pub const STEP_LIMIT: usize = 200_000;

/// How deep the matcher may recurse before it gives up: a repeat of a group or an alternation
/// recurses a few frames an iteration, and a long line must not take the stack with it.
pub const DEPTH_LIMIT: usize = 1_500;

/// Why a scan stopped without an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchError {
    /// An attempt, or the whole scan, used up its steps.
    TooMuchWork,
    /// The pattern recursed past [`DEPTH_LIMIT`] on a long line.
    TooDeep,
}

#[derive(Clone, Debug)]
enum ClassItem {
    Range(char, char),
    Digit(bool),
    Word(bool),
    Space(bool),
}

#[derive(Clone, Debug)]
enum Node {
    Char(char),
    Any,
    Class {
        negated: bool,
        items: Vec<ClassItem>,
    },
    Digit(bool),
    Word(bool),
    Space(bool),
    Start,
    End,
    WordEdge(bool),
    Group(Box<Node>, Option<usize>),
    Alt(Vec<Node>),
    Concat(Vec<Node>),
    Repeat {
        node: Box<Node>,
        min: usize,
        max: usize,
        greedy: bool,
    },
}

/// A compiled expression.
#[derive(Clone, Debug)]
pub struct Regex {
    node: Node,
    groups: usize,
    insensitive: bool,
}

/// One match: byte ranges of the whole match and of each group (`None` for a group that did not
/// take part).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captures {
    ranges: Vec<Option<(usize, usize)>>,
}

impl Captures {
    /// The whole match, as byte offsets.
    pub fn whole(&self) -> (usize, usize) {
        self.ranges[0].unwrap_or((0, 0))
    }

    /// Group `n`'s text in `text`, empty when it did not take part.
    fn group<'a>(&self, text: &'a str, n: usize) -> &'a str {
        match self.ranges.get(n).copied().flatten() {
            Some((start, end)) => &text[start..end],
            None => "",
        }
    }

    /// `template` with `$1`, `${1}` and `$$` replaced, from `text`, the line matched.
    pub fn expand(&self, template: &str, text: &str) -> String {
        let mut out = String::new();
        let mut chars = template.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '$' {
                out.push(c);
                continue;
            }
            match chars.peek().copied() {
                Some('$') => {
                    chars.next();
                    out.push('$');
                }
                Some('{') => {
                    let mut digits = String::new();
                    let mut probe = chars.clone();
                    probe.next();
                    let mut closed = false;
                    for d in probe.by_ref() {
                        if d == '}' {
                            closed = true;
                            break;
                        }
                        digits.push(d);
                    }
                    if closed && let Ok(n) = digits.parse::<usize>() {
                        out.push_str(self.group(text, n));
                        chars = probe;
                    } else {
                        out.push('$');
                    }
                }
                Some(d) if d.is_ascii_digit() => {
                    let mut digits = String::new();
                    while let Some(&d) = chars.peek() {
                        if !d.is_ascii_digit() {
                            break;
                        }
                        digits.push(d);
                        chars.next();
                    }
                    if let Ok(n) = digits.parse::<usize>() {
                        out.push_str(self.group(text, n));
                    }
                }
                _ => out.push('$'),
            }
        }
        out
    }
}

struct Parser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    groups: usize,
}

type Parsed = Result<Node, String>;

impl Parser<'_> {
    fn alt(&mut self) -> Parsed {
        let mut branches = vec![self.concat()?];
        while self.chars.peek() == Some(&'|') {
            self.chars.next();
            branches.push(self.concat()?);
        }
        Ok(if branches.len() == 1 {
            branches.swap_remove(0)
        } else {
            Node::Alt(branches)
        })
    }

    fn concat(&mut self) -> Parsed {
        let mut parts = Vec::new();
        while let Some(&c) = self.chars.peek() {
            if c == '|' || c == ')' {
                break;
            }
            parts.push(self.repeat()?);
        }
        Ok(Node::Concat(parts))
    }

    fn number(&mut self) -> Option<usize> {
        let mut digits = String::new();
        while let Some(&d) = self.chars.peek() {
            if !d.is_ascii_digit() {
                break;
            }
            digits.push(d);
            self.chars.next();
        }
        digits.parse().ok()
    }

    fn repeat(&mut self) -> Parsed {
        let atom = self.atom()?;
        let (min, max) = match self.chars.peek().copied() {
            Some('*') => (0, usize::MAX),
            Some('+') => (1, usize::MAX),
            Some('?') => (0, 1),
            Some('{') => {
                self.chars.next();
                let min = self.number().ok_or("a repeat count is missing")?;
                let max = if self.chars.peek() == Some(&',') {
                    self.chars.next();
                    self.number().unwrap_or(usize::MAX)
                } else {
                    min
                };
                if self.chars.next() != Some('}') {
                    return Err("a repeat is missing its closing brace".into());
                }
                if max < min {
                    return Err("a repeat's largest count is under its smallest".into());
                }
                return self.lazy(atom, min, max);
            }
            _ => return Ok(atom),
        };
        self.chars.next();
        self.lazy(atom, min, max)
    }

    fn lazy(&mut self, atom: Node, min: usize, max: usize) -> Parsed {
        let greedy = if self.chars.peek() == Some(&'?') {
            self.chars.next();
            false
        } else {
            true
        };
        Ok(Node::Repeat {
            node: Box::new(atom),
            min,
            max,
            greedy,
        })
    }

    fn escape(&mut self) -> Result<char, String> {
        self.chars
            .next()
            .ok_or_else(|| "a backslash ends the expression".to_owned())
    }

    fn atom(&mut self) -> Parsed {
        let c = self.chars.next().ok_or("the expression ended early")?;
        Ok(match c {
            '.' => Node::Any,
            '^' => Node::Start,
            '$' => Node::End,
            '*' | '+' | '?' => return Err(format!("`{c}` has nothing to repeat")),
            '(' => {
                let capture = if self.chars.peek() == Some(&'?') {
                    self.chars.next();
                    if self.chars.next() != Some(':') {
                        return Err("only `(?:` groups are supported".into());
                    }
                    None
                } else {
                    self.groups += 1;
                    Some(self.groups)
                };
                let inner = self.alt()?;
                if self.chars.next() != Some(')') {
                    return Err("a group is missing its closing parenthesis".into());
                }
                Node::Group(Box::new(inner), capture)
            }
            ')' => return Err("a closing parenthesis has no opening one".into()),
            '[' => self.class()?,
            '\\' => match self.escape()? {
                'd' => Node::Digit(true),
                'D' => Node::Digit(false),
                'w' => Node::Word(true),
                'W' => Node::Word(false),
                's' => Node::Space(true),
                'S' => Node::Space(false),
                'b' => Node::WordEdge(true),
                'B' => Node::WordEdge(false),
                other => Node::Char(literal(other)?),
            },
            other => Node::Char(other),
        })
    }

    fn class(&mut self) -> Parsed {
        let negated = if self.chars.peek() == Some(&'^') {
            self.chars.next();
            true
        } else {
            false
        };
        let mut items = Vec::new();
        let mut first = true;
        loop {
            let c = self
                .chars
                .next()
                .ok_or("a class is missing its closing bracket")?;
            if c == ']' && !first {
                break;
            }
            first = false;
            let lo = match c {
                '\\' => match self.escape()? {
                    'd' => {
                        items.push(ClassItem::Digit(true));
                        continue;
                    }
                    'D' => {
                        items.push(ClassItem::Digit(false));
                        continue;
                    }
                    'w' => {
                        items.push(ClassItem::Word(true));
                        continue;
                    }
                    'W' => {
                        items.push(ClassItem::Word(false));
                        continue;
                    }
                    's' => {
                        items.push(ClassItem::Space(true));
                        continue;
                    }
                    'S' => {
                        items.push(ClassItem::Space(false));
                        continue;
                    }
                    other => literal(other)?,
                },
                other => other,
            };
            let mut ahead = self.chars.clone();
            if ahead.next() == Some('-') && ahead.peek().is_some_and(|&e| e != ']') {
                self.chars.next();
                let hi = match self.chars.next() {
                    Some('\\') => literal(self.escape()?)?,
                    Some(h) => h,
                    None => return Err("a class is missing its closing bracket".into()),
                };
                if hi < lo {
                    return Err("a class range runs backwards".into());
                }
                items.push(ClassItem::Range(lo, hi));
            } else {
                items.push(ClassItem::Range(lo, lo));
            }
        }
        Ok(Node::Class { negated, items })
    }
}

/// The character a backslash before `c` stands for, outside the classes the parser handles: a tab,
/// or an escaped mark. A letter or digit the engine does not know is refused, not taken as itself.
fn literal(c: char) -> Result<char, String> {
    match c {
        't' => Ok('\t'),
        'n' | 'r' => Err(format!(
            "`\\{c}` cannot match: a search looks at one line at a time, without its break"
        )),
        c if c.is_alphanumeric() => Err(format!("`\\{c}` is not supported")),
        c => Ok(c),
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The state of one attempt.
struct State<'a> {
    chars: &'a [char],
    caps: Vec<Option<(usize, usize)>>,
    steps: usize,
    /// Steps this attempt may take: [`STEP_LIMIT`], or what the scan has left.
    allowed: usize,
    depth: usize,
    failed: Option<MatchError>,
    insensitive: bool,
}

type Next<'k, 'a> = &'k mut dyn FnMut(usize, &mut State<'a>) -> bool;

impl State<'_> {
    fn same(&self, a: char, b: char) -> bool {
        a == b || (self.insensitive && a.to_lowercase().eq(b.to_lowercase()))
    }

    fn class_has(&self, items: &[ClassItem], c: char) -> bool {
        let tests = |c: char| {
            items.iter().any(|item| match *item {
                ClassItem::Range(lo, hi) => (lo..=hi).contains(&c),
                ClassItem::Digit(yes) => c.is_ascii_digit() == yes,
                ClassItem::Word(yes) => is_word(c) == yes,
                ClassItem::Space(yes) => c.is_whitespace() == yes,
            })
        };
        tests(c)
            || (self.insensitive && (c.to_lowercase().any(&tests) || c.to_uppercase().any(&tests)))
    }

    /// Whether `node`, a one-character node, matches the character at `i`.
    fn one(&self, node: &Node, i: usize) -> bool {
        let Some(&c) = self.chars.get(i) else {
            return false;
        };
        match node {
            Node::Char(want) => self.same(c, *want),
            Node::Any => c != '\n',
            Node::Class { negated, items } => self.class_has(items, c) != *negated,
            Node::Digit(yes) => c.is_ascii_digit() == *yes,
            Node::Word(yes) => is_word(c) == *yes,
            Node::Space(yes) => c.is_whitespace() == *yes,
            _ => false,
        }
    }
}

fn single(node: &Node) -> bool {
    matches!(
        node,
        Node::Char(_)
            | Node::Any
            | Node::Class { .. }
            | Node::Digit(_)
            | Node::Word(_)
            | Node::Space(_)
    )
}

fn go<'a>(node: &Node, i: usize, st: &mut State<'a>, k: Next<'_, 'a>) -> bool {
    st.steps += 1;
    if st.failed.is_some() {
        return false;
    }
    if st.steps > st.allowed {
        st.failed = Some(MatchError::TooMuchWork);
        return false;
    }
    if st.depth >= DEPTH_LIMIT {
        st.failed = Some(MatchError::TooDeep);
        return false;
    }
    st.depth += 1;
    let matched = step(node, i, st, k);
    st.depth -= 1;
    matched
}

fn step<'a>(node: &Node, i: usize, st: &mut State<'a>, k: Next<'_, 'a>) -> bool {
    match node {
        Node::Char(_)
        | Node::Any
        | Node::Class { .. }
        | Node::Digit(_)
        | Node::Word(_)
        | Node::Space(_) => st.one(node, i) && k(i + 1, st),
        Node::Start => i == 0 && k(i, st),
        Node::End => i == st.chars.len() && k(i, st),
        Node::WordEdge(want) => {
            let before = i > 0 && is_word(st.chars[i - 1]);
            let after = st.chars.get(i).copied().is_some_and(is_word);
            ((before != after) == *want) && k(i, st)
        }
        Node::Group(inner, capture) => match capture {
            None => go(inner, i, st, k),
            Some(index) => {
                let index = *index;
                go(inner, i, st, &mut |j, st| {
                    let old = st.caps[index];
                    st.caps[index] = Some((i, j));
                    if k(j, st) {
                        true
                    } else {
                        st.caps[index] = old;
                        false
                    }
                })
            }
        },
        Node::Alt(branches) => branches.iter().any(|branch| go(branch, i, st, k)),
        Node::Concat(parts) => seq(parts, i, st, k),
        Node::Repeat {
            node,
            min,
            max,
            greedy,
        } => {
            if single(node) {
                // Count how far the repeat can run, then try the lengths in order: no recursion.
                let mut run = 0;
                while run < *max && st.one(node, i + run) {
                    run += 1;
                }
                if run < *min {
                    return false;
                }
                if *greedy {
                    (*min..=run).rev().any(|n| k(i + n, st))
                } else {
                    (*min..=run).any(|n| k(i + n, st))
                }
            } else {
                rep(node, *min, *max, *greedy, 0, i, st, k)
            }
        }
    }
}

fn seq<'a>(parts: &[Node], i: usize, st: &mut State<'a>, k: Next<'_, 'a>) -> bool {
    match parts.split_first() {
        None => k(i, st),
        Some((first, rest)) => go(first, i, st, &mut |j, st| seq(rest, j, st, k)),
    }
}

// The order of the two tries is the engine's (greedy or lazy), so the branches only look alike.
#[allow(clippy::too_many_arguments, clippy::if_same_then_else)]
fn rep<'a>(
    node: &Node,
    min: usize,
    max: usize,
    greedy: bool,
    count: usize,
    i: usize,
    st: &mut State<'a>,
    k: Next<'_, 'a>,
) -> bool {
    let more = |st: &mut State<'a>, k: Next<'_, 'a>| {
        count < max
            && go(node, i, st, &mut |j, st| {
                // A pass that consumed nothing past the minimum cannot help: stop the loop.
                (j != i || count < min) && rep(node, min, max, greedy, count + 1, j, st, k)
            })
    };
    if greedy {
        more(st, k) || (count >= min && k(i, st))
    } else {
        (count >= min && k(i, st)) || more(st, k)
    }
}

impl Regex {
    /// Compiles `pattern`; `insensitive` ignores case. The reason is given when it cannot be.
    pub fn new(pattern: &str, insensitive: bool) -> Result<Self, String> {
        let mut parser = Parser {
            chars: pattern.chars().peekable(),
            groups: 0,
        };
        let node = parser.alt()?;
        if parser.chars.peek().is_some() {
            return Err("a closing parenthesis has no opening one".into());
        }
        Ok(Self {
            node,
            groups: parser.groups,
            insensitive,
        })
    }

    /// The expression with a word boundary on each side.
    pub fn whole_word(pattern: &str, insensitive: bool) -> Result<Self, String> {
        let inner = Self::new(pattern, insensitive)?;
        Ok(Self {
            node: Node::Concat(vec![
                Node::WordEdge(true),
                Node::Group(Box::new(inner.node), None),
                Node::WordEdge(true),
            ]),
            ..inner
        })
    }

    /// Every non-overlapping, non-empty match in `text`, in order. `budget` is the steps the whole
    /// scan has left: this takes what it uses from it, and gives [`MatchError::TooMuchWork`] when an
    /// attempt, or the scan, runs it out.
    pub fn captures(&self, text: &str, budget: &mut usize) -> Result<Vec<Captures>, MatchError> {
        let chars: Vec<char> = text.chars().collect();
        // The byte offset of each character, and of the end.
        let mut bytes: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
        bytes.push(text.len());
        let mut found = Vec::new();
        let mut at = 0;
        while at <= chars.len() {
            let mut st = State {
                chars: &chars,
                caps: vec![None; self.groups + 1],
                steps: 0,
                allowed: STEP_LIMIT.min(*budget),
                depth: 0,
                failed: None,
                insensitive: self.insensitive,
            };
            let mut end = None;
            let matched = go(&self.node, at, &mut st, &mut |j, st| {
                end = Some(j);
                st.caps[0] = Some((at, j));
                true
            });
            *budget = budget.saturating_sub(st.steps);
            if let Some(why) = st.failed {
                return Err(why);
            }
            match (matched, end) {
                (true, Some(end)) if end > at => {
                    let ranges = st
                        .caps
                        .iter()
                        .map(|range| range.map(|(a, b)| (bytes[a], bytes[b])))
                        .collect();
                    found.push(Captures { ranges });
                    at = end;
                }
                _ => at += 1,
            }
        }
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A budget nothing here runs out of.
    fn plenty() -> usize {
        usize::MAX
    }

    fn spans(pattern: &str, text: &str) -> Vec<(usize, usize)> {
        Regex::new(pattern, false)
            .expect("a pattern")
            .captures(text, &mut plenty())
            .expect("within budget")
            .iter()
            .map(Captures::whole)
            .collect()
    }

    #[test]
    fn the_common_syntax_matches() {
        assert_eq!(spans("fo+", "fo foo f"), vec![(0, 2), (3, 6)]);
        assert_eq!(spans(r"\d+", "a1 b22 c333"), vec![(1, 2), (4, 6), (8, 11)]);
        assert_eq!(spans("^ab", "ab ab"), vec![(0, 2)]);
        assert_eq!(spans("b$", "ab ab"), vec![(4, 5)]);
        assert_eq!(spans("a|bc", "abc"), vec![(0, 1), (1, 3)]);
        assert_eq!(
            spans("[a-c]x?", "ax bx cc d"),
            vec![(0, 2), (3, 5), (6, 7), (7, 8)]
        );
        assert_eq!(spans("[^ ]+", "ab cd"), vec![(0, 2), (3, 5)]);
        assert_eq!(spans(r"\bcat\b", "cat concat cat"), vec![(0, 3), (11, 14)]);
        assert_eq!(spans("a{2,3}", "a aa aaaa"), vec![(2, 4), (5, 8)]);
        assert_eq!(spans("(?:ab)+", "ababab x"), vec![(0, 6)]);
        assert_eq!(spans("a.*?b", "axxbxxb"), vec![(0, 4)]);
        assert_eq!(spans("a.*b", "axxbxxb"), vec![(0, 7)]);
    }

    #[test]
    fn case_unicode_and_empty_matches() {
        let re = Regex::new("hé", true).expect("pattern");
        let found = re.captures("HÉ hé", &mut plenty()).expect("within budget");
        assert_eq!(
            found.iter().map(Captures::whole).collect::<Vec<_>>(),
            vec![(0, 3), (4, 7)]
        );
        // An expression that can match nothing finds only what it consumes.
        assert_eq!(spans("x*", "axxb"), vec![(1, 3)]);
    }

    #[test]
    fn groups_and_replacements() {
        let re = Regex::new(r"let (\w+) = (\d+)", false).expect("pattern");
        let text = "let a = 1; let bb = 22";
        let found = re.captures(text, &mut plenty()).expect("within budget");
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].expand("$2 -> ${1}$$", text), "22 -> bb$");
        assert_eq!(found[0].expand("$9|$x", text), "|$x");
    }

    #[test]
    fn a_bad_pattern_says_why_and_a_long_line_does_not_overflow_the_stack() {
        for bad in ["(", ")", "*a", "[a", "a{2", "a{3,1}", "(?=x)", "\\"] {
            assert!(Regex::new(bad, false).is_err(), "{bad:?}");
        }
        let long = "x".repeat(200_000);
        assert_eq!(spans(".*", &long), vec![(0, 200_000)]);
    }

    #[test]
    fn an_escape_the_engine_does_not_know_is_refused_not_taken_as_a_literal() {
        for bad in [
            r"\1", r"\p{L}", r"\A", r"\z", r"\x41", r"\k<a>", r"\<", r"[\1]", r"[a-\x]",
        ] {
            // `\<` is punctuation: it is a literal, the others are errors.
            let built = Regex::new(bad, false);
            if bad == r"\<" {
                assert!(built.is_ok());
            } else {
                let why = built.expect_err(bad);
                assert!(why.contains("not supported"), "{bad}: {why}");
            }
        }
        for line_break in [r"a\nb", r"\r", r"[^\n]"] {
            let why = Regex::new(line_break, false).expect_err(line_break);
            assert!(why.contains("one line at a time"), "{line_break}: {why}");
        }
        assert_eq!(spans(r"a\.b\t", "a.b\t axb\t"), vec![(0, 4)]);
    }

    #[test]
    fn work_is_bounded_and_the_bound_is_an_error_not_no_match() {
        let re = Regex::new("(a*)*b", false).expect("pattern");
        let text = "a".repeat(60);
        assert_eq!(
            re.captures(&text, &mut plenty()),
            Err(MatchError::TooMuchWork),
            "one attempt is cut off at STEP_LIMIT"
        );
        // A scan budget is spent across lines: the second call finds it gone.
        let plain = Regex::new(r"\d+", false).expect("pattern");
        let mut budget = 30;
        assert!(plain.captures("1 22 333", &mut budget).is_ok());
        assert!(budget < 30, "what a line uses comes off the budget");
        assert_eq!(
            plain.captures(&"1 ".repeat(500), &mut budget),
            Err(MatchError::TooMuchWork)
        );
    }

    #[test]
    fn a_long_line_under_a_recursing_pattern_is_an_error_not_a_stack_overflow() {
        let long = "ab".repeat(20_000);
        let re = Regex::new("(?:ab)+", false).expect("pattern");
        assert_eq!(re.captures(&long, &mut plenty()), Err(MatchError::TooDeep));
        let alt = Regex::new("(a|b)*", false).expect("pattern");
        assert_eq!(alt.captures(&long, &mut plenty()), Err(MatchError::TooDeep));
        // Short enough, it is just a match.
        assert_eq!(spans("(?:ab)+", &"ab".repeat(100)), vec![(0, 200)]);
    }
}
