//! The tokens of a Java source, as the reference's `JavaScanners.JavaScanner` reads them (dotty 3.8.4): unicode
//! escapes decoded below the tokens (`decodeUni`), a CR LF one LF, a string or character literal's escapes, a text
//! block's indentation stripped, numbers in base 8, 10 and 16 (no binary literal, no underscore: the reference reads
//! neither), with the reference's words for what it refuses.

/// A token's kind (`JavaTokens`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum T {
    Eof,
    Ident,
    CharLit,
    IntLit,
    LongLit,
    FloatLit,
    DoubleLit,
    StringLit,
    // keywords the declarations read
    Abstract,
    Class,
    Default,
    Enum,
    Extends,
    False,
    Final,
    Implements,
    Import,
    Interface,
    Native,
    New,
    Package,
    Permits,
    Private,
    Protected,
    Public,
    Sealed,
    Static,
    Strictfp,
    Super,
    Synchronized,
    Throws,
    Transient,
    True,
    Void,
    Volatile,
    Boolean,
    Byte,
    Short,
    Char,
    Int,
    Long,
    Float,
    Double,
    /// A keyword no declaration reads (`if`, `return`, ...): it stands in a body or an initializer, which are skipped.
    OtherKeyword,
    // punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semi,
    Comma,
    Dot,
    DotDotDot,
    At,
    Equals,
    Lt,
    Gt,
    GtEq,
    GtGt,
    GtGtEq,
    GtGtGt,
    GtGtGtEq,
    QMark,
    Amp,
    Minus,
    Bang,
    Asterisk,
    /// Any other operator: only bodies and initializers hold them.
    Op,
}

fn keyword(s: &str) -> Option<T> {
    Some(match s {
        "abstract" => T::Abstract,
        "class" => T::Class,
        "default" => T::Default,
        "enum" => T::Enum,
        "extends" => T::Extends,
        "false" => T::False,
        "final" => T::Final,
        "implements" => T::Implements,
        "import" => T::Import,
        "interface" => T::Interface,
        "native" => T::Native,
        "new" => T::New,
        "package" => T::Package,
        "permits" => T::Permits,
        "private" => T::Private,
        "protected" => T::Protected,
        "public" => T::Public,
        "sealed" => T::Sealed,
        "static" => T::Static,
        "strictfp" => T::Strictfp,
        "super" => T::Super,
        "synchronized" => T::Synchronized,
        "throws" => T::Throws,
        "transient" => T::Transient,
        "true" => T::True,
        "void" => T::Void,
        "volatile" => T::Volatile,
        "boolean" => T::Boolean,
        "byte" => T::Byte,
        "short" => T::Short,
        "char" => T::Char,
        "int" => T::Int,
        "long" => T::Long,
        "float" => T::Float,
        "double" => T::Double,
        "if" | "for" | "else" | "this" | "null" | "do" | "throw" | "try" | "catch" | "finally" | "while" | "return" | "instanceof" | "const" | "break" | "continue" | "goto"
        | "switch" | "assert" => T::OtherKeyword,
        _ => return None,
    })
}

const SU: char = '\u{1A}';
const CR: char = '\r';
const LF: char = '\n';
const FF: char = '\u{0C}';

