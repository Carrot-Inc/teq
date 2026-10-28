//! Default getters, dotty's members of their own (`Desugar.addDefaultGetters`): each default of a
//! program member's parameter is a synthetic symbol `m$default$n`, completed by the reads that
//! need it (a call that leaves the parameter out, an overload's test of it) or where the
//! traversal reaches it (after its method, at its class's companion for a constructor's), never
//! by its method's body. Its completion types the default against the parameter's type with the
//! method's type parameters approximated (`Namer.expectedDefaultArgType`) and gives it a type of
//! its own (`Namer.inferredResultType`'s `rhsType`), which an omitted argument has.

use std::sync::Arc;

use super::site::SiteOwner;
use super::{Frame, Worker};
use crate::ast::{self, DefKind};
use crate::intern::FxMap;
use crate::symbols::*;
use crate::tir::{DefaultOf, TExpr, TExprId};
use crate::types::*;

/// The program's default getters, entered by the namer before the bodies (`enter_getters`) and
/// read only after it.
#[derive(Default)]
pub struct GetterTable {
    /// By whose default it is and the parameter's place across the clauses, its getter.
    pub of: FxMap<(DefaultOf, u32), SymId>,
    /// By getter, whose default it is.
    pub link: FxMap<SymId, (DefaultOf, u32)>,
    /// By whose defaults they are, the parameters' count up to the last that has one.
    pub count: FxMap<DefaultOf, u32>,
}

/// The type of the getter of the default of parameter `index` of `of`, as its completion gave it
/// (`Namer.inferredResultType`'s `rhsType`: the parameter's type where the approximation changed
/// nothing and the default conforms to it, else the default's type widened), which the products'
/// writers give the getter as scalac's pickle and class file have it (`a$default$1[A]: String` of
/// `x: A = "hi"`); none where no getter exists or it is not complete.
pub(crate) fn getter_type(table: &GetterTable, syms: &Symbols, of: DefaultOf, index: usize) -> Option<TypeId> {
    let g = *table.of.get(&(of, index as u32))?;
    let info = syms.sym(g);
    let ret = info.sig.as_ref()?.ret;
    (ret != ERROR).then_some(ret)
}

