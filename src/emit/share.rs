//! Anonymous classes made while inline methods expanded that share one class in the output.
//!
//! An anonymous class in an inline body is a class of its own at every expansion, as scalac
//! duplicates it (tapir's `Schema.derived` makes one `Derivation` per derived type, each with
//! the same eleven kilobytes of members). Classes made at one expression whose bodies are the
//! same up to their own members, locals and captures are written once: the first in canonical
//! order stands for the others, which are neither written, registered nor numbered, and `new`
//! of any of them makes the representative. The shared class is named by the expression and a
//! hash of its body, and stands in the module of the inline method's file, which no edit of a
//! site moves.

use super::layout::compare_classes;
use super::reach::Reach;
use crate::intern::{FxHasher, FxMap, Interner};
use crate::source::FileId;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::hash::Hasher;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Default)]
pub struct Shared {
    /// A class that another one of the same body stands for, with that one.
    pub rep: FxMap<ClassId, ClassId>,
    /// Per representative: its name in the output.
    pub groups: FxMap<ClassId, SharedClass>,
}

pub struct SharedClass {
    pub name: String,
}

impl Shared {
    #[inline]
    pub fn rep_of(&self, c: ClassId) -> ClassId {
        if self.rep.is_empty() {
            return c;
        }
        self.rep.get(&c).copied().unwrap_or(c)
    }

    pub fn stands_for_another(&self, c: ClassId) -> bool {
        self.rep.contains_key(&c)
    }

    pub fn compute(
        prog: &Program,
        syms: &Symbols,
        interner: &Interner,
        reach: &Reach,
        closure_anons: &FxMap<ClassId, usize>,
        cache: Option<&mut ClassCache>,
    ) -> Shared {
        let mut by_origin: FxMap<(u32, u32), Vec<ClassId>> = FxMap::default();
        let mut index: FxMap<ClassId, usize> = FxMap::default();
        for (i, tc) in prog.classes.iter().enumerate() {
            index.insert(tc.id, i);
            let info = syms.class(tc.id);
            if info.kind == ClassKind::Anon && reach.classes.get(tc.id.idx()).copied().unwrap_or(false) && !closure_anons.contains_key(&tc.id) {
                by_origin.entry(syms.place(info.file, info.def, info.span.start)).or_default().push(tc.id);
            }
        }
        let mut origins: Vec<((u32, u32), Vec<ClassId>)> = by_origin.into_iter().filter(|(_, cs)| cs.len() > 1).collect();
        origins.sort_by_key(|(o, _)| *o);
        let mut shared = Shared::default();
        let mut cache = cache;
        let (encoded, known) = match cache.as_deref_mut() {
            Some(c) => (std::mem::take(&mut c.encoded), std::mem::take(&mut c.known)),
            None => Default::default(),
        };
        let mut eq = Equiv { prog, syms, interner, index, encoded, known, memo: Some(Box::default()) };
        let mut taken: FxMap<String, ()> = FxMap::default();
        for (_, mut classes) in origins {
            classes.sort_by(|&a, &b| compare_classes(syms, interner, a, b));
            let mut partitions: Vec<Vec<ClassId>> = Vec::new();
            for c in classes {
                match partitions.iter_mut().find(|p| eq.same(p[0], c)) {
                    Some(p) => p.push(c),
                    None => partitions.push(vec![c]),
                }
            }
            for members in partitions.into_iter().filter(|p| p.len() > 1) {
                let rep = members[0];
                let info = syms.class(rep);
                let position = match syms.product_position(info.file, info.def) {
                    Some((token, offset)) => format!("{}_{}", crate::source::tag_text(token), offset),
                    None => prog.position(info.file, info.span.start),
                };
                let head = super::names::sanitize(&origin_head(interner.get(info.name), &position));
                let hash = eq.encoding(rep).map_or(0, |e| e.stable);
                let mut name = format!("{}$h{:08x}", head, hash as u32);
                let mut n = 2;
                while taken.contains_key(name.as_str()) {
                    name = format!("{}$h{:08x}${}", head, hash as u32, n);
                    n += 1;
                }
                taken.insert(name.clone(), ());
                for &m in &members[1..] {
                    shared.rep.insert(m, rep);
                }
                shared.groups.insert(rep, SharedClass { name });
            }
        }
        if let Some(cache) = cache {
            let Equiv { index, mut encoded, mut known, .. } = eq;
            encoded.retain(|c, _| index.contains_key(c));
            known.retain(|(a, b), _| index.contains_key(a) && index.contains_key(b));
            *cache = ClassCache { encoded, known };
        }
        shared
    }
}

/// What the encodings of anonymous classes gave in the builds of a watch session: a class's
/// encoding is a function of its body, and a class of a re-typed file is a class anew.
#[derive(Default)]
pub struct ClassCache {
    encoded: FxMap<ClassId, Option<Rc<Encoded>>>,
    known: FxMap<(ClassId, ClassId), bool>,
}

impl ClassCache {
    /// The bytes the cache holds.
    pub fn held(&self) -> usize {
        use crate::held::{array, table};
        table(&self.encoded) + self.encoded.values().flatten().map(|e| array(&e.exact) + array(&e.nested)).sum::<usize>() + table(&self.known)
    }
}

/// `Outer$$anon$<position>` for a class made at that position, `Outer` the named class around
/// it, without the positions of the sites that the names of the class and of the anonymous
/// classes around it go on with.
fn origin_head(full: &str, position: &str) -> String {
    let outer = full.find("$$anon$").map_or(full, |at| &full[..at]);
    format!("{}$$anon${}", outer, position)
}

/// Whether two classes are one shape: their encodings are equal and the classes made per
/// expansion that they create or name are, pairwise, one shape too.
struct Equiv<'a> {
    prog: &'a Program,
    syms: &'a Symbols,
    interner: &'a Interner,
    index: FxMap<ClassId, usize>,
    encoded: FxMap<ClassId, Option<Rc<Encoded>>>,
    known: FxMap<(ClassId, ClassId), bool>,
    memo: Option<Box<Memo>>,
}

pub(crate) struct Encoded {
    exact: Vec<u64>,
    pub stable: u64,
    nested: Vec<ClassId>,
}

impl<'a> Equiv<'a> {
    fn encoding(&mut self, c: ClassId) -> Option<Rc<Encoded>> {
        if let Some(e) = self.encoded.get(&c) {
            return e.clone();
        }
        let e = self.index.get(&c).copied().and_then(|i| {
            let mut enc = Enc::new(self.prog, self.syms, self.interner);
            enc.memo = self.memo.take();
            enc.restart_bindings();
            enc.class_body(&self.prog.classes[i]);
            self.memo = enc.memo.take();
            enc.finish()
        });
        let e = e.map(Rc::new);
        self.encoded.insert(c, e.clone());
        e
    }

    fn same(&mut self, a: ClassId, b: ClassId) -> bool {
        if a == b {
            return true;
        }
        if let Some(&r) = self.known.get(&(a, b)) {
            return r;
        }
        let (Some(x), Some(y)) = (self.encoding(a), self.encoding(b)) else { return false };
        if x.exact != y.exact || x.nested.len() != y.nested.len() {
            self.known.insert((a, b), false);
            return false;
        }
        // Assumed while the nested classes are compared, which may name these two again.
        self.known.insert((a, b), true);
        let r = x.nested.iter().zip(&y.nested).all(|(&p, &q)| self.same(p, q));
        self.known.insert((a, b), r);
        r
    }
}

