//! Regular expressions with the syntax and semantics of `java.util.regex`: a backtracking
//! matcher over the characters of the input by UTF-16 index (`unit_chars`), so that a position
//! is a UTF-16 index as the JVM's are, a supplementary character is one character that takes two
//! positions, and a search that starts inside a pair sees its low surrogate alone.

#[derive(Clone, Copy, PartialEq)]
enum Greed {
    Greedy,
    Lazy,
    Possessive,
}

/// How characters compare under `(?i)`: as they are, the ASCII letters alone case-blind (Java's
/// `CASE_INSENSITIVE`), or every letter so (with `UNICODE_CASE`, `(?iu)`).
#[derive(Clone, Copy, PartialEq)]
enum Fold {
    Exact,
    Ascii,
    Unicode,
}

enum Node {
    Empty,
    Char(char, Fold),
    Any(bool),
    Class(Box<Class>, Fold),
    LineStart(bool),
    LineEnd(bool),
    InputStart,
    InputEnd,
    InputEndNl,
    WordBoundary(bool),
    Group(Option<usize>, Box<Node>),
    /// `(?>X)`: the first way `X` matches, never taken back (Java's `X*+` is `(?>X*)`).
    Atomic(Box<Node>),
    Concat(Vec<Node>),
    Alt(Vec<Node>),
    Repeat(Box<Node>, u32, Option<u32>, Greed),
    Backref(usize, Fold),
    /// A lookaround; a lookbehind keeps the least and the most units its node takes (`span`), the
    /// most none when unbounded, so that only the starts that can end at the position are tried.
    Look { ahead: bool, negative: bool, node: Box<Node>, span: (usize, Option<usize>) },
}

/// The least and the most UTF-16 units a node matches, the most none when unbounded: what a
/// lookbehind tries its node from (Java bounds a lookbehind; a backreference here is unbounded).
fn span(node: &Node) -> (usize, Option<usize>) {
    match node {
        Node::Empty | Node::LineStart(_) | Node::LineEnd(_) | Node::InputStart | Node::InputEnd | Node::InputEndNl | Node::WordBoundary(_) | Node::Look { .. } => (0, Some(0)),
        Node::Char(c, Fold::Exact) => (width(*c), Some(width(*c))),
        Node::Char(..) | Node::Any(_) | Node::Class(..) => (1, Some(2)),
        Node::Backref(..) => (0, None),
        Node::Group(_, inner) | Node::Atomic(inner) => span(inner),
        Node::Concat(items) => items.iter().map(span).fold((0, Some(0)), |(lo, hi), (a, b)| (lo + a, hi.zip(b).map(|(x, y)| x + y))),
        Node::Alt(items) => items.iter().map(span).reduce(|(lo, hi), (a, b)| (lo.min(a), hi.zip(b).map(|(x, y)| x.max(y)))).unwrap_or((0, Some(0))),
        Node::Repeat(inner, min, max, _) => {
            let (lo, hi) = span(inner);
            (lo * *min as usize, hi.zip(*max).map(|(h, m)| h * m as usize))
        }
    }
}

#[derive(Clone)]
struct Class {
    negated: bool,
    items: Vec<Item>,
    /// Classes joined by `&&`, all of which a character has to be in.
    intersections: Vec<Class>,
}

#[derive(Clone)]
enum Item {
    Range(char, char),
    Pred(Pred),
    Sub(Class),
    /// While a class is parsed, its characters named alone, as far as the class goes.
    Bits,
}

#[derive(Clone, Copy)]
enum Pred {
    Digit,
    NotDigit,
    Word,
    NotWord,
    Space,
    NotSpace,
    HSpace,
    NotHSpace,
    VSpace,
    NotVSpace,
    Letter,
    Upper,
    Lower,
    Title,
    /// Lowercase, uppercase or titlecase: a case-insensitive `Lower`, `Upper` or `Title` of
    /// Unicode's (the JDK's `CharPredicates`: `LOWERCASE().union(UPPERCASE(), TITLECASE())`).
    Cased,
    Alnum,
    Punct,
    Cntrl,
    XDigit,
    Blank,
    Print,
    Ascii,
    Numeric,
    Any,
    // The POSIX classes, of ASCII characters alone but under `UNICODE_CHARACTER_CLASS`; under
    // `CASE_INSENSITIVE` `\p{Lower}` and `\p{Upper}` take either case.
    AsciiLower,
    AsciiUpper,
    AsciiCased,
    AsciiAlpha,
    AsciiAlnum,
    AsciiCntrl,
    AsciiGraph,
    AsciiPrint,
}

/// The characters of `s` by UTF-16 index, as `Character.codePointAt` reads them: a supplementary
/// character at the index of its high surrogate and the stand-in of its low one at the next; and
/// per index its byte offset, `u32::MAX` inside a pair, with the end's last.
pub fn unit_chars(s: &str) -> (Vec<char>, Vec<u32>) {
    let mut chars = Vec::with_capacity(s.len());
    let mut bytes = Vec::with_capacity(s.len() + 1);
    for (at, c) in s.char_indices() {
        chars.push(c);
        bytes.push(at as u32);
        if width(c) == 2 {
            let mut units = [0u16; 2];
            c.encode_utf16(&mut units);
            chars.push(crate::text::lone_surrogate(units[1] as u32));
            bytes.push(u32::MAX);
        }
    }
    bytes.push(s.len() as u32);
    (chars, bytes)
}

/// The UTF-16 units a character of the input takes.
#[inline]
fn width(c: char) -> usize {
    if (c as u32) >= 0x10000 && crate::text::surrogate_of(c).is_none() { 2 } else { 1 }
}

/// The unit a position of the input starts with.
fn unit_at(c: char) -> u16 {
    match crate::text::surrogate_of(c) {
        Some(u) => u,
        None => c.encode_utf16(&mut [0; 2])[0],
    }
}

fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `Character.isTitleCase` of JDK 24: the titlecase letters (`Lt`), which are neither lowercase
/// nor uppercase.
fn is_titlecase(c: char) -> bool {
    matches!(c, '\u{1c5}' | '\u{1c8}' | '\u{1cb}' | '\u{1f2}' | '\u{1f88}'..='\u{1f8f}' | '\u{1f98}'..='\u{1f9f}' | '\u{1fa8}'..='\u{1faf}' | '\u{1fbc}' | '\u{1fcc}' | '\u{1ffc}')
}

fn eq_ci(a: char, b: char, fold: Fold) -> bool {
    match fold {
        Fold::Exact => a == b,
        Fold::Ascii => a.eq_ignore_ascii_case(&b),
        Fold::Unicode => a == b || a.to_lowercase().eq(b.to_lowercase()) || a.to_uppercase().eq(b.to_uppercase()),
    }
}

impl Pred {
    fn test(self, c: char) -> bool {
        match self {
            Pred::Digit => c.is_ascii_digit(),
            Pred::NotDigit => !c.is_ascii_digit(),
            Pred::Word => c.is_ascii_alphanumeric() || c == '_',
            Pred::NotWord => !(c.is_ascii_alphanumeric() || c == '_'),
            Pred::Space => matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r'),
            Pred::NotSpace => !matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r'),
            Pred::HSpace => matches!(c, ' ' | '\t' | '\u{a0}' | '\u{1680}' | '\u{180e}' | '\u{2000}'..='\u{200a}' | '\u{202f}' | '\u{205f}' | '\u{3000}'),
            Pred::NotHSpace => !Pred::HSpace.test(c),
            Pred::VSpace => matches!(c, '\n' | '\u{b}' | '\u{c}' | '\r' | '\u{85}' | '\u{2028}' | '\u{2029}'),
            Pred::NotVSpace => !Pred::VSpace.test(c),
            Pred::Letter => c.is_alphabetic(),
            Pred::Upper => c.is_uppercase(),
            Pred::Lower => c.is_lowercase(),
            Pred::Title => is_titlecase(c),
            Pred::Cased => c.is_lowercase() || c.is_uppercase() || is_titlecase(c),
            Pred::Alnum => c.is_alphanumeric(),
            Pred::Punct => c.is_ascii_punctuation(),
            Pred::Cntrl => c.is_control(),
            Pred::XDigit => c.is_ascii_hexdigit(),
            Pred::Blank => c == ' ' || c == '\t',
            Pred::Print => !c.is_control(),
            Pred::Ascii => c.is_ascii(),
            Pred::Numeric => c.is_numeric(),
            Pred::Any => true,
            Pred::AsciiLower => c.is_ascii_lowercase(),
            Pred::AsciiUpper => c.is_ascii_uppercase(),
            Pred::AsciiCased | Pred::AsciiAlpha => c.is_ascii_alphabetic(),
            Pred::AsciiAlnum => c.is_ascii_alphanumeric(),
            Pred::AsciiCntrl => c.is_ascii_control(),
            Pred::AsciiGraph => c.is_ascii_graphic(),
            Pred::AsciiPrint => c.is_ascii_graphic() || c == ' ',
        }
    }
}

impl Class {
    fn matches(&self, c: char, fold: Fold) -> bool {
        let mut hit = self.items.iter().any(|item| match item {
            Item::Range(a, b) => {
                let range = *a..=*b;
                range.contains(&c)
                    || match fold {
                        Fold::Exact => false,
                        Fold::Ascii => range.contains(&c.to_ascii_lowercase()) || range.contains(&c.to_ascii_uppercase()),
                        Fold::Unicode => c.to_lowercase().any(|l| range.contains(&l)) || c.to_uppercase().any(|u| range.contains(&u)),
                    }
            }
            Item::Pred(p) => p.test(c),
            Item::Sub(sub) => sub.matches(c, fold),
            Item::Bits => false,
        });
        if hit {
            hit = self.intersections.iter().all(|k| k.matches(c, fold));
        }
        hit != self.negated
    }
}

pub struct Regex {
    root: Node,
    pub groups: usize,
    pub names: Vec<(String, usize)>,
    /// A pattern with a supplementary character or a surrogate starts a search at a character,
    /// never inside a pair, as the JVM's `StartS` does; another starts at every index.
    supplementary: bool,
}

fn names_supplementary(node: &Node) -> bool {
    let wide = |c: char| (c as u32) >= 0x10000;
    fn class_wide(class: &Class, wide: &dyn Fn(char) -> bool) -> bool {
        class.items.iter().any(|item| match item {
            Item::Range(lo, hi) => wide(*lo) || wide(*hi),
            Item::Sub(sub) => class_wide(sub, wide),
            Item::Pred(_) | Item::Bits => false,
        }) || class.intersections.iter().any(|c| class_wide(c, wide))
    }
    match node {
        Node::Char(c, _) => wide(*c),
        Node::Class(class, _) => class_wide(class, &wide),
        Node::Group(_, inner) | Node::Atomic(inner) | Node::Repeat(inner, ..) | Node::Look { node: inner, .. } => names_supplementary(inner),
        Node::Concat(items) | Node::Alt(items) => items.iter().any(names_supplementary),
        _ => false,
    }
}

#[derive(Clone, Copy)]
struct Flags {
    ci: bool,
    unicode_case: bool,
    /// `UNICODE_CHARACTER_CLASS` (`(?U)`): the POSIX classes are Unicode's.
    unicode_class: bool,
    dotall: bool,
    multiline: bool,
    comments: bool,
}

impl Flags {
    fn fold(&self) -> Fold {
        match (self.ci, self.unicode_case) {
            (false, _) => Fold::Exact,
            (true, false) => Fold::Ascii,
            (true, true) => Fold::Unicode,
        }
    }
}