impl<'a> Worker<'a> {
    /// The getters of the program's members' defaults (`Desugar.addDefaultGetters`): a method's,
    /// a secondary constructor's and a class's primary constructor's. A block's method's defaults
    /// are typed with the block, which owns what they report and traverses them with the method.
    pub(super) fn enter_getters(&mut self) {
        let mut table = GetterTable::default();
        let syms: Vec<SymId> = (0..self.syms.syms.len() as u32).map(SymId).collect();
        for m in syms {
            let (file, def, owner, kind) = {
                let s = self.syms.sym(m);
                (s.file, s.def, s.owner, s.kind)
            };
            let Some(d) = def else { continue };
            if kind != SymKind::Def || owner == Owner::Local || !self.program_file(file) || self.std.slot_of(file).is_some() {
                continue;
            }
            let defaults: Vec<Option<ast::ExprId>> = match &self.ast(file).def(d).kind {
                DefKind::Fun(f) => f.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.default)).collect(),
                _ => continue,
            };
            let name = self.name_str(self.syms.sym(m).name).to_string();
            self.enter_getters_of(&mut table, DefaultOf::Method(m), file, &name, &defaults);
        }
        for i in 0..self.syms.classes.len() as u32 {
            let c = ClassId(i);
            let (file, def) = {
                let k = self.syms.class(c);
                (k.file, k.def)
            };
            let Some(d) = def else { continue };
            if !self.program_file(file) || self.std.slot_of(file).is_some() || self.syms.class(c).owner == Owner::Local {
                continue;
            }
            let defaults: Vec<Option<ast::ExprId>> = match &self.ast(file).def(d).kind {
                DefKind::Class(cls) => cls.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.default)).collect(),
                _ => continue,
            };
            self.enter_getters_of(&mut table, DefaultOf::Ctor(c), file, "$lessinit$greater", &defaults);
        }
        self.getters = Arc::new(table);
    }

    fn enter_getters_of(&mut self, table: &mut GetterTable, of: DefaultOf, file: crate::source::FileId, name: &str, defaults: &[Option<ast::ExprId>]) {
        for (i, d) in defaults.iter().enumerate() {
            let Some(e) = *d else { continue };
            let span = self.ast(file).expr_span(e);
            let n = self.interner.intern(&format!("{}$default${}", name, i + 1));
            let g = self.syms.new_sym(n, SymKind::Def, 0, Owner::Local, file, None, span);
            self.syms.sym_mut(g).synthetic = true;
            table.of.insert((of, i as u32), g);
            table.link.insert(g, (of, i as u32));
            table.count.insert(of, i as u32 + 1);
        }
    }

    /// The getter of the default of parameter `index` of `of`, where it is a program member's.
    #[inline]
    pub(super) fn getter_of(&self, of: DefaultOf, index: usize) -> Option<SymId> {
        if self.getters.of.is_empty() {
            return None;
        }
        self.getters.of.get(&(of, index as u32)).copied()
    }

    /// The default expression a getter types, with the parameters it sees: its method's (or
    /// class's) file, definition, type parameters, the parameter's signature and the parameters
    /// of the clauses before its own (`getterParamss`: `takeUpTo` keeps whole clauses).
    fn getter_site(&mut self, of: DefaultOf, index: usize) -> Option<GetterSite> {
        match of {
            DefaultOf::Method(m) => {
                let sig = self.sig_arc(m);
                let (file, def, owner) = {
                    let s = self.syms.sym(m);
                    (s.file, s.def?, s.owner)
                };
                let DefKind::Fun(f) = &self.ast(file).def(def).kind else { return None };
                let expr = f.clauses.iter().flat_map(|c| c.params.iter()).nth(index)?.default?;
                let start = super::elaborate::clause_start(&sig, index);
                let earlier: Vec<(Vec<ParamSig>, bool)> = {
                    let mut out = Vec::new();
                    let mut k = 0;
                    for c in &sig.clauses {
                        if k >= start {
                            break;
                        }
                        out.push((c.params.clone(), c.is_using));
                        k += c.params.len();
                    }
                    out
                };
                let param = sig.clauses.iter().flat_map(|c| c.params.iter()).nth(index)?.clone();
                Some(GetterSite { file, def, owner, member: Some(m), class: None, tparams: sig.tparams.clone(), earlier, param, expr })
            }
            DefaultOf::Ctor(c) => {
                self.complete_class(c);
                let (file, def, owner) = {
                    let k = self.syms.class(c);
                    (k.file, k.def?, k.owner)
                };
                let DefKind::Class(cls) = &self.ast(file).def(def).kind else { return None };
                let expr = cls.clauses.iter().flat_map(|cl| cl.params.iter()).nth(index)?.default?;
                let clauses = self.syms.class(c).ctor.clone();
                let mut earlier = Vec::new();
                let mut k = 0;
                let mut param = None;
                for cl in &clauses {
                    if index < k + cl.params.len() {
                        param = cl.params.get(index - k).cloned();
                        break;
                    }
                    earlier.push((cl.params.clone(), cl.is_using));
                    k += cl.params.len();
                }
                let tparams = self.syms.class(c).tparams.to_vec();
                Some(GetterSite { file, def, owner, member: None, class: Some(c), tparams, earlier, param: param?, expr })
            }
        }
    }

    /// The completion of the getter `g` (`Namer.Completer.complete` of `m$default$n`): its
    /// default typed where its method is defined, against `expectedDefaultArgType`, under the
    /// getter's own owner (`DiagOwner::sig` at the default), its tree kept as the getter's
    /// initialiser for its method's record (`fill_getters`) and its type as the getter's
    /// signature (`rhsType`).
    pub(super) fn complete_getter(&mut self, g: SymId, of: DefaultOf, index: u32) -> (MethodSig, bool) {
        let Some(site) = self.getter_site(of, index as usize) else { return (MethodSig::value(ERROR), false) };
        let owner = crate::source::DiagOwner::sig(site.file, self.syms.sym(g).span.start);
        let outer_owner = self.enter_owner(owner);
        let marks = (self.inlining.demands.len(), self.diags.items.len());
        // A constructor's getters are the companion's members (`Desugar.decompose`'s
        // `companionDefs`, before the companion's own statements): completed in the companion's
        // definition context, with the class's type parameters and the earlier parameter clauses
        // alone (`type_getter`), not in the class's, whose members a default cannot name; a class
        // without a written companion has them in its enclosing scope, where its companion would
        // stand.
        let env = match site.class {
            Some(c) => match self.getter_companion(c) {
                // The companion's scope: a written one's at its definition, a made one's at the
                // class's, where `companionDefs` puts it; its frame inside the enclosing class
                // makes `O.this` an outer instance there (`this_ref`).
                Some(k) => {
                    let at = match self.syms.class(k).def {
                        Some(kd) => self.ast(site.file).def(kd).span.start,
                        None => self.ast(site.file).def(site.def).span.start,
                    };
                    self.env_at(site.file, Owner::Class(k), at)
                }
                None => self.env_at(site.file, self.syms.class(c).owner, self.ast(site.file).def(site.def).span.start),
            },
            None => self.env_at(site.file, site.owner, self.ast(site.file).def(site.def).span.start),
        };
        // A constructor's getter of a class nested in a class reads the enclosing instance
        // through the constructor's outer parameter (dotty's `ExplicitOuter`; the companion's
        // outer, where the getter is), whatever typing demands it: bound as `type_secondary_ctor`
        // binds it, so the getter's tree is the same from a call's demand and from the walk.
        let ctor_outer = match of {
            DefaultOf::Method(m) if self.syms.sym(m).name == crate::names::INIT => match self.syms.sym(m).owner {
                Owner::Class(c) => self.outer_class(c).map(|o| (o, self.ctor_outer_param(m, o))),
                _ => None,
            },
            // The primary constructor's getters read the enclosing instance through the local
            // that stands for `O.this` in the classes nested in `O` (`outer_this_sym`), the
            // constructor's captured outer, as the class's own body does.
            DefaultOf::Ctor(c) => match self.outer_class(c) {
                Some(o) => Some((o, self.outer_this_sym(o))),
                None => None,
            },
            _ => None,
        };
        let outer_ctor = match ctor_outer {
            Some(bound) => Some(std::mem::replace(&mut self.ctor_outer, Some(bound))),
            None => None,
        };
        let (typed, ret) = self.outside_inline(|t| t.with_env(env, |t| t.type_getter(&site)));
        if let Some(outer) = outer_ctor {
            self.ctor_outer = outer;
        }
        let noted = self.inlining.demands.len() > marks.0 || self.diags.items.len() > marks.1;
        self.leave_owner(outer_owner, false);
        self.val_init.insert(g, typed);
        if self.forked && self.shared_sym(g) && self.syms.body_cells.claim(g.0) {
            self.publish_body(g);
        }
        (MethodSig::value(ret), noted)
    }

    fn type_getter(&mut self, site: &GetterSite) -> (TExprId, TypeId) {
        let owners_depth = self.sites.owners.len();
        match (site.member, site.class) {
            (Some(m), _) => self.sites.owners.push(SiteOwner::Sym(m)),
            // The getter's owner is the written companion module class (`decompose`), the class
            // where none is written (a made companion's getters are emitted as the class's).
            (None, Some(c)) => {
                let owner = self.getter_companion(c).filter(|&k| self.syms.class(k).def.is_some()).unwrap_or(c);
                self.sites.owners.push(SiteOwner::Dummy(owner));
            }
            _ => {}
        }
        // The getter's parameters: the method's type parameters, without their context bounds,
        // and the value parameters of the clauses before the default's own (`getterParamss`).
        let tparams = site.tparams.iter().map(|&p| (self.syms.tparam(p).name, p)).collect();
        let mut names = Vec::new();
        let mut givens = Vec::new();
        for (params, using) in &site.earlier {
            names.extend(params.iter().map(|p| (p.name, p.sym)));
            if *using {
                givens.extend(params.iter().map(|p| p.sym));
            }
        }
        let terms: Vec<SymId> = site.earlier.iter().flat_map(|(ps, _)| ps.iter().map(|p| p.sym)).collect();
        self.env.frames.push(Frame::Locals { names, tparams, givens, classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 });
        let outer_ext_scope = self.ext_scope;
        if let Some(m) = site.member {
            let sig = self.sig_arc(m);
            let def = self.ast(site.file).def(site.def);
            if let Some(scope) = self.ext_scope_of(m, &sig, def) {
                self.ext_scope = Some(scope);
            }
        }
        let default_tp = site.param.ty;
        let outer_proto = std::mem::take(&mut self.proto_vars);
        let var_mark = self.tvars.len();
        let pt = self.expected_default_arg_type(default_tp, &site.tparams, &terms);
        // The default typed and adapted to the prototype, its type the adapted tree's
        // (`typedAheadRhs(pt).tpe`): a conversion's result where one applied.
        let (typed, own) = self.type_expr(site.expr, Some(pt));
        let span = self.cur_ast().expr_span(site.expr);
        let (te, tp) = self.adapt_typed(typed, own, pt, span);
        // The right-hand side typed, its variables interpolated at once (`interpolateTypeVars`
        // over the typer state's owned variables): the default's own, which its trees left open,
        // and the prototype's wildcards, variables the state owns too (`legalBound`'s
        // `newTypeVar`), each by its variance in the default's type, one that does not occur
        // from below where it has a lower bound, from above otherwise.
        let protos: Vec<TVarId> = self.proto_vars.clone();
        for v in protos {
            if self.tvars[v].inst.is_none() && !self.open_vars.contains(&v) {
                self.open_vars.push(v);
            }
        }
        self.interpolate_open(var_mark, tp, Some(pt));
        let ret = self.default_getter_result(pt, default_tp, tp);
        self.proto_vars = outer_proto;
        self.ext_scope = outer_ext_scope;
        self.env.frames.pop();
        self.sites.owners.truncate(owners_depth);
        let stored = if site.param.by_name { self.prog.add(TExpr::Lambda(ast::ListRef::EMPTY, te)) } else { te };
        (stored, ret)
    }

    /// The companion object of the class `c`, which its constructor's getters are members of
    /// (`Desugar.decompose`'s `companionDefs`): the written one, or the one made for the getters
    /// (`Desugar.classDef`, an object at the class's place in the enclosing scope).
    fn getter_companion(&self, c: ClassId) -> Option<ClassId> {
        let k = self.syms.class(c).companion?;
        let info = self.syms.class(k);
        // An object nested in a class is a class with its module val (`inner_object`).
        (info.kind == ClassKind::Object || info.inner_object.is_some()).then_some(k)
    }

    /// `Namer.expectedDefaultArgType`: the parameter's type with what refers to the method's own
    /// parameters approximated (`wildApprox`), so that the default may have a type more specific
    /// than the parameter's. A type parameter's wildcard of its bounds is an open variable here,
    /// which no reading of the default fixes for the getter's type (teq's prototypes have no
    /// `WildcardType`); an earlier parameter's path is its type. A context function type keeps the
    /// original where it is refined or its result holds such a wildcard (`pos/i12019`).
    fn expected_default_arg_type(&mut self, t: TypeId, tparams: &[TParamId], terms: &[SymId]) -> TypeId {
        // `wildApprox`: each occurrence of a type parameter of the method or class is a wildcard of
        // its bounds (`TypeParamRef` → a new `WildcardType(bounds)`, one per occurrence). teq has no
        // wildcard type apart from a written wildcard argument, which dotty's `TypeBounds` argument
        // is and which its comparer keeps; a prototype's wildcard it compares by its bounds
        // (`TypeComparer`'s `WildcardType` cases) and, where it lands in a type variable's bound,
        // replaces by a fresh type variable of those bounds (`ConstraintHandling.legalBound`,
        // `LevelAvoidMap.mapWild`). So each occurrence is a fresh variable of the parameter's bounds
        // from the start: unconstrained, never unified with another occurrence's, instantiated as
        // dotty's `IsFullyDefinedAccumulator` instantiates the variables `legalBound` made (from
        // above where it stands invariantly or contravariantly in the default's type). The
        // variables are the prototype's (`proto_vars`), which `wrap_context_function` reads.
        let mut approx = self.proto_wild_params(t, tparams, &[]);
        // An earlier parameter's path is a wildcard below its type (`TermParamRef` →
        // `WildcardType(TypeBounds.upper(..))`): its type, the bound a comparison reads.
        let named: Vec<SymId> = terms.iter().copied().filter(|&s| self.mentions_term(approx, s)).collect();
        let paths: Vec<(SymId, TypeId)> = named
            .into_iter()
            .map(|s| {
                let ty = self.sig_of(s).ret;
                (s, ty)
            })
            .collect();
        if !paths.is_empty() {
            approx = self.subst_paths(approx, &paths);
        }
        if approx == t {
            return t;
        }
        // A context function type keeps the original where it is refined or its result holds a
        // wildcard (`expectedDefaultArgType`'s `ContextFunctionType` case).
        if let Type::Class(c, args) = self.types.get(approx) {
            if self.is_context_function_class(c) {
                let result = self.types.items(args).last().copied().unwrap_or(ANY);
                if self.mentions_proto_var(result) {
                    return t;
                }
            }
        }
        // A wildcard standing for the whole type is compared by its upper bound (`isSubType` of a
        // `WildcardType` on the right, TypeComparer's `compareWild`): the parameter's upper bound
        // is what the default has to conform to.
        if let Type::Var(v) = self.types.get(approx) {
            if self.proto_vars.contains(&v) {
                return self.tvars[v].declared().1;
            }
        }
        approx
    }

    /// `t` with each occurrence of one of `tparams` a fresh type variable of that parameter's
    /// bounds, recorded in `proto_vars` (`wildApprox`'s `WildcardType` per occurrence). The
    /// bounds are approximated themselves, the parameter among `seen` (`wildApproxBounds`:
    /// `wildApprox(bounds, seen + tp)`, a parameter already seen an unbounded wildcard), so an
    /// `A <: Sys[A]` is a variable below `Sys[?]`.
    fn proto_wild_params(&mut self, t: TypeId, tparams: &[TParamId], seen: &[TParamId]) -> TypeId {
        if tparams.is_empty() || !self.mentions_tparam_of(t, Some(tparams)) {
            return t;
        }
        // A retained form first: what it is made over and its record's types, each occurrence its own approximation
        // (a fresh variable each: no pass), the form over them.
        if self.semantic {
            if let Some((under, r)) = self.types.form_of(t) {
                return self.proto_wild_params_form(t, tparams, seen, under, r);
            }
        }
        match self.types.get(t) {
            Type::Param(p) if tparams.contains(&p) => {
                let (lo, hi) = self.proto_wild_bounds(p, tparams, seen);
                // The variable keeps the bounds its parameter declares, as a poly application's
                // variable does (`instDirection` tells the constraint's apart from them).
                let (ty, v) = self.fresh_var_id();
                if self.syms.tparam(p).arity > 0 {
                    self.tvars[v].set_origin(p);
                }
                let (top, bottom) = (self.canon(hi) == ANY, self.canon(lo) == NOTHING);
                if !top || !bottom {
                    self.tvars[v].set_declared(lo, hi);
                    if !top {
                        self.is_sub(ty, hi);
                    }
                    if !bottom {
                        self.is_sub(lo, ty);
                    }
                }
                self.proto_vars.push(v);
                ty
            }
            // A higher-kinded parameter applied: each occurrence of one parameter the same
            // wildcard of its bounds (`wildApprox`'s `AppliedType` case over a wildcard
            // constructor applies the bounds to the arguments).
            Type::AppParam(..) => {
                let named: Vec<TParamId> = tparams.iter().copied().filter(|&p| self.mentions_tparam_of(t, Some(&[p]))).collect();
                let subst: Subst = named
                    .into_iter()
                    .map(|p| {
                        let (lo, hi) = self.proto_wild_bounds(p, tparams, seen);
                        (p, self.types.bounded_wild(lo, hi))
                    })
                    .collect();
                self.types.subst(t, &subst)
            }
            // Every other type is made again of its parts approximated (`wildApprox` is a
            // `TypeMap` over the whole type: an applied type's arguments, a refinement's parent
            // and info, a union's and an intersection's sides, a lambda's result, an annotated
            // type's parent, a path's prefix, a match type's parts), each occurrence of a part
            // its own approximation (`Map[T, T]` is a `Map[?, ?]` of two wildcards).
            _ => {
                let mut parts = Vec::new();
                self.types.parts(t, &mut parts);
                let mut mapped: Vec<(TypeId, Vec<TypeId>)> = Vec::with_capacity(parts.len());
                for &part in &parts {
                    let m = self.proto_wild_params(part, tparams, seen);
                    match mapped.iter_mut().find(|(q, _)| *q == part) {
                        Some((_, ms)) => ms.push(m),
                        None => mapped.push((part, vec![m])),
                    }
                }
                if mapped.iter().all(|(q, ms)| ms.iter().all(|m| m == q)) {
                    return t;
                }
                // The store maps the same parts, each occurrence taking the next approximation
                // made for its type.
                let mut queues = mapped;
                self.types.map_parts(t, &mut |part| match queues.iter_mut().find(|(q, _)| *q == part) {
                    Some((_, ms)) if !ms.is_empty() => ms.remove(0),
                    _ => part,
                })
            }
        }
    }

    #[cold]
    #[inline(never)]
    fn proto_wild_params_form(&mut self, t: TypeId, tparams: &[TParamId], seen: &[TParamId], under: TypeId, r: crate::types::RetainId) -> TypeId {
        let types = self.types.clone();
        let nu = self.proto_wild_params(under, tparams, seen);
        let nr = types.map_retention(r, &mut |x| self.proto_wild_params(x, tparams, seen));
        types.form_over(t, under, r, nu, nr)
    }

    /// The bounds of the parameter `p`'s wildcard (`wildApproxBounds`): none where `p` is among
    /// `seen`, else its declared bounds approximated with `p` seen.
    fn proto_wild_bounds(&mut self, p: TParamId, tparams: &[TParamId], seen: &[TParamId]) -> (TypeId, TypeId) {
        if seen.contains(&p) {
            return (NOTHING, ANY);
        }
        let (lo, hi) = (self.syms.tparam(p).lower, self.syms.tparam(p).upper);
        let mut seen: Vec<TParamId> = seen.to_vec();
        seen.push(p);
        let lo = if lo == NOTHING { lo } else { self.proto_wild_params(lo, tparams, &seen) };
        let hi = if hi == ANY { hi } else { self.proto_wild_params(hi, tparams, &seen) };
        (lo, hi)
    }

    /// Whether `t` names one of the prototype's wildcards (`proto_vars`), as `makeContextualFunction`
    /// asks `existsPart(_.isInstanceOf[WildcardType])` of a formal.
    pub(super) fn mentions_proto_var(&self, t: TypeId) -> bool {
        if self.proto_vars.is_empty() {
            return false;
        }
        let mut vars = Vec::new();
        self.collect_vars(t, &mut vars);
        vars.iter().any(|v| self.proto_vars.contains(v))
    }

    /// `Namer.inferredResultType`'s `rhsType` for a default getter: the parameter's type where
    /// the approximation changed nothing and the default's type conforms to it, frozen; otherwise
    /// the default's type, widened as an inferred result is.
    fn default_getter_result(&mut self, pt: TypeId, default_tp: TypeId, tp: TypeId) -> TypeId {
        if pt == default_tp {
            // The parameter's type itself where the default conforms to it, frozen (`frozen_<:<`).
            let mark = self.snapshot();
            let conforms = self.is_sub(tp, default_tp);
            self.rollback(&mark);
            if conforms {
                return default_tp;
            }
        }
        let t = self.solve_inferred(tp);
        let t = self.widen_annotated_singleton(t);
        if matches!(self.types.get(t), Type::Term(_) | Type::Select(..)) {
            let widened = self.widen_path(t);
            return self.widen_lit(widened);
        }
        self.widen_lit(t)
    }

    /// The traversal reaches the getters of `of` (`Typer.typedStats` after the method, at the
    /// companion's template for a constructor's): each completed where it is not, a demand of the
    /// definition being typed (`completed_sig`).
    pub(super) fn reach_getters(&mut self, of: DefaultOf) {
        if self.getters.of.is_empty() {
            return;
        }
        // Only a member with a default has a getter; no other's signature is read here.
        let Some(&n) = self.getters.count.get(&of) else { return };
        for i in 0..n as usize {
            if let Some(g) = self.getter_of(of, i) {
                self.sig_of(g);
            }
        }
    }

    /// The traversal reaches a class's constructor getters, its primary constructor's and its
    /// secondary constructors', where its companion's template is (`Desugar.classDef`'s
    /// `decompose` and `companionMembers`).
    pub(super) fn reach_class_getters(&mut self, c: ClassId) {
        if self.getters.of.is_empty() {
            return;
        }
        self.reach_getters(DefaultOf::Ctor(c));
        for ctor in self.syms.class(c).ctors.clone() {
            self.reach_getters(DefaultOf::Method(ctor));
        }
    }

    /// The defaults of the record this worker made for `m` (`TFun::defaults`), each its getter's
    /// tree where the getter is complete, before the record is published. A record published
    /// already (a body typed on demand, published before the traversal reached its getters) has one
    /// version until the merge, which a peer reads (`arena::EscapeMarks`): its defaults wait for the
    /// merge's `fill_getters`, and a reader before it asks the getter (`default_tree`).
    pub(super) fn fill_own_getters(&mut self, m: SymId) {
        if self.forked && self.shared_sym(m) && self.syms.sym(m).body() == crate::symbols::Completion::Done {
            return;
        }
        let Some(&f) = self.fun_of_sym.local.get(&m) else { return };
        let n = self.prog.funs[f.idx()].defaults.len();
        for i in 0..n {
            if self.prog.funs[f.idx()].defaults[i].is_none() {
                if let Some(tree) = self.getter_tree(DefaultOf::Method(m), i) {
                    self.prog.funs[f.idx()].defaults[i] = Some(tree);
                }
            }
        }
    }

    /// The getters of the defaults of `file`, completed again by their next reader as a retyped
    /// file's members are (`open_inferred`, `reopen_suspended`), each at its default's place in
    /// the file as it is now.
    pub(super) fn reopen_getters(&mut self, file: crate::source::FileId) {
        if self.getters.of.is_empty() {
            return;
        }
        let entries: Vec<((DefaultOf, u32), SymId)> = self.getters.of.iter().filter(|(_, &g)| self.syms.sym(g).file == file).map(|(&k, &g)| (k, g)).collect();
        for ((of, index), g) in entries {
            self.syms.syms[g.idx()].sig = None;
            self.syms.sym_cells.set(g.0, Completion::NotStarted);
            self.syms.body_cells.set(g.0, Completion::NotStarted);
            self.val_init.local.remove(&g);
            let expr = match of {
                DefaultOf::Method(m) => self.syms.sym(m).def.and_then(|d| match &self.ast(file).def(d).kind {
                    DefKind::Fun(f) => f.clauses.iter().flat_map(|c| c.params.iter()).nth(index as usize).and_then(|p| p.default),
                    _ => None,
                }),
                DefaultOf::Ctor(c) => self.syms.class(c).def.and_then(|d| match &self.ast(file).def(d).kind {
                    DefKind::Class(cls) => cls.clauses.iter().flat_map(|c| c.params.iter()).nth(index as usize).and_then(|p| p.default),
                    _ => None,
                }),
            };
            if let Some(e) = expr {
                let span = self.ast(file).expr_span(e);
                self.syms.syms[g.idx()].span = span;
            }
        }
    }

    /// The typed tree of the getter of parameter `index` of `of` where the getter is complete, for
    /// its method's record (`TFun::defaults`); none where it is not yet, which `fill_getters` fills.
    pub(super) fn getter_tree(&self, of: DefaultOf, index: usize) -> Option<TExprId> {
        let g = self.getter_of(of, index)?;
        self.val_init.get(&g).copied()
    }

    /// The tree of the default of parameter `index` of `of`, its getter completed where it is not
    /// yet: what a macro's run reads where the method's record has no default yet (a getter
    /// completed on another worker), as dotty's interpreter calls the getter.
    pub(crate) fn default_tree(&mut self, of: DefaultOf, index: usize) -> Option<TExprId> {
        let g = self.getter_of(of, index)?;
        self.sig_of(g);
        self.val_init.get(&g).copied()
    }

    /// The defaults of the members' records, each its getter's tree (`fill_getters`): run once the
    /// bodies are typed and merged, for the getters a call or another worker completed after the
    /// record was made.
    pub(super) fn fill_getters(&mut self) {
        if self.getters.of.is_empty() {
            return;
        }
        let entries: Vec<((DefaultOf, u32), SymId)> = self.getters.of.iter().map(|(&k, &g)| (k, g)).collect();
        let mut tclass_of: FxMap<ClassId, usize> = FxMap::default();
        for (i, tc) in self.prog.classes.iter().enumerate() {
            tclass_of.insert(tc.id, i);
        }
        for ((of, index), g) in entries {
            let Some(&tree) = self.val_init.get(&g) else { continue };
            let index = index as usize;
            match of {
                DefaultOf::Method(m) => {
                    if let Some(&f) = self.fun_of_sym.get(&m) {
                        let fun = &mut self.prog.funs[f.idx()];
                        if fun.defaults.len() > index && fun.defaults[index].is_none() {
                            fun.defaults[index] = Some(tree);
                        }
                    }
                }
                DefaultOf::Ctor(c) => {
                    if let Some(&ti) = tclass_of.get(&c) {
                        let tc = &mut self.prog.classes[ti];
                        let at = tc.captures + index;
                        if tc.ctor_defaults.len() > at && tc.ctor_defaults[at].is_none() {
                            tc.ctor_defaults[at] = Some(tree);
                        }
                    }
                }
            }
        }
    }
}

/// What a getter's completion reads of its method or class.
struct GetterSite {
    file: crate::source::FileId,
    def: ast::DefId,
    owner: Owner,
    member: Option<SymId>,
    class: Option<ClassId>,
    tparams: Vec<TParamId>,
    earlier: Vec<(Vec<ParamSig>, bool)>,
    param: ParamSig,
    expr: ast::ExprId,
}