const T_INT: u64 = 1;
const T_LONG: u64 = 2;
const T_DOUBLE: u64 = 3;
const T_BOOL: u64 = 4;
const T_CHAR: u64 = 5;
const T_STR: u64 = 6;
const T_UNIT: u64 = 7;
const T_LOCAL: u64 = 8;
const T_THIS: u64 = 9;
const T_SUPER: u64 = 10;
const T_STATIC: u64 = 11;
const T_MODULE: u64 = 12;
const T_FIELD: u64 = 13;
const T_CALL_STATIC: u64 = 14;
const T_CALL_METHOD: u64 = 15;
const T_CALL_CLOSURE: u64 = 16;
const T_NEW: u64 = 17;
const T_NEW_VIA: u64 = 18;
const T_LAMBDA: u64 = 19;
const T_IF: u64 = 20;
const T_WHILE: u64 = 21;
const T_BLOCK: u64 = 22;
const T_ASSIGN: u64 = 23;
const T_MATCH: u64 = 24;
const T_PRIM: u64 = 25;
const T_UNARY: u64 = 26;
const T_CONCAT: u64 = 27;
const T_TOSTR: u64 = 28;
const T_JS: u64 = 29;
const T_TYPE_TEST: u64 = 30;
const T_CLASS_OF: u64 = 31;
const T_SEQ: u64 = 32;
const T_ARRAY: u64 = 33;
const T_INDEX: u64 = 34;
const T_JS_IMPORT: u64 = 35;
const T_JS_GLOBAL: u64 = 36;
const T_JS_SELECT: u64 = 37;
const T_OBJ: u64 = 38;
const T_SPREAD: u64 = 39;
const T_RETURN: u64 = 40;
const T_NULL: u64 = 41;
const T_THROW: u64 = 42;
const T_TRY: u64 = 43;
const T_BIND: u64 = 44;
const T_SYM: u64 = 45;
const T_MEMBER: u64 = 46;
const T_CLASS: u64 = 47;
const T_SITE_CLASS: u64 = 48;
const T_NONE: u64 = 49;
const T_SOME: u64 = 50;
const T_PAT: u64 = 51;
const T_TEST: u64 = 52;
const T_STMT: u64 = 53;
const T_END: u64 = 54;
const T_HOLE: u64 = 55;
const T_CLOSURE: u64 = 56;
const T_NESTED: u64 = 57;
const T_HOLE_AGAIN: u64 = 58;
const T_CAST: u64 = 59;

/// An argument of an outlined expansion: where the call site's expression stands in its tree.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum HoleNode {
    Expr(TExprId),
    /// The test of a `TypeTest` expression, and of a pattern.
    TestExpr(TExprId),
    TestPat(TPatId),
}

/// How the shared function takes an argument: as the value, as a function it calls where the
/// expression stood (evaluated there, as often as there), or as the predicate of a type test.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HoleKind {
    Value,
    Thunk,
    Test,
}

#[derive(Clone, Copy, Debug)]
pub struct Hole {
    pub node: HoleNode,
    pub kind: HoleKind,
}

/// What encoding an expansion for outlining adds: the root, the definitions around it that it
/// must not call, and the arguments found.
/// What every encoding for outlining reads.
#[derive(Clone, Copy)]
pub(crate) struct OlCx<'a> {
    pub prog: &'a Program,
    pub syms: &'a Symbols,
    pub interner: &'a Interner,
    pub closure_anons: &'a FxMap<ClassId, usize>,
    pub shared: &'a Shared,
    /// Whether a watch session keeps the records: only then does an encoding note what it read.
    pub track: bool,
}

pub(crate) struct Ol<'a> {
    pub cx: OlCx<'a>,
    pub root: TExprId,
    pub holes: Vec<Hole>,
    /// The reads of a plain local already passed, each with the argument it reads again.
    pub aliases: Vec<(HoleNode, u32)>,
    /// The argument each plain local passed by value is, and each node passed.
    pub passed: FxMap<SymId, u32>,
    pub nodes_passed: FxMap<HoleNode, u32>,
    /// How many lambdas and local defs around the node being encoded belong to the root.
    pub lambdas: u32,
    /// The nodes of the shape, what outlining saves at each site.
    pub nodes: u32,
    /// The locals of the site the root reads, and whether a `return` stands in it.
    pub free_seen: Vec<SymId>,
    pub returns: bool,
    /// What a cached encoding depends on besides its tree: the expansions nested in it that it
    /// encoded, the representative it read for each class made per expansion, and whether each
    /// anonymous class it creates is written as a closure.
    pub children: Vec<TExprId>,
    pub reps: Vec<(ClassId, ClassId)>,
    pub closures: Vec<(ClassId, bool)>,
}

#[derive(Clone)]
pub struct Shape {
    /// Two hashes of the exact encoding, which stand for it in an enclosing expansion's: the
    /// encodings of one thread and another compare through them.
    pub keys: (u64, u64),
    pub stable: u64,
    pub holes: Rc<Vec<Hole>>,
    pub aliases: Rc<Vec<(HoleNode, u32)>>,
    pub nodes: u32,
}

/// What encodings share: per expansion encoded so far, the locals it reads that it does not
/// bind and whether a `return` stands in it (expansions are encoded innermost first, so that an
/// enclosing one finds those of the expansions nested in it here), and the stable keys of
/// names, symbols and classes.
#[derive(Default)]
pub struct Memo {
    pub free: FxMap<TExprId, Rc<Vec<SymId>>>,
    pub returns: FxMap<TExprId, bool>,
    /// The shape of each expansion that can be shared: an expansion around it is encoded as
    /// that shape and its arguments rather than as its whole tree again.
    pub shapes: FxMap<TExprId, Shape>,
    names: Vec<u64>,
    syms: Vec<u64>,
    classes: Vec<u64>,
    /// Per class, 1 when it is made per expansion, 2 when not, 0 before it is asked.
    site_classes: Vec<u8>,
    /// Per symbol, the encoding that bound it and its number there.
    bound: Vec<(u32, u32)>,
    stamps: u32,
    /// The entries of `bound` an expansion encoded inside another replaced, restored after it.
    undo: Vec<(SymId, (u32, u32))>,
    /// The expansions encoded so far, shared or not, as bits by expression.
    encoded: Vec<u64>,
    pub records: Vec<(TExprId, Record)>,
    /// The exact encodings met so far, one allocation per shape, by their first hash.
    keys: FxMap<u64, Vec<Arc<[u64]>>>,
    buf: Vec<u64>,
}

/// An expansion as encoded: its shape when it can be shared and what the encoding read (`Ol`),
/// which a watch rebuild checks before it takes the record again.
pub struct Record {
    pub shape: Option<RecordShape>,
    pub children: Vec<TExprId>,
    pub reps: Vec<(ClassId, ClassId)>,
    pub closures: Vec<(ClassId, bool)>,
}

pub struct RecordShape {
    pub key: Arc<[u64]>,
    pub hash: u64,
    pub stable: u64,
    pub nodes: u32,
    pub holes: Arc<[Hole]>,
    pub aliases: Arc<[(HoleNode, u32)]>,
}

