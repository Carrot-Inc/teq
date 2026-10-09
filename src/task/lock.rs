//! `teq.lock`'s format (docs/TARGETS.md, "The export and the project verbs"): a subset of YAML 1.2, read
//! into a tree and written in its canonical form. A reader takes the subset alone and refuses the
//! rest with its line, so that whatever it accepts a YAML reader reads to the same tree. The
//! module depends on the standard library alone, so that the Zed extension holds a copy of it
//! (`integrations/zed/src/lock.rs`, which a test holds equal to this file).
#![allow(dead_code)]

use std::fmt;

pub const FILE: &str = "teq.lock";
/// The format this reader reads and the writer writes, the lock's second line.
pub const FORMAT: i64 = 1;
/// YAML's bound on an implicit key, in characters as written.
pub const MAX_KEY: usize = 1024;
/// How deep blocks and flow mappings may nest.
const MAX_DEPTH: usize = 64;
/// The largest integer a reader takes, JavaScript's largest exact one, so that every reader reads
/// the same number.
const MAX_INT: i64 = (1 << 53) - 1;

/// The root's keys that come first, in this order: the header a launcher reads.
const HEADER: [&str; 3] = ["teq", "format", "binaries"];

/// Where a record is written on one line as a flow mapping, unless it holds a list: a
/// classpath's entries and a stage layer's mappings, by the keys above them (`*` any key or
/// index; sbt-teq's `Lock.Records`).
const RECORDS: [&[&str]; 2] = [&["projects", "*", "configurations", "*", "classpath", "*"], &["projects", "*", "stage", "layers", "*", "*"]];

/// The words a YAML reader may take for null or a truth value, YAML 1.1's among them, and the
/// core schema's infinities and not-a-number.
const RESERVED: [&str; 31] = [
    "null", "Null", "NULL", "true", "True", "TRUE", "false", "False", "FALSE", "yes", "Yes", "YES", "no", "No", "NO", "on", "On", "ON", "off", "Off", "OFF", "y", "Y", "n", "N", ".inf", ".Inf", ".INF", ".nan",
    ".NaN", ".NAN",
];

