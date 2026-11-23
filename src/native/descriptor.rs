//! A library's `scala-native.properties` read as Scala Native 0.5.12's `build.Descriptor.load` reads it: a
//! `FileReader` over the file, `java.util.Properties.load` over that reader, then its own reading of each property
//! (`parseStrings`, the `toBoolean` of `project.gcProject`). The decoder is the JVM's default charset since JDK 18,
//! UTF-8, as `sun.nio.cs.UTF_8` decodes it with every malformed sequence replaced; the reader is `Properties`'
//! `LineReader`, `load0` and `loadConvert` (JDK 24's java/util/Properties.java), ported as written. Its strings are
//! Java's, UTF-16 code units. tests/native-build/descriptors.expected holds what the JVM answers for each descriptor
//! of tests/native-build/descriptor-features.txt (docs/NATIVE.md, "The toolchain").

use crate::intern::FxMap;

/// A Java string: UTF-16 code units.
pub type JString = Vec<u16>;

const REPLACEMENT: u16 = 0xFFFD;

/// `sun.nio.cs.UTF_8`'s decoder over a whole file as `StreamDecoder` drives it with `CodingErrorAction.REPLACE`:
/// a malformed sequence, of the length `decodeArrayLoop` and `malformedN` give it, is one U+FFFD; what is left at
/// the end of the input of a sequence cut short is one U+FFFD (`CharsetDecoder.decode` at the end of its input).
pub fn decode_utf8(src: &[u8]) -> JString {
    let continues = |b: u8| b & 0xc0 == 0x80;
    let mut out = Vec::with_capacity(src.len());
    let mut sp = 0;
    while sp < src.len() {
        let remaining = src.len() - sp;
        let b1 = src[sp];
        // The bytes a step takes: decoded, or replaced as one malformed sequence, or the rest of the input cut short.
        let malformed = if b1 < 0x80 {
            out.push(b1 as u16);
            sp += 1;
            continue;
        } else if b1 >> 5 == 0b110 && b1 & 0x1e != 0 {
            if remaining < 2 {
                remaining
            } else if !continues(src[sp + 1]) {
                1
            } else {
                out.push(((b1 as u16 & 0x1f) << 6) | (src[sp + 1] as u16 & 0x3f));
                sp += 2;
                continue;
            }
        } else if b1 >> 4 == 0b1110 {
            let overlong = |b2: u8| b1 == 0xe0 && b2 & 0xe0 == 0x80;
            if remaining < 3 {
                if remaining > 1 && (overlong(src[sp + 1]) || !continues(src[sp + 1])) { 1 } else { remaining }
            } else {
                let (b2, b3) = (src[sp + 1], src[sp + 2]);
                if overlong(b2) || !continues(b2) || !continues(b3) {
                    if overlong(b2) || !continues(b2) { 1 } else { 2 }
                } else {
                    let c = ((b1 as u16 & 0x0f) << 12) | ((b2 as u16 & 0x3f) << 6) | (b3 as u16 & 0x3f);
                    if (0xd800..=0xdfff).contains(&c) {
                        3
                    } else {
                        out.push(c);
                        sp += 3;
                        continue;
                    }
                }
            }
        } else if b1 >> 3 == 0b11110 {
            let second_bad = |b2: u8| b1 > 0xf4 || b1 == 0xf0 && !(0x90..=0xbf).contains(&b2) || b1 == 0xf4 && b2 & 0xf0 != 0x80 || !continues(b2);
            if remaining < 4 {
                if b1 > 0xf4 || remaining > 1 && second_bad(src[sp + 1]) {
                    1
                } else if remaining > 2 && !continues(src[sp + 2]) {
                    2
                } else {
                    remaining
                }
            } else {
                let (b2, b3, b4) = (src[sp + 1], src[sp + 2], src[sp + 3]);
                let uc = ((b1 as u32 & 0x07) << 18) | ((b2 as u32 & 0x3f) << 12) | ((b3 as u32 & 0x3f) << 6) | (b4 as u32 & 0x3f);
                if !continues(b2) || !continues(b3) || !continues(b4) || !(0x10000..0x110000).contains(&uc) {
                    if second_bad(b2) {
                        1
                    } else if !continues(b3) {
                        2
                    } else {
                        3
                    }
                } else {
                    let v = uc - 0x10000;
                    out.extend([0xd800 | (v >> 10) as u16, 0xdc00 | (v & 0x3ff) as u16]);
                    sp += 4;
                    continue;
                }
            }
        } else {
            1
        };
        out.push(REPLACEMENT);
        sp += malformed;
    }
    out
}

