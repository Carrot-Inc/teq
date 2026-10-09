//! Deferred givens (SIP-64): `given x: T = deferred` in a trait is abstract, and the first class
//! below the trait that does not implement it is given its implementation, a final lazy given
//! whose value a search finds where the class is defined, as dotty's
//! `Typer.implementDeferredGivens` (3376-3456) makes it.
//!
//! The marker is `scala.compiletime.deferred` written under its own name as the right-hand side
//! of a given of a trait (`Namer`, 1929-1937): the given's signature completion decides it
//! (`mods::DEFERRED`); anywhere else the reference is an error (`Intrinsic::Deferred`, as
//! scalac's `@compileTimeOnly`). A deferred given read from a pickle is a bodyless given with
//! `HASDEFAULT`, which a pickle of teq's writes as scalac does.
//!
//! The search runs in the context enclosing the class, with the class's own using parameters
//! added (`parent_args_of`, `deferred_impl_of`): neither the class's members nor the givens it
//! inherits are candidates. A class whose superclass derives from the trait inherits the
//! superclass's implementation; a parameterized deferred given is never implemented this way.
//! The implementation is no member of the class's table, so that a lookup finds the trait's
//! declaration whatever was typed first; the class's `TClass` holds it (`TClass::deferred_givens`)
//! for the backends and the pickle, and the override checks find it from the classes' structure
//! (`deferred_implemented_above`).

use super::*;
use crate::ast::{self, mods, DefKind};
use crate::tir::{TClass, TExpr, TInit};
use std::sync::Arc;

impl<'a> Worker<'a> {
    /// Whether `s` is a deferred given (`mods::DEFERRED`): one read from a pickle says so, a
    /// program's once its signature is complete, which decides it.
    pub(super) fn is_deferred_given(&mut self, s: SymId) -> bool {
        if self.syms.sym(s).mods & mods::DEFERRED != 0 {
            return true;
        }
        if !self.deferred_candidate(s) {
            return false;
        }
        self.sig_arc(s);
        self.syms.sym(s).mods & mods::DEFERRED != 0
    }

    /// A given of a trait whose right-hand side is a reference written `deferred`: a deferred
    /// given if the reference is `scala.compiletime.deferred` (`is_deferred_marker`).
    pub(super) fn deferred_candidate(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        if info.kind != SymKind::Given {
            return false;
        }
        let (Some(d), Owner::Class(c)) = (info.def, info.owner) else { return false };
        if self.syms.class(c).kind != ClassKind::Trait {
            return false;
        }
        let ast = self.ast(info.file);
        let DefKind::Given(g) = &ast.def(d).kind else { return false };
        g.alias.is_some_and(|a| matches!(ast.expr(a), ast::Expr::Ident(n) | ast::Expr::Select(_, n) if self.interner.get(n) == "deferred"))
    }

    /// Whether the path `e` names `scala.compiletime.deferred`, resolved where the definition
    /// holding it stands: a user's own `deferred` does not mark, and an import renaming the
    /// marker leaves it the error it is where it is typed.
    pub(super) fn is_deferred_marker(&mut self, e: ast::ExprId) -> bool {
        self.static_ref(e).and_then(|r| r.sym()).is_some_and(|s| self.intrinsic_of(s) == Some(super::inline::Intrinsic::Deferred))
    }

