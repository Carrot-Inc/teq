//! Identity hashes as content hashes: in a compile-time run,
//! an object's, a function's, an array's, a map's or a trie's identity hash is a function of its
//! contents when it is first asked, computed once and kept on the value for its life, as the JVM
//! keeps an identity hash whatever the object becomes. The contents are read to a bounded depth, in a
//! canonical order, never through another value's kept hash and never by running the program's
//! code, so that the same contents hash alike on every worker, in a fresh build and in a session,
//! whatever the heap or the order of the runs.

use super::value::*;
use super::*;
use crate::ast::mods;
use crate::tir::TExpr;

/// How far a content hash reads: the references to `depth` below the hashed value (beyond it a
/// reference is hashed by its class alone), `elements` of a container (an array, a map, a trie
/// node, a closure's captures; past them a container is hashed by its class, its size and its
/// first `elements` in canonical order), `nodes` values in all (past them by their class
/// alone) and `bytes` of a string, with its length.
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub depth: u32,
    pub elements: usize,
    pub nodes: u32,
    pub bytes: usize,
}

const BOUNDS: Bounds = Bounds { depth: 4, elements: 16, nodes: 256, bytes: 256 };

/// The bounds in force: `TEQ_HASH_BOUNDS=<depth>,<elements>,<nodes>,<bytes>` replaces them for
/// the census's sweep of the bounds, a diagnostic.
pub fn bounds() -> Bounds {
    static B: std::sync::OnceLock<Bounds> = std::sync::OnceLock::new();
    *B.get_or_init(|| {
        let Ok(s) = std::env::var("TEQ_HASH_BOUNDS") else { return BOUNDS };
        let n: Vec<usize> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
        match n[..] {
            [d, e, k, b] => Bounds { depth: d as u32, elements: e, nodes: k as u32, bytes: b },
            _ => BOUNDS,
        }
    })
}

/// Why a walk hashed a value by its class alone, or a container by its first elements.
#[derive(Clone, Copy)]
pub enum Cut {
    Depth = 0,
    Cycle = 1,
    Nodes = 2,
    Elements = 3,
}

const CUT_MARKS: [u64; 4] = [0x6465_7074_6800_0001, 0x6379_636c_6500_0002, 0x6e6f_6465_7300_0003, 0x656c_656d_7300_0004];

/// The kinds of value whose identity hash is a content hash, as the census counts them.
#[derive(Clone, Copy)]
pub enum Kind {
    Object = 0,
    Function = 1,
    Array = 2,
    Map = 3,
    Trie = 4,
}

struct Walk {
    bounds: Bounds,
    nodes: u32,
    /// The containers on the way from the hashed value, by address: a cycle's mark. Nothing of
    /// the address enters a hash.
    path: Vec<usize>,
    cuts: [u32; 4],
}

impl Walk {
    fn cut(&mut self, why: Cut, class: u64) -> u64 {
        self.cuts[why as usize] += 1;
        mix(class, CUT_MARKS[why as usize])
    }
}

#[inline]
fn mix(h: u64, x: u64) -> u64 {
    (h.rotate_left(5) ^ x).wrapping_mul(0x517c_c1b7_2722_0a95)
}

/// MurmurHash3's finaliser, so that every bit of the digest reaches the 31 the hash keeps.
fn finish(mut h: u64) -> u64 {
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
}

fn bytes_digest(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &x| (h ^ x as u64).wrapping_mul(0x0100_0000_01b3))
}

const T_UNIT: u64 = 1;
const T_NULL: u64 = 2;
const T_BOOL: u64 = 3;
const T_INT: u64 = 4;
const T_LONG: u64 = 5;
const T_DOUBLE: u64 = 6;
const T_FLOAT: u64 = 7;
const T_BYTE: u64 = 8;
const T_SHORT: u64 = 9;
const T_CHAR: u64 = 10;
const T_STR: u64 = 11;
const T_OBJ: u64 = 12;
const T_FUN: u64 = 13;
const T_ARRAY: u64 = 14;
const T_MAP: u64 = 15;
const T_TRIE: u64 = 16;
const T_CLASS: u64 = 17;
const T_MATCH: u64 = 18;
const T_TREE: u64 = 19;
const T_TYPE: u64 = 20;
const T_SYM: u64 = 21;
const T_POS: u64 = 22;
const T_SRC: u64 = 23;
const T_ABSENT: u64 = 24;
const T_FRAME: u64 = 25;
const T_BUILTIN: u64 = 26;