struct Parser<'a> {
    chars: Vec<char>,
    at: usize,
    groups: usize,
    names: Vec<(String, usize)>,
    src: &'a str,
    /// The flags of the class being parsed, which its properties read.
    class_flags: Flags,
}

type Parsed<T> = Result<T, String>;

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.at + n).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.at += 1;
        }
        c
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn error<T>(&self, msg: &str) -> Parsed<T> {
        Err(format!("{} near index {}\n{}", msg, self.at, self.src))
    }

    fn parse_alt(&mut self, flags: &mut Flags) -> Parsed<Node> {
        let mut alts = vec![self.parse_concat(flags)?];
        while self.eat('|') {
            alts.push(self.parse_concat(flags)?);
        }
        Ok(if alts.len() == 1 { alts.pop().unwrap() } else { Node::Alt(alts) })
    }

    fn parse_concat(&mut self, flags: &mut Flags) -> Parsed<Node> {
        let mut items: Vec<Node> = Vec::new();
        loop {
            match self.peek() {
                None | Some('|') | Some(')') => break,
                // The JDK's comments mode skips ASCII white space alone.
                Some(' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r') if flags.comments => {
                    self.at += 1;
                    continue;
                }
                Some('#') if flags.comments => {
                    while let Some(c) = self.next() {
                        if c == '\n' {
                            break;
                        }
                    }
                    continue;
                }
                _ => {}
            }
            let Some(atom) = self.parse_atom(flags)? else { continue };
            let atom = self.parse_quantifier(atom)?;
            items.push(atom);
        }
        Ok(match items.len() {
            0 => Node::Empty,
            1 => items.pop().unwrap(),
            _ => Node::Concat(items),
        })
    }

    fn parse_quantifier(&mut self, atom: Node) -> Parsed<Node> {
        let (min, max) = match self.peek() {
            Some('*') => {
                self.at += 1;
                (0, None)
            }
            Some('+') => {
                self.at += 1;
                (1, None)
            }
            Some('?') => {
                self.at += 1;
                (0, Some(1))
            }
            Some('{') => {
                let save = self.at;
                self.at += 1;
                let n = self.parse_number()?;
                let (Some(min), close) = (n, self.peek()) else {
                    self.at = save;
                    return self.error("Illegal repetition");
                };
                let max = match close {
                    Some('}') => {
                        self.at += 1;
                        Some(min)
                    }
                    Some(',') => {
                        self.at += 1;
                        let m = self.parse_number()?;
                        if !self.eat('}') {
                            return self.error("Unclosed counted closure");
                        }
                        m
                    }
                    _ => return self.error("Unclosed counted closure"),
                };
                (min, max)
            }
            _ => return Ok(atom),
        };
        let greed = if self.eat('?') {
            Greed::Lazy
        } else if self.eat('+') {
            Greed::Possessive
        } else {
            Greed::Greedy
        };
        Ok(Node::Repeat(Box::new(atom), min, max, greed))
    }

    /// A bound of a counted closure, which the JDK reads as an `int`.
    fn parse_number(&mut self) -> Parsed<Option<u32>> {
        let start = self.at;
        while self.peek().map_or(false, |c| c.is_ascii_digit()) {
            self.at += 1;
        }
        if start == self.at {
            return Ok(None);
        }
        match self.chars[start..self.at].iter().collect::<String>().parse::<u64>() {
            Ok(n) if n <= i32::MAX as u64 => Ok(Some(n as u32)),
            _ => self.error("Illegal repetition range"),
        }
    }

    fn parse_atom(&mut self, flags: &mut Flags) -> Parsed<Option<Node>> {
        let Some(c) = self.next() else { return Ok(None) };
        Ok(Some(match c {
            '(' => return self.parse_group(flags),
            '[' => {
                self.class_flags = *flags;
                Node::Class(Box::new(self.parse_class(true, flags.fold())?), flags.fold())
            }
            '.' => Node::Any(flags.dotall),
            '^' => Node::LineStart(flags.multiline),
            '$' => Node::LineEnd(flags.multiline),
            '\\' => return self.parse_escape(flags).map(Some),
            '*' | '+' | '?' => return self.error("Dangling meta character"),
            c => Node::Char(c, flags.fold()),
        }))
    }

    fn parse_group(&mut self, flags: &mut Flags) -> Parsed<Option<Node>> {
        if self.eat('?') {
            match self.next() {
                Some(':') => {
                    let mut inner = *flags;
                    let node = self.parse_alt(&mut inner)?;
                    self.close_group()?;
                    return Ok(Some(Node::Group(None, Box::new(node))));
                }
                Some('=') | Some('!') => {
                    let negative = self.chars[self.at - 1] == '!';
                    let mut inner = *flags;
                    let node = self.parse_alt(&mut inner)?;
                    self.close_group()?;
                    return Ok(Some(Node::Look { ahead: true, negative, node: Box::new(node), span: (0, None) }));
                }
                Some('>') => {
                    let mut inner = *flags;
                    let node = self.parse_alt(&mut inner)?;
                    self.close_group()?;
                    return Ok(Some(Node::Atomic(Box::new(node))));
                }
                Some('<') => {
                    if matches!(self.peek(), Some('=') | Some('!')) {
                        let negative = self.next() == Some('!');
                        let mut inner = *flags;
                        let node = self.parse_alt(&mut inner)?;
                        self.close_group()?;
                        let span = span(&node);
                        return Ok(Some(Node::Look { ahead: false, negative, node: Box::new(node), span }));
                    }
                    let mut name = String::new();
                    while let Some(c) = self.next() {
                        if c == '>' {
                            break;
                        }
                        name.push(c);
                    }
                    self.groups += 1;
                    let idx = self.groups;
                    self.names.push((name, idx));
                    let mut inner = *flags;
                    let node = self.parse_alt(&mut inner)?;
                    self.close_group()?;
                    return Ok(Some(Node::Group(Some(idx), Box::new(node))));
                }
                Some(c) if c.is_ascii_alphabetic() || c == '-' => {
                    self.at -= 1;
                    let mut on = true;
                    let mut scoped = *flags;
                    loop {
                        match self.next() {
                            Some('-') => on = false,
                            Some('i') => scoped.ci = on,
                            Some('s') => scoped.dotall = on,
                            Some('m') => scoped.multiline = on,
                            Some('x') => scoped.comments = on,
                            Some('u') => scoped.unicode_case = on,
                            // `UNICODE_CHARACTER_CLASS` sets and clears `UNICODE_CASE` with it
                            // (the JDK's `Pattern.addFlag`, `subFlag`).
                            Some('U') => {
                                scoped.unicode_class = on;
                                scoped.unicode_case = on;
                            }
                            Some('d') => {}
                            Some(')') => {
                                *flags = scoped;
                                return Ok(None);
                            }
                            Some(':') => {
                                let node = self.parse_alt(&mut scoped)?;
                                self.close_group()?;
                                return Ok(Some(Node::Group(None, Box::new(node))));
                            }
                            _ => return self.error("Unknown inline modifier"),
                        }
                    }
                }
                _ => return self.error("Unknown group type"),
            }
        }
        self.groups += 1;
        let idx = self.groups;
        let mut inner = *flags;
        let node = self.parse_alt(&mut inner)?;
        self.close_group()?;
        Ok(Some(Node::Group(Some(idx), Box::new(node))))
    }

    fn close_group(&mut self) -> Parsed<()> {
        if self.eat(')') { Ok(()) } else { self.error("Unclosed group") }
    }

    fn parse_escape(&mut self, flags: &Flags) -> Parsed<Node> {
        let Some(c) = self.next() else { return self.error("Unexpected internal error") };
        Ok(match c {
            'd' => Node::Class(Box::new(Class { negated: false, items: vec![Item::Pred(Pred::Digit)], intersections: Vec::new() }), Fold::Exact),
            'D' => Node::Class(Box::new(Class { negated: false, items: vec![Item::Pred(Pred::NotDigit)], intersections: Vec::new() }), Fold::Exact),
            'w' => Node::Class(Box::new(Class { negated: false, items: vec![Item::Pred(Pred::Word)], intersections: Vec::new() }), Fold::Exact),
            'W' => Node::Class(Box::new(Class { negated: false, items: vec![Item::Pred(Pred::NotWord)], intersections: Vec::new() }), Fold::Exact),
            's' => Node::Class(Box::new(Class { negated: false, items: vec![Item::Pred(Pred::Space)], intersections: Vec::new() }), Fold::Exact),
            'S' => Node::Class(Box::new(Class { negated: false, items: vec![Item::Pred(Pred::NotSpace)], intersections: Vec::new() }), Fold::Exact),
            'h' => Node::Class(Box::new(Class { negated: false, items: vec![Item::Pred(Pred::HSpace)], intersections: Vec::new() }), Fold::Exact),
            'H' => Node::Class(Box::new(Class { negated: false, items: vec![Item::Pred(Pred::NotHSpace)], intersections: Vec::new() }), Fold::Exact),
            'v' => Node::Class(Box::new(Class { negated: false, items: vec![Item::Pred(Pred::VSpace)], intersections: Vec::new() }), Fold::Exact),
            'V' => Node::Class(Box::new(Class { negated: false, items: vec![Item::Pred(Pred::NotVSpace)], intersections: Vec::new() }), Fold::Exact),
            'R' => Node::Alt(vec![
                Node::Concat(vec![Node::Char('\r', Fold::Exact), Node::Char('\n', Fold::Exact)]),
                Node::Class(Box::new(Class { negated: false, items: vec![Item::Pred(Pred::VSpace)], intersections: Vec::new() }), Fold::Exact),
            ]),
            'b' => Node::WordBoundary(false),
            'B' => Node::WordBoundary(true),
            'A' => Node::InputStart,
            'z' => Node::InputEnd,
            'Z' => Node::InputEndNl,
            'G' => Node::InputStart,
            'k' => {
                if !self.eat('<') {
                    return self.error("\\k is not followed by '<' for named capturing group");
                }
                let mut name = String::new();
                while let Some(c) = self.next() {
                    if c == '>' {
                        break;
                    }
                    name.push(c);
                }
                match self.names.iter().find(|(n, _)| *n == name) {
                    Some(&(_, idx)) => Node::Backref(idx, flags.fold()),
                    None => return self.error(&format!("named capturing group <{}> does not exist", name)),
                }
            }
            'p' | 'P' => {
                let pred = self.parse_property(flags)?;
                Node::Class(Box::new(Class { negated: c == 'P', items: vec![Item::Pred(pred)], intersections: Vec::new() }), Fold::Exact)
            }
            '1'..='9' => {
                let mut n = c.to_digit(10).unwrap() as usize;
                while let Some(d) = self.peek().and_then(|d| d.to_digit(10)) {
                    if n * 10 + d as usize > self.groups {
                        break;
                    }
                    n = n * 10 + d as usize;
                    self.at += 1;
                }
                Node::Backref(n, flags.fold())
            }
            c => Node::Char(self.escaped_char(c)?, flags.fold()),
        })
    }

    fn parse_property(&mut self, flags: &Flags) -> Parsed<Pred> {
        let name: String = if self.eat('{') {
            let mut name = String::new();
            while let Some(c) = self.next() {
                if c == '}' {
                    break;
                }
                name.push(c);
            }
            name
        } else {
            match self.next() {
                Some(c) => c.to_string(),
                None => return self.error("Unknown character property"),
            }
        };
        // The POSIX names (`Lower`, not `IsLowercase` or `Ll`) are ASCII classes, as the JDK's
        // `CharPredicates.forPOSIXName`.
        if !flags.unicode_class {
            let posix = match name.as_str() {
                "Lower" | "Upper" if flags.ci => Some(Pred::AsciiCased),
                "Lower" => Some(Pred::AsciiLower),
                "Upper" => Some(Pred::AsciiUpper),
                "Alpha" => Some(Pred::AsciiAlpha),
                "Digit" => Some(Pred::Digit),
                "Alnum" => Some(Pred::AsciiAlnum),
                "Cntrl" => Some(Pred::AsciiCntrl),
                "Graph" => Some(Pred::AsciiGraph),
                "Print" => Some(Pred::AsciiPrint),
                _ => None,
            };
            if let Some(p) = posix {
                return Ok(p);
            }
        }
        // Unicode's otherwise (`forUnicodeProperty`, `getPosixPredicate`, `forProperty`): under
        // `CASE_INSENSITIVE` lowercase, uppercase and titlecase each take all three.
        let name = name.strip_prefix("Is").unwrap_or(&name);
        let cased = matches!(name, "Lu" | "Upper" | "Uppercase" | "javaUpperCase" | "Ll" | "Lower" | "Lowercase" | "javaLowerCase" | "Lt" | "Titlecase" | "javaTitleCase");
        if cased && flags.ci {
            return Ok(Pred::Cased);
        }
        Ok(match name {
            "L" | "Letter" | "Alpha" | "Alphabetic" | "javaLetter" => Pred::Letter,
            "Lu" | "Upper" | "Uppercase" | "javaUpperCase" => Pred::Upper,
            "Ll" | "Lower" | "Lowercase" | "javaLowerCase" => Pred::Lower,
            "Lt" | "Titlecase" | "javaTitleCase" => Pred::Title,
            "Nd" | "Digit" | "N" => Pred::Numeric,
            "Alnum" | "javaLetterOrDigit" => Pred::Alnum,
            "Punct" | "P" => Pred::Punct,
            "Space" | "White_Space" | "javaWhitespace" | "Zs" => Pred::Space,
            "Cntrl" | "Cc" => Pred::Cntrl,
            "XDigit" => Pred::XDigit,
            "Blank" => Pred::Blank,
            "Print" | "Graph" => Pred::Print,
            "ASCII" => Pred::Ascii,
            "Any" | "all" => Pred::Any,
            other => return self.error(&format!("Unknown character property name {{{}}}", other)),
        })
    }

    fn escaped_char(&mut self, c: char) -> Parsed<char> {
        Ok(match c {
            't' => '\t',
            'n' => '\n',
            'r' => '\r',
            'f' => '\u{c}',
            'a' => '\u{7}',
            'e' => '\u{1b}',
            '0' => {
                let mut n = 0u32;
                let mut count = 0;
                while count < 3 && self.peek().map_or(false, |d| ('0'..='7').contains(&d)) {
                    n = n * 8 + self.next().unwrap().to_digit(8).unwrap();
                    count += 1;
                }
                char::from_u32(n).unwrap_or('\0')
            }
            'x' => {
                let hex: String = if self.eat('{') {
                    let mut h = String::new();
                    while let Some(c) = self.next() {
                        if c == '}' {
                            break;
                        }
                        h.push(c);
                    }
                    h
                } else {
                    let h: String = self.chars[self.at..(self.at + 2).min(self.chars.len())].iter().collect();
                    self.at += h.len();
                    h
                };
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(c) => c,
                    None => return self.error("Illegal hexadecimal escape sequence"),
                }
            }
            'u' => {
                let h: String = self.chars[self.at..(self.at + 4).min(self.chars.len())].iter().collect();
                self.at += h.len();
                let mut code = match u32::from_str_radix(&h, 16) {
                    Ok(c) => c,
                    Err(_) => return self.error("Illegal Unicode escape sequence"),
                };
                // A surrogate pair written as two escapes is one character.
                if crate::text::pair_joins(code) && self.peek() == Some('\\') && self.peek_at(1) == Some('u') {
                    let low: String = self.chars[self.at + 2..(self.at + 6).min(self.chars.len())].iter().collect();
                    if let Ok(l) = u32::from_str_radix(&low, 16) {
                        if (0xdc00..0xe000).contains(&l) {
                            self.at += 6;
                            code = 0x10000 + ((code - 0xd800) << 10) + (l - 0xdc00);
                        }
                    }
                }
                match code {
                    0xD800..=0xDFFF => crate::text::lone_surrogate(code),
                    _ => char::from_u32(code).unwrap_or('\u{fffd}'),
                }
            }
            'c' => match self.next() {
                Some(x) => char::from_u32((x as u32) ^ 64).unwrap_or('\0'),
                None => return self.error("Illegal control escape sequence"),
            },
            c if c.is_ascii_alphanumeric() => return self.error(&format!("Illegal/unsupported escape sequence \\{}", c)),
            c => c,
        })
    }

    /// A class after its `[` (`consume` then takes the `]`), or the operand of an `&&` that no
    /// brackets delimit, as the JDK's `Pattern.clazz` reads it: `^` right after the bracket
    /// negates, a nested class joins the union, `&&` intersects the union so far with what
    /// follows up to `]` or the next `&&` (an empty operand repeats the last element), and a `]`
    /// that nothing precedes is itself.
    fn parse_class(&mut self, consume: bool, fold: Fold) -> Parsed<Class> {
        let negated = self.at > 0 && self.chars[self.at - 1] == '[' && self.eat('^');
        let mut prev: Option<Class> = None;
        let mut curr: Option<Class> = None;
        let mut bits: Vec<Item> = Vec::new();
        let mut has_bits = false;
        loop {
            match self.peek() {
                None => return self.error("Unclosed character class"),
                Some('[') => {
                    self.at += 1;
                    let nested = self.parse_class(true, fold)?;
                    curr = Some(nested.clone());
                    prev = Some(union(prev, nested));
                    continue;
                }
                Some('&') if self.peek_at(1) == Some('&') => {
                    self.at += 2;
                    let mut right: Option<Class> = None;
                    loop {
                        let operand = match self.peek() {
                            None => return self.error("Unclosed character class"),
                            Some(']') | Some('&') => break,
                            Some('[') => {
                                self.at += 1;
                                self.parse_class(true, fold)?
                            }
                            Some(_) => self.parse_class(false, fold)?,
                        };
                        right = Some(union(right, operand));
                    }
                    // The JDK puts its one set of the characters named alone into the union, so
                    // that those named after the `&&` join it there too (`fill_bits`).
                    if has_bits {
                        let taken = plain(vec![Item::Bits]);
                        prev = Some(match prev {
                            None => {
                                curr = Some(taken.clone());
                                taken
                            }
                            Some(p) => union(Some(p), taken),
                        });
                        has_bits = false;
                    }
                    let had_right = right.is_some();
                    if right.is_some() {
                        curr = right;
                    }
                    prev = Some(match (prev, &curr) {
                        (None, Some(c)) if had_right => c.clone(),
                        (None, _) => return self.error("Bad class syntax"),
                        (Some(_), None) => return self.error("Bad intersection syntax"),
                        (Some(p), Some(c)) => and(p, c.clone()),
                    });
                    continue;
                }
                Some(']') if prev.is_some() || has_bits => {
                    if consume {
                        self.at += 1;
                    }
                    let mut class = match prev {
                        None => plain(bits.clone()),
                        Some(p) if has_bits => union(Some(p), plain(bits.clone())),
                        Some(p) => p,
                    };
                    fill_bits(&mut class, &bits);
                    return Ok(if negated { negate(class) } else { class });
                }
                Some(_) => {}
            }
            match self.class_range(&mut bits, fold)? {
                None => {
                    has_bits = true;
                    curr = None;
                }
                Some(c) => {
                    curr = Some(c.clone());
                    prev = Some(union(prev, c));
                }
            }
        }
    }

    /// A character, a range of two, a predefined class or a property of a class, as the JDK's
    /// `Pattern.range`: a hyphen before `[` or `]` is itself, and `None` is a character put in
    /// `bits`, as the JDK puts one below U+0100.
    fn class_range(&mut self, bits: &mut Vec<Item>, fold: Fold) -> Parsed<Option<Class>> {
        let lo = if self.eat('\\') {
            if let Some(e @ ('p' | 'P')) = self.peek() {
                self.at += 1;
                let flags = self.class_flags;
                let p = self.parse_property(&flags)?;
                return Ok(Some(Class { negated: e == 'P', items: vec![Item::Pred(p)], intersections: Vec::new() }));
            }
            let is_range = self.peek_at(1) == Some('-');
            match self.class_escape(is_range)? {
                Ok(c) => c,
                Err(class) => return Ok(Some(class)),
            }
        } else {
            self.next().unwrap()
        };
        if self.peek() == Some('-') && !matches!(self.peek_at(1), Some('[' | ']')) {
            self.at += 1;
            let hi = match self.next() {
                Some('\\') => match self.class_escape(true)? {
                    Ok(c) => c,
                    Err(_) => return self.error("Illegal character range"),
                },
                Some(c) => c,
                None => return self.error("Illegal character range"),
            };
            if hi < lo {
                return self.error("Illegal character range");
            }
            return Ok(Some(plain(vec![Item::Range(lo, hi)])));
        }
        let folded_apart = fold == Fold::Unicode && matches!(lo, '\u{ff}' | '\u{b5}' | 'I' | 'i' | 'S' | 's' | 'K' | 'k' | '\u{c5}' | '\u{e5}');
        if (lo as u32) < 0x100 && !folded_apart {
            bits.push(Item::Range(lo, lo));
            return Ok(None);
        }
        Ok(Some(plain(vec![Item::Range(lo, lo)])))
    }

    /// An escape in a class after its backslash: the character it stands for, or a predefined
    /// class; `\v` is the vertical tab next to a range's hyphen, as the JDK reads it.
    fn class_escape(&mut self, is_range: bool) -> Parsed<Result<char, Class>> {
        let Some(e) = self.next() else { return self.error("Unexpected internal error") };
        let pred = match e {
            'd' => Pred::Digit,
            'D' => Pred::NotDigit,
            'w' => Pred::Word,
            'W' => Pred::NotWord,
            's' => Pred::Space,
            'S' => Pred::NotSpace,
            'h' => Pred::HSpace,
            'H' => Pred::NotHSpace,
            'v' if is_range => return Ok(Ok('\u{b}')),
            'v' => Pred::VSpace,
            'V' => Pred::NotVSpace,
            c => return self.escaped_char(c).map(Ok),
        };
        Ok(Err(plain(vec![Item::Pred(pred)])))
    }
}