/// Encodes the expansion `root` and those nested in it that it needs, and records them.
pub(crate) fn encode_root(cx: OlCx, slot: &mut Option<Box<Memo>>, root: TExprId) {
    let memo = slot.as_mut().unwrap();
    let (word, bit) = (root.idx() / 64, 1u64 << (root.idx() % 64));
    if memo.encoded.len() <= word {
        memo.encoded.resize(word + 1, 0);
    }
    if memo.encoded[word] & bit != 0 {
        return;
    }
    memo.encoded[word] |= bit;
    let callee = cx.prog.expansions[&root].callee;
    let undo = memo.undo.len();
    let mut enc = Enc::new(cx.prog, cx.syms, cx.interner);
    enc.memo = slot.take();
    enc.restart_bindings();
    let buf = std::mem::take(&mut enc.m().buf);
    enc.reuse(buf);
    enc.ol = Some(Ol {
        cx,
        root,
        holes: Vec::new(),
        aliases: Vec::new(),
        passed: FxMap::default(),
        nodes_passed: FxMap::default(),
        lambdas: 0,
        nodes: 0,
        free_seen: Vec::new(),
        returns: false,
        children: Vec::new(),
        reps: Vec::new(),
        closures: Vec::new(),
    });
    enc.callee(callee);
    enc.expr(root);
    enc.memoize_root();
    let ol = enc.ol.take().unwrap();
    let (_, stable, ok) = enc.result();
    let key = enc.take_exact();
    *slot = enc.memo.take();
    let memo = slot.as_mut().unwrap();
    while memo.undo.len() > undo {
        let (s, entry) = memo.undo.pop().unwrap();
        memo.bound[s.idx()] = entry;
    }
    let Ol { holes, aliases, nodes, children, reps, closures, .. } = ol;
    let shape = if ok {
        let keys = key_hashes(&key);
        memo.shapes.insert(root, Shape { keys, stable, holes: Rc::new(holes.clone()), aliases: Rc::new(aliases.clone()), nodes });
        let bucket = memo.keys.entry(keys.0).or_default();
        let key = match bucket.iter().find(|k| k[..] == key[..]) {
            Some(k) => {
                let k = k.clone();
                memo.buf = key;
                k
            }
            None => {
                let k: Arc<[u64]> = key.into();
                bucket.push(k.clone());
                k
            }
        };
        Some(RecordShape { key, hash: keys.0, stable, nodes, holes: holes.into(), aliases: aliases.into() })
    } else {
        memo.buf = key;
        None
    };
    memo.records.push((root, Record { shape, children, reps, closures }));
}

/// Two independent hashes of an exact encoding.
pub(crate) fn key_hashes(key: &[u64]) -> (u64, u64) {
    let (mut a, mut b) = (FxHasher::default(), FxHasher::default());
    b.write_u64(0x2545_f491_4f6c_dd1d);
    for &k in key {
        a.write_u64(k);
        b.write_u64(k.rotate_left(29) ^ 0x9e37_79b9_7f4a_7c15);
    }
    (a.finish(), b.finish())
}

/// Encodes the expansions of `roots` and those nested in them.
pub(crate) fn encode_roots(cx: OlCx, roots: &[TExprId]) -> Vec<(TExprId, Record)> {
    let mut memo: Option<Box<Memo>> = Some(Box::default());
    for &root in roots {
        encode_root(cx, &mut memo, root);
    }
    std::mem::take(&mut memo.unwrap().records)
}

/// The cached key at `i`, 0 when there is none yet.
#[inline]
fn cached(v: &[u64], i: usize) -> u64 {
    v.get(i).copied().unwrap_or(0)
}

#[inline]
fn cache(v: &mut Vec<u64>, i: usize, key: u64) -> u64 {
    if v.len() <= i {
        v.resize(i + 1, 0);
    }
    let key = key | 1;
    v[i] = key;
    key
}

/// Writes a class body or an expression as two streams: the exact one, whose equality is the
/// equality of the output the two would give (the program's own ids for what is shared, the
/// order of binding for the locals, the position among its members for a member of a class
/// made per expansion), and a stable hash over names and source positions alone, the same in
/// every build, which names what is shared.
pub(crate) struct Enc<'a> {
    prog: &'a Program,
    syms: &'a Symbols,
    interner: &'a Interner,
    exact: Vec<u64>,
    stable: FxHasher,
    /// How many locals the tree bound so far; which ones is `Memo::bound`, stamped with `stamp`.
    bound_count: u32,
    stamp: u32,
    nested: Vec<ClassId>,
    ok: bool,
    pub ol: Option<Ol<'a>>,
    /// Boxed so that an encoding hands it to the one it starts inside without copying it.
    memo: Option<Box<Memo>>,
}

impl<'a> Enc<'a> {
    pub fn new(prog: &'a Program, syms: &'a Symbols, interner: &'a Interner) -> Enc<'a> {
        Enc { prog, syms, interner, exact: Vec::new(), stable: FxHasher::default(), bound_count: 0, stamp: 1, nested: Vec::new(), ok: true, ol: None, memo: Some(Box::default()) }
    }

    /// The exact stream, the stable hash, and whether the encoded tree can be shared.
    pub fn result(&self) -> (&[u64], u64, bool) {
        (&self.exact, self.stable.finish(), self.ok)
    }

    /// Encodes into `buf`, whose allocation the next encoding may take back with `take_exact`.
    pub fn reuse(&mut self, mut buf: Vec<u64>) {
        buf.clear();
        self.exact = buf;
    }

    pub fn take_exact(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.exact)
    }

    /// The method whose expansion follows.
    pub fn callee(&mut self, s: SymId) {
        let key = self.sym_key(s);
        self.tok2(s.0 as u64, key);
    }

    pub fn finish(self) -> Option<Encoded> {
        self.ok.then(|| Encoded { exact: self.exact, stable: self.stable.finish(), nested: self.nested })
    }

    #[inline]
    fn tok(&mut self, x: u64) {
        self.exact.push(x);
        self.stable.write_u64(x);
    }

    #[inline]
    fn tok2(&mut self, exact: u64, stable: u64) {
        self.exact.push(exact);
        self.stable.write_u64(stable);
    }

    fn text(&mut self, s: &str) {
        let bytes = s.as_bytes();
        self.tok(bytes.len() as u64);
        for c in bytes.chunks(8) {
            let mut buf = [0u8; 8];
            buf[..c.len()].copy_from_slice(c);
            self.tok(u64::from_le_bytes(buf));
        }
    }

    fn str_hash(s: &str) -> u64 {
        let mut h = FxHasher::default();
        h.write(s.as_bytes());
        h.finish()
    }

    fn name_hash(&mut self, n: crate::intern::Name) -> u64 {
        let h = cached(&self.m().names, n.0 as usize);
        if h != 0 {
            return h;
        }
        let h = Self::str_hash(self.interner.get(n));
        cache(&mut self.m().names, n.0 as usize, h)
    }

    /// A class made at each expansion anew: a local or anonymous class, or a class nested in one.
    fn is_site_class(&mut self, c: ClassId) -> bool {
        match self.m().site_classes.get(c.idx()).copied().unwrap_or(0) {
            1 => return true,
            2 => return false,
            _ => {}
        }
        let site = match self.syms.class(c).owner {
            Owner::Local => true,
            Owner::Class(o) => o != c && self.is_site_class(o),
            Owner::Package(_) => false,
        };
        if self.m().site_classes.len() <= c.idx() {
            self.m().site_classes.resize(c.idx() + 1, 0);
        }
        self.m().site_classes[c.idx()] = if site { 1 } else { 2 };
        site
    }

