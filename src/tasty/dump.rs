//! `teq tasty --trees`: the raw form of a TASTy file, tag by tag, as scalac's `TastyPrinter`
//! shows it: the header, the name table with the kind of each name, the trees with their
//! addresses and lengths, the positions and the attributes. `teq tasty --nodes` prints the same
//! trees as one escaped record per node, which a reader consumes without parsing printed text.
//! It reads what a writer wrote without interpreting it, which is what comparing two picklers needs.

use super::show::name_kind;
use super::tags::*;
use super::{Reader, TName, TastyFile};

pub fn dump(file: &TastyFile, out: &mut String) {
    let bytes = &file.bytes;
    let mut r = Reader::new(bytes);
    r.pos = 4;
    let (major, minor, experimental) = (r.nat(), r.nat(), r.nat());
    let tooling_len = r.nat() as usize;
    let tooling = String::from_utf8_lossy(&bytes[r.pos..(r.pos + tooling_len).min(bytes.len())]).to_string();
    r.pos += tooling_len;
    let uuid: String = bytes.get(r.pos..r.pos + 16).map_or(String::new(), |u| u.iter().map(|b| format!("{:02x}", b)).collect());
    out.push_str(&format!("Header: version {}.{}.{}, tooling \"{}\", UUID {}\n", major, minor, experimental, tooling, uuid));
    out.push_str(&format!("Names ({}):\n", file.names.len()));
    for (i, n) in file.names.iter().enumerate() {
        out.push_str(&format!("{:>6}: {} {}\n", i, name_kind(n), name_text(file, i as u32)));
    }
    let trees = &bytes[file.asts.clone()];
    out.push_str(&format!("Trees ({} bytes):\n", trees.len()));
    let mut d = Dumper { file, r: Reader::new(trees), out, indent: 0 };
    while !d.r.at_end() {
        let before = d.r.pos;
        d.tree();
        if d.r.pos == before {
            break;
        }
    }
    positions(file, out);
    attributes(file, out);
    super::origins::show(file, out);
}

/// `teq tasty --nodes`: the trees section as records, one per node, in the order `Dumper::tree` visits them:
/// `<addr>\t<parent addr or ->\t<tag>[\t<key>=<value>]...`, then `end\t<the number of records>`. A name's text is
/// escaped as `observe.scala`'s `quoted`, so no record holds a tab or a newline; a read that does not reach a node's
/// end is the record `incomplete\t<addr>\t<end>`.
pub fn nodes(file: &TastyFile, out: &mut String) -> Result<(), String> {
    check_names(file)?;
    let trees = &file.bytes[file.asts.clone()];
    let mut d = NodeDumper { file, r: Reader::new(trees), out, records: 0 };
    while !d.r.at_end() {
        let before = d.r.pos;
        d.tree(None);
        if d.r.pos == before {
            break;
        }
    }
    d.out.push_str(&format!("end\t{}\n", d.records));
    Ok(())
}

/// A string as `observe.scala`'s `quoted` prints it: printable ASCII as it is, `\` and `"` as `\\` and `\"`,
/// `\n`, `\r`, `\t`, and every other UTF-16 unit as `\uXXXX` (four lowercase hex digits).
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for unit in s.encode_utf16() {
        match unit {
            0x5c => out.push_str("\\\\"),
            0x22 => out.push_str("\\\""),
            0x0a => out.push_str("\\n"),
            0x0d => out.push_str("\\r"),
            0x09 => out.push_str("\\t"),
            0x21..=0x7e => out.push(unit as u8 as char),
            _ => out.push_str(&format!("\\u{:04x}", unit)),
        }
    }
    out.push('"');
    out
}

struct NodeDumper<'a> {
    file: &'a TastyFile,
    r: Reader<'a>,
    out: &'a mut String,
    records: usize,
}

impl<'a> NodeDumper<'a> {
    fn record(&mut self, addr: usize, parent: Option<usize>, tag: &str, fields: &[(&str, String)]) {
        self.out.push_str(&format!("{}\t{}\t{}", addr, parent.map_or("-".to_string(), |p| p.to_string()), tag));
        for (key, value) in fields {
            self.out.push_str(&format!("\t{}={}", key, value));
        }
        self.out.push('\n');
        self.records += 1;
    }

    fn name(&mut self) -> (u32, String) {
        let n = self.r.nat();
        (n, name_json(self.file, n))
    }

