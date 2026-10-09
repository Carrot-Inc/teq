//! Reads TASTy, the typed trees a Scala 3 artifact ships next to its class files. The format is
//! specified in `dotty.tools.tasty.TastyFormat`; tag and grammar names here follow it.
//!
//! A file is opened in two steps that decode no definition: the header, name table and section
//! table (`TastyFile::parse`), and an index of the definitions by address (`tree::index_*`).
//! The signature of one definition is decoded when it is asked for (`tree::Decoder`).

pub mod cli;
pub mod dump;
pub mod origins;
pub mod positions;
pub mod shapes;
pub mod show;
pub mod stats;
pub mod tags;
pub mod terms;
pub mod tree;
pub mod write;

use std::ops::Range;

pub type NameRef = u32;

/// What this reader was written against (Scala 3.8.4). An older minor version of the same major
/// reads the same way; a newer one may hold tags this reader does not know.
pub const MAJOR_VERSION: u32 = 28;
pub const MINOR_VERSION: u32 = 8;

#[derive(Debug)]
pub enum TName {
    Simple(Range<u32>),
    Qualified(NameRef, NameRef),
    Expanded(NameRef, NameRef),
    ExpandPrefix(NameRef, NameRef),
    Unique { separator: NameRef, num: u32, underlying: Option<NameRef> },
    DefaultGetter(NameRef, u32),
    SuperAccessor(NameRef),
    InlineAccessor(NameRef),
    ObjectClass(NameRef),
    BodyRetainer(NameRef),
    /// A name with the erased signature that tells overloads apart; `params` holds name
    /// references, or the negated length of a type parameter section.
    Signed { original: NameRef, target: Option<NameRef>, result: NameRef, params: Vec<i32> },
}

pub struct TastyFile {
    pub bytes: Vec<u8>,
    pub version: (u32, u32, u32),
    pub names: Vec<TName>,
    /// The "ASTs" section; addresses in the trees count from its start.
    pub asts: Range<usize>,
    pub attributes: Range<usize>,
    /// The "Positions" section, empty when the file has none.
    pub positions: Range<usize>,
    /// teq's own "TeqOrigins" section (`origins.rs`), `None` when the file has none.
    pub origins: Option<Range<usize>>,
    /// Per name, the reader's interned name for its source form (`Typer::lname`), once asked.
    pub interned: std::cell::RefCell<Vec<u32>>,
    /// Whether every simple name is UTF-8, checked once when the file is read.
    names_utf8: bool,
}

// A file is read by the typer's loader alone, on one thread at a time: today the thread that
// holds the typer (the typing thread during a phase, the compiler thread otherwise, never both
// at once, `typer/thread.rs`), in the parallel typer the thread holding the loader's lock.
// That is what makes sharing the `RefCell` memo sound. The reach's
// decoding threads (`decode_templates_ahead`) read the bytes and the names while that thread
// waits for them, never the memo.
unsafe impl Sync for TastyFile {}

/// A cursor over bytes; positions are relative to the start of the slice.
#[derive(Clone)]
pub struct Reader<'a> {
    pub bytes: &'a [u8],
    pub pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Reader<'a> {
        Reader { bytes, pos: 0 }
    }

    #[inline]
    pub fn at(&self, pos: usize) -> Reader<'a> {
        Reader { bytes: self.bytes, pos }
    }

    #[inline]
    pub fn byte(&mut self) -> u8 {
        let b = self.bytes.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        b
    }

    #[inline]
    pub fn peek(&self) -> u8 {
        self.bytes.get(self.pos).copied().unwrap_or(0)
    }

    /// Big endian base 128, the last digit marked by its high bit.
    #[inline]
    pub fn long_nat(&mut self) -> u64 {
        let mut x = 0u64;
        loop {
            let b = self.byte();
            x = (x << 7) | (b & 0x7f) as u64;
            if b & 0x80 != 0 || self.pos > self.bytes.len() {
                return x;
            }
        }
    }

    #[inline]
    pub fn nat(&mut self) -> u32 {
        self.long_nat() as u32
    }

    /// Two's complement, the sign taken from the first digit.
    pub fn long_int(&mut self) -> i64 {
        let mut b = self.byte();
        let mut x = (((b << 1) as i8) >> 1) as i64;
        while b & 0x80 == 0 && self.pos <= self.bytes.len() {
            b = self.byte();
            x = (x << 7) | (b & 0x7f) as i64;
        }
        x
    }

    /// Reads a length and returns the position where the entry ends, which a length read off
    /// a wrong address cannot put past the bytes.
    #[inline]
    pub fn end(&mut self) -> usize {
        let len = self.nat() as usize;
        (self.pos + len).min(self.bytes.len())
    }

    #[inline]
    pub fn at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }
}