    fn pkg_key(&mut self, p: PkgId) -> u64 {
        let syms = self.syms;
        let info = syms.pkg(p);
        let parent = info.parent.map_or(0, |q| self.pkg_key(q));
        parent.rotate_left(7) ^ self.name_hash(info.name)
    }

    fn owner_key(&mut self, o: Owner) -> u64 {
        match o {
            Owner::Package(p) => self.pkg_key(p),
            Owner::Class(c) => self.class_key(c),
            Owner::Local => 0x10ca1,
        }
    }

    fn class_key(&mut self, c: ClassId) -> u64 {
        let h = cached(&self.m().classes, c.idx());
        if h != 0 {
            return h;
        }
        let syms = self.syms;
        let info = syms.class(c);
        let mut h = self.owner_key(info.owner).rotate_left(11) ^ self.name_hash(info.name);
        if info.owner == Owner::Local {
            h ^= self.position_key(info.file, info.def, info.span.start).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        }
        cache(&mut self.m().classes, c.idx(), h)
    }

    fn sym_key(&mut self, s: SymId) -> u64 {
        let h = cached(&self.m().syms, s.idx());
        if h != 0 {
            return h;
        }
        let syms = self.syms;
        let info = syms.sym(s);
        let mut h = self.owner_key(info.owner).rotate_left(13) ^ self.name_hash(info.name);
        if matches!(info.owner, Owner::Local) {
            h ^= self.position_key(info.file, info.def, info.span.start).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        }
        cache(&mut self.m().syms, s.idx(), h)
    }

    fn origin(&self, c: ClassId) -> u64 {
        let info = self.syms.class(c);
        self.position_key(info.file, info.def, info.span.start)
    }

    /// A definition's position as a key: its file's tag and its start, or a product's
    /// definition's recorded token and place.
    fn position_key(&self, file: FileId, def: Option<crate::ast::DefId>, start: u32) -> u64 {
        let (tag, start) = match self.syms.product_position(file, def) {
            Some((token, offset)) => (token, offset),
            None => (self.prog.file_tags[file.0 as usize], start),
        };
        // A program file's tag has 32 bits, a library body's 64.
        if tag >> 32 == 0 {
            tag << 32 | start as u64
        } else {
            tag.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ start as u64
        }
    }

    #[inline]
    fn m(&mut self) -> &mut Memo {
        self.memo.as_mut().unwrap()
    }

    #[inline]
    fn mr(&self) -> &Memo {
        self.memo.as_ref().unwrap()
    }

    /// A fresh encoding: no local of an earlier one counts as bound.
    pub fn restart_bindings(&mut self) {
        self.m().stamps += 1;
        self.stamp = self.m().stamps;
        self.bound_count = 0;
    }

    #[inline]
    fn bound_at(&self, s: SymId) -> Option<u32> {
        match self.mr().bound.get(s.idx()) {
            Some(&(stamp, n)) if stamp == self.stamp => Some(n),
            _ => None,
        }
    }

    #[inline]
    fn is_bound(&self, s: SymId) -> bool {
        self.bound_at(s).is_some()
    }

    fn bind(&mut self, s: SymId) {
        let n = match self.bound_at(s) {
            Some(n) => n,
            None => {
                let n = self.bound_count;
                self.bound_count += 1;
                n
            }
        };
        if self.m().bound.len() <= s.idx() {
            self.m().bound.resize(s.idx() + 1, (0, 0));
        }
        let old = self.m().bound[s.idx()];
        if old.0 != self.stamp && old.0 != 0 {
            self.m().undo.push((s, old));
        }
        self.m().bound[s.idx()] = (self.stamp, n);
        let syms = self.syms;
        let info = syms.sym(s);
        let (name, kind, mods) = (self.name_hash(info.name), info.kind, info.mods);
        self.tok(T_BIND);
        self.tok(name);
        self.tok(sym_kind_tag(kind));
        self.tok(mods as u64);
    }

    fn sym(&mut self, s: SymId) {
        if let Some(n) = self.bound_at(s) {
            self.tok(T_LOCAL);
            self.tok(n as u64);
            return;
        }
        let info = self.syms.sym(s);
        if let Owner::Class(c) = info.owner {
            if self.is_site_class(c) {
                let at = self.syms.class(c).member_order.iter().position(|&m| m == s);
                if let Some(at) = at {
                    self.tok(T_MEMBER);
                    match &mut self.ol {
                        Some(ol) => {
                            let rep = ol.cx.shared.rep_of(c);
                            if ol.cx.track {
                                ol.reps.push((c, rep));
                            }
                            self.tok2(rep.0 as u64, self.origin(c));
                        }
                        None => {
                            self.tok(self.origin(c));
                            self.nested.push(c);
                        }
                    }
                    self.tok(at as u64);
                    return;
                }
            }
        }
        self.tok(T_SYM);
        let key = self.sym_key(s);
        self.tok2(s.0 as u64, key);
    }

    fn class(&mut self, c: ClassId) {
        if self.is_site_class(c) {
            self.tok(T_SITE_CLASS);
            match &mut self.ol {
                Some(ol) => {
                    let rep = ol.cx.shared.rep_of(c);
                    if ol.cx.track {
                        ol.reps.push((c, rep));
                    }
                    self.tok2(rep.0 as u64, self.origin(c));
                }
                None => {
                    self.tok(self.origin(c));
                    self.nested.push(c);
                }
            }
        } else {
            self.tok(T_CLASS);
            let key = self.class_key(c);
            self.tok2(c.0 as u64, key);
        }
    }

    fn opt_expr(&mut self, e: Option<TExprId>) {
        match e {
            Some(e) => {
                self.tok(T_SOME);
                self.expr(e);
            }
            None => self.tok(T_NONE),
        }
    }

    fn opt_sym(&mut self, s: Option<SymId>) {
        match s {
            Some(s) => {
                self.tok(T_SOME);
                self.sym(s);
            }
            None => self.tok(T_NONE),
        }
    }

    fn list(&mut self, l: crate::ast::ListRef) {
        self.tok(l.len as u64);
        let prog = self.prog;
        for &e in prog.expr_list(l) {
            self.expr(e);
        }
    }

    fn stmts(&mut self, l: crate::ast::ListRef) {
        let prog = self.prog;
        let stmts = &prog.stmts[l.range()];
        self.tok(T_STMT);
        self.tok(stmts.len() as u64);
        // Local defs may call each other before they are defined.
        for s in stmts {
            if let TStmt::Fun(f) = *s {
                self.bind(prog.funs[f.idx()].sym);
            }
        }
        for s in stmts {
            match *s {
                TStmt::Expr(e) => {
                    self.tok(1);
                    self.expr(e);
                }
                TStmt::Val(v, e) => {
                    self.tok(2);
                    self.expr(e);
                    self.bind(v);
                }
                TStmt::Fun(f) => {
                    self.tok(3);
                    self.enter_lambda();
                    self.fun(f);
                    self.leave_lambda();
                }
                TStmt::Pat(p, e) => {
                    self.tok(4);
                    self.expr(e);
                    self.pat(p);
                }
            }
        }
    }

    fn fun(&mut self, f: FunId) {
        let prog = self.prog;
        let fun = &prog.funs[f.idx()];
        self.tok(fun.params.len() as u64);
        for &p in &fun.params {
            self.bind(p);
        }
        for &d in &fun.defaults {
            self.opt_expr(d);
        }
        self.opt_expr(fun.body);
    }