    fn tree(&mut self, parent: Option<usize>) {
        let addr = self.r.pos;
        let tag = self.r.byte();
        let tag_name = tag_name(tag);
        if tag >= 128 {
            let end = self.r.end();
            let len = end - self.r.pos;
            match tag {
                VALDEF | DEFDEF | TYPEDEF | TYPEPARAM | PARAM | BIND
                | REFINEDTYPE | TERMREFIN | TYPEREFIN | SELECTIN => {
                    let (ref_, name) = self.name();
                    self.record(addr, parent, tag_name, &[("len", len.to_string()), ("name", name), ("ref", ref_.to_string())]);
                }
                RETURN | HOLE => {
                    let target = self.r.nat();
                    self.record(addr, parent, tag_name, &[("len", len.to_string()), ("addr", target.to_string())]);
                }
                PARAMTYPE => {
                    let binder = self.r.nat();
                    let num = self.r.nat();
                    self.record(addr, parent, tag_name,
                                &[("len", len.to_string()), ("binder", binder.to_string()), ("num", num.to_string())]);
                }
                _ => self.record(addr, parent, tag_name, &[("len", len.to_string())]),
            }
            match tag {
                METHODTYPE | POLYTYPE | TYPELAMBDATYPE => {
                    self.tree(Some(addr));
                    while self.r.pos < end && !is_modifier(self.r.peek()) {
                        self.tree(Some(addr));
                        let (ref_, name) = self.name();
                        let at = self.r.pos;
                        self.record(at, Some(addr), "name", &[("name", name), ("ref", ref_.to_string())]);
                    }
                }
                _ => {}
            }
            while self.r.pos < end {
                let before = self.r.pos;
                self.tree(Some(addr));
                if self.r.pos == before {
                    break;
                }
            }
            if self.r.pos != end {
                self.out.push_str(&format!("incomplete\t{}\t{}\n", self.r.pos, end));
                self.r.pos = end;
            }
        } else if tag >= 110 {
            match tag {
                IDENT | IDENTTPT | SELECT | SELECTTPT | TERMREF | TYPEREF | SELFDEF | NAMEDARG => {
                    let (ref_, name) = self.name();
                    self.record(addr, parent, tag_name, &[("name", name), ("ref", ref_.to_string())]);
                }
                _ => {
                    let target = self.r.nat();
                    self.record(addr, parent, tag_name, &[("addr", target.to_string())]);
                }
            }
            self.tree(Some(addr));
        } else if tag >= 90 {
            self.record(addr, parent, tag_name, &[]);
            self.tree(Some(addr));
        } else if tag >= 60 {
            match tag {
                TERMREFPKG | TYPEREFPKG | STRINGCONST | IMPORTED | RENAMED => {
                    let (ref_, name) = self.name();
                    self.record(addr, parent, tag_name, &[("name", name), ("ref", ref_.to_string())]);
                }
                LONGCONST | DOUBLECONST | BYTECONST | SHORTCONST | INTCONST | FLOATCONST => {
                    let value = self.r.long_int();
                    self.record(addr, parent, tag_name, &[("value", value.to_string())]);
                }
                CHARCONST => {
                    let value = self.r.nat();
                    self.record(addr, parent, tag_name, &[("value", value.to_string())]);
                }
                _ => {
                    let target = self.r.nat();
                    self.record(addr, parent, tag_name, &[("addr", target.to_string())]);
                }
            }
        } else {
            self.record(addr, parent, tag_name, &[]);
        }
    }
}

/// A name as TASTy's name table has it: its kind and constituent fields, recursively. `Simple` holds the UTF-8 text;
/// the others hold the name references they are built from, without joining any text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamePart {
    Utf8(String),
    Qualified(Box<NamePart>, Box<NamePart>),
    Expanded(Box<NamePart>, Box<NamePart>),
    ExpandPrefix(Box<NamePart>, Box<NamePart>),
    Unique { separator: Box<NamePart>, num: u32, underlying: Option<Box<NamePart>> },
    DefaultGetter(Box<NamePart>, u32),
    SuperAccessor(Box<NamePart>),
    InlineAccessor(Box<NamePart>),
    ObjectClass(Box<NamePart>),
    BodyRetainer(Box<NamePart>),
    Signed { original: Box<NamePart>, target: Option<Box<NamePart>>, params: Vec<ParamSig>, result: Box<NamePart> },
}

/// A signed name's parameter signature: a name, or a type parameter section's length (TName's negated length).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamSig {
    Name(Box<NamePart>),
    TypeParams(u32),
}