/// The source's characters with unicode escapes decoded and a CR LF made one LF, each with the byte offset it
/// starts at, and whether it came from an escape (`isUnicodeEscape`).
fn decode(text: &str) -> Vec<(char, u32, bool)> {
    let bytes: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::with_capacity(bytes.len() + 1);
    let mut i = 0;
    let mut slashes = 0usize;
    while i < bytes.len() {
        let (at, c) = bytes[i];
        if c == '\\' && slashes % 2 == 0 && bytes.get(i + 1).is_some_and(|&(_, n)| n == 'u') {
            let mut j = i + 1;
            while bytes.get(j).is_some_and(|&(_, n)| n == 'u') {
                j += 1;
            }
            let hex: String = bytes.get(j..j + 4).map(|h| h.iter().map(|&(_, x)| x).collect()).unwrap_or_default();
            if let Some(v) = (hex.len() == 4).then(|| u32::from_str_radix(&hex, 16).ok()).flatten() {
                i = j + 4;
                slashes = 0;
                // A high surrogate's escape followed by a low one's is the one character of the pair, as Java's UTF-16
                // text holds it; a lone half is teq's stand-in for its unit (`crate::text`), as a Scala literal's is,
                // and so is each half of a pair whose character is a stand-in's own (`text::pair_joins`).
                if (0xD800..0xDC00).contains(&v) && crate::text::pair_joins(v) && bytes.get(i).is_some_and(|&(_, n)| n == '\\') && bytes.get(i + 1).is_some_and(|&(_, n)| n == 'u') {
                    let mut k = i + 1;
                    while bytes.get(k).is_some_and(|&(_, n)| n == 'u') {
                        k += 1;
                    }
                    let low: String = bytes.get(k..k + 4).map(|h| h.iter().map(|&(_, x)| x).collect()).unwrap_or_default();
                    if let Some(w) = (low.len() == 4).then(|| u32::from_str_radix(&low, 16).ok()).flatten().filter(|w| (0xDC00..0xE000).contains(w)) {
                        let c = char::from_u32(0x10000 + ((v - 0xD800) << 10) + (w - 0xDC00)).unwrap_or('\u{FFFD}');
                        out.push((c, bytes[j + 3].0 as u32, true));
                        i = k + 4;
                        continue;
                    }
                }
                let c = if (0xD800..0xE000).contains(&v) { crate::text::lone_surrogate(v) } else { char::from_u32(v).unwrap_or('\u{FFFD}') };
                // An escaped character stands where its escape's last character does: the reference's place for it
                // (`charOffset - 1`, the reader past the escape), a token's and an error's alike.
                out.push((c, bytes[j + 3].0 as u32, true));
                continue;
            }
        }
        slashes = if c == '\\' { slashes + 1 } else { 0 };
        if c == CR && bytes.get(i + 1).is_some_and(|&(_, n)| n == LF) {
            i += 1;
            continue;
        }
        out.push((c, at as u32, false));
        i += 1;
    }
    out.push((SU, text.len() as u32, false));
    out
}

#[derive(Clone)]
struct State {
    pos: usize,
    tok: T,
    offset: u32,
    name: String,
    str_val: String,
    base: u32,
}

pub struct Scanner {
    chars: Vec<(char, u32, bool)>,
    s: State,
    /// One token of lookahead (`next`), as the reference keeps it.
    next: Option<State>,
    lit: String,
    /// What the scanner refused, by byte offset, in the reference's words.
    pub errors: Vec<(u32, String)>,
}

/// Java's `Character.isWhitespace`: a Unicode space separator but a no-break one, a line or paragraph separator, or
/// one of `\t \n \u000B \f \r` and the separators `\u001C`-`\u001F`.
fn java_whitespace(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{B}' | '\u{C}' | '\r' | '\u{1C}'..='\u{1F}') || (c.is_whitespace() && !matches!(c, '\u{85}' | '\u{A0}' | '\u{2007}' | '\u{202F}'))
}

fn digit(c: char, base: u32) -> i64 {
    c.to_digit(36).map_or(-1, |d| if d < base { d as i64 } else { -1 })
}

impl Scanner {
    pub fn new(text: &str) -> Scanner {
        let mut s = Scanner { chars: decode(text), s: State { pos: 0, tok: T::Eof, offset: 0, name: String::new(), str_val: String::new(), base: 10 }, next: None, lit: String::new(), errors: Vec::new() };
        s.next_token();
        s
    }

    pub fn tok(&self) -> T {
        self.s.tok
    }
    pub fn offset(&self) -> u32 {
        self.s.offset
    }
    pub fn name(&self) -> &str {
        &self.s.name
    }
    pub fn str_val(&self) -> &str {
        &self.s.str_val
    }
    pub fn set_tok(&mut self, t: T) {
        self.s.tok = t;
    }

    fn ch(&self) -> char {
        self.chars.get(self.s.pos).map_or(SU, |c| c.0)
    }
    fn ch_at(&self, k: usize) -> char {
        self.chars.get(self.s.pos + k).map_or(SU, |c| c.0)
    }
    fn char_offset(&self) -> u32 {
        self.chars.get(self.s.pos).map_or(0, |c| c.1)
    }
    fn is_unicode_escape(&self) -> bool {
        self.chars.get(self.s.pos).is_some_and(|c| c.2)
    }
    fn next_char(&mut self) {
        if self.s.pos < self.chars.len() - 1 {
            self.s.pos += 1;
        }
    }
    fn at_end(&self) -> bool {
        self.s.pos >= self.chars.len() - 1
    }
    fn error(&mut self, at: u32, msg: &str) {
        self.errors.push((at, msg.to_string()));
    }