/// A node of the tree: a mapping keeps its entries in the order of the text.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Map(Vec<(String, Value)>),
    List(Vec<Value>),
    Str(String),
    Int(i64),
    Bool(bool),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries().iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn at(&self, path: &[&str]) -> Option<&Value> {
        path.iter().try_fold(self, |v, k| v.get(k))
    }

    /// A mapping's entries; none for any other node.
    pub fn entries(&self) -> &[(String, Value)] {
        match self {
            Value::Map(entries) => entries,
            _ => &[],
        }
    }

    /// A list's elements; none for any other node.
    pub fn items(&self) -> &[Value] {
        match self {
            Value::List(items) => items,
            _ => &[],
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn uint(&self) -> Option<u32> {
        self.int().and_then(|n| u32::try_from(n).ok())
    }

    pub fn bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Why a text is no lock of this format, and the line (from 1) where it shows.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

fn error<T>(line: usize, message: impl Into<String>) -> Result<T, Error> {
    Err(Error { line, message: message.into() })
}

/// A word of a plain string: ASCII letters, digits and `_ . / + - : @`, led by a letter, a digit,
/// `_`, `.` or `/`, and not ended by `:`.
pub fn word(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.iter().all(|c| c.is_ascii_alphanumeric() || b"_./+-:@".contains(c))
        && (b[0].is_ascii_alphanumeric() || b"_./".contains(&b[0]))
        && b[b.len() - 1] != b':'
}

/// Whether a string is written plain where a key or a flow mapping's value stands: a word that no
/// YAML reader takes for null, a truth value, a number or a date.
pub fn plain(s: &str) -> bool {
    word(s) && !RESERVED.contains(&s) && !number_like(s) && !radix(s)
}

/// Whether a string is written plain on a line of its own: one plain word, or words joined by
/// single spaces, the first plain.
pub fn plain_line(s: &str) -> bool {
    let mut words = s.split(' ');
    words.next().is_some_and(plain) && words.all(word)
}

/// Led by a digit, or by `.` and a digit, made of digits and `. _ : - + e E t T z Z` alone, with
/// one `.` at most: the core schema's integers and floats, YAML 1.1's base 60 and dates.
fn number_like(s: &str) -> bool {
    let b = s.as_bytes();
    let led = b.first().is_some_and(u8::is_ascii_digit) || (b.first() == Some(&b'.') && b.get(1).is_some_and(u8::is_ascii_digit));
    led && b.iter().all(|c| c.is_ascii_digit() || b"._:-+eEtTzZ".contains(c)) && b.iter().filter(|c| **c == b'.').count() <= 1
}

/// `0x`, `0o` or `0b` and then hexadecimal digits and `_` alone.
fn radix(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() > 2 && b[0] == b'0' && b"xob".contains(&b[1]) && b[2..].iter().all(|c| c.is_ascii_hexdigit() || *c == b'_')
}

/// A character a double-quoted string escapes as `\uxxxx`: one YAML does not print, or that a
/// reader may take for a line break (YAML 1.1's U+0085, U+2028 and U+2029).
fn unprinted(c: char) -> bool {
    matches!(c, '\0'..='\x1f' | '\x7f'..='\u{9f}' | '\u{2028}' | '\u{2029}' | '\u{feff}' | '\u{fffe}' | '\u{ffff}')
}

/// A string in double quotes: `"` and `\` escaped, line feed, carriage return and tab as `\n`,
/// `\r` and `\t`, and the characters `unprinted` names as `\uxxxx`.
pub fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if unprinted(c) => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn key(k: &str, out: &mut String) {
    if plain(k) {
        out.push_str(k);
    } else {
        quote(k, out);
    }
}

/// The canonical text of a tree: a line per entry or element, two spaces deeper per level, keys
/// sorted by their UTF-8 bytes (the root's header first), a record on one line, strings plain where
/// `plain` or `plain_line` allows and double-quoted elsewhere, a trailing newline.
pub fn write(value: &Value) -> String {
    fn holds_list(v: &Value) -> bool {
        match v {
            Value::List(_) => true,
            Value::Map(entries) => entries.iter().any(|(_, v)| holds_list(v)),
            _ => false,
        }
    }
    fn record(path: &[&str], v: &Value) -> bool {
        matches!(v, Value::Map(_)) && !holds_list(v) && RECORDS.iter().any(|r| r.len() == path.len() && r.iter().zip(path).all(|(a, b)| *a == "*" || a == b))
    }
    fn sorted(entries: &[(String, Value)], root: bool) -> Vec<&(String, Value)> {
        let mut sorted: Vec<&(String, Value)> = entries.iter().collect();
        let rank = |k: &str| if root { HEADER.iter().position(|h| *h == k).unwrap_or(HEADER.len()) } else { 0 };
        sorted.sort_by(|a, b| rank(&a.0).cmp(&rank(&b.0)).then_with(|| a.0.cmp(&b.0)));
        sorted
    }
    fn scalar(v: &Value, line: bool, out: &mut String) {
        match v {
            Value::Str(s) if (if line { plain_line(s) } else { plain(s) }) => out.push_str(s),
            Value::Str(s) => quote(s, out),
            Value::Int(n) => out.push_str(&n.to_string()),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::List(_) => out.push_str("[]"),
            Value::Map(_) => out.push_str("{}"),
        }
    }
    fn flow(v: &Value, out: &mut String) {
        match v {
            Value::Map(entries) if !entries.is_empty() => {
                out.push('{');
                for (i, (k, v)) in sorted(entries, false).into_iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    key(k, out);
                    out.push_str(": ");
                    flow(v, out);
                }
                out.push('}');
            }
            other => scalar(other, false, out),
        }
    }
    /// The value on its key's or its dash's line, when it is not a block.
    fn on_line(v: &Value, path: &[&str], out: &mut String) -> bool {
        match v {
            _ if record(path, v) => flow(v, out),
            Value::Map(entries) if !entries.is_empty() => return false,
            Value::List(items) if !items.is_empty() => return false,
            other => scalar(other, true, out),
        }
        true
    }
    /// The block's lines from `out`'s end, the first line's indentation left for the caller when
    /// `first` is false (the element of a list, which starts on the dash's line).
    fn block<'a>(v: &'a Value, depth: usize, path: &mut Vec<&'a str>, first: bool, out: &mut String) {
        let indent = |out: &mut String, n: usize| out.extend(std::iter::repeat_n("  ", n));
        match v {
            Value::Map(entries) => {
                for (i, (k, v)) in sorted(entries, path.is_empty()).into_iter().enumerate() {
                    if i > 0 || first {
                        indent(out, depth);
                    }
                    key(k, out);
                    path.push(k);
                    out.push_str(": ");
                    if !on_line(v, path, out) {
                        out.pop();
                        out.push('\n');
                        block(v, depth + 1, path, true, out);
                        path.pop();
                        continue;
                    }
                    path.pop();
                    out.push('\n');
                }
            }
            Value::List(items) => {
                for (i, v) in items.iter().enumerate() {
                    if i > 0 || first {
                        indent(out, depth);
                    }
                    out.push_str("- ");
                    path.push("*");
                    if !on_line(v, path, out) {
                        block(v, depth + 1, path, false, out);
                    } else {
                        out.push('\n');
                    }
                    path.pop();
                }
            }
            other => {
                if first {
                    indent(out, depth);
                }
                scalar(other, true, out);
                out.push('\n');
            }
        }
    }
    let mut out = String::new();
    block(value, 0, &mut Vec::new(), true, &mut out);
    out
}