fn is(c: u16, ascii: u8) -> bool {
    c == ascii as u16
}

fn is_blank(c: u16) -> bool {
    is(c, b' ') || is(c, b'\t') || is(c, 0x0c)
}

fn is_line_end(c: u16) -> bool {
    is(c, b'\r') || is(c, b'\n')
}

/// `Properties.LineReader` over a `Reader` that gives the whole file: the logical lines, comments and blank lines
/// skipped, each natural line's leading blanks dropped, a line ending in an odd run of backslashes continued.
struct LineReader<'a> {
    input: &'a [u16],
    off: usize,
    line: Vec<u16>,
}

impl LineReader<'_> {
    /// `readLine`: the length of the next logical line in `line`, or `None` (its -1) at the end.
    fn read_line(&mut self) -> Option<usize> {
        let limit = self.input.len();
        let mut skip_white_space = true;
        let mut appended_line_begin = false;
        let mut preceding_backslash = false;
        self.line.clear();
        loop {
            if self.off >= limit {
                if self.line.is_empty() {
                    return None;
                }
                return Some(self.ended(preceding_backslash));
            }
            let c = self.input[self.off];
            self.off += 1;
            if skip_white_space {
                if is_blank(c) || !appended_line_begin && is_line_end(c) {
                    continue;
                }
                skip_white_space = false;
                appended_line_begin = false;
            }
            if self.line.is_empty() && (is(c, b'#') || is(c, b'!')) {
                loop {
                    let Some(&c) = self.input.get(self.off) else { return None };
                    self.off += 1;
                    if is_line_end(c) {
                        break;
                    }
                }
                skip_white_space = true;
                continue;
            }
            if !is_line_end(c) {
                self.line.push(c);
                preceding_backslash = is(c, b'\\') && !preceding_backslash;
                continue;
            }
            if self.line.is_empty() {
                skip_white_space = true;
                continue;
            }
            if self.off >= limit {
                return Some(self.ended(preceding_backslash));
            }
            if !preceding_backslash {
                return Some(self.line.len());
            }
            // The backslash at the line's end is not of the line, whose next natural line continues it.
            self.line.pop();
            skip_white_space = true;
            appended_line_begin = true;
            preceding_backslash = false;
            if is(c, b'\r') && is(self.input[self.off], b'\n') {
                self.off += 1;
            }
        }
    }

    /// The line at the input's end, without a backslash left continuing it.
    fn ended(&mut self, preceding_backslash: bool) -> usize {
        if preceding_backslash {
            self.line.pop();
        }
        self.line.len()
    }
}

/// What `Properties.load` throws, as its message says it.
const MALFORMED: &str = "Malformed \\uxxxx encoding.";

/// `loadConvert`: `\uXXXX` as that code unit, `\t`, `\r`, `\n` and `\f` as those characters, a backslash before any
/// other character dropped.
fn load_convert(input: &[u16]) -> Result<JString, String> {
    let mut out = Vec::with_capacity(input.len());
    let mut off = 0;
    while off < input.len() {
        let c = input[off];
        off += 1;
        if !is(c, b'\\') {
            out.push(c);
            continue;
        }
        // The line reader leaves no line ending in a backslash that escapes nothing.
        let escaped = input[off];
        off += 1;
        if is(escaped, b'u') {
            if off + 4 > input.len() {
                return Err(MALFORMED.to_string());
            }
            let mut value = 0u16;
            for &digit in &input[off..off + 4] {
                let digit = char::from_u32(digit as u32).and_then(|d| d.to_digit(16)).ok_or_else(|| MALFORMED.to_string())?;
                value = (value << 4) + digit as u16;
            }
            off += 4;
            out.push(value);
        } else {
            out.push(match u8::try_from(escaped) {
                Ok(b't') => b'\t' as u16,
                Ok(b'r') => b'\r' as u16,
                Ok(b'n') => b'\n' as u16,
                Ok(b'f') => 0x0c,
                _ => escaped,
            });
        }
    }
    Ok(out)
}

