//! The lowered program as LLVM IR, after `codegen.llvm.AbstractCodeGen` and `CodeGen` of Scala
//! Native 0.5.12 with its debug metadata off: one module for each directory of sources, a
//! function for each method, a block for each label and each place a call that can throw
//! returns to, a landing pad for each such call, and a phi for each parameter of a block. The
//! text is Scala Native's own, byte for byte, so that it can be held against it.

use super::meta::{self, Meta};
use super::nir::names::{self, Rt};
use super::nir::*;
use super::reach::{self, Info, Linked, ScopeId};
use super::scala;
use crate::intern::{FxBuild, FxMap};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

type FxSet<T> = HashSet<T, FxBuild>;
type Res<T> = Result<T, String>;

/// A module: the name of its file without the extension, and its text.
pub struct Module {
    pub name: String,
    pub text: Vec<u8>,
}

struct Edge {
    from: usize,
    next: Next,
}

struct Block {
    id: Local,
    params: Box<[(Local, Ty)]>,
    start: usize,
    end: usize,
    in_edges: Vec<Edge>,
    /// The calls of the block that can throw: each ends a piece of the block.
    split_count: u32,
    reachable: bool,
}

fn next_label(next: &Next) -> Option<Local> {
    match next {
        Next::None => None,
        Next::Unwind { next, .. } | Next::Case(_, next) => next_label(next),
        Next::Label(id, _) => Some(*id),
    }
}

/// `ControlFlow.Graph`, with the edges in the order it finds them: a block is visited when it
/// comes off a stack it was put on where it was first named.
fn graph(insts: &[Inst]) -> Res<Vec<Block>> {
    let is_cf = |inst: &Inst| !matches!(inst.kind, InstKind::Label { .. } | InstKind::Let { .. });
    let mut blocks: Vec<Block> = Vec::new();
    let mut index: FxMap<Local, usize> = FxMap::default();
    for (i, inst) in insts.iter().enumerate() {
        if let InstKind::Label { id, params } = &inst.kind {
            let end = (i + 1..insts.len()).find(|&j| is_cf(&insts[j])).map_or(insts.len(), |j| j + 1);
            let split_count = insts[i + 1..end].iter().filter(|inst| matches!(&inst.kind, InstKind::Let { op: Op::Call { .. }, unwind, .. } if *unwind != Next::None)).count() as u32;
            index.insert(*id, blocks.len());
            blocks.push(Block { id: *id, params: params.clone(), start: i + 1, end, in_edges: Vec::new(), split_count, reachable: false });
        }
    }
    if blocks.first().is_none_or(|b| b.start != 1) {
        return Err("a method that starts with no label".to_string());
    }
    let mut named = vec![false; blocks.len()];
    let mut todo = vec![0usize];
    named[0] = true;
    while let Some(b) = todo.pop() {
        if std::mem::replace(&mut blocks[b].reachable, true) {
            continue;
        }
        let mut out: Vec<Next> = Vec::new();
        let (start, end) = (blocks[b].start, blocks[b].end);
        for (i, inst) in insts[start..end].iter().enumerate() {
            let last = start + i + 1 == end;
            match &inst.kind {
                InstKind::Let { unwind, .. } if !last && *unwind != Next::None => out.push(unwind.clone()),
                InstKind::Jump(next) if last => out.push(next.clone()),
                InstKind::If { then, otherwise, .. } if last => out.extend([then.clone(), otherwise.clone()]),
                InstKind::Switch { default, cases, .. } if last => {
                    out.push(default.clone());
                    out.extend(cases.iter().cloned());
                }
                InstKind::Throw { unwind, .. } | InstKind::Unreachable { unwind } if last && *unwind != Next::None => out.push(unwind.clone()),
                _ => {}
            }
        }
        for next in out {
            let label = next_label(&next).ok_or("a jump to no label")?;
            let &to = index.get(&label).ok_or("a jump to a label the method has not")?;
            if !std::mem::replace(&mut named[to], true) {
                todo.push(to);
            }
            blocks[to].in_edges.push(Edge { from: b, next });
        }
    }
    Ok(blocks)
}

/// Whether Scala Native's code generator takes two values for one (`nir.Val`'s equality over what
/// it reads): by value, a number by its bits, which are the canonical ones for every NaN it reads
/// whatever its entry, but a byte string by the array that holds it, never by its bytes. The
/// reader holds one array for each place of a file, so two literals of the same bytes are two.
fn same(a: &Val, b: &Val) -> bool {
    match (a, b) {
        (Val::ByteString(x, _), Val::ByteString(y, _)) => Arc::ptr_eq(x, y),
        (Val::Float(x, _), Val::Float(y, _)) => x == y,
        (Val::Double(x, _), Val::Double(y, _)) => x == y,
        (Val::StructValue(xs), Val::StructValue(ys)) => all_same(xs, ys),
        (Val::ArrayValue(t, xs), Val::ArrayValue(u, ys)) => t == u && all_same(xs, ys),
        (Val::Const(x), Val::Const(y)) => same(x, y),
        _ => a == b,
    }
}

fn all_same(xs: &[Val], ys: &[Val]) -> bool {
    xs.len() == ys.len() && xs.iter().zip(ys.iter()).all(|(x, y)| same(x, y))
}

/// A constant as Scala Native's table of a module's constants tells two apart: by `same`.
struct Constant(Val);

impl PartialEq for Constant {
    fn eq(&self, other: &Self) -> bool {
        same(&self.0, &other.0)
    }
}

impl Eq for Constant {}