    /// Implements the deferred givens `c` is the first class to inherit, into its body: each a
    /// final lazy given whose initialiser is the search, or scalac's error at the class.
    pub(super) fn implement_deferred_givens(&mut self, c: ClassId, tclass: &mut TClass) {
        // Most classes inherit no given from a trait: nothing to look at. A library trait not
        // yet complete may not have entered its givens.
        let info = self.syms.class(c);
        if !info.base_types.iter().skip(1).any(|&(b, _)| {
            let base = self.syms.class(b);
            base.kind == ClassKind::Trait && (!base.givens.is_empty() || self.syms.class_done(b).is_none())
        }) {
            return;
        }
        // What the rule reports stands at the class, once: scalac's reporter keeps one message per
        // position (`UniqueMessagePositions`), so a class missing several givens shows the first.
        let first = self.diags.items.len();
        let todo = self.deferred_givens_to_implement(c, first);
        if todo.is_empty() {
            self.check_mixins_over_implementations(c, first);
            return;
        }
        let (file, span) = {
            let i = self.syms.class(c);
            (i.file, i.span)
        };
        // The class's frame stays, for the paths to its enclosing instances, but neither its
        // members nor its imports are in scope: the context outside it.
        let env = self.env_at(file, Owner::Class(c), span.start);
        self.with_env(env, |t| {
            let outer_args = t.parent_args_of.replace(c);
            let outer_impl = t.deferred_impl_of.replace(c);
            for (m, owner_ty) in todo {
                t.outside_search(|t| t.implement_deferred_given(c, m, owner_ty, tclass, first));
            }
            t.deferred_impl_of = outer_impl;
            t.parent_args_of = outer_args;
        });
        self.check_mixins_over_implementations(c, first);
    }

    /// Reports `msg` at the class `c` unless the rule reported there since `first`.
    fn report_at_class(&mut self, c: ClassId, first: usize, msg: String) {
        let (file, span) = {
            let i = self.syms.class(c);
            (i.file, i.span)
        };
        if !self.diags.items[first..].iter().any(|d| !d.is_warning && d.file == file && d.span == span) {
            self.diags.error(file, span, msg);
        }
    }

    /// The deferred givens of the traits of `c` that neither a member of its linearization
    /// implements nor its superclass inherits (which implements them itself), each with the
    /// type of its trait as `c` sees it, by name: a trait further down redeclaring one is met
    /// first.
    fn deferred_givens_to_implement(&mut self, c: ClassId, first: usize) -> Vec<(SymId, TypeId)> {
        let superclass = self.extended_class(c);
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.iter().skip(1).copied().collect();
        let mut out: Vec<(SymId, TypeId)> = Vec::new();
        for (b, bt) in bases {
            if self.syms.class(b).kind != ClassKind::Trait || superclass.is_some_and(|s| self.derives_from(s, b)) {
                continue;
            }
            self.complete_class(b);
            for m in self.syms.class(b).givens.clone() {
                if self.syms.sym(m).owner != Owner::Class(b) || !self.is_deferred_given(m) {
                    continue;
                }
                let name = self.syms.sym(m).name;
                if self.deferred_implemented(c, m) || self.redeclared_below(c, m) {
                    continue;
                }
                // One implementation serves every deferred given of its name, of the type of the
                // first met, the most derived: it has to conform to each other's, as scalac's
                // `RefChecks` checks the member it synthesized.
                if let Some(&(o, ot)) = out.iter().find(|&&(o, _)| self.syms.sym(o).name == name) {
                    self.check_shared_implementation(c, (o, ot), (m, bt), first);
                    continue;
                }
                if self.overloaded_beside(c, m, bt, first) {
                    continue;
                }
                out.push((m, bt));
            }
        }
        out
    }

    /// The type of the deferred given `m` of the trait whose type in `c` is `owner_ty`, as `c`
    /// sees it: its implementation's.
    fn deferred_target(&mut self, c: ClassId, m: SymId, owner_ty: TypeId) -> TypeId {
        let Owner::Class(b) = self.syms.sym(m).owner else { return ERROR };
        let ret = self.sig_arc(m).ret;
        let subst = self.owner_subst(owner_ty);
        let target = self.types.subst(ret, &subst);
        let this_ty = self.syms.this_type(c);
        self.as_seen_from(target, this_ty, b)
    }

    /// Reports at `c` a deferred given `m` that the implementation of `chosen`, another of its
    /// name, cannot implement: scalac's E164 "has incompatible type" (`trait B { given x: Int =
    /// deferred }` beside `trait D { given x: String = deferred }`).
    fn check_shared_implementation(&mut self, c: ClassId, chosen: (SymId, TypeId), m: (SymId, TypeId), first: usize) {
        let (implemented, overridden) = (self.deferred_target(c, chosen.0, chosen.1), self.deferred_target(c, m.0, m.1));
        if self.is_sub(implemented, overridden) {
            return;
        }
        let Owner::Class(b) = self.syms.sym(m.0).owner else { return };
        let msg = format!(
            "error overriding {} of type {}: {} of type {} has incompatible type",
            self.member_description(m.0, Some(b)),
            self.show(overridden),
            self.member_description(chosen.0, None),
            self.show(implemented)
        );
        self.report_at_class(c, first, msg);
    }