fn fill_bits(class: &mut Class, bits: &[Item]) {
    for item in class.items.iter_mut() {
        match item {
            Item::Bits => *item = Item::Sub(plain(bits.to_vec())),
            Item::Sub(sub) => fill_bits(sub, bits),
            _ => {}
        }
    }
    for sub in class.intersections.iter_mut() {
        fill_bits(sub, bits);
    }
}

fn plain(items: Vec<Item>) -> Class {
    Class { negated: false, items, intersections: Vec::new() }
}

/// What either class takes.
fn union(a: Option<Class>, b: Class) -> Class {
    match a {
        None => b,
        Some(mut a) if is_plain(&a) && is_plain(&b) => {
            a.items.extend(b.items);
            a
        }
        Some(a) => plain(vec![Item::Sub(a), Item::Sub(b)]),
    }
}

fn is_plain(c: &Class) -> bool {
    !c.negated && c.intersections.is_empty()
}

/// What both classes take.
fn and(a: Class, b: Class) -> Class {
    let mut a = if a.negated { plain(vec![Item::Sub(a)]) } else { a };
    a.intersections.push(b);
    a
}

fn negate(a: Class) -> Class {
    let mut a = if a.negated { plain(vec![Item::Sub(a)]) } else { a };
    a.negated = true;
    a
}

