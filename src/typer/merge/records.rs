//! The records' visitors of the merge: one per kind of record,
//! its fields' order defined here once, over what maps the record's references (`Mapping`): the
//! serial walk's `Remap`, which promotes a type made after the fork the first time a record hands
//! it over; the crew's `Renumber`, which reads the published mappings and changes nothing else;
//! the listing's `Lister`, which maps nothing and lists the types made after the fork in the order
//! the records hand them over.

use super::Tables;
use crate::ast::ListRef;
use crate::typer::{Env, Frame, ImportTarget};
use crate::intern::FxMap;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::sync::Arc;

/// What a record's references are mapped through.
pub(in crate::typer) trait Mapping {
    /// The arenas' new ids.
    fn ids(&self) -> &Tables;
    /// A type under the merge.
    fn ty(&self, t: TypeId) -> TypeId;

    #[inline]
    fn sym(&self, s: SymId) -> SymId {
        self.ids().sym(s)
    }
    #[inline]
    fn class(&self, c: ClassId) -> ClassId {
        self.ids().class(c)
    }
    #[inline]
    fn tparam(&self, p: TParamId) -> TParamId {
        self.ids().tparam(p)
    }
    #[inline]
    fn alias(&self, a: AliasId) -> AliasId {
        self.ids().alias(a)
    }
    #[inline]
    fn pkg(&self, p: PkgId) -> PkgId {
        self.ids().pkg(p)
    }
    #[inline]
    fn overload(&self, i: u32) -> u32 {
        self.ids().overloads.map(i)
    }
    #[inline]
    fn expr(&self, e: TExprId) -> TExprId {
        self.ids().expr(e)
    }
    #[inline]
    fn pat(&self, p: TPatId) -> TPatId {
        self.ids().pat(p)
    }
    #[inline]
    fn string(&self, s: StrRef) -> StrRef {
        self.ids().string(s)
    }
    #[inline]
    fn test(&self, t: TestId) -> TestId {
        self.ids().test(t)
    }
    #[inline]
    fn fun(&self, f: FunId) -> FunId {
        self.ids().fun(f)
    }
    #[inline]
    fn tri(&self, i: u32) -> u32 {
        self.ids().tries.map(i)
    }
    #[inline]
    fn expr_list(&self, l: ListRef) -> ListRef {
        self.ids().expr_list(l)
    }
    #[inline]
    fn sym_list(&self, l: ListRef) -> ListRef {
        self.ids().sym_list(l)
    }
    #[inline]
    fn pat_list(&self, l: ListRef) -> ListRef {
        self.ids().pat_list(l)
    }
    #[inline]
    fn stmt_list(&self, l: ListRef) -> ListRef {
        self.ids().stmt_list(l)
    }
    #[inline]
    fn case_list(&self, l: ListRef) -> ListRef {
        self.ids().case_list(l)
    }
    #[inline]
    fn owner(&self, o: Owner) -> Owner {
        match o {
            Owner::Package(p) => Owner::Package(self.pkg(p)),
            Owner::Class(c) => Owner::Class(self.class(c)),
            Owner::Local => Owner::Local,
        }
    }
}

/// The signatures renumbered so far, by the one each replaced: a signature is shared by several
/// records (`value_sigs`, overload sets), and each holder renumbered by the same walk gets the same
/// copy. And the records walked, for the profile.
#[derive(Default)]
pub(in crate::typer) struct SigMemo {
    done: FxMap<*const MethodSig, Arc<MethodSig>>,
    pub records: u64,
}

pub(in crate::typer) fn syms_in<M: Mapping + ?Sized>(m: &M, v: &mut [SymId]) {
    for s in v {
        *s = m.sym(*s);
    }
}

pub(in crate::typer) fn sig_moves<M: Mapping + ?Sized>(m: &M, sig: &MethodSig) -> bool {
    sig.tparams.iter().any(|&p| m.tparam(p) != p) || m.ty(sig.ret) != sig.ret || sig.clauses.iter().any(|c| c.params.iter().any(|p| m.sym(p.sym) != p.sym || m.ty(p.ty) != p.ty))
}