fn part(file: &TastyFile, n: u32) -> NamePart {
    let p = |r: u32| Box::new(part(file, r));
    match file.names.get(n as usize) {
        Some(TName::Simple(_)) => NamePart::Utf8(file.simple(n).unwrap_or("<bad utf8>").to_string()),
        Some(TName::Qualified(a, b)) => NamePart::Qualified(p(*a), p(*b)),
        Some(TName::Expanded(a, b)) => NamePart::Expanded(p(*a), p(*b)),
        Some(TName::ExpandPrefix(a, b)) => NamePart::ExpandPrefix(p(*a), p(*b)),
        Some(TName::Unique { separator, num, underlying }) => NamePart::Unique {
            separator: p(*separator),
            num: *num,
            underlying: underlying.map(p),
        },
        Some(TName::DefaultGetter(u, i)) => NamePart::DefaultGetter(p(*u), *i),
        Some(TName::SuperAccessor(u)) => NamePart::SuperAccessor(p(*u)),
        Some(TName::InlineAccessor(u)) => NamePart::InlineAccessor(p(*u)),
        Some(TName::ObjectClass(u)) => NamePart::ObjectClass(p(*u)),
        Some(TName::BodyRetainer(u)) => NamePart::BodyRetainer(p(*u)),
        Some(TName::Signed { original, target, result, params }) => NamePart::Signed {
            original: p(*original),
            target: target.map(p),
            params: params
                .iter()
                .map(|&q| if q < 0 { ParamSig::TypeParams((-q) as u32) } else { ParamSig::Name(p(q as u32)) })
                .collect(),
            result: p(*result),
        },
        None => NamePart::Utf8("<bad name>".to_string()),
    }
}

pub fn name_json(file: &TastyFile, n: u32) -> String {
    let mut out = String::new();
    write_name_json(&part(file, n), &mut out);
    out
}

fn write_name_json(n: &NamePart, out: &mut String) {
    fn pair(out: &mut String, tag: &str, a: &NamePart, b: &NamePart) {
        out.push_str("[\"");
        out.push_str(tag);
        out.push_str("\",");
        write_name_json(a, out);
        out.push(',');
        write_name_json(b, out);
        out.push(']');
    }
    fn one(out: &mut String, tag: &str, a: &NamePart) {
        out.push_str("[\"");
        out.push_str(tag);
        out.push_str("\",");
        write_name_json(a, out);
        out.push(']');
    }
    match n {
        NamePart::Utf8(text) => {
            out.push_str("[\"UTF8\",");
            out.push_str(&quote(text));
            out.push(']');
        }
        NamePart::Qualified(a, b) => pair(out, "QUALIFIED", a, b),
        NamePart::Expanded(a, b) => pair(out, "EXPANDED", a, b),
        NamePart::ExpandPrefix(a, b) => pair(out, "EXPANDPREFIX", a, b),
        NamePart::Unique { separator, num, underlying } => {
            out.push_str("[\"UNIQUE\",");
            write_name_json(separator, out);
            out.push_str(&format!(",{}", num));
            out.push(',');
            match underlying {
                Some(u) => write_name_json(u, out),
                None => out.push_str("null"),
            }
            out.push(']');
        }
        NamePart::DefaultGetter(u, i) => {
            out.push_str("[\"DEFAULTGETTER\",");
            write_name_json(u, out);
            out.push_str(&format!(",{}]", i));
        }
        NamePart::SuperAccessor(u) => one(out, "SUPERACCESSOR", u),
        NamePart::InlineAccessor(u) => one(out, "INLINEACCESSOR", u),
        NamePart::ObjectClass(u) => one(out, "OBJECTCLASS", u),
        NamePart::BodyRetainer(u) => one(out, "BODYRETAINER", u),
        NamePart::Signed { original, target, params, result } => {
            out.push_str(if target.is_some() { "[\"TARGETSIGNED\"," } else { "[\"SIGNED\"," });
            write_name_json(original, out);
            out.push(',');
            if let Some(t) = target {
                write_name_json(t, out);
                out.push(',');
            }
            out.push('[');
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                match p {
                    ParamSig::Name(n) => write_name_json(n, out),
                    ParamSig::TypeParams(len) => out.push_str(&format!("[\"TYPEPARAMS\",{}]", len)),
                }
            }
            out.push_str("],");
            write_name_json(result, out);
            out.push(']');
        }
    }
}

/// The JSON the name serializer wrote, read back into the structure it serialized: arrays, strings, numbers and null.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NameJson {
    Str(String),
    Num(u64),
    Arr(Vec<NameJson>),
    Null,
}