/// The 31 bits of an identity hash from a digest, never 0, which marks a hash not yet asked.
fn to_hash(d: u64) -> i32 {
    let h = finish(d);
    match ((h ^ (h >> 32)) as u32 & 0x7fff_ffff) as i32 {
        0 => 1,
        h => h,
    }
}

impl<'a, 't> Interp<'a, 't> {
    /// The identity hash of a value: its kept one, or its contents' now, kept.
    pub(super) fn identity_hash_of(&mut self, v: &Value) -> i32 {
        let (cell, kind) = match v {
            Value::Obj(o) => (&o.hash, Kind::Object),
            Value::Fun(c) => (&c.hash, Kind::Function),
            Value::Array(a) => (&a.hash, Kind::Array),
            Value::Map(m) => (&m.hash, Kind::Map),
            Value::Trie(t) => (&t.hash, Kind::Trie),
            other => return prim_hash(other).unwrap_or(0),
        };
        let mut h = cell.get();
        let computed = h == 0;
        if computed {
            h = if self.content_hashes { self.content_hash(v, kind) } else { self.next_identity() };
            cell.set(h);
        }
        if watching() {
            hash_asked(computed);
        }
        if crate::measure::on() {
            crate::measure::identity_requested();
            if watching() {
                census_collision(v, kind, h, self);
            }
        }
        h
    }

    pub(super) fn identity_hash(&mut self, o: &Rc<Object>) -> i32 {
        self.identity_hash_of(&Value::Obj(o.clone()))
    }

    /// The next number of the run's sequence, never 0.
    fn next_identity(&mut self) -> i32 {
        self.next_hash = self.next_hash.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
        if self.next_hash == 0 {
            self.next_hash = 1;
        }
        self.next_hash
    }

    fn content_hash(&mut self, v: &Value, kind: Kind) -> i32 {
        let clock = crate::measure::on().then(std::time::Instant::now);
        let mut w = Walk { bounds: bounds(), nodes: 0, path: Vec::new(), cuts: [0; 4] };
        let h = to_hash(self.digest(&mut w, v, 0));
        if let Some(t) = clock {
            crate::measure::identity_computed(kind, w.nodes, w.cuts, t.elapsed().as_nanos() as u64);
        }
        h
    }

