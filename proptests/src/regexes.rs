//! Generated regular expressions for the property that the targets agree (`exprs::E::Regex`):
//! patterns in Java's syntax over what JavaScript's translator reads as JDK 24's `Pattern` reads
//! it (class intersections and nested classes, comments mode, properties under `(?i)`) among
//! groups, alternation, quantifiers and back references, applied to short inputs by extraction,
//! positions, groups, splitting, whole-input matching and replacement. A pattern the JDK refuses
//! prints `rejected` on every target.
//!
//! Left out, as known differences from the JVM: a pattern that can match nothing (a
//! zero-width match next to a supplementary character steps past the pair on JavaScript), `(?iu)`
//! (the engine's folding), flags other than `x` past the head, a back reference under `(?i)`, a
//! back reference anywhere but right after its group (the engine matches one to a group that
//! took no part empty), a group that a quantifier repeats around a capturing group (the engine
//! clears it at each repetition), lookbehinds (one before a supplementary character written as
//! an escape), POSIX classes and the `Is`, `In` and `java` names of properties. The interpreter's disagreements of
//! `known` are kept out by their triggers: `regex-comments` (white space or a comment inside a
//! class, a comment that ends at other than `\n`, white space where the JDK's parser reads past
//! it) and `regex-properties` (`Lt`, a `gc=` value, `Lu` or `Ll` under `(?i)`, and a property
//! over the characters where Rust's properties and the JDK's categories part).

use crate::exprs::{Case, E, T};
use hegel::generators as gs;
use hegel::TestCase;

fn index(tc: &TestCase, count: usize) -> usize {
    tc.draw_silent(gs::integers::<usize>().min_value(0).max_value(count - 1))
}

fn chance(tc: &TestCase, p: f64) -> bool {
    tc.draw_silent(gs::weighted_booleans(p))
}

fn pick<'a>(tc: &TestCase, from: &[&'a str]) -> &'a str {
    from[index(tc, from.len())]
}

/// Characters of a class, as the pattern writes them.
const CLASS_CHARS: [&str; 46] = [
    "a", "b", "c", "e", "k", "s", "x", "z", "A", "B", "E", "K", "S", "Z", "0", "5", "9", "_", "\u{e9}", "\u{c9}", "\u{131}", "\u{17f}", "\u{212a}",
    "\u{1d11e}", ".", "*", "+", "?", "(", ")", "{", "}", "|", "$", "/", "!", ",", ":", "<", "=", "@", "~", "\\-", "\\[", "\\]", "\\^",
];
const CLASS_ESCAPES: [&str; 10] = ["\\&", "\\\\", "\\.", "\\x41", "\\u00e9", "\\t", "\\x{1D11E}", "\\0101", "\\cA", "\\ "];
/// Characters outside a class, as the pattern writes them.
const CHARS: [&str; 34] = [
    "a", "b", "c", "e", "k", "s", "x", "z", "A", "E", "K", "S", "Z", "0", "5", "_", "-", "&", ",", "\u{e9}", "\u{131}", "\u{17f}", "\u{212a}",
    "\u{1d11e}", "\\.", "\\*", "\\(", "\\[", "\\{", "\\|", "\\$", "\\^", "\\\\", "\\-",
];
const ESCAPES: [&str; 6] = ["\\x41", "\\u0065", "\\t", "\\#", "\\ ", "\\x{1D11E}"];
const RANGES: [(&str, &str); 14] = [
    ("a", "e"), ("a", "z"), ("A", "Z"), ("0", "9"), ("b", "y"), ("e", "k"), ("!", "/"), ("0", "z"), ("\u{e9}", "\u{17f}"), ("a", "\u{1d11e}"),
    ("K", "k"), ("z", "a"), ("\\x41", "\\x5a"), ("a", "\\u0065"),
];
const PREDEFINED: [&str; 10] = ["\\d", "\\D", "\\w", "\\W", "\\s", "\\S", "\\h", "\\H", "\\v", "\\V"];
const PROPERTIES: [&str; 6] = ["\\p{L}", "\\P{L}", "\\pL", "\\p{N}", "\\p{Nd}", "\\P{Nd}"];
const CASED: [&str; 3] = ["\\p{Lu}", "\\p{Ll}", "\\P{Lu}"];
const CASED_ASKED: [&str; 4] = ["\\p{Lt}", "\\P{Lt}", "\\p{gc=Lu}", "\\p{General_Category=Ll}"];
/// White space and comments where both parsers pass over them in comments mode, and where the
/// interpreter does not (`regex-comments`).
const SPACES: [&str; 6] = [" ", "  ", "\n", "\t", " # note\n", "#\n"];
const SPACES_ASKED: [&str; 4] = [" # note\r", "#x\u{85}", " #x\u{2028}", "#\u{2029}"];
const INPUT: [&str; 36] = [
    "a", "b", "c", "e", "k", "s", "x", "z", "A", "E", "K", "S", "Z", "0", "5", "9", "_", "-", "&", "^", "]", "[", " ", ".", "#", "\t", "\n",
    "\u{e9}", "\u{c9}", "\u{131}", "\u{17f}", "\u{212a}", "\u{1d11e}", "\u{2028}", "\u{1c5}", "\u{3a9}",
];
/// Characters where the interpreter's properties, Rust's Alphabetic, Uppercase, Lowercase and
/// Numeric, part from the JDK's categories.
const INPUT_PROPERTIES: [&str; 5] = ["\u{24b6}", "\u{2170}", "\u{aa}", "\u{2b0}", "\u{b2}"];