    /// Whether a member of `c`'s linearization other than `m` implements the deferred given `m`:
    /// a concrete one of its name and parameters, which wins over the declaration wherever it
    /// stands; one of other parameters is an overload (`given myc(using String)` beside a
    /// deferred `given myc`).
    fn deferred_implemented(&mut self, c: ClassId, m: SymId) -> bool {
        let name = self.syms.sym(m).name;
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
        let Some(m_owner) = bases.iter().find(|&&(b, _)| Owner::Class(b) == self.syms.sym(m).owner).map(|&(_, t)| t) else { return false };
        bases.iter().enumerate().any(|(i, &(b, bt))| {
            let mut k = 0;
            while let Some(s) = self.inherited_member(b, name, false, k) {
                k += 1;
                if s == m || self.is_private(s) || self.is_deferred_given(s) || self.is_abstract_member(s) {
                    continue;
                }
                let ssig = self.sig_arc(s);
                if self.compare_sigs(c, ssig, (i > 0).then_some(bt), m, m_owner) != super::check::Agreement::Params {
                    return true;
                }
            }
            false
        })
    }

    /// Whether a class or trait of `c`'s linearization below `m`'s trait declares its name anew
    /// other than as a deferred given (`override given x: T`, abstract): `c`'s member is that one,
    /// which nothing synthesizes, and which `c` is reported for leaving unimplemented.
    fn redeclared_below(&mut self, c: ClassId, m: SymId) -> bool {
        let Owner::Class(t) = self.syms.sym(m).owner else { return false };
        let name = self.syms.sym(m).name;
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
        let Some(m_owner) = bases.iter().find(|&&(b, _)| b == t).map(|&(_, ty)| ty) else { return false };
        bases.iter().enumerate().any(|(i, &(b, bt))| {
            if b == t || !self.derives_from(b, t) {
                return false;
            }
            // One of other parameters overloads the name rather than declares it anew.
            let mut k = 0;
            while let Some(s) = self.inherited_member(b, name, false, k) {
                k += 1;
                if self.syms.sym(s).owner != Owner::Class(b) || self.is_private(s) || self.is_deferred_given(s) {
                    continue;
                }
                let ssig = self.sig_arc(s);
                if self.compare_sigs(c, ssig, (i > 0).then_some(bt), m, m_owner) != super::check::Agreement::Params {
                    return true;
                }
            }
            false
        })
    }

    /// Whether `m`'s class derives from the class of `d`, a deferred given, so that `m` overrides
    /// it rather than being implemented by it.
    pub(super) fn overrides_declaration_of(&mut self, m: SymId, d: SymId) -> bool {
        match (self.syms.sym(m).owner, self.syms.sym(d).owner) {
            (Owner::Class(mc), Owner::Class(dc)) => mc != dc && self.derives_from(mc, dc),
            _ => false,
        }
    }

    /// Whether a method of `c`'s linearization overloads the deferred given `m`, which would
    /// stand beside its implementation, not in the class's table: teq's "a val beside a method of
    /// its name is an alternative in a class only", reported at the class.
    fn overloaded_beside(&mut self, c: ClassId, m: SymId, m_owner: TypeId, first: usize) -> bool {
        let name = self.syms.sym(m).name;
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
        for (i, &(b, bt)) in bases.iter().enumerate() {
            let mut k = 0;
            while let Some(s) = self.inherited_member(b, name, false, k) {
                k += 1;
                if s == m || self.syms.sym(s).kind != SymKind::Def || self.is_private(s) {
                    continue;
                }
                let ssig = self.sig_arc(s);
                if self.compare_sigs(c, ssig, (i > 0).then_some(bt), m, m_owner) == super::check::Agreement::Params {
                    let msg = format!(
                        "{} of type {} does not match {} of type {}; only methods can be overloaded",
                        self.member_description(m, None),
                        self.sig_string(m),
                        self.member_description(s, Some(b)),
                        self.sig_string(s)
                    );
                    self.report_at_class(c, first, msg);
                    return true;
                }
            }
        }
        false
    }