    fn digest(&mut self, w: &mut Walk, v: &Value, depth: u32) -> u64 {
        w.nodes += 1;
        match v {
            Value::Unit => T_UNIT,
            Value::Null => T_NULL,
            Value::Absent => T_ABSENT,
            Value::Bool(b) => mix(T_BOOL, *b as u64),
            Value::Int(i) => mix(T_INT, *i as u32 as u64),
            Value::Long(l) => mix(T_LONG, *l as u64),
            Value::Double(d) => mix(T_DOUBLE, d.to_bits()),
            Value::Float(f) => mix(T_FLOAT, f.to_bits() as u64),
            Value::Byte(b) => mix(T_BYTE, *b as u8 as u64),
            Value::Short(s) => mix(T_SHORT, *s as u16 as u64),
            Value::Char(c) => mix(T_CHAR, *c as u64),
            Value::Str(s) => self.str_digest(w, s),
            Value::Obj(o) => {
                let class = self.class_key(o.class);
                self.reference(w, Rc::as_ptr(o) as usize, mix(T_OBJ, class), depth, |it, w| it.object_digest(w, o, class, depth))
            }
            Value::Fun(c) => {
                let class = mix(T_FUN, self.fun_key(c));
                self.reference(w, Rc::as_ptr(c) as usize, class, depth, |it, w| it.closure_digest(w, c, class, depth))
            }
            Value::Array(a) => self.reference(w, Rc::as_ptr(a) as usize, T_ARRAY, depth, |it, w| {
                let (len, items) = {
                    let items = a.borrow();
                    (items.len(), items[..items.len().min(w.bounds.elements)].to_vec())
                };
                it.sequence_digest(w, T_ARRAY, len, &items, depth)
            }),
            Value::Map(m) => self.reference(w, Rc::as_ptr(m) as usize, T_MAP, depth, |it, w| it.map_digest(w, m, depth)),
            Value::Trie(t) => self.reference(w, Rc::as_ptr(t) as usize, T_TRIE, depth, |it, w| it.trie_digest(w, t, depth)),
            Value::Class(c) => mix(T_CLASS, bytes_digest(c.qname.as_bytes())),
            Value::Match(m) => {
                let mut h = mix(T_MATCH, self.str_digest(w, &m.source));
                for g in &m.groups {
                    h = mix(h, g.map_or(u64::MAX, |(a, b)| (a as u64) << 32 | b as u64));
                }
                h
            }
            Value::Tree(t) => mix(T_TREE, self.tree_key(*t)),
            Value::Type(_) => T_TYPE,
            Value::Sym(s) => mix(T_SYM, self.sym_key(*s)),
            Value::Pos(f, s, e) => mix(mix(mix(T_POS, self.file_key(*f)), *s as u64), *e as u64),
            Value::Src(f) => mix(T_SRC, self.file_key(*f)),
        }
    }

    /// A reference's contents through `f`, or its class alone where a bound or a cycle cuts it.
    fn reference(&mut self, w: &mut Walk, at: usize, class: u64, depth: u32, f: impl FnOnce(&mut Self, &mut Walk) -> u64) -> u64 {
        if depth > w.bounds.depth {
            return w.cut(Cut::Depth, class);
        }
        if w.nodes > w.bounds.nodes {
            return w.cut(Cut::Nodes, class);
        }
        if w.path.contains(&at) {
            return w.cut(Cut::Cycle, class);
        }
        w.path.push(at);
        let h = f(self, w);
        w.path.pop();
        h
    }

    fn str_digest(&mut self, w: &mut Walk, s: &str) -> u64 {
        let b = s.as_bytes();
        mix(mix(T_STR, b.len() as u64), bytes_digest(&b[..b.len().min(w.bounds.bytes)]))
    }

    /// An object: its class, its enum name and ordinal, its fields in the order of their names
    /// (a layout's slots are numbered in the order an interpreter first met them), a lazy val's
    /// slot left out whether or not it was read, and the scope an anonymous class captured.
    fn object_digest(&mut self, w: &mut Walk, o: &Rc<Object>, class: u64, depth: u32) -> u64 {
        let mut h = class;
        if let Some(n) = &o.name {
            h = mix(mix(h, bytes_digest(n.as_bytes())), o.ordinal.get() as u32 as u64);
        }
        let mut fields: Vec<(u64, u32)> = {
            let values = o.fields.borrow();
            let present = |slot: u32| values.get(slot as usize).is_some_and(|v| !matches!(v, Value::Absent));
            self.layout_entries(o.class).into_iter().filter(|&(_, slot)| present(slot)).map(|(key, slot)| (self.field_digest(key), slot)).collect()
        };
        fields.sort_unstable_by_key(|f| f.0);
        h = mix(h, fields.len() as u64);
        for (i, &(key, slot)) in fields.iter().enumerate() {
            // Past the node budget, the fields left count by their number alone.
            if w.nodes > w.bounds.nodes {
                h = mix(w.cut(Cut::Nodes, h), (fields.len() - i) as u64);
                break;
            }
            let v = o.fields.borrow()[slot as usize].clone();
            let d = self.digest(w, &v, depth + 1);
            h = mix(mix(h, key), d);
        }
        if let Some(env) = &o.env {
            h = mix(h, self.frame_digest(w, env, depth + 1));
        }
        h
    }

