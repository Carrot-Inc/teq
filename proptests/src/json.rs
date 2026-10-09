//! The JSON of a session's answers, read without a dependency: values compare structurally and
//! print in one canonical form.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    List(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    pub fn parse(text: &str) -> Result<Json, String> {
        let mut reader = Reader { bytes: text.as_bytes(), at: 0 };
        let value = reader.value()?;
        reader.space();
        if reader.at != reader.bytes.len() {
            return Err(format!("text after the value at byte {}", reader.at));
        }
        Ok(value)
    }

    pub fn get(&self, key: &str) -> &Json {
        match self {
            Json::Object(fields) => fields.get(key).unwrap_or(&Json::Null),
            _ => &Json::Null,
        }
    }

    pub fn is_true(&self) -> bool {
        *self == Json::Bool(true)
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Json::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn items(&self) -> &[Json] {
        match self {
            Json::List(items) => items,
            _ => &[],
        }
    }
}

impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Json::Null => write!(f, "null"),
            Json::Bool(b) => write!(f, "{b}"),
            Json::Number(n) => write!(f, "{n}"),
            Json::Text(text) => write!(f, "{text:?}"),
            Json::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Json::Object(fields) => {
                write!(f, "{{")?;
                for (i, (key, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{key:?}:{value}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn space(&mut self) {
        while self.at < self.bytes.len() && self.bytes[self.at].is_ascii_whitespace() {
            self.at += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        if self.bytes.get(self.at) == Some(&byte) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("expected {:?} at byte {}", byte as char, self.at))
        }
    }

    fn word(&mut self, word: &str, value: Json) -> Result<Json, String> {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(format!("unknown word at byte {}", self.at))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.space();
        match self.bytes.get(self.at) {
            None => Err("the text ends where a value is expected".to_string()),
            Some(b'n') => self.word("null", Json::Null),
            Some(b't') => self.word("true", Json::Bool(true)),
            Some(b'f') => self.word("false", Json::Bool(false)),
            Some(b'"') => Ok(Json::Text(self.string()?)),
            Some(b'[') => {
                self.at += 1;
                let mut items = Vec::new();
                self.space();
                if self.bytes.get(self.at) == Some(&b']') {
                    self.at += 1;
                    return Ok(Json::List(items));
                }
                loop {
                    items.push(self.value()?);
                    self.space();
                    if self.bytes.get(self.at) == Some(&b',') {
                        self.at += 1;
                    } else {
                        self.expect(b']')?;
                        return Ok(Json::List(items));
                    }
                }
            }
            Some(b'{') => {
                self.at += 1;
                let mut fields = BTreeMap::new();
                self.space();
                if self.bytes.get(self.at) == Some(&b'}') {
                    self.at += 1;
                    return Ok(Json::Object(fields));
                }
                loop {
                    self.space();
                    let key = self.string()?;
                    self.space();
                    self.expect(b':')?;
                    let value = self.value()?;
                    fields.insert(key, value);
                    self.space();
                    if self.bytes.get(self.at) == Some(&b',') {
                        self.at += 1;
                    } else {
                        self.expect(b'}')?;
                        return Ok(Json::Object(fields));
                    }
                }
            }
            Some(_) => {
                let start = self.at;
                while self.at < self.bytes.len()
                    && matches!(self.bytes[self.at], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                {
                    self.at += 1;
                }
                if start == self.at {
                    return Err(format!("no value at byte {start}"));
                }
                Ok(Json::Number(String::from_utf8_lossy(&self.bytes[start..self.at]).into_owned()))
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = Vec::new();
        loop {
            let byte = *self.bytes.get(self.at).ok_or("the text ends inside a string")?;
            self.at += 1;
            match byte {
                b'"' => return String::from_utf8(out).map_err(|e| e.to_string()),
                b'\\' => {
                    let escape = *self.bytes.get(self.at).ok_or("the text ends inside an escape")?;
                    self.at += 1;
                    match escape {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'u' => {
                            let unit = self.unit()?;
                            let code = if (0xD800..0xDC00).contains(&unit)
                                && self.bytes[self.at..].starts_with(b"\\u")
                            {
                                self.at += 2;
                                let low = self.unit()?;
                                0x10000 + ((unit - 0xD800) << 10) + (low.wrapping_sub(0xDC00) & 0x3FF)
                            } else {
                                unit
                            };
                            let c = char::from_u32(code).unwrap_or('\u{FFFD}');
                            out.extend_from_slice(c.to_string().as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                other => out.push(other),
            }
        }
    }

    fn unit(&mut self) -> Result<u32, String> {
        let digits = self.bytes.get(self.at..self.at + 4).ok_or("a short \\u escape")?;
        self.at += 4;
        u32::from_str_radix(&String::from_utf8_lossy(digits), 16).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::Json;

    #[test]
    fn reads_an_answer() {
        let answer = Json::parse(
            r#"{"ok":false,"errors":[{"file":"/a/b.scala","line":6,"message":"a \"b\"\n c é"}],"ms":{"total":0.04}}"#,
        )
        .unwrap();
        assert!(!answer.get("ok").is_true());
        let error = &answer.get("errors").items()[0];
        assert_eq!(error.get("message").text(), Some("a \"b\"\n c é"));
        assert_eq!(error.get("line"), &Json::Number("6".to_string()));
        assert_eq!(answer.get("absent"), &Json::Null);
    }
}