impl Hash for Constant {
    fn hash<H: Hasher>(&self, state: &mut H) {
        fn hash<H: Hasher>(v: &Val, state: &mut H) {
            std::mem::discriminant(v).hash(state);
            match v {
                Val::ByteString(bytes, _) => Arc::as_ptr(bytes).cast::<u8>().hash(state),
                Val::Float(bits, _) => bits.hash(state),
                Val::Double(bits, _) => bits.hash(state),
                Val::StructValue(vs) | Val::ArrayValue(_, vs) => vs.iter().for_each(|v| hash(v, state)),
                Val::Const(inner) => hash(inner, state),
                _ => v.hash(state),
            }
        }
        hash(&self.0, state)
    }
}

/// The definitions a module holds and what all the modules know of each other.
pub struct Codegen<'a> {
    cx: &'a Cx,
    triple: &'a str,
    rt: &'a Rt,
    linked: &'a Linked,
    meta: &'a Meta,
    env: &'a FxMap<Global, &'a Defn>,
    object: ScopeId,
    out: Vec<u8>,
    indentation: usize,
    copies: FxMap<Local, Val>,
    deps: scala::HashSet<Global>,
    /// The names written so far: two symbols of C's linkage can have one.
    generated: FxSet<String>,
    extern_sigs: FxMap<Str, Global>,
    consts: FxMap<Constant, u32>,
    const_order: Vec<Val>,
    fresh: Local,
    block_name: Local,
    block_split: u32,
}

/// The names of the modules of directories sorted by their bytes: the hexadecimal `String.hashCode`
/// of the directory, Scala Native's name, where no other directory has its hash; of directories of
/// one hash the first keeps it and the next are `<hash>-1`, `<hash>-2`, ... (docs/NATIVE.md, "The
/// order of a link").
fn module_names(dirs: &[&[u8]]) -> Vec<String> {
    let mut taken: FxMap<u32, u32> = FxMap::default();
    dirs.iter()
        .map(|dir| {
            let hash = scala::string_hash(dir) as u32;
            let before = taken.entry(hash).or_insert(0);
            let name = if *before == 0 { format!("{:x}", hash) } else { format!("{:x}-{}", hash, before) };
            *before += 1;
            name
        })
        .collect()
}

/// The modules of a lowered program for a target: one for each directory its definitions'
/// sources lie in, named as `module_names` says.
pub fn modules(cx: &Cx, rt: &Rt, linked: &Linked, meta: &Meta, lowered: &[Defn], triple: &str) -> Res<Vec<Module>> {
    let env: FxMap<Global, &Defn> = lowered.iter().map(|d| (d.name, d)).collect();
    let object = match linked.infos.get(&rt.object) {
        Some(&Info::Scope(id)) => id,
        _ => return Err("java.lang.Object is not linked".to_string()),
    };
    let mut by_dir: FxMap<&[u8], Vec<&Defn>> = FxMap::default();
    for defn in lowered {
        let file = cx.bytes(defn.pos.file);
        let dir: &[u8] = match file.iter().rposition(|&b| b == b'/') {
            Some(0) => b"/",
            Some(at) => &file[..at],
            None => b"__empty",
        };
        by_dir.entry(dir).or_default().push(defn);
    }
    let mut dirs: Vec<(&[u8], Vec<&Defn>)> = by_dir.into_iter().collect();
    dirs.sort_by(|a, b| a.0.cmp(b.0));
    let names = module_names(&dirs.iter().map(|(dir, _)| *dir).collect::<Vec<_>>());
    let mut out = Vec::with_capacity(dirs.len());
    for ((_, mut defns), name) in dirs.into_iter().zip(names) {
        defns.sort_by(|a, b| cx.cmp_globals(a.name, b.name));
        let mut writer = Codegen {
            cx,
            triple,
            rt,
            linked,
            meta,
            env: &env,
            object,
            out: Vec::new(),
            indentation: 0,
            copies: FxMap::default(),
            // `AbstractCodeGen`'s `deps.sizeHint(1024)`.
            deps: scala::HashSet::with_size_hint(1024),
            generated: FxSet::default(),
            extern_sigs: FxMap::default(),
            consts: FxMap::default(),
            const_order: Vec::new(),
            fresh: 0,
            block_name: 0,
            block_split: 0,
        };
        let text = writer.module(&defns)?;
        out.push(Module { name, text });
    }
    Ok(out)
}

impl<'a> Codegen<'a> {
    fn str(&mut self, s: &str) {
        self.out.extend_from_slice(s.as_bytes());
    }

    fn num(&mut self, n: impl std::fmt::Display) {
        use std::io::Write;
        let _ = write!(self.out, "{}", n);
    }

    fn newline(&mut self) {
        self.out.push(b'\n');
        for _ in 0..self.indentation {
            self.out.extend_from_slice(b"  ");
        }
    }

    fn line(&mut self, s: &str) {
        self.newline();
        self.str(s);
    }