/// The signature under the new ids: the one it was where nothing in it moved, else a copy shared
/// by every record of the walk that held the old one.
pub(in crate::typer) fn sig<M: Mapping + ?Sized>(m: &M, memo: &mut SigMemo, sig: &mut Arc<MethodSig>) {
    memo.records += 1;
    if !sig_moves(m, sig) {
        return;
    }
    if let Some(done) = memo.done.get(&Arc::as_ptr(sig)) {
        *sig = done.clone();
        return;
    }
    let mut moved = (**sig).clone();
    for p in &mut moved.tparams {
        *p = m.tparam(*p);
    }
    moved.ret = m.ty(moved.ret);
    for c in &mut moved.clauses {
        clause(m, c);
    }
    let moved = Arc::new(moved);
    memo.done.insert(Arc::as_ptr(sig), moved.clone());
    *sig = moved;
}

pub(in crate::typer) fn clause<M: Mapping + ?Sized>(m: &M, c: &mut ClauseSig) {
    for p in &mut c.params {
        p.sym = m.sym(p.sym);
        p.ty = m.ty(p.ty);
    }
}

pub(in crate::typer) fn parent_call<M: Mapping + ?Sized>(m: &M, pc: &mut ParentCall) {
    pc.prelude = m.stmt_list(pc.prelude);
    pc.args = m.expr_list(pc.args);
    pc.via = pc.via.map(|s| m.sym(s));
}

/// A symbol's owner, kind, implementing class and signature.
pub(in crate::typer) fn symbol<M: Mapping + ?Sized>(m: &M, memo: &mut SigMemo, s: &mut SymInfo) {
    s.owner = m.owner(s.owner);
    s.kind = match s.kind {
        SymKind::Object(c) => SymKind::Object(m.class(c)),
        SymKind::EnumValue(c) => SymKind::EnumValue(m.class(c)),
        SymKind::Overloaded(i) => SymKind::Overloaded(m.overload(i)),
        k => k,
    };
    s.impl_class = s.impl_class.map(|c| m.class(c));
    if let Some(x) = s.sig.as_mut() {
        sig(m, memo, x);
    }
}

/// A class's subclasses: a parent of the prefix gets the anonymous and local subclasses a body
/// makes, so every class's are renumbered.
pub(in crate::typer) fn subclasses<M: Mapping + ?Sized>(m: &M, c: &mut ClassInfo) {
    for d in &mut c.subclasses {
        *d = m.class(*d);
    }
}

/// A class's info but its subclasses.
pub(in crate::typer) fn class<M: Mapping + ?Sized>(m: &M, c: &mut ClassInfo) {
    c.owner = m.owner(c.owner);
    for p in &mut c.tparams {
        *p = m.tparam(*p);
    }
    for t in &mut c.parents {
        *t = m.ty(*t);
    }
    c.underlying = c.underlying.map(|t| m.ty(t));
    c.declared_self = c.declared_self.map(|t| m.ty(t));
    c.this_type = c.this_type.map(|t| m.ty(t));
    c.superclass = c.superclass.map(|s| m.class(s));
    for s in c.members.values_mut() {
        *s = m.sym(*s);
    }
    syms_in(m, &mut c.member_order);
    for d in c.nested.values_mut() {
        *d = m.class(*d);
    }
    for a in c.type_aliases.values_mut() {
        *a = m.alias(*a);
    }
    c.companion = c.companion.map(|k| m.class(k));
    c.module_sym = c.module_sym.map(|s| m.sym(s));
    for d in &mut c.children {
        *d = m.class(*d);
    }
    for x in &mut c.ctor {
        clause(m, x);
    }
    for syms in &mut c.ctor_syms {
        syms_in(m, syms);
    }
    for (b, t) in &mut c.base_types {
        *b = m.class(*b);
        *t = m.ty(*t);
    }
    syms_in(m, &mut c.extensions);
    syms_in(m, &mut c.givens);
    c.singleton = c.singleton.map(|s| m.sym(s));
    c.local_module = c.local_module.map(|s| m.sym(s));
    c.inner_object = c.inner_object.map(|s| m.sym(s));
    syms_in(m, &mut c.ctors);
    c.primary_ctor = c.primary_ctor.map(|s| m.sym(s));
}

pub(in crate::typer) fn alias<M: Mapping + ?Sized>(m: &M, a: &mut AliasInfo) {
    a.owner = m.owner(a.owner);
    for p in &mut a.tparams {
        *p = m.tparam(*p);
    }
    a.rhs = m.ty(a.rhs);
    a.bounds = a.bounds.map(|(lo, hi)| (m.ty(lo), m.ty(hi)));
}

pub(in crate::typer) fn tparam<M: Mapping + ?Sized>(m: &M, tp: &mut TParamInfo) {
    tp.upper = m.ty(tp.upper);
    tp.lower = m.ty(tp.lower);
}