    fn implement_deferred_given(&mut self, c: ClassId, m: SymId, owner_ty: TypeId, tclass: &mut TClass, first: usize) {
        let Owner::Class(b) = self.syms.sym(m).owner else { return };
        let sig = self.sig_arc(m);
        // The declaration's access, its boundary (`protected[p]`, `private[p]`, which the pickle
        // reads off the declaration) and its name's origin, as dotty's `dcl.copy` keeps them.
        let (name, kept, scoped_private) = {
            let info = self.syms.sym(m);
            // A declaration read from a pickle holds its boundary in the loader's table.
            let loaded = (info.mods & mods::QUALIFIED != 0).then(|| self.loaded.as_ref()?.access_within.get(&m).map(|&(_, protected)| protected)).flatten();
            let (protected, scoped) = match loaded {
                Some(protected) => (protected, !protected),
                None => (info.mods & mods::PROTECTED != 0, info.scoped_private),
            };
            let access = if protected { mods::PROTECTED } else if scoped { mods::PRIVATE } else { 0 };
            (info.name, info.mods & mods::ANONYMOUS | access, scoped)
        };
        let (file, span) = {
            let i = self.syms.class(c);
            (i.file, i.span)
        };
        let what = format!("the deferred given instance {} in {}", self.name_str(name), self.class_description(b));
        if !sig.tparams.is_empty() || !sig.clauses.is_empty() {
            let msg = format!("Cannnot infer the implementation of {}\nsince that given is parameterized. An implementing given needs to be written explicitly.", what);
            self.report_at_class(c, first, msg);
            return;
        }
        let target = self.deferred_target(c, m, owner_ty);
        let e = match self.resolve_given_telling(target, span) {
            Ok((e, _)) => e,
            Err(_) => {
                let msg = match self.given_ambiguity.take() {
                    Some(ambiguous) => format!("{} of inferring the implementation of {}", ambiguous, what),
                    None => format!("No given instance of type {} was found for inferring the implementation of {}", self.show(target), what),
                };
                let msg = msg + &self.given_failure_notes();
                if !self.diags.items[first..].iter().any(|d| !d.is_warning && d.file == file && d.span == span) {
                    self.given_failure_error(span, msg, target);
                }
                return;
            }
        };
        // The search finding the declaration itself through the class (`object Foo extends
        // T[Foo]` reaching its own `sh` through the companion): no implementation.
        if self.reads_own_member(e, c, name) {
            let msg = format!("Inferred implementation of {} is self-recursive.\nAn implementing given needs to be written explicitly.", what);
            self.report_at_class(c, first, msg);
            return;
        }
        let s = self.syms.new_sym(name, SymKind::Given, mods::LAZY | mods::GIVEN | mods::FINAL | mods::OVERRIDE | kept, Owner::Class(c), file, None, span);
        {
            let mut info = self.syms.sym_mut(s);
            info.sig = Some(Arc::new(MethodSig::value(target)));
            info.scoped_private = scoped_private;
            info.state().set(Completion::Done);
        }
        self.name_like(s, m);
        // An implementation of a parameterless def as well (an old-style abstract given the
        // deferred one overrides) is reached through a method, as `mark_accessors` gives a val.
        if self.implements_parameterless_def(c, name) {
            self.syms.sym_mut(s).needs_accessor = true;
        }
        tclass.init.push(TInit::Field(s, e));
        tclass.deferred_givens.push((m, s));
    }

    /// Whether an ancestor of `c` declares a def `name` without parameters but using clauses.
    fn implements_parameterless_def(&mut self, c: ClassId, name: Name) -> bool {
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        bases.into_iter().any(|b| {
            let Some(p) = self.syms.class(b).members.get(&name).copied() else { return false };
            self.syms.sym(p).kind == SymKind::Def && self.sig_of(p).clauses.iter().all(|cl| cl.is_using)
        })
    }