/// The compiler a lock names and its format, from its first two lines alone: what a reader
/// checks before the rest, so that a lock of another format is refused by name.
pub fn header(text: &str) -> Result<(String, i64), Error> {
    let mut lines = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l));
    let mut entry = |line: usize, name: &str| {
        let content = lines.next().unwrap_or("");
        if content.is_empty() {
            return error(line, format!("nothing where `{}:` stands", name));
        }
        let parser = Parser { lines: vec![content], pos: 0, inner: None, first_line: line };
        let (k, rest) = parser.key(content, line)?;
        if k != name {
            return error(line, format!("`{}` where `{}:` stands", content, name));
        }
        match rest.strip_prefix(' ') {
            Some(value) => parser.inline(value, line, 0),
            None => error(line, format!("`{}:` with no value", name)),
        }
    };
    let teq = entry(1, "teq")?;
    let format = entry(2, "format")?;
    match (teq, format) {
        (Value::Str(v), Value::Int(f)) => Ok((v, f)),
        (Value::Str(_), _) => error(2, "the format is no integer"),
        _ => error(1, "the compiler's version is no string"),
    }
}

/// The tree of a lock's text.
pub fn parse(text: &str) -> Result<Value, Error> {
    let Some(body) = text.strip_suffix('\n') else {
        return error(text.split('\n').count().max(1), if text.is_empty() { "an empty file" } else { "no newline at the end" });
    };
    let lines: Vec<&str> = body.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
    let mut parser = Parser { lines, pos: 0, inner: None, first_line: 1 };
    match parser.current()? {
        Some((0, content)) if !content.starts_with('-') => {}
        _ => return error(1, "the lock is no mapping at its first column"),
    }
    parser.mapping(0, 0)
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    /// The column the current line's content starts at when a list's element opened a block on
    /// the dash's line.
    inner: Option<usize>,
    /// The number of the first line, for the messages.
    first_line: usize,
}

impl<'a> Parser<'a> {
    fn line(&self) -> usize {
        self.pos + self.first_line
    }

    /// The current line's indentation and content; none at the end.
    fn current(&self) -> Result<Option<(usize, &'a str)>, Error> {
        let Some(line) = self.lines.get(self.pos) else { return Ok(None) };
        if let Some(column) = self.inner {
            return Ok(Some((column, &line[column..])));
        }
        let content = line.trim_start_matches(' ');
        if content.is_empty() {
            return error(self.line(), "a blank line");
        }
        if content.starts_with('\t') {
            return error(self.line(), "a tab in the indentation");
        }
        Ok(Some((line.len() - content.len(), content)))
    }

    fn advance(&mut self) {
        self.pos += 1;
        self.inner = None;
    }

    /// A mapping or a list whose first line is the current one, at `column`.
    fn block(&mut self, column: usize, depth: usize) -> Result<Value, Error> {
        if depth > MAX_DEPTH {
            return error(self.line(), format!("nested deeper than {} levels", MAX_DEPTH));
        }
        match self.current()? {
            Some((indent, _)) if indent != column => error(self.line(), format!("indented {} spaces where its block is at {}", indent, column)),
            Some((_, content)) if is_element(content) => self.list(column, depth),
            Some(_) => self.mapping(column, depth),
            None => error(self.line(), "the end where a block stands"),
        }
    }

