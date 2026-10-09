//! The name table: every name a file refers to, numbered in the order it is first asked for,
//! the parts of a derived name before the name itself (scalac's `NameBuffer.nameIndex`).

use super::buf::write_nat;
use super::buf::write_long_int;
use crate::intern::FxMap;
use crate::tasty::tags::name as tag;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Key {
    Simple(String),
    Qualified(u32, u32),
    Expanded(u32, u32),
    ExpandPrefix(u32, u32),
    DefaultGetter(u32, u32),
    SuperAccessor(u32),
    ObjectClass(u32),
    BodyRetainer(u32),
    /// `params`: a name reference per term parameter, a clause of type parameters as its
    /// negated length.
    Signed { original: u32, target: Option<u32>, result: u32, params: Vec<i32> },
}

#[derive(Default)]
pub struct Names {
    index: FxMap<Key, u32>,
    /// The simple names by their text, looked up without making a key.
    simple: FxMap<String, u32>,
    /// The qualified names by their dotted text.
    qualified: FxMap<String, u32>,
    /// The simple names by the interned names they are.
    interned: FxMap<crate::intern::Name, u32>,
    order: Vec<Key>,
}

impl Names {
    pub fn get(&mut self, key: Key) -> u32 {
        if let Some(&i) = self.index.get(&key) {
            return i;
        }
        let i = self.order.len() as u32;
        self.index.insert(key.clone(), i);
        self.order.push(key);
        i
    }

    pub fn simple(&mut self, s: &str) -> u32 {
        if let Some(&i) = self.simple.get(s) {
            return i;
        }
        let i = self.get(Key::Simple(s.to_string()));
        self.simple.insert(s.to_string(), i);
        i
    }

    /// The simple name of the interned name `n`, whose text is `text`.
    pub fn interned(&mut self, n: crate::intern::Name, text: &str) -> u32 {
        if let Some(&i) = self.interned.get(&n) {
            return i;
        }
        let i = self.simple(text);
        self.interned.insert(n, i);
        i
    }

    /// `a.b.c` as qualified names over its segments.
    /// A qualified name; a segment `X$`, an object's class, is the object class of the name up
    /// to `X`, as scalac's `fullName` of a class nested in an object has it
    /// (`java.util.AbstractMap$.SimpleImmutableEntry`).
    pub fn qualified(&mut self, dotted: &str) -> u32 {
        if let Some(&n) = self.qualified.get(dotted) {
            return n;
        }
        let n = self.qualified_now(dotted);
        self.qualified.insert(dotted.to_string(), n);
        n
    }

    fn qualified_now(&mut self, dotted: &str) -> u32 {
        let module = |s: &str| s.len() > 1 && s.ends_with('$');
        let mut segs = dotted.split('.');
        let first = segs.next().unwrap_or("");
        let mut n = if module(first) {
            let u = self.simple(&first[..first.len() - 1]);
            self.get(Key::ObjectClass(u))
        } else {
            self.simple(first)
        };
        for s in segs {
            if module(s) {
                let m = self.simple(&s[..s.len() - 1]);
                let q = self.get(Key::Qualified(n, m));
                n = self.get(Key::ObjectClass(q));
            } else {
                let m = self.simple(s);
                n = self.get(Key::Qualified(n, m));
            }
        }
        n
    }

    pub fn object_class(&mut self, name: &str) -> u32 {
        let u = self.simple(name);
        self.get(Key::ObjectClass(u))
    }

    pub fn default_getter(&mut self, method: &str, index: u32) -> u32 {
        let u = self.simple(method);
        self.get(Key::DefaultGetter(u, index))
    }

    /// A method's name with its signature: the erased parameter types as qualified names (a
    /// type parameter section as its negated length) and the erased result type.
    pub fn signed(&mut self, name: &str, target: Option<&str>, params: &[SigParam], result: &str) -> u32 {
        let original = self.simple(name);
        let target = target.filter(|t| *t != name).map(|t| self.simple(t));
        let result = self.qualified(result);
        let params = params
            .iter()
            .map(|p| match p {
                SigParam::Type(t) => self.qualified(t) as i32,
                SigParam::Types(n) => -(*n as i32),
            })
            .collect();
        self.get(Key::Signed { original, target, result, params })
    }