    /// A closure: its definition's place in the source, the receiver, the class it runs in and
    /// the scope it captured; a `{ case ... }` literal its two functions.
    fn closure_digest(&mut self, w: &mut Walk, c: &Rc<Closure>, class: u64, depth: u32) -> u64 {
        let mut h = class;
        if let ClosureKind::Partial(a, b) = &c.kind {
            let da = self.digest(w, a, depth + 1);
            let db = self.digest(w, b, depth + 1);
            return mix(mix(h, da), db);
        }
        if matches!(c.kind, ClosureKind::Miss | ClosureKind::MatchError | ClosureKind::Builtin(_)) {
            return h;
        }
        if let Some(k) = c.class {
            h = mix(h, self.class_key(k));
        }
        let this = self.digest(w, &c.this, depth + 1);
        h = mix(h, this);
        mix(h, self.frame_digest(w, &c.env, depth + 1))
    }

    /// A scope: the values of its frames, innermost first and each in the order they were
    /// bound, to the element bound; a lazy local left out whether or not it was read.
    fn frame_digest(&mut self, w: &mut Walk, f: &Rc<Frame>, depth: u32) -> u64 {
        if depth > w.bounds.depth {
            return w.cut(Cut::Depth, T_FRAME);
        }
        let mut values = Vec::new();
        let mut size = 0usize;
        let mut at = Some(f.clone());
        while let Some(frame) = at {
            for (s, v) in frame.vals.borrow().iter() {
                if self.syms().sym(*s).mods & mods::LAZY != 0 || matches!(v, Value::Fun(c) if matches!(c.kind, ClosureKind::Lazy(_))) {
                    continue;
                }
                size += 1;
                if values.len() < w.bounds.elements {
                    values.push(v.clone());
                }
            }
            at = frame.parent.clone();
        }
        if size > values.len() {
            w.cuts[Cut::Elements as usize] += 1;
        }
        let mut h = mix(T_FRAME, size as u64);
        for v in &values {
            let d = self.digest(w, v, depth);
            h = mix(h, d);
        }
        h
    }

    /// An array's items in their order, the first `elements` of a longer one, with its length.
    fn sequence_digest(&mut self, w: &mut Walk, class: u64, len: usize, first: &[Value], depth: u32) -> u64 {
        if first.len() < len {
            w.cuts[Cut::Elements as usize] += 1;
        }
        let mut h = mix(class, len as u64);
        for v in first {
            let d = self.digest(w, v, depth + 1);
            h = mix(h, d);
        }
        h
    }

    /// A map's entries in the canonical order: by the hash the map keeps for the key, then by
    /// the key's contents, then by the value's, entries alike in all three interchangeable; the
    /// insertion order and the holes left out. A larger map than the element bound is hashed by
    /// its size and its first entries in that order, chosen without ordering the rest.
    fn map_digest(&mut self, w: &mut Walk, m: &Rc<MapCell>, depth: u32) -> u64 {
        let (live, mut keyed) = {
            let map = m.borrow();
            let mut keyed: Vec<(i32, u32)> = Vec::with_capacity(map.live);
            for (&h, bucket) in map.index.iter() {
                keyed.extend(bucket.iter().filter(|&&i| matches!(map.entries.get(i as usize), Some(Some(_)))).map(|&i| (h, i)));
            }
            (map.live, keyed)
        };
        let chosen = self.entries_in_order(w, &mut keyed, |i| m.borrow().entries[i as usize].clone().unwrap_or((Value::Unit, Value::Unit)), depth);
        let mut h = mix(T_MAP, live as u64);
        for (kh, kd, vd) in chosen {
            h = mix(mix(mix(h, kh as u32 as u64), kd), vd);
        }
        h
    }