impl TastyFile {
    pub fn parse(bytes: Vec<u8>) -> Result<TastyFile, String> {
        let mut r = Reader::new(&bytes);
        if bytes.len() < 4 || bytes[..4] != [0x5c, 0xa1, 0xab, 0x1f] {
            return Err("not a TASTy file (bad magic number)".to_string());
        }
        r.pos = 4;
        let version = (r.nat(), r.nat(), r.nat());
        let (major, minor, experimental) = version;
        if major != MAJOR_VERSION || minor > MINOR_VERSION || experimental != 0 {
            let exp = if experimental != 0 { format!("-experimental-{}", experimental) } else { String::new() };
            return Err(format!(
                "TASTy version {}.{}{} is not supported: this reader accepts {}.0 to {}.{} (Scala 3.0 to 3.8)",
                major, minor, exp, MAJOR_VERSION, MAJOR_VERSION, MINOR_VERSION
            ));
        }
        let tooling_len = r.nat() as usize;
        r.pos += tooling_len + 16;
        let names_end = r.end();
        if names_end > bytes.len() {
            return Err("truncated name table".to_string());
        }
        let mut names = Vec::with_capacity(256);
        while r.pos < names_end {
            names.push(read_name(&mut r)?);
        }
        let (mut asts, mut attributes, mut positions, mut origins) = (0..0, 0..0, 0..0, None);
        while !r.at_end() {
            let name = r.nat();
            let len = r.long_nat() as usize;
            let end = r.pos.saturating_add(len);
            if end > bytes.len() {
                return Err("truncated section".to_string());
            }
            if let Some(TName::Simple(range)) = names.get(name as usize) {
                match &bytes[range.start as usize..range.end as usize] {
                    b"ASTs" => asts = r.pos..end,
                    b"Attributes" => attributes = r.pos..end,
                    b"Positions" => positions = r.pos..end,
                    b"TeqOrigins" => origins = Some(r.pos..end),
                    _ => {}
                }
            }
            r.pos = end;
        }
        if asts.is_empty() {
            return Err("no ASTs section".to_string());
        }
        let interned = std::cell::RefCell::new(vec![u32::MAX; names.len()]);
        let names_utf8 = names.iter().all(|n| match n {
            TName::Simple(r) => std::str::from_utf8(&bytes[r.start as usize..r.end as usize]).is_ok(),
            _ => true,
        });
        Ok(TastyFile { bytes, version, names, asts, attributes, positions, origins, interned, names_utf8 })
    }