struct NameJsonReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> NameJsonReader<'a> {
    fn read(&mut self) -> Option<NameJson> {
        match self.bytes.get(self.pos).copied()? {
            b'[' => {
                self.pos += 1;
                let mut items = Vec::new();
                if self.bytes.get(self.pos) == Some(&b']') {
                    self.pos += 1;
                    return Some(NameJson::Arr(items));
                }
                loop {
                    items.push(self.read()?);
                    match self.bytes.get(self.pos).copied()? {
                        b',' => self.pos += 1,
                        b']' => {
                            self.pos += 1;
                            return Some(NameJson::Arr(items));
                        }
                        _ => return None,
                    }
                }
            }
            b'"' => {
                self.pos += 1;
                let mut units = Vec::new();
                loop {
                    let b = *self.bytes.get(self.pos)?;
                    self.pos += 1;
                    match b {
                        b'"' => return String::from_utf16(&units).ok().map(NameJson::Str),
                        b'\\' => {
                            let e = *self.bytes.get(self.pos)?;
                            self.pos += 1;
                            match e {
                                b'"' => units.push(b'"' as u16),
                                b'\\' => units.push(b'\\' as u16),
                                b'/' => units.push(b'/' as u16),
                                b'b' => units.push(8),
                                b'f' => units.push(12),
                                b'n' => units.push(10),
                                b'r' => units.push(13),
                                b't' => units.push(9),
                                b'u' => {
                                    let hex = self.bytes.get(self.pos..self.pos + 4)?;
                                    let s = std::str::from_utf8(hex).ok()?;
                                    units.push(u16::from_str_radix(s, 16).ok()?);
                                    self.pos += 4;
                                }
                                _ => return None,
                            }
                        }
                        _ => units.push(b as u16),
                    }
                }
            }
            b'n' if self.bytes.get(self.pos..self.pos + 4) == Some(b"null") => {
                self.pos += 4;
                Some(NameJson::Null)
            }
            b'0'..=b'9' => {
                let start = self.pos;
                while matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
                std::str::from_utf8(&self.bytes[start..self.pos]).ok()?.parse().ok().map(NameJson::Num)
            }
            _ => None,
        }
    }
}

fn name_from_json(j: &NameJson) -> Option<NamePart> {
    let items = match j {
        NameJson::Arr(items) => items,
        _ => return None,
    };
    let tag = match items.first()? {
        NameJson::Str(t) => t.as_str(),
        _ => return None,
    };
    let name = |j: &NameJson| name_from_json(j).map(Box::new);
    let two = |tag: &str| match items.as_slice() {
        [NameJson::Str(t), a, b] if t == tag => Some((a, b)),
        _ => None,
    };
    let one = |tag: &str| match items.as_slice() {
        [NameJson::Str(t), a] if t == tag => name(a),
        _ => None,
    };
    match tag {
        "UTF8" => match items.as_slice() {
            [NameJson::Str(_), NameJson::Str(text)] => Some(NamePart::Utf8(text.clone())),
            _ => None,
        },
        "QUALIFIED" => two("QUALIFIED").and_then(|(a, b)| Some(NamePart::Qualified(name(a)?, name(b)?))),
        "EXPANDED" => two("EXPANDED").and_then(|(a, b)| Some(NamePart::Expanded(name(a)?, name(b)?))),
        "EXPANDPREFIX" => two("EXPANDPREFIX").and_then(|(a, b)| Some(NamePart::ExpandPrefix(name(a)?, name(b)?))),
        "UNIQUE" => match items.as_slice() {
            [NameJson::Str(_), sep, NameJson::Num(num), u] => Some(NamePart::Unique {
                separator: name(sep)?,
                num: *num as u32,
                underlying: match u {
                    NameJson::Null => None,
                    other => Some(name(other)?),
                },
            }),
            _ => None,
        },
        "DEFAULTGETTER" => match items.as_slice() {
            [NameJson::Str(_), u, NameJson::Num(i)] => Some(NamePart::DefaultGetter(name(u)?, *i as u32)),
            _ => None,
        },
        "SUPERACCESSOR" => one("SUPERACCESSOR").map(NamePart::SuperAccessor),
        "INLINEACCESSOR" => one("INLINEACCESSOR").map(NamePart::InlineAccessor),
        "OBJECTCLASS" => one("OBJECTCLASS").map(NamePart::ObjectClass),
        "BODYRETAINER" => one("BODYRETAINER").map(NamePart::BodyRetainer),
        "SIGNED" | "TARGETSIGNED" => {
            let (original, target, params, result) = if tag == "SIGNED" {
                match items.as_slice() {
                    [NameJson::Str(_), original, NameJson::Arr(params), result] => (original, None, params, result),
                    _ => return None,
                }
            } else {
                match items.as_slice() {
                    [NameJson::Str(_), original, target, NameJson::Arr(params), result] => {
                        (original, Some(target), params, result)
                    }
                    _ => return None,
                }
            };
            let params = params
                .iter()
                .map(|p| match p {
                    NameJson::Arr(v) if matches!(v.as_slice(), [NameJson::Str(t), NameJson::Num(_)] if t == "TYPEPARAMS") => {
                        match v.as_slice() {
                            [_, NameJson::Num(len)] => Some(ParamSig::TypeParams(*len as u32)),
                            _ => None,
                        }
                    }
                    other => name(&other).map(ParamSig::Name),
                })
                .collect::<Option<Vec<_>>>()?;
            Some(NamePart::Signed {
                original: name(original)?,
                target: match target {
                    Some(t) => Some(name(t)?),
                    None => None,
                },
                params,
                result: name(result)?,
            })
        }
        _ => None,
    }
}

fn read_name_json(text: &str) -> Option<NamePart> {
    let mut r = NameJsonReader { bytes: text.as_bytes(), pos: 0 };
    let j = r.read()?;
    if r.pos != text.len() {
        return None;
    }
    name_from_json(&j)
}