    /// The first `elements` of `keyed` (a hash and where `entry` finds the key and the value) in
    /// the canonical order of `map_digest`, each as its hash and its key's and value's digests.
    /// The entries of one hash share what the node budget leaves, in equal parts read from the
    /// same count, so that which of them comes first decides nothing; a group of more of them
    /// than the element bound is hashed by its hash and its size alone.
    fn entries_in_order(&mut self, w: &mut Walk, keyed: &mut Vec<(i32, u32)>, entry: impl Fn(u32) -> (Value, Value), depth: u32) -> Vec<(i32, u64, u64)> {
        let bound = w.bounds.elements;
        if keyed.len() > bound {
            w.cuts[Cut::Elements as usize] += 1;
            if bound == 0 {
                return Vec::new();
            }
            keyed.select_nth_unstable_by_key(bound - 1, |e| e.0);
            let last = keyed[bound - 1].0;
            keyed.retain(|e| e.0 <= last);
        }
        keyed.sort_unstable_by_key(|e| e.0);
        let mut out = Vec::with_capacity(keyed.len().min(bound));
        let mut i = 0;
        while i < keyed.len() {
            let h = keyed[i].0;
            let n = keyed[i..].iter().take_while(|e| e.0 == h).count();
            if n > bound {
                w.cuts[Cut::Elements as usize] += 1;
                out.push((h, CUT_MARKS[Cut::Elements as usize], n as u64));
            } else {
                let (start, cap) = (w.nodes, w.bounds.nodes);
                let share = cap.saturating_sub(start) / n as u32;
                let mut used = 0;
                let mut group = Vec::with_capacity(n);
                for &(_, at) in &keyed[i..i + n] {
                    let (k, v) = entry(at);
                    w.nodes = start;
                    w.bounds.nodes = start + share;
                    let kd = self.digest(w, &k, depth + 1);
                    let vd = self.digest(w, &v, depth + 1);
                    used += w.nodes - start;
                    group.push((h, kd, vd));
                }
                w.bounds.nodes = cap;
                w.nodes = start + used;
                group.sort_unstable();
                out.extend(group);
            }
            i += n;
        }
        out.truncate(bound);
        out
    }

    /// A trie node: its keys with their hashes in the order of their bits, the ordinals left
    /// out, and its sub-nodes; a node of keys whose hashes are equal (past the last bits) holds
    /// them in their insertion order, which is ordered as a map's entries are.
    fn trie_digest(&mut self, w: &mut Walk, t: &Rc<Trie>, depth: u32) -> u64 {
        let data = t.data_map.count_ones() as usize;
        let subs = t.node_map.count_ones() as usize;
        let mut h = mix(mix(T_TRIE, t.data_map as u32 as u64), t.node_map as u32 as u64);
        if t.data_map == 0 && t.node_map == 0 {
            let mut keyed: Vec<(i32, u32)> = t.content.chunks(3).enumerate().filter_map(|(i, c)| match c {
                [_, Value::Int(hash), _] => Some((*hash, 3 * i as u32)),
                _ => None,
            }).collect();
            h = mix(h, keyed.len() as u64);
            for (kh, kd, _) in self.entries_in_order(w, &mut keyed, |at| (t.content[at as usize].clone(), Value::Unit), depth) {
                h = mix(mix(h, kh as u32 as u64), kd);
            }
            return h;
        }
        let n = data + subs;
        if n > w.bounds.elements {
            w.cuts[Cut::Elements as usize] += 1;
        }
        let mut taken = 0;
        for i in 0..data.min(t.content.len() / 3) {
            if taken == w.bounds.elements {
                break;
            }
            taken += 1;
            let kd = self.digest(w, &t.content[3 * i], depth + 1);
            h = mix(mix(h, kd), match t.content[3 * i + 1] { Value::Int(x) => x as u32 as u64, _ => 0 });
        }
        for s in t.content.iter().skip(3 * data).take(subs) {
            if taken == w.bounds.elements {
                break;
            }
            taken += 1;
            let d = self.digest(w, s, depth + 1);
            h = mix(h, d);
        }
        h
    }

    /// The fields of class `c`'s layout as their keys and slots, the lazy vals' left out.
    fn layout_entries(&self, c: ClassId) -> Vec<(FieldKey, u32)> {
        let Some(Some(layout)) = self.layouts.get(c.idx()) else { return Vec::new() };
        let lazy = self.lazy_slots.get(c.idx()).and_then(|l| l.as_deref());
        layout.iter().filter(|(k, _)| !lazy.is_some_and(|l| l.contains_key(k))).map(|(&k, &s)| (k, s)).collect()
    }

