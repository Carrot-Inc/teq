//! The accessors scalac 3.8.4's `PrepareInlineable` gives a class for the members its inline
//! methods' bodies read that the call sites the bodies expand at could not: a private,
//! protected or qualified-private term member of the class,
//! not a constant val and not an inline method, read through `inline$<name>` (expanded by the
//! class's full name in a class a subclass can extend, `inline$fix$acc$Counter$$bump`), and
//! assigned through `inline$<name>_=`. The TASTy writer writes them and the bodies' reads
//! through them; the JVM backend of a products build emits them, so that a scalac downstream
//! expanding a body links.

use super::*;
use crate::ast::mods;
use crate::tir::{DefinitionState, TExpr};

/// One accessor: the member it reads or assigns, its name, whether it is `final` (a private
/// member's; a protected or qualified-private one's is not), and what the body reads the member
/// on, which the accessor's body reads it on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineAccessor {
    pub target: SymId,
    pub setter: bool,
    pub name: String,
    pub is_final: bool,
    pub recv: AccessorRecv,
}

/// What an inline body reads an accessor's member on: `this` (an instance's member, which the
/// accessor reads on its own `this`), the companion object, or the object's static path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessorRecv {
    This,
    Module(ClassId),
    Static,
}

impl<'a> Worker<'a> {
    /// The accessors of the class `c`, in the order its inline methods' stored bodies first
    /// need them (`MakeInlineableDirect` appends each where it makes it).
    pub fn inline_accessors(&self, c: ClassId) -> Vec<InlineAccessor> {
        let info = self.syms.class(c);
        let mut uses: Vec<(u32, u32, SymId, bool, AccessorRecv)> = Vec::new();
        for (order, &m) in info.member_order.iter().enumerate() {
            let s = self.syms.sym(m);
            if s.mods & mods::INLINE == 0 || s.kind == SymKind::Val {
                continue;
            }
            let Some(d) = self.inline_definitions.get(&m) else { continue };
            let Some(body) = d.body.filter(|_| d.state == DefinitionState::Checked) else { continue };
            let on = |r: TExprId| match self.prog.expr(r) {
                TExpr::Module(o) => AccessorRecv::Module(o),
                _ => AccessorRecv::This,
            };
            for e in self.prog.descendants(body) {
                let (target, setter, recv) = match self.prog.expr(e) {
                    TExpr::Field(r, t) | TExpr::CallMethod(r, t, _) => (t, false, on(r)),
                    TExpr::Static(t) | TExpr::CallStatic(t, _) => (t, false, AccessorRecv::Static),
                    TExpr::Assign(lhs, _) => match self.prog.expr(lhs) {
                        TExpr::Field(r, t) => (t, true, on(r)),
                        TExpr::Static(t) => (t, true, AccessorRecv::Static),
                        _ => continue,
                    },
                    _ => continue,
                };
                if !self.needs_inline_accessor(c, target) {
                    continue;
                }
                let at = self.prog.span_of(e).map_or(u32::MAX, |(_, s)| s.start);
                uses.push((order as u32, at, target, setter, recv));
            }
        }
        uses.sort_by_key(|u| (u.0, u.1, u.2, u.3));
        let mut out: Vec<InlineAccessor> = Vec::new();
        for (_, _, target, setter, recv) in uses {
            if out.iter().any(|a| a.target == target && a.setter == setter) {
                continue;
            }
            let t = self.syms.sym(target);
            let name = accessor_text(&self.inline_accessor_parts(c, target, setter));
            let is_final = t.mods & mods::PROTECTED == 0 && !t.scoped_private;
            out.push(InlineAccessor { target, setter, name, is_final, recv });
        }
        out
    }