/// A package's entries, givens and package object; its parent too when `parent`: a package of
/// the prefix gets a body's mirror val and what a std file entered on demand defines. The
/// entries walked, for the profile.
pub(in crate::typer) fn package<M: Mapping + ?Sized>(m: &M, p: &mut PkgInfo, parent: bool) -> u64 {
    for e in p.entries.values_mut() {
        e.term = e.term.map(|s| m.sym(s));
        e.class = e.class.map(|c| m.class(c));
        e.alias = e.alias.map(|a| m.alias(a));
        e.pkg = e.pkg.map(|q| m.pkg(q));
        syms_in(m, &mut e.extensions);
    }
    syms_in(m, &mut p.givens);
    p.package_object = p.package_object.map(|c| m.class(c));
    if parent {
        p.parent = p.parent.map(|q| m.pkg(q));
    }
    1 + p.entries.len() as u64
}

pub(in crate::typer) fn overload<M: Mapping + ?Sized>(m: &M, o: &mut (SymId, Vec<SymId>)) {
    o.0 = m.sym(o.0);
    syms_in(m, &mut o.1);
}

pub(in crate::typer) fn texpr<M: Mapping + ?Sized>(m: &M, e: &mut TExpr) {
    use TExpr::*;
    *e = match *e {
        Str(s) => Str(m.string(s)),
        Local(s) => Local(m.sym(s)),
        Super(t) => Super(match t {
            SuperTarget::Chain => SuperTarget::Chain,
            SuperTarget::Class(c) => SuperTarget::Class(m.class(c)),
            SuperTarget::Mixin(c) => SuperTarget::Mixin(m.class(c)),
        }),
        Static(s) => Static(m.sym(s)),
        Module(c) => Module(m.class(c)),
        Field(r, s) => Field(m.expr(r), m.sym(s)),
        CallStatic(s, args) => CallStatic(m.sym(s), m.expr_list(args)),
        CallMethod(r, s, args) => CallMethod(m.expr(r), m.sym(s), m.expr_list(args)),
        CallClosure(f, args) => CallClosure(m.expr(f), m.expr_list(args)),
        New(c, args) => New(m.class(c), m.expr_list(args)),
        NewVia(s, args) => NewVia(m.sym(s), m.expr_list(args)),
        Lambda(params, body) => Lambda(m.sym_list(params), m.expr(body)),
        If(c, t, f) => If(m.expr(c), m.expr(t), f.map(|f| m.expr(f))),
        While(c, b) => While(m.expr(c), m.expr(b)),
        Block(stmts, v) => Block(m.stmt_list(stmts), m.expr(v)),
        Assign(l, r) => Assign(m.expr(l), m.expr(r)),
        Match(s, cases) => Match(m.expr(s), m.case_list(cases)),
        Prim(op, l, r) => Prim(op, m.expr(l), m.expr(r)),
        Unary(op, x) => Unary(op, m.expr(x)),
        StrConcat(parts) => StrConcat(m.expr_list(parts)),
        ToStr(x, k) => ToStr(m.expr(x), k),
        Js(s, args) => Js(m.string(s), m.expr_list(args)),
        TypeTest(x, t) => TypeTest(m.expr(x), m.test(t)),
        Cast(x, op, t) => Cast(
            m.expr(x),
            match op {
                CastOp::Check(test, erased) => CastOp::Check(m.test(test), m.ty(erased)),
                CastOp::Unbox(test, erased) => CastOp::Unbox(m.test(test), m.ty(erased)),
                CastOp::Written | CastOp::Nothing => op,
            },
            m.ty(t),
        ),
        ClassOf(c) => ClassOf(m.class(c)),
        SeqLit(items) => SeqLit(m.expr_list(items)),
        ArrayLit(items) => ArrayLit(m.expr_list(items)),
        Index(x, i) => Index(m.expr(x), i),
        JsSelect(x, n) => JsSelect(m.expr(x), n),
        ObjLit(items) => ObjLit(m.expr_list(items)),
        Spread(x) => Spread(m.expr(x)),
        Return(x) => Return(m.expr(x)),
        Throw(x, js) => Throw(m.expr(x), js),
        Try(i) => Try(m.tri(i)),
        Splice(x) => Splice(m.expr(x)),
        Int(_) | Long(_) | Double(_) | Bool(_) | Char(_) | Unit | This | JsImport(_) | JsGlobal(..) | Null => *e,
    };
}