    /// A field's key as its name and, for a private trait field, its class's key: never the
    /// ids, which the interner and the merge number by the order of the workers.
    fn field_digest(&self, key: FieldKey) -> u64 {
        let h = bytes_digest(self.name(key.0).as_bytes());
        match key.1 {
            0 => h,
            tag => mix(h, self.class_key(ClassId(tag - 1))),
        }
    }

    /// A class by its place: its path through the packages and the classes that own it, and
    /// for a local class its file and span.
    pub(super) fn class_key(&self, c: ClassId) -> u64 {
        let info = self.syms().class(c);
        let name = bytes_digest(self.name(info.name).as_bytes());
        match info.owner {
            Owner::Package(p) => mix(self.package_key(p), name),
            Owner::Class(o) => mix(mix(self.class_key(o), 0x2e), name),
            Owner::Local => mix(mix(mix(self.file_key(info.file), info.span.start as u64), info.span.end as u64), name),
        }
    }

    fn package_key(&self, p: crate::types::PkgId) -> u64 {
        let info = self.syms().pkg(p);
        let name = bytes_digest(self.name(info.name).as_bytes());
        match info.parent {
            Some(parent) => mix(self.package_key(parent), name),
            None => name,
        }
    }

    fn file_key(&self, f: FileId) -> u64 {
        bytes_digest(self.typer.source(f).key.as_bytes())
    }

    fn span_key(&self, place: Option<(FileId, crate::source::Span)>) -> u64 {
        match place {
            Some((f, s)) => mix(mix(self.file_key(f), s.start as u64), s.end as u64),
            None => 0,
        }
    }

    fn sym_place(&self, s: SymId) -> u64 {
        let info = self.syms().sym(s);
        mix(mix(self.file_key(info.file), info.span.start as u64), bytes_digest(self.name(info.name).as_bytes()))
    }

    /// A closure's definition by its place in the source: a lambda's and a lazy local's
    /// expression, a local def's symbol.
    fn fun_key(&self, c: &Closure) -> u64 {
        match &c.kind {
            ClosureKind::Lambda(params, e) => match self.prog().span_of(*e) {
                Some(place) => self.span_key(Some(place)),
                None if params.len > 0 => self.sym_place(self.prog().sym_lists[params.start as usize]),
                None => self.expr_shape(*e),
            },
            ClosureKind::Lazy(e) => match self.prog().span_of(*e) {
                Some(place) => self.span_key(Some(place)),
                None => self.expr_shape(*e),
            },
            ClosureKind::LocalDef(f) => self.sym_place(self.prog().funs[f.idx()].sym),
            ClosureKind::Partial(..) => 1,
            ClosureKind::Miss => 2,
            ClosureKind::MatchError => 3,
            ClosureKind::Builtin(_) => T_BUILTIN,
        }
    }

    /// What an expression without a recorded place is, as far as it says without ids.
    fn expr_shape(&self, e: TExprId) -> u64 {
        match self.prog().exprs[e.idx()] {
            TExpr::Lambda(params, _) => mix(0x4c, params.len as u64),
            _ => 0x45,
        }
    }

    fn tree_key(&self, t: TreeRef) -> u64 {
        match t {
            TreeRef::Expr(e) => mix(1, self.span_key(self.prog().span_of(e))),
            TreeRef::Class(c) | TreeRef::Ctor(c) => mix(2, self.class_key(c)),
            TreeRef::Def(s) => mix(3, self.sym_place(s)),
            _ => 4,
        }
    }