    /// A module's text: what it declares (the prelude, its constants, the definitions of
    /// other modules it refers to), then its definitions, then the module's flags. The
    /// declarations are known once the definitions are written.
    fn module(&mut self, defns: &[&'a Defn]) -> Res<Vec<u8>> {
        let kinds: [fn(&DefnKind) -> bool; 4] = [
            |k| matches!(k, DefnKind::Const { .. }),
            |k| matches!(k, DefnKind::Var { .. }),
            |k| matches!(k, DefnKind::Declare { .. }),
            |k| matches!(k, DefnKind::Define { .. }),
        ];
        for is_kind in kinds {
            for &defn in defns {
                if is_kind(&defn.kind) && self.generated.insert(self.mangled(defn.name)) {
                    self.newline();
                    self.defn(defn, false)?;
                }
            }
        }
        let body = std::mem::take(&mut self.out);
        // `genPrelude`: Scala Native names the target where its build is given one, and teq's
        // link always has one.
        let triple = self.triple;
        self.str("target triple = \"");
        self.str(triple);
        self.str("\"\ndeclare i32 @scalanative_personality(...)\ndeclare ptr @scalanative_catch(ptr)\n");
        // A constant's name is `__const` and its number: they stand in the order of the names'
        // text, a number of two digits after every number of one.
        let mut consts: Vec<u32> = (0..self.const_order.len() as u32).collect();
        consts.sort_by_key(|&i| (i.to_string().len(), i.to_string()));
        for i in consts {
            self.newline();
            self.str("@");
            self.const_name(i);
            self.str(" = private unnamed_addr constant ");
            let value = self.const_order[i as usize].clone();
            self.val(&value)?;
        }
        for dep in self.deps.iter() {
            if !self.generated.insert(self.mangled(dep)) {
                continue;
            }
            self.newline();
            let defn = *self.env.get(&dep).ok_or_else(|| format!("{} is referred to and not defined", self.mangled(dep)))?;
            self.defn(defn, true)?;
        }
        self.out.extend_from_slice(&body);
        self.str("\n!1 = !{i32 8, !\"PIC Level\", i32 2}\n!2 = !{i32 7, !\"PIE Level\", i32 2}\n!llvm.module.flags = !{!1, !2}");
        Ok(std::mem::take(&mut self.out))
    }

    fn mangled(&self, g: Global) -> String {
        let sig = self.cx.sig_bytes(g);
        if names::is_extern(sig) {
            return String::from_utf8_lossy(names::sig_id(sig).unwrap_or(b"")).into_owned();
        }
        let mut out = b"_S".to_vec();
        self.cx.mangle(g, &mut out);
        String::from_utf8_lossy(&out).into_owned()
    }

    fn global(&mut self, g: Global) {
        self.out.push(b'"');
        let sig = self.cx.sig_bytes(g);
        if names::is_extern(sig) {
            self.out.extend_from_slice(names::sig_id(sig).unwrap_or(b""));
        } else {
            self.out.extend_from_slice(b"_S");
            self.cx.mangle(g, &mut self.out);
        }
        self.out.push(b'"');
    }

    fn const_name(&mut self, index: u32) {
        let digits = index.to_string();
        self.str("\"_SM7__constG");
        self.num(digits.len());
        self.str("-");
        self.str(&digits);
        self.str("\"");
    }

    fn local(&mut self, id: Local) {
        self.str("_");
        self.num(id);
    }

    fn fresh(&mut self) -> Local {
        self.fresh += 1;
        self.fresh
    }

    // ---- types ----

    fn ty(&mut self, ty: Ty) {
        match self.cx.type_of(ty) {
            Type::Vararg => self.str("..."),
            Type::Unit => self.str("void"),
            Type::Null | Type::Array { .. } | Type::Ref { .. } | Type::Ptr | Type::Nothing => self.str("ptr"),
            Type::Bool => self.str("i1"),
            Type::Char | Type::Short => self.str("i16"),
            Type::Byte => self.str("i8"),
            Type::Int => self.str("i32"),
            Type::Long | Type::Size => self.str("i64"),
            Type::Int128 => self.str("i128"),
            Type::Float => self.str("float"),
            Type::Double => self.str("double"),
            Type::ArrayValue(of, n) => {
                self.str("[");
                self.num(*n);
                self.str(" x ");
                self.ty(*of);
                self.str("]");
            }
            Type::StructValue(tys) => {
                self.str("{ ");
                for (i, &t) in tys.iter().enumerate() {
                    if i > 0 {
                        self.str(", ");
                    }
                    self.ty(t);
                }
                self.str(" }");
            }
            Type::Function(args, ret) => {
                self.ty(*ret);
                self.str(" (");
                for (i, &t) in args.iter().enumerate() {
                    if i > 0 {
                        self.str(", ");
                    }
                    self.ty(t);
                }
                self.str(")");
            }
            Type::Virtual | Type::Var(_) => self.str("ptr"),
        }
    }

    fn is_ref(&self, ty: Ty) -> bool {
        matches!(self.cx.type_of(ty), Type::Null | Type::Unit | Type::Array { .. } | Type::Ref { .. })
    }

    /// What LLVM may assume of a reference: so many bytes behind it, and that it is not null
    /// where its type says so.
    fn dereferenceable(&self, ty: Ty) -> Res<(bool, &'static str, i64)> {
        let name = self.rt.class_name(self.cx, ty).ok_or("no reference type")?;
        let cls = match self.linked.infos.get(&name) {
            Some(&Info::Scope(id)) if self.linked.scopes[id as usize].is_trait => self.object,
            Some(&Info::Scope(id)) => id,
            _ => return Err(format!("{} is not linked", self.mangled(name))),
        };
        let size = self.meta.layouts[&cls].size;
        let nullable = match self.cx.type_of(ty) {
            Type::Null => true,
            Type::Array { nullable, .. } | Type::Ref { nullable, .. } => *nullable,
            _ => false,
        };
        Ok(if nullable { (false, "dereferenceable_or_null", size) } else { (true, "dereferenceable", size) })
    }

    fn return_type(&mut self, ret: Ty) -> Res<()> {
        if self.is_ref(ret) && ret != self.rt.unit {
            let (nonnull, deref, size) = self.dereferenceable(ret)?;
            if nonnull {
                self.str("nonnull ");
            }
            self.str(deref);
            self.str("(");
            self.num(size);
            self.str(") ");
        }
        self.ty(ret);
        Ok(())
    }

    /// The type of a value, as `genType(value.ty)` writes it.
    fn val_ty(&mut self, v: &Val) {
        match v {
            Val::Zero(ty) | Val::Local(_, ty) | Val::Global(_, ty) => self.ty(*ty),
            Val::True | Val::False => self.str("i1"),
            Val::Char(_) | Val::Short(_) => self.str("i16"),
            Val::Byte(_) => self.str("i8"),
            Val::Int(_) => self.str("i32"),
            Val::Size(_) | Val::Long(_) => self.str("i64"),
            Val::Int128 { .. } => self.str("i128"),
            Val::Float(..) => self.str("float"),
            Val::Double(..) => self.str("double"),
            Val::StructValue(vs) => {
                self.str("{ ");
                for (i, v) in vs.iter().enumerate() {
                    if i > 0 {
                        self.str(", ");
                    }
                    self.val_ty(v);
                }
                self.str(" }");
            }
            Val::ArrayValue(of, vs) => {
                self.str("[");
                self.num(vs.len());
                self.str(" x ");
                self.ty(*of);
                self.str("]");
            }
            Val::ByteString(bytes, _) => {
                self.str("[");
                self.num(bytes.len() + 1);
                self.str(" x i8]");
            }
            Val::Unit => self.str("void"),
            Val::Null | Val::Const(_) | Val::String(_) | Val::Virtual(_) | Val::ClassOf(_) => self.str("ptr"),
        }
    }

    fn val_is_ref(&self, v: &Val) -> Option<Ty> {
        match v {
            Val::Local(_, ty) if self.is_ref(*ty) => Some(*ty),
            _ => None,
        }
    }

    // ---- values ----

    /// `deconstify`: a copy's local is the value copied, and a constant is the global that
    /// holds it, one for each distinct constant of the module.
    fn deconstify(&mut self, v: &Val) -> Val {
        match v {
            Val::Local(id, _) => match self.copies.get(id).cloned() {
                Some(copied) => self.deconstify(&copied),
                None => v.clone(),
            },
            Val::StructValue(vs) => Val::StructValue(vs.iter().map(|v| self.deconstify(v)).collect()),
            Val::ArrayValue(ty, vs) => Val::ArrayValue(*ty, vs.iter().map(|v| self.deconstify(v)).collect()),
            Val::Const(inner) => {
                let value = Constant(self.deconstify(inner));
                let index = match self.consts.get(&value) {
                    Some(&i) => i,
                    None => {
                        let i = self.const_order.len() as u32;
                        self.const_order.push(value.0.clone());
                        self.consts.insert(value, i);
                        i
                    }
                };
                Val::Virtual(index as i64)
            }
            _ => v.clone(),
        }
    }

    fn just_val(&mut self, v: &Val) -> Res<()> {
        let v = self.deconstify(v);
        self.plain_val(&v)
    }

    fn plain_val(&mut self, v: &Val) -> Res<()> {
        match v {
            Val::True => self.str("true"),
            Val::False => self.str("false"),
            Val::Null => self.str("null"),
            Val::Unit => self.str("void"),
            Val::Zero(_) => self.str("zeroinitializer"),
            Val::Byte(b) => self.num(*b),
            Val::Size(s) => self.num(*s),
            Val::Char(c) => self.num(*c),
            Val::Short(s) => self.num(*s),
            Val::Int(i) => self.num(*i),
            Val::Long(l) => self.num(*l),
            Val::Int128 { hi, lo } => self.num(((*hi as i128) << 64) | (*lo as u64 as i128)),
            Val::Float(bits, _) => {
                self.str("0x");
                let wide = (f32::from_bits(*bits) as f64).to_bits();
                self.str(&format!("{:x}", wide));
            }
            Val::Double(bits, _) => {
                self.str("0x");
                self.str(&format!("{:x}", bits));
            }
            Val::StructValue(vs) => {
                self.str("{ ");
                for (i, v) in vs.iter().enumerate() {
                    if i > 0 {
                        self.str(", ");
                    }
                    self.val(v)?;
                }
                self.str(" }");
            }
            Val::ArrayValue(_, vs) => {
                self.str("[ ");
                for (i, v) in vs.iter().enumerate() {
                    if i > 0 {
                        self.str(", ");
                    }
                    self.val(v)?;
                }
                self.str(" ]");
            }
            Val::ByteString(bytes, _) => {
                self.str("c\"");
                for &b in bytes.iter() {
                    match b {
                        b'\\' => self.str("\\\\"),
                        0x20..=0x7e if b != b'"' => self.out.push(b),
                        _ => self.str(&format!("\\{:02x}", b)),
                    }
                }
                self.str("\\00\"");
            }
            Val::Local(id, _) => {
                self.str("%");
                self.local(*id);
            }
            // A constant of the module, by its number.
            Val::Virtual(index) => {
                self.str("@");
                self.const_name(*index as u32);
            }
            Val::Global(name, _) if !self.cx.is_top(*name) => {
                self.deps.add(*name, reach::global_hash(self.cx, *name));
                self.str("@");
                self.global(*name);
            }
            Val::Global(..) | Val::Const(_) | Val::String(_) | Val::ClassOf(_) => return Err("a value the lowering should have replaced".to_string()),
        }
        Ok(())
    }

    fn val(&mut self, v: &Val) -> Res<()> {
        if *v != Val::Unit {
            self.val_ty(v);
            self.str(" ");
        }
        self.just_val(v)
    }

    fn next(&mut self, next: &Next) -> Res<()> {
        match next {
            Next::Case(v, inner) => {
                self.val(v)?;
                self.str(", label %");
                self.local(next_label(inner).unwrap_or(0));
                self.str(".0");
            }
            Next::Unwind { exc, .. } => {
                self.str("label %_");
                self.num(*exc);
                self.str(".landingpad");
            }
            _ => {
                self.str("label %");
                self.local(next_label(next).unwrap_or(0));
                self.str(".0");
            }
        }
        Ok(())
    }

    // ---- definitions ----

    /// A definition; `declared` writes what another module defines: a variable or a constant
    /// as an external one, a method as its declaration.
    fn defn(&mut self, defn: &'a Defn, declared: bool) -> Res<()> {
        match &defn.kind {
            DefnKind::Var { ty, rhs } | DefnKind::Const { ty, rhs } => {
                let external = declared || defn.attrs.is_extern;
                self.str("@");
                self.global(defn.name);
                self.str(" = ");
                self.str(if external { "external " } else { "hidden " });
                self.str(if matches!(defn.kind, DefnKind::Const { .. }) { "constant " } else { "global " });
                if external {
                    self.ty(*ty);
                } else {
                    self.val(rhs)?;
                }
            }
            DefnKind::Declare { ty } => self.function(defn, *ty, &[])?,
            DefnKind::Define { ty, insts, .. } => self.function(defn, *ty, if declared { &[] } else { insts })?,
            _ => return Err("a class among the lowered definitions".to_string()),
        }
        Ok(())
    }

    fn function(&mut self, defn: &Defn, ty: Ty, insts: &[Inst]) -> Res<()> {
        let Type::Function(args, ret) = self.cx.type_of(ty) else { return Err("a method whose type is no function's".to_string()) };
        let is_decl = insts.is_empty();
        self.newline();
        self.str(if is_decl { "declare " } else { "define " });
        self.return_type(*ret)?;
        self.str(" @");
        self.global(defn.name);
        self.str("(");
        if is_decl {
            for (i, &arg) in args.iter().enumerate() {
                if i > 0 {
                    self.str(", ");
                }
                self.ty(arg);
            }
        } else if let InstKind::Label { params, .. } = &insts[0].kind {
            for (i, &(param, ty)) in params.iter().enumerate() {
                if i > 0 {
                    self.str(", ");
                }
                self.ty(ty);
                self.str(" %");
                self.local(param);
            }
        }
        self.str(")");
        if defn.attrs.opt == Opt::No {
            self.str(" optnone noinline");
        } else {
            match defn.attrs.inline {
                Inline::May => {}
                Inline::Hint => self.str(" inlinehint"),
                Inline::No => self.str(" noinline"),
                Inline::Always => self.str(" alwaysinline"),
            }
        }
        if is_decl {
            return Ok(());
        }
        for inst in insts {
            if let InstKind::Let { id, op: Op::Copy(v), .. } = &inst.kind {
                self.copies.insert(*id, v.clone());
            }
        }
        let blocks = graph(insts)?;
        self.fresh = insts.iter().map(max_local).max().unwrap_or(0);
        self.str(" personality ptr @scalanative_personality {");
        for (b, block) in blocks.iter().enumerate() {
            if block.reachable {
                self.block(&blocks, b, insts)?;
            }
        }
        for block in blocks.iter().filter(|b| b.reachable) {
            for inst in &insts[block.start..block.end] {
                if let InstKind::Let { unwind: Next::Unwind { exc, next, .. }, .. } = &inst.kind {
                    self.landing_pad(*exc, next)?;
                }
            }
        }
        self.newline();
        self.str("}");
        self.copies.clear();
        Ok(())
    }

    /// Where an exception thrown by a call lands: the runtime's `catch` gives the thrown
    /// object, which goes to the handler's block.
    fn landing_pad(&mut self, exc: Local, next: &Next) -> Res<()> {
        let (rec, r0) = (self.fresh(), self.fresh());
        // Four more locals that Scala Native makes for the C++ form of the pad.
        self.fresh += 4;
        self.line(&format!("_{}.landingpad:", exc));
        self.indentation += 1;
        self.line(&format!("%_{} = landingpad {{ ptr, i32 }} catch ptr null", rec));
        self.line(&format!("%_{} = extractvalue {{ ptr, i32 }} %_{}, 0", r0, rec));
        self.line(&format!("br label %_{}.landingpad.succ", exc));
        self.indentation -= 1;
        self.line(&format!("_{}.landingpad.succ:", exc));
        self.indentation += 1;
        self.line(&format!("%_{} = call ptr @scalanative_catch(ptr %_{})", exc, r0));
        self.line("br ");
        self.next(next)?;
        self.indentation -= 1;
        Ok(())
    }

    fn block_header(&mut self) {
        self.newline();
        self.local(self.block_name);
        self.str(".");
        self.num(self.block_split);
        self.str(":");
    }

    fn block(&mut self, blocks: &[Block], b: usize, insts: &[Inst]) -> Res<()> {
        let block = &blocks[b];
        self.block_name = block.id;
        self.block_split = 0;
        self.block_header();
        self.indentation += 1;
        if b != 0 {
            for (n, &(param, ty)) in block.params.iter().enumerate() {
                if ty == self.rt.unit {
                    continue;
                }
                self.newline();
                self.str("%");
                self.local(param);
                self.str(" = phi ");
                self.ty(ty);
                self.str(" ");
                for (i, edge) in block.in_edges.iter().enumerate() {
                    if i > 0 {
                        self.str(", ");
                    }
                    self.str("[");
                    match &edge.next {
                        Next::Unwind { exc, next, .. } => {
                            let Next::Label(_, args) = &**next else { return Err("a handler with no label".to_string()) };
                            self.just_val(args.get(n).ok_or("a jump with too few arguments")?)?;
                            self.str(", %");
                            self.local(*exc);
                            self.str(".landingpad.succ");
                        }
                        Next::Label(..) | Next::Case(..) => {
                            let target = match &edge.next {
                                Next::Case(_, inner) => &**inner,
                                other => other,
                            };
                            let Next::Label(_, args) = target else { return Err("a jump to no label".to_string()) };
                            self.just_val(args.get(n).ok_or("a jump with too few arguments")?)?;
                            self.str(", %");
                            self.local(blocks[edge.from].id);
                            self.str(".");
                            self.num(blocks[edge.from].split_count);
                        }
                        _ => return Err("a jump to no label".to_string()),
                    }
                    self.str("]");
                }
            }
        }
        for inst in &insts[block.start..block.end] {
            self.inst(inst)?;
        }
        self.indentation -= 1;
        Ok(())
    }

    fn inst(&mut self, inst: &Inst) -> Res<()> {
        match &inst.kind {
            InstKind::Let { id, op, unwind, .. } => self.let_(*id, op, unwind)?,
            InstKind::Unreachable { .. } => self.line("unreachable"),
            InstKind::Ret(v) => {
                self.line("ret ");
                self.val(v)?;
            }
            InstKind::Jump(next) => {
                self.line("br ");
                self.next(next)?;
            }
            InstKind::If { value, then: Next::Label(then, then_args), otherwise: Next::Label(otherwise, else_args) } if then == otherwise => {
                // Both ways lead to one block: a jump, with the arguments selected.
                let args: Box<[Val]> = if all_same(then_args, else_args) {
                    then_args.clone()
                } else {
                    let mut selected = Vec::new();
                    for (t, e) in then_args.iter().zip(else_args.iter()) {
                        let id = self.fresh();
                        self.newline();
                        self.str("%");
                        self.local(id);
                        self.str(" = select ");
                        self.val(value)?;
                        self.str(", ");
                        self.val(t)?;
                        self.str(", ");
                        self.val(e)?;
                        selected.push(Val::Local(id, self.rt.ptr));
                    }
                    selected.into_boxed_slice()
                };
                self.line("br ");
                self.next(&Next::Label(*then, args))?;
            }
            InstKind::If { value, then, otherwise } => {
                self.line("br ");
                self.val(value)?;
                self.str(", ");
                self.next(then)?;
                self.str(", ");
                self.next(otherwise)?;
            }
            InstKind::Switch { value, default, cases } => {
                self.line("switch ");
                self.val(value)?;
                self.str(", ");
                self.next(default)?;
                self.str(" [");
                self.indentation += 1;
                for case in cases.iter() {
                    self.newline();
                    self.next(case)?;
                }
                self.indentation -= 1;
                self.line("]");
            }
            _ => return Err("an instruction the lowering should have replaced".to_string()),
        }
        Ok(())
    }

    fn result_is_void(&self, op: &Op) -> bool {
        match op {
            Op::Call { ty, .. } => matches!(self.cx.type_of(*ty), Type::Function(_, ret) if matches!(self.cx.type_of(*ret), Type::Unit | Type::Nothing)),
            Op::Load { ty, .. } | Op::Bin { ty, .. } | Op::Conv { ty, .. } => matches!(self.cx.type_of(*ty), Type::Unit | Type::Nothing),
            Op::Store { .. } | Op::Fence(_) => true,
            _ => false,
        }
    }

    fn bind(&mut self, id: Local, op: &Op) {
        if !self.result_is_void(op) {
            self.str("%");
            self.local(id);
            self.str(" = ");
        }
    }

    fn memory_order(&mut self, order: MemoryOrder) {
        self.str(match order {
            MemoryOrder::Unordered => "unordered",
            MemoryOrder::Monotonic => "monotonic",
            MemoryOrder::Acquire => "acquire",
            MemoryOrder::Release => "release",
            MemoryOrder::AcqRel => "acq_rel",
            MemoryOrder::SeqCst => "seq_cst",
        });
    }

    fn let_(&mut self, id: Local, op: &Op, unwind: &Next) -> Res<()> {
        match op {
            Op::Copy(_) => {}
            Op::Call { ty, ptr, args } => {
                // Two extern objects may declare one C function with two types: the module
                // declares it once, and a call under the other type goes through its pointer.
                let ptr = match ptr {
                    Val::Global(member, val_ty) if names::is_extern(self.cx.sig_bytes(*member)) => {
                        let sig = self.cx.sig(*member).unwrap_or(Str::EMPTY);
                        Val::Global(*self.extern_sigs.entry(sig).or_insert(*member), *val_ty)
                    }
                    _ => ptr.clone(),
                };
                self.call(id, op, *ty, &ptr, args, unwind)?;
            }
            Op::Load { ty, ptr, order } => {
                self.fresh();
                let atomic = self.meta.multithreading && order.is_some();
                self.newline();
                self.bind(id, op);
                self.str("load ");
                if atomic {
                    self.str("atomic ");
                }
                self.ty(*ty);
                self.str(", ");
                self.val(ptr)?;
                if let (true, Some(order)) = (atomic, order) {
                    self.str(" ");
                    self.memory_order(*order);
                    self.str(", align ");
                    self.num(meta::alignment_of(self.cx, *ty));
                } else if self.is_ref(*ty) {
                    let (nonnull, deref, size) = self.dereferenceable(*ty)?;
                    if nonnull {
                        self.str(", !nonnull !{}");
                    }
                    self.str(", !");
                    self.str(deref);
                    self.str(" !{i64 ");
                    self.num(size);
                    self.str("}");
                }
            }
            Op::Store { ty, ptr, value, order } => {
                self.fresh();
                let atomic = self.meta.multithreading && order.is_some();
                self.newline();
                self.bind(id, op);
                self.str("store ");
                if atomic {
                    self.str("atomic ");
                }
                self.val(value)?;
                self.str(", ptr");
                self.just_val(ptr)?;
                if let (true, Some(order)) = (atomic, order) {
                    self.str(" ");
                    self.memory_order(*order);
                }
                self.str(", align ");
                self.num(meta::alignment_of(self.cx, *ty));
            }
            Op::Elem { ty, ptr, indexes } => {
                self.fresh += 2;
                self.newline();
                self.bind(id, op);
                self.str("getelementptr ");
                self.ty(*ty);
                self.str(", ");
                if matches!(self.cx.type_of(*ty), Type::ArrayValue(..) | Type::StructValue(_)) {
                    self.ty(*ty);
                    self.str("*");
                } else {
                    self.str("ptr");
                }
                self.str(" ");
                self.just_val(ptr)?;
                self.str(", ");
                for (i, index) in indexes.iter().enumerate() {
                    if i > 0 {
                        self.str(", ");
                    }
                    self.val(index)?;
                }
            }
            Op::Stackalloc { ty, n } => {
                self.fresh();
                self.newline();
                self.bind(id, op);
                self.str("alloca ");
                self.ty(*ty);
                self.str(", ");
                self.val(n)?;
                self.str(", align ");
                self.num(meta::PTR_SIZE);
            }
            Op::Extract { aggr, indexes } => {
                self.newline();
                self.bind(id, op);
                self.str("extractvalue ");
                self.val(aggr)?;
                self.str(", ");
                self.str(&indexes.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", "));
            }
            Op::Insert { aggr, value, indexes } => {
                self.newline();
                self.bind(id, op);
                self.str("insertvalue ");
                self.val(aggr)?;
                self.str(", ");
                self.val(value)?;
                self.str(", ");
                self.str(&indexes.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", "));
            }
            Op::Bin { bin, l, r, .. } => {
                self.newline();
                self.bind(id, op);
                self.str(match bin {
                    Bin::Iadd => "add",
                    Bin::Fadd => "fadd",
                    Bin::Isub => "sub",
                    Bin::Fsub => "fsub",
                    Bin::Imul => "mul",
                    Bin::Fmul => "fmul",
                    Bin::Sdiv => "sdiv",
                    Bin::Udiv => "udiv",
                    Bin::Fdiv => "fdiv",
                    Bin::Srem => "srem",
                    Bin::Urem => "urem",
                    Bin::Frem => "frem",
                    Bin::Shl => "shl",
                    Bin::Lshr => "lshr",
                    Bin::Ashr => "ashr",
                    Bin::And => "and",
                    Bin::Or => "or",
                    Bin::Xor => "xor",
                });
                self.str(" ");
                self.val(l)?;
                self.str(", ");
                self.just_val(r)?;
            }
            Op::Comp { comp, l, r, .. } => {
                self.newline();
                self.bind(id, op);
                self.str(match comp {
                    Comp::Ieq => "icmp eq",
                    Comp::Ine => "icmp ne",
                    Comp::Ult => "icmp ult",
                    Comp::Ule => "icmp ule",
                    Comp::Ugt => "icmp ugt",
                    Comp::Uge => "icmp uge",
                    Comp::Slt => "icmp slt",
                    Comp::Sle => "icmp sle",
                    Comp::Sgt => "icmp sgt",
                    Comp::Sge => "icmp sge",
                    Comp::Feq => "fcmp oeq",
                    Comp::Fne => "fcmp une",
                    Comp::Flt => "fcmp olt",
                    Comp::Fle => "fcmp ole",
                    Comp::Fgt => "fcmp ogt",
                    Comp::Fge => "fcmp oge",
                });
                self.str(" ");
                self.val(l)?;
                self.str(", ");
                self.just_val(r)?;
            }
            Op::Conv { conv, ty, value } => {
                self.newline();
                self.bind(id, op);
                let width = |t: &Type| match t {
                    Type::Byte => Some(8),
                    Type::Char | Type::Short => Some(16),
                    Type::Int => Some(32),
                    Type::Long | Type::Size => Some(64),
                    Type::Int128 => Some(128),
                    _ => None,
                };
                let name = match conv {
                    Conv::Trunc => "trunc",
                    Conv::Zext => "zext",
                    Conv::Sext => "sext",
                    Conv::Fptrunc => "fptrunc",
                    Conv::Fpext => "fpext",
                    Conv::Fptoui => "fptoui",
                    Conv::Fptosi => "fptosi",
                    Conv::Uitofp => "uitofp",
                    Conv::Sitofp => "sitofp",
                    Conv::Ptrtoint => "ptrtoint",
                    Conv::Inttoptr => "inttoptr",
                    Conv::Bitcast => "bitcast",
                    Conv::SSizeCast | Conv::ZSizeCast => {
                        let from = match value {
                            Val::Local(_, t) | Val::Global(_, t) | Val::Zero(t) => width(self.cx.type_of(*t)),
                            Val::Byte(_) => Some(8),
                            Val::Char(_) | Val::Short(_) => Some(16),
                            Val::Int(_) => Some(32),
                            Val::Long(_) | Val::Size(_) => Some(64),
                            _ => None,
                        };
                        let to = width(self.cx.type_of(*ty));
                        let (Some(from), Some(to)) = (from, to) else { return Err("a size cast of what is no integer".to_string()) };
                        if from == to {
                            "bitcast"
                        } else if from > to {
                            "trunc"
                        } else if *conv == Conv::ZSizeCast {
                            "zext"
                        } else {
                            "sext"
                        }
                    }
                };
                self.str(name);
                self.str(" ");
                self.val(value)?;
                self.str(" to ");
                self.ty(*ty);
            }
            Op::Fence(order) => {
                self.newline();
                self.str("fence ");
                self.memory_order(*order);
            }
            _ => return Err("an operation the lowering should have replaced".to_string()),
        }
        Ok(())
    }

    fn call(&mut self, id: Local, op: &Op, ty: Ty, ptr: &Val, args: &[Val], unwind: &Next) -> Res<()> {
        let Type::Function(arg_tys, ret) = self.cx.type_of(ty) else { return Err("a call under a type that is no function's".to_string()) };
        let direct = match ptr {
            Val::Global(pointee, _) if !self.cx.is_top(*pointee) => {
                self.deps.add(*pointee, reach::global_hash(self.cx, *pointee));
                let declared = match self.env.get(pointee).map(|d| &d.kind) {
                    Some(DefnKind::Var { ty, .. } | DefnKind::Const { ty, .. } | DefnKind::Declare { ty } | DefnKind::Define { ty, .. }) => Some(*ty),
                    _ => None,
                };
                if declared == Some(ty) { Some(*pointee) } else { None }
            }
            _ => None,
        };
        if let Some(pointee) = direct {
            self.deps.add(pointee, reach::global_hash(self.cx, pointee));
        } else {
            self.fresh();
        }
        self.newline();
        self.bind(id, op);
        self.str(if *unwind != Next::None { "invoke " } else { "call " });
        if arg_tys.iter().any(|&a| matches!(self.cx.type_of(a), Type::Vararg)) {
            self.ty(ty);
        } else {
            self.return_type(*ret)?;
        }
        match direct {
            Some(pointee) => {
                self.str(" @");
                self.global(pointee);
            }
            None => {
                self.str(" ");
                self.just_val(ptr)?;
            }
        }
        self.str("(");
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.str(", ");
            }
            match self.val_is_ref(arg) {
                Some(ref_ty) => {
                    let (nonnull, deref, size) = self.dereferenceable(ref_ty)?;
                    self.str("ptr");
                    if nonnull {
                        self.str(" nonnull");
                    }
                    self.str(" ");
                    self.str(deref);
                    self.str("(");
                    self.num(size);
                    self.str(") ");
                    self.just_val(arg)?;
                }
                None => self.val(arg)?,
            }
        }
        self.str(")");
        if *unwind != Next::None {
            self.str(" to label %");
            self.block_split += 1;
            self.local(self.block_name);
            self.str(".");
            self.num(self.block_split);
            self.str(" unwind ");
            self.next(unwind)?;
            self.indentation -= 1;
            self.block_header();
            self.indentation += 1;
        }
        Ok(())
    }
}