/// `\Q...\E` written out as the characters it quotes, as the JDK's `Pattern.RemoveQEQuoting` does
/// before it parses: inside a block an ASCII character that is neither a letter nor a digit gets
/// a backslash, a digit that opens the block is `\x3N`, so that it cannot extend a back reference
/// before it, and an empty block leaves nothing. The parser never sees a block.
fn remove_qe_quoting(src: Vec<char>) -> Vec<char> {
    let n = src.len();
    let mut i = 0;
    while i + 1 < n {
        if src[i] != '\\' {
            i += 1;
        } else if src[i + 1] != 'Q' {
            i += 2;
        } else {
            break;
        }
    }
    if i + 1 >= n {
        return src;
    }
    let mut out = src[..i].to_vec();
    i += 2;
    let (mut in_quote, mut begin_quote) = (true, true);
    while i < n {
        let c = src[i];
        i += 1;
        if !c.is_ascii() || c.is_ascii_alphabetic() {
            out.push(c);
        } else if c.is_ascii_digit() {
            if begin_quote {
                out.extend(['\\', 'x', '3']);
            }
            out.push(c);
        } else if c != '\\' {
            if in_quote {
                out.push('\\');
            }
            out.push(c);
        } else if in_quote {
            if src.get(i) == Some(&'E') {
                i += 1;
                in_quote = false;
            } else {
                out.extend(['\\', '\\']);
            }
        } else if src.get(i) == Some(&'Q') {
            i += 1;
            in_quote = true;
            begin_quote = true;
            continue;
        } else {
            out.push(c);
            if i < n {
                out.push(src[i]);
                i += 1;
            }
        }
        begin_quote = false;
    }
    out
}