    fn cases(&mut self, l: crate::ast::ListRef) {
        let prog = self.prog;
        let cases = &prog.cases[l.range()];
        self.tok(cases.len() as u64);
        for c in cases {
            self.pat(c.pat);
            self.opt_expr(c.guard);
            self.expr(c.body);
        }
    }

    fn pat(&mut self, p: TPatId) {
        self.tok(T_PAT);
        match self.prog.pats[p.idx()] {
            TPat::Wildcard => self.tok(1),
            TPat::Bind(s, inner) => {
                self.tok(2);
                self.bind(s);
                match inner {
                    Some(i) => {
                        self.tok(T_SOME);
                        self.pat(i);
                    }
                    None => self.tok(T_NONE),
                }
            }
            TPat::Test(t, _, inner) => {
                self.tok(3);
                if self.ol.is_some() && self.prog.leaf_tests.contains_key(&t) {
                    self.hole(HoleNode::TestPat(p), HoleKind::Test);
                } else {
                    self.test(t);
                }
                self.pat(inner);
            }
            TPat::Equals(e, strict) => {
                self.tok(4);
                self.tok(strict as u64);
                self.expr(e);
            }
            TPat::Class(c, _, fields, subs) => {
                self.tok(5);
                self.class(c);
                self.tok(fields.len as u64);
                let prog = self.prog;
                for &f in prog.sym_list(fields) {
                    self.sym(f);
                }
                self.pats(subs);
            }
            TPat::Alt(subs) => {
                self.tok(6);
                self.pats(subs);
            }
            TPat::Seq(items, rest) => {
                self.tok(7);
                self.pats(items);
                match rest {
                    Some(r) => {
                        self.tok(T_SOME);
                        self.pat(r);
                    }
                    None => self.tok(T_NONE),
                }
            }
            TPat::Unapply(s, call, inner) => {
                self.tok(8);
                self.bind(s);
                self.expr(call);
                self.pat(inner);
            }
        }
    }

    fn pats(&mut self, l: crate::ast::ListRef) {
        let prog = self.prog;
        let items = &prog.pat_lists[l.range()];
        self.tok(items.len() as u64);
        for &p in items {
            self.pat(p);
        }
    }

    fn test(&mut self, t: TestId) {
        self.tok(T_TEST);
        if self.prog.deferred_tests.contains_key(&t) {
            self.ok = false;
        }
        match self.prog.tests[t.idx()] {
            TypeTest::Class(c) => {
                self.tok(100);
                self.class(c);
            }
            TypeTest::Trait(c) => {
                self.tok(101);
                self.class(c);
            }
            TypeTest::Value(e) => {
                self.tok(102);
                self.expr(e);
            }
            TypeTest::Or(a, b) => {
                self.tok(103);
                self.test(a);
                self.test(b);
            }
            TypeTest::And(a, b) => {
                self.tok(104);
                self.test(a);
                self.test(b);
            }
            TypeTest::Function(n) => {
                self.tok(105);
                self.tok(n as u64);
            }
            other => {
                self.tok(106);
                self.tok(simple_test_tag(other));
            }
        }
    }

    /// `e` under the widenings that leave a JavaScript number as it is.
    fn through_same(&self, mut e: TExprId) -> TExprId {
        while let TExpr::Unary(op, a) = self.prog.expr(e) {
            if !op.same_number() {
                break;
            }
            e = a;
        }
        e
    }

    pub fn expr(&mut self, e: TExprId) {
        // A widening that is the same JavaScript number is no node of the shape: the operand is,
        // a hole included, as the emitter writes the operand (`Emitter::through_same`).
        let e = self.through_same(e);
        if self.ol.is_some() {
            if self.outline_hole(e) {
                return;
            }
            self.ol.as_mut().unwrap().nodes += 1;
        }
        match self.prog.expr(e) {
            TExpr::Int(v) => {
                self.tok(T_INT);
                self.tok(v as u32 as u64);
            }
            TExpr::Long(v) => {
                self.tok(T_LONG);
                self.tok(v as u64);
            }
            TExpr::Double(v) => {
                self.tok(T_DOUBLE);
                self.tok(v.to_bits());
            }
            TExpr::Bool(v) => {
                self.tok(T_BOOL);
                self.tok(v as u64);
            }
            TExpr::Char(v) => {
                self.tok(T_CHAR);
                self.tok(v as u64);
            }
            TExpr::Str(s) => {
                self.tok(T_STR);
                let prog = self.prog;
                self.text(&prog.strings[s.idx()]);
            }
            TExpr::Unit => self.tok(T_UNIT),
            TExpr::Null => self.tok(T_NULL),
            TExpr::Local(s) => {
                self.tok(T_LOCAL);
                self.sym(s);
            }
            TExpr::This => {
                self.tok(T_THIS);
                if self.ol.is_some() {
                    self.ok = false;
                }
            }
            TExpr::Super(target) => {
                self.tok(T_SUPER);
                if self.ol.is_some() {
                    self.ok = false;
                }
                match target {
                    SuperTarget::Chain => self.tok(0),
                    SuperTarget::Class(c) => {
                        self.tok(1);
                        self.class(c);
                    }
                    SuperTarget::Mixin(c) => {
                        self.tok(2);
                        self.class(c);
                    }
                }
            }
            TExpr::Static(s) => {
                self.tok(T_STATIC);
                self.sym(s);
            }
            TExpr::Module(c) => {
                self.tok(T_MODULE);
                self.class(c);
            }
            TExpr::Field(r, s) => {
                self.tok(T_FIELD);
                self.expr(r);
                self.sym(s);
            }
            TExpr::CallStatic(s, args) => {
                self.tok(T_CALL_STATIC);
                if self.ol.is_some() && !self.is_bound(s) && self.syms.sym(s).owner == Owner::Local {
                    self.ok = false;
                    self.ol.as_mut().unwrap().free_seen.push(s);
                }
                self.sym(s);
                self.list(args);
            }
            TExpr::CallMethod(r, s, args) => {
                self.tok(T_CALL_METHOD);
                self.expr(r);
                self.sym(s);
                self.list(args);
            }
            TExpr::CallClosure(f, args) => {
                self.tok(T_CALL_CLOSURE);
                self.expr(f);
                self.list(args);
            }
            TExpr::New(c, args) if self.writes_closure(c) => self.closure(c, args),
            TExpr::New(c, args) => {
                self.tok(T_NEW);
                if self.ol.is_some() && self.syms.class(c).kind == ClassKind::Anon {
                    self.check_captures(args);
                }
                self.class(c);
                self.list(args);
            }
            TExpr::NewVia(s, args) => {
                self.tok(T_NEW_VIA);
                self.sym(s);
                self.list(args);
            }
            TExpr::Lambda(params, body) => {
                self.tok(T_LAMBDA);
                let prog = self.prog;
                let ps = prog.sym_list(params);
                self.tok(ps.len() as u64);
                for &p in ps {
                    self.bind(p);
                }
                self.enter_lambda();
                self.expr(body);
                self.leave_lambda();
            }
            TExpr::If(c, t, els) => {
                self.tok(T_IF);
                self.expr(c);
                self.expr(t);
                self.opt_expr(els);
            }
            TExpr::While(c, b) => {
                self.tok(T_WHILE);
                self.expr(c);
                self.expr(b);
            }
            TExpr::Block(stmts, res) => {
                self.tok(T_BLOCK);
                self.stmts(stmts);
                self.expr(res);
            }
            TExpr::Assign(a, b) => {
                self.tok(T_ASSIGN);
                if let (Some(_), TExpr::Local(s)) = (&self.ol, self.prog.expr(a)) {
                    if !self.is_bound(s) {
                        self.ok = false;
                    }
                }
                self.expr(a);
                self.expr(b);
            }
            TExpr::Match(scrut, cases) => {
                self.tok(T_MATCH);
                self.expr(scrut);
                self.cases(cases);
            }
            TExpr::Prim(op, a, b) => {
                self.tok(T_PRIM);
                self.tok(op as u64);
                self.expr(a);
                self.expr(b);
            }
            TExpr::Unary(op, a) => {
                self.tok(T_UNARY);
                self.tok(op as u64);
                self.expr(a);
            }
            // (`UnOp::same_number` never reaches here: `through_same` above.)
            TExpr::StrConcat(l) => {
                self.tok(T_CONCAT);
                self.list(l);
            }
            TExpr::ToStr(a, conv) => {
                self.tok(T_TOSTR);
                self.tok(conv.bits() as u64);
                self.expr(a);
            }
            TExpr::Js(s, args) => {
                self.tok(T_JS);
                let prog = self.prog;
                self.text(&prog.strings[s.idx()]);
                self.list(args);
            }
            TExpr::TypeTest(a, t) => {
                self.tok(T_TYPE_TEST);
                self.expr(a);
                if self.ol.is_some() && self.prog.leaf_tests.contains_key(&t) {
                    self.hole(HoleNode::TestExpr(e), HoleKind::Test);
                } else {
                    self.test(t);
                }
            }
            TExpr::Cast(a, op, _) => {
                self.tok(T_CAST);
                self.expr(a);
                match op {
                    CastOp::Written => self.tok(0),
                    CastOp::Nothing => self.tok(1),
                    CastOp::Check(t, _) => {
                        self.tok(2);
                        self.test(t);
                    }
                    CastOp::Unbox(t, _) => {
                        self.tok(3);
                        self.test(t);
                    }
                }
            }
            TExpr::ClassOf(c) => {
                self.tok(T_CLASS_OF);
                self.class(c);
            }
            TExpr::SeqLit(l) => {
                self.tok(T_SEQ);
                self.list(l);
            }
            TExpr::ArrayLit(l) => {
                self.tok(T_ARRAY);
                self.list(l);
            }
            TExpr::Index(a, i) => {
                self.tok(T_INDEX);
                self.tok(i as u64);
                self.expr(a);
            }
            TExpr::JsImport(i) => {
                self.tok(T_JS_IMPORT);
                self.tok(i as u64);
            }
            TExpr::JsGlobal(n, guarded) => {
                self.tok(T_JS_GLOBAL);
                self.tok(guarded as u64);
                let interner = self.interner;
                self.text(interner.get(n));
            }
            TExpr::JsSelect(a, n) => {
                self.tok(T_JS_SELECT);
                let interner = self.interner;
                self.text(interner.get(n));
                self.expr(a);
            }
            TExpr::ObjLit(l) => {
                self.tok(T_OBJ);
                self.list(l);
            }
            TExpr::Spread(a) => {
                self.tok(T_SPREAD);
                self.expr(a);
            }
            TExpr::Return(a) => {
                self.tok(T_RETURN);
                if let Some(ol) = &mut self.ol {
                    ol.returns = true;
                    self.ok = false;
                }
                self.expr(a);
            }
            TExpr::Throw(a, wraps) => {
                self.tok(T_THROW);
                self.tok(wraps as u64);
                self.expr(a);
            }
            TExpr::Try(i) => {
                self.tok(T_TRY);
                let t = &self.prog.tries[i as usize];
                let (body, cases, finalizer, wraps) = (t.body, t.cases, t.finalizer, t.wraps);
                self.tok(wraps as u64);
                self.expr(body);
                self.cases(cases);
                self.opt_expr(finalizer);
            }
            TExpr::Splice(_) => unreachable!("a stored inline body is emitted nowhere"),
        }
    }