    pub fn next_token(&mut self) {
        if let Some(n) = self.next.take() {
            self.s = n;
            return;
        }
        self.fetch();
    }

    /// The token after the current one, the scanner left where it is (`lookaheadToken`).
    pub fn lookahead(&mut self) -> T {
        self.lookahead_state().tok
    }

    /// The token two after the current one, with its name, the scanner left where it is (the reference's
    /// `LookaheadScanner` for `non-sealed`).
    pub fn lookahead2(&mut self) -> (T, String) {
        let (s, next, errors) = (self.s.clone(), self.next.clone(), self.errors.len());
        self.next_token();
        self.next_token();
        let out = (self.s.tok, self.s.name.clone());
        self.s = s;
        self.next = next;
        self.errors.truncate(errors);
        out
    }

    fn lookahead_state(&mut self) -> State {
        if self.next.is_none() {
            let cur = self.s.clone();
            let errors = self.errors.len();
            self.fetch();
            let after = std::mem::replace(&mut self.s, cur);
            // What the lookahead refused is told when the token is read.
            self.errors.truncate(errors);
            self.next = Some(after);
        }
        self.next.clone().unwrap()
    }

    fn put(&mut self, c: char) {
        self.lit.push(c);
    }
    fn set_str(&mut self) {
        self.s.str_val = std::mem::take(&mut self.lit);
    }