const STEP_LIMIT: u64 = 50_000_000;

struct Matcher<'r, 's> {
    re: &'r Regex,
    chars: &'s [char],
    caps: Vec<Option<(usize, usize)>>,
    steps: u64,
    overflow: bool,
}

impl<'r, 's> Matcher<'r, 's> {
    fn at_line_end(&self, i: usize, multiline: bool) -> bool {
        let n = self.chars.len();
        if i == n {
            return true;
        }
        if multiline {
            return is_line_terminator(self.chars[i]) && !(self.chars[i] == '\n' && i > 0 && self.chars[i - 1] == '\r');
        }
        (i == n - 1 && is_line_terminator(self.chars[i])) || (i == n - 2 && self.chars[i] == '\r' && self.chars[i + 1] == '\n')
    }

    fn at_line_start(&self, i: usize, multiline: bool) -> bool {
        if i == 0 {
            return true;
        }
        multiline && i < self.chars.len() && is_line_terminator(self.chars[i - 1]) && !(self.chars[i - 1] == '\r' && self.chars[i] == '\n')
    }

    fn word_boundary(&self, i: usize) -> bool {
        let before = match i {
            0 => false,
            1 => is_word_char(self.chars[0]),
            _ if width(self.chars[i - 2]) == 2 => is_word_char(self.chars[i - 2]),
            _ => is_word_char(self.chars[i - 1]),
        };
        let after = i < self.chars.len() && is_word_char(self.chars[i]);
        before != after
    }