    /// The body of a class made per expansion: its parents and member names, its constructor
    /// and its methods, the captured locals bound in order.
    pub fn class_body(&mut self, tc: &TClass) {
        let syms = self.syms;
        let info = syms.class(tc.id);
        if !info.nested.is_empty() || info.member_order.iter().any(|&m| matches!(self.syms.sym(m).kind, SymKind::Object(_))) {
            self.ok = false;
        }
        self.tok(info.kind as u64);
        self.tok(info.mods as u64);
        self.tok(info.js as u64);
        self.tok(info.base_types.len() as u64);
        for &(b, _) in info.base_types.iter().skip(1) {
            self.class(b);
        }
        match info.superclass {
            Some(s) => {
                self.tok(T_SOME);
                self.class(s);
            }
            None => self.tok(T_NONE),
        }
        self.tok(info.member_order.len() as u64);
        for &m in &info.member_order {
            let s = syms.sym(m);
            let (name, kind, mods) = (self.name_hash(s.name), s.kind, s.mods);
            self.tok(name);
            self.tok(sym_kind_tag(kind));
            self.tok(mods as u64);
        }
        self.tok(tc.ctor_params.len() as u64);
        for &p in &tc.ctor_params {
            self.bind(p);
        }
        for &d in &tc.ctor_defaults {
            self.opt_expr(d);
        }
        self.tok(tc.captures as u64);
        self.tok(tc.inherited_case_members as u64);
        self.stmts(tc.parent_prelude);
        match tc.parent_args {
            Some(l) => {
                self.tok(T_SOME);
                self.list(l);
            }
            None => self.tok(T_NONE),
        }
        self.opt_sym(tc.parent_via);
        self.tok(tc.init.len() as u64);
        for init in &tc.init {
            match init {
                TInit::Field(s, e) => {
                    self.tok(1);
                    self.sym(*s);
                    self.expr(*e);
                }
                TInit::Stmt(e) => {
                    self.tok(2);
                    self.expr(*e);
                }
                TInit::Parent(c, call) => {
                    self.tok(3);
                    self.class(*c);
                    self.stmts(call.prelude);
                    self.list(call.args);
                    self.opt_sym(call.via);
                }
            }
        }
        for funs in [&tc.methods, &tc.ctors] {
            self.tok(funs.len() as u64);
            for &f in funs {
                let prog = self.prog;
                self.sym(prog.funs[f.idx()].sym);
                self.fun(f);
            }
        }
        self.tok(tc.forwarders.len() as u64);
        for &(a, b) in &tc.forwarders {
            self.sym(a);
            self.sym(b);
        }
        self.tok(tc.super_accessors.len() as u64);
        for acc in &tc.super_accessors {
            self.class(acc.of_trait);
            self.sym(acc.member);
            self.opt_sym(acc.target);
        }
        self.tok(tc.bridges.len() as u64);
        for &(a, b) in &tc.bridges {
            self.sym(a);
            self.sym(b);
        }
        self.tok(T_END);
    }
}

fn sym_kind_tag(k: SymKind) -> u64 {
    match k {
        SymKind::Val => 1,
        SymKind::Var => 2,
        SymKind::Def => 3,
        SymKind::Param => 4,
        SymKind::Object(_) => 5,
        SymKind::EnumValue(_) => 6,
        SymKind::Given => 7,
        SymKind::Overloaded(_) => 8,
    }
}