pub(in crate::typer) fn tpat<M: Mapping + ?Sized>(m: &M, p: &mut TPat) {
    use TPat::*;
    *p = match *p {
        Wildcard => Wildcard,
        Bind(s, inner) => Bind(m.sym(s), inner.map(|i| m.pat(i))),
        Test(t, ty, inner) => Test(m.test(t), m.ty(ty), m.pat(inner)),
        Equals(e, strict) => Equals(m.expr(e), strict),
        Class(c, ty, fields, pats) => Class(m.class(c), m.ty(ty), m.sym_list(fields), m.pat_list(pats)),
        Alt(alts) => Alt(m.pat_list(alts)),
        Seq(items, rest) => Seq(m.pat_list(items), rest.map(|r| m.pat(r))),
        Unapply(s, call, inner) => Unapply(m.sym(s), m.expr(call), m.pat(inner)),
    };
}

pub(in crate::typer) fn type_test<M: Mapping + ?Sized>(m: &M, t: &mut TypeTest) {
    use TypeTest::*;
    *t = match *t {
        Class(c) => Class(m.class(c)),
        Trait(c) => Trait(m.class(c)),
        Value(e) => Value(m.expr(e)),
        Or(a, b) => Or(m.test(a), m.test(b)),
        And(a, b) => And(m.test(a), m.test(b)),
        Outer(s, t) => Outer(m.sym(s), m.test(t)),
        other => other,
    };
}

pub(in crate::typer) fn stmt<M: Mapping + ?Sized>(m: &M, s: &mut TStmt) {
    *s = match *s {
        TStmt::Expr(e) => TStmt::Expr(m.expr(e)),
        TStmt::Val(v, e) => TStmt::Val(m.sym(v), m.expr(e)),
        TStmt::Fun(f) => TStmt::Fun(m.fun(f)),
        TStmt::Pat(pat, e) => TStmt::Pat(m.pat(pat), m.expr(e)),
    };
}

pub(in crate::typer) fn case<M: Mapping + ?Sized>(m: &M, c: &mut TCase) {
    c.pat = m.pat(c.pat);
    c.guard = c.guard.map(|g| m.expr(g));
    c.body = m.expr(c.body);
}

pub(in crate::typer) fn tri<M: Mapping + ?Sized>(m: &M, t: &mut TTry) {
    t.body = m.expr(t.body);
    t.cases = m.case_list(t.cases);
    t.finalizer = t.finalizer.map(|f| m.expr(f));
}

pub(in crate::typer) fn fun<M: Mapping + ?Sized>(m: &M, f: &mut TFun) {
    f.sym = m.sym(f.sym);
    syms_in(m, &mut f.params);
    for d in &mut f.defaults {
        *d = d.map(|e| m.expr(e));
    }
    f.body = f.body.map(|b| m.expr(b));
}

/// A class body's ids, a stored inline body's class's among them.
pub(in crate::typer) fn tclass<M: Mapping + ?Sized>(m: &M, tc: &mut TClass) {
    tc.id = m.class(tc.id);
    syms_in(m, &mut tc.ctor_params);
    for d in &mut tc.ctor_defaults {
        *d = d.map(|e| m.expr(e));
    }
    tc.parent_args = tc.parent_args.map(|l| m.expr_list(l));
    tc.parent_via = tc.parent_via.map(|s| m.sym(s));
    tc.parent_prelude = m.stmt_list(tc.parent_prelude);
    for init in &mut tc.init {
        *init = match init {
            TInit::Field(s, e) => TInit::Field(m.sym(*s), m.expr(*e)),
            TInit::Stmt(e) => TInit::Stmt(m.expr(*e)),
            TInit::Parent(c, pc) => {
                let mut pc = *pc;
                parent_call(m, &mut pc);
                TInit::Parent(m.class(*c), pc)
            }
        };
    }
    for f in &mut tc.methods {
        *f = m.fun(*f);
    }
    for f in &mut tc.ctors {
        *f = m.fun(*f);
    }
    for (a, b) in &mut tc.forwarders {
        *a = m.sym(*a);
        *b = m.sym(*b);
    }
    for sa in &mut tc.super_accessors {
        sa.of_trait = m.class(sa.of_trait);
        sa.member = m.sym(sa.member);
        sa.target = sa.target.map(|t| m.sym(t));
    }
    for (a, b) in &mut tc.bridges {
        *a = m.sym(*a);
        *b = m.sym(*b);
    }
    for (a, b) in &mut tc.deferred_givens {
        *a = m.sym(*a);
        *b = m.sym(*b);
    }
}