    fn m(&mut self, node: &Node, i: usize, k: &mut dyn FnMut(&mut Matcher<'r, 's>, usize) -> bool) -> bool {
        self.steps += 1;
        if self.steps > STEP_LIMIT {
            self.overflow = true;
            return false;
        }
        let n = self.chars.len();
        match node {
            Node::Empty => k(self, i),
            Node::Char(c, fold) => i < n && (self.chars[i] == *c || eq_ci(self.chars[i], *c, *fold)) && k(self, i + width(self.chars[i])),
            Node::Any(dotall) => i < n && (*dotall || !is_line_terminator(self.chars[i])) && k(self, i + width(self.chars[i])),
            Node::Class(class, fold) => i < n && class.matches(self.chars[i], *fold) && k(self, i + width(self.chars[i])),
            Node::LineStart(ml) => self.at_line_start(i, *ml) && k(self, i),
            Node::LineEnd(ml) => self.at_line_end(i, *ml) && k(self, i),
            Node::InputStart => i == 0 && k(self, i),
            Node::InputEnd => i == n && k(self, i),
            Node::InputEndNl => self.at_line_end(i, false) && k(self, i),
            Node::WordBoundary(negated) => (self.word_boundary(i) != *negated) && k(self, i),
            Node::Group(None, inner) => self.m(inner, i, k),
            Node::Atomic(inner) => {
                let saved = self.caps.clone();
                let mut end = None;
                let matched = self.m(inner, i, &mut |_, j| {
                    end = Some(j);
                    true
                });
                match (matched, end) {
                    (true, Some(j)) if k(self, j) => true,
                    _ => {
                        self.caps = saved;
                        false
                    }
                }
            }
            Node::Group(Some(idx), inner) => {
                let idx = *idx;
                let saved = self.caps[idx];
                let matched = self.m(inner, i, &mut |s, j| {
                    let old = s.caps[idx];
                    s.caps[idx] = Some((i, j));
                    if k(s, j) {
                        true
                    } else {
                        s.caps[idx] = old;
                        false
                    }
                });
                if !matched {
                    self.caps[idx] = saved;
                }
                matched
            }
            Node::Concat(items) => self.concat(items, 0, i, k),
            Node::Alt(alts) => {
                for a in alts {
                    if self.m(a, i, k) {
                        return true;
                    }
                    if self.overflow {
                        return false;
                    }
                }
                false
            }
            Node::Repeat(inner, min, max, greed) => self.rep(inner, *min, *max, *greed, i, 0, k),
            Node::Backref(idx, fold) => {
                let Some(Some((a, b))) = self.caps.get(*idx).copied() else { return false };
                let len = b - a;
                if i + len > n {
                    return false;
                }
                // Unit by unit, a supplementary character on both sides as the one character it is.
                let mut d = 0;
                while d < len {
                    let (x, y) = (self.chars[a + d], self.chars[i + d]);
                    let step = if width(x) == 2 && width(y) == 2 && d + 2 <= len { 2 } else { 1 };
                    let same = if step == 2 { x == y || eq_ci(x, y, *fold) } else { unit_at(x) == unit_at(y) || eq_ci(x, y, *fold) };
                    if !same {
                        return false;
                    }
                    d += step;
                }
                k(self, i + len)
            }
            Node::Look { ahead, negative, node, span: (least, most) } => {
                let saved = self.caps.clone();
                let matched = if *ahead {
                    self.m(node, i, &mut |_, _| true)
                } else {
                    let mut found = false;
                    // The starts the node can end at `i` from, the nearest first.
                    let from = most.map_or(0, |m| i.saturating_sub(m));
                    let starts = if i >= *least { from..=i - *least } else { 1..=0 };
                    for j in starts.rev() {
                        if self.m(node, j, &mut |_, e| e == i) {
                            found = true;
                            break;
                        }
                        if self.overflow {
                            return false;
                        }
                    }
                    found
                };
                if *negative {
                    self.caps = saved;
                    !matched && k(self, i)
                } else if matched {
                    if k(self, i) {
                        true
                    } else {
                        self.caps = saved;
                        false
                    }
                } else {
                    self.caps = saved;
                    false
                }
            }
        }
    }