fn names_injective(parts: &[NamePart], serialize: &dyn Fn(&NamePart) -> String) -> Result<(), String> {
    let mut seen: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for (i, part) in parts.iter().enumerate() {
        let text = serialize(part);
        if read_name_json(&text).as_ref() != Some(part) {
            return Err(format!("injectivity\tnames\t{}\t-", i));
        }
        if let Some(&j) = seen.get(&text) {
            if parts[j] != *part {
                return Err(format!("injectivity\tnames\t{}\t{}", j, i));
            }
        } else {
            seen.insert(text, i);
        }
    }
    Ok(())
}

/// `--nodes`' names are TASTy's name table's: every entry reads back to the entry it came from, before the first
/// record is written, and two entries may serialize alike only when they are equal (dotty's `NameTable` allows equal
/// entries). A failed read or an unequal collision is the run's failure.
pub fn check_names(file: &TastyFile) -> Result<(), String> {
    let parts: Vec<NamePart> = (0..file.names.len()).map(|i| part(file, i as u32)).collect();
    names_injective(&parts, &|p| {
        let mut out = String::new();
        write_name_json(p, &mut out);
        out
    })
}

/// The name with its structure: a signed name shows its signature, a qualified one its parts.
pub fn name_text(file: &TastyFile, n: u32) -> String {
    match file.names.get(n as usize) {
        Some(TName::Signed { original, target, result, params }) => {
            let ps: Vec<String> = params.iter().map(|&p| if p < 0 { format!("[{}]", -p) } else { file.name(p as u32) }).collect();
            let target = target.map_or(String::new(), |t| format!(" @{}", file.name(t)));
            format!("{}({}): {}{}", file.name(*original), ps.join(", "), file.name(*result), target)
        }
        _ => file.name(n),
    }
}

struct Dumper<'a> {
    file: &'a TastyFile,
    r: Reader<'a>,
    out: &'a mut String,
    indent: usize,
}

impl<'a> Dumper<'a> {
    fn line(&mut self, addr: usize, text: &str) {
        self.out.push_str(&format!("{:>6}:{}{}\n", addr, " ".repeat(self.indent), text));
    }

    fn name(&mut self) -> String {
        let n = self.r.nat();
        format!("{} [{}]", n, name_text(self.file, n))
    }

    fn tree(&mut self) {
        let addr = self.r.pos;
        let tag = self.r.byte();
        let tag_name = tag_name(tag);
        if tag >= 128 {
            let end = self.r.end();
            let head = match tag {
                VALDEF | DEFDEF | TYPEDEF | TYPEPARAM | PARAM | BIND => format!("{}({}) {}", tag_name, end - self.r.pos, self.name()),
                REFINEDTYPE | TERMREFIN | TYPEREFIN | SELECTIN => format!("{}({}) {}", tag_name, end - self.r.pos, self.name()),
                RETURN | HOLE => format!("{}({}) {}", tag_name, end - self.r.pos, self.r.nat()),
                PARAMTYPE => {
                    let len = end - self.r.pos;
                    let binder = self.r.nat();
                    let num = self.r.nat();
                    format!("{}({}) binder {} #{}", tag_name, len, binder, num)
                }
                _ => format!("{}({})", tag_name, end - self.r.pos),
            };
            self.line(addr, &head);
            self.indent += 2;
            match tag {
                METHODTYPE | POLYTYPE | TYPELAMBDATYPE => {
                    self.tree();
                    while self.r.pos < end && !is_modifier(self.r.peek()) {
                        self.tree();
                        let n = self.name();
                        let at = self.r.pos;
                        self.line(at, &format!("  name {}", n));
                    }
                }
                _ => {}
            }
            while self.r.pos < end {
                let before = self.r.pos;
                self.tree();
                if self.r.pos == before {
                    break;
                }
            }
            if self.r.pos != end {
                self.line(self.r.pos, &format!("<incomplete read, end {}>", end));
                self.r.pos = end;
            }
            self.indent -= 2;
        } else if tag >= 110 {
            let head = match tag {
                IDENT | IDENTTPT | SELECT | SELECTTPT | TERMREF | TYPEREF | SELFDEF | NAMEDARG => format!("{} {}", tag_name, self.name()),
                _ => format!("{} {}", tag_name, self.r.nat()),
            };
            self.line(addr, &head);
            self.indent += 2;
            self.tree();
            self.indent -= 2;
        } else if tag >= 90 {
            self.line(addr, tag_name);
            self.indent += 2;
            self.tree();
            self.indent -= 2;
        } else if tag >= 60 {
            let text = match tag {
                TERMREFPKG | TYPEREFPKG | STRINGCONST | IMPORTED | RENAMED => format!("{} {}", tag_name, self.name()),
                LONGCONST | DOUBLECONST => format!("{} {}", tag_name, self.r.long_int()),
                BYTECONST | SHORTCONST | INTCONST | FLOATCONST => format!("{} {}", tag_name, self.r.long_int()),
                _ => format!("{} {}", tag_name, self.r.nat()),
            };
            self.line(addr, &text);
        } else {
            self.line(addr, tag_name);
        }
    }
}