    fn mapping(&mut self, column: usize, depth: usize) -> Result<Value, Error> {
        let mut entries: Vec<(String, Value)> = Vec::new();
        let mut lines: Vec<usize> = Vec::new();
        while let Some((indent, content)) = self.current()? {
            let line = self.line();
            if indent < column {
                break;
            }
            if indent > column {
                return error(line, format!("indented {} spaces where its mapping's entries are at {}", indent, column));
            }
            if is_element(content) {
                return error(line, "a list's element among a mapping's entries");
            }
            let (k, rest) = self.key(content, line)?;
            self.advance();
            let value = if rest.is_empty() {
                match self.current()? {
                    Some((indent, _)) if indent > column => self.block(column + 2, depth + 1)?,
                    _ => return error(line, format!("{} with nothing below it", shown(&k))),
                }
            } else {
                match rest.strip_prefix(' ') {
                    Some(value) => self.inline(value, line, depth)?,
                    None => return error(line, "no space after the key's colon"),
                }
            };
            entries.push((k, value));
            lines.push(line);
        }
        // A key twice, found by sorting rather than by a scan per entry, which a table of
        // hundreds of jars would make quadratic.
        let mut order: Vec<usize> = (0..entries.len()).collect();
        order.sort_unstable_by(|&a, &b| entries[a].0.cmp(&entries[b].0).then(a.cmp(&b)));
        if let Some(pair) = order.windows(2).find(|w| entries[w[0]].0 == entries[w[1]].0) {
            return error(lines[pair[1]], format!("the key {} a second time", shown(&entries[pair[1]].0)));
        }
        Ok(Value::Map(entries))
    }

    fn list(&mut self, column: usize, depth: usize) -> Result<Value, Error> {
        let mut items = Vec::new();
        while let Some((indent, content)) = self.current()? {
            let line = self.line();
            if indent < column {
                break;
            }
            if indent > column {
                return error(line, format!("indented {} spaces where its list's elements are at {}", indent, column));
            }
            let Some(rest) = content.strip_prefix("- ") else {
                return error(line, if content == "-" { "an element with nothing after its dash" } else { "a mapping's entry among a list's elements" });
            };
            if is_element(rest) || self.is_entry(rest) {
                self.inner = Some(column + 2);
                items.push(self.block(column + 2, depth + 1)?);
            } else {
                self.advance();
                items.push(self.inline(rest, line, depth)?);
            }
        }
        Ok(Value::List(items))
    }

    /// Whether an element's text opens a mapping on the dash's line: a key and its colon.
    fn is_entry(&self, text: &str) -> bool {
        if text.starts_with('"') {
            return quoted_end(text).is_some_and(|end| text[end..].starts_with(':'));
        }
        !text.starts_with('{') && text.split(' ').next().is_some_and(|w| w.ends_with(':'))
    }

    /// A line's key and what follows its colon.
    fn key(&self, content: &'a str, line: usize) -> Result<(String, &'a str), Error> {
        let (k, written, rest) = if content.starts_with('"') {
            let (k, end) = quoted(content, line)?;
            (k, end, &content[end..])
        } else {
            let token = content.split(' ').next().unwrap_or("");
            let Some(k) = token.strip_suffix(':') else {
                return error(line, format!("`{}` where a key and its colon stand", content));
            };
            if !plain(k) {
                return error(line, format!("the bare key `{}`, which YAML reads as other than this text: quote it", k));
            }
            (k.to_string(), k.len(), &content[k.len()..])
        };
        if content[..written].chars().count() > MAX_KEY {
            return error(line, format!("a key of more than YAML's {} characters", MAX_KEY));
        }
        match rest.strip_prefix(':') {
            Some(rest) => Ok((k, rest)),
            None => error(line, format!("no colon after the key {}", shown(&k))),
        }
    }

