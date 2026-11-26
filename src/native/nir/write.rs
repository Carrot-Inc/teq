//! The writer of a NIR file, after `nir.serialization.BinarySerializer` of Scala Native 0.5.12:
//! the inverse of `read`, byte for byte what Scala Native writes for the same definitions. The
//! definitions are walked once, in their order, each section growing in a buffer of its own. A
//! string, a position, a symbol, a type or a value is written where the walk first meets it,
//! after what it is made of, and is its offset from then on: a section's order is the walk's, so
//! the walk visits a definition's parts in the reference's order and in no other.

use super::read::{COMPAT, MAGIC, PRELUDE_LEN};
use super::*;

type Res<T> = Result<T, String>;

const OFFSETS: usize = 0;
const STRINGS: usize = 1;
const POSITIONS: usize = 2;
const GLOBALS: usize = 3;
const TYPES: usize = 4;
const DEFNS: usize = 5;
const VALS: usize = 6;
const INSTS: usize = 7;

/// The bytes of the file that holds `defns`, in their order. `revision` is the prelude's:
/// `read::REVISION` for a file made now, a file's own for that file written again. What Scala
/// Native's writer refuses is refused with the definition that holds it: a type or a value that
/// only a link has, a number beyond the bits the format gives it.
pub fn file(cx: &Cx, defns: &[Defn], revision: u32) -> Res<Vec<u8>> {
    let mut w = Writer {
        cx,
        sections: Default::default(),
        strings: FxMap::default(),
        positions: FxMap::default(),
        globals: FxMap::default(),
        types: FxMap::default(),
        vals: FxMap::default(),
        units: Vec::new(),
        has_entry_points: false,
        refused: None,
    };
    for defn in defns {
        let offset = w.at(DEFNS);
        w.defn(defn);
        w.global(OFFSETS, defn.name);
        w.signed(OFFSETS, offset as i64);
        if let Some(what) = w.refused {
            let mut mangled = Vec::new();
            cx.mangle(defn.name, &mut mangled);
            return Err(format!("{}: {}", String::from_utf8_lossy(&mangled), what));
        }
    }
    w.global(OFFSETS, Global::NONE);
    w.signed(OFFSETS, -1);
    let len = PRELUDE_LEN + w.sections.iter().map(Vec::len).sum::<usize>();
    if len > i32::MAX as usize {
        return Err("a file beyond the 31 bits of its offsets".to_string());
    }
    let mut file = Vec::with_capacity(len);
    for word in [MAGIC, COMPAT, revision] {
        file.extend_from_slice(&word.to_be_bytes());
    }
    let mut start = PRELUDE_LEN as u32;
    for section in &w.sections {
        file.extend_from_slice(&start.to_be_bytes());
        start += section.len() as u32;
    }
    file.push(w.has_entry_points as u8);
    for section in &w.sections {
        file.extend_from_slice(section);
    }
    Ok(file)
}

struct Writer<'a> {
    cx: &'a Cx,
    /// The sections after the prelude, in the file's order.
    sections: [Vec<u8>; 8],
    strings: FxMap<Str, u32>,
    positions: FxMap<Pos, u32>,
    globals: FxMap<Global, u32>,
    types: FxMap<Ty, u32>,
    vals: FxMap<&'a Val, u32>,
    units: Vec<u16>,
    has_entry_points: bool,
    /// The first thing met that Scala Native's writer refuses.
    refused: Option<&'static str>,
}

impl<'a> Writer<'a> {
    fn at(&self, section: usize) -> u32 {
        u32::try_from(self.sections[section].len()).unwrap_or(u32::MAX)
    }

    fn byte(&mut self, to: usize, b: u8) {
        self.sections[to].push(b);
    }

    fn bool(&mut self, to: usize, b: bool) {
        self.byte(to, b as u8);
    }