fn simple_test_tag(t: TypeTest) -> u64 {
    match t {
        TypeTest::Always => 1,
        TypeTest::Number => 2,
        TypeTest::Int => 3,
        TypeTest::Array => 4,
        TypeTest::Long => 5,
        TypeTest::Byte => 6,
        TypeTest::Short => 7,
        TypeTest::Float => 8,
        TypeTest::Str => 9,
        TypeTest::Char => 10,
        TypeTest::Bool => 11,
        TypeTest::Unit => 12,
        TypeTest::Null => 13,
        TypeTest::AnyRef => 14,
        TypeTest::AnyVal => 15,
        _ => 16,
    }
}

impl<'a> Enc<'a> {
    /// Whether `new c` is written as a closure, noted for the record of the expansion.
    fn writes_closure(&mut self, c: ClassId) -> bool {
        let syms = self.syms;
        let Some(ol) = &mut self.ol else { return false };
        if syms.class(c).kind != ClassKind::Anon {
            return false;
        }
        let closure = ol.cx.closure_anons.contains_key(&c);
        if ol.cx.track {
            ol.closures.push((c, closure));
        }
        closure
    }

    fn enter_lambda(&mut self) {
        if let Some(ol) = &mut self.ol {
            ol.lambdas += 1;
        }
    }

    fn leave_lambda(&mut self) {
        if let Some(ol) = &mut self.ol {
            ol.lambdas -= 1;
        }
    }

    fn hole(&mut self, node: HoleNode, kind: HoleKind) {
        let local = match (kind, node) {
            (HoleKind::Value, HoleNode::Expr(e)) => match self.prog.expr(e) {
                TExpr::Local(s) => Some(s),
                _ => None,
            },
            _ => None,
        };
        let ol = self.ol.as_mut().unwrap();
        // One node met twice (the typer shares a node between two places) is one argument too,
        // so that whether a shape has one is in its encoding rather than in one member.
        let again = ol.nodes_passed.get(&node).copied().or_else(|| local.and_then(|s| ol.passed.get(&s).copied()));
        if let Some(again) = again {
            ol.aliases.push((node, again));
            self.tok(T_HOLE_AGAIN);
            self.tok(again as u64);
            return;
        }
        if let Some(s) = local {
            ol.passed.insert(s, ol.holes.len() as u32);
        }
        ol.nodes_passed.insert(node, ol.holes.len() as u32);
        ol.holes.push(Hole { node, kind });
        self.tok(T_HOLE);
        self.tok(kind as u64);
    }

    /// A local that only its value stands for: a val or a parameter, neither lazy nor a `var`.
    fn is_plain_local(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        matches!(info.kind, SymKind::Val | SymKind::Param) && info.mods & crate::ast::mods::LAZY == 0
    }

    /// An anonymous class written as a class receives the locals it captures as they are: a
    /// `var` as a cell, a lazy val or a local def as its function, which an argument of the
    /// shared function could not pass on.
    fn check_captures(&mut self, args: crate::ast::ListRef) {
        let prog = self.prog;
        for &a in prog.expr_list(args) {
            if let TExpr::Local(s) = prog.expr(a) {
                if !self.is_bound(s) && !self.is_plain_local(s) {
                    self.ok = false;
                }
            }
        }
    }

    /// A class written as a closure where it is created: its `apply` as a lambda of the root,
    /// which reads the locals it captures as themselves.
    fn closure(&mut self, c: ClassId, args: crate::ast::ListRef) {
        let prog = self.prog;
        let ol = self.ol.as_ref().unwrap();
        let tc = &prog.classes[ol.cx.closure_anons[&c]];
        let plain = tc.ctor_params.iter().zip(prog.expr_list(args)).all(|(&s, &a)| matches!(prog.expr(a), TExpr::Local(t) if t == s));
        if !plain || tc.ctor_params.len() != args.len as usize {
            self.ok = false;
        }
        self.tok(T_CLOSURE);
        self.tok(self.origin(c));
        let fun = &prog.funs[tc.methods[0].idx()];
        self.tok(fun.params.len() as u64);
        for &p in &fun.params {
            self.bind(p);
        }
        self.enter_lambda();
        self.opt_expr(fun.body);
        self.leave_lambda();
    }

    /// Whether `e` is an argument of the outlined root: an expansion nested in it, a leaf, or a
    /// local of the site, none of which names a local of the root. Encodes it as such.
    fn outline_hole(&mut self, e: TExprId) -> bool {
        let prog = self.prog;
        let ol = self.ol.as_ref().unwrap();
        if e == ol.root {
            return false;
        }
        let nested = prog.is_expansion(e);
        if nested {
            let cx = ol.cx;
            if cx.track {
                self.ol.as_mut().unwrap().children.push(e);
            }
            encode_root(cx, &mut self.memo, e);
        }
        if nested || prog.is_leaf(e) {
            if self.is_closed(e) {
                if self.has_return(e) {
                    self.ok = false;
                    self.ol.as_mut().unwrap().returns = true;
                }
                let kind = if nested { HoleKind::Thunk } else { self.leaf_kind(e) };
                self.hole(HoleNode::Expr(e), kind);
                return true;
            }
            return nested && self.nested_shape(e);
        }
        if let TExpr::Local(s) = prog.expr(e) {
            if !self.is_bound(s) {
                self.ol.as_mut().unwrap().free_seen.push(s);
                let kind = self.leaf_kind(e);
                self.hole(HoleNode::Expr(e), kind);
                return true;
            }
        }
        false
    }

    /// An expansion nested in the root that reads the root's locals, as its shape and the
    /// arguments it takes, each encoded in the root's terms: what its whole tree would give.
    fn nested_shape(&mut self, e: TExprId) -> bool {
        let Some(shape) = self.m().shapes.get(&e).cloned() else { return false };
        self.tok(T_NESTED);
        self.tok2(shape.keys.0, shape.stable);
        self.exact.push(shape.keys.1);
        self.ol.as_mut().unwrap().nodes += shape.nodes;
        for h in shape.holes.iter() {
            match (h.kind, h.node) {
                (HoleKind::Test, node) => self.hole(node, HoleKind::Test),
                (kind, HoleNode::Expr(x)) => {
                    let delayed = kind == HoleKind::Thunk;
                    if delayed {
                        self.enter_lambda();
                    }
                    self.expr(x);
                    if delayed {
                        self.leave_lambda();
                    }
                }
                (_, node) => self.hole(node, HoleKind::Test),
            }
        }
        for &(node, n) in shape.aliases.iter() {
            match (shape.holes[n as usize].kind, node) {
                (HoleKind::Thunk, HoleNode::Expr(x)) => {
                    self.enter_lambda();
                    self.expr(x);
                    self.leave_lambda();
                }
                (HoleKind::Value, HoleNode::Expr(x)) => self.expr(x),
                (_, node) => self.hole(node, HoleKind::Test),
            }
        }
        true
    }

