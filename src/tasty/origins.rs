//! The origins of a product's generated names: a
//! section of teq's own, `TeqOrigins`, which teq's writer puts last in every pickle it writes
//! and its reader reads back. It records what the names teq makes from positions are made of,
//! which the standard sections cannot give back: the file's key and the tag the producer
//! assigned it (`Program::file_tags`), and per maker of names a variant with the inputs of its
//! recipe: the file's record and the definitions' byte offsets, the classes the bodies define
//! (anonymous and named local ones) and the other sources their records name.
//!
//! ```text
//! Section  = Version Key Token Variant*
//! Version  = Nat                          1
//! Key      = Length UTF8*                 the file's key, `SourceFile::key`
//! Token    = LongNat                      the file's tag in the producer's build
//! Variant  = Kind Version Length Payload  skipped by its length where the kind or the version is unknown
//! ```
//!
//! scalac's `TastyUnpickler` indexes the sections by name and reads the ones it knows, so a
//! pickle with this section reads as one without it there.

use super::tree::Addr;
use super::{Reader, TastyFile};

pub const SECTION: &str = "TeqOrigins";
pub const VERSION: u32 = 1;

/// The kinds of variant, one per maker of names whose recipe
/// reads a position, and the sources the others name. `DEFINITIONS`, `ANONYMOUS_CLASSES`,
/// `LOCAL_CLASSES` and `SOURCES` are written; the others are reserved.
pub mod kind {
    /// Per definition of the pickle with an explicit position, in address order: its address
    /// as a delta from the previous one's and the byte offset of its name in the source text.
    pub const DEFINITIONS: u32 = 1;
    pub const ANONYMOUS_CLASSES: u32 = 2;
    pub const OUTER_THIS: u32 = 3;
    pub const LOCAL_CLASSES: u32 = 4;
    pub const SHARED_FUNCTIONS: u32 = 5;
    pub const QUOTE_COPIES: u32 = 6;
    pub const MACRO_FRESH_NAMES: u32 = 7;
    pub const OUTLINED_CALLEES: u32 = 8;
    /// The other sources a record names, `(Key Token)*`: a source reference `n` is the `n`-th,
    /// 0 the pickle's own.
    pub const SOURCES: u32 = 9;
    /// Per `ELIDED` right-hand side, its address and why it is withheld, the bodies census's
    /// `elided: <reason>` key: what the failure of a downstream that needs the body names.
    pub const WITHHELD: u32 = 10;
    /// Per module class whose template the writer gave parents after the declared ones, as
    /// scalac's `Desugar` and `SyntheticMembers` do (a case object's `Product`, `Serializable`
    /// and `Mirror.Singleton`, a companion's mirror), its `TYPEDEF`'s address and how many: what
    /// a reader that models them by its own rules takes off the end.
    pub const APPENDED_PARENTS: u32 = 11;

    pub fn name(kind: u32) -> &'static str {
        match kind {
            DEFINITIONS => "definitions",
            ANONYMOUS_CLASSES => "anonymous classes",
            OUTER_THIS => "outer this",
            LOCAL_CLASSES => "local classes",
            SHARED_FUNCTIONS => "shared functions",
            QUOTE_COPIES => "quote copies",
            MACRO_FRESH_NAMES => "macro fresh names",
            OUTLINED_CALLEES => "outlined callees",
            SOURCES => "sources",
            WITHHELD => "withheld bodies",
            APPENDED_PARENTS => "appended parents",
            _ => "unknown",
        }
    }
}

/// A place in a source: a source reference (0 the pickle's own, `n` the `n`-th of `SOURCES`)
/// and a byte offset in its text.
pub type Place = (u32, u32);

/// What a class a body defines is named from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassOrigin {
    /// An anonymous class (`ANONYMOUS_CLASSES`, `anon_name`): the enclosing class's name or the
    /// file's `<stem>$package`, the place of its `new`, the outermost expansion site's where an
    /// expansion made it, and its repetition at them.
    Anonymous { prefix: String, at: Place, site: Option<Place>, repeat: u32 },
    /// A named local class (`LOCAL_CLASSES`): the place of its definition.
    Local { at: Place },
}