pub(in crate::typer) fn quote<M: Mapping + ?Sized>(m: &M, q: &mut TQuote) {
    q.ty = m.ty(q.ty);
    q.body = q.body.map(|b| m.expr(b));
    for (s, e) in &mut q.holes {
        *s = m.sym(*s);
        *e = m.expr(*e);
    }
    for (t, e) in &mut q.types {
        *t = m.tparam(*t);
        *e = m.expr(*e);
    }
    syms_in(m, &mut q.binders);
    q.quotes = q.quotes.map(|e| m.expr(e));
}

pub(in crate::typer) fn quote_pat<M: Mapping + ?Sized>(m: &M, q: &mut TQuotePat) {
    q.ty = m.ty(q.ty);
    q.body = q.body.map(|b| m.expr(b));
    syms_in(m, &mut q.holes);
    for t in &mut q.type_params {
        *t = m.tparam(*t);
    }
    for (t, e) in &mut q.types {
        *t = m.tparam(*t);
        *e = m.expr(*e);
    }
    q.quotes = q.quotes.map(|e| m.expr(e));
}

/// Rebuilds a map keyed by ids where a key moved, and renumbers the values in place.
pub(in crate::typer) fn map<M: Mapping + ?Sized, K: Copy + Eq + std::hash::Hash, V>(m: &M, map: &mut FxMap<K, V>, key: impl Fn(&M, K) -> K, value: impl Fn(&M, &mut V)) {
    let moved = map.keys().any(|&k| key(m, k) != k);
    if moved {
        let old = std::mem::take(map);
        map.reserve(old.len());
        for (k, mut v) in old {
            value(m, &mut v);
            map.insert(key(m, k), v);
        }
    } else {
        for v in map.values_mut() {
            value(m, v);
        }
    }
}

/// Whether an environment names an id the merge moves.
pub(in crate::typer) fn env_moves<M: Mapping + ?Sized>(m: &M, env: &Env) -> bool {
    let frame_moves = |f: &Frame| match f {
        Frame::Locals { names, tparams, givens, classes, aliases, .. } => {
            names.iter().any(|&(_, s)| m.sym(s) != s)
                || tparams.iter().any(|&(_, p)| m.tparam(p) != p)
                || givens.iter().any(|&s| m.sym(s) != s)
                || classes.iter().any(|&(_, c)| m.class(c) != c)
                || aliases.iter().any(|&(_, a)| m.alias(a) != a)
        }
        Frame::Class(c) => m.class(*c) != *c,
    };
    env.frames.iter().any(frame_moves)
        || env.imports.iter().any(|i| {
            i.bound.is_some_and(|t| m.ty(t) != t)
                || match i.target {
                    ImportTarget::PkgMember(p, _) | ImportTarget::PkgAll(p) | ImportTarget::PkgGivens(p) => m.pkg(p) != p,
                    ImportTarget::ClassMember(c, _) | ImportTarget::ClassAll(c) | ImportTarget::ClassGivens(c) => m.class(c) != c,
                    _ => false,
                }
        })
}

/// An environment under the new ids.
pub(in crate::typer) fn env<M: Mapping + ?Sized>(m: &M, env: &mut Env) {
    for f in &mut env.frames {
        match f {
            Frame::Locals { names, tparams, givens, classes, aliases, .. } => {
                for (_, s) in names {
                    *s = m.sym(*s);
                }
                for (_, p) in tparams {
                    *p = m.tparam(*p);
                }
                for s in givens {
                    *s = m.sym(*s);
                }
                for (_, c) in classes {
                    *c = m.class(*c);
                }
                for (_, a) in aliases {
                    *a = m.alias(*a);
                }
            }
            Frame::Class(c) => *c = m.class(*c),
        }
    }
    for i in &mut env.imports {
        i.bound = i.bound.map(|t| m.ty(t));
        i.target = match i.target {
            ImportTarget::PkgMember(p, n) => ImportTarget::PkgMember(m.pkg(p), n),
            ImportTarget::PkgAll(p) => ImportTarget::PkgAll(m.pkg(p)),
            ImportTarget::PkgGivens(p) => ImportTarget::PkgGivens(m.pkg(p)),
            ImportTarget::ClassMember(c, n) => ImportTarget::ClassMember(m.class(c), n),
            ImportTarget::ClassAll(c) => ImportTarget::ClassAll(m.class(c)),
            ImportTarget::ClassGivens(c) => ImportTarget::ClassGivens(m.class(c)),
            other => other,
        };
    }
}