    /// A value on its key's or its dash's line: a scalar, a flow mapping, `[]`.
    fn inline(&self, text: &str, line: usize, depth: usize) -> Result<Value, Error> {
        if text.starts_with('{') {
            let (value, end) = flow(text, 0, line, depth + 1)?;
            if end != text.len() {
                return error(line, format!("`{}` after a flow mapping", &text[end..]));
            }
            return Ok(value);
        }
        if text.starts_with('[') {
            return if text == "[]" { Ok(Value::List(Vec::new())) } else { error(line, "a flow list, which the lock writes as `[]` alone") };
        }
        if text.starts_with('"') {
            let (s, end) = quoted(text, line)?;
            if end != text.len() {
                return error(line, format!("`{}` after a quoted string", &text[end..]));
            }
            return Ok(Value::Str(s));
        }
        if text.contains(' ') {
            return if plain_line(text) { Ok(Value::Str(text.to_string())) } else { error(line, format!("the bare text `{}`, which YAML may read otherwise: quote it", text)) };
        }
        bare(text, line)
    }
}

fn shown(k: &str) -> String {
    let mut out = String::new();
    key(k, &mut out);
    out
}

fn is_element(content: &str) -> bool {
    content == "-" || content.starts_with("- ")
}

/// A bare scalar: an integer, a truth value, or a string `plain` allows.
fn bare(token: &str, line: usize) -> Result<Value, Error> {
    match token {
        "true" => return Ok(Value::Bool(true)),
        "false" => return Ok(Value::Bool(false)),
        _ => {}
    }
    let digits = token.strip_prefix('-').unwrap_or(token);
    if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) && (digits == "0" || !digits.starts_with('0')) && token != "-0" {
        return match token.parse::<i64>() {
            Ok(n) if (-MAX_INT..=MAX_INT).contains(&n) => Ok(Value::Int(n)),
            _ => error(line, format!("the integer {} is out of range", token)),
        };
    }
    if plain(token) {
        return Ok(Value::Str(token.to_string()));
    }
    error(line, format!("the bare `{}`, which YAML reads as other than this text: quote it", token))
}

/// A flow mapping from `text[at..]`, `{key: value, ...}` with `{}` empty: its tree and where it
/// ends.
fn flow(text: &str, at: usize, line: usize, depth: usize) -> Result<(Value, usize), Error> {
    if depth > MAX_DEPTH {
        return error(line, format!("nested deeper than {} levels", MAX_DEPTH));
    }
    let mut pos = at + 1;
    let mut entries: Vec<(String, Value)> = Vec::new();
    if text[pos..].starts_with('}') {
        return Ok((Value::Map(entries), pos + 1));
    }
    loop {
        let (k, end) = if text[pos..].starts_with('"') {
            let (k, len) = quoted(&text[pos..], line)?;
            (k, pos + len)
        } else {
            let len = text[pos..].find(": ").unwrap_or(text.len() - pos);
            let k = &text[pos..pos + len];
            if !plain(k) {
                return error(line, format!("the bare key `{}` in a flow mapping, which YAML may read otherwise: quote it", k));
            }
            (k.to_string(), pos + len)
        };
        if !text[end..].starts_with(": ") {
            return error(line, format!("no `: ` after the key {} in a flow mapping", shown(&k)));
        }
        if text[pos..end].chars().count() > MAX_KEY {
            return error(line, format!("a key of more than YAML's {} characters", MAX_KEY));
        }
        if entries.iter().any(|(e, _)| *e == k) {
            return error(line, format!("the key {} a second time", shown(&k)));
        }
        pos = end + 2;
        let value = if text[pos..].starts_with('{') {
            let (v, end) = flow(text, pos, line, depth + 1)?;
            pos = end;
            v
        } else if text[pos..].starts_with('"') {
            let (s, len) = quoted(&text[pos..], line)?;
            pos += len;
            Value::Str(s)
        } else {
            let len = text[pos..].find([',', '}', ' ']).unwrap_or(text.len() - pos);
            let v = bare(&text[pos..pos + len], line)?;
            pos += len;
            v
        };
        entries.push((k, value));
        if text[pos..].starts_with(", ") {
            pos += 2;
        } else if text[pos..].starts_with('}') {
            return Ok((Value::Map(entries), pos + 1));
        } else {
            return error(line, "a flow mapping not closed by `}`");
        }
    }
}

/// Where the double-quoted string at the text's start ends, past its closing quote.
fn quoted_end(text: &str) -> Option<usize> {
    let mut escaped = false;
    for (i, c) in text.char_indices().skip(1) {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => return Some(i + 1),
            _ => {}
        }
    }
    None
}