/// `Properties.load(Reader)` over a decoded file: its keys and values, the last of a key's lines winning; or what it
/// throws.
pub fn load(chars: &[u16]) -> Result<FxMap<JString, JString>, String> {
    let mut properties = FxMap::default();
    let mut reader = LineReader { input: chars, off: 0, line: Vec::new() };
    while let Some(limit) = reader.read_line() {
        let line = &reader.line[..limit];
        let mut key_len = 0;
        let mut value_start = limit;
        let mut has_separator = false;
        let mut preceding_backslash = false;
        while key_len < limit {
            let c = line[key_len];
            if (is(c, b'=') || is(c, b':')) && !preceding_backslash {
                value_start = key_len + 1;
                has_separator = true;
                break;
            } else if is_blank(c) && !preceding_backslash {
                value_start = key_len + 1;
                break;
            }
            preceding_backslash = is(c, b'\\') && !preceding_backslash;
            key_len += 1;
        }
        while value_start < limit {
            let c = line[value_start];
            if !is_blank(c) {
                if !has_separator && (is(c, b'=') || is(c, b':')) {
                    has_separator = true;
                } else {
                    break;
                }
            }
            value_start += 1;
        }
        let key = load_convert(&line[..key_len])?;
        let value = load_convert(&line[value_start..limit])?;
        properties.insert(key, value);
    }
    Ok(properties)
}

/// `String.trim`: every code unit up to U+0020 off both ends.
fn java_trim(s: &[u16]) -> &[u16] {
    let start = s.iter().position(|&c| c > 0x20).unwrap_or(s.len());
    let end = s.iter().rposition(|&c| c > 0x20).map_or(start, |at| at + 1);
    &s[start..end]
}

/// `String.split` by one character: the pieces between its occurrences, the trailing empty ones dropped; the string
/// itself where the character is not in it.
fn java_split(s: &[u16], separator: u8) -> Vec<&[u16]> {
    if !s.iter().any(|&c| is(c, separator)) {
        return vec![s];
    }
    let mut pieces: Vec<&[u16]> = s.split(|&c| is(c, separator)).collect();
    while pieces.last().is_some_and(|piece| piece.is_empty()) {
        pieces.pop();
    }
    pieces
}

fn ascii(s: &str) -> JString {
    s.encode_utf16().collect()
}

/// What Scala Native's `Descriptor` holds of a descriptor that loads, in Java's strings.
#[derive(Debug, PartialEq)]
pub struct Read {
    /// `project.gcProject`, `"false"` where it is absent; `Err` with the value where `toBoolean` throws.
    pub gc_project: Result<bool, JString>,
    pub links: Vec<JString>,
    pub defines: Vec<JString>,
    pub includes: Vec<JString>,
    pub c_options: Vec<JString>,
    pub cpp_options: Vec<JString>,
}

/// Scala Native's reading of the properties: `parseStrings` (`value.split(',').map(_.trim())`, an absent property
/// no string) and `toBoolean`, which takes `true` and `false` in any case of their ASCII letters. The `toBoolean` of
/// Scala 2.13, which builds through mill and scala-cli run, also takes `ſ` (U+017F) for `s`, where that of 2.12, which
/// sbt's runs, throws: a value the two read apart is taken as neither.
pub fn read(properties: &FxMap<JString, JString>) -> Read {
    let strings = |key: &str| -> Vec<JString> {
        properties.get(&ascii(key)).map_or_else(Vec::new, |value| java_split(value, b',').into_iter().map(|piece| java_trim(piece).to_vec()).collect())
    };
    let gc = properties.get(&ascii("project.gcProject")).cloned().unwrap_or_else(|| ascii("false"));
    let lowered: Option<String> = String::from_utf16(&gc).ok().filter(|s| s.is_ascii()).map(|s| s.to_ascii_lowercase());
    let gc_project = match lowered.as_deref() {
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        _ => Err(gc),
    };
    Read {
        gc_project,
        links: strings("nir.link.names"),
        defines: strings("preprocessor.defines"),
        includes: strings("compile.include.paths"),
        c_options: strings("compile.c.options"),
        cpp_options: strings("compile.cpp.options"),
    }
}