    fn sym_key(&self, s: SymRef) -> u64 {
        match s {
            SymRef::Term(s) => mix(1, self.sym_place(s)),
            SymRef::Class(c) => mix(2, self.class_key(c)),
            SymRef::Ctor(c) => mix(3, self.class_key(c)),
            SymRef::Default(c, i) => mix(mix(4, self.class_key(c)), i as u64),
            SymRef::Pkg(p) | SymRef::PkgClass(p) => mix(5, self.package_key(p)),
            SymRef::FilePackage(f) => mix(6, self.file_key(f)),
            SymRef::TParam(t) => mix(7, bytes_digest(self.name(self.syms().tparam(t).name).as_bytes())),
            SymRef::Alias(a) => mix(8, bytes_digest(self.name(self.syms().alias(a).name).as_bytes())),
            SymRef::Lambda(e) => mix(9, self.span_key(self.prog().span_of(e))),
            SymRef::Splice(_) => 10,
            SymRef::Site(..) => 11,
            SymRef::Root => 12,
            SymRef::Any => 13,
            SymRef::Nothing => 14,
            SymRef::None => 15,
        }
    }
}

thread_local! {
    /// The values given an identity hash in the run under way, by their class's key and hash,
    /// for the census's count of distinct live values of one class that hash alike.
    static HASHED: RefCell<FxMap<(u64, i32), Vec<Held>>> = RefCell::new(FxMap::default());
}

pub(super) fn hashed() -> usize {
    HASHED.with(|m| m.borrow().values().map(Vec::len).sum())
}

pub(super) enum Held {
    Object(std::rc::Weak<Object>),
    Function(std::rc::Weak<Closure>),
    Array(std::rc::Weak<ArrayCell>),
    Map(std::rc::Weak<MapCell>),
    Trie(std::rc::Weak<Trie>),
}

impl Held {
    fn of(v: &Value) -> Option<Held> {
        Some(match v {
            Value::Obj(o) => Held::Object(Rc::downgrade(o)),
            Value::Fun(c) => Held::Function(Rc::downgrade(c)),
            Value::Array(a) => Held::Array(Rc::downgrade(a)),
            Value::Map(m) => Held::Map(Rc::downgrade(m)),
            Value::Trie(t) => Held::Trie(Rc::downgrade(t)),
            _ => return None,
        })
    }

    /// Whether the value is alive, and whether it is `v`.
    fn state(&self, v: &Value) -> (bool, bool) {
        fn of<T>(w: &std::rc::Weak<T>, p: Option<&Rc<T>>) -> (bool, bool) {
            match w.upgrade() {
                Some(r) => (true, p.is_some_and(|p| Rc::ptr_eq(&r, p))),
                None => (false, false),
            }
        }
        match (self, v) {
            (Held::Object(w), Value::Obj(o)) => of(w, Some(o)),
            (Held::Object(w), _) => of(w, None),
            (Held::Function(w), Value::Fun(c)) => of(w, Some(c)),
            (Held::Function(w), _) => of(w, None),
            (Held::Array(w), Value::Array(a)) => of(w, Some(a)),
            (Held::Array(w), _) => of(w, None),
            (Held::Map(w), Value::Map(m)) => of(w, Some(m)),
            (Held::Map(w), _) => of(w, None),
            (Held::Trie(w), Value::Trie(t)) => of(w, Some(t)),
            (Held::Trie(w), _) => of(w, None),
        }
    }
}

/// Counts `v` as a collision when another live value of its class asked in the run under way
/// has its hash.
fn census_collision(v: &Value, kind: Kind, h: i32, it: &Interp) {
    let class = match v {
        Value::Obj(o) => it.class_key(o.class),
        _ => kind as u64,
    };
    HASHED.with(|m| {
        let mut m = m.borrow_mut();
        let held = m.entry((class, h)).or_default();
        let mut seen = false;
        let mut other = false;
        held.retain(|x| {
            let (alive, same) = x.state(v);
            seen |= same;
            other |= alive && !same;
            alive
        });
        if !seen {
            if other {
                crate::measure::identity_collided();
            }
            if let Some(x) = Held::of(v) {
                held.push(x);
            }
        }
    });
}

/// The values hashed in the run under way, set aside for a nested run and given back after it.
pub(super) fn take_hashed() -> FxMap<(u64, i32), Vec<Held>> {
    HASHED.with(|m| std::mem::take(&mut *m.borrow_mut()))
}

pub(super) fn give_hashed(outer: FxMap<(u64, i32), Vec<Held>>) {
    HASHED.with(|m| *m.borrow_mut() = outer);
}