fn max_local(inst: &Inst) -> Local {
    let exc = |n: &Next| if let Next::Unwind { exc, .. } = n { *exc } else { 0 };
    match &inst.kind {
        InstKind::Label { id, params } => params.iter().map(|&(p, _)| p).fold(*id, Local::max),
        InstKind::Let { id, unwind, .. } => (*id).max(exc(unwind)),
        InstKind::Throw { unwind, .. } | InstKind::Unreachable { unwind } => exc(unwind),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "Aa" and "BB" have one `String.hashCode` (0x840), and so has every string of as many of
    /// them; the hashes below are Java's.
    #[test]
    fn directories_of_one_hash_take_numbers_in_their_order() {
        let mut dirs: Vec<&[u8]> = ["main", "BBBB", "collision/BB", "Aa", "AaBB", "collision/Aa", "BB", "AaAa"].map(str::as_bytes).to_vec();
        dirs.sort();
        let names = module_names(&dirs);
        assert_eq!(names, ["840", "1f0080", "1f0080-1", "840-1", "1f0080-2", "3cfacc3d", "3cfacc3d-1", "3305b9"]);
    }

    #[test]
    fn module_names_are_distinct_and_a_unique_hash_is_scala_natives_name() {
        let mut dirs: Vec<Vec<u8>> = ["/", "__empty", "main", "p"].map(|d| d.as_bytes().to_vec()).to_vec();
        for blocks in 1..=4u32 {
            for bits in 0..1u32 << blocks {
                let dir: String = (0..blocks).map(|i| if bits >> i & 1 == 0 { "Aa" } else { "BB" }).collect();
                dirs.push(format!("p/{}", dir).into_bytes());
            }
        }
        dirs.sort();
        let dirs: Vec<&[u8]> = dirs.iter().map(Vec::as_slice).collect();
        let names = module_names(&dirs);
        assert_eq!(names.iter().collect::<HashSet<_>>().len(), dirs.len());
        for (i, dir) in dirs.iter().enumerate() {
            let hash = scala::string_hash(dir) as u32;
            let earlier = dirs[..i].iter().filter(|d| scala::string_hash(d) as u32 == hash).count();
            let alone = dirs.iter().filter(|d| scala::string_hash(d) as u32 == hash).count() == 1;
            let expected = if earlier == 0 { format!("{:x}", hash) } else { format!("{:x}-{}", hash, earlier) };
            assert_eq!(names[i], expected, "{}", String::from_utf8_lossy(dir));
            assert!(!alone || !names[i].contains('-'));
        }
    }

    /// NaNs of other entries, of one file or of two, are one constant, as they are to Scala Native,
    /// which reads each as the canonical one; zero and negative zero are two.
    #[test]
    fn tells_constants_apart_by_the_bits_of_their_numbers() {
        let nan = 0x7ff8_0000_0000_0000;
        let constant = |places: [Option<u32>; 2]| Constant(Val::Const(Arc::new(Val::ArrayValue(Ty(0), Arc::new(places.map(|p| Val::Double(nan, p)))))));
        let hash = |c: &Constant| {
            let mut h = std::hash::DefaultHasher::new();
            c.hash(&mut h);
            h.finish()
        };
        let (a, b) = (constant([Some(3), Some(12)]), constant([Some(12), Some(40)]));
        assert!(a == b && hash(&a) == hash(&b));
        assert!(Constant(Val::Float(0, None)) != Constant(Val::Float(0x8000_0000, None)));
    }
}