/// `NativeLib.createPathString` for a path written `a/b` under a directory: each piece of its `split("/")` resolved in
/// turn, as `Path.resolve` resolves it (an empty piece is the directory itself); `None` where a piece holds NUL,
/// which `Path` refuses.
pub fn include_path(unix_path: &[u16], dir: &[u16]) -> Option<JString> {
    let mut path = dir.to_vec();
    for piece in java_split(unix_path, b'/') {
        if piece.contains(&0) {
            return None;
        }
        if !piece.is_empty() {
            path.push(b'/' as u16);
            path.extend_from_slice(piece);
        }
    }
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A string as the table writes it: in quotes, every code unit outside printable ASCII, a quote and a backslash
    /// as `\uXXXX`.
    fn quoted(s: &[u16]) -> String {
        let mut out = String::from("\"");
        for &c in s {
            if (0x20..0x7f).contains(&c) && !is(c, b'"') && !is(c, b'\\') {
                out.push(c as u8 as char);
            } else {
                out.push_str(&format!("\\u{:04X}", c));
            }
        }
        out.push('"');
        out
    }

    fn list(name: &str, items: &[JString]) -> String {
        if items.is_empty() {
            String::new()
        } else {
            format!(" {}=[{}]", name, items.iter().map(|s| quoted(s)).collect::<Vec<_>>().join(","))
        }
    }

    /// What teq reads of a descriptor's bytes, as tests/native-build/descriptor-oracle.scala writes the JVM's answer:
    /// the properties sorted by key, then of Scala Native's reading what is not its default (`gc` where it is not
    /// false, each list with a string in it), the include paths under `/code`; or what `Properties.load` throws.
    fn reading(bytes: &[u8]) -> String {
        let properties = match load(&decode_utf8(bytes)) {
            Ok(properties) => properties,
            Err(why) => return format!("error {}", quoted(&ascii(&why))),
        };
        let mut sorted: Vec<(&JString, &JString)> = properties.iter().collect();
        sorted.sort();
        let mut out = format!("{{{}}}", sorted.iter().map(|(k, v)| format!("{}:{}", quoted(k), quoted(v))).collect::<Vec<_>>().join(","));
        let read = read(&properties);
        match &read.gc_project {
            Ok(false) => {}
            Ok(true) => out.push_str(" gc=true"),
            Err(_) => out.push_str(" gc=error"),
        }
        out.push_str(&list("links", &read.links));
        out.push_str(&list("defines", &read.defines));
        out.push_str(&list("includes", &read.includes));
        let paths: Option<Vec<JString>> = read.includes.iter().map(|path| include_path(path, &ascii("/code"))).collect();
        match paths {
            Some(paths) => out.push_str(&list("paths", &paths)),
            None => out.push_str(" paths=error"),
        }
        out.push_str(&list("c", &read.c_options));
        out.push_str(&list("cpp", &read.cpp_options));
        out
    }

    /// A feature's bytes as the features file writes them: `\xHH` for a byte, `\\` for a backslash.
    fn unescaped(text: &str) -> Vec<u8> {
        let bytes = text.as_bytes();
        let mut out = Vec::new();
        let mut at = 0;
        while at < bytes.len() {
            if bytes[at] == b'\\' && bytes.get(at + 1) == Some(&b'x') {
                out.push(u8::from_str_radix(&text[at + 2..at + 4], 16).expect("a \\xHH escape"));
                at += 4;
            } else if bytes[at] == b'\\' && bytes.get(at + 1) == Some(&b'\\') {
                out.push(b'\\');
                at += 2;
            } else {
                out.push(bytes[at]);
                at += 1;
            }
        }
        out
    }

    /// The features: each a name and the bytes of a piece of a descriptor.
    fn features(text: &str) -> Vec<(String, Vec<u8>)> {
        text.lines().filter(|line| !line.is_empty() && !line.starts_with('#')).map(|line| {
            let (name, bytes) = line.split_once(' ').unwrap_or((line, ""));
            (name.to_string(), unescaped(bytes))
        }).collect()
    }

    /// The descriptors of the table: each feature alone, and every two of them in the features' order.
    fn descriptors(features: &[(String, Vec<u8>)]) -> Vec<(String, Vec<u8>)> {
        let mut out: Vec<(String, Vec<u8>)> = features.to_vec();
        for (i, (a, first)) in features.iter().enumerate() {
            for (b, second) in &features[i + 1..] {
                out.push((format!("{}+{}", a, b), [first.as_slice(), second.as_slice()].concat()));
            }
        }
        out
    }

    /// The table's verdict: every descriptor of the features once in the expectations, read by `read_as` as the JVM
    /// read it; no id twice, none of no descriptor; the rows named by the review among them.
    fn faults(features: &[(String, Vec<u8>)], expected: &str, named: &[&str], read_as: &dyn Fn(&[u8]) -> String) -> Vec<String> {
        let mut faults = Vec::new();
        let mut rows: FxMap<&str, &str> = FxMap::default();
        for line in expected.lines().filter(|line| !line.is_empty() && !line.starts_with('#')) {
            let Some((id, answer)) = line.split_once(' ') else {
                faults.push(format!("{line}: a row without an answer"));
                continue;
            };
            if rows.insert(id, answer).is_some() {
                faults.push(format!("{id}: twice"));
            }
        }
        let wanted = descriptors(features);
        for (id, bytes) in &wanted {
            match rows.get(id.as_str()) {
                None => faults.push(format!("{id}: no row")),
                Some(answer) => {
                    let read = read_as(bytes);
                    if read != *answer {
                        faults.push(format!("{id}: teq reads {read}, the JVM {answer}"));
                    }
                }
            }
        }
        let ids: Vec<&str> = wanted.iter().map(|(id, _)| id.as_str()).collect();
        faults.extend(rows.keys().filter(|id| !ids.contains(id)).map(|id| format!("{id}: a row of no descriptor")));
        faults.extend(named.iter().filter(|name| !ids.contains(name)).map(|name| format!("{name}: no such feature")));
        if wanted.is_empty() {
            faults.push("no features".to_string());
        }
        faults
    }

    const FEATURES: &str = include_str!("../../tests/native-build/descriptor-features.txt");
    const EXPECTED: &str = include_str!("../../tests/native-build/descriptors.expected");
    /// The review's rows: a byte that is not UTF-8 in a comment, a blank as the separator, a trailing comma.
    const NAMED: [&str; 4] = ["latin1-comment", "sep-blank", "trailing-comma", "defines-trailing-comma"];

    /// The verdict's self-test, first: planted bad tables and a planted wrong reader, each of which it must find.
    #[test]
    fn reads_every_descriptor_of_the_table_as_the_jvm_does() {
        let features = features(FEATURES);
        let good: Vec<(String, String)> = descriptors(&features).into_iter().map(|(id, bytes)| (id, reading(&bytes))).collect();
        let table = |rows: &[(String, String)]| rows.iter().map(|(id, answer)| format!("{id} {answer}\n")).collect::<String>();
        let mut wrong = good.clone();
        wrong[1].1.push('x');
        let mut stray = good.clone();
        stray.push(("no-such-feature".to_string(), good[0].1.clone()));
        let mut twice = good.clone();
        twice.push(good[2].clone());
        let cut = table(&good);
        let planted: Vec<(&str, Vec<String>)> = vec![
            ("a wrong value", faults(&features, &table(&wrong), &NAMED, &reading)),
            ("a missing row", faults(&features, &table(&good[1..]), &NAMED, &reading)),
            ("an empty table", faults(&features, "", &NAMED, &reading)),
            ("a table cut short", faults(&features, &cut[..cut.len() / 2], &NAMED, &reading)),
            ("a stray id", faults(&features, &table(&stray), &NAMED, &reading)),
            ("an id twice", faults(&features, &table(&twice), &NAMED, &reading)),
            ("a named row that is no feature", faults(&features, &table(&good), &["no-such-feature"], &reading)),
            ("no features", faults(&[], "", &[], &reading)),
            ("a reader that drops a byte", faults(&features, &table(&good), &NAMED, &|bytes: &[u8]| reading(&bytes[..bytes.len().saturating_sub(1)]))),
        ];
        for (what, found) in &planted {
            assert!(!found.is_empty(), "the self-test: {what} was taken");
        }
        assert!(faults(&features, &table(&good), &NAMED, &reading).is_empty(), "the self-test: the good table was refused");
        let found = faults(&features, EXPECTED, &NAMED, &reading);
        assert!(found.is_empty(), "{} faults over {} descriptors:\n{}", found.len(), good.len(), found.join("\n"));
    }

    #[test]
    fn decodes_utf8_as_the_jvm_replaces_it() {
        let decoded = |bytes: &[u8]| String::from_utf16_lossy(&decode_utf8(bytes));
        assert_eq!(decoded("a\u{e9}\u{20ac}\u{1f600}".as_bytes()), "a\u{e9}\u{20ac}\u{1f600}");
        // A surrogate's three bytes are one sequence to the JVM, three to `from_utf8_lossy`.
        assert_eq!(decode_utf8(b"\xed\xa0\x80"), [REPLACEMENT]);
        assert_eq!(decode_utf8(b"\xe2\x82"), [REPLACEMENT]);
        assert_eq!(decode_utf8(b"\xe0\x80\x80"), [REPLACEMENT; 3]);
        assert_eq!(decode_utf8(b"\xf0\x9f\x98A"), [REPLACEMENT, b'A' as u16]);
    }
}