struct Writer<'a> {
    tc: &'a TestCase,
    x: bool,
    fold: bool,
    groups: usize,
    property: bool,
    comments: bool,
    properties: bool,
}

impl Writer<'_> {
    fn space(&self) -> String {
        match self.comments && chance(self.tc, 0.3) {
            true => pick(self.tc, &SPACES_ASKED).to_string(),
            false => pick(self.tc, &SPACES).to_string(),
        }
    }

    fn property(&mut self) -> String {
        self.property = true;
        let cased = !self.fold || self.properties;
        match index(self.tc, 4) {
            0 if cased => pick(self.tc, &CASED).to_string(),
            1 if self.properties => pick(self.tc, &CASED_ASKED).to_string(),
            _ => pick(self.tc, &PROPERTIES).to_string(),
        }
    }

    fn class_char(&self) -> String {
        match index(self.tc, 6) {
            0 => pick(self.tc, &CLASS_ESCAPES).to_string(),
            // In comments mode a space or a `#` in a class is white space or a comment.
            1 if !self.x || self.comments => pick(self.tc, &[" ", "#"]).to_string(),
            _ => pick(self.tc, &CLASS_CHARS).to_string(),
        }
    }

    fn range(&self) -> String {
        let (lo, hi) = RANGES[index(self.tc, RANGES.len())];
        format!("{lo}-{hi}")
    }

    fn class(&mut self, depth: usize) -> String {
        let mut s = String::from("[");
        if chance(self.tc, 0.3) {
            s.push('^');
        }
        if chance(self.tc, 0.08) {
            s.push(']');
        }
        for k in 0..1 + index(self.tc, 4) {
            if k > 0 && self.x && self.comments && chance(self.tc, 0.4) {
                s.push_str(pick(self.tc, &[" ", "\t", "#c\n", " & "]));
            }
            let part = match index(self.tc, 12) {
                0..=3 => self.class_char(),
                4 | 5 => self.range(),
                6 => pick(self.tc, &PREDEFINED).to_string(),
                7 => self.property(),
                8 if depth > 0 => self.class(depth - 1),
                9 => "&&".to_string(),
                10 if depth > 0 => format!("&&{}", self.class(depth - 1)),
                10 => format!("&&{}", self.range()),
                _ => pick(self.tc, &["-", "&", "^", "&&&", "-a", "a-"]).to_string(),
            };
            s.push_str(&part);
        }
        s.push(']');
        s
    }

    fn quantifier(&self, first: bool, captures: bool) -> &'static str {
        match (first, captures) {
            (true, _) => pick(self.tc, &["", "", "+", "{1,2}", "{2}", "+?"]),
            (false, true) => pick(self.tc, &["", "", "?", "??"]),
            (false, false) => pick(self.tc, &["", "", "", "+", "*", "?", "{1,2}", "{0,2}", "{2}", "*?", "??"]),
        }
    }

    /// An atom and its quantifier; the first of a sequence takes a character at least.
    fn atom(&mut self, depth: usize, first: bool) -> String {
        let choice = match first {
            true => index(self.tc, 5),
            false => index(self.tc, 10),
        };
        let groups = self.groups;
        let atom = match choice {
            0 | 1 => self.class(2),
            2 => match index(self.tc, 4) {
                0 => pick(self.tc, &ESCAPES).to_string(),
                _ => pick(self.tc, &CHARS).to_string(),
            },
            3 => pick(self.tc, &PREDEFINED).to_string(),
            4 => self.property(),
            5 if depth > 0 => {
                self.groups += 1;
                format!("({})", self.group_body(depth - 1))
            }
            6 if depth > 0 => format!("(?:{})", self.group_body(depth - 1)),
            7 if depth > 0 => {
                let outer = self.x;
                let (open, x) = [("(?x:", true), ("(?-x:", false), ("(?x)", true)][index(self.tc, 3)];
                self.x = x;
                let s = match open {
                    "(?x)" => format!("(?:{open}{})", self.group_body(depth - 1)),
                    _ => format!("{open}{})", self.group_body(depth - 1)),
                };
                self.x = outer;
                s
            }
            8 if depth > 0 && !self.fold => {
                self.groups += 1;
                let n = self.groups;
                format!("({}){}\\{n}", self.group_body(depth - 1), if self.x { self.space() } else { String::new() })
            }
            _ => pick(self.tc, &CHARS).to_string(),
        };
        // The back reference after its group (8) takes the quantifier, not the group.
        let quantifier = self.quantifier(first, choice != 8 && self.groups > groups);
        match self.x && self.comments && !quantifier.is_empty() && chance(self.tc, 0.2) {
            // The JDK reads past white space before a quantifier and inside its braces.
            true => format!("{atom} {}", quantifier.replace(',', ", ")),
            false => format!("{atom}{quantifier}"),
        }
    }

    fn sequence(&mut self, depth: usize) -> String {
        let mut s = String::new();
        for k in 0..1 + index(self.tc, 3) {
            if self.x && chance(self.tc, 0.5) {
                s.push_str(&self.space());
            }
            s.push_str(&self.atom(depth, k == 0));
        }
        s
    }

    fn group_body(&mut self, depth: usize) -> String {
        let mut s = self.sequence(depth);
        if chance(self.tc, 0.2) {
            s.push('|');
            s.push_str(&self.sequence(depth));
        }
        s
    }
}