    /// A derived name with a signature (a default getter's), the name given by its reference.
    pub fn signed_ref(&mut self, original: u32, params: &[SigParam], result: &str) -> u32 {
        let result = self.qualified(result);
        let params = params
            .iter()
            .map(|p| match p {
                SigParam::Type(t) => self.qualified(t) as i32,
                SigParam::Types(n) => -(*n as i32),
            })
            .collect();
        self.get(Key::Signed { original, target: None, result, params })
    }

    /// How many names are numbered, for `truncate`.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Forgets the names numbered since `len` was what it was.
    pub fn truncate(&mut self, len: usize) {
        if self.order.len() > len {
            self.qualified.retain(|_, n| (*n as usize) < len);
            self.interned.retain(|_, n| (*n as usize) < len);
        }
        for key in self.order.drain(len..) {
            if let Key::Simple(text) = &key {
                self.simple.remove(text);
            }
            self.index.remove(&key);
        }
    }

    /// The table as it is written: its length, then every name.
    pub fn assemble(&self) -> Vec<u8> {
        let mut body = Vec::new();
        for key in &self.order {
            match key {
                Key::Simple(s) => {
                    // A lone surrogate is `?` as scalac's UTF-8 encoder writes it.
                    let bytes: std::borrow::Cow<'_, str> = if s.chars().any(|c| crate::text::surrogate_of(c).is_some()) {
                        s.chars().map(|c| if crate::text::surrogate_of(c).is_some() { '?' } else { c }).collect::<String>().into()
                    } else {
                        s.as_str().into()
                    };
                    body.push(tag::UTF8);
                    write_nat(&mut body, bytes.len() as u64);
                    body.extend_from_slice(bytes.as_bytes());
                }
                Key::Qualified(a, b) => with_length(&mut body, tag::QUALIFIED, |p| {
                    write_nat(p, *a as u64);
                    write_nat(p, *b as u64);
                }),
                Key::Expanded(a, b) => with_length(&mut body, tag::EXPANDED, |p| {
                    write_nat(p, *a as u64);
                    write_nat(p, *b as u64);
                }),
                Key::ExpandPrefix(a, b) => with_length(&mut body, tag::EXPANDPREFIX, |p| {
                    write_nat(p, *a as u64);
                    write_nat(p, *b as u64);
                }),
                Key::DefaultGetter(u, i) => with_length(&mut body, tag::DEFAULTGETTER, |p| {
                    write_nat(p, *u as u64);
                    write_nat(p, *i as u64);
                }),
                Key::SuperAccessor(u) => with_length(&mut body, tag::SUPERACCESSOR, |p| write_nat(p, *u as u64)),
                Key::ObjectClass(u) => with_length(&mut body, tag::OBJECTCLASS, |p| write_nat(p, *u as u64)),
                Key::BodyRetainer(u) => with_length(&mut body, tag::BODYRETAINER, |p| write_nat(p, *u as u64)),
                Key::Signed { original, target, result, params } => {
                    let t = if target.is_some() { tag::TARGETSIGNED } else { tag::SIGNED };
                    with_length(&mut body, t, |p| {
                        write_nat(p, *original as u64);
                        if let Some(t) = target {
                            write_nat(p, *t as u64);
                        }
                        write_nat(p, *result as u64);
                        for &x in params {
                            write_long_int(p, x as i64);
                        }
                    })
                }
            }
        }
        let mut out = Vec::with_capacity(body.len() + 4);
        write_nat(&mut out, body.len() as u64);
        out.extend_from_slice(&body);
        out
    }
}

/// A parameter of a signature: the erased type's qualified name, or a clause of type
/// parameters by its length.
#[derive(Clone)]
pub enum SigParam {
    Type(String),
    Types(usize),
}

fn with_length(out: &mut Vec<u8>, t: u8, f: impl FnOnce(&mut Vec<u8>)) {
    let mut payload = Vec::new();
    f(&mut payload);
    out.push(t);
    write_nat(out, payload.len() as u64);
    out.extend_from_slice(&payload);
}