/// The method an `INLINED` expands (`OUTLINED_CALLEES`), apart from the class it names: its
/// owner's qualified name, its name, its signature's text (`<params>:<result>`) and the place of
/// its source (the offset its definition's). Version 2 adds what the expansion took from its
/// call site, which the tree does not tell from what the body gave: the
/// trees of the leaves (`Program::leaf_bits`) and of the type tests of the call's type
/// arguments (`Program::leaf_tests`), by their addresses, the innermost `INLINED`'s.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Callee {
    pub owner: String,
    pub name: String,
    pub signature: String,
    pub at: Place,
    pub leaves: Vec<Addr>,
    pub leaf_tests: Vec<Addr>,
}

/// The section as read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origins {
    pub key: String,
    pub token: u64,
    /// The definitions' addresses and their names' byte offsets, the offsets the compiler
    /// orders and names definitions by, in address order.
    pub definitions: Vec<(Addr, u32)>,
    /// The other sources the records name, by key and token.
    pub sources: Vec<(String, u64)>,
    /// The classes the bodies define, by the address of their `TYPEDEF`, in address order.
    pub classes: Vec<(Addr, ClassOrigin)>,
    /// The methods the `INLINED`s expand, by their addresses, in address order.
    pub callees: Vec<(Addr, Callee)>,
    /// The `ELIDED` right-hand sides by their addresses, each with the census's reason.
    pub withheld: Vec<(Addr, String)>,
    /// The module classes the writer appended parents to, by their `TYPEDEF`s, with how many.
    pub appended: Vec<(Addr, u32)>,
    /// Every variant present, in its order: its kind, its version and whether this reader
    /// read it.
    pub variants: Vec<(u32, u32, bool)>,
}

/// What a file holds of the section.
#[derive(Debug, PartialEq, Eq)]
pub enum Found {
    /// A file without the section: every scalac pickle.
    Absent,
    /// A version of the section this reader does not know, ignored.
    Unknown(u32),
    /// A payload that does not read within its framing, ignored after a diagnostic.
    Malformed(String),
    Read(Origins),
}

/// The section's payload.
pub fn write(key: &str, token: u64, definitions: &[(Addr, u32)], sources: &[(String, u64)], classes: &[(Addr, ClassOrigin)], callees: &[(Addr, Callee)], withheld: &[(Addr, String)], appended: &[(Addr, u32)]) -> Vec<u8> {
    let mut out = Vec::new();
    nat(&mut out, VERSION as u64);
    nat(&mut out, key.len() as u64);
    out.extend_from_slice(key.as_bytes());
    nat(&mut out, token);
    let mut defs = Vec::new();
    let mut last = 0;
    for &(addr, offset) in definitions {
        nat(&mut defs, (addr - last) as u64);
        nat(&mut defs, offset as u64);
        last = addr;
    }
    variant(&mut out, kind::DEFINITIONS, 1, &defs);
    if !sources.is_empty() {
        let mut payload = Vec::new();
        for (key, token) in sources {
            text(&mut payload, key);
            nat(&mut payload, *token);
        }
        variant(&mut out, kind::SOURCES, 1, &payload);
    }
    let (mut anonymous, mut local) = (Vec::new(), Vec::new());
    let (mut last_anonymous, mut last_local) = (0, 0);
    for (addr, origin) in classes {
        match origin {
            ClassOrigin::Anonymous { prefix, at, site, repeat } => {
                nat(&mut anonymous, (addr - last_anonymous) as u64);
                last_anonymous = *addr;
                text(&mut anonymous, prefix);
                place(&mut anonymous, *at);
                match site {
                    Some(s) => {
                        nat(&mut anonymous, 1);
                        place(&mut anonymous, *s);
                    }
                    None => nat(&mut anonymous, 0),
                }
                nat(&mut anonymous, *repeat as u64);
            }
            ClassOrigin::Local { at } => {
                nat(&mut local, (addr - last_local) as u64);
                last_local = *addr;
                place(&mut local, *at);
            }
        }
    }
    if !anonymous.is_empty() {
        variant(&mut out, kind::ANONYMOUS_CLASSES, 1, &anonymous);
    }
    if !local.is_empty() {
        variant(&mut out, kind::LOCAL_CLASSES, 1, &local);
    }
    if !callees.is_empty() {
        let mut payload = Vec::new();
        let mut last = 0;
        for (addr, c) in callees {
            nat(&mut payload, (addr - last) as u64);
            last = *addr;
            text(&mut payload, &c.owner);
            text(&mut payload, &c.name);
            text(&mut payload, &c.signature);
            place(&mut payload, c.at);
            for addrs in [&c.leaves, &c.leaf_tests] {
                nat(&mut payload, addrs.len() as u64);
                let mut prev = *addr;
                for &a in addrs.iter() {
                    nat(&mut payload, (a - prev) as u64);
                    prev = a;
                }
            }
        }
        variant(&mut out, kind::OUTLINED_CALLEES, 2, &payload);
    }
    if !withheld.is_empty() {
        let mut payload = Vec::new();
        let mut last = 0;
        for (addr, reason) in withheld {
            nat(&mut payload, (addr - last) as u64);
            last = *addr;
            text(&mut payload, reason);
        }
        variant(&mut out, kind::WITHHELD, 1, &payload);
    }
    if !appended.is_empty() {
        let mut payload = Vec::new();
        let mut last = 0;
        for &(addr, n) in appended {
            nat(&mut payload, (addr - last) as u64);
            last = addr;
            nat(&mut payload, n as u64);
        }
        variant(&mut out, kind::APPENDED_PARENTS, 1, &payload);
    }
    out
}

