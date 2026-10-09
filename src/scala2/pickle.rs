//! The symbol table scalac 2 pickles into a class file (`scala.reflect.ScalaSignature`): an
//! array of entries (names, symbols, types, constants, annotations), each a tag, a length and
//! references to other entries by index. The format is `PickleFormat` of the compiler.

pub const TERMNAME: u8 = 1;
pub const TYPENAME: u8 = 2;
pub const NONESYM: u8 = 3;
pub const TYPESYM: u8 = 4;
pub const ALIASSYM: u8 = 5;
pub const CLASSSYM: u8 = 6;
pub const MODULESYM: u8 = 7;
pub const VALSYM: u8 = 8;
pub const EXTREF: u8 = 9;
pub const EXTMODCLASSREF: u8 = 10;
pub const NOPREFIXTPE: u8 = 12;
pub const THISTPE: u8 = 13;
pub const SINGLETPE: u8 = 14;
pub const CONSTANTTPE: u8 = 15;
pub const TYPEREFTPE: u8 = 16;
pub const TYPEBOUNDSTPE: u8 = 17;
pub const REFINEDTPE: u8 = 18;
pub const CLASSINFOTPE: u8 = 19;
pub const METHODTPE: u8 = 20;
pub const POLYTPE: u8 = 21;
pub const IMPLICITMETHODTPE: u8 = 22;
pub const LITERALUNIT: u8 = 24;
pub const LITERALBOOLEAN: u8 = 25;
pub const LITERALBYTE: u8 = 26;
pub const LITERALSHORT: u8 = 27;
pub const LITERALCHAR: u8 = 28;
pub const LITERALINT: u8 = 29;
pub const LITERALLONG: u8 = 30;
pub const LITERALFLOAT: u8 = 31;
pub const LITERALDOUBLE: u8 = 32;
pub const LITERALSTRING: u8 = 33;
pub const LITERALNULL: u8 = 34;
pub const LITERALCLASS: u8 = 35;
pub const LITERALENUM: u8 = 36;
pub const ANNOTATEDTPE: u8 = 42;
pub const SUPERTPE: u8 = 46;
pub const EXISTENTIALTPE: u8 = 48;

/// Symbol flags as scalac 2 keeps them (`Flags.scala`); a pickle stores the low twelve
/// remapped (`pickledToRawFlags`).
pub mod flags {
    pub const PROTECTED: u64 = 1 << 0;
    pub const OVERRIDE: u64 = 1 << 1;
    pub const PRIVATE: u64 = 1 << 2;
    pub const ABSTRACT: u64 = 1 << 3;
    pub const DEFERRED: u64 = 1 << 4;
    pub const FINAL: u64 = 1 << 5;
    pub const METHOD: u64 = 1 << 6;
    pub const MODULE: u64 = 1 << 8;
    pub const IMPLICIT: u64 = 1 << 9;
    pub const SEALED: u64 = 1 << 10;
    pub const CASE: u64 = 1 << 11;
    pub const PARAM: u64 = 1 << 13;
    pub const MACRO: u64 = 1 << 15;
    pub const COVARIANT: u64 = 1 << 16;
    pub const CONTRAVARIANT: u64 = 1 << 17;
    pub const LOCAL: u64 = 1 << 19;
    pub const SYNTHETIC: u64 = 1 << 21;
    pub const STABLE: u64 = 1 << 22;
    pub const CASEACCESSOR: u64 = 1 << 24;
    /// `TRAIT` on a class, `DEFAULTPARAM` on a parameter.
    pub const TRAIT: u64 = 1 << 25;
    pub const BRIDGE: u64 = 1 << 26;
    pub const ACCESSOR: u64 = 1 << 27;
    pub const SUPERACCESSOR: u64 = 1 << 28;
    pub const PARAMACCESSOR: u64 = 1 << 29;
    pub const LAZY: u64 = 1 << 31;
    pub const EXISTENTIAL: u64 = 1 << 35;
    pub const EXPANDEDNAME: u64 = 1 << 36;
    pub const SPECIALIZED: u64 = 1 << 40;
    pub const ARTIFACT: u64 = 1 << 46;
}

const PICKLED_LOW: [u64; 12] = [
    flags::IMPLICIT,
    flags::FINAL,
    flags::PRIVATE,
    flags::PROTECTED,
    flags::SEALED,
    flags::OVERRIDE,
    flags::CASE,
    flags::ABSTRACT,
    flags::DEFERRED,
    flags::METHOD,
    flags::MODULE,
    1 << 7,
];