    /// Whether `e` reads the member `name` of the class `c` itself, its instance or its module.
    fn reads_own_member(&self, e: TExprId, c: ClassId, name: Name) -> bool {
        let (recv, s) = match self.prog.expr(e) {
            TExpr::Field(r, s) => (r, s),
            TExpr::CallMethod(r, s, args) if args.len == 0 => (r, s),
            _ => return false,
        };
        let own = match self.prog.expr(recv) {
            TExpr::This => true,
            TExpr::Module(k) => k == c,
            _ => false,
        };
        own && self.syms.sym(s).name == name
    }

    /// A trait `c` mixes in anew whose member overrides the implementation a superclass was given
    /// for a deferred given, which is final: scalac's E164, reported at `c` since the pair is
    /// `c`'s alone (`class C extends B with T`, `T` overriding what `B` implements).
    fn check_mixins_over_implementations(&mut self, c: ClassId, first: usize) {
        let Some(sup) = self.extended_class(c) else { return };
        let above: Vec<ClassId> = self.syms.class(sup).base_types.iter().map(|&(b, _)| b).filter(|&b| self.syms.class(b).kind == ClassKind::Trait).collect();
        let mixed: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).filter(|&b| self.syms.class(b).kind == ClassKind::Trait && !above.contains(&b)).collect();
        if mixed.is_empty() {
            return;
        }
        for t in above {
            for p in self.syms.class(t).givens.clone() {
                if self.syms.sym(p).owner != Owner::Class(t) || !self.is_deferred_given(p) {
                    continue;
                }
                let name = self.syms.sym(p).name;
                let overriding = mixed.iter().find_map(|&b| {
                    let s = self.inherited_member(b, name, false, 0)?;
                    (self.syms.sym(s).owner == Owner::Class(b) && !self.is_private(s) && !self.is_abstract_member(s)).then_some((s, b))
                });
                let Some((s, b)) = overriding else { continue };
                if let Some(k) = self.deferred_implemented_above(c, p) {
                    let msg = format!("{} cannot override final member {}", self.member_description(s, Some(b)), self.member_description(p, Some(k)));
                    self.report_at_class(c, first, msg);
                }
            }
        }
    }

    /// The class above `c` whose implementation of the deferred given `p` the search made, which
    /// is final: the topmost class deriving from `p`'s trait, where nothing implements it.
    pub(super) fn deferred_implemented_above(&mut self, c: ClassId, p: SymId) -> Option<ClassId> {
        let Owner::Class(t) = self.syms.sym(p).owner else { return None };
        let k = self.extended_class(c).filter(|&k| self.derives_from(k, t))?;
        self.implementing_class(k, p)
    }

    /// The class whose implementation of the deferred given `p` an instance of the class `k`
    /// holds, the search's: `k` or a class above it, the topmost deriving from `p`'s trait, where
    /// nothing implements it or declares it anew. An access error names it, as scalac and a build
    /// over the products, whose pickle holds it as that class's member, do.
    pub(super) fn implementing_class(&mut self, k: ClassId, p: SymId) -> Option<ClassId> {
        let Owner::Class(t) = self.syms.sym(p).owner else { return None };
        if self.syms.class(k).kind == ClassKind::Trait || !self.derives_from(k, t) || !self.is_deferred_given(p) {
            return None;
        }
        let mut k = k;
        while let Some(up) = self.extended_class(k).filter(|&up| self.derives_from(up, t)) {
            k = up;
        }
        (!self.deferred_implemented(k, p) && !self.redeclared_below(k, p)).then_some(k)
    }

    /// The class `c` extends: its superclass, or the enum of an enum's case, which teq's records
    /// keep apart (`ClassInfo::superclass` names classes alone).
    fn extended_class(&self, c: ClassId) -> Option<ClassId> {
        let info = self.syms.class(c);
        match (info.superclass, info.kind, info.owner) {
            (Some(s), ..) => Some(s),
            (None, ClassKind::EnumCase, Owner::Class(companion)) => self.syms.class(companion).companion.filter(|&e| self.syms.class(e).kind == ClassKind::Enum),
            _ => None,
        }
    }
}