fn positions(file: &TastyFile, out: &mut String) {
    let Some(range) = section(file, "Positions") else { return };
    let mut r = Reader::new(&file.bytes[range]);
    let lines = r.nat();
    let sizes: Vec<u32> = (0..lines).map(|_| r.nat()).collect();
    out.push_str(&format!("Positions: {} lines, sizes {:?}\n", lines, sizes));
    let (mut addr, mut start, mut end) = (0i64, 0i64, 0i64);
    while !r.at_end() {
        let header = r.long_int();
        if header == 4 {
            let n = r.long_int();
            out.push_str(&format!("  source {} [{}]\n", n, file.name(n as u32)));
            continue;
        }
        addr += header >> 3;
        if header & 4 != 0 {
            start += r.long_int();
        }
        if header & 2 != 0 {
            end += r.long_int();
        }
        let point = if header & 1 != 0 { format!(" point {}", start + r.long_int()) } else { String::new() };
        out.push_str(&format!("  {}: {} .. {}{}\n", addr, start, end, point));
    }
}

fn attributes(file: &TastyFile, out: &mut String) {
    let Some(range) = section(file, "Attributes") else { return };
    let mut r = Reader::new(&file.bytes[range]);
    let mut shown = Vec::new();
    while !r.at_end() {
        let tag = r.byte();
        let name = match tag {
            1 => "SCALA2STANDARDLIBRARYattr".to_string(),
            2 => "EXPLICITNULLSattr".to_string(),
            3 => "CAPTURECHECKEDattr".to_string(),
            4 => "WITHPUREFUNSattr".to_string(),
            5 => "JAVAattr".to_string(),
            6 => "OUTLINEattr".to_string(),
            129 => {
                let n = r.nat();
                format!("SOURCEFILEattr {} [{}]", n, file.name(n))
            }
            t => format!("<attribute {}>", t),
        };
        shown.push(name);
    }
    out.push_str(&format!("Attributes: {}\n", shown.join(", ")));
}

/// The bytes of the section of that name, after its length.
pub fn section(file: &TastyFile, wanted: &str) -> Option<std::ops::Range<usize>> {
    let bytes = &file.bytes;
    let mut r = Reader::new(bytes);
    r.pos = 4;
    r.nat();
    r.nat();
    r.nat();
    let tooling = r.nat() as usize;
    r.pos += tooling + 16;
    let names_end = r.end();
    r.pos = names_end;
    while !r.at_end() {
        let name = r.nat();
        let end = r.end();
        if file.simple(name) == Some(wanted) {
            return Some(r.pos..end);
        }
        r.pos = end;
    }
    None
}