    /// Whether a read of `target` in an inline body of `c` goes through an accessor
    /// (`PrepareInlineable.needsAccessor`): a term member of `c` that is private (a constructor
    /// parameter that is no field among them), protected or qualified-private, or a protected
    /// one `c` inherits, read on `this`; neither a constant val nor an inline method.
    pub fn needs_inline_accessor(&self, c: ClassId, target: SymId) -> bool {
        let t = self.syms.sym(target);
        if !matches!(t.kind, SymKind::Val | SymKind::Var | SymKind::Def) {
            return false;
        }
        let companion = self.syms.class(c).companion;
        let inherited = match t.owner {
            Owner::Class(o) if o == c => false,
            Owner::Class(o) if t.mods & mods::PROTECTED != 0 && self.syms.class(c).base_types.iter().any(|&(b, _)| b == o) => true,
            // The companion object's private member, which `c` reads and a downstream cannot.
            Owner::Class(o) if Some(o) == companion && self.syms.class(o).kind == ClassKind::Object => {
                return t.mods & mods::INLINE == 0 && self.constant_value(target).is_none() && (t.mods & (mods::PRIVATE | mods::PROTECTED) != 0 || t.scoped_private);
            }
            _ => return false,
        };
        if inherited {
            return t.mods & mods::INLINE == 0 && self.constant_value(target).is_none();
        }
        if t.mods & mods::INLINE != 0 || self.constant_value(target).is_some() {
            return false;
        }
        let param = self.syms.class(c).ctor_syms.iter().flatten().any(|&p| p == target) && t.mods & mods::FIELD == 0;
        t.mods & (mods::PRIVATE | mods::PROTECTED) != 0 || t.scoped_private || param
    }

    /// The name of the accessor through which an inline body of `c` reads `target`, by the
    /// naming `inline_accessors` gives it (`inline$x`, `inline$fix$acc$Counter$$x` in a class a
    /// subclass can extend).
    pub fn inline_accessor_name(&self, c: ClassId, target: SymId) -> String {
        accessor_text(&self.inline_accessor_parts(c, target, false))
    }

    /// The parts of the derived name of `c`'s accessor of `target` (a setter's of `target_=`), as
    /// `PrepareInlineable.MakeInlineableMap.accessorNameOf` makes it: the class's full name, its
    /// segments, where `c` is extensible (`EXPANDPREFIX`es, empty otherwise), and the member's name.
    pub fn inline_accessor_parts(&self, c: ClassId, target: SymId, setter: bool) -> (Vec<String>, String) {
        let base = self.name_str(self.syms.sym(target).name);
        let member = if setter { format!("{}_=", base) } else { base };
        let prefix = if self.extensible(c) { self.expanded_prefix(c) } else { Vec::new() };
        (prefix, member)
    }

    /// Whether `c` is a class another can extend, whose accessors' names scalac expands by the
    /// class's name (`Symbol.isExtensibleClass`).
    fn extensible(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        matches!(info.kind, ClassKind::Class | ClassKind::Trait) && info.mods & mods::FINAL == 0 && !self.name_str(info.name).contains("$anon")
    }

    /// `fix`, `acc`, `Counter` of `fix.acc.Counter`: the class's full name's segments, which
    /// the accessor's name joins with `$`.
    fn expanded_prefix(&self, c: ClassId) -> Vec<String> {
        let mut segs = vec![self.name_str(self.syms.class(c).name)];
        let mut owner = self.syms.class(c).owner;
        loop {
            match owner {
                Owner::Class(o) => {
                    segs.push(self.name_str(self.syms.class(o).name));
                    owner = self.syms.class(o).owner;
                }
                Owner::Package(p) => {
                    let mut at = Some(p);
                    while let Some(q) = at {
                        let pkg = self.syms.pkg(q);
                        if pkg.parent.is_none() {
                            break;
                        }
                        segs.push(self.name_str(pkg.name));
                        at = pkg.parent;
                    }
                    break;
                }
                Owner::Local => break,
            }
        }
        segs.reverse();
        segs
    }
}

/// An accessor's name as its text: `inline$x`, `inline$fix$acc$Counter$$x`.
pub fn accessor_text((prefix, member): &(Vec<String>, String)) -> String {
    match prefix.is_empty() {
        true => format!("inline${}", member),
        false => format!("inline${}$${}", prefix.join("$"), member),
    }
}

