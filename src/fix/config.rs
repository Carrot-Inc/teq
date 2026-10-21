//! The configuration of `teq fix` (docs/FIX.md, "Configuration"): the HOCON that scalafix's configurations use,
//! `.scalafix.conf` beside `teq.lock` or the file `--config` names, read by teq. What it reads: `key = value`,
//! `key: value` and `key { ... }`, dotted keys (`DisableSyntax.noVars = true`), objects merged as HOCON merges them,
//! lists with or without commas, quoted and bare strings, adjacent strings joined (`"print""ln"`), numbers,
//! booleans, `null`, and `#` and `//` comments. Anything else of HOCON (`include`, `${...}`, `+=`, `"""`) is an
//! error at its place, never skipped.

/// A value with where it was written.
#[derive(Clone, Debug)]
pub struct Node {
    pub value: Value,
    pub line: u32,
    pub col: u32,
}

#[derive(Clone, Debug)]
pub enum Value {
    Str(String),
    /// A number as written.
    Num(String),
    Bool(bool),
    Null,
    List(Vec<u32>),
    Obj(Vec<(String, u32)>),
}

/// A configuration: its nodes, the root object first.
#[derive(Clone, Debug, Default)]
pub struct Conf {
    pub file: String,
    pub nodes: Vec<Node>,
}

impl Conf {
    pub fn empty(file: &str) -> Conf {
        Conf { file: file.to_string(), nodes: vec![Node { value: Value::Obj(Vec::new()), line: 1, col: 1 }] }
    }

    pub fn place(&self, n: u32) -> String {
        let node = &self.nodes[n as usize];
        format!("{}:{}:{}", self.file, node.line, node.col)
    }

    /// The field `key` of the object `n`.
    pub fn field(&self, n: u32, key: &str) -> Option<u32> {
        match &self.nodes[n as usize].value {
            Value::Obj(fields) => fields.iter().rev().find(|(k, _)| k == key).map(|&(_, v)| v),
            _ => None,
        }
    }

    /// The value at a dotted path from the root (`lint.error.includes`).
    pub fn at(&self, path: &[&str]) -> Option<u32> {
        let mut n = 0;
        for k in path {
            n = self.field(n, k)?;
        }
        Some(n)
    }

    pub fn str(&self, n: u32) -> Option<String> {
        match &self.nodes[n as usize].value {
            Value::Str(s) | Value::Num(s) => Some(s.clone()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    /// A string or a list of strings.
    pub fn strings(&self, n: u32) -> Result<Vec<String>, String> {
        match &self.nodes[n as usize].value {
            Value::List(items) => items.iter().map(|&i| self.str(i).ok_or_else(|| format!("{}: expected a string", self.place(i)))).collect(),
            Value::Null => Ok(Vec::new()),
            _ => self.str(n).map(|s| vec![s]).ok_or_else(|| format!("{}: expected a string or a list of strings", self.place(n))),
        }
    }
}

pub fn parse(file: &str, text: &str) -> Result<Conf, String> {
    let mut p = Parser { file, b: text.as_bytes(), text, pos: 0, line: 1, col: 1, conf: Conf::empty(file) };
    p.skip_ws(true)?;
    let root = if p.peek() == Some(b'{') {
        p.bump();
        let fields = p.fields(Some(b'}'))?;
        p.expect(b'}')?;
        fields
    } else {
        p.fields(None)?
    };
    p.skip_ws(true)?;
    if p.pos < p.b.len() {
        return Err(p.error("unexpected text after the configuration"));
    }
    p.conf.nodes[0].value = Value::Obj(Vec::new());
    for (path, v) in root {
        p.put(0, &path, v);
    }
    Ok(p.conf)
}

struct Parser<'a> {
    file: &'a str,
    b: &'a [u8],
    text: &'a str,
    pos: usize,
    line: u32,
    col: u32,
    conf: Conf,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
            self.col = 1;
        } else if c & 0xc0 != 0x80 {
            self.col += 1;
        }
        Some(c)
    }