    fn fetch(&mut self) {
        self.lit.clear();
        loop {
            self.s.offset = self.char_offset();
            let c = self.ch();
            match c {
                ' ' | '\t' | CR | LF | FF => {
                    self.next_char();
                    continue;
                }
                'A'..='Z' | 'a'..='z' | '$' | '_' => {
                    self.put(c);
                    self.next_char();
                    return self.ident_rest();
                }
                '0' => {
                    self.put(c);
                    self.next_char();
                    if self.ch() == 'x' || self.ch() == 'X' {
                        self.next_char();
                        self.s.base = 16;
                    } else {
                        self.s.base = 8;
                    }
                    return self.number();
                }
                '1'..='9' => {
                    self.s.base = 10;
                    return self.number();
                }
                '"' => {
                    self.next_char();
                    if self.ch() != '"' {
                        while self.ch() != '"' && (self.is_unicode_escape() || (self.ch() != CR && self.ch() != LF && self.ch() != SU)) {
                            self.litch(false, false);
                        }
                        if self.ch() == '"' {
                            self.s.tok = T::StringLit;
                            self.set_str();
                            self.next_char();
                        } else {
                            let at = self.s.offset;
                            self.error(at, "unclosed string literal");
                            self.s.tok = T::StringLit;
                            self.set_str();
                        }
                    } else {
                        self.next_char();
                        if self.ch() != '"' {
                            self.s.tok = T::StringLit;
                            self.set_str();
                        } else {
                            self.next_char();
                            self.text_block();
                        }
                    }
                    return;
                }
                '\'' => {
                    self.next_char();
                    self.litch(false, false);
                    // A character literal is one UTF-16 unit: a supplementary character, written or escaped as a
                    // pair, is two, the second where the closing quote is due (a lone surrogate's stand-in is one).
                    let two_units = self.lit.chars().last().is_some_and(|c| c as u32 > 0xFFFF && crate::text::surrogate_of(c).is_none());
                    if self.ch() == '\'' && !two_units {
                        self.next_char();
                        self.s.tok = T::CharLit;
                        self.set_str();
                    } else {
                        let at = self.s.offset;
                        self.error(at, "unclosed character literal");
                        self.s.tok = T::CharLit;
                        self.set_str();
                    }
                    return;
                }
                '/' => {
                    self.next_char();
                    match self.ch() {
                        '/' => {
                            while !matches!(self.ch(), CR | LF | SU) {
                                self.next_char();
                            }
                            continue;
                        }
                        '*' => {
                            self.next_char();
                            loop {
                                match self.ch() {
                                    SU if self.at_end() => {
                                        let at = self.s.offset;
                                        self.error(at, "unclosed comment");
                                        break;
                                    }
                                    '*' => {
                                        self.next_char();
                                        if self.ch() == '/' {
                                            self.next_char();
                                            break;
                                        }
                                    }
                                    _ => self.next_char(),
                                }
                            }
                            continue;
                        }
                        _ => {
                            if self.ch() == '=' {
                                self.next_char();
                            }
                            self.s.tok = T::Op;
                            return;
                        }
                    }
                }
                '.' => {
                    self.s.tok = T::Dot;
                    self.next_char();
                    if self.ch().is_ascii_digit() {
                        self.put('.');
                        return self.fraction();
                    } else if self.ch() == '.' {
                        self.next_char();
                        if self.ch() == '.' {
                            self.next_char();
                            self.s.tok = T::DotDotDot;
                        } else {
                            let at = self.s.offset;
                            self.error(at, "`.` character expected");
                        }
                    }
                    return;
                }
                SU if self.at_end() => {
                    self.s.tok = T::Eof;
                    return;
                }
                _ => {}
            }
            self.next_char();
            let two = |s: &mut Scanner, after: char, long: T, short: T| {
                if s.ch() == after {
                    s.next_char();
                    long
                } else {
                    short
                }
            };
            self.s.tok = match c {
                ';' => T::Semi,
                ',' => T::Comma,
                '(' => T::LParen,
                ')' => T::RParen,
                '{' => T::LBrace,
                '}' => T::RBrace,
                '[' => T::LBracket,
                ']' => T::RBracket,
                '@' => T::At,
                '?' => T::QMark,
                '~' | ':' => T::Op,
                '=' => two(self, '=', T::Op, T::Equals),
                '!' => two(self, '=', T::Op, T::Bang),
                '*' => two(self, '=', T::Op, T::Asterisk),
                '^' | '%' => two(self, '=', T::Op, T::Op),
                '&' => {
                    if self.ch() == '&' || self.ch() == '=' {
                        self.next_char();
                        T::Op
                    } else {
                        T::Amp
                    }
                }
                '|' | '+' => {
                    if self.ch() == c || self.ch() == '=' {
                        self.next_char();
                    }
                    T::Op
                }
                '-' => {
                    if self.ch() == '-' || self.ch() == '=' {
                        self.next_char();
                        T::Op
                    } else {
                        T::Minus
                    }
                }
                '<' => {
                    if self.ch() == '=' {
                        self.next_char();
                        T::Op
                    } else if self.ch() == '<' {
                        self.next_char();
                        if self.ch() == '=' {
                            self.next_char();
                        }
                        T::Op
                    } else {
                        T::Lt
                    }
                }
                '>' => {
                    if self.ch() == '=' {
                        self.next_char();
                        T::GtEq
                    } else if self.ch() == '>' {
                        self.next_char();
                        if self.ch() == '=' {
                            self.next_char();
                            T::GtGtEq
                        } else if self.ch() == '>' {
                            self.next_char();
                            if self.ch() == '=' {
                                self.next_char();
                                T::GtGtGtEq
                            } else {
                                T::GtGtGt
                            }
                        } else {
                            T::GtGt
                        }
                    } else {
                        T::Gt
                    }
                }
                c if super::ident::is_start(c) => {
                    self.put(c);
                    return self.ident_rest();
                }
                c => {
                    // `illegal character`, the loop past it already (the `next_char()` above, the reference's
                    // `nextChar()`): one error a UTF-16 unit, as the reference reads them, so a code point above
                    // U+FFFF is its surrogate pair's two.
                    let at = self.s.offset;
                    let mut units = [0u16; 2];
                    for u in c.encode_utf16(&mut units).iter() {
                        self.error(at, &format!("illegal character: {}", u));
                    }
                    continue;
                }
            };
            return;
        }
    }

    fn ident_rest(&mut self) {
        loop {
            let c = self.ch();
            if super::ident::is_part(c) {
                self.put(c);
                self.next_char();
            } else {
                break;
            }
        }
        let w = std::mem::take(&mut self.lit);
        self.s.tok = keyword(&w).unwrap_or(T::Ident);
        self.s.name = w;
    }