impl<'a> Worker<'a> {
    /// Makes each class's accessors, in a build that writes products, as members of the class:
    /// a `Def` of no definition whose body reads, calls or assigns its target on `this` or the
    /// companion object (`inline$x = this.x`, `inline$m(a) = this.m(a)`, `inline$x_=(x$0) =
    /// this.x = x$0`, `inline$lib$C$$hidden = C.hidden`), which
    /// the TASTy writer pickles after the class's members and the JVM backend emits. The
    /// accessor of each target, by class, target and kind, is `inline_accessor_syms`.
    pub fn add_inline_accessors(&mut self, owned: &dyn Fn(FileId) -> bool) {
        let classes: Vec<ClassId> = (0..self.syms.classes.len() as u32).map(ClassId).filter(|&c| owned(self.syms.class(c).file)).collect();
        for c in classes {
            for a in self.inline_accessors(c) {
                if self.inline_accessor_syms.contains_key(&(c, a.target, a.setter)) {
                    continue;
                }
                let (t_file, t_span, t_kind) = {
                    let t = self.syms.sym(a.target);
                    (t.file, t.span, t.kind)
                };
                let tsig = self.sig_of(a.target).clone();
                let name = self.interner.intern(&a.name);
                let m = if a.is_final { mods::FINAL } else { 0 };
                let s = self.syms.new_sym(name, SymKind::Def, m, Owner::Class(c), t_file, None, t_span);
                // The member on what the body reads it on: `this`, the companion, a static path.
                let recv = match a.recv {
                    AccessorRecv::This => Some(self.prog.add(TExpr::This)),
                    AccessorRecv::Module(o) => Some(self.prog.add(TExpr::Module(o))),
                    AccessorRecv::Static => None,
                };
                let (sig, params, body) = if a.setter {
                    let x = self.new_local(self.interner.intern("x$0"), SymKind::Val, tsig.ret, t_span);
                    let param = ParamSig { name: self.interner.intern("x$0"), ty: tsig.ret, by_name: false, repeated: false, has_default: false, sym: x };
                    let sig = MethodSig { tparams: Vec::new(), clauses: vec![ClauseSig { params: vec![param], is_using: false, is_implicit: false }], ret: self.b.t_unit };
                    let field = match recv {
                        Some(r) => self.prog.add(TExpr::Field(r, a.target)),
                        None => self.prog.add(TExpr::Static(a.target)),
                    };
                    let value = self.prog.add(TExpr::Local(x));
                    let assign = self.prog.add(TExpr::Assign(field, value));
                    let unit = self.b.t_unit;
                    self.prog.set_type(assign, unit);
                    (sig, vec![x], assign)
                } else if t_kind == SymKind::Def && !(tsig.clauses.is_empty() && tsig.tparams.is_empty()) {
                    // A method's: its parameters again, the target called with them.
                    let mut clauses = Vec::new();
                    let mut params = Vec::new();
                    let mut args = Vec::new();
                    for cl in &tsig.clauses {
                        let mut ps = Vec::new();
                        for p in &cl.params {
                            let q = self.accessor_param(p, t_file, t_span);
                            params.push(q);
                            let r = self.prog.add(TExpr::Local(q));
                            self.prog.set_type(r, p.ty);
                            args.push(r);
                            ps.push(ParamSig { sym: q, ..*p });
                        }
                        clauses.push(ClauseSig { params: ps, ..*cl });
                    }
                    let sig = MethodSig { tparams: tsig.tparams.clone(), clauses, ret: tsig.ret };
                    let list = self.prog.list(&args);
                    let call = match recv {
                        Some(r) => self.prog.add(TExpr::CallMethod(r, a.target, list)),
                        None => self.prog.add(TExpr::CallStatic(a.target, list)),
                    };
                    self.prog.set_type(call, tsig.ret);
                    (sig, params, call)
                } else {
                    let sig = MethodSig { tparams: Vec::new(), clauses: Vec::new(), ret: tsig.ret };
                    let read = match (t_kind, recv) {
                        (SymKind::Def, Some(r)) => self.prog.add(TExpr::CallMethod(r, a.target, crate::ast::ListRef::EMPTY)),
                        (SymKind::Def, None) => self.prog.add(TExpr::CallStatic(a.target, crate::ast::ListRef::EMPTY)),
                        (_, None) => self.prog.add(TExpr::Static(a.target)),
                        (_, Some(r)) => {
                            // A by-name constructor parameter's field holds its thunk: its value
                            // is the thunk's.
                            let field = self.prog.add(TExpr::Field(r, a.target));
                            match self.syms.sym(a.target).by_name {
                                true => self.prog.add(TExpr::CallClosure(field, crate::ast::ListRef::EMPTY)),
                                false => field,
                            }
                        }
                    };
                    self.prog.set_type(read, tsig.ret);
                    (sig, Vec::new(), read)
                };
                {
                    let mut info = self.syms.sym_mut(s);
                    info.sig = Some(std::sync::Arc::new(sig));
                    info.state().set(crate::symbols::Completion::Done);
                }
                self.syms.add_member(c, s);
                let defaults = vec![None; params.len()];
                let f = self.prog.add_fun(crate::tir::TFun { sym: s, params, defaults, body: Some(body), body_unconsuming: false });
                self.fun_of_sym.insert(s, f);
                // Among the class's methods, which the backends emit.
                if let Some(i) = self.prog_index.class(&self.prog, c) {
                    self.prog.classes[i].methods.push(f);
                }
                self.inline_accessor_syms.insert((c, a.target, a.setter), s);
            }
        }
    }
}