    fn unsigned(&mut self, to: usize, mut v: u64) {
        loop {
            let low = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                return self.byte(to, low);
            }
            self.byte(to, low | 0x80);
        }
    }

    /// An unsigned number is a JVM `Int` that is not negative.
    fn unsigned_int(&mut self, to: usize, v: u32) {
        if v > i32::MAX as u32 {
            self.refused.get_or_insert("a count, an offset or a number beyond 31 bits");
        }
        self.unsigned(to, v as u64);
    }

    fn unsigned_long(&mut self, to: usize, v: u64) {
        if v > i64::MAX as u64 {
            self.refused.get_or_insert("a local or a size beyond 63 bits");
        }
        self.unsigned(to, v);
    }

    fn signed(&mut self, to: usize, mut v: i64) {
        loop {
            let low = (v & 0x7f) as u8;
            v >>= 7;
            if (v == 0 && low & 0x40 == 0) || (v == -1 && low & 0x40 != 0) {
                return self.byte(to, low);
            }
            self.byte(to, low | 0x80);
        }
    }

    fn len(&mut self, to: usize, n: usize) {
        self.unsigned_int(to, u32::try_from(n).unwrap_or(u32::MAX));
    }

    fn string(&mut self, to: usize, s: Str) {
        let at = self.intern_string(s);
        self.unsigned_int(to, at);
    }

    fn position(&mut self, to: usize, p: Pos) {
        let at = self.intern_position(p);
        self.unsigned_int(to, at);
    }

    fn global(&mut self, to: usize, g: Global) {
        let at = self.intern_global(g);
        self.unsigned_int(to, at);
    }

    fn globals(&mut self, to: usize, gs: &[Global]) {
        self.len(to, gs.len());
        for &g in gs {
            self.global(to, g);
        }
    }

    fn ty(&mut self, to: usize, ty: Ty) {
        let at = self.intern_type(ty);
        self.unsigned_int(to, at);
    }

    fn types(&mut self, to: usize, tys: &[Ty]) {
        self.len(to, tys.len());
        for &ty in tys {
            self.ty(to, ty);
        }
    }

    fn val(&mut self, to: usize, v: &'a Val) {
        let at = self.intern_val(v);
        self.unsigned_int(to, at);
    }

    fn vals(&mut self, to: usize, vs: &'a [Val]) {
        self.len(to, vs.len());
        for v in vs {
            self.val(to, v);
        }
    }

    fn intern_string(&mut self, s: Str) -> u32 {
        if let Some(&at) = self.strings.get(&s) {
            return at;
        }
        let at = self.at(STRINGS);
        self.strings.insert(s, at);
        let mut units = std::mem::take(&mut self.units);
        units.clear();
        units.extend(utf16_units(self.cx.bytes(s)));
        self.len(STRINGS, units.len());
        for &unit in &units {
            self.unsigned_int(STRINGS, unit as u32);
        }
        self.units = units;
        at
    }

    fn intern_position(&mut self, p: Pos) -> u32 {
        if let Some(&at) = self.positions.get(&p) {
            return at;
        }
        let at = self.at(POSITIONS);
        self.positions.insert(p, at);
        self.string(POSITIONS, p.file);
        self.unsigned_int(POSITIONS, p.line);
        self.unsigned_int(POSITIONS, p.column);
        at
    }

    fn intern_global(&mut self, g: Global) -> u32 {
        if let Some(&at) = self.globals.get(&g) {
            return at;
        }
        let data = self.cx.global_data(g);
        if let GlobalData::Member(owner, _) = data {
            self.intern_global(owner);
        }
        let at = self.at(GLOBALS);
        self.globals.insert(g, at);
        match data {
            GlobalData::None => self.byte(GLOBALS, 1),
            GlobalData::Top(id) => {
                self.byte(GLOBALS, 2);
                self.string(GLOBALS, id);
            }
            GlobalData::Member(owner, sig) => {
                self.byte(GLOBALS, 3);
                self.global(GLOBALS, owner);
                self.string(GLOBALS, sig);
            }
        }
        at
    }

    fn intern_type(&mut self, ty: Ty) -> u32 {
        if let Some(&at) = self.types.get(&ty) {
            return at;
        }
        let cx = self.cx;
        let data = cx.type_of(ty);
        match data {
            Type::Function(args, ret) => {
                for &arg in args.iter() {
                    self.intern_type(arg);
                }
                self.intern_type(*ret);
            }
            Type::StructValue(tys) => {
                for &ty in tys.iter() {
                    self.intern_type(ty);
                }
            }
            Type::Array { of, .. } | Type::ArrayValue(of, _) | Type::Var(of) => {
                self.intern_type(*of);
            }
            _ => {}
        }
        let at = self.at(TYPES);
        self.types.insert(ty, at);
        match data {
            Type::Vararg => self.byte(TYPES, 1),
            Type::Bool => self.byte(TYPES, 2),
            Type::Ptr => self.byte(TYPES, 3),
            Type::Char => self.byte(TYPES, 4),
            Type::Byte => self.byte(TYPES, 5),
            Type::Short => self.byte(TYPES, 6),
            Type::Int => self.byte(TYPES, 7),
            Type::Long => self.byte(TYPES, 8),
            Type::Float => self.byte(TYPES, 9),
            Type::Double => self.byte(TYPES, 10),
            Type::ArrayValue(of, n) => {
                self.byte(TYPES, 11);
                self.ty(TYPES, *of);
                self.unsigned_int(TYPES, *n as u32);
            }
            Type::StructValue(tys) => {
                self.byte(TYPES, 12);
                self.types(TYPES, tys);
            }
            Type::Function(args, ret) => {
                self.byte(TYPES, 13);
                self.types(TYPES, args);
                self.ty(TYPES, *ret);
            }
            Type::Null => self.byte(TYPES, 14),
            Type::Nothing => self.byte(TYPES, 15),
            Type::Virtual => {
                self.refused.get_or_insert("a virtual type, which no file holds");
            }
            Type::Var(of) => {
                self.byte(TYPES, 17);
                self.ty(TYPES, *of);
            }
            Type::Unit => self.byte(TYPES, 18),
            Type::Array { of, nullable } => {
                self.byte(TYPES, 19);
                self.ty(TYPES, *of);
                self.bool(TYPES, *nullable);
            }
            Type::Ref { name, exact, nullable } => {
                self.byte(TYPES, 20);
                self.global(TYPES, *name);
                self.bool(TYPES, *exact);
                self.bool(TYPES, *nullable);
            }
            Type::Size => self.byte(TYPES, 21),
            Type::Int128 => {
                self.refused.get_or_insert("a 128-bit integer type, which no file holds");
            }
        }
        at
    }

    /// Two values are one entry when they are equal, a byte string by the literal it is and a
    /// floating-point number by its bits, a NaN read from a file by its entry there.
    fn intern_val(&mut self, v: &'a Val) -> u32 {
        if let Some(&at) = self.vals.get(v) {
            return at;
        }
        match v {
            Val::Const(of) => {
                self.intern_val(of);
            }
            Val::ArrayValue(_, vs) | Val::StructValue(vs) => {
                for v in vs.iter() {
                    self.intern_val(v);
                }
            }
            _ => {}
        }
        let at = self.at(VALS);
        self.vals.insert(v, at);
        match v {
            Val::True => self.byte(VALS, 1),
            Val::False => self.byte(VALS, 2),
            Val::Null => self.byte(VALS, 3),
            Val::Zero(ty) => {
                self.byte(VALS, 4);
                self.ty(VALS, *ty);
            }
            Val::Char(c) => {
                self.byte(VALS, 5);
                self.unsigned_int(VALS, *c as u32);
            }
            Val::Byte(b) => {
                self.byte(VALS, 6);
                self.byte(VALS, *b as u8);
            }
            Val::Short(n) => {
                self.byte(VALS, 7);
                self.signed(VALS, *n as i64);
            }
            Val::Int(n) => {
                self.byte(VALS, 8);
                self.signed(VALS, *n as i64);
            }
            Val::Long(n) => {
                self.byte(VALS, 9);
                self.signed(VALS, *n);
            }
            // The JVM's `DataOutputStream` writes every NaN as its canonical one, whatever its bits.
            Val::Float(bits, _) => {
                self.byte(VALS, 10);
                let written = if f32::from_bits(*bits).is_nan() { 0x7fc0_0000 } else { *bits };
                self.sections[VALS].extend_from_slice(&written.to_be_bytes());
            }
            Val::Double(bits, _) => {
                self.byte(VALS, 11);
                let written = if f64::from_bits(*bits).is_nan() { 0x7ff8_0000_0000_0000 } else { *bits };
                self.sections[VALS].extend_from_slice(&written.to_be_bytes());
            }
            Val::StructValue(vs) => {
                self.byte(VALS, 12);
                self.vals(VALS, vs);
            }
            Val::ArrayValue(ty, vs) => {
                self.byte(VALS, 13);
                self.ty(VALS, *ty);
                self.vals(VALS, vs);
            }
            Val::ByteString(bytes, _) => {
                self.byte(VALS, 14);
                self.len(VALS, bytes.len());
                self.sections[VALS].extend_from_slice(bytes);
            }
            Val::Local(id, ty) => {
                self.byte(VALS, 15);
                self.unsigned_long(VALS, *id);
                self.ty(VALS, *ty);
            }
            Val::Global(name, ty) => {
                self.byte(VALS, 16);
                self.global(VALS, *name);
                self.ty(VALS, *ty);
            }
            Val::Unit => self.byte(VALS, 17),
            Val::Const(of) => {
                self.byte(VALS, 18);
                self.val(VALS, of);
            }
            Val::String(s) => {
                self.byte(VALS, 19);
                self.string(VALS, *s);
            }
            Val::Virtual(_) => {
                self.refused.get_or_insert("a virtual value, which no file holds");
            }
            Val::ClassOf(name) => {
                self.byte(VALS, 21);
                self.global(VALS, *name);
            }
            Val::Size(n) => {
                self.byte(VALS, 23);
                self.unsigned_long(VALS, *n as u64);
            }
            Val::Int128 { .. } => {
                self.refused.get_or_insert("a 128-bit integer, which no file holds");
            }
        }
        at
    }

    /// The attributes in `nir.Attrs.toSeq`'s order, those that are not the default.
    fn attrs(&mut self, a: &Attrs) {
        let flags = [
            (a.is_dyn, 13),
            (a.is_stub, 14),
            (a.link_cpp_runtime, 23),
            (a.is_abstract, 15),
            (a.is_volatile, 16),
            (a.is_final, 17),
            (a.is_safe_publish, 18),
            (a.is_linktime_resolved, 19),
            (a.uses_intrinsic, 20),
        ];
        let single = [a.inline != Inline::May, a.no_specialize, a.opt != Opt::Un, a.align.is_some(), a.is_extern];
        let count = single.iter().filter(|&&set| set).count() + flags.iter().filter(|f| f.0).count() + a.links.len() + a.defines.len();
        self.len(DEFNS, count);
        match a.inline {
            Inline::May => {}
            Inline::Hint => self.byte(DEFNS, 2),
            Inline::No => self.byte(DEFNS, 3),
            Inline::Always => self.byte(DEFNS, 4),
        }
        if a.no_specialize {
            self.byte(DEFNS, 6);
        }
        match a.opt {
            Opt::Un => {}
            Opt::No => self.byte(DEFNS, 8),
            Opt::Did => self.byte(DEFNS, 9),
            Opt::Bail(msg) => {
                self.byte(DEFNS, 10);
                self.string(DEFNS, msg);
            }
        }
        if let Some((size, group)) = a.align {
            self.byte(DEFNS, 21);
            self.signed(DEFNS, size as i64);
            self.bool(DEFNS, group.is_some());
            if let Some(group) = group {
                self.string(DEFNS, group);
            }
        }
        if a.is_extern {
            self.byte(DEFNS, 11);
            self.bool(DEFNS, a.is_blocking);
        }
        for (set, tag) in flags {
            if set {
                self.byte(DEFNS, tag);
            }
        }
        for (list, tag) in [(&a.links, 12), (&a.defines, 22)] {
            for &s in list {
                self.byte(DEFNS, tag);
                self.string(DEFNS, s);
            }
        }
    }

    fn defn(&mut self, defn: &'a Defn) {
        let tag = match defn.kind {
            DefnKind::Var { .. } => 1,
            DefnKind::Const { .. } => 2,
            DefnKind::Declare { .. } => 3,
            DefnKind::Define { .. } => 4,
            DefnKind::Trait { .. } => 5,
            DefnKind::Class { .. } => 6,
            DefnKind::Module { .. } => 7,
        };
        self.byte(DEFNS, tag);
        self.global(DEFNS, defn.name);
        self.attrs(&defn.attrs);
        self.position(DEFNS, defn.pos);
        match &defn.kind {
            DefnKind::Var { ty, rhs } | DefnKind::Const { ty, rhs } => {
                self.ty(DEFNS, *ty);
                self.val(DEFNS, rhs);
            }
            DefnKind::Declare { ty } => self.ty(DEFNS, *ty),
            DefnKind::Define { ty, insts, debug } => {
                // A class's initializer and a method of C are reached whatever a link reaches.
                if let GlobalData::Member(_, sig) = self.cx.global_data(defn.name) {
                    self.has_entry_points |= defn.attrs.is_extern || self.cx.bytes(sig).first() == Some(&b'I');
                }
                self.ty(DEFNS, *ty);
                let at = self.at(INSTS);
                self.unsigned_int(DEFNS, at);
                self.len(INSTS, insts.len());
                for inst in insts {
                    self.inst(inst);
                }
                self.len(DEFNS, debug.local_names().len());
                for &(local, name) in debug.local_names() {
                    self.unsigned_long(DEFNS, local);
                    self.string(DEFNS, name);
                }
                self.len(DEFNS, debug.scopes.len());
                if debug.scopes.is_sorted_by_key(|scope| scope.id) {
                    debug.scopes.iter().for_each(|scope| self.scope(scope));
                } else {
                    let mut scopes: Vec<&LexicalScope> = debug.scopes.iter().collect();
                    scopes.sort_by_key(|scope| scope.id);
                    scopes.into_iter().for_each(|scope| self.scope(scope));
                }
            }
            DefnKind::Trait { traits } => self.globals(DEFNS, traits),
            DefnKind::Class { parent, traits } | DefnKind::Module { parent, traits } => {
                self.bool(DEFNS, parent.is_some());
                if let Some(parent) = parent {
                    self.global(DEFNS, *parent);
                }
                self.globals(DEFNS, traits);
            }
        }
    }

    fn scope(&mut self, scope: &LexicalScope) {
        self.unsigned_int(DEFNS, scope.id);
        self.unsigned_int(DEFNS, scope.parent);
        self.position(DEFNS, scope.pos);
    }

    fn inst(&mut self, inst: &'a Inst) {
        let tag = match &inst.kind {
            InstKind::Label { .. } => 1,
            InstKind::Let { unwind: Next::None, .. } => 2,
            InstKind::Let { .. } => 3,
            InstKind::Ret(_) => 4,
            InstKind::Jump(_) => 5,
            InstKind::If { .. } => 6,
            InstKind::Switch { .. } => 7,
            InstKind::Throw { .. } => 8,
            InstKind::Unreachable { .. } => 9,
            InstKind::LinktimeIf { .. } => 10,
        };
        self.byte(INSTS, tag);
        self.position(INSTS, inst.pos);
        match &inst.kind {
            InstKind::Label { id, params } => {
                self.unsigned_long(INSTS, *id);
                self.len(INSTS, params.len());
                for &(param, ty) in params.iter() {
                    self.unsigned_long(INSTS, param);
                    self.ty(INSTS, ty);
                }
            }
            InstKind::Let { id, op, unwind, scope } => {
                self.unsigned_long(INSTS, *id);
                self.op(op);
                if tag == 3 {
                    self.next(unwind);
                }
                self.unsigned_int(INSTS, *scope);
            }
            InstKind::Ret(value) => self.val(INSTS, value),
            InstKind::Jump(next) => self.next(next),
            InstKind::If { value, then, otherwise } => {
                self.val(INSTS, value);
                self.next(then);
                self.next(otherwise);
            }
            InstKind::Switch { value, default, cases } => {
                self.val(INSTS, value);
                self.next(default);
                self.len(INSTS, cases.len());
                for case in cases.iter() {
                    self.next(case);
                }
            }
            InstKind::Throw { value, unwind } => {
                self.val(INSTS, value);
                self.next(unwind);
            }
            InstKind::Unreachable { unwind } => self.next(unwind),
            InstKind::LinktimeIf { cond, then, otherwise } => {
                self.linktime_condition(cond);
                self.next(then);
                self.next(otherwise);
            }
        }
    }

    fn next(&mut self, next: &'a Next) {
        match next {
            Next::None => self.byte(INSTS, 1),
            Next::Unwind { exc, ty, next } => {
                self.byte(INSTS, 2);
                self.unsigned_long(INSTS, *exc);
                self.ty(INSTS, *ty);
                self.next(next);
            }
            Next::Case(value, next) => {
                self.byte(INSTS, 3);
                self.val(INSTS, value);
                self.next(next);
            }
            Next::Label(id, args) => {
                self.byte(INSTS, 4);
                self.unsigned_long(INSTS, *id);
                self.vals(INSTS, args);
            }
        }
    }

    fn linktime_condition(&mut self, cond: &'a LinktimeCondition) {
        match cond {
            LinktimeCondition::Simple { property, comparison, value, pos } => {
                self.byte(INSTS, 1);
                self.string(INSTS, *property);
                self.byte(INSTS, *comparison as u8 + 1);
                self.val(INSTS, value);
                self.position(INSTS, *pos);
            }
            LinktimeCondition::Complex { op, left, right, pos } => {
                self.byte(INSTS, 2);
                self.byte(INSTS, *op as u8 + 1);
                self.linktime_condition(left);
                self.linktime_condition(right);
                self.position(INSTS, *pos);
            }
        }
    }

    fn indexes(&mut self, indexes: &[i32]) {
        self.len(INSTS, indexes.len());
        for &index in indexes {
            self.signed(INSTS, index as i64);
        }
    }

    /// An operation's tag, its type where it has one, and its operands in the reader's order. The
    /// tag of a binary operation, a comparison, a conversion or a memory order is its place in the
    /// model's enumeration, counted from one, as the reader's tables have them.
    fn op(&mut self, op: &'a Op) {
        let typed = |w: &mut Self, tag: u8, ty: Ty, operands: &[&'a Val]| {
            w.byte(INSTS, tag);
            w.ty(INSTS, ty);
            for &operand in operands {
                w.val(INSTS, operand);
            }
        };
        match op {
            Op::Call { ty, ptr, args } => {
                typed(self, 1, *ty, &[ptr]);
                self.vals(INSTS, args);
            }
            Op::Load { ty, ptr, order: None } => typed(self, 2, *ty, &[ptr]),
            Op::Load { ty, ptr, order: Some(order) } => {
                typed(self, 3, *ty, &[ptr]);
                self.byte(INSTS, *order as u8 + 1);
            }
            Op::Store { ty, ptr, value, order: None } => typed(self, 4, *ty, &[ptr, value]),
            Op::Store { ty, ptr, value, order: Some(order) } => {
                typed(self, 5, *ty, &[ptr, value]);
                self.byte(INSTS, *order as u8 + 1);
            }
            Op::Elem { ty, ptr, indexes } => {
                typed(self, 6, *ty, &[ptr]);
                self.vals(INSTS, indexes);
            }
            Op::Extract { aggr, indexes } => {
                self.byte(INSTS, 7);
                self.val(INSTS, aggr);
                self.indexes(indexes);
            }
            Op::Insert { aggr, value, indexes } => {
                self.byte(INSTS, 8);
                self.val(INSTS, aggr);
                self.val(INSTS, value);
                self.indexes(indexes);
            }
            Op::Stackalloc { ty, n } => typed(self, 9, *ty, &[n]),
            Op::Bin { bin, ty, l, r } => {
                self.byte(INSTS, 10);
                typed(self, *bin as u8 + 1, *ty, &[l, r]);
            }
            Op::Comp { comp, ty, l, r } => {
                self.byte(INSTS, 11);
                typed(self, *comp as u8 + 1, *ty, &[l, r]);
            }
            Op::Conv { conv, ty, value } => {
                self.byte(INSTS, 12);
                typed(self, *conv as u8 + 1, *ty, &[value]);
            }
            Op::Classalloc { name, zone } => {
                self.byte(INSTS, if zone.is_some() { 14 } else { 13 });
                self.global(INSTS, *name);
                if let Some(zone) = zone {
                    self.val(INSTS, zone);
                }
            }
            Op::Field { obj, name } => {
                self.byte(INSTS, 15);
                self.val(INSTS, obj);
                self.global(INSTS, *name);
            }
            Op::Fieldload { ty, obj, name } => {
                typed(self, 16, *ty, &[obj]);
                self.global(INSTS, *name);
            }
            Op::Fieldstore { ty, obj, name, value } => {
                typed(self, 17, *ty, &[obj]);
                self.global(INSTS, *name);
                self.val(INSTS, value);
            }
            Op::Method { obj, sig } => {
                self.byte(INSTS, 18);
                self.val(INSTS, obj);
                self.string(INSTS, *sig);
            }
            Op::Module(name) => {
                self.byte(INSTS, 19);
                self.global(INSTS, *name);
            }
            Op::As { ty, obj } => typed(self, 20, *ty, &[obj]),
            Op::Is { ty, obj } => typed(self, 21, *ty, &[obj]),
            Op::Copy(value) => {
                self.byte(INSTS, 22);
                self.val(INSTS, value);
            }
            Op::SizeOf(ty) => typed(self, 23, *ty, &[]),
            Op::AlignmentOf(ty) => typed(self, 24, *ty, &[]),
            Op::Box { ty, obj } => typed(self, 25, *ty, &[obj]),
            Op::Unbox { ty, obj } => typed(self, 26, *ty, &[obj]),
            Op::Dynmethod { obj, sig } => {
                self.byte(INSTS, 27);
                self.val(INSTS, obj);
                self.string(INSTS, *sig);
            }
            Op::Var(ty) => typed(self, 28, *ty, &[]),
            Op::Varload(slot) => {
                self.byte(INSTS, 29);
                self.val(INSTS, slot);
            }
            Op::Varstore { slot, value } => {
                self.byte(INSTS, 30);
                self.val(INSTS, slot);
                self.val(INSTS, value);
            }
            Op::Arrayalloc { ty, init, zone: None } => typed(self, 31, *ty, &[init]),
            Op::Arrayalloc { ty, init, zone: Some(zone) } => typed(self, 32, *ty, &[init, zone]),
            Op::Arrayload { ty, arr, idx } => typed(self, 33, *ty, &[arr, idx]),
            Op::Arraystore { ty, arr, idx, value } => typed(self, 34, *ty, &[arr, idx, value]),
            Op::Arraylength(arr) => {
                self.byte(INSTS, 35);
                self.val(INSTS, arr);
            }
            Op::Fence(order) => {
                self.byte(INSTS, 36);
                self.byte(INSTS, *order as u8 + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::read::{NirFile, REVISION};
    use super::*;

    fn prelude(starts: [u32; 8], has_entry_points: bool) -> Vec<u8> {
        let mut file = Vec::new();
        for word in [MAGIC, COMPAT, REVISION].into_iter().chain(starts) {
            file.extend_from_slice(&word.to_be_bytes());
        }
        file.push(has_entry_points as u8);
        file
    }

    fn read(cx: &mut Cx, file: &[u8]) -> Vec<Defn> {
        NirFile::open(file).and_then(|mut f| f.written_defns(cx)).unwrap()
    }

    const NOWHERE: Pos = Pos { file: Str::EMPTY, line: 0, column: 0, place: None };

    /// The symbol `A.<sig>` and the type `() => unit`.
    fn member(cx: &mut Cx, sig: &[u8]) -> (Global, Ty) {
        let class = cx.str(b"A");
        let owner = cx.global(GlobalData::Top(class));
        let sig = cx.str(sig);
        let unit = cx.ty(Type::Unit);
        (cx.global(GlobalData::Member(owner, sig)), cx.ty(Type::Function(Box::new([]), unit)))
    }

    #[test]
    fn writes_an_empty_file() {
        let mut cx = Cx::new();
        let written = file(&cx, &[], REVISION).unwrap();
        // The table is its end, the symbol that is none at the offset -1, and that symbol is the
        // one there is.
        let mut expected = prelude([45, 47, 47, 47, 48, 48, 48, 48], false);
        expected.extend_from_slice(&[0, 0x7f, 1]);
        assert_eq!(written, expected);
        assert_eq!(read(&mut cx, &written), Vec::new());
    }

    #[test]
    fn writes_a_declaration() {
        let mut cx = Cx::new();
        let (name, ty) = member(&mut cx, b"D1fuEO");
        let defns = [Defn { attrs: Attrs::default(), name, pos: NOWHERE, kind: DefnKind::Declare { ty } }];
        let written = file(&cx, &defns, REVISION).unwrap();
        let mut expected = prelude([45, 49, 59, 62, 68, 72, 77, 77], false);
        // The table: the declaration at 0, then its end.
        expected.extend_from_slice(&[2, 0, 5, 0x7f]);
        // The strings as the walk meets them: the class, the signature, the position's file.
        expected.extend_from_slice(b"\x01A\x06D1fuEO\x00");
        expected.extend_from_slice(&[9, 0, 0]);
        // The symbols: a member's owner before it, the table's end last.
        expected.extend_from_slice(&[2, 0, 3, 0, 2, 1]);
        // The types: a function's result before it.
        expected.extend_from_slice(&[18, 13, 0, 0]);
        // The declaration: its name, no attributes, its position, its type.
        expected.extend_from_slice(&[3, 2, 0, 0, 1]);
        assert_eq!(written, expected);
        // Read back, the position in no file has the place of its entry, the first, counted from one.
        let placed = Defn { pos: Pos { place: NonZeroU32::new(1), ..NOWHERE }, ..defns[0].clone() };
        assert_eq!(read(&mut cx, &written), [placed]);
    }

    fn count(file: &[u8], bytes: &[u8]) -> usize {
        file.windows(bytes.len()).filter(|window| window == &bytes).count()
    }

    #[test]
    fn writes_the_attributes_in_their_order() {
        let mut cx = Cx::new();
        let (name, ty) = member(&mut cx, b"D1fuEO");
        let [m, g, x, y, d] = [b"m", b"g", b"x", b"y", b"d"].map(|s| cx.str(s));
        let attrs = Attrs {
            inline: Inline::Always,
            no_specialize: true,
            opt: Opt::Bail(m),
            align: Some((16, Some(g))),
            is_extern: true,
            is_blocking: true,
            is_dyn: true,
            is_stub: true,
            is_abstract: true,
            is_volatile: true,
            is_final: true,
            is_safe_publish: true,
            is_linktime_resolved: true,
            uses_intrinsic: true,
            link_cpp_runtime: true,
            links: vec![x, y],
            defines: vec![d],
        };
        let written = file(&cx, &[Defn { attrs, name, pos: NOWHERE, kind: DefnKind::Declare { ty } }], REVISION).unwrap();
        // Scala Native's `Attrs.toSeq`: inlining, specialization, optimization, alignment, extern, the flags, the
        // links, the definitions; the strings at 9, 11, 13, 15 and 17, after the symbol's two.
        let expected: &[u8] = &[17, 4, 6, 10, 9, 21, 16, 1, 11, 11, 1, 13, 14, 23, 15, 16, 17, 18, 19, 20, 12, 13, 12, 15, 22, 17];
        assert_eq!(count(&written, expected), 1);
    }

    #[test]
    fn keeps_what_a_file_orders_and_tells_apart() {
        let mut cx = Cx::new();
        let (name, ty) = member(&mut cx, b"IE");
        let (first, second) = (cx.str(b"first"), cx.str(b"second"));
        // One surrogate without its pair, then a character of two.
        let text = cx.str(b"\xed\xa0\x80\xf0\x9f\x99\x82");
        let literal = |which| Val::Const(Arc::new(Val::ByteString((*b"hi").into(), which)));
        let copy = |id, value| Inst { pos: NOWHERE, kind: InstKind::Let { id, op: Op::Copy(value), unwind: Next::None, scope: 0 } };
        let insts = vec![
            Inst { pos: NOWHERE, kind: InstKind::Label { id: 0, params: Box::new([]) } },
            copy(7, literal(0)),
            copy(3, literal(1)),
            copy(9, literal(1)),
            copy(4, Val::String(text)),
            Inst { pos: NOWHERE, kind: InstKind::Ret(Val::Unit) },
        ];
        let debug = DebugInfo::new(vec![(7, first), (3, second)], vec![LexicalScope { id: 0, parent: 0, pos: NOWHERE }]);
        let defns = [Defn { attrs: Attrs::default(), name, pos: NOWHERE, kind: DefnKind::Define { ty, insts, debug } }];
        let written = file(&cx, &defns, REVISION).unwrap();
        // A class's initializer is an entry point.
        assert_eq!(written[44], 1);
        // Two literals of the same bytes are two entries, one literal used twice is one.
        assert_eq!(count(&written, b"\x0e\x02hi"), 2);
        // The string's three units of UTF-16.
        assert_eq!(count(&written, &[3, 0x80, 0xb0, 0x03, 0xbd, 0xb0, 0x03, 0xc2, 0xbc, 0x03]), 1);
        let back = read(&mut cx, &written);
        let DefnKind::Define { debug, .. } = &back[0].kind else { panic!("no method read back") };
        // The names in the order given, not in their locals'.
        assert_eq!(debug.local_names(), [(7, first), (3, second)]);
        assert_eq!((debug.local_name(3), debug.local_name(7), debug.local_name(9)), (Some(second), Some(first), None));
        let in_order = DebugInfo::new(vec![(3, second), (7, first)], Vec::new());
        assert_eq!((in_order.local_name(3), in_order.local_name(7), in_order.local_name(9)), (Some(second), Some(first), None));
        assert_eq!(file(&cx, &back, REVISION).unwrap(), written);
    }

    fn method(name: Global, ty: Ty, is_extern: bool, scopes: Vec<LexicalScope>) -> Defn {
        let insts = vec![Inst { pos: NOWHERE, kind: InstKind::Ret(Val::Unit) }];
        let debug = DebugInfo::new(Vec::new(), scopes);
        Defn { attrs: Attrs { is_extern, ..Attrs::default() }, name, pos: NOWHERE, kind: DefnKind::Define { ty, insts, debug } }
    }

    #[test]
    fn writes_the_scopes_in_their_order() {
        let mut cx = Cx::new();
        let (name, ty) = member(&mut cx, b"D1fuEO");
        let scope = |id| LexicalScope { id, parent: 0, pos: Pos { file: Str::EMPTY, line: id + 1, column: 0, place: None } };
        let sorted = file(&cx, &[method(name, ty, false, vec![scope(0), scope(1), scope(2)])], REVISION).unwrap();
        assert_eq!(file(&cx, &[method(name, ty, false, vec![scope(2), scope(0), scope(1)])], REVISION).unwrap(), sorted);
    }

    #[test]
    fn marks_a_file_that_holds_an_entry_point() {
        let mut cx = Cx::new();
        let (name, ty) = member(&mut cx, b"D1fuEO");
        let (initializer, _) = member(&mut cx, b"IE");
        let declared = Defn { attrs: Attrs { is_extern: true, ..Attrs::default() }, name, pos: NOWHERE, kind: DefnKind::Declare { ty } };
        // A method of C declared, or a method defined that is neither of C nor an initializer, is none.
        assert_eq!(file(&cx, &[method(name, ty, false, Vec::new()), declared], REVISION).unwrap()[44], 0);
        assert_eq!(file(&cx, &[method(name, ty, true, Vec::new())], REVISION).unwrap()[44], 1);
        assert_eq!(file(&cx, &[method(initializer, ty, false, Vec::new())], REVISION).unwrap()[44], 1);
    }

    /// One definition of each kind, the method made of every instruction, operation, successor,
    /// value and type a file holds: what the reader makes of the file written is what was written.
    /// No byte string is among the values, since the reader numbers a literal by its place.
    #[test]
    fn reads_back_every_shape() {
        let mut cx = Cx::new();
        let (name, _) = member(&mut cx, b"D1fuEO");
        let texts = [cx.str(b"Shapes.scala"), cx.str(b"some text"), cx.str(b"a.property")];
        let class = cx.global(GlobalData::Top(texts[1]));
        let at = |line| Pos { file: texts[0], line, column: 2, place: None };
        let (int, unit) = (cx.ty(Type::Int), cx.ty(Type::Unit));
        let simple = [Type::Vararg, Type::Ptr, Type::Bool, Type::Char, Type::Byte, Type::Short, Type::Long, Type::Float, Type::Double];
        let nested = [
            Type::Size,
            Type::Null,
            Type::Nothing,
            Type::ArrayValue(int, 3),
            Type::StructValue(Box::new([int, unit])),
            Type::Var(int),
            Type::Array { of: int, nullable: false },
            Type::Ref { name: class, exact: true, nullable: false },
        ];
        let params: Vec<Ty> = simple.into_iter().chain(nested).map(|ty| cx.ty(ty)).collect();
        let ty = cx.ty(Type::Function(params.into(), unit));
        let (a, b, c) = (Val::Local(1, int), Val::Local(2, int), Val::Local(3, int));
        let every_val = Val::StructValue(Arc::new([
            Val::True,
            Val::False,
            Val::Null,
            Val::Zero(int),
            Val::Size(5),
            Val::Char(0xffff),
            Val::Byte(-1),
            Val::Short(-300),
            Val::Int(i32::MIN),
            Val::Long(i64::MIN),
            Val::Float(1.5f32.to_bits(), None),
            Val::Double((-0.0f64).to_bits(), None),
            Val::ArrayValue(int, Arc::new([Val::Int(1), Val::Int(2)])),
            Val::Global(name, int),
            Val::Unit,
            Val::Const(Arc::new(Val::Int(7))),
            Val::String(texts[1]),
            Val::ClassOf(class),
        ]));
        let (a2, b2, c2) = (|| a.clone(), || b.clone(), || c.clone());
        let ops = vec![
            Op::Call { ty, ptr: a2(), args: Box::new([b2(), c2()]) },
            Op::Load { ty: int, ptr: a2(), order: None },
            Op::Load { ty: int, ptr: a2(), order: Some(MemoryOrder::Acquire) },
            Op::Store { ty: int, ptr: a2(), value: b2(), order: None },
            Op::Store { ty: int, ptr: a2(), value: b2(), order: Some(MemoryOrder::SeqCst) },
            Op::Elem { ty: int, ptr: a2(), indexes: Box::new([b2(), c2()]) },
            Op::Extract { aggr: a2(), indexes: Box::new([1, -2]) },
            Op::Insert { aggr: a2(), value: b2(), indexes: Box::new([300]) },
            Op::Stackalloc { ty: int, n: a2() },
            Op::Bin { bin: Bin::Xor, ty: int, l: a2(), r: b2() },
            Op::Comp { comp: Comp::Fle, ty: int, l: a2(), r: b2() },
            Op::Conv { conv: Conv::ZSizeCast, ty: int, value: a2() },
            Op::Fence(MemoryOrder::Release),
            Op::Classalloc { name: class, zone: None },
            Op::Classalloc { name: class, zone: Some(a2()) },
            Op::Fieldload { ty: int, obj: a2(), name },
            Op::Fieldstore { ty: int, obj: a2(), name, value: b2() },
            Op::Field { obj: a2(), name },
            Op::Method { obj: a2(), sig: texts[1] },
            Op::Dynmethod { obj: a2(), sig: texts[2] },
            Op::Module(class),
            Op::As { ty: int, obj: a2() },
            Op::Is { ty: int, obj: a2() },
            Op::Copy(a2()),
            Op::SizeOf(int),
            Op::AlignmentOf(int),
            Op::Box { ty: int, obj: a2() },
            Op::Unbox { ty: int, obj: a2() },
            Op::Var(int),
            Op::Varload(a2()),
            Op::Varstore { slot: a2(), value: b2() },
            Op::Arrayalloc { ty: int, init: a2(), zone: None },
            Op::Arrayalloc { ty: int, init: a2(), zone: Some(b2()) },
            Op::Arrayload { ty: int, arr: a2(), idx: b2() },
            Op::Arraystore { ty: int, arr: a2(), idx: b2(), value: c2() },
            Op::Arraylength(a2()),
        ];
        let label = || Next::Label(5, Box::new([a2(), b2()]));
        let handler = || Next::Unwind { exc: 9, ty: int, next: Box::new(label()) };
        let property = |comparison, line| LinktimeCondition::Simple { property: texts[2], comparison, value: Val::True, pos: at(line) };
        let mut insts = vec![Inst { pos: at(0), kind: InstKind::Label { id: 0, params: Box::new([(1, int), (2, unit)]) } }];
        for (i, op) in ops.into_iter().enumerate() {
            let unwind = if i % 2 == 0 { Next::None } else { handler() };
            insts.push(Inst { pos: at(i as u32), kind: InstKind::Let { id: 10 + i as u64, op, unwind, scope: i as u32 % 2 } });
        }
        let cond = LinktimeCondition::Complex { op: Bin::And, left: Box::new(property(Comp::Ieq, 1)), right: Box::new(property(Comp::Sge, 2)), pos: at(3) };
        for kind in [
            InstKind::Jump(label()),
            InstKind::If { value: a2(), then: label(), otherwise: Next::None },
            InstKind::Switch { value: a2(), default: label(), cases: Box::new([Next::Case(Val::Int(1), Box::new(label())), Next::Case(Val::Int(2), Box::new(label()))]) },
            InstKind::Throw { value: a2(), unwind: handler() },
            InstKind::Unreachable { unwind: Next::None },
            InstKind::LinktimeIf { cond, then: label(), otherwise: label() },
            InstKind::Ret(every_val.clone()),
        ] {
            insts.push(Inst { pos: at(200), kind });
        }
        let scopes = vec![LexicalScope { id: 0, parent: 0, pos: at(0) }, LexicalScope { id: 1, parent: 0, pos: at(7) }];
        let debug = DebugInfo::new(vec![(12, texts[1]), (11, texts[2])], scopes);
        let attrs = Attrs {
            inline: Inline::Always,
            no_specialize: true,
            opt: Opt::Bail(texts[1]),
            align: Some((-1, Some(texts[2]))),
            is_extern: true,
            is_blocking: true,
            is_dyn: true,
            is_stub: true,
            is_abstract: true,
            is_volatile: true,
            is_final: true,
            is_safe_publish: true,
            is_linktime_resolved: true,
            uses_intrinsic: true,
            link_cpp_runtime: true,
            links: vec![texts[0], texts[1]],
            defines: vec![texts[2]],
        };
        let defn = |attrs: &Attrs, name, kind| Defn { attrs: attrs.clone(), name, pos: at(100), kind };
        let plain = Attrs { align: Some((8, None)), ..Attrs::default() };
        let defns = [
            defn(&attrs, name, DefnKind::Define { ty, insts, debug }),
            defn(&plain, name, DefnKind::Var { ty: int, rhs: Val::Int(1) }),
            defn(&plain, name, DefnKind::Const { ty: unit, rhs: every_val }),
            defn(&plain, name, DefnKind::Declare { ty }),
            defn(&plain, class, DefnKind::Trait { traits: Box::new([class]) }),
            defn(&plain, class, DefnKind::Class { parent: Some(class), traits: Box::new([class, class]) }),
            defn(&plain, class, DefnKind::Module { parent: None, traits: Box::new([]) }),
        ];
        let written = file(&cx, &defns, REVISION).unwrap();
        assert_eq!(read(&mut cx, &written), defns);
    }

    #[test]
    fn writes_each_nan_as_the_canonical_one() {
        let mut cx = Cx::new();
        let (name, _) = member(&mut cx, b"F1v");
        let float = cx.ty(Type::Float);
        let nans = Val::StructValue(Arc::new([Val::Float(0x7fc0_0001, None), Val::Float(0xffa0_0000, None), Val::Float(0x7fc0_0001, None), Val::Double(0x7ff0_0000_0000_0001, None)]));
        let defns = [Defn { attrs: Attrs::default(), name, pos: NOWHERE, kind: DefnKind::Var { ty: float, rhs: nans } }];
        let written = file(&cx, &defns, REVISION).unwrap();
        // Two entries for the two floats that differ in their bits, the canonical bits in both.
        assert_eq!(count(&written, &[10, 0x7f, 0xc0, 0, 0]), 2);
        assert_eq!(count(&written, &[11, 0x7f, 0xf8, 0, 0, 0, 0, 0, 0]), 1);
    }

    /// The numbers of a value and of what it is made of, in their order.
    fn numbers<'v>(v: &'v Val, out: &mut Vec<&'v Val>) {
        match v {
            Val::Float(..) | Val::Double(..) => out.push(v),
            Val::StructValue(vs) | Val::ArrayValue(_, vs) => vs.iter().for_each(|v| numbers(v, out)),
            Val::Const(of) => numbers(of, out),
            _ => {}
        }
    }

    /// A NaN of the plugin's is told apart from another by its bits, and written as the canonical
    /// one: a file holds an entry for each NaN of other bits, all of the same bytes. The reader
    /// tells them apart by their places, in an array, a struct and a constant as on their own, so
    /// that they are written again as they were; a NaN met twice is one entry, and zero and
    /// negative zero are two, with no place.
    #[test]
    fn keeps_the_nans_of_a_file_apart() {
        let mut cx = Cx::new();
        let (name, _) = member(&mut cx, b"F1v");
        let (float, double) = (cx.ty(Type::Float), cx.ty(Type::Double));
        // `Float.NaN` and `0.0f / 0.0f` as a JVM on x86 makes it, and the same of doubles.
        let (f1, f2) = (Val::Float(0x7fc0_0000, None), Val::Float(0xffc0_0000, None));
        let (d1, d2) = (Val::Double(0x7ff8_0000_0000_0000, None), Val::Double(0xfff8_0000_0000_0000, None));
        let zeros = [Val::Float(0, None), Val::Float(0x8000_0000, None), Val::Double(0, None), Val::Double(1 << 63, None)];
        let var = |rhs| Defn { attrs: Attrs::default(), name, pos: NOWHERE, kind: DefnKind::Var { ty: float, rhs } };
        let defns = [
            var(Val::ArrayValue(double, Arc::new([d2.clone(), d1.clone(), d2.clone()]))),
            var(Val::StructValue(Arc::new([f1.clone(), f2.clone(), Val::Const(Arc::new(Val::ArrayValue(float, Arc::new([f2, f1]))))]))),
            var(Val::Const(Arc::new(d1))),
            var(Val::StructValue(Arc::new(zeros))),
        ];
        let written = file(&cx, &defns, REVISION).unwrap();
        assert_eq!(count(&written, &[10, 0x7f, 0xc0, 0, 0]), 2);
        assert_eq!(count(&written, &[11, 0x7f, 0xf8, 0, 0, 0, 0, 0, 0]), 2);
        let back = read(&mut cx, &written);
        let mut met = Vec::new();
        for defn in &back {
            let DefnKind::Var { rhs, .. } = &defn.kind else { panic!("no variable read back") };
            numbers(rhs, &mut met);
        }
        let met: Vec<(bool, Option<u32>)> = met
            .into_iter()
            .map(|v| match *v {
                Val::Float(bits, place) => (f32::from_bits(bits).is_nan(), place),
                Val::Double(bits, place) => (f64::from_bits(bits).is_nan(), place),
                _ => unreachable!(),
            })
            .collect();
        // Only a NaN has a place.
        assert!(met.iter().all(|&(nan, place)| nan == place.is_some()), "{:?}", met);
        // d2 d1 d2, f1 f2 f2 f1, d1, and the zeros.
        let place = |i: usize| met[i].1;
        assert_eq!((place(0), place(4), place(3)), (place(2), place(5), place(6)));
        assert_eq!(place(1), place(7));
        assert!(place(0) != place(1) && place(3) != place(4));
        assert_eq!(file(&cx, &back, REVISION).unwrap(), written);
    }

    /// A position in no file and one in the file at the empty path are two entries of the same
    /// bytes, which stand here for the plugin's two by places, as the reader tells them apart; a
    /// position in a file has no place.
    #[test]
    fn keeps_the_positions_in_no_file_apart() {
        let mut cx = Cx::new();
        let (name, ty) = member(&mut cx, b"D1fuEO");
        let named = Pos { file: cx.str(b"P.scala"), ..NOWHERE };
        let (first, second) = (Pos { place: NonZeroU32::new(1), ..NOWHERE }, Pos { place: NonZeroU32::new(2), ..NOWHERE });
        let insts = vec![Inst { pos: second, kind: InstKind::Label { id: 0, params: Box::new([]) } }, Inst { pos: named, kind: InstKind::Ret(Val::Unit) }];
        let defns = [Defn { attrs: Attrs::default(), name, pos: first, kind: DefnKind::Define { ty, insts, debug: DebugInfo::NONE } }];
        let written = file(&cx, &defns, REVISION).unwrap();
        // The positions' section: two in no file, the empty string being the third string, after the class's
        // name and the signature, and the one in a file.
        let word = |k: usize| u32::from_be_bytes(written.chunks_exact(4).nth(k).unwrap().try_into().unwrap()) as usize;
        assert_eq!(written[word(5)..word(6)], [9, 0, 0, 9, 0, 0, 10, 0, 0]);
        let back = read(&mut cx, &written);
        let DefnKind::Define { insts, .. } = &back[0].kind else { panic!("no method read back") };
        let (first, second, named) = (back[0].pos, insts[0].pos, insts[1].pos);
        assert!(first.place.is_some() && second.place.is_some() && first != second);
        assert_eq!((named.file, named.place), (cx.str(b"P.scala"), None));
        assert_eq!(file(&cx, &back, REVISION).unwrap(), written);
    }

    /// A file written from the definitions of two: a number is one entry by its bits, wherever
    /// each file had its own, since only a NaN has its place.
    #[test]
    fn writes_a_number_of_two_files_once() {
        let mut cx = Cx::new();
        let (name, _) = member(&mut cx, b"F1v");
        let float = cx.ty(Type::Float);
        let var = |rhs| Defn { attrs: Attrs::default(), name, pos: NOWHERE, kind: DefnKind::Var { ty: float, rhs } };
        let half = |n: f32| Val::Float((n + 0.5).to_bits(), None);
        let first = file(&cx, &[var(Val::ArrayValue(float, Arc::new([half(2.0), half(1.0)])))], REVISION).unwrap();
        let second = file(&cx, &[var(half(1.0))], REVISION).unwrap();
        let both: Vec<Defn> = [first, second].iter().flat_map(|f| read(&mut cx, f)).collect();
        assert_eq!(count(&file(&cx, &both, REVISION).unwrap(), &[10, 0x3f, 0xc0, 0, 0]), 1);
    }

    #[test]
    fn refuses_what_no_file_holds() {
        let mut cx = Cx::new();
        let (name, _) = member(&mut cx, b"D1fuEO");
        let virtual_type = cx.ty(Type::Virtual);
        let var = |ty, rhs| [Defn { attrs: Attrs::default(), name, pos: NOWHERE, kind: DefnKind::Var { ty, rhs } }];
        assert_eq!(file(&cx, &var(virtual_type, Val::Null), REVISION), Err("M1AD1fuEO: a virtual type, which no file holds".to_string()));
        let size = cx.ty(Type::Size);
        assert_eq!(file(&cx, &var(size, Val::Size(-1)), REVISION), Err("M1AD1fuEO: a local or a size beyond 63 bits".to_string()));
        assert!(file(&cx, &var(size, Val::Size(i64::MAX)), REVISION).is_ok());
    }
}