    /// One character of a string or character literal (`getlitch`): an escape decoded, a text block's escaped line
    /// end dropped. Every backslash starts one, a unicode escape's too (`\u005cn` is a line feed): the unicode
    /// escapes are the stage before.
    fn litch(&mut self, scan_only: bool, in_text_block: bool) {
        let mut skip = false;
        let c = if self.ch() == '\\' {
            self.next_char();
            if ('0'..='7').contains(&self.ch()) {
                let lead = self.ch();
                let mut oct = self.ch().to_digit(8).unwrap();
                self.next_char();
                if ('0'..='7').contains(&self.ch()) {
                    oct = oct * 8 + self.ch().to_digit(8).unwrap();
                    self.next_char();
                    if lead <= '3' && ('0'..='7').contains(&self.ch()) {
                        oct = oct * 8 + self.ch().to_digit(8).unwrap();
                        self.next_char();
                    }
                }
                char::from_u32(oct).unwrap_or('\0')
            } else {
                let x = match self.ch() {
                    'b' => '\u{8}',
                    's' => ' ',
                    't' => '\t',
                    'n' => '\n',
                    'f' => '\u{C}',
                    'r' => '\r',
                    '"' => '"',
                    '\'' => '\'',
                    '\\' => '\\',
                    CR | LF if in_text_block => {
                        if !scan_only {
                            self.next_char();
                        }
                        skip = true;
                        '\0'
                    }
                    other => {
                        if !scan_only {
                            let at = self.char_offset();
                            self.error(at, "invalid escape character");
                        }
                        other
                    }
                };
                if !skip {
                    self.next_char();
                }
                x
            }
        } else {
            let c = self.ch();
            self.next_char();
            c
        };
        if !skip && !scan_only {
            self.put(c);
        }
    }

    /// A text block, from past its opening `"""` (`getTextBlock`): the lines' common indentation and trailing white
    /// space dropped, line ends made `\n`.
    fn text_block(&mut self) {
        while self.ch() == ' ' || self.ch() == '\t' || self.ch() == FF {
            self.next_char();
        }
        if self.ch() != LF && self.ch() != CR {
            let at = self.s.offset;
            self.error(at, "illegal text block open delimiter sequence, missing line terminator");
            self.s.tok = T::StringLit;
            self.set_str();
            return;
        }
        self.next_char();
        // The lookahead pass: the common white space prefix and where the block ends.
        let start = self.s.pos;
        let mut common = usize::MAX;
        let mut end = None;
        let mut line_ws = 0usize;
        let mut only_ws = true;
        let mut k = start;
        let at = |k: usize, s: &Scanner| s.chars.get(k).map_or(SU, |c| c.0);
        while k < self.chars.len() - 1 {
            let c = at(k, self);
            if c == '"' {
                if at(k + 1, self) == '"' && at(k + 2, self) == '"' {
                    common = common.min(line_ws);
                    end = Some(k);
                    break;
                }
                only_ws = false;
                k += 1;
            } else if c == CR || c == LF {
                k += 1;
                if !only_ws {
                    common = common.min(line_ws);
                }
                line_ws = 0;
                only_ws = true;
            } else if only_ws && java_whitespace(c) {
                k += 1;
                line_ws += 1;
            } else {
                only_ws = false;
                // an escape takes its following character with it (`getlitch(scanOnly = true)`)
                k += if c == '\\' && !self.chars[k].2 { 2 } else { 1 };
            }
        }
        let Some(end) = end else {
            let at = self.s.offset;
            self.error(at, "unclosed text block");
            self.s.tok = T::StringLit;
            self.set_str();
            return;
        };
        while self.s.pos < end {
            let mut prefix = common;
            while prefix > 0 && self.ch() != CR && self.ch() != LF && self.s.pos < end {
                self.next_char();
                prefix -= 1;
            }
            let mut trailing = 0usize;
            let mut escaped_newline = false;
            while self.ch() != CR && self.ch() != LF && self.s.pos < end && !escaped_newline {
                if java_whitespace(self.ch()) {
                    trailing += 1;
                } else {
                    trailing = 0;
                }
                if self.ch() == '\\' && matches!(self.ch_at(1), CR | LF) {
                    escaped_newline = true;
                }
                self.litch(false, true);
            }
            for _ in 0..trailing.min(self.lit.chars().count()) {
                self.lit.pop();
            }
            if (self.ch() == CR || self.ch() == LF) && !escaped_newline {
                self.next_char();
                self.put('\n');
            }
        }
        self.s.tok = T::StringLit;
        self.set_str();
        self.next_char();
        self.next_char();
        self.next_char();
    }