    /// A literal, or a plain local outside the root's lambdas, goes by value; anything else is
    /// evaluated where it stood.
    fn leaf_kind(&self, e: TExprId) -> HoleKind {
        let lambdas = self.ol.as_ref().map_or(0, |ol| ol.lambdas);
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Null => HoleKind::Value,
            TExpr::Local(s) if lambdas == 0 && self.is_plain_local(s) => HoleKind::Value,
            _ => HoleKind::Thunk,
        }
    }

    /// Whether no local that `e` reads is one the root binds; a closed subtree's locals are the
    /// root's reads from outside.
    fn is_closed(&mut self, e: TExprId) -> bool {
        let prog = self.prog;
        match prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit
            | TExpr::Null | TExpr::Module(_) | TExpr::Static(_) | TExpr::ClassOf(_) | TExpr::JsImport(_) | TExpr::JsGlobal(..) => return true,
            TExpr::Local(s) => {
                if self.is_bound(s) {
                    return false;
                }
                self.ol.as_mut().unwrap().free_seen.push(s);
                return true;
            }
            _ => {}
        }
        let free = match self.m().free.get(&e) {
            Some(f) => f.clone(),
            None => {
                let mut scan = Scan::default();
                self.scan(e, &mut scan);
                let mut used = scan.used;
                let mut bound = scan.bound;
                bound.sort_unstable_by_key(|s| s.0);
                used.retain(|s| bound.binary_search_by_key(&s.0, |b| b.0).is_err());
                used.sort_unstable_by_key(|s| s.0);
                used.dedup();
                if scan.returns {
                    self.m().returns.insert(e, true);
                }
                if used.is_empty() {
                    return true;
                }
                let f = Rc::new(used);
                self.m().free.insert(e, f.clone());
                f
            }
        };
        let closed = !free.iter().any(|s| self.is_bound(*s));
        if closed {
            self.ol.as_mut().unwrap().free_seen.extend(free.iter().copied());
        }
        closed
    }

    fn has_return(&self, e: TExprId) -> bool {
        self.mr().returns.get(&e).copied().unwrap_or(false)
    }

    /// The locals read, assigned or called under `e`, those it binds, and whether a `return`
    /// stands in it, the bodies of the closures it creates included.
    fn scan(&self, e: TExprId, out: &mut Scan) {
        let prog = self.prog;
        match prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit
            | TExpr::This | TExpr::Super(_) | TExpr::Static(_) | TExpr::Module(_) | TExpr::ClassOf(_) | TExpr::JsImport(_)
            | TExpr::JsGlobal(..) | TExpr::Null => {}
            TExpr::Local(s) => out.used.push(s),
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::Index(r, _) | TExpr::Spread(r) | TExpr::JsSelect(r, _)
            | TExpr::Throw(r, _) => self.scan(r, out),
            TExpr::Return(r) => {
                out.returns = true;
                self.scan(r, out);
            }
            TExpr::Lambda(ps, body) => {
                out.bound.extend_from_slice(prog.sym_list(ps));
                self.scan(body, out);
            }
            TExpr::TypeTest(r, t) | TExpr::Cast(r, CastOp::Check(t, _) | CastOp::Unbox(t, _), _) => {
                self.scan(r, out);
                self.scan_test(t, out);
            }
            TExpr::Cast(r, ..) => self.scan(r, out),
            TExpr::CallStatic(s, args) => {
                if self.syms.sym(s).owner == Owner::Local {
                    out.used.push(s);
                }
                self.scan_list(args, out);
            }
            TExpr::New(c, args) => {
                if let Some(&i) = self.ol.as_ref().unwrap().cx.closure_anons.get(&c) {
                    let fun = &prog.funs[prog.classes[i].methods[0].idx()];
                    out.bound.extend_from_slice(&fun.params);
                    if let Some(b) = fun.body {
                        self.scan(b, out);
                    }
                }
                self.scan_list(args, out);
            }
            TExpr::NewVia(_, args) | TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args)
            | TExpr::ObjLit(args) => self.scan_list(args, out),
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.scan(r, out);
                self.scan_list(args, out);
            }
            TExpr::If(c, t, els) => {
                self.scan(c, out);
                self.scan(t, out);
                if let Some(x) = els {
                    self.scan(x, out);
                }
            }
            TExpr::While(a, b) | TExpr::Prim(_, a, b) | TExpr::Assign(a, b) => {
                self.scan(a, out);
                self.scan(b, out);
            }
            TExpr::Block(stmts, res) => {
                for st in &prog.stmts[stmts.range()] {
                    match *st {
                        TStmt::Expr(x) => self.scan(x, out),
                        TStmt::Val(v, x) => {
                            out.bound.push(v);
                            self.scan(x, out);
                        }
                        TStmt::Fun(f) => {
                            let fun = &prog.funs[f.idx()];
                            out.bound.push(fun.sym);
                            out.bound.extend_from_slice(&fun.params);
                            for d in fun.defaults.iter().flatten() {
                                self.scan(*d, out);
                            }
                            if let Some(b) = fun.body {
                                self.scan(b, out);
                            }
                        }
                        TStmt::Pat(p, x) => {
                            self.scan_pat(p, out);
                            self.scan(x, out);
                        }
                    }
                }
                self.scan(res, out);
            }
            TExpr::Match(scrut, cases) => {
                self.scan(scrut, out);
                self.scan_cases(cases, out);
            }
            TExpr::Try(i) => {
                let t = &prog.tries[i as usize];
                self.scan(t.body, out);
                self.scan_cases(t.cases, out);
                if let Some(f) = t.finalizer {
                    self.scan(f, out);
                }
            }
            TExpr::Splice(_) => unreachable!("a stored inline body is emitted nowhere"),
        }
    }

    fn scan_list(&self, l: crate::ast::ListRef, out: &mut Scan) {
        for &x in self.prog.expr_list(l) {
            self.scan(x, out);
        }
    }

    fn scan_cases(&self, l: crate::ast::ListRef, out: &mut Scan) {
        for c in &self.prog.cases[l.range()] {
            self.scan_pat(c.pat, out);
            if let Some(g) = c.guard {
                self.scan(g, out);
            }
            self.scan(c.body, out);
        }
    }

    fn scan_pat(&self, p: TPatId, out: &mut Scan) {
        let prog = self.prog;
        match prog.pats[p.idx()] {
            TPat::Wildcard => {}
            TPat::Bind(s, inner) => {
                out.bound.push(s);
                if let Some(i) = inner {
                    self.scan_pat(i, out);
                }
            }
            TPat::Test(t, _, inner) => {
                self.scan_test(t, out);
                self.scan_pat(inner, out);
            }
            TPat::Equals(e, _) => self.scan(e, out),
            TPat::Class(_, _, _, subs) | TPat::Alt(subs) => {
                for &q in &prog.pat_lists[subs.range()] {
                    self.scan_pat(q, out);
                }
            }
            TPat::Seq(items, rest) => {
                for &q in &prog.pat_lists[items.range()] {
                    self.scan_pat(q, out);
                }
                if let Some(r) = rest {
                    self.scan_pat(r, out);
                }
            }
            TPat::Unapply(s, call, inner) => {
                out.bound.push(s);
                self.scan(call, out);
                self.scan_pat(inner, out);
            }
        }
    }

    fn scan_test(&self, t: TestId, out: &mut Scan) {
        match self.prog.tests[t.idx()] {
            TypeTest::Value(e) => self.scan(e, out),
            TypeTest::Or(a, b) | TypeTest::And(a, b) => {
                self.scan_test(a, out);
                self.scan_test(b, out);
            }
            _ => {}
        }
    }

    /// Records what the root just encoded reads from outside and whether it returns, for the
    /// expansions around it.
    pub fn memoize_root(&mut self) {
        let ol = self.ol.as_mut().unwrap();
        let mut free = std::mem::take(&mut ol.free_seen);
        let (root, returns) = (ol.root, ol.returns);
        free.sort_unstable_by_key(|s| s.0);
        free.dedup();
        self.m().free.insert(root, Rc::new(free));
        self.m().returns.insert(root, returns);
    }
}

#[derive(Default)]
struct Scan {
    used: Vec<SymId>,
    bound: Vec<SymId>,
    returns: bool,
}