    fn concat(&mut self, items: &[Node], idx: usize, i: usize, k: &mut dyn FnMut(&mut Matcher<'r, 's>, usize) -> bool) -> bool {
        if idx == items.len() {
            return k(self, i);
        }
        self.m(&items[idx], i, &mut |s, j| s.concat(items, idx + 1, j, k))
    }

    fn rep(&mut self, inner: &Node, min: u32, max: Option<u32>, greed: Greed, i: usize, count: u32, k: &mut dyn FnMut(&mut Matcher<'r, 's>, usize) -> bool) -> bool {
        let can_more = max.map_or(true, |m| count < m);
        match greed {
            Greed::Greedy => {
                if can_more {
                    let more = self.m(inner, i, &mut |s, j| {
                        if j == i && count >= min {
                            return false;
                        }
                        s.rep(inner, min, max, greed, j, count + 1, k)
                    });
                    if more || self.overflow {
                        return more;
                    }
                }
                count >= min && k(self, i)
            }
            Greed::Lazy => {
                if count >= min && k(self, i) {
                    return true;
                }
                if self.overflow || !can_more {
                    return false;
                }
                self.m(inner, i, &mut |s, j| {
                    if j == i && count >= min {
                        return false;
                    }
                    s.rep(inner, min, max, greed, j, count + 1, k)
                })
            }
            Greed::Possessive => {
                let mut pos = i;
                let mut n = count;
                while max.map_or(true, |m| n < m) {
                    let mut end = None;
                    let matched = self.m(inner, pos, &mut |_, j| {
                        end = Some(j);
                        true
                    });
                    match (matched, end) {
                        (true, Some(j)) if j != pos => {
                            pos = j;
                            n += 1;
                        }
                        _ => break,
                    }
                }
                n >= min && k(self, pos)
            }
        }
    }
}

pub struct Found {
    /// Per group the range of UTF-16 indices it took, `None` for one that took no part; group 0
    /// is the whole match.
    pub groups: Vec<Option<(usize, usize)>>,
}

impl Regex {
    pub fn compile(src: &str) -> Result<Regex, String> {
        let mut flags = Flags { ci: false, unicode_case: false, unicode_class: false, dotall: false, multiline: false, comments: false };
        let mut p = Parser { chars: remove_qe_quoting(src.chars().collect()), at: 0, groups: 0, names: Vec::new(), src, class_flags: flags };
        let root = p.parse_alt(&mut flags)?;
        if p.at < p.chars.len() {
            return p.error("Unmatched closing ')'");
        }
        let supplementary = names_supplementary(&root);
        Ok(Regex { root, groups: p.groups, names: p.names, supplementary })
    }

    /// The first match at or after `start`; anchored at `start` when `anchored`, and required to
    /// reach the end of the input when `full`.
    pub fn find(&self, chars: &[char], start: usize, anchored: bool, full: bool) -> Result<Option<Found>, String> {
        let mut m = Matcher { re: self, chars, caps: vec![None; self.groups + 1], steps: 0, overflow: false };
        let last = if anchored { start } else { chars.len() };
        let mut at = start;
        loop {
            if at > chars.len() {
                break;
            }
            let mut end = None;
            let n = chars.len();
            let matched = m.m(&m.re.root, at, &mut |_, j| {
                if full && j != n {
                    return false;
                }
                end = Some(j);
                true
            });
            if m.overflow {
                return Err("the regular expression took too many steps".to_string());
            }
            if let (true, Some(j)) = (matched, end) {
                let mut groups = m.caps.clone();
                groups[0] = Some((at, j));
                return Ok(Some(Found { groups }));
            }
            if at >= last {
                break;
            }
            at += if self.supplementary { width(chars[at]) } else { 1 };
        }
        Ok(None)
    }

    pub fn find_all(&self, chars: &[char]) -> Result<Vec<Found>, String> {
        let mut out = Vec::new();
        let mut at = 0;
        while at <= chars.len() {
            let Some(found) = self.find(chars, at, false, false)? else { break };
            let (a, b) = found.groups[0].unwrap();
            out.push(found);
            at = if b == a { b + 1 } else { b };
        }
        Ok(out)
    }
}