    fn error(&self, msg: &str) -> String {
        format!("{}:{}:{}: {}", self.file, self.line, self.col, msg)
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.bump();
            Ok(())
        } else {
            Err(self.error(&format!("expected `{}`", c as char)))
        }
    }

    /// Skips spaces and comments, newlines too where `newlines`; returns whether a newline was crossed.
    fn skip_ws(&mut self, newlines: bool) -> Result<bool, String> {
        let mut crossed = false;
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r') => {
                    self.bump();
                }
                Some(b'\n') if newlines => {
                    crossed = true;
                    self.bump();
                }
                Some(b'#') => self.skip_line(),
                Some(b'/') if self.b.get(self.pos + 1) == Some(&b'/') => self.skip_line(),
                _ => return Ok(crossed),
            }
        }
    }

    fn skip_line(&mut self) {
        while self.peek().is_some_and(|c| c != b'\n') {
            self.bump();
        }
    }

    fn node(&mut self, value: Value, line: u32, col: u32) -> u32 {
        self.conf.nodes.push(Node { value, line, col });
        (self.conf.nodes.len() - 1) as u32
    }

    /// The fields of an object up to `close` (`}`), or to the end of the text for the root.
    fn fields(&mut self, close: Option<u8>) -> Result<Vec<(Vec<String>, u32)>, String> {
        let mut out = Vec::new();
        loop {
            self.skip_ws(true)?;
            match self.peek() {
                None if close.is_none() => return Ok(out),
                None => return Err(self.error("the object is not closed")),
                Some(c) if Some(c) == close => return Ok(out),
                Some(b',') => {
                    self.bump();
                    continue;
                }
                _ => {}
            }
            if self.text[self.pos..].starts_with("include") && self.text[self.pos + 7..].starts_with(|c: char| c.is_whitespace()) {
                return Err(self.error("`include` is not supported"));
            }
            let path = self.key()?;
            self.skip_ws(false)?;
            let value = match self.peek() {
                Some(b'=' | b':') => {
                    self.bump();
                    self.skip_ws(true)?;
                    self.value()?
                }
                Some(b'+') if self.b.get(self.pos + 1) == Some(&b'=') => return Err(self.error("`+=` is not supported")),
                Some(b'{') => self.value()?,
                _ => return Err(self.error("expected `=`, `:` or `{` after a key")),
            };
            out.push((path, value));
        }
    }

    /// A key: names joined by dots, each bare or quoted.
    fn key(&mut self) -> Result<Vec<String>, String> {
        let mut parts = Vec::new();
        loop {
            let part = if self.peek() == Some(b'"') {
                self.quoted()?
            } else {
                let start = self.pos;
                while self.peek().is_some_and(|c| !matches!(c, b'.' | b'=' | b':' | b'{' | b' ' | b'\t' | b'\r' | b'\n' | b'#' | b'"' | b',' | b'}' | b'+')) {
                    self.bump();
                }
                if self.pos == start {
                    return Err(self.error("expected a key"));
                }
                self.text[start..self.pos].to_string()
            };
            parts.push(part);
            if self.peek() == Some(b'.') {
                self.bump();
            } else {
                return Ok(parts);
            }
        }
    }

    fn quoted(&mut self) -> Result<String, String> {
        if self.text[self.pos..].starts_with("\"\"\"") {
            return Err(self.error("triple-quoted strings are not supported"));
        }
        self.bump();
        let mut s = String::new();
        loop {
            match self.bump() {
                None | Some(b'\n') => return Err(self.error("the string is not closed")),
                Some(b'"') => return Ok(s),
                Some(b'\\') => match self.bump() {
                    Some(b'"') => s.push('"'),
                    Some(b'\\') => s.push('\\'),
                    Some(b'/') => s.push('/'),
                    Some(b'n') => s.push('\n'),
                    Some(b't') => s.push('\t'),
                    Some(b'r') => s.push('\r'),
                    Some(b'b') => s.push('\u{8}'),
                    Some(b'f') => s.push('\u{c}'),
                    Some(b'u') => {
                        let hex = self.text.get(self.pos..self.pos + 4).ok_or_else(|| self.error("a \\u escape needs four digits"))?;
                        let c = u32::from_str_radix(hex, 16).ok().and_then(char::from_u32).ok_or_else(|| self.error("a \\u escape needs four digits"))?;
                        for _ in 0..4 {
                            self.bump();
                        }
                        s.push(c);
                    }
                    _ => return Err(self.error("an unknown escape")),
                },
                Some(c) if c < 0x80 => s.push(c as char),
                Some(_) => {
                    // The rest of a multi-byte character: take it whole from the text.
                    let start = self.pos - 1;
                    let len = self.text[start..].chars().next().map_or(1, char::len_utf8);
                    s.push_str(&self.text[start..start + len]);
                    for _ in 1..len {
                        self.bump();
                    }
                }
            }
        }
    }

    fn value(&mut self) -> Result<u32, String> {
        let (line, col) = (self.line, self.col);
        if self.text[self.pos..].starts_with("${") {
            return Err(self.error("substitutions (`${...}`) are not supported"));
        }
        match self.peek() {
            Some(b'{') => {
                self.bump();
                let fields = self.fields(Some(b'}'))?;
                self.expect(b'}')?;
                let obj = self.node(Value::Obj(Vec::new()), line, col);
                for (path, v) in fields {
                    self.put(obj, &path, v);
                }
                Ok(obj)
            }
            Some(b'[') => {
                self.bump();
                let mut items = Vec::new();
                loop {
                    self.skip_ws(true)?;
                    match self.peek() {
                        Some(b']') => {
                            self.bump();
                            break;
                        }
                        Some(b',') => {
                            self.bump();
                        }
                        None => return Err(self.error("the list is not closed")),
                        _ => items.push(self.value()?),
                    }
                }
                Ok(self.node(Value::List(items), line, col))
            }
            _ => {
                // The value's parts, each quoted or not: a quoted one's characters are kept as written, and only the
                // whitespace before the first part and after the last is not the value's (HOCON's concatenation).
                let mut parts: Vec<(bool, String)> = Vec::new();
                let mut quoted_any = false;
                // A value runs to the end of its line, a comma, a closing bracket or a comment: adjacent strings are
                // one, with what stands between them (HOCON's value concatenation).
                loop {
                    match self.peek() {
                        Some(b'"') => {
                            parts.push((true, self.quoted()?));
                            quoted_any = true;
                        }
                        Some(b'\n' | b',' | b']' | b'}' | b'#') | None => break,
                        Some(b'/') if self.b.get(self.pos + 1) == Some(&b'/') => break,
                        Some(b'$') if self.b.get(self.pos + 1) == Some(&b'{') => return Err(self.error("substitutions (`${...}`) are not supported")),
                        Some(_) => {
                            let start = self.pos;
                            while self.peek().is_some_and(|c| !matches!(c, b'"' | b'\n' | b',' | b']' | b'}' | b'#')) && !(self.peek() == Some(b'/') && self.b.get(self.pos + 1) == Some(&b'/')) {
                                self.bump();
                            }
                            parts.push((false, self.text[start..self.pos].to_string()));
                        }
                    }
                }
                if let Some((false, first)) = parts.first_mut() {
                    *first = first.trim_start().to_string();
                }
                if let Some((false, last)) = parts.last_mut() {
                    *last = last.trim_end().to_string();
                }
                let trimmed: String = parts.into_iter().map(|(_, p)| p).collect();
                let value = if quoted_any {
                    Value::Str(trimmed)
                } else {
                    match trimmed.as_str() {
                        "" => return Err(self.error("expected a value")),
                        "true" => Value::Bool(true),
                        "false" => Value::Bool(false),
                        "null" => Value::Null,
                        t if t.parse::<f64>().is_ok() && t.starts_with(|c: char| c.is_ascii_digit() || c == '-') => Value::Num(t.to_string()),
                        t => Value::Str(t.to_string()),
                    }
                };
                Ok(self.node(value, line, col))
            }
        }
    }

    /// Sets `path` in the object `obj` to `v`, objects on the way made or merged: a later object merges into an
    /// earlier one, any other later value replaces it.
    fn put(&mut self, obj: u32, path: &[String], v: u32) {
        let key = &path[0];
        let existing = self.conf.field(obj, key);
        if path.len() > 1 {
            let inner = match existing {
                Some(e) if matches!(self.conf.nodes[e as usize].value, Value::Obj(_)) => e,
                _ => {
                    let (line, col) = (self.conf.nodes[v as usize].line, self.conf.nodes[v as usize].col);
                    let n = self.node(Value::Obj(Vec::new()), line, col);
                    self.set(obj, key, n);
                    n
                }
            };
            self.put(inner, &path[1..], v);
            return;
        }
        match (existing, &self.conf.nodes[v as usize].value) {
            (Some(e), Value::Obj(new_fields)) if matches!(self.conf.nodes[e as usize].value, Value::Obj(_)) => {
                for (k, f) in new_fields.clone() {
                    self.put(e, &[k], f);
                }
            }
            _ => self.set(obj, key, v),
        }
    }

    fn set(&mut self, obj: u32, key: &str, v: u32) {
        if let Value::Obj(fields) = &mut self.conf.nodes[obj as usize].value {
            fields.retain(|(k, _)| k != key);
            fields.push((key.to_string(), v));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalafix_shapes() {
        let text = "rules = [\n  DisableSyntax\n  \"file:x/NoDummy.scala\",\n]\nDisableSyntax.noVars = true\nDisableSyntax {\n  noNulls: true # comment\n  regex = [\n    \"print\"\"ln\"\n    { id = offensive, pattern = \"[Pp]imp\" }\n  ]\n}\n// a comment\nlint.error.includes = \".*\"\nn = 42\n";
        let c = parse("c.conf", text).unwrap();
        assert_eq!(c.strings(c.at(&["rules"]).unwrap()).unwrap(), ["DisableSyntax", "file:x/NoDummy.scala"]);
        let ds = c.at(&["DisableSyntax"]).unwrap();
        assert!(matches!(c.nodes[c.field(ds, "noVars").unwrap() as usize].value, Value::Bool(true)));
        assert!(matches!(c.nodes[c.field(ds, "noNulls").unwrap() as usize].value, Value::Bool(true)));
        let Value::List(regex) = &c.nodes[c.field(ds, "regex").unwrap() as usize].value else { panic!() };
        assert_eq!(c.str(regex[0]).as_deref(), Some("println"));
        assert_eq!(c.str(c.field(regex[1], "pattern").unwrap()).as_deref(), Some("[Pp]imp"));
        assert_eq!(c.str(c.at(&["lint", "error", "includes"]).unwrap()).as_deref(), Some(".*"));
        assert!(matches!(&c.nodes[c.at(&["n"]).unwrap() as usize].value, Value::Num(n) if n == "42"));
        assert_eq!(c.place(c.at(&["DisableSyntax", "noVars"]).unwrap()), "c.conf:5:24");
    }

    #[test]
    fn quoted_parts_keep_their_characters() {
        // HOCON's concatenation: the quoted parts as written, what stands between them kept, what stands outside not.
        let c = parse("c", "a = \" var \"\nb = \" x \" \"y\"\nc = x \"y\" z  \nd = [\" v \", w ]\n").unwrap();
        assert_eq!(c.str(c.at(&["a"]).unwrap()).as_deref(), Some(" var "));
        assert_eq!(c.str(c.at(&["b"]).unwrap()).as_deref(), Some(" x  y"));
        assert_eq!(c.str(c.at(&["c"]).unwrap()).as_deref(), Some("x y z"));
        assert_eq!(c.strings(c.at(&["d"]).unwrap()).unwrap(), [" v ", "w"]);
    }

    #[test]
    fn unsupported_is_an_error_at_its_place() {
        assert_eq!(parse("c", "a = ${b}\n").unwrap_err(), "c:1:5: substitutions (`${...}`) are not supported");
        assert_eq!(parse("c", "include \"x\"\n").unwrap_err(), "c:1:1: `include` is not supported");
        assert_eq!(parse("c", "a += 1\n").unwrap_err(), "c:1:3: `+=` is not supported");
        assert!(parse("c", "a = \"\"\"x\"\"\"\n").unwrap_err().contains("triple-quoted"));
        assert!(parse("c", "a = [1, 2\n").unwrap_err().contains("not closed"));
    }
}
