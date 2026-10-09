//! A JSON value with a reader and a writer, enough for the language server's messages: objects
//! keep their keys in order, numbers are `f64` (an id or a position is an integer well within
//! its exact range), strings take every escape of RFC 8259 with surrogate pairs.

use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

/// How deeply arrays and objects may nest: deeper input is refused rather than read on a
/// stack that the nesting could exhaust.
const MAX_DEPTH: u32 = 512;

impl Json {
    pub fn parse(text: &str) -> Result<Json, String> {
        let mut r = Reader { bytes: text.as_bytes(), text, pos: 0 };
        r.skip_ws();
        let v = r.value(0)?;
        r.skip_ws();
        if r.pos != r.bytes.len() {
            return Err(format!("unexpected text after the value at byte {}", r.pos));
        }
        Ok(v)
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The value at a path of keys, `None` where one is missing or `null`.
    pub fn at(&self, path: &[&str]) -> Option<&Json> {
        let mut v = self;
        for key in path {
            v = v.get(key)?;
        }
        (!matches!(v, Json::Null)).then_some(v)
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn uint(&self) -> Option<u32> {
        self.num().filter(|n| *n >= 0.0 && n.fract() == 0.0 && *n <= u32::MAX as f64).map(|n| n as u32)
    }

    pub fn bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn arr(&self) -> &[Json] {
        match self {
            Json::Arr(items) => items,
            _ => &[],
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }

    pub fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => write_num(*n, out),
            Json::Str(s) => write_str(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Json {
        Json::Str(s.to_string())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Json {
        Json::Str(s)
    }
}

impl From<bool> for Json {
    fn from(b: bool) -> Json {
        Json::Bool(b)
    }
}

impl From<u32> for Json {
    fn from(n: u32) -> Json {
        Json::Num(n as f64)
    }
}

impl From<usize> for Json {
    fn from(n: usize) -> Json {
        Json::Num(n as f64)
    }
}

impl From<Vec<Json>> for Json {
    fn from(items: Vec<Json>) -> Json {
        Json::Arr(items)
    }
}

/// An object from its fields, in order.
pub fn obj<const N: usize>(fields: [(&str, Json); N]) -> Json {
    Json::Obj(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn write_num(n: f64, out: &mut String) {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 9.0e15 {
        let _ = write!(out, "{}", n as i64);
    } else if n.is_finite() {
        let _ = write!(out, "{}", n);
    } else {
        out.push_str("null");
    }
}

pub fn write_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Reader<'t> {
    bytes: &'t [u8],
    text: &'t str,
    pos: usize,
}

impl<'t> Reader<'t> {
    fn skip_ws(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.bytes.get(self.pos) {
            self.pos += 1;
        }
    }

    fn fail<T>(&self, what: &str) -> Result<T, String> {
        Err(format!("{} at byte {}", what, self.pos))
    }

    fn value(&mut self, depth: u32) -> Result<Json, String> {
        if depth > MAX_DEPTH {
            return self.fail("nesting too deep");
        }
        match self.bytes.get(self.pos) {
            None => self.fail("unexpected end of input"),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => self.fail("unexpected character"),
        }
    }

    fn literal(&mut self, word: &str, v: Json) -> Result<Json, String> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(v)
        } else {
            self.fail("unexpected character")
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.bytes.get(self.pos) == Some(&b'-') {
            self.pos += 1;
        }
        let digits = |r: &mut Reader| {
            let from = r.pos;
            while let Some(b'0'..=b'9') = r.bytes.get(r.pos) {
                r.pos += 1;
            }
            r.pos > from
        };
        if !digits(self) {
            return self.fail("malformed number");
        }
        if self.bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            if !digits(self) {
                return self.fail("malformed number");
            }
        }
        if let Some(b'e' | b'E') = self.bytes.get(self.pos) {
            self.pos += 1;
            if let Some(b'+' | b'-') = self.bytes.get(self.pos) {
                self.pos += 1;
            }
            if !digits(self) {
                return self.fail("malformed number");
            }
        }
        self.text[start..self.pos].parse::<f64>().map(Json::Num).or_else(|_| self.fail("malformed number"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let Some(digits) = self.text.get(self.pos..self.pos + 4) else { return self.fail("truncated escape") };
        if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return self.fail("malformed escape");
        }
        let v = u32::from_str_radix(digits, 16).or_else(|_| self.fail("malformed escape"))?;
        self.pos += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, String> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            let run = self.pos;
            while let Some(&b) = self.bytes.get(self.pos) {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.pos += 1;
            }
            out.push_str(&self.text[run..self.pos]);
            match self.bytes.get(self.pos) {
                None => return self.fail("unterminated string"),
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let Some(&e) = self.bytes.get(self.pos) else { return self.fail("unterminated string") };
                    self.pos += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let c = if (0xd800..0xdc00).contains(&hi) && self.bytes[self.pos..].starts_with(b"\\u") {
                                let back = self.pos;
                                self.pos += 2;
                                let lo = self.hex4()?;
                                if (0xdc00..0xe000).contains(&lo) {
                                    0x10000 + ((hi - 0xd800) << 10) + (lo - 0xdc00)
                                } else {
                                    self.pos = back;
                                    0xfffd
                                }
                            } else {
                                hi
                            };
                            // A lone surrogate is no character: it reads as the replacement one.
                            out.push(char::from_u32(c).unwrap_or('\u{fffd}'));
                        }
                        _ => return self.fail("malformed escape"),
                    }
                }
                Some(_) => return self.fail("control character in a string"),
            }
        }
    }

    fn array(&mut self, depth: u32) -> Result<Json, String> {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value(depth + 1)?);
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Arr(items));
                }
                _ => return self.fail("expected , or ]"),
            }
        }
    }

    fn object(&mut self, depth: u32) -> Result<Json, String> {
        self.pos += 1;
        let mut fields = Vec::new();
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Ok(Json::Obj(fields));
        }
        loop {
            self.skip_ws();
            if self.bytes.get(self.pos) != Some(&b'"') {
                return self.fail("expected a key");
            }
            let key = self.string()?;
            self.skip_ws();
            if self.bytes.get(self.pos) != Some(&b':') {
                return self.fail("expected :");
            }
            self.pos += 1;
            self.skip_ws();
            let v = self.value(depth + 1)?;
            fields.push((key, v));
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Obj(fields));
                }
                _ => return self.fail("expected , or }"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes() {
        let text = r#"{"a":[1,-2.5,3e2,true,false,null],"s":"x\"\\\/\n\u00e9\ud83d\ude00","o":{}}"#;
        let v = Json::parse(text).unwrap();
        assert_eq!(v.at(&["s"]).and_then(Json::str), Some("x\"\\/\né😀"));
        assert_eq!(v.to_text(), r#"{"a":[1,-2.5,300,true,false,null],"s":"x\"\\/\né😀","o":{}}"#);
    }

    #[test]
    fn refuses_malformed() {
        for bad in ["", "{", "[1,]", "{\"a\" 1}", "\"\\x\"", "01x", "[1] 2", "\"\u{1}\""] {
            assert!(Json::parse(bad).is_err(), "{}", bad);
        }
        let deep = "[".repeat(10_000);
        assert!(Json::parse(&deep).is_err());
    }

    #[test]
    fn lone_surrogate_reads_as_replacement() {
        assert_eq!(Json::parse(r#""\ud800x""#).unwrap(), Json::Str("\u{fffd}x".to_string()));
    }
}