fn raw_flags(pickled: u64) -> u64 {
    let mut raw = pickled & !0xfff;
    for (bit, &flag) in PICKLED_LOW.iter().enumerate() {
        if pickled & (1 << bit) != 0 {
            raw |= flag;
        }
    }
    raw
}

/// Decodes the `bytes` string of `ScalaSignature` in place (`ByteCodecs.decode`): the string's
/// modified UTF-8 bytes shifted by one with zero written as `0xc0 0x80`, packing 7 bits a byte.
pub fn decode_signature(mut src: Vec<u8>) -> Vec<u8> {
    let (mut i, mut j) = (0, 0);
    while i < src.len() {
        let b = src[i];
        if b == 0xc0 && src.get(i + 1) == Some(&0x80) {
            src[j] = 0x7f;
            i += 2;
        } else if b == 0 {
            src[j] = 0x7f;
            i += 1;
        } else {
            src[j] = b.wrapping_sub(1);
            i += 1;
        }
        j += 1;
    }
    let len = j;
    let mut out = Vec::with_capacity(len * 7 / 8 + 1);
    let (mut acc, mut bits) = (0u32, 0u32);
    for &b in &src[..len] {
        acc |= ((b & 0x7f) as u32) << bits;
        bits += 7;
        if bits >= 8 {
            out.push(acc as u8);
            acc >>= 8;
            bits -= 8;
        }
    }
    out
}

pub struct Pickle {
    pub bytes: Vec<u8>,
    /// Per entry, the offset of its tag.
    starts: Vec<u32>,
}

#[derive(Clone, Copy)]
pub struct Cursor<'a> {
    bytes: &'a [u8],
    pub pos: usize,
    pub end: usize,
}

impl<'a> Cursor<'a> {
    fn byte(&mut self) -> u8 {
        let b = self.bytes.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        b
    }

    /// Big endian base 128, every digit but the last marked by its high bit.
    pub fn nat(&mut self) -> u64 {
        let mut x = 0u64;
        loop {
            let b = self.byte();
            x = (x << 7) | (b & 0x7f) as u64;
            if b & 0x80 == 0 || self.pos >= self.bytes.len() {
                return x;
            }
        }
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.end
    }

    /// The rest of the entry as references.
    pub fn refs(&mut self) -> Vec<u32> {
        let mut out = Vec::new();
        while !self.at_end() {
            out.push(self.nat() as u32);
        }
        out
    }

    /// A signed number in `end - pos` bytes, big endian.
    pub fn long(&mut self) -> i64 {
        let mut x = 0i64;
        let n = self.end.saturating_sub(self.pos);
        for _ in 0..n {
            x = (x << 8) | self.byte() as i64;
        }
        if n > 0 && n < 8 {
            let shift = 64 - 8 * n as u32;
            x = (x << shift) >> shift;
        }
        x
    }
}

/// A symbol defined by the pickle.
#[derive(Clone, Debug)]
pub struct LocalSym {
    pub tag: u8,
    pub name: u32,
    pub owner: u32,
    pub flags: u64,
    pub within: Option<u32>,
    pub info: u32,
    /// A class's self type, a val's alias.
    pub extra: Option<u32>,
}

impl Pickle {
    pub fn parse(bytes: Vec<u8>) -> Result<Pickle, String> {
        let mut c = Cursor { bytes: &bytes, pos: 0, end: bytes.len() };
        let (major, minor) = (c.nat(), c.nat());
        if major != 5 || minor > 2 {
            return Err(format!("Scala 2 signature version {}.{} is not supported: this reader accepts 5.0 to 5.2", major, minor));
        }
        let n = c.nat() as usize;
        if n > bytes.len() {
            return Err("truncated Scala 2 signature".to_string());
        }
        let mut starts = Vec::with_capacity(n);
        for _ in 0..n {
            starts.push(c.pos as u32);
            c.byte();
            let len = c.nat() as usize;
            c.pos += len;
            if c.pos > bytes.len() {
                return Err("truncated Scala 2 signature".to_string());
            }
        }
        Ok(Pickle { bytes, starts })
    }

    pub fn len(&self) -> usize {
        self.starts.len()
    }

    pub fn tag(&self, i: u32) -> u8 {
        self.starts.get(i as usize).map_or(0, |&s| self.bytes[s as usize])
    }