/// The double-quoted string at the text's start and its length as written: the escapes the
/// writer writes alone, and no character it escapes.
fn quoted(text: &str, line: usize) -> Result<(String, usize), Error> {
    let mut out = String::new();
    let mut chars = text.char_indices().skip(1);
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Ok((out, i + 1)),
            '\\' => match chars.next().map(|(_, c)| c) {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).map(|(_, c)| c).collect();
                    let digits = hex.len() == 4 && hex.bytes().all(|b| b.is_ascii_hexdigit());
                    let code = digits.then(|| u32::from_str_radix(&hex, 16).ok()).flatten();
                    match code.and_then(char::from_u32) {
                        Some(c) => out.push(c),
                        None => return error(line, format!("the escape \\u{}, which is no character", hex)),
                    }
                }
                Some(other) => return error(line, format!("the escape \\{}, which the lock does not use", other)),
                None => break,
            },
            c if unprinted(c) => return error(line, format!("the character U+{:04X} in quotes, which the lock escapes", c as u32)),
            c => out.push(c),
        }
    }
    error(line, "a quoted string not closed on its line")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> Value {
        Value::Str(text.to_string())
    }

    fn map(entries: &[(&str, Value)]) -> Value {
        Value::Map(entries.iter().map(|(k, v)| (k.to_string(), v.clone())).collect())
    }

    #[test]
    fn the_quoting_predicate() {
        for yes in ["a", "3.8.4", "0.1.2-teq-lock-SNAPSHOT", "org.typelevel:cats-core_3:2.13.0", "https://repo1.maven.org/maven2/", "ff15f2278065734035de325534184128c33eeba1", "/abs", "_x", ".", ".gitignore", "1.0@build", "a:b"] {
            assert!(plain(yes), "{yes}");
        }
        for no in [
            "", "true", "True", "FALSE", "null", "Null", "~", "yes", "on", "Off", "y", "N", ".inf", ".NaN", "2", "17", "1.0", "1e3", "1E-3", ".5", "2026-10-05", "2026-10-05T10:00:00Z", "10:30", "1_000", "0x1F", "0o17", "0b101", "-Xmx1g", "+1", ":a", "a:", "@a",
            "a b", "a#b", "a,b", "{a}", "[a]", "a'b", "a\"b", "é", "a\tb", "a%20b", "a~b", "a=b", "a?b", "a!b", "a*b", "a&b", "a|b", "a>b", "a`b",
        ] {
            assert!(!plain(no), "{no}");
        }
        // A sha1 of digits alone, or of digits and an exponent's `e`, is a number to YAML.
        assert!(!plain("1234567890123456789012345678901234567890"));
        assert!(!plain("1234567890123456789012345678901234567e90"));
        assert!(plain("1234567890123456789012345678901234567d90"));
        assert!(plain_line("maven-central ff15f2278065734035de325534184128c33eeba1 17334592"));
        assert!(plain_line("a 2 true"));
        for no in ["2 a", "true a", "a  b", " a", "a ", "a -b", "a #b", "a b:", ""] {
            assert!(!plain_line(no), "{no}");
        }
    }

    #[test]
    fn scalars_of_every_shape_read_and_write_back() {
        let text = "teq: 0.1.2\nformat: 1\nbinaries: {}\na:\n  bare: x.y/z:1@b\n  empty: \"\"\n  escapes: \"\\\"\\\\\\n\\r\\t\\u0000\\u007f\\u0085\\u009f\\ufeff\\uffff\"\n  \"false\": false\n  \"int\": -12\n  list: []\n  map: {}\n  number: \"2\"\n  \"on\": \"on\"\n  text: a few words\n  unicode: \"é 😀\"\n  zero: 0\n";
        let value = parse(text).unwrap();
        let a = value.get("a").unwrap();
        assert_eq!(a.get("bare"), Some(&s("x.y/z:1@b")));
        assert_eq!(a.get("escapes"), Some(&s("\"\\\n\r\t\0\x7f\u{85}\u{9f}\u{feff}\u{ffff}")));
        assert_eq!(a.get("false"), Some(&Value::Bool(false)));
        assert_eq!(a.get("int"), Some(&Value::Int(-12)));
        assert_eq!(a.get("list"), Some(&Value::List(vec![])));
        assert_eq!(a.get("map"), Some(&Value::Map(vec![])));
        assert_eq!(a.get("number"), Some(&s("2")));
        assert_eq!(a.get("on"), Some(&s("on")));
        assert_eq!(a.get("text"), Some(&s("a few words")));
        assert_eq!(a.get("unicode"), Some(&s("é 😀")));
        assert_eq!(a.get("zero"), Some(&Value::Int(0)));
        assert_eq!(write(&value), text.replace("  \"int\": -12", "  int: -12"));
        assert_eq!(header(text).unwrap(), ("0.1.2".to_string(), 1));
    }

    #[test]
    fn blocks_lists_and_records() {
        let value = map(&[
            ("teq", s("1.0")),
            ("format", Value::Int(1)),
            ("binaries", map(&[("linux-x86_64", s("https://r/teq.exe ff15f2278065734035de325534184128c33eeba1 3"))])),
            (
                "projects",
                map(&[(
                    "p",
                    map(&[
                        (
                            "configurations",
                            map(&[("compile", map(&[("classpath", Value::List(vec![map(&[("project", s("q")), ("configuration", s("compile"))]), s("a:b:1"), map(&[("file", s("lib/x y.jar"))])]))]))]),
                        ),
                        ("generators", Value::List(vec![map(&[("kind", s("command")), ("run", Value::List(vec![s("sh"), s("-c")])), ("dynamic", map(&[("x", map(&[("w", s("z"))]))]))])])),
                        (
                            "stage",
                            map(&[(
                                "layers",
                                map(&[(
                                    "2",
                                    Value::List(vec![
                                        map(&[("to", s("opt/docker/lib/p.jar")), ("from", map(&[("project", s("p")), ("configuration", s("compile"))])), ("manifest", map(&[("Main-Class", s("M"))]))]),
                                        map(&[("script", s("opt/docker/bin/p")), ("classpath", Value::List(vec![s("lib/a.jar")]))]),
                                    ]),
                                )]),
                            )]),
                        ),
                    ]),
                )]),
            ),
            ("lists", Value::List(vec![Value::List(vec![s("a"), s("b")]), Value::Int(3)])),
        ]);
        let wanted = r#"teq: "1.0"
format: 1
binaries:
  linux-x86_64: https://r/teq.exe ff15f2278065734035de325534184128c33eeba1 3
lists:
  - - a
    - b
  - 3
projects:
  p:
    configurations:
      compile:
        classpath:
          - {configuration: compile, project: q}
          - a:b:1
          - {file: "lib/x y.jar"}
    generators:
      - dynamic:
          x:
            w: z
        kind: command
        run:
          - sh
          - "-c"
    stage:
      layers:
        "2":
          - {from: {configuration: compile, project: p}, manifest: {Main-Class: M}, to: opt/docker/lib/p.jar}
          - classpath:
              - lib/a.jar
            script: opt/docker/bin/p
"#;
        assert_eq!(write(&value), wanted);
        let read = parse(wanted).unwrap();
        assert_eq!(write(&read), wanted);
        assert_eq!(read.at(&["projects", "p", "stage", "layers", "2"]).unwrap().items()[1].get("script"), Some(&s("opt/docker/bin/p")));
    }

    #[test]
    fn what_a_reader_refuses_by_its_line() {
        let refused = |text: &str| parse(text).err().map(|e| e.to_string()).unwrap_or_default();
        let lock = |body: &str| format!("teq: 1.2.3\nformat: 1\n{}", body);
        assert_eq!(refused(&lock("a: 1.0\n")), "line 3: the bare `1.0`, which YAML reads as other than this text: quote it");
        assert!(refused(&lock("a: null\n")).starts_with("line 3: the bare `null`"));
        assert!(refused(&lock("a: yes\n")).starts_with("line 3: the bare `yes`"));
        assert!(refused(&lock("a: ~\n")).starts_with("line 3: the bare `~`"));
        assert!(refused(&lock("a: 007\n")).starts_with("line 3: the bare `007`"));
        assert!(refused(&lock("a: -x\n")).starts_with("line 3: the bare `-x`"));
        assert!(refused(&lock("a: x\na: z\n")).starts_with("line 4: the key a a second time"));
        assert!(refused(&lock("a:\n   b: 1\n")).starts_with("line 4: indented 3 spaces"));
        assert!(refused(&lock("a:\n  b: 1\n   c: 2\n")).starts_with("line 5: indented 3 spaces"));
        assert!(refused(&lock("a:\n\tb: 1\n")).starts_with("line 4: a tab"));
        assert!(refused(&lock("a: 1\n\nb: 2\n")).starts_with("line 4: a blank line"));
        assert!(refused(&lock("a: \"x\\by\"\n")).starts_with("line 3: the escape \\b"));
        assert!(refused(&lock("a: \"x\u{85}\"\n")).starts_with("line 3: the character U+0085"));
        assert!(refused(&lock("a: \"x\ty\"\n")).starts_with("line 3: the character U+0009"));
        assert!(refused(&lock("a: \"x\n")).starts_with("line 3: a quoted string not closed"));
        assert!(refused(&lock("a: [x]\n")).starts_with("line 3: a flow list"));
        assert!(refused(&lock("a: {b: 1,c: 2}\n")).starts_with("line 3: a flow mapping not closed"));
        assert!(refused(&lock("a: {b: 1, b: 2}\n")).starts_with("line 3: the key b a second time"));
        assert!(refused(&lock("a: {b: 1} x\n")).starts_with("line 3: ` x` after a flow mapping"));
        assert!(refused(&lock("a:\n  - x\n  b: v\n")).starts_with("line 5: a mapping's entry among a list's elements"));
        assert!(refused(&lock("a:\n  b: v\n  - x\n")).starts_with("line 5: a list's element among a mapping's entries"));
        assert!(refused(&lock("a:\n")).starts_with("line 3: a with nothing below it"));
        assert!(refused(&lock("a: x #c\n")).starts_with("line 3: the bare text `x #c`"));
        assert!(refused(&lock("a b: x\n")).starts_with("line 3: `a b: x` where a key"));
        assert!(refused(&lock("2: x\n")).starts_with("line 3: the bare key `2`"));
        assert!(refused(&lock("a:x\n")).starts_with("line 3: `a:x` where a key"));
        assert!(refused(&lock("a: 1")).starts_with("line 3: no newline at the end"));
        assert!(refused("").starts_with("line 1: an empty file"));
        assert!(refused("- a\n").starts_with("line 1: the lock is no mapping"));
        assert!(refused(&lock(&format!("{}: 1\n", "k".repeat(MAX_KEY + 1)))).starts_with("line 3: a key of more than YAML's 1024 characters"));
        assert!(refused(&lock(&format!("{}: 1\n", "k".repeat(MAX_KEY)))).is_empty());
        assert!(refused(&lock("a: 9007199254740992\n")).starts_with("line 3: the integer 9007199254740992 is out of range"));
        assert!(refused(&lock("a: -9007199254740991\n")).is_empty());
        assert!(refused(&lock("a: -9223372036854775808\n")).starts_with("line 3: the integer -9223372036854775808 is out of range"));
        assert!(refused(&lock("a: \"\\u+041\"\n")).starts_with("line 3: the escape \\u+041, which is no character"));
        assert!(refused(&lock(&format!("a: {{{}: 1}}\n", "k".repeat(MAX_KEY + 1)))).starts_with("line 3: a key of more than YAML's 1024 characters"));
        let deep = (0..70).map(|i| format!("{}k:\n", "  ".repeat(i))).collect::<String>() + &format!("{}k: 1\n", "  ".repeat(70));
        assert!(refused(&lock(&deep)).contains("nested deeper than 64 levels"));
        let deep_flow = format!("a: {}{}\n", "{a: ".repeat(70), "}".repeat(70));
        assert!(refused(&lock(&deep_flow)).contains("nested deeper than 64 levels"));
        // A line ended by `\r\n` reads as one ended by `\n`.
        assert_eq!(parse("teq: 1.2.3\r\nformat: 1\r\n").unwrap(), parse("teq: 1.2.3\nformat: 1\n").unwrap());
    }

    #[test]
    fn the_header_alone() {
        assert_eq!(header("teq: 0.1.3\nformat: 2\nanything at all {\n").unwrap(), ("0.1.3".to_string(), 2));
        assert_eq!(header("teq: \"1.0\"\nformat: 1\n").unwrap(), ("1.0".to_string(), 1));
        assert_eq!(header("{\n").unwrap_err().to_string(), "line 1: `{` where a key and its colon stand");
        assert_eq!(header("format: 1\nteq: 1.2.3\n").unwrap_err().to_string(), "line 1: `format: 1` where `teq:` stands");
        assert_eq!(header("teq: 1.2.3\n").unwrap_err().to_string(), "line 2: nothing where `format:` stands");
    }
}