fn variant(out: &mut Vec<u8>, kind: u32, version: u32, payload: &[u8]) {
    nat(out, kind as u64);
    nat(out, version as u64);
    nat(out, payload.len() as u64);
    out.extend_from_slice(payload);
}

fn text(out: &mut Vec<u8>, s: &str) {
    nat(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

fn place(out: &mut Vec<u8>, (source, offset): Place) {
    nat(out, source as u64);
    nat(out, offset as u64);
}

/// The name the producer gave the class a record is of, as its emitters name it: an anonymous
/// class's `<prefix>$$anon$<tag>_<offset>`, the outermost site's `$<tag>_<offset>` after it,
/// the repetition's `$<n>` after that from the second on; a local class's `<name>$<tag>_<offset>`;
/// the tag of a source its token's text (`source::tag_text`). `None` for a source reference the
/// section does not hold.
pub fn class_name(o: &Origins, origin: &ClassOrigin, name: &str) -> Option<String> {
    let position = |(source, offset): Place| -> Option<String> {
        let token = if source == 0 { o.token } else { o.sources.get(source as usize - 1)?.1 };
        Some(format!("{}_{}", crate::source::tag_text(token), offset))
    };
    match origin {
        ClassOrigin::Anonymous { prefix, at, site, repeat } => {
            let mut out = format!("{}$$anon${}", prefix, position(*at)?);
            if let Some(s) = site {
                out.push('$');
                out.push_str(&position(*s)?);
            }
            if *repeat > 1 {
                out.push_str(&format!("${}", repeat));
            }
            Some(out)
        }
        ClassOrigin::Local { at } => Some(format!("{}${}", name, position(*at)?)),
    }
}

fn nat(out: &mut Vec<u8>, x: u64) {
    super::write::write_nat(out, x);
}

pub fn read(file: &TastyFile) -> Found {
    let Some(range) = file.origins.clone() else { return Found::Absent };
    let bytes = &file.bytes[range];
    let mut r = Reader::new(bytes);
    let version = match checked_nat(&mut r, "the version") {
        Ok(v) => v,
        Err(e) => return Found::Malformed(e),
    };
    if version != VERSION as u64 {
        return Found::Unknown(version.min(u32::MAX as u64) as u32);
    }
    match read_v1(&mut r, file.asts.len()) {
        Ok(o) => Found::Read(o),
        Err(e) => Found::Malformed(e),
    }
}

fn read_v1(r: &mut Reader, trees: usize) -> Result<Origins, String> {
    let len = checked_nat(r, "the key's length")?;
    let end = (r.pos as u64).checked_add(len).filter(|&e| e <= r.bytes.len() as u64).ok_or("a key past the section")? as usize;
    let key = std::str::from_utf8(&r.bytes[r.pos..end]).map_err(|_| "a key that is not UTF-8")?.to_string();
    r.pos = end;
    let token = checked_nat(r, "the token")?;
    let mut definitions = Vec::new();
    let mut sources = Vec::new();
    let mut classes = Vec::new();
    let mut callees = Vec::new();
    let mut withheld = Vec::new();
    let mut appended = Vec::new();
    let mut variants = Vec::new();
    while !r.at_end() {
        let kind = narrow(checked_nat(r, "a variant's kind")?, "a variant's kind")?;
        let version = narrow(checked_nat(r, "a variant's version")?, "a variant's version")?;
        let len = checked_nat(r, "a variant's length")?;
        let end = (r.pos as u64).checked_add(len).filter(|&e| e <= r.bytes.len() as u64).ok_or("a variant past the section")? as usize;
        let known = match kind {
            kind::DEFINITIONS | kind::ANONYMOUS_CLASSES | kind::LOCAL_CLASSES | kind::SOURCES | kind::WITHHELD | kind::APPENDED_PARENTS => version == 1,
            kind::OUTLINED_CALLEES => version == 1 || version == 2,
            _ => false,
        };
        if known {
            let mut v = Reader { bytes: &r.bytes[..end], pos: r.pos };
            let mut addr: u64 = 0;
            let next_addr = |v: &mut Reader, addr: &mut u64, what: &str| -> Result<Addr, String> {
                let delta = checked_nat(v, what)?;
                *addr = addr.checked_add(delta).ok_or(format!("{} past 64 bits", what))?;
                if *addr >= trees as u64 {
                    return Err(format!("{} {}, past the trees", what, addr));
                }
                Ok(*addr as Addr)
            };
            while !v.at_end() {
                match kind {
                    kind::DEFINITIONS => {
                        let at = next_addr(&mut v, &mut addr, "a definition's address")?;
                        let offset = narrow(checked_nat(&mut v, "a definition's offset")?, "a definition's offset")?;
                        definitions.push((at, offset));
                    }
                    kind::WITHHELD => {
                        let at = next_addr(&mut v, &mut addr, "a withheld body's address")?;
                        let reason = read_text(&mut v, "a withheld body's reason")?;
                        withheld.push((at, reason));
                    }
                    kind::SOURCES => {
                        let key = read_text(&mut v, "a source's key")?;
                        let token = checked_nat(&mut v, "a source's token")?;
                        sources.push((key, token));
                    }
                    kind::ANONYMOUS_CLASSES => {
                        let at = next_addr(&mut v, &mut addr, "an anonymous class's address")?;
                        let prefix = read_text(&mut v, "an anonymous class's prefix")?;
                        let place = read_place(&mut v, "an anonymous class's place")?;
                        let site = match checked_nat(&mut v, "an anonymous class's site")? {
                            0 => None,
                            1 => Some(read_place(&mut v, "an anonymous class's site")?),
                            n => return Err(format!("an anonymous class's site marked {}", n)),
                        };
                        let repeat = narrow(checked_nat(&mut v, "an anonymous class's repetition")?, "an anonymous class's repetition")?;
                        classes.push((at, ClassOrigin::Anonymous { prefix, at: place, site, repeat }));
                    }
                    kind::APPENDED_PARENTS => {
                        let at = next_addr(&mut v, &mut addr, "a class's appended parents' address")?;
                        let n = narrow(checked_nat(&mut v, "a count of appended parents")?, "a count of appended parents")?;
                        appended.push((at, n));
                    }
                    kind::OUTLINED_CALLEES => {
                        let at = next_addr(&mut v, &mut addr, "an outlined callee's address")?;
                        let owner = read_text(&mut v, "an outlined callee's owner")?;
                        let name = read_text(&mut v, "an outlined callee's name")?;
                        let signature = read_text(&mut v, "an outlined callee's signature")?;
                        let place = read_place(&mut v, "an outlined callee's place")?;
                        let mut lists = [Vec::new(), Vec::new()];
                        if version == 2 {
                            for list in lists.iter_mut() {
                                let n = checked_nat(&mut v, "an expansion's count of leaves")?;
                                let mut prev = at as u64;
                                for _ in 0..n {
                                    prev = prev.checked_add(checked_nat(&mut v, "an expansion's leaf")?).filter(|&a| a < trees as u64).ok_or("an expansion's leaf past the trees")?;
                                    list.push(prev as Addr);
                                }
                            }
                        }
                        let [leaves, leaf_tests] = lists;
                        callees.push((at, Callee { owner, name, signature, at: place, leaves, leaf_tests }));
                    }
                    _ => {
                        let at = next_addr(&mut v, &mut addr, "a local class's address")?;
                        let place = read_place(&mut v, "a local class's place")?;
                        classes.push((at, ClassOrigin::Local { at: place }));
                    }
                }
            }
        }
        variants.push((kind, version, known));
        r.pos = end;
    }
    let callee_places: Vec<Vec<Option<Place>>> = callees.iter().map(|(_, c)| vec![Some(c.at)]).collect();
    let class_places: Vec<Vec<Option<Place>>> = classes
        .iter()
        .map(|(_, origin)| match origin {
            ClassOrigin::Anonymous { at, site, .. } => vec![Some(*at), *site],
            ClassOrigin::Local { at } => vec![Some(*at)],
        })
        .collect();
    for places in class_places.into_iter().chain(callee_places) {
        if let Some((source, _)) = places.into_iter().flatten().find(|&(s, _)| s as usize > sources.len()) {
            return Err(format!("a record's source {} past the {} sources", source, sources.len()));
        }
    }
    classes.sort_by_key(|&(a, _)| a);
    Ok(Origins { key, token, definitions, sources, classes, callees, withheld, appended, variants })
}

fn read_text(r: &mut Reader, what: &str) -> Result<String, String> {
    let len = checked_nat(r, what)?;
    let end = (r.pos as u64).checked_add(len).filter(|&e| e <= r.bytes.len() as u64).ok_or(format!("{} past its variant", what))? as usize;
    let s = std::str::from_utf8(&r.bytes[r.pos..end]).map_err(|_| format!("{} that is not UTF-8", what))?.to_string();
    r.pos = end;
    Ok(s)
}

fn read_place(r: &mut Reader, what: &str) -> Result<Place, String> {
    let source = narrow(checked_nat(r, what)?, what)?;
    let offset = narrow(checked_nat(r, what)?, what)?;
    Ok((source, offset))
}

/// A natural number that ends within the bytes and fits 64 bits.
fn checked_nat(r: &mut Reader, what: &str) -> Result<u64, String> {
    let mut x = 0u64;
    loop {
        let Some(&b) = r.bytes.get(r.pos) else { return Err(format!("{} cut short", what)) };
        r.pos += 1;
        if x >> 57 != 0 {
            return Err(format!("{} past 64 bits", what));
        }
        x = (x << 7) | (b & 0x7f) as u64;
        if b & 0x80 != 0 {
            return Ok(x);
        }
    }
}

fn narrow(x: u64, what: &str) -> Result<u32, String> {
    u32::try_from(x).map_err(|_| format!("{} past 32 bits", what))
}

/// `teq tasty --trees`: the section as read.
pub fn show(file: &TastyFile, out: &mut String) {
    match read(file) {
        Found::Absent => {}
        Found::Unknown(v) => out.push_str(&format!("{}: version {}, unknown to this reader\n", SECTION, v)),
        Found::Malformed(e) => out.push_str(&format!("{}: malformed: {}\n", SECTION, e)),
        Found::Read(o) => {
            out.push_str(&format!("{}: version {}, key {:?}, token {}\n", SECTION, VERSION, o.key, o.token));
            for &(kind, version, known) in &o.variants {
                let note = if known { "" } else { ", unknown to this reader" };
                out.push_str(&format!("  variant {} ({}), version {}{}\n", kind, kind::name(kind), version, note));
                if known && kind == kind::DEFINITIONS {
                    for &(addr, offset) in &o.definitions {
                        out.push_str(&format!("    {}: byte {}\n", addr, offset));
                    }
                }
                if known && kind == kind::WITHHELD {
                    for (addr, reason) in &o.withheld {
                        out.push_str(&format!("    {}: {}\n", addr, reason));
                    }
                }
                if known && kind == kind::APPENDED_PARENTS {
                    for (addr, n) in &o.appended {
                        out.push_str(&format!("    {}: {} parents\n", addr, n));
                    }
                }
                if known && kind == kind::SOURCES {
                    for (i, (key, token)) in o.sources.iter().enumerate() {
                        out.push_str(&format!("    {}: key {:?}, token {}\n", i + 1, key, token));
                    }
                }
                if known && kind == kind::OUTLINED_CALLEES {
                    for (addr, c) in &o.callees {
                        out.push_str(&format!("    {}: {}.{}({}) of source {} byte {}", addr, c.owner, c.name, c.signature, c.at.0, c.at.1));
                        if !c.leaves.is_empty() {
                            out.push_str(&format!(", leaves {:?}", c.leaves));
                        }
                        if !c.leaf_tests.is_empty() {
                            out.push_str(&format!(", leaf tests {:?}", c.leaf_tests));
                        }
                        out.push('\n');
                    }
                }
                if known && matches!(kind, kind::ANONYMOUS_CLASSES | kind::LOCAL_CLASSES) {
                    for (addr, origin) in &o.classes {
                        let anonymous = matches!(origin, ClassOrigin::Anonymous { .. });
                        if anonymous != (kind == kind::ANONYMOUS_CLASSES) {
                            continue;
                        }
                        let name = typedef_name(file, *addr).unwrap_or("?");
                        let produced = class_name(&o, origin, name).unwrap_or_else(|| "?".to_string());
                        out.push_str(&format!("    {}: {:?}, the class {}\n", addr, origin, produced));
                    }
                }
            }
        }
    }
}

/// The name of the `TYPEDEF` at an address of the trees.
pub fn typedef_name(file: &TastyFile, addr: Addr) -> Option<&str> {
    let trees = &file.bytes[file.asts.clone()];
    let mut r = Reader::new(trees.get(addr as usize..)?);
    if r.byte() != super::tags::TYPEDEF {
        return None;
    }
    r.nat();
    file.simple(r.nat() as super::NameRef)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A TASTy file of eight bytes of trees and the payload as its `TeqOrigins` section.
    fn file_with(payload: &[u8]) -> TastyFile {
        let mut names = Vec::new();
        for n in ["ASTs", SECTION] {
            names.push(1u8);
            nat(&mut names, n.len() as u64);
            names.extend_from_slice(n.as_bytes());
        }
        let mut b = vec![0x5c, 0xa1, 0xab, 0x1f];
        for v in [28, 8, 0, 0] {
            nat(&mut b, v);
        }
        b.extend_from_slice(&[0; 16]);
        nat(&mut b, names.len() as u64);
        b.extend_from_slice(&names);
        for (name, section) in [(0u64, &[0u8; 8][..]), (1, payload)] {
            nat(&mut b, name);
            nat(&mut b, section.len() as u64);
            b.extend_from_slice(section);
        }
        TastyFile::parse(b).expect("a TASTy file")
    }

    #[test]
    fn the_payload_reads_back() {
        let defs = vec![(0, 0), (2, 17), (3, 40)];
        let withheld = vec![(5, "elided: inline method".to_string())];
        let f = file_with(&write("p/A.scala", 1_589_615_084, &defs, &[], &[], &[], &withheld, &[]));
        let Found::Read(o) = read(&f) else { panic!("not read") };
        assert_eq!(o.key, "p/A.scala");
        assert_eq!(o.token, 1_589_615_084);
        assert_eq!(o.definitions, defs);
        assert_eq!(o.withheld, withheld);
        assert_eq!(o.variants, vec![(kind::DEFINITIONS, 1, true), (kind::WITHHELD, 1, true)]);
    }

    #[test]
    fn an_expansions_leaves_read_back_with_its_callee() {
        let callee = Callee { owner: "p.Fmt$".to_string(), name: "show".to_string(), signature: "scala.Int:java.lang.String".to_string(), at: (0, 50), leaves: vec![3, 5, 6], leaf_tests: vec![4] };
        let f = file_with(&write("p/A.scala", 7, &[], &[], &[], &[(2, callee.clone())], &[], &[]));
        let Found::Read(o) = read(&f) else { panic!("not read") };
        assert_eq!(o.callees, vec![(2, callee)]);
        assert!(o.variants.contains(&(kind::OUTLINED_CALLEES, 2, true)));
    }

    #[test]
    fn the_appended_parents_read_back() {
        let f = file_with(&write("p/A.scala", 7, &[], &[], &[], &[], &[], &[(1, 3), (4, 1)]));
        let Found::Read(o) = read(&f) else { panic!("not read") };
        assert_eq!(o.appended, vec![(1, 3), (4, 1)]);
    }

    #[test]
    fn an_unknown_version_and_a_malformed_payload_are_told_apart() {
        let mut p = write("A.scala", 7, &[], &[], &[], &[], &[], &[]);
        p[0] = 0x82;
        assert_eq!(read(&file_with(&p)), Found::Unknown(2));
        let p = write("A.scala", 7, &[(1, 3)], &[], &[], &[], &[], &[]);
        assert!(matches!(read(&file_with(&p[..p.len() - 1])), Found::Malformed(_)));
        assert!(matches!(read(&file_with(&p[..3])), Found::Malformed(_)));
    }

    #[test]
    fn an_address_or_a_number_past_its_width_is_malformed() {
        // A definition at 1, then one whose delta, u64::MAX, would wrap the address round to 0.
        let mut p = write("A.scala", 7, &[], &[], &[], &[], &[], &[]);
        let defs = [0x81, 0x80, 0x01, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0xff, 0x80];
        let at = p.len() - 1;
        p[at] = 0x80 | defs.len() as u8;
        p.extend_from_slice(&defs);
        assert_eq!(read(&file_with(&p)), Found::Malformed("a definition's address past 64 bits".to_string()));
        let mut p = write("A.scala", 7, &[], &[], &[], &[], &[], &[]);
        let at = p.len() - 1;
        // A delta of 71 bits.
        let eleven = [0x01, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0xff, 0x80];
        p[at] = 0x80 | eleven.len() as u8;
        p.extend_from_slice(&eleven);
        assert_eq!(read(&file_with(&p)), Found::Malformed("a definition's address past 64 bits".to_string()));
        let mut p = write("A.scala", 7, &[], &[], &[], &[], &[], &[]);
        let at = p.len() - 1;
        p[at] = 0x86;
        p.extend_from_slice(&[0x81, 0x10, 0x00, 0x00, 0x00, 0x80]);
        assert_eq!(read(&file_with(&p)), Found::Malformed("a definition's offset past 32 bits".to_string()));
    }

    #[test]
    fn an_empty_section_is_malformed() {
        assert_eq!(read(&file_with(&[])), Found::Malformed("the version cut short".to_string()));
    }

    #[test]
    fn an_unknown_variant_is_skipped() {
        let mut p = write("A.scala", 7, &[(1, 3)], &[], &[], &[], &[], &[]);
        p.extend_from_slice(&[0x8c, 0x81, 0x82, 0x01, 0x02]);
        let Found::Read(o) = read(&file_with(&p)) else { panic!("not read") };
        assert_eq!(o.definitions, vec![(1, 3)]);
        assert_eq!(o.variants, vec![(kind::DEFINITIONS, 1, true), (12, 1, false)]);
    }

    #[test]
    fn the_classes_read_back_and_name_their_classes() {
        let sources = vec![("Make.scala".to_string(), 3_964_894_679)];
        let classes = vec![
            (2, ClassOrigin::Anonymous { prefix: "Make".to_string(), at: (1, 200), site: Some((0, 106)), repeat: 1 }),
            (4, ClassOrigin::Local { at: (0, 450) }),
            (6, ClassOrigin::Anonymous { prefix: "Use$package".to_string(), at: (0, 12), site: None, repeat: 2 }),
        ];
        let f = file_with(&write("Use.scala", 306_633_504, &[(0, 0)], &sources, &classes, &[], &[], &[]));
        let Found::Read(o) = read(&f) else { panic!("not read") };
        assert_eq!(o.sources, sources);
        assert_eq!(o.classes, classes);
        let names: Vec<String> = o.classes.iter().map(|(_, c)| class_name(&o, c, "Twice").unwrap()).collect();
        assert_eq!(names, vec!["Make$$anon$1tklgbb_200$52k7xc_106", "Twice$52k7xc_450", "Use$package$$anon$52k7xc_12$2"]);
        // A source reference past the sources is malformed.
        let bad = vec![(2, ClassOrigin::Local { at: (2, 1) })];
        assert!(matches!(read(&file_with(&write("Use.scala", 7, &[], &sources, &bad, &[], &[], &[]))), Found::Malformed(_)));
    }
}