    pub fn trees(&self) -> Reader<'_> {
        Reader::new(&self.bytes[self.asts.clone()])
    }

    /// Whether the file was written by the Java outline parser or for the Scala 2 library.
    pub fn has_attribute(&self, tag: u8) -> bool {
        let mut r = Reader::new(&self.bytes[self.attributes.clone()]);
        while !r.at_end() {
            let t = r.byte();
            if t == tag {
                return true;
            }
            // A string attribute's tag (from 128, `SOURCEFILEattr`) is followed by its name.
            if t >= 128 {
                r.nat();
            }
        }
        false
    }

    pub fn simple(&self, name: NameRef) -> Option<&str> {
        match self.names.get(name as usize)? {
            TName::Simple(r) => {
                let bytes = &self.bytes[r.start as usize..r.end as usize];
                if self.names_utf8 {
                    // SAFETY: `parse` found every simple name to be UTF-8.
                    Some(unsafe { std::str::from_utf8_unchecked(bytes) })
                } else {
                    std::str::from_utf8(bytes).ok()
                }
            }
            _ => None,
        }
    }

    /// The name as scalac would print it, signatures left out.
    pub fn name(&self, name: NameRef) -> String {
        let mut out = String::new();
        self.write_name(name, &mut out);
        out
    }

    pub fn write_name(&self, name: NameRef, out: &mut String) {
        let Some(n) = self.names.get(name as usize) else {
            out.push_str("<bad name>");
            return;
        };
        match n {
            TName::Simple(_) => out.push_str(self.simple(name).unwrap_or("<bad utf8>")),
            TName::Qualified(a, b) => self.write_joined(*a, ".", *b, out),
            TName::Expanded(a, b) => self.write_joined(*a, "$$", *b, out),
            TName::ExpandPrefix(a, b) => self.write_joined(*a, "$", *b, out),
            TName::Unique { separator, num, underlying } => {
                if let Some(u) = underlying {
                    self.write_name(*u, out);
                }
                self.write_name(*separator, out);
                out.push_str(&num.to_string());
            }
            TName::DefaultGetter(u, index) => {
                self.write_name(*u, out);
                out.push_str("$default$");
                out.push_str(&(index + 1).to_string());
            }
            TName::SuperAccessor(u) => {
                out.push_str("super$");
                self.write_name(*u, out);
            }
            TName::InlineAccessor(u) => {
                out.push_str("inline$");
                self.write_name(*u, out);
            }
            TName::ObjectClass(u) => {
                self.write_name(*u, out);
                out.push('$');
            }
            TName::BodyRetainer(u) => {
                self.write_name(*u, out);
                out.push_str("$retainedBody");
            }
            TName::Signed { original, .. } => self.write_name(*original, out),
        }
    }

    fn write_joined(&self, a: NameRef, sep: &str, b: NameRef, out: &mut String) {
        self.write_name(a, out);
        out.push_str(sep);
        self.write_name(b, out);
    }

    /// The name without what marks it as the class of an object or as one of several overloads.
    pub fn source_name(&self, name: NameRef) -> NameRef {
        match self.names.get(name as usize) {
            Some(TName::ObjectClass(u)) => self.source_name(*u),
            Some(TName::Signed { original, .. }) => self.source_name(*original),
            _ => name,
        }
    }

    /// Whether the name is one of the accessors scalac makes for what an inline body reaches.
    pub fn is_inline_accessor(&self, name: NameRef) -> bool {
        matches!(self.names.get(self.source_name(name) as usize), Some(TName::InlineAccessor(_)))
    }

    /// The method whose runtime version the name holds, for scalac's `m$retainedBody`: the
    /// body of an inline override as its dispatch runs it, expanded where it was compiled.
    pub fn body_retained(&self, name: NameRef) -> Option<NameRef> {
        match self.names.get(self.source_name(name) as usize) {
            Some(TName::BodyRetainer(u)) => Some(self.source_name(*u)),
            _ => None,
        }
    }

    pub fn is_object_class(&self, name: NameRef) -> bool {
        matches!(self.names.get(name as usize), Some(TName::ObjectClass(_)))
    }

    /// The segments of a qualified name, outermost first.
    pub fn segments(&self, name: NameRef, out: &mut Vec<NameRef>) {
        match self.names.get(name as usize) {
            Some(TName::Qualified(a, b)) => {
                self.segments(*a, out);
                out.push(*b);
            }
            _ => out.push(name),
        }
    }
}

/// Whether the name table of a TASTy file holds one of the simple names `wanted`, read without
/// the rest of the file.
pub fn names_one_of(bytes: &[u8], wanted: &[&[u8]]) -> bool {
    if bytes.len() < 4 || bytes[..4] != [0x5c, 0xa1, 0xab, 0x1f] {
        return false;
    }
    let mut r = Reader::new(bytes);
    r.pos = 4;
    r.nat();
    r.nat();
    r.nat();
    let tooling_len = r.nat() as usize;
    r.pos += tooling_len + 16;
    let names_end = r.end();
    while r.pos < names_end {
        let tag = r.byte();
        let end = r.end();
        if tag == tags::name::UTF8 && wanted.iter().any(|w| bytes.get(r.pos..end) == Some(*w)) {
            return true;
        }
        r.pos = end;
    }
    false
}

fn read_name(r: &mut Reader) -> Result<TName, String> {
    use tags::name::*;
    let tag = r.byte();
    if tag == UTF8 {
        let len = r.nat();
        let start = r.pos as u32;
        r.pos += len as usize;
        return Ok(TName::Simple(start..start + len));
    }
    let end = r.end();
    let name = match tag {
        QUALIFIED => TName::Qualified(r.nat(), r.nat()),
        EXPANDED => TName::Expanded(r.nat(), r.nat()),
        EXPANDPREFIX => TName::ExpandPrefix(r.nat(), r.nat()),
        UNIQUE => {
            let separator = r.nat();
            let num = r.nat();
            let underlying = (r.pos < end).then(|| r.nat());
            TName::Unique { separator, num, underlying }
        }
        DEFAULTGETTER => TName::DefaultGetter(r.nat(), r.nat()),
        SUPERACCESSOR => TName::SuperAccessor(r.nat()),
        INLINEACCESSOR => TName::InlineAccessor(r.nat()),
        OBJECTCLASS => TName::ObjectClass(r.nat()),
        BODYRETAINER => TName::BodyRetainer(r.nat()),
        SIGNED | TARGETSIGNED => {
            let original = r.nat();
            let target = (tag == TARGETSIGNED).then(|| r.nat());
            let result = r.nat();
            let mut params = Vec::new();
            while r.pos < end {
                params.push(r.long_int() as i32);
            }
            TName::Signed { original, target, result, params }
        }
        other => return Err(format!("unknown name tag {}", other)),
    };
    r.pos = end;
    Ok(name)
}