pub fn tag_name(tag: u8) -> &'static str {
    match tag {
        UNITCONST => "UNITconst",
        FALSECONST => "FALSEconst",
        TRUECONST => "TRUEconst",
        NULLCONST => "NULLconst",
        PRIVATE => "PRIVATE",
        PROTECTED => "PROTECTED",
        ABSTRACT => "ABSTRACT",
        FINAL => "FINAL",
        SEALED => "SEALED",
        CASE => "CASE",
        IMPLICIT => "IMPLICIT",
        LAZY => "LAZY",
        OVERRIDE => "OVERRIDE",
        INLINEPROXY => "INLINEPROXY",
        INLINE => "INLINE",
        STATIC => "STATIC",
        OBJECT => "OBJECT",
        TRAIT => "TRAIT",
        ENUM => "ENUM",
        LOCAL => "LOCAL",
        SYNTHETIC => "SYNTHETIC",
        ARTIFACT => "ARTIFACT",
        MUTABLE => "MUTABLE",
        FIELDACCESSOR => "FIELDaccessor",
        CASEACCESSOR => "CASEaccessor",
        COVARIANT => "COVARIANT",
        CONTRAVARIANT => "CONTRAVARIANT",
        HASDEFAULT => "HASDEFAULT",
        STABLE => "STABLE",
        MACRO => "MACRO",
        ERASED => "ERASED",
        OPAQUE => "OPAQUE",
        EXTENSION => "EXTENSION",
        GIVEN => "GIVEN",
        PARAMSETTER => "PARAMsetter",
        EXPORTED => "EXPORTED",
        OPEN => "OPEN",
        PARAMALIAS => "PARAMalias",
        TRANSPARENT => "TRANSPARENT",
        INFIX => "INFIX",
        INVISIBLE => "INVISIBLE",
        EMPTYCLAUSE => "EMPTYCLAUSE",
        SPLITCLAUSE => "SPLITCLAUSE",
        TRACKED => "TRACKED",
        SUBMATCH => "SUBMATCH",
        INTO => "INTO",
        SHAREDTERM => "SHAREDterm",
        SHAREDTYPE => "SHAREDtype",
        TERMREFDIRECT => "TERMREFdirect",
        TYPEREFDIRECT => "TYPEREFdirect",
        TERMREFPKG => "TERMREFpkg",
        TYPEREFPKG => "TYPEREFpkg",
        RECTHIS => "RECthis",
        BYTECONST => "BYTEconst",
        SHORTCONST => "SHORTconst",
        CHARCONST => "CHARconst",
        INTCONST => "INTconst",
        LONGCONST => "LONGconst",
        FLOATCONST => "FLOATconst",
        DOUBLECONST => "DOUBLEconst",
        STRINGCONST => "STRINGconst",
        IMPORTED => "IMPORTED",
        RENAMED => "RENAMED",
        THIS => "THIS",
        QUALTHIS => "QUALTHIS",
        CLASSCONST => "CLASSconst",
        BYNAMETYPE => "BYNAMEtype",
        BYNAMETPT => "BYNAMEtpt",
        NEW => "NEW",
        THROW => "THROW",
        IMPLICITARG => "IMPLICITarg",
        PRIVATEQUALIFIED => "PRIVATEqualified",
        PROTECTEDQUALIFIED => "PROTECTEDqualified",
        RECTYPE => "RECtype",
        SINGLETONTPT => "SINGLETONtpt",
        BOUNDED => "BOUNDED",
        EXPLICITTPT => "EXPLICITtpt",
        ELIDED => "ELIDED",
        IDENT => "IDENT",
        IDENTTPT => "IDENTtpt",
        SELECT => "SELECT",
        SELECTTPT => "SELECTtpt",
        TERMREFSYMBOL => "TERMREFsymbol",
        TERMREF => "TERMREF",
        TYPEREFSYMBOL => "TYPEREFsymbol",
        TYPEREF => "TYPEREF",
        SELFDEF => "SELFDEF",
        NAMEDARG => "NAMEDARG",
        PACKAGE => "PACKAGE",
        VALDEF => "VALDEF",
        DEFDEF => "DEFDEF",
        TYPEDEF => "TYPEDEF",
        IMPORT => "IMPORT",
        TYPEPARAM => "TYPEPARAM",
        PARAM => "PARAM",
        APPLY => "APPLY",
        TYPEAPPLY => "TYPEAPPLY",
        TYPED => "TYPED",
        ASSIGN => "ASSIGN",
        BLOCK => "BLOCK",
        IF => "IF",
        LAMBDA => "LAMBDA",
        MATCH => "MATCH",
        RETURN => "RETURN",
        WHILE => "WHILE",
        TRY => "TRY",
        INLINED => "INLINED",
        SELECTOUTER => "SELECTouter",
        REPEATED => "REPEATED",
        BIND => "BIND",
        ALTERNATIVE => "ALTERNATIVE",
        UNAPPLY => "UNAPPLY",
        ANNOTATEDTYPE => "ANNOTATEDtype",
        ANNOTATEDTPT => "ANNOTATEDtpt",
        CASEDEF => "CASEDEF",
        TEMPLATE => "TEMPLATE",
        SUPER => "SUPER",
        SUPERTYPE => "SUPERtype",
        REFINEDTYPE => "REFINEDtype",
        REFINEDTPT => "REFINEDtpt",
        APPLIEDTYPE => "APPLIEDtype",
        APPLIEDTPT => "APPLIEDtpt",
        TYPEBOUNDS => "TYPEBOUNDS",
        TYPEBOUNDSTPT => "TYPEBOUNDStpt",
        ANDTYPE => "ANDtype",
        ORTYPE => "ORtype",
        POLYTYPE => "POLYtype",
        TYPELAMBDATYPE => "TYPELAMBDAtype",
        LAMBDATPT => "LAMBDAtpt",
        PARAMTYPE => "PARAMtype",
        ANNOTATION => "ANNOTATION",
        TERMREFIN => "TERMREFin",
        TYPEREFIN => "TYPEREFin",
        SELECTIN => "SELECTin",
        EXPORT => "EXPORT",
        QUOTE => "QUOTE",
        SPLICE => "SPLICE",
        METHODTYPE => "METHODtype",
        APPLYSIGPOLY => "APPLYsigpoly",
        QUOTEPATTERN => "QUOTEPATTERN",
        SPLICEPATTERN => "SPLICEPATTERN",
        MATCHTYPE => "MATCHtype",
        MATCHTPT => "MATCHtpt",
        MATCHCASETYPE => "MATCHCASEtype",
        FLEXIBLETYPE => "FLEXIBLEtype",
        HOLE => "HOLE",
        _ => "<unknown tag>",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasty::{MAJOR_VERSION, MINOR_VERSION};

    /// A TASTy file of eight zero bytes of trees and a name table of `ASTs` then `extra`.
    fn file_with_names(extra: &[&str]) -> TastyFile {
        let mut names = Vec::new();
        for name in ["ASTs"].iter().chain(extra.iter()) {
            names.push(1);
            crate::tasty::write::write_nat(&mut names, name.len() as u64);
            names.extend_from_slice(name.as_bytes());
        }
        let mut bytes = vec![0x5c, 0xa1, 0xab, 0x1f];
        for v in [MAJOR_VERSION as u64, MINOR_VERSION as u64, 0, 0] {
            crate::tasty::write::write_nat(&mut bytes, v);
        }
        bytes.extend_from_slice(&[0; 16]);
        crate::tasty::write::write_nat(&mut bytes, names.len() as u64);
        bytes.extend_from_slice(&names);
        crate::tasty::write::write_nat(&mut bytes, 0);
        crate::tasty::write::write_nat(&mut bytes, 8);
        bytes.extend_from_slice(&[0; 8]);
        TastyFile::parse(bytes).expect("a TASTy file")
    }

    fn utf8(s: &str) -> Box<NamePart> {
        Box::new(NamePart::Utf8(s.to_string()))
    }

    fn path(parts: &[&str]) -> Box<NamePart> {
        let mut it = parts.iter();
        let mut n = utf8(it.next().unwrap());
        for p in it {
            n = Box::new(NamePart::Qualified(n, utf8(p)));
        }
        n
    }

    fn signed(first: bool) -> NamePart {
        let (a, b) = if first {
            (path(&["repr", "rec", "X, repr.rec.Y"]), path(&["repr", "rec", "Z"]))
        } else {
            (path(&["repr", "rec", "X"]), path(&["repr", "rec", "Y, repr.rec.Z"]))
        };
        NamePart::Signed {
            original: utf8("f"),
            target: None,
            params: vec![ParamSig::Name(a), ParamSig::Name(b)],
            result: path(&["scala", "Int"]),
        }
    }

    fn serialize(n: &NamePart) -> String {
        let mut out = String::new();
        write_name_json(n, &mut out);
        out
    }

    #[test]
    fn repeated_equal_name_entries_pass() {
        // Two unpaired surrogates both encode to `?` through `Codec.toUTF8` (NameBuffer.scala 80-89); dotty's
        // NameTable permits the repeated entry.
        let file = file_with_names(&["?", "?"]);
        assert!(check_names(&file).is_ok());
    }

    #[test]
    fn signed_names_serialize_apart() {
        let (a, b) = (signed(true), signed(false));
        assert_ne!(serialize(&a), serialize(&b));
        assert_eq!(read_name_json(&serialize(&a)), Some(a.clone()));
        assert_eq!(read_name_json(&serialize(&b)), Some(b.clone()));
        assert!(names_injective(&[a, b], &serialize).is_ok());
    }

    #[test]
    fn every_name_kind_round_trips() {
        let n = utf8("x");
        let parts = vec![
            *n.clone(),
            *path(&["a", "b"]),
            NamePart::Expanded(n.clone(), utf8("y")),
            NamePart::ExpandPrefix(n.clone(), utf8("y")),
            NamePart::Unique { separator: utf8("$"), num: 2, underlying: Some(n.clone()) },
            NamePart::Unique { separator: utf8("$"), num: 3, underlying: None },
            NamePart::DefaultGetter(n.clone(), 4),
            NamePart::SuperAccessor(n.clone()),
            NamePart::InlineAccessor(n.clone()),
            NamePart::ObjectClass(n.clone()),
            NamePart::BodyRetainer(n.clone()),
            NamePart::Signed {
                original: n.clone(),
                target: None,
                params: vec![ParamSig::TypeParams(2), ParamSig::Name(n.clone())],
                result: n.clone(),
            },
            NamePart::Signed {
                original: n.clone(),
                target: Some(n.clone()),
                params: vec![],
                result: n.clone(),
            },
        ];
        for p in parts {
            assert_eq!(read_name_json(&serialize(&p)), Some(p));
        }
    }

    fn flattened(n: &NamePart) -> String {
        match n {
            NamePart::Signed { original, target, result, .. } => {
                let mut out = String::new();
                out.push_str(if target.is_some() { "[\"TARGETSIGNED\"," } else { "[\"SIGNED\"," });
                write_name_json(original, &mut out);
                out.push(',');
                if let Some(t) = target {
                    write_name_json(t, &mut out);
                    out.push(',');
                }
                out.push_str("[],");
                write_name_json(result, &mut out);
                out.push(']');
                out
            }
            _ => serialize(n),
        }
    }

    #[test]
    fn a_serializer_that_drops_the_parameters_is_caught() {
        let (a, b) = (signed(true), signed(false));
        assert_eq!(flattened(&a), flattened(&b));
        assert!(names_injective(&[a, b], &flattened).is_err());
    }
}