    fn fraction(&mut self) {
        self.s.tok = T::DoubleLit;
        while self.ch().is_ascii_digit() {
            let c = self.ch();
            self.put(c);
            self.next_char();
        }
        if self.ch() == 'e' || self.ch() == 'E' {
            let mut k = 1;
            if matches!(self.ch_at(k), '+' | '-') {
                k += 1;
            }
            if self.ch_at(k).is_ascii_digit() {
                let c = self.ch();
                self.put(c);
                self.next_char();
                if matches!(self.ch(), '+' | '-') {
                    let c = self.ch();
                    self.put(c);
                    self.next_char();
                }
                while self.ch().is_ascii_digit() {
                    let c = self.ch();
                    self.put(c);
                    self.next_char();
                }
            }
            self.s.tok = T::DoubleLit;
        }
        if self.ch() == 'd' || self.ch() == 'D' {
            let c = self.ch();
            self.put(c);
            self.next_char();
            self.s.tok = T::DoubleLit;
        } else if self.ch() == 'f' || self.ch() == 'F' {
            let c = self.ch();
            self.put(c);
            self.next_char();
            self.s.tok = T::FloatLit;
        }
        self.set_str();
    }

    fn number(&mut self) {
        let b = if self.s.base < 10 { 10 } else { self.s.base };
        while digit(self.ch(), b) >= 0 {
            let c = self.ch();
            self.put(c);
            self.next_char();
        }
        self.s.tok = T::IntLit;
        if self.s.base <= 10 && self.ch() == '.' {
            let after = self.ch_at(1);
            if after.is_ascii_digit() || matches!(after, 'd' | 'D' | 'e' | 'E' | 'f' | 'F') || !(after.is_alphabetic() || after == '_' || after == '$') {
                self.put('.');
                self.next_char();
                return self.fraction();
            }
        }
        if self.s.base <= 10 && matches!(self.ch(), 'e' | 'E' | 'f' | 'F' | 'd' | 'D') {
            return self.fraction();
        }
        self.set_str();
        if self.ch() == 'l' || self.ch() == 'L' {
            self.next_char();
            self.s.tok = T::LongLit;
        }
    }

    /// The integer the current literal holds (`intVal`), negated where a `-` stood before it; `None` where the
    /// reference reports it malformed or too large (the report made).
    pub fn int_val(&mut self, negated: bool) -> Option<i64> {
        if self.s.tok == T::CharLit && !negated {
            return Some(self.s.str_val.chars().next().map_or(0, |c| c as i64));
        }
        let base = self.s.base as i64;
        let divider = if base == 10 { 1 } else { 2 };
        let limit: i64 = if self.s.tok == T::LongLit { i64::MAX } else { i32::MAX as i64 };
        let mut value: i64 = 0;
        let digits: Vec<char> = self.s.str_val.chars().collect();
        for c in digits {
            let d = digit(c, base as u32);
            if d < 0 {
                let at = self.s.offset;
                self.error(at, "malformed integer number");
                return None;
            }
            if value < 0 || limit / (base / divider) < value || (limit - (d / divider) < value * (base / divider) && !(negated && limit == value.wrapping_mul(base) - 1 + d)) {
                let at = self.s.offset;
                self.error(at, "integer number too large");
                return None;
            }
            value = value.wrapping_mul(base) + d;
        }
        Some(if negated { -value } else { value })
    }

    /// The floating point number the current literal holds (`floatVal`).
    pub fn float_val(&mut self, negated: bool) -> Option<f64> {
        let limit = if self.s.tok == T::DoubleLit { f64::MAX } else { f32::MAX as f64 };
        let text = self.s.str_val.trim_end_matches(['d', 'D', 'f', 'F']);
        match text.parse::<f64>() {
            Ok(v) => {
                if v > limit {
                    let at = self.s.offset;
                    self.error(at, "floating point number too large");
                }
                Some(if negated { -v } else { v })
            }
            Err(_) => {
                let at = self.s.offset;
                self.error(at, "malformed floating point number");
                None
            }
        }
    }
}