impl<'a> Worker<'a> {
    /// A parameter of an accessor as the target's is: by-name or repeated as it is, so that the
    /// forwarder's descriptor is the target's (`Function0` for `=> Int`, `Seq` for `Int*`).
    fn accessor_param(&mut self, p: &ParamSig, file: FileId, span: Span) -> SymId {
        let local_ty = match (p.repeated, self.seq_class()) {
            (true, Some(seq)) => self.types.class(seq, &[p.ty]),
            _ => p.ty,
        };
        let q = self.syms.new_sym(p.name, SymKind::Param, 0, Owner::Local, file, None, span);
        let mut info = self.syms.sym_mut(q);
        info.sig = Some(std::sync::Arc::new(MethodSig::value(local_ty)));
        info.state().set(crate::symbols::Completion::Done);
        info.by_name = p.by_name;
        q
    }

    /// The result type scalac infers for each inline method of the owned files declared without
    /// one, which teq's typer leaves to each expansion: the stored body's type, its singletons
    /// and constants widened (`Namer.inferredResultType`), `inline_results`.
    pub fn infer_inline_results(&mut self, owned: &dyn Fn(FileId) -> bool) {
        for i in 0..self.syms.syms.len() as u32 {
            let s = SymId(i);
            let info = self.syms.sym(s);
            if info.kind != SymKind::Def || info.mods & mods::INLINE == 0 || !owned(info.file) || self.inline_results.contains_key(&s) {
                continue;
            }
            if self.sig_of(s).ret != ERROR {
                continue;
            }
            // An override's, the member's it overrides; else the stored body's.
            let sig = self.sig_of(s).clone();
            let t = match self.inherited_result_type(s, &std::sync::Arc::new(sig)) {
                Some(t) => t,
                None => {
                    let body = self.inline_definitions.get(&s).filter(|d| d.state == DefinitionState::Checked).and_then(|d| d.body);
                    let Some(t) = body.and_then(|b| self.prog.type_of(b)).filter(|&t| t != ERROR) else { continue };
                    let t = self.widen_path(t);
                    self.widen_lit(t)
                }
            };
            if t != ERROR {
                self.inline_results.insert(s, t);
            }
        }
    }
}