    /// The body of entry `i`, after its tag and length.
    pub fn body(&self, i: u32) -> Cursor<'_> {
        let start = self.starts.get(i as usize).copied().unwrap_or(self.bytes.len() as u32) as usize;
        let mut c = Cursor { bytes: &self.bytes, pos: start, end: self.bytes.len() };
        if start < self.bytes.len() {
            c.byte();
            let len = c.nat() as usize;
            c.end = (c.pos + len).min(self.bytes.len());
        }
        c
    }

    pub fn name(&self, i: u32) -> &str {
        match self.tag(i) {
            TERMNAME | TYPENAME => {
                let c = self.body(i);
                std::str::from_utf8(&self.bytes[c.pos..c.end]).unwrap_or("?")
            }
            _ => "?",
        }
    }

    pub fn is_type_name(&self, i: u32) -> bool {
        self.tag(i) == TYPENAME
    }

    pub fn is_sym(&self, i: u32) -> bool {
        matches!(self.tag(i), NONESYM..=EXTMODCLASSREF)
    }

    pub fn local_sym(&self, i: u32) -> Option<LocalSym> {
        let tag = self.tag(i);
        if !(TYPESYM..=VALSYM).contains(&tag) {
            return None;
        }
        let mut c = self.body(i);
        let name = c.nat() as u32;
        let owner = c.nat() as u32;
        let flags = raw_flags(c.nat());
        let mut info = c.nat() as u32;
        let mut within = None;
        if self.is_sym(info) {
            within = Some(info);
            info = c.nat() as u32;
        }
        let extra = (!c.at_end()).then(|| c.nat() as u32);
        Some(LocalSym { tag, name, owner, flags, within, info, extra })
    }

    /// Name and owner of an `EXTref` or `EXTMODCLASSref`; no owner is the root package.
    pub fn ext_ref(&self, i: u32) -> (u32, Option<u32>) {
        let mut c = self.body(i);
        let name = c.nat() as u32;
        let owner = (!c.at_end()).then(|| c.nat() as u32).filter(|&o| self.tag(o) != NONESYM);
        (name, owner)
    }

    /// The owner of any symbol entry.
    pub fn owner_of(&self, i: u32) -> Option<u32> {
        match self.tag(i) {
            EXTREF | EXTMODCLASSREF => self.ext_ref(i).1,
            TYPESYM..=VALSYM => {
                let mut c = self.body(i);
                c.nat();
                Some(c.nat() as u32)
            }
            _ => None,
        }
    }

    pub fn sym_name(&self, i: u32) -> u32 {
        let mut c = self.body(i);
        c.nat() as u32
    }

    /// The class symbol, not a module class, whose binary name after its package's path is
    /// `binary` (`Box$T` for a trait `T` of object `Box`, `A$B` for a trait written `` `A$B` ``):
    /// each candidate's name made from the names of its owners up to its package, joined by `$`,
    /// so that a `$` written in a name is not taken for an owner. A companion's module class, which
    /// the pickle may hold first under the same name, is no candidate; an owner may be one.
    pub fn class_named(&self, binary: &str) -> Option<u32> {
        (0..self.len() as u32).find(|&i| {
            self.local_sym(i).map_or(false, |s| s.tag == CLASSSYM && s.flags & flags::MODULE == 0) && self.local_binary_name(i).as_deref() == Some(binary)
        })
    }

    fn local_binary_name(&self, i: u32) -> Option<String> {
        let mut names = Vec::new();
        let mut at = i;
        while let Some(s) = self.local_sym(at) {
            if s.tag != CLASSSYM || names.len() > 64 {
                return None;
            }
            names.push(self.name(s.name).trim_end().to_string());
            at = s.owner;
        }
        names.reverse();
        Some(names.join("$"))
    }
}

impl Pickle {
    /// Every entry with its tag and fields, for `TEQ_SCALA2_DUMP`.
    pub fn dump(&self) -> String {
        let mut out = String::new();
        for i in 0..self.len() as u32 {
            let tag = self.tag(i);
            let line = match tag {
                TERMNAME | TYPENAME => format!("{} {:?}", if tag == TERMNAME { "term" } else { "type" }, self.name(i)),
                TYPESYM..=VALSYM => {
                    let s = self.local_sym(i).unwrap();
                    format!("sym{} {} owner={} flags={:#x} within={:?} info={} extra={:?}", tag, self.name(s.name), s.owner, s.flags, s.within, s.info, s.extra)
                }
                EXTREF | EXTMODCLASSREF => {
                    let (n, o) = self.ext_ref(i);
                    format!("{} {} owner={:?}", if tag == EXTREF { "ext" } else { "extmodclass" }, self.name(n), o)
                }
                _ => format!("tag{} {:?}", tag, self.body(i).refs()),
            };
            out.push_str(&format!("{:4}: {}\n", i, line));
        }
        out
    }
}