/// A pattern, and whether it names a property.
fn pattern(tc: &TestCase) -> (String, bool) {
    let head = pick(tc, &["", "", "", "(?i)", "(?x)", "(?ix)", "(?x)(?i)", "(?xi)"]);
    let mut w = Writer {
        tc,
        x: head.contains('x'),
        fold: head.contains('i'),
        groups: 0,
        property: false,
        comments: crate::known::asked("regex-comments"),
        properties: crate::known::asked("regex-properties"),
    };
    let body = w.group_body(2);
    (format!("{head}{body}"), w.property)
}

/// A Scala string literal of the text: ASCII as it is, the rest as `\u` escapes.
fn literal(text: &str) -> String {
    let mut s = String::from("\"");
    for c in text.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '"' => s.push_str("\\\""),
            '\n' => s.push_str("\\n"),
            '\t' => s.push_str("\\t"),
            '\r' => s.push_str("\\r"),
            ' '..='~' => s.push(c),
            _ => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    s.push_str(&format!("\\u{:04x}", unit));
                }
            }
        }
    }
    s.push('"');
    s
}

/// What `E::Regex`'s operation `op` does with the pattern `p` and the input `s`, as Scala.
pub fn operation(op: u8, p: &str, s: &str) -> String {
    let body = match op {
        0 => format!("{p}.r.findAllIn({s}).mkString(\"|\")"),
        1 => format!("{p}.r.findAllMatchIn({s}).map(m => m.start.toString + \"-\" + m.end.toString).mkString(\",\")"),
        2 => format!("{p}.r.replaceAllIn({s}, \"<$0>\")"),
        3 => format!("{p}.r.split({s}).mkString(\"|\")"),
        4 => format!("{s}.matches({p}).toString"),
        5 => format!("{p}.r.replaceAllIn({s}, (m: scala.util.matching.Regex.Match) => m.matched.length.toString)"),
        6 => format!("{p}.r.replaceFirstIn({s}, \"[$0]\")"),
        _ => format!("{p}.r.findFirstMatchIn({s}).map(m => (0 to m.groupCount).map(g => m.group(g)).mkString(\"/\")).getOrElse(\"none\")"),
    };
    format!("(try {body} catch {{ case _: IllegalArgumentException => \"rejected\" }})")
}

pub fn case(tc: &TestCase) -> Case {
    let (pattern, property) = pattern(tc);
    let excluded = property && !crate::known::asked("regex-properties");
    let mut input = String::new();
    for _ in 0..index(tc, 7) {
        let c = match !excluded && chance(tc, 0.1) {
            true => pick(tc, &INPUT_PROPERTIES),
            false => pick(tc, &INPUT),
        };
        input.push_str(c);
    }
    let leaf = |text: &str| E::Leaf(T::Str, literal(text), index(tc, 3) as u8, index(tc, 2) as u8);
    let op = index(tc, 8) as u8;
    let e = E::Regex(op, Box::new(leaf(&pattern)), Box::new(leaf(&input)));
    Case { of: T::Str, e }
}
