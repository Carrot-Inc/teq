//! Anonymous classes. `new T { ... }` becomes a final class of its own, created where the
//! expression stands and typed in the environment of that place. What its members use from there
//! (locals, using evidence, the `this` of enclosing classes) is passed to the constructor, so the
//! class itself can be emitted at the top level like any other.

use super::{Env, Frame, Worker};
use crate::ast::{self, mods, DefId, DefKind, TyExpr};
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::sync::Arc;

impl<'a> Worker<'a> {
    pub fn type_new_anon(&mut self, d: DefId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let file = self.env.file;
        let ast = self.cur_ast();
        let DefKind::Class(cls) = &ast.def(d).kind else { unreachable!() };
        // A class the body under the definition check makes is the stored body's own, which
        // each expansion copies (`InlineDefinition::classes`).
        let stored = self.stores_classes();
        if self.checks_inline_definition() && !stored {
            self.note_held(HeldForm::LocalClass);
            return (self.prog.add(TExpr::Unit), expected.unwrap_or(ERROR));
        }
        // An anonymous class in an inline body is a class of its own at each expansion, as
        // scalac duplicates it, since its members are typed under the expansion's arguments.
        let inlined = self.inline.depth > 0 || stored;
        if !inlined {
            if let Some(&c) = self.def_classes.get(file.0 as usize, &d) {
                return self.anon_instance(c);
            }
        }
        let first = &cls.parents[0];
        let class_parent = match self.plain_class_parent(first.ty) {
            Err(()) => return (self.prog.add(TExpr::Unit), ERROR),
            Ok(Some(k))
                if cls.parents.len() == 1 && cls.body.is_empty() && self.syms.class(k).mods & mods::ABSTRACT == 0 =>
            {
                let first_written = matches!(first.args.as_slice(), [(_, false), _, ..]);
                return self.type_new(first.ty, &first.args, first_written, span, expected);
            }
            Ok(k) => k,
        };
        if self.js_trait_parent(first.ty) {
            return self.type_js_object(cls, span, expected);
        }
        let name = match self.product_class_name(file, d) {
            Some(n) => n,
            None => self.class_name_at(span, stored),
        };
        let c = self.syms.new_class(name, ClassKind::Anon, mods::FINAL, Owner::Local, file, Some(d), span);
        self.record_self_alias(c);
        self.note_made_at(c, span, stored);
        if !inlined {
            self.def_classes.insert(file.0 as usize, d, c);
        }
        self.anon_envs.insert(c, Arc::new(self.env.clone()));
        self.enter_body(file, c, &cls.body);
        self.syms.class_mut(c).state().set(Completion::InProgress);
        let mut parents = Vec::with_capacity(cls.parents.len());
        // The arguments of the superclass are evaluated here, where the class is created, and
        // fix its type arguments when the parent clause leaves them out.
        let mut parent_args = None;
        for (i, p) in cls.parents.iter().enumerate() {
            let written = match ast.ty(p.ty) {
                TyExpr::Apply(..) => true,
                TyExpr::Resolved(t) => !matches!(self.types.get(t), Type::Ctor(_)),
                _ => false,
            };
            let infers = i == 0 && !p.args.is_empty() && !written && class_parent.map_or(false, |k| !self.syms.class(k).tparams.is_empty());
            let t = if infers {
                // `new C(args) { ... }` where a `C[X, Y]` is expected: the expected type fixes
                // the arguments, as scalac infers them from the args and the expected type.
                let from_expected = expected.and_then(|e| {
                    let d = self.dealias(e);
                    matches!(self.types.get(d), Type::Class(k, _) if Some(k) == class_parent).then_some(d)
                });
                let typed = self.type_parent_ctor(c, Some(p), class_parent.unwrap(), from_expected, true);
                parent_args = typed.map(|(call, _)| call);
                typed.map_or(ERROR, |(_, ty)| ty)
            } else {
                let r = self.resolve_anon_parent(p.ty, expected);
                r
            };
            parents.push((t, ast.ty_spans[p.ty.idx()]));
        }
        self.set_parents(c, parents);
        self.syms.class_mut(c).state().set(Completion::Done);
        let extends_class = class_parent.filter(|&k| self.syms.class(c).superclass == Some(k));
        if let (Some(k), None) = (extends_class, parent_args) {
            let parent_ty = self.syms.class(c).parents.first().copied();
            parent_args = self.type_parent_ctor(c, Some(first), k, parent_ty, true).map(|(call, _)| call);
        }
        let parent_args = parent_args.filter(|call| extends_class.is_some() && (!call.args.is_empty() || call.via.is_some()));
        let first_expr = self.prog.exprs.len();
        self.check_class(c);
        let captures = self.captured_by(c, first_expr);
        let tclass = self.checked_class_mut(c).expect("the class was just checked");
        tclass.ctor_defaults = vec![None; captures.len()];
        tclass.ctor_params = captures.clone();
        tclass.parent_args = parent_args.map(|call| call.args);
        tclass.parent_via = parent_args.and_then(|call| call.via);
        self.capture_through_nested(c, &captures);
        self.anon_captures.insert(c, captures);
        if let Some(call) = parent_args {
            self.anon_parent_args.insert(c, call);
        }
        self.anon_instance(c)
    }

    /// The class named by the first parent, if it is one rather than a trait: `new C(args) { }`
    /// is a plain `new C(args)`, and a class is what a body cannot extend. `Err` for a parent
    /// that does not resolve, which has been reported.
    fn plain_class_parent(&mut self, ty: ast::TyExprId) -> Result<Option<ClassId>, ()> {
        let head = match self.cur_ast().ty(ty) {
            TyExpr::Apply(f, _) => f,
            _ => ty,
        };
        let ctor = self.resolve_type_ctor(head);
        if ctor == ERROR {
            return Err(());
        }
        let Some(class) = self.ctor_class(ctor) else { return Ok(None) };
        self.complete_class(class);
        Ok(matches!(self.syms.class(class).kind, ClassKind::Class | ClassKind::EnumCase).then_some(class))
    }

    /// Whether the parent is a JS trait, whose anonymous instance is a plain object.
    fn js_trait_parent(&mut self, ty: ast::TyExprId) -> bool {
        let head = match self.cur_ast().ty(ty) {
            TyExpr::Apply(f, _) => f,
            _ => ty,
        };
        let ctor = self.resolve_type_ctor(head);
        let Some(class) = self.ctor_class(ctor) else { return false };
        self.complete_class(class);
        let info = self.syms.class(class);
        info.kind == ClassKind::Trait && info.js != JsKind::Scala
    }

    /// The class a resolved type constructor names: a class, a class through its prefix, a
    /// generic one through its prefix (`[A] =>> o.C[A]`).
    fn ctor_class(&self, ctor: TypeId) -> Option<ClassId> {
        match self.types.get(ctor) {
            Type::Ctor(c) => Some(c),
            Type::Lambda(_, body) if matches!(self.types.get(body), Type::Nested(..)) => self.types.named_class(body),
            _ => self.types.named_class(ctor).filter(|_| !matches!(self.types.get(ctor), Type::This(_))),
        }
    }

    /// `new T { val a = e }` for a JS trait `T`: an object literal of the members the body gives,
    /// without the traits' `js.undefined` vals that the body leaves out, but for a native trait's
    /// instance. A member of a native trait may be left out; an abstract one of a JS trait has to be
    /// given.
    fn type_js_object(&mut self, cls: &'a ast::ClassDef, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let mut parents = Vec::with_capacity(cls.parents.len());
        for p in &cls.parents {
            let pspan = ast.ty_spans[p.ty.idx()];
            if !p.args.is_empty() {
                self.error(pspan, "a JS trait takes no constructor arguments");
            }
            let t = self.resolve_anon_parent(p.ty, expected);
            match self.class_of(t) {
                Some(c) if self.syms.class(c).kind == ClassKind::Trait && self.syms.class(c).js != JsKind::Scala => {
                    parents.push(t);
                }
                _ if t == ERROR => {}
                _ => self.error(pspan, "the parents of an anonymous JS object have to be JS traits"),
            }
        }
        let Some((&first, rest)) = parents.split_first() else { return (self.prog.add(TExpr::Unit), ERROR) };
        let ty = rest.iter().fold(first, |acc, &p| self.types.inter(acc, p));
        // An instance of a native trait, which Scala.js refuses, keeps the literal it has always had:
        // the `js.undefined` vals of its non-native bases first, then the body's members in source order.
        let native = parents.iter().any(|&p| matches!(self.class_of(p), Some(c) if self.syms.class(c).js == JsKind::Native));

        // The bases of every parent, outermost last, each once.
        let mut bases: Vec<(ClassId, TypeId)> = Vec::new();
        for &p in &parents {
            let Type::Class(pc, _) = self.types.get(p) else { continue };
            let subst = self.owner_subst(p);
            for (bc, bt) in self.syms.class(pc).base_types.clone() {
                if !bases.iter().any(|&(c, _)| c == bc) {
                    bases.push((bc, self.types.subst(bt, &subst)));
                }
            }
        }
        let mut given: Vec<(crate::intern::Name, TExprId, bool)> = Vec::new();
        let mut names_given: Vec<crate::intern::Name> = Vec::new();
        for stmt in &cls.body {
            let (name, declared, rhs, stmt_span, assigns) = match *stmt {
                ast::Stmt::Def(d) => {
                    let def = ast.def(d);
                    match &def.kind {
                        DefKind::Val { pat: None, ty, rhs: Some(rhs) } if def.mods & mods::LAZY == 0 => {
                            (def.name, *ty, *rhs, def.span, false)
                        }
                        DefKind::Val { .. } => {
                            self.error(def.span, "a val of an anonymous JS object needs an initialiser and cannot be lazy");
                            continue;
                        }
                        _ => {
                            let msg = format!(
                                "{} cannot be defined in an anonymous JS object, which holds vals and assignments to \
                                 the vars of its trait; write a class for methods",
                                self.name_str(def.name)
                            );
                            self.error(def.span, msg);
                            continue;
                        }
                    }
                }
                ast::Stmt::Expr(e) => match ast.expr(e) {
                    ast::Expr::Assign(lhs, rhs) if matches!(ast.expr(lhs), ast::Expr::Ident(_)) => {
                        let ast::Expr::Ident(n) = ast.expr(lhs) else { unreachable!() };
                        (n, None, rhs, ast.expr_span(e), true)
                    }
                    _ => {
                        self.error(
                            ast.expr_span(e),
                            "an anonymous JS object holds vals and assignments to the vars of its trait only",
                        );
                        continue;
                    }
                },
                ast::Stmt::Import(_) => {
                    self.error(span, "imports are not supported in an anonymous JS object");
                    continue;
                }
            };
            let Some((sym, owner_ty)) = self.find_member(ty, name) else {
                let msg = format!("value {} is not a member of {}", self.name_str(name), self.show(ty));
                self.error(stmt_span, msg);
                self.type_expr(rhs, None);
                continue;
            };
            let kind = self.syms.sym(sym).kind;
            let (has_params, ret) = {
                let sig = self.sig_of(sym);
                (!sig.clauses.is_empty(), sig.ret)
            };
            if has_params || kind == SymKind::Given {
                let msg = format!("{} is a method of {}; an anonymous JS object gives vals only", self.name_str(name), self.show(ty));
                self.error(stmt_span, msg);
                self.type_expr(rhs, None);
                continue;
            }
            if assigns && kind != SymKind::Var {
                let msg = format!("reassignment to val {}", self.name_str(name));
                self.error(stmt_span, msg);
            }
            if names_given.contains(&name) {
                let msg = format!("{} is given twice", self.name_str(name));
                self.error(stmt_span, msg);
            }
            names_given.push(name);
            let subst = self.owner_subst(owner_ty);
            let member_ty = self.types.subst(ret, &subst);
            let target = match declared {
                Some(t) => {
                    let t = self.resolve_type(t);
                    if !self.is_sub(t, member_ty) {
                        let msg = format!(
                            "type mismatch: {} is declared as {} in {}, found {}",
                            self.name_str(name),
                            self.show(member_ty),
                            self.show(ty),
                            self.show(t)
                        );
                        self.error(stmt_span, msg);
                    }
                    t
                }
                None => member_ty,
            };
            let te = self.check_expr(rhs, target);
            let assigns_optional = !native
                && assigns
                && kind == SymKind::Var
                && !self.is_abstract_member(sym)
                && matches!(self.syms.sym(sym).owner, Owner::Class(c) if self.syms.class(c).js == JsKind::Object);
            given.push((self.syms.js_member_name(sym), te, assigns_optional));
        }

        let mut items: Vec<TExprId> = Vec::new();
        let mut missing: Vec<String> = Vec::new();
        let mut seen: Vec<crate::intern::Name> = Vec::new();
        for &(bc, _) in &bases {
            let (kind, js) = (self.syms.class(bc).kind, self.syms.class(bc).js);
            if kind != ClassKind::Trait || js == JsKind::Scala {
                continue;
            }
            for m in self.syms.class(bc).member_order.clone() {
                let mname = self.syms.sym(m).name;
                // An abstract var's setter is the var's property, given or missing with it.
                if seen.contains(&mname) || names_given.contains(&mname) || self.syms.sym(m).mods & crate::ast::mods::SETTER != 0 {
                    continue;
                }
                seen.push(mname);
                if js == JsKind::Object && self.is_abstract_member(m) {
                    missing.push(self.name_str(mname));
                } else if native && js == JsKind::Object && matches!(self.syms.sym(m).kind, SymKind::Val | SymKind::Var) {
                    let key = self.name_str(self.syms.js_member_name(m));
                    let key = self.prog.add_str(&key);
                    items.push(self.prog.add(TExpr::Str(key)));
                    items.push(self.prog.add(TExpr::Unit));
                }
            }
        }
        if !missing.is_empty() {
            missing.sort();
            let msg = format!("the anonymous JS object does not implement abstract member(s): {}", missing.join(", "));
            self.error(span, msg);
        }

        // Scala.js creates the fields of the body's vals before it runs the body's assignments to
        // the traits' optional vars, and every initialiser in source order: a val after the first
        // such assignment holds its key's place with `undefined` ahead of it, and the val's own
        // pair later in the literal sets the value without moving the key. A `__proto__` pair sets
        // the prototype rather than a property, and a literal may hold it once.
        let (before, after) = given.split_at(given.iter().position(|&(_, _, assigns)| assigns).unwrap_or(given.len()));
        let held = after
            .iter()
            .filter(|&&(name, _, assigns)| !assigns && self.name_ref(name) != "__proto__")
            .map(|&(name, _, _)| (name, None));
        let gives = |&(name, te, _): &(crate::intern::Name, TExprId, bool)| (name, Some(te));
        let pairs: Vec<_> = before.iter().map(gives).chain(held).chain(after.iter().map(gives)).collect();
        for (name, value) in pairs {
            let key = self.name_str(name);
            let key = self.prog.add_str(&key);
            items.push(self.prog.add(TExpr::Str(key)));
            items.push(match value {
                Some(te) => te,
                None => self.prog.add(TExpr::Unit),
            });
        }
        let l = self.prog.list(&items);
        let te = self.prog.add(TExpr::ObjLit(l));
        if self.capturing() {
            self.capture_form(te, crate::tir::capture::Form::JsObject(ty));
        }
        (te, ty)
    }

    /// A generic trait named without its type arguments takes them from the expected type.
    fn resolve_anon_parent(&mut self, ty: ast::TyExprId, expected: Option<TypeId>) -> TypeId {
        let ast = self.cur_ast();
        if let TyExpr::Apply(..) = ast.ty(ty) {
            return self.resolve_type(ty);
        }
        let ctor = self.resolve_type_ctor(ty);
        let c = match self.types.get(ctor) {
            Type::Ctor(c) => c,
            // A generic class through its prefix (`[S] =>> context.NonAcceptingState[S]`) takes
            // the arguments of the expected type over that prefix.
            Type::Lambda(_, body) if matches!(self.types.get(body), Type::Nested(..)) => {
                let c = self.types.named_class(body);
                let from_expected = self.concrete_expected(expected).map(|t| self.dealias(t));
                if let (Some(c), Some(t)) = (c, from_expected) {
                    if let Type::Class(ec, args) = self.types.get(self.types.strip_nested(t)) {
                        if ec == c {
                            let args = self.types.items(args).to_vec();
                            return self.types.apply_ctor(ctor, &args);
                        }
                    }
                }
                match c {
                    Some(c) => c,
                    None => return ctor,
                }
            }
            _ => return ctor,
        };
        let from_expected = self.concrete_expected(expected).map(|t| self.dealias(t));
        if let Some(t) = from_expected {
            if let Type::Class(ec, _) = self.types.get(t) {
                if ec == c {
                    return t;
                }
            }
        }
        let msg = format!(
            "the type arguments of {} cannot be inferred here; write them out",
            self.name_str(self.syms.class(c).name)
        );
        self.error(ast.ty_spans[ty.idx()], msg);
        ERROR
    }

    /// scalac's `Outer$$anon$1`, with the position of the expression (`Program::position`) in
    /// the place of the number, so that the name depends on the source alone. A class made
    /// while an inline method expands carries the position of the outermost call site as well,
    /// and those the call makes at one expression (a body expanded once per field) are numbered in the order
    /// the call makes them; the initialiser of `val a, b` is typed once per name, and the
    /// classes made after the first take a further number the same way.
    pub(super) fn anon_name(&mut self, span: Span) -> crate::intern::Name {
        let file = self.env.file;
        let site = self.expansion_site(span);
        let repeat = self.anon_sites.entry((file, span.start, site)).or_insert(0);
        *repeat += 1;
        let repeat = *repeat;
        let enclosing = match self.innermost_class() {
            Some(c) => self.name_str(self.syms.class(c).name),
            None => {
                let path = &self.source(self.env.file).path;
                let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
                format!("{}$package", file.strip_suffix(".scala").unwrap_or(file))
            }
        };
        let mut name = format!("{}$$anon${}", enclosing, self.prog.position(file, span.start));
        if let Some((site_file, site_start)) = site {
            name.push_str(&format!("${}", self.prog.position(site_file, site_start)));
        }
        if repeat > 1 {
            name.push_str(&format!("${}", repeat));
        }
        self.interner.intern(&name)
    }

    /// The name of the copy of a named local class a stored inline body makes, for the expansion
    /// under way: its own name with the outermost site's position and the repetition at that
    /// site, counted as `anon_name` counts, so that each expansion's class is a class of its own
    /// (the emitter adds the class's position, as for any named local class).
    pub(super) fn local_class_name(&mut self, span: Span, name: crate::intern::Name) -> crate::intern::Name {
        let file = self.env.file;
        let site = self.expansion_site(span);
        let repeat = self.anon_sites.entry((file, span.start, site)).or_insert(0);
        *repeat += 1;
        let repeat = *repeat;
        let mut text = self.name_str(name);
        if let Some((site_file, site_start)) = site {
            text.push_str(&format!("${}", self.prog.position(site_file, site_start)));
        }
        if repeat > 1 {
            text.push_str(&format!("${}", repeat));
        }
        self.interner.intern(&text)
    }

    /// Names the class `c` a product's inline body declares, entered again at each expansion of
    /// the body it is retyped for, by the expansion's site and repetition, as the copy of a
    /// stored body's class is (`C$<site>`): the class of the definition's place alone would be
    /// every expansion's name.
    pub(super) fn name_retyped_product_class(&mut self, c: ClassId) {
        let (file, def, span, name) = {
            let info = self.syms.class(c);
            (info.file, info.def, info.span, info.name)
        };
        if self.inline.sites.is_empty() || self.product_position(file, def).is_none() {
            return;
        }
        let name = self.local_class_name(span, name);
        self.syms.class_mut(c).name = name;
        self.note_made_at(c, span, false);
    }

    /// The outermost call site of the expansion under way, when the expression at `span` of the
    /// current file is not that site itself.
    fn expansion_site(&self, span: Span) -> Option<(FileId, u32)> {
        let here = (self.env.file, span.start);
        self.inline.sites.first().map(|s| (s.file, s.span.start)).filter(|&s| s != here)
    }

    /// The file whose typing makes what is typed now: the file of the outermost call site of
    /// the expansion under way, which makes the expansion again when it is typed again, or
    /// the file being typed.
    pub(super) fn typing_unit(&self) -> FileId {
        self.inline.sites.first().map_or(self.env.file, |s| s.file)
    }

    /// The name a converted product's anonymous class `d` of the pseudo file `file` had in its
    /// producer's build, which its origins record.
    fn product_class_name(&mut self, file: FileId, d: DefId) -> Option<crate::intern::Name> {
        let loaded = self.loaded.as_ref()?;
        let name = loaded.product_places.get(&(file, d))?.name.clone()?;
        Some(self.interner.intern(&name))
    }

    /// The token and offset of a converted product's definition in its source.
    fn unconverted_product_place(&mut self, c: ClassId) -> Option<(u64, u32)> {
        if self.syms.class(c).def.is_some() {
            return None;
        }
        let lc = self.loaded.as_ref()?.classes.get(&c).copied()?;
        let (source, offset) = if self.forked && crate::shared::lock_depth() == 0 {
            self.with_loader(|w| w.loaded_mut().definition_place(lc.file, lc.addr))?
        } else {
            self.loaded_mut().definition_place(lc.file, lc.addr)?
        };
        Some((self.loaded.as_ref()?.product_sources[source as usize].1, offset))
    }

    fn product_position(&self, file: FileId, def: Option<DefId>) -> Option<(u64, u32)> {
        let loaded = self.loaded.as_ref()?;
        let place = loaded.product_places.get(&(file, def?))?;
        Some((loaded.product_sources[place.source as usize].1, place.offset))
    }

    /// The name of an anonymous class made at `span`: `anon_name`'s, or, for a class the stored
    /// body of an inline method makes, none of the output's, which the copy an expansion makes
    /// takes (`InlineDefinition::classes`): the definition check consumes no name.
    fn class_name_at(&mut self, span: Span, stored: bool) -> crate::intern::Name {
        if stored {
            return self.interner.intern("$stored");
        }
        self.anon_name(span)
    }

    /// Records the anonymous class `c` made at `span` as the expansion site's, when one is under
    /// way: the site's module holds it, and the site's file being typed again makes it anew. A
    /// class the stored body makes is the record's, noted with it.
    pub(super) fn note_made_at(&mut self, c: ClassId, span: Span, stored: bool) {
        if stored {
            self.note_stored_class(c);
            return;
        }
        if let Some((file, _)) = self.expansion_site(span) {
            self.syms.class_mut(c).made_at = Some(file);
        }
        if self.capturing() {
            self.capture_expansion_class(c);
        }
    }

    pub fn anon_env(&self, c: ClassId) -> Env {
        match self.anon_envs.get(&c) {
            Some(env) => (**env).clone(),
            None => Env { file: self.syms.class(c).file, frames: Vec::new(), imports: Vec::new() },
        }
    }

    /// The locals of the creation site that the class body refers to: everything typed since
    /// `first` belongs to the body, apart from definitions completed on demand meanwhile, whose
    /// references to the same locals only add unused constructor parameters.
    pub(super) fn captured_by(&mut self, c: ClassId, first: usize) -> Vec<SymId> {
        let visible = self.visible_to_class(c);
        // The code of a hole inside a quote runs at the outer level and captures nothing; the
        // class is copied for each run of the quote with its holes filled (`Copier::class`).
        let quoted = self.quote.level > 0;
        let mut captures: Vec<SymId> = Vec::new();
        for i in first..self.prog.exprs.len() {
            let s = match self.prog.exprs[i] {
                TExpr::Local(s) | TExpr::CallStatic(s, _) => s,
                _ => continue,
            };
            if quoted && self.quote.in_hole_filler(i) {
                continue;
            }
            if visible.contains(&s) && !captures.contains(&s) {
                captures.push(s);
            }
        }
        captures
    }

    /// The locals a class made where `anon_envs` says can capture: those of the scopes there, of
    /// the call site's where it stands in an inline body, the enclosing instances' `this`, and
    /// the locals of the expansions around it.
    pub(super) fn visible_to_class(&mut self, c: ClassId) -> Vec<SymId> {
        let env = self.anon_envs[&c].clone();
        // A class in an inline body also sees the call site's scope, where `summonInline`
        // finds its givens.
        let site = self.inline.site_env.clone();
        let mut visible: Vec<SymId> = Vec::new();
        for frame in env.frames.iter().chain(site.iter().flat_map(|e| e.frames.iter())) {
            match frame {
                Frame::Locals { names, givens, .. } => {
                    visible.extend(names.iter().map(|&(_, s)| s));
                    visible.extend(givens.iter().copied());
                }
                Frame::Class(k) => {
                    if let Some(o) = self.outer_class(*k) {
                        visible.push(self.outer_this_sym(o));
                    }
                    if let Some(&s) = self.outer_this.get(k) {
                        visible.push(s);
                    }
                    // In an inline body `this` of the method's class is the receiver's local,
                    // whether or not a class elsewhere reads the class's `this` through its own.
                    if let Some((Some(local), _)) = self.inline_this(*k) {
                        visible.push(local);
                    }
                }
            }
        }
        // A class in a nested expansion reads the locals of the expansions around it.
        for locals in &self.inline.outer_locals {
            visible.extend(locals.iter().copied());
        }
        visible
    }

    /// The arguments that hand a lifted class what it captures. The `this` of a class the body
    /// names is the enclosing instance, or the receiver of the inline expansion the class is
    /// created in; `inside` the class itself every capture is read from its own field.
    pub(super) fn capture_args(&mut self, captures: &[SymId], inside: bool, selfs: &[(SymId, TExprId)]) -> Vec<TExprId> {
        let this_class = self.innermost_class();
        let mut args = Vec::with_capacity(captures.len());
        for &s in captures {
            let stands_for = self.outer_this.find(|_, &sym| sym == s).map(|(k, _)| k);
            let e = match stands_for {
                _ if inside => TExpr::Local(s),
                Some(k) if Some(k) == this_class => TExpr::This,
                // The `this` of an enclosing object is the object itself.
                Some(k) if self.syms.class(k).kind == ClassKind::Object => TExpr::Module(k),
                Some(k) => match self.inline_this(k) {
                    Some((Some(local), _)) => TExpr::Local(local),
                    Some((None, _)) => TExpr::This,
                    None => match self.outer_instance_beside(k) {
                        Some(e) => {
                            args.push(e);
                            continue;
                        }
                        None => TExpr::Local(s),
                    },
                },
                // The val of a local object, read inside the object's own body, is `this`.
                None => match selfs.iter().find(|&&(v, _)| v == s) {
                    Some(&(_, this)) => {
                        args.push(this);
                        continue;
                    }
                    None => TExpr::Local(s),
                },
            };
            args.push(self.prog.add(e));
        }
        args
    }

    /// The outer instance a parent class nested in `o` takes where the code does not stand in
    /// `o`, as scalac passes the parent type's prefix: the nearest enclosing instance that
    /// derives from `o` or has it in its self type (`object PatchImplicitRule extends Rule(..)`
    /// in a module trait of the cake), or, for an object nested in a class, the object read from
    /// the enclosing instance that holds it (`P.this.Helpers` for `object Two extends
    /// Helpers.Base(2)`).
    fn outer_instance_beside(&mut self, o: ClassId) -> Option<TExprId> {
        if self.inside_class(o) {
            return None;
        }
        let frames: Vec<ClassId> = self.env.frames.iter().rev().filter_map(|f| match f {
            Frame::Class(k) => Some(*k),
            _ => None,
        }).collect();
        for k in frames {
            if self.derives_from(k, o) || self.self_type_derives(k, o) {
                return Some(self.this_ref(k));
            }
        }
        let val = self.syms.class(o).inner_object?;
        let Owner::Class(h) = self.syms.sym(val).owner else { return None };
        let holder = self.holder_of(h);
        if holder != h && !self.derives_from(holder, h) || !self.inside_class(holder) {
            return None;
        }
        let recv = self.this_ref(holder);
        Some(self.prog.add(TExpr::Field(recv, val)))
    }

    /// The local objects whose bodies the code being typed stands in, each with the expression
    /// that is `this` of it here: what a capture of the object's own val stands for.
    pub(super) fn enclosing_object_selfs(&mut self) -> Vec<(SymId, TExprId)> {
        let objects: Vec<(ClassId, SymId)> = self
            .env
            .frames
            .iter()
            .filter_map(|f| match f {
                Frame::Class(k) => self.syms.class(*k).local_module.map(|s| (*k, s)),
                _ => None,
            })
            .collect();
        // The class reads the capture as the lazy val it names, so `this` goes in as a thunk.
        objects
            .into_iter()
            .map(|(k, s)| {
                let this = self.this_ref(k);
                (s, self.prog.add(TExpr::Lambda(crate::ast::ListRef::EMPTY, this)))
            })
            .collect()
    }

    /// The object nested in a class or trait whose lazy val is `v`, read from the enclosing
    /// instance that holds it, where the code stands in that class or one deriving from it.
    pub(super) fn inner_object_ref(&mut self, v: SymId) -> Option<TExprId> {
        let Owner::Class(o) = self.syms.sym(v).owner else { return None };
        let holder = self.holder_of(o);
        if !self.inside_class(holder) {
            return None;
        }
        let recv = self.this_ref(holder);
        Some(self.prog.add(TExpr::Field(recv, v)))
    }

    /// Whether the code being typed stands in the body of the local class `c`.
    fn inside_class(&self, c: ClassId) -> bool {
        self.env.frames.iter().any(|f| matches!(f, Frame::Class(k) if *k == c))
    }

    /// `new C(args)`: for a named local class the captures come first, once the class has been
    /// checked and they are known; a `new` typed before that is completed then.
    pub fn new_instance(&mut self, c: ClassId, args: ast::ListRef, span: Span) -> TExprId {
        if let Some(o) = self.outer_class(c) {
            let inside = self.inside_class(c);
            let mut items = vec![self.new_outer_arg(c, o, span)];
            // What the class takes of its lifted owner's captures, passed here where the owner
            // is done; inside the owner they are given once it is (`capture_through_nested`).
            if let Some(captures) = self.anon_captures.get(&c).cloned() {
                if self.lifted_owner(c).is_some_and(|l| !self.inside_class(l)) {
                    let selfs = self.enclosing_object_selfs();
                    items.extend(self.capture_args(&captures, inside, &selfs));
                }
            }
            items.extend_from_slice(&self.prog.expr_list(args).to_vec());
            let l = self.prog.list(&items);
            return self.prog.add(TExpr::New(c, l));
        }
        let info = self.syms.class(c);
        if info.owner != Owner::Local || info.kind == ClassKind::Anon {
            return self.prog.add(TExpr::New(c, args));
        }
        let inside = self.inside_class(c);
        let selfs = self.enclosing_object_selfs();
        match self.anon_captures.get(&c).cloned() {
            Some(captures) => {
                let mut items = self.capture_args(&captures, inside, &selfs);
                items.extend_from_slice(&self.prog.expr_list(args).to_vec());
                let l = self.prog.list(&items);
                self.prog.add(TExpr::New(c, l))
            }
            None => {
                let e = self.prog.add(TExpr::New(c, args));
                self.local_news.push((c, e, inside, selfs));
                e
            }
        }
    }

    /// The enclosing instance an instance of `c`, a class nested in the class `o`, is made
    /// with, its constructors' first argument (dotty's `ExplicitOuter.OuterOps.args`): `this` of
    /// the enclosing instance deriving from `o`, the outer instance inside `c` itself (the outer
    /// parameter in one of its secondary constructors, `ctor_outer`), a placeholder that the
    /// prefix's value takes the place of where it is made through one (`prefix_outer`).
    pub(super) fn new_outer_arg(&mut self, c: ClassId, o: ClassId, span: Span) -> TExprId {
        let outer = self.outer_this_sym(o);
        let inside = self.inside_class(c);
        let holder = self.holder_of(o);
        let enclosed = self.env.frames.iter().any(|f| matches!(f, Frame::Class(k) if *k == holder));
        if !inside && !enclosed && !self.new_prefixed && !self.is_library_class(c) {
            let msg = format!("not supported yet: an instance of {}, a class nested in a class, made outside that class", self.name_str(self.syms.class(c).name));
            self.error(span, msg);
        }
        if !inside && enclosed {
            return self.this_ref(holder);
        }
        self.capture_args(&[outer], inside, &[])[0]
    }

    /// The local class `c` has been checked: what its body captures leads its constructor
    /// parameters, and the instances created so far get those arguments.
    pub(super) fn finish_local_class(&mut self, c: ClassId, first: usize) {
        let captures = self.captured_by(c, first);
        let n = captures.len();
        if let Some(tc) = self.checked_class_mut(c) {
            tc.ctor_params.splice(0..0, captures.iter().copied());
            tc.ctor_defaults.splice(0..0, std::iter::repeat(None).take(n));
            tc.captures = n;
        }
        self.anon_captures.insert(c, captures.clone());
        let pending: Vec<(TExprId, bool, Vec<(SymId, TExprId)>)> =
            self.local_news.iter().filter(|(k, _, _, _)| *k == c).map(|(_, e, inside, selfs)| (*e, *inside, selfs.clone())).collect();
        self.local_news.retain(|(k, _, _, _)| *k != c);
        for (e, inside, selfs) in pending {
            let TExpr::New(_, args) = self.prog.expr(e) else { continue };
            let rest = self.prog.expr_list(args).to_vec();
            let mut items = self.capture_args(&captures, inside, &selfs);
            items.extend(rest);
            let l = self.prog.list(&items);
            self.prog.exprs[e.idx()] = TExpr::New(c, l);
        }
        self.capture_through_nested(c, &captures);
    }

    /// The lifted class `l` captures `captures`: a class nested in it, a member of it or of one of
    /// its members, reads those its body names from its own constructor parameters, after its
    /// outer instance, as `l` reads them from its own (scalac's `LambdaLift` reads them through
    /// the outer instances). A nested class takes what its body names and what the classes it
    /// makes take; each creation of one, or the parent call of a subclass, passes them, read in
    /// turn from the creator's own. An anonymous or local class inside `l` that makes one takes
    /// what it passes on after what it captures already.
    pub(super) fn capture_through_nested(&mut self, l: ClassId, captures: &[SymId]) {
        if captures.is_empty() {
            return;
        }
        let mut nested: Vec<ClassId> = Vec::new();
        let mut todo = vec![l];
        while let Some(k) = todo.pop() {
            // The classes its body defines, objects included, and the companions made for them.
            let mut inner: Vec<ClassId> = self.syms.class(k).nested.values().copied().collect();
            let (file, def) = (self.syms.class(k).file, self.syms.class(k).def);
            if let Some(d) = def {
                if let DefKind::Class(cls) = &self.ast(file).def(d).kind {
                    for st in &cls.body {
                        if let ast::Stmt::Def(nd) = *st {
                            inner.extend(self.def_classes.get(file.0 as usize, &nd).copied());
                        }
                    }
                }
            }
            inner.extend(inner.clone().into_iter().filter_map(|n| self.syms.class(n).companion));
            inner.sort();
            inner.dedup();
            for n in inner {
                if !nested.contains(&n) {
                    todo.push(n);
                    nested.push(n);
                }
            }
        }
        nested.retain(|&n| self.outer_class(n).is_some() && self.prog_index.class(&self.prog, n).is_some());
        if nested.is_empty() {
            return;
        }
        // The classes that take part: `l`, the nested ones, and the lifted ones inside `l` that
        // the walk meets a creation of, each with what it captures already.
        let mut parts: Vec<(ClassId, Part, Vec<SymId>)> = vec![(l, Part::Root, Vec::new())];
        parts.extend(nested.iter().map(|&n| (n, Part::Nested, Vec::new())));
        let mut names: Vec<Vec<SymId>> = Vec::new();
        let mut makes: Vec<Vec<usize>> = Vec::new();
        let mut news: Vec<(TExprId, usize)> = Vec::new();
        let mut parents: Vec<(usize, usize)> = Vec::new();
        let mut i = 0;
        while i < parts.len() {
            names.push(Vec::new());
            makes.push(Vec::new());
            let k = parts[i].0;
            let Some(index) = self.prog_index.class(&self.prog, k) else {
                i += 1;
                continue;
            };
            let tc = self.prog.classes[index].clone();
            for root in self.class_roots(std::slice::from_ref(&tc)) {
                let found: Vec<TExprId> = self.prog.descendants(root).collect();
                for e in found {
                    match self.prog.expr(e) {
                        TExpr::Local(s) | TExpr::CallStatic(s, _) if captures.contains(&s) => {
                            if !names[i].contains(&s) {
                                names[i].push(s);
                            }
                        }
                        TExpr::New(m, _) => {
                            let made = match parts.iter().position(|p| p.0 == m) {
                                Some(made) => made,
                                None if m != l && self.syms.class(m).owner == Owner::Local && self.lifted_inside(m, l) && self.prog_index.class(&self.prog, m).is_some() => {
                                    let anon = self.syms.class(m).kind == ClassKind::Anon;
                                    let has = self.anon_captures.get(&m).cloned().unwrap_or_default();
                                    parts.push((m, Part::Lifted { anon }, has));
                                    parts.len() - 1
                                }
                                None => continue,
                            };
                            if made == 0 {
                                continue;
                            }
                            news.push((e, made));
                            makes[i].push(made);
                            // An anonymous class's parent arguments are evaluated where it is made.
                            if let Part::Lifted { anon: true } = parts[made].1 {
                                if let Some(sup) = self.syms.class(m).superclass.and_then(|s| parts.iter().position(|p| p.0 == s && p.1 == Part::Nested)) {
                                    makes[i].push(sup);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            if i > 0 && tc.parent_args.is_some() {
                if let Some(sup) = self.syms.class(k).superclass.and_then(|s| parts.iter().position(|p| p.0 == s)).filter(|&s| s > 0) {
                    parents.push((i, sup));
                    if parts[i].1 != (Part::Lifted { anon: true }) {
                        makes[i].push(sup);
                    }
                }
            }
            i += 1;
        }
        // What each takes: what its body names, and what the classes it makes take that it
        // does not hold already, to the fixpoint.
        let mut needed: Vec<Vec<SymId>> = names.iter().enumerate().map(|(i, n)| if parts[i].1 == Part::Nested { n.clone() } else { Vec::new() }).collect();
        loop {
            let mut grew = false;
            for at in 1..parts.len() {
                for made in makes[at].clone() {
                    let extra: Vec<SymId> = needed[made].iter().copied().filter(|s| !parts[made].2.contains(s)).collect();
                    for s in extra {
                        if !needed[at].contains(&s) {
                            needed[at].push(s);
                            grew = true;
                        }
                    }
                }
            }
            if !grew {
                break;
            }
        }
        // What each takes anew, in the order `l` takes them.
        let taken: Vec<Vec<SymId>> =
            (0..parts.len()).map(|i| captures.iter().copied().filter(|s| i > 0 && needed[i].contains(s) && !parts[i].2.contains(s)).collect()).collect();
        // Where each one's new parameters stand: after the outer instance of a nested class,
        // after the captures of a lifted one.
        let before: Vec<usize> = parts.iter().map(|p| if p.1 == Part::Nested { 1 } else { p.2.len() }).collect();
        for (at, (k, part, has)) in parts.iter().enumerate() {
            let n = taken[at].len();
            if n == 0 {
                continue;
            }
            let all: Vec<SymId> = has.iter().chain(&taken[at]).copied().collect();
            self.anon_captures.insert(*k, all);
            let part = *part;
            if let Some(tc) = self.checked_class_mut(*k) {
                let after = before[at].min(tc.ctor_params.len());
                tc.ctor_params.splice(after..after, taken[at].iter().copied());
                tc.ctor_defaults.splice(after..after, std::iter::repeat(None).take(n));
                if part != (Part::Lifted { anon: true }) {
                    tc.captures += n;
                }
            }
        }
        // The parent calls: in the subclass's constructor, or for an anonymous class where it
        // is made.
        for &(at, sup) in &parents {
            if taken[sup].is_empty() {
                continue;
            }
            let k = parts[at].0;
            let Some(args) = self.checked_class_mut(k).and_then(|tc| tc.parent_args) else { continue };
            let items = self.with_captures_at(args, before[sup], &taken[sup]);
            if let Some(tc) = self.checked_class_mut(k) {
                tc.parent_args = Some(items);
            }
            if let Some(call) = self.anon_parent_args.get_mut(&k) {
                call.args = items;
            }
        }
        for (e, made) in news {
            let TExpr::New(m, args) = self.prog.expr(e) else { continue };
            let items = match parts[made].1 {
                Part::Lifted { anon: true } => {
                    let Some(index) = self.prog_index.class(&self.prog, m) else { continue };
                    let parent: Vec<TExprId> = self.prog.classes[index].parent_args.map_or(Vec::new(), |p| self.prog.expr_list(p).to_vec());
                    let old = self.prog.expr_list(args).to_vec();
                    let own = before[made].min(old.len());
                    if taken[made].is_empty() && old.len() - own == parent.len() && old[own..] == parent[..] {
                        continue;
                    }
                    let mut items = old[..own].to_vec();
                    items.extend(taken[made].iter().map(|&s| self.prog.add(TExpr::Local(s))));
                    items.extend(parent);
                    self.prog.list(&items)
                }
                _ if taken[made].is_empty() => continue,
                _ => self.with_captures_at(args, before[made], &taken[made]),
            };
            self.prog.exprs[e.idx()] = TExpr::New(m, items);
        }
    }

    /// Whether the lifted class `c` is made inside the lifted class `l`, in its body or a
    /// nested class's.
    fn lifted_inside(&self, c: ClassId, l: ClassId) -> bool {
        self.anon_envs.get(&c).is_some_and(|env| env.frames.iter().any(|f| matches!(f, Frame::Class(k) if *k == l)))
    }

    /// The lifted class a class nested in it at any depth reads its captures from.
    fn lifted_owner(&self, c: ClassId) -> Option<ClassId> {
        let mut k = c;
        loop {
            let Owner::Class(o) = self.syms.class(k).owner else { return None };
            if self.anon_envs.contains_key(&o) {
                return Some(o);
            }
            k = o;
        }
    }

    /// The arguments `args` of a creation with the captures `taken` put in at `at`, after
    /// what the class takes there already, each read where the creation stands.
    fn with_captures_at(&mut self, args: ast::ListRef, at: usize, taken: &[SymId]) -> ast::ListRef {
        let mut items = self.prog.expr_list(args).to_vec();
        let reads: Vec<TExprId> = taken.iter().map(|&s| self.prog.add(TExpr::Local(s))).collect();
        let at = at.min(items.len());
        items.splice(at..at, reads);
        self.prog.list(&items)
    }

    /// `new Anon(captures)`, typed as the parents the class implements.
    pub(super) fn anon_instance(&mut self, c: ClassId) -> (TExprId, TypeId) {
        let captures = self.anon_captures.get(&c).cloned().unwrap_or_default();
        let selfs = self.enclosing_object_selfs();
        let mut args = self.capture_args(&captures, false, &selfs);
        let parent_call = self.anon_parent_args.get(&c).copied();
        if let Some(call) = parent_call {
            args.extend_from_slice(self.prog.expr_list(call.args));
        }
        let l = self.prog.list(&args);
        let parents = self.syms.class(c).parents.clone();
        let mut ty = match parents.split_first() {
            Some((&first, rest)) => rest.iter().fold(first, |acc, &p| self.types.inter(acc, p)),
            None => self.b.t_any_ref,
        };
        // The type members the body defines refine the parents, as scalac types the instance.
        let mut aliases: Vec<AliasId> = self.syms.class(c).type_aliases.values().copied().collect();
        aliases.sort();
        for a in aliases {
            self.complete_alias(a);
            let (name, rhs, tparams, abstract_member) = {
                let i = &self.syms.aliases[a.idx()];
                (i.name, i.rhs, i.tparams.clone(), i.is_abstract())
            };
            if abstract_member || rhs == ERROR {
                continue;
            }
            let this = self.this_prefix(c);
            let rhs = self.as_seen_from(rhs, this, c);
            let rhs = if tparams.is_empty() {
                rhs
            } else {
                let ps: Vec<TypeId> = tparams.iter().map(|&p| self.types.param(p)).collect();
                let l = self.types.list(&ps);
                self.types.mk(Type::Lambda(l, rhs))
            };
            let r = self.types.refine(Refinement::Alias(name, rhs));
            ty = self.types.mk(Type::Refined(ty, r));
        }
        ty = self.refined_by_narrowing_members(c, ty);
        let instance = self.prog.add(TExpr::New(c, l));
        match parent_call.filter(|call| !call.prelude.is_empty()) {
            Some(call) => (self.prog.add(TExpr::Block(call.prelude, instance)), ty),
            None => (instance, ty),
        }
    }

    /// The parents `ty` of the anonymous class `c` refined by each public val or def of its
    /// body whose type is strictly narrower than the member it overrides (`new Base { val v: A
    /// = a }` is a `Base { val v: A }`), as scalac's `classBound` types the instance.
    fn refined_by_narrowing_members(&mut self, c: ClassId, parents: TypeId) -> TypeId {
        let mut members: Vec<SymId> = self.syms.class(c).members.values().copied().filter(|&m| self.syms.alternatives(m).is_none()).collect();
        members.sort();
        let mut ty = parents;
        for m in members {
            let (name, kind, private) = {
                let i = self.syms.sym(m);
                (i.name, i.kind, i.mods & crate::ast::mods::PRIVATE != 0)
            };
            if private || name == crate::names::INIT || !matches!(kind, SymKind::Val | SymKind::Def) {
                continue;
            }
            let Some((inherited, owner_ty)) = self.find_member(parents, name) else { continue };
            if self.syms.alternatives(inherited).is_some() || self.is_private(inherited) {
                continue;
            }
            let own = self.sig_arc(m);
            if !own.tparams.is_empty() {
                continue;
            }
            let theirs = self.sig_arc(inherited);
            let subst = self.owner_subst(owner_ty);
            let shape = |sig: &MethodSig| sig.clauses.iter().map(|c| c.params.len()).collect::<Vec<_>>();
            if !theirs.tparams.is_empty() || shape(&own) != shape(&theirs) {
                continue;
            }
            let this = self.this_prefix(c);
            let own_ret = self.as_seen_from(own.ret, this, c);
            let their_ret = self.types.subst(theirs.ret, &subst);
            let mark = self.trail.len();
            let narrower = self.is_sub(own_ret, their_ret) && !self.is_sub(their_ret, own_ret);
            self.rollback(mark);
            if !narrower {
                continue;
            }
            let r = if kind == SymKind::Val && own.clauses.is_empty() {
                Refinement::Val(name, m, own_ret)
            } else {
                let mut seen = (*own).clone();
                seen.ret = own_ret;
                Refinement::Term(name, m, self.sig_types(&seen))
            };
            let r = self.types.refine(r);
            ty = self.types.mk(Type::Refined(ty, r));
        }
        ty
    }

    /// The trait with a single abstract method that a lambda of `arity` parameters is expected
    /// to implement, with that method and its signature as seen from the trait type. An open
    /// variable stands for the one such trait among its upper bounds (`List[Handler]` expected
    /// of `List(x => ...)`). The trait carries no evidence and its members leave exactly one
    /// method open, which takes `arity` plain parameters and no type parameters.
    pub fn sam_method(&mut self, t: TypeId, arity: usize) -> Option<(TypeId, SymId, Arc<MethodSig>, Subst)> {
        let t = self.deref_alias(t);
        if let Type::Var(v) = self.types.get(t) {
            let mut found = None;
            for u in self.tvars[v].upper.clone() {
                let u = self.deref(u);
                if matches!(self.types.get(u), Type::Class(..)) {
                    match (self.sam_method(u, arity), &found) {
                        (Some(sam), None) => found = Some(sam),
                        (Some(_), Some(_)) => return None,
                        (None, _) => {}
                    }
                }
            }
            return found;
        }
        let Type::Class(c, _) = self.types.get(t) else { return None };
        self.complete_class(c);
        let function_like = self.is_function_class(c)
            || self.is_partial_function(c)
            || Some(c) == self.b.sub_evidence
            || Some(c) == self.b.eq_evidence;
        // A trait, or an abstract class with a constructor that takes nothing, as Scala's SAM
        // conversion has it (monocle's `Index.apply` is a lambda where an `Index` is expected).
        let info = self.syms.class(c);
        let abstract_class = info.kind == ClassKind::Class && info.mods & crate::ast::mods::ABSTRACT != 0 && info.ctor.iter().all(|cl| cl.params.is_empty());
        if function_like || (info.kind != ClassKind::Trait && !abstract_class) {
            return None;
        }
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        if bases.iter().any(|&b| self.has_evidence_traits(b) || !self.syms.class(b).ctor.is_empty()) {
            return None;
        }
        let mut open: Option<SymId> = None;
        let mut seen: Vec<crate::intern::Name> = Vec::new();
        for &b in &bases {
            let members = self.syms.class(b).member_order.clone();
            for m in members {
                let name = self.syms.sym(m).name;
                // An alternative of an overloaded name is open unless one with its parameters
                // implements it; the others of its name say nothing about it.
                if self.syms.sym(m).alternative {
                    if self.is_abstract_member(m) && !self.alternative_is_implemented(c, m) {
                        if open.is_some() {
                            return None;
                        }
                        open = Some(m);
                    }
                    continue;
                }
                if seen.contains(&name) {
                    continue;
                }
                seen.push(name);
                // The first declaration in linearisation order decides; a concrete one closes the name.
                let implemented = bases
                    .iter()
                    .filter_map(|&other| self.syms.class(other).members.get(&name).copied())
                    .any(|s| !self.is_abstract_member(s));
                if implemented || !self.is_abstract_member(m) {
                    continue;
                }
                if open.is_some() {
                    return None;
                }
                open = Some(m);
            }
        }
        let m = open?;
        if self.syms.sym(m).kind != SymKind::Def || self.syms.sym(m).is_extension {
            return None;
        }
        let sig = self.sig_arc(m);
        let fits = sig.tparams.is_empty()
            && sig.clauses.len() == 1
            && !sig.clauses[0].is_using
            && sig.clauses[0].params.len() == arity;
        if !fits {
            return None;
        }
        let t = self.approx_sam_wildcards(t, c, m);
        let (_, owner_ty) = self.find_member(t, self.syms.sym(m).name)?;
        let subst = self.owner_subst(owner_ty);
        Some((t, m, sig, subst))
    }

    /// scalac's `SAMType.samParent`: a wildcard argument of the expected trait becomes its lower
    /// bound when its parameter occurs only contravariantly in the method, else its upper one,
    /// so that `Function[? >: String, ? <: Int]` is implemented as a `Function[String, Int]`.
    fn approx_sam_wildcards(&mut self, t: TypeId, c: ClassId, m: SymId) -> TypeId {
        let Type::Class(_, args) = self.types.get(t) else { return t };
        let args = self.types.items(args).to_vec();
        if !args.iter().any(|&a| self.types.is_wild(a)) {
            return t;
        }
        self.settle_class(c);
        let tparams = self.syms.class(c).tparams.clone();
        let own: Vec<TypeId> = tparams.iter().map(|&p| self.types.param(p)).collect();
        let generic = self.types.class(c, &own);
        let Some((_, owner_ty)) = self.find_member(generic, self.syms.sym(m).name) else { return t };
        let subst = self.owner_subst(owner_ty);
        let sig = self.sig_arc(m);
        let mut variances = Vec::new();
        for p in &sig.clauses[0].params {
            let ty = self.types.subst(p.ty, &subst);
            self.collect_param_variances(ty, -1, &mut variances);
        }
        let ret = self.types.subst(sig.ret, &subst);
        self.collect_param_variances(ret, 1, &mut variances);
        let approx: Vec<TypeId> = args
            .iter()
            .zip(&tparams)
            .map(|(&a, p)| match self.types.wild_bounds(a) {
                Some((lo, hi)) => match variances.iter().find(|(q, _)| q == p) {
                    Some((_, -1)) => lo,
                    _ => hi,
                },
                None => a,
            })
            .collect();
        self.types.class(c, &approx)
    }

    /// The type parameters of `t` with the variance each occurs with: `1` or `-1` when every
    /// occurrence agrees, `0` otherwise.
    fn collect_param_variances(&mut self, t: TypeId, sign: i8, out: &mut Vec<(TParamId, i8)>) {
        let t = self.deref_alias(t);
        match self.types.get(t) {
            Type::Param(p) => match out.iter_mut().find(|(q, _)| *q == p) {
                Some((_, s)) if *s != sign => *s = 0,
                Some(_) => {}
                None => out.push((p, sign)),
            },
            Type::Class(k, args) => {
                self.settle_class(k);
                let items = self.types.items(args).to_vec();
                for (i, a) in items.into_iter().enumerate() {
                    let v = self.syms.class(k).tparams.get(i).map_or(0, |&q| self.syms.tparam(q).variance);
                    self.collect_param_variances(a, sign * v, out);
                }
            }
            Type::Union(a, b) | Type::Inter(a, b) => {
                self.collect_param_variances(a, sign, out);
                self.collect_param_variances(b, sign, out);
            }
            _ => {}
        }
    }

    /// Whether an ancestor of `c`, or `c` itself, has a concrete method with the parameters of
    /// the abstract alternative `m`.
    fn alternative_is_implemented(&mut self, c: ClassId, m: SymId) -> bool {
        let name = self.syms.sym(m).name;
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
        let Owner::Class(mc) = self.syms.sym(m).owner else { return false };
        let Some(&(_, m_owner)) = bases.iter().find(|&&(b, _)| b == mc) else { return false };
        for &(b, _) in &bases {
            let mut k = 0;
            while let Some(s) = self.own_alternative(b, name, k) {
                k += 1;
                if s != m && !self.is_abstract_member(s) && self.same_parameters(c, &bases, s, m, m_owner) {
                    return true;
                }
            }
        }
        false
    }

    /// A lambda where a trait with a single abstract method is expected implements that method
    /// in an anonymous class, as scalac's SAM conversion does.
    pub fn sam_lambda(
        &mut self,
        trait_ty: TypeId,
        method: SymId,
        sig: Arc<MethodSig>,
        subst: Subst,
        params: &[ast::LambdaParam],
        body: ast::ExprId,
        span: Span,
    ) -> (TExprId, TypeId) {
        let stored = self.stores_classes();
        if self.checks_inline_definition() && !stored {
            self.note_held(HeldForm::SamLambda);
            return (self.prog.add(TExpr::Unit), trait_ty);
        }
        let file = self.env.file;
        // A product's SAM class takes the name and the place its producer gave it.
        let recorded = self.cur_ast().reader.as_deref().and_then(|r| r.sam_classes.get(&body).cloned());
        let name = match &recorded {
            Some((n, ..)) if !stored => self.interner.intern(n),
            _ => self.class_name_at(span, stored),
        };
        let c = self.syms.new_class(name, ClassKind::Anon, mods::FINAL, Owner::Local, file, None, span);
        if let Some((_, source, offset)) = recorded.filter(|_| !stored) {
            self.loaded_mut().product_class_places.insert(c, (source, offset));
        }
        self.note_made_at(c, span, stored);
        self.anon_envs.insert(c, Arc::new(self.env.clone()));
        self.syms.class_mut(c).state().set(Completion::InProgress);
        self.set_parents(c, vec![(trait_ty, span)]);
        self.syms.class_mut(c).state().set(Completion::Done);
        self.class_done.insert(c, ());
        self.syms.check_cells.set(c.0, Completion::Done);

        let mname = self.syms.sym(method).name;
        let msym = self.syms.new_sym(mname, SymKind::Def, mods::OVERRIDE, Owner::Class(c), file, None, span);
        let mut param_syms = Vec::with_capacity(params.len());
        let mut param_sigs = Vec::with_capacity(params.len());
        // A declared parameter type takes every argument the method's parameter does.
        let mut declared_narrower = false;
        for (lp, p) in params.iter().zip(&sig.clauses[0].params) {
            // A repeated parameter `A*` is a `Seq[A]` in the body, a by-name `=> A` an `A` read
            // where the body reads it; the method keeps the forms it overrides.
            let seq = |t: &mut Self, elem: TypeId| match t.seq_class() {
                Some(c) => t.types.class(c, &[elem]),
                None => ERROR,
            };
            let (ty, elem) = match lp.ty {
                Some(t) => {
                    let declared = self.resolve_type(t);
                    let offered = self.types.subst(p.ty, &subst);
                    let expected = if p.repeated { seq(self, offered) } else { offered };
                    let mark = self.snapshot();
                    // scalac takes no declared type for a by-name parameter.
                    if p.by_name || (!self.types.has_wild(expected) && expected != ERROR && declared != ERROR && !self.is_sub(expected, declared)) {
                        self.rollback(mark);
                        declared_narrower = true;
                    }
                    (declared, if p.repeated { offered } else { declared })
                }
                None => {
                    let t = self.types.subst(p.ty, &subst);
                    let t = self.solve_maximized_in(t);
                    (if p.repeated { seq(self, t) } else { t }, t)
                }
            };
            let s = self.new_local(lp.name, SymKind::Val, ty, lp.span);
            if p.by_name {
                self.syms.sym_mut(s).by_name = true;
            }
            param_syms.push(s);
            param_sigs.push(ParamSig { name: lp.name, ty: elem, by_name: p.by_name, repeated: p.repeated, has_default: false, sym: s });
        }
        let ret = self.types.subst(sig.ret, &subst);
        // A result that depends on the method's parameter (`TypeTest.unapply(x: S): Option[x.type
        // & T]`) depends on the lambda's own parameter here, which stands for it.
        let ret = self.sam_dependent_ret(ret, &sig.clauses[0].params, &param_syms);
        if declared_narrower {
            let param_tys: Vec<TypeId> = param_sigs.iter().map(|p| p.ty).collect();
            let found = self.fun_type(&param_tys, ret);
            let msg = format!("type mismatch: found {}, required {}", self.show(found), self.show(trait_ty));
            self.error(span, msg);
        }
        if self.as_context_function(ret).is_some() {
            let msg = format!(
                "Implementation restriction: cannot convert this expression to `{}` because its result type `{}` is a contextual function type.",
                self.show(trait_ty),
                self.show(ret)
            );
            self.error(span, msg);
        }
        let msig = Arc::new(MethodSig {
            tparams: Vec::new(),
            clauses: vec![ClauseSig { params: param_sigs, is_using: false, is_implicit: false }],
            ret,
        });
        {
            let mut s = self.syms.sym_mut(msym);
            s.sig = Some(msig);
            s.state().set(Completion::Done);
        }
        self.syms.add_member(c, msym);
        self.name_like(msym, method);
        if self.syms.sym(method).alternative {
            self.push_override(msym, method);
        }

        let first_expr = self.prog.exprs.len();
        let pending = self.attempts.pending_len();
        self.env.frames.push(Frame::Class(c));
        self.push_scope();
        for (lp, &s) in params.iter().zip(&param_syms) {
            self.bind_local(lp.name, s);
        }
        self.sam_classes.insert(c, ());
        let tb = match self.concrete_expected(Some(ret)) {
            Some(r) => self.check_expr(body, r),
            None => {
                let (tb, ty) = self.type_expr(body, Some(ret));
                self.adapt(tb, ty, ret, span)
            }
        };
        self.pop_scope();
        self.env.frames.pop();
        // Published once pushed, as a checked class is, and its captures read off the method's
        // nodes: its plain inline calls are expanded first.
        self.flush_pending_since(pending);
        self.sam_class_body(c, msym, param_syms, tb, first_expr)
    }

    /// The body of the SAM class `c`, whose one method `msym` over `param_syms` is `body`, and
    /// its instance; `first_expr` is where the body's expressions start, for its captures.
    fn sam_class_body(&mut self, c: ClassId, msym: SymId, param_syms: Vec<SymId>, body: TExprId, first_expr: usize) -> (TExprId, TypeId) {
        let captures = self.captured_by(c, first_expr);
        let f = self.prog.add_fun(TFun {
            sym: msym,
            defaults: vec![None; param_syms.len()],
            params: param_syms,
            body: Some(body),
        });
        self.fun_of_sym.insert(msym, f);
        let tclass = TClass {
            parent_via: None,
            ctors: Vec::new(),
            id: c,
            ctor_defaults: vec![None; captures.len()],
            ctor_params: captures.clone(),
            captures: captures.len(),
            parent_args: None,
            parent_prelude: ast::ListRef::EMPTY,
            inherited_case_members: 0,
            init: self.parent_inits(c, &mut Vec::new()),
            methods: vec![f],
            forwarders: Vec::new(),
            bridges: Vec::new(),
            super_accessors: Vec::new(),
            deferred_givens: Vec::new(),
        };
        self.push_class_body(tclass);
        self.anon_captures.insert(c, captures);
        self.anon_instance(c)
    }

    /// An instance of the SAM trait `trait_ty` whose method is a body the compiler types itself:
    /// the class and the method made as `sam_lambda` makes them, the method's parameters as the
    /// trait's signature has them, and `body` given the parameter symbols. None where the trait
    /// has no single abstract method of `arity` parameters.
    pub(super) fn sam_instance_typed(
        &mut self,
        trait_ty: TypeId,
        arity: usize,
        span: Span,
        body: impl FnOnce(&mut Self, &[(SymId, TypeId)]) -> TExprId,
    ) -> Option<(TExprId, TypeId)> {
        let (trait_ty, method, sig, subst) = self.sam_method(trait_ty, arity)?;
        let stored = self.stores_classes();
        if self.checks_inline_definition() && !stored {
            self.note_held(HeldForm::SamLambda);
            return Some((self.prog.add(TExpr::Unit), trait_ty));
        }
        let file = self.env.file;
        let name = self.class_name_at(span, stored);
        let c = self.syms.new_class(name, ClassKind::Anon, mods::FINAL, Owner::Local, file, None, span);
        self.note_made_at(c, span, stored);
        self.anon_envs.insert(c, Arc::new(self.env.clone()));
        self.syms.class_mut(c).state().set(Completion::InProgress);
        self.set_parents(c, vec![(trait_ty, span)]);
        self.syms.class_mut(c).state().set(Completion::Done);
        self.class_done.insert(c, ());
        self.syms.check_cells.set(c.0, Completion::Done);

        let mname = self.syms.sym(method).name;
        let msym = self.syms.new_sym(mname, SymKind::Def, mods::OVERRIDE, Owner::Class(c), file, None, span);
        let mut params = Vec::with_capacity(arity);
        let mut param_sigs = Vec::with_capacity(arity);
        for p in &sig.clauses[0].params {
            let ty = self.types.subst(p.ty, &subst);
            let ty = self.solve_maximized_in(ty);
            let s = self.new_local(p.name, SymKind::Val, ty, span);
            params.push((s, ty));
            param_sigs.push(ParamSig { name: p.name, ty, by_name: false, repeated: false, has_default: false, sym: s });
        }
        let ret = self.types.subst(sig.ret, &subst);
        let param_syms: Vec<SymId> = params.iter().map(|&(s, _)| s).collect();
        let ret = self.sam_dependent_ret(ret, &sig.clauses[0].params, &param_syms);
        let msig = Arc::new(MethodSig {
            tparams: Vec::new(),
            clauses: vec![ClauseSig { params: param_sigs, is_using: false, is_implicit: false }],
            ret,
        });
        {
            let mut s = self.syms.sym_mut(msym);
            s.sig = Some(msig);
            s.state().set(Completion::Done);
        }
        self.syms.add_member(c, msym);
        self.name_like(msym, method);
        if self.syms.sym(method).alternative {
            self.push_override(msym, method);
        }

        let first_expr = self.prog.exprs.len();
        self.env.frames.push(Frame::Class(c));
        self.push_scope();
        self.sam_classes.insert(c, ());
        let tb = body(self, &params);
        self.pop_scope();
        self.env.frames.pop();
        Some(self.sam_class_body(c, msym, param_syms, tb, first_expr))
    }

    /// The SAM method's result type with the paths of the trait method's parameters replaced by
    /// the implementing method's own (`x.type & T` of `unapply(x: S)` over the new `x`).
    fn sam_dependent_ret(&mut self, ret: TypeId, params: &[ParamSig], syms: &[SymId]) -> TypeId {
        if !self.types.has_paths(ret) {
            return ret;
        }
        let terms: Vec<(SymId, TypeId)> = params.iter().zip(syms).map(|(p, &s)| (p.sym, self.types.mk(Type::Term(s)))).collect();
        self.subst_paths(ret, &terms)
    }

    /// A class body checked, into the program's classes, or kept with the record of the stored
    /// body that makes it (`keep_stored_class`).
    pub(super) fn push_class_body(&mut self, tclass: TClass) {
        if let Err(tclass) = self.keep_stored_class(tclass) {
            let id = self.prog.classes.push(tclass);
            self.note_nested_in_check(id);
        }
    }

    /// A function value where a trait with a single abstract method is expected, the shape an
    /// eta-expansion takes there (`cache.computeIfAbsent(key, parse)` against the JDK's
    /// `Function`): an anonymous class whose method applies the function, held in a local of
    /// the enclosing block so that the class captures it.
    pub fn sam_from_function(
        &mut self,
        trait_ty: TypeId,
        method: SymId,
        sig: Arc<MethodSig>,
        subst: Subst,
        f: TExprId,
        f_ty: TypeId,
        span: Span,
    ) -> (TExprId, TypeId) {
        let stored = self.stores_classes();
        if self.checks_inline_definition() && !stored {
            self.note_held(HeldForm::SamLambda);
            return (self.prog.add(TExpr::Unit), trait_ty);
        }
        let file = self.env.file;
        self.push_scope();
        let fsym = self.indexed_local("sam", span.start, f_ty, span);
        self.bind_local(self.syms.sym(fsym).name, fsym);
        let name = self.class_name_at(span, stored);
        let c = self.syms.new_class(name, ClassKind::Anon, mods::FINAL, Owner::Local, file, None, span);
        self.note_made_at(c, span, stored);
        self.anon_envs.insert(c, Arc::new(self.env.clone()));
        self.syms.class_mut(c).state().set(Completion::InProgress);
        self.set_parents(c, vec![(trait_ty, span)]);
        self.syms.class_mut(c).state().set(Completion::Done);
        self.class_done.insert(c, ());
        self.syms.check_cells.set(c.0, Completion::Done);

        let mname = self.syms.sym(method).name;
        let msym = self.syms.new_sym(mname, SymKind::Def, mods::OVERRIDE, Owner::Class(c), file, None, span);
        let mut param_syms = Vec::with_capacity(sig.clauses[0].params.len());
        let mut param_sigs = Vec::with_capacity(sig.clauses[0].params.len());
        for (i, p) in sig.clauses[0].params.iter().enumerate() {
            let ty = self.types.subst(p.ty, &subst);
            let ty = self.solve_maximized_in(ty);
            let s = self.indexed_local("p", i as u32, ty, span);
            if p.by_name {
                self.syms.sym_mut(s).by_name = true;
            }
            param_syms.push(s);
            param_sigs.push(ParamSig { name: p.name, ty, by_name: p.by_name, repeated: false, has_default: false, sym: s });
        }
        // A by-name parameter is passed on as the thunk where the function takes one, and
        // evaluated for it where it takes the value.
        let fn_params = self.as_function(f_ty).map(|(ps, _)| ps).unwrap_or_default();
        let thunk_taken: Vec<bool> =
            (0..param_syms.len()).map(|i| fn_params.get(i).map_or(false, |&t| self.b.by_name.is_some() && self.by_name_arg(t).is_some())).collect();
        let ret = self.types.subst(sig.ret, &subst);
        if self.as_context_function(ret).is_some() {
            let msg = format!(
                "Implementation restriction: cannot convert this expression to `{}` because its result type `{}` is a contextual function type.",
                self.show(trait_ty),
                self.show(ret)
            );
            self.error(span, msg);
        }
        let msig = Arc::new(MethodSig {
            tparams: Vec::new(),
            clauses: vec![ClauseSig { params: param_sigs, is_using: false, is_implicit: false }],
            ret,
        });
        {
            let mut s = self.syms.sym_mut(msym);
            s.sig = Some(msig);
            s.state().set(Completion::Done);
        }
        self.syms.add_member(c, msym);
        self.name_like(msym, method);
        if self.syms.sym(method).alternative {
            self.push_override(msym, method);
        }

        let first_expr = self.prog.exprs.len();
        self.env.frames.push(Frame::Class(c));
        self.sam_classes.insert(c, ());
        let fref = self.prog.add(TExpr::Local(fsym));
        let args: Vec<TExprId> = param_syms
            .iter()
            .zip(&thunk_taken)
            .map(|(&s, &thunk)| {
                let local = self.prog.add(TExpr::Local(s));
                if self.syms.sym(s).by_name && !thunk {
                    self.prog.add(TExpr::CallClosure(local, ast::ListRef::EMPTY))
                } else {
                    local
                }
            })
            .collect();
        let l = self.prog.list(&args);
        let mut body = self.prog.add(TExpr::CallClosure(fref, l));
        // The closure's own result type, widened to the method's where that is wider (`apply(x:
        // Int): Long` over an `Int => Int`): a backend unboxes the erased result to the closure's
        // kind and widens from there, as scalac's `(x: Int) => m(x)` does.
        if let Some((_, fret)) = self.as_function(f_ty) {
            self.prog.set_type(body, fret);
            body = self.widen_numeric(body, fret, ret).unwrap_or(body);
        }
        self.env.frames.pop();
        let (instance, ty) = self.sam_class_body(c, msym, param_syms, body, first_expr);
        self.pop_scope();
        let stmts = self.prog.stmts.push_slice(&[TStmt::Val(fsym, f)]);
        (self.prog.add(TExpr::Block(stmts, instance)), ty)
    }

    /// The receiver for a member of the class `c` in scope: `this`, or inside a lifted class
    /// (anonymous or local) nested in `c` the local that stands for the `this` of `c` there.
    pub fn this_ref(&mut self, c: ClassId) -> TExprId {
        match self.inline_this(c) {
            Some((Some(proxy), _)) => {
                return match self.inline_arg(proxy) {
                    Some((receiver, _)) => receiver,
                    None => self.prog.add(TExpr::Local(proxy)),
                }
            }
            Some((None, _)) if self.syms.class(c).kind == ClassKind::Object => return self.prog.add(TExpr::Module(c)),
            _ => {}
        }
        if let Some(e) = self.through_outer_accessor(c) {
            return e;
        }
        let mut through_anon = false;
        let mut depth = 0;
        for frame in self.env.frames.iter().rev() {
            if let Frame::Class(k) = frame {
                if *k == c {
                    break;
                }
                through_anon |= self.syms.class(*k).owner == Owner::Local || self.outer_class(*k).is_some();
                depth += 1;
            }
        }
        if through_anon {
            let sym = self.outer_this_sym(c);
            return self.prog.add(TExpr::Local(sym));
        }
        // `O.this` from a class nested in the object `O` is the module itself (a jar's
        // `Chunk.this.Tags` in `Chunk.Singleton`); a class lifted out of the object's own body
        // captured it above.
        if depth > 0 && self.syms.class(c).kind == ClassKind::Object && self.syms.class(c).local_module.is_none() {
            return self.prog.add(TExpr::Module(c));
        }
        self.prog.add(TExpr::This)
    }

    /// The enclosing class whose `this` holds a member declared by `c`: the innermost one that
    /// is `c` or derives from it (a trait's member named by symbol from the object that mixes
    /// the trait in), else `c` itself.
    pub fn holder_of(&mut self, c: ClassId) -> ClassId {
        let frames: Vec<ClassId> = self.env.frames.iter().rev().filter_map(|f| match f {
            Frame::Class(k) => Some(*k),
            _ => None,
        }).collect();
        for k in frames {
            if k == c || self.derives_from(k, c) {
                return k;
            }
        }
        c
    }

    /// The class that a class nested in a class holds the instance of: its constructor takes
    /// that instance first, as a captured `this`. A given's class of a class or trait instance
    /// is one, as scalac's `ExplicitOuter` gives it its outer.
    pub(super) fn outer_class(&self, c: ClassId) -> Option<ClassId> {
        let info = self.syms.class(c);
        let Owner::Class(o) = info.owner else { return None };
        // A Java class's static member class is static: it takes no outer (`cls.isStatic`).
        (matches!(info.kind, ClassKind::Class | ClassKind::GivenImpl) && info.mods & mods::JAVA_STATIC == 0 && self.syms.class(o).kind != ClassKind::Object).then_some(o)
    }

    /// `C.this` from inside a class or trait nested in `C` whose instance does not hold it in a
    /// captured local: the inner one's outer accessor.
    #[cold]
    #[inline(never)]
    fn through_outer_accessor(&mut self, c: ClassId) -> Option<TExprId> {
        let inner = self.frame_inside(c)?;
        if self.enclosing_instance_class(inner) != Some(c) || self.only_local_frames_inside(inner) {
            return None;
        }
        let recv = self.this_ref(inner);
        let accessor = self.outer_accessor(inner);
        Some(self.prog.add(TExpr::CallMethod(recv, accessor, crate::ast::ListRef::EMPTY)))
    }

    /// The class frame directly inside the frame of `c`, when `c` encloses the code typed.
    fn frame_inside(&self, c: ClassId) -> Option<ClassId> {
        let mut inner = None;
        for frame in self.env.frames.iter().rev() {
            if let Frame::Class(k) = frame {
                if *k == c {
                    return inner;
                }
                inner = Some(*k);
            }
        }
        None
    }

    /// Whether the code typed stands in `c` itself or in local and anonymous classes of its
    /// body only, which capture the outer instance `c` holds; a trait holds none.
    fn only_local_frames_inside(&self, c: ClassId) -> bool {
        if self.syms.class(c).kind == ClassKind::Trait {
            return false;
        }
        for frame in self.env.frames.iter().rev() {
            if let Frame::Class(k) = frame {
                if *k == c {
                    return true;
                }
                if self.syms.class(*k).owner != Owner::Local {
                    return false;
                }
            }
        }
        true
    }

    /// The class whose instance encloses a class or trait nested in a class, as scalac's outer
    /// accessor returns it.
    pub(super) fn enclosing_instance_class(&self, c: ClassId) -> Option<ClassId> {
        let info = self.syms.class(c);
        let Owner::Class(o) = info.owner else { return None };
        let nested = matches!(info.kind, ClassKind::Class | ClassKind::Trait | ClassKind::GivenImpl) && info.mods & mods::JAVA_STATIC == 0 && self.syms.class(o).kind != ClassKind::Object;
        nested.then_some(o)
    }

    /// The member of `c` returning its enclosing instance: a class defines it from the outer
    /// instance it captured, the classes mixing in a trait define the trait's.
    #[cold]
    #[inline(never)]
    pub(super) fn outer_accessor(&mut self, c: ClassId) -> SymId {
        if self.shared_class(c) {
            self.with_loader(|w| w.outer_accessor_unlocked(c))
        } else {
            self.outer_accessor_unlocked(c)
        }
    }

    pub(super) fn outer_accessor_unlocked(&mut self, c: ClassId) -> SymId {
        if let Some(&s) = self.outer_accessors.get(&c) {
            return s;
        }
        let o = self.enclosing_instance_class(c).expect("an inner class");
        let (file, owner_name, name) = {
            let info = self.syms.class(c);
            (info.file, self.syms.class(o).name, info.name)
        };
        // A given's class has scalac's, as its class file is scalac's layout (`jvm::static_given_objects`).
        let name = if self.is_library_class(c) && !self.is_product_class(c) || self.syms.class(c).kind == ClassKind::GivenImpl {
            self.scalac_outer_accessor_name(c)
        } else {
            format!("$outer${}${}", self.name_str(owner_name), self.name_str(name))
        };
        let name = self.interner.intern(&name);
        let ret = self.syms.this_type(o);
        let s = self.syms.new_sym(name, SymKind::Def, mods::ABSTRACT, Owner::Class(c), file, None, Span::default());
        let mut info = self.syms.sym_mut(s);
        info.sig = Some(Arc::new(MethodSig::value(ret)));
        info.state().set(Completion::Done);
        self.syms.add_member(c, s);
        self.outer_accessors.insert(c, s);
        s
    }

    /// scalac's name for the outer accessor of a jar's class, which its class file declares and
    /// its methods call: `p$Outer$Inner$$$outer`, the qualified name with `$` between the parts
    /// and an object's own `$` left out.
    fn scalac_outer_accessor_name(&self, c: ClassId) -> String {
        let name = crate::tasty::write::scala_name(self, c).replace("$.", ".").replace('.', "$");
        // A given object's class is a module class, `T$given_G$`, as an inner object's.
        let info = self.syms.class(c);
        let module = info.kind == ClassKind::GivenImpl
            && info.def.map_or(false, |d| matches!(&self.ast(info.file).def(d).kind, DefKind::Given(g) if g.tparams.is_empty() && g.clauses.is_empty()));
        format!("{}{}$$$outer", name, if module { "$" } else { "" })
    }

    /// The outer accessors that the class `c` defines: its own, and those of the traits nested
    /// in a class that it mixes in, each returning the enclosing instance of `c` that derives
    /// from the trait's outer class.
    pub(super) fn define_outer_accessors(&mut self, c: ClassId) -> Vec<FunId> {
        if self.shared_class(c) {
            // Whether there is one to define is read from the records of `c`'s bases, which its
            // completion fixed: the lock is taken only for one.
            if !self.has_outer_accessor_bases(c) {
                return Vec::new();
            }
            self.with_loader(|w| w.define_outer_accessors_unlocked(c))
        } else {
            self.define_outer_accessors_unlocked(c)
        }
    }

    /// Whether `c` or a trait it mixes in is nested in a class: the bases an outer accessor is
    /// defined for (`define_outer_accessors_unlocked`).
    fn has_outer_accessor_bases(&self, c: ClassId) -> bool {
        self.syms.class(c).base_types.iter().any(|&(b, _)| (b == c || self.syms.class(b).kind == ClassKind::Trait) && self.enclosing_instance_class(b).is_some())
    }

    pub(super) fn define_outer_accessors_unlocked(&mut self, c: ClassId) -> Vec<FunId> {
        let mut funs = Vec::new();
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        for b in bases {
            if b != c && self.syms.class(b).kind != ClassKind::Trait {
                continue;
            }
            let Some(o) = self.enclosing_instance_class(b) else { continue };
            if b != c && self.superclass_mixes_in(c, b) {
                continue;
            }
            // A pure interface takes no outer (`ExplicitOuter.needsOuterIfReferenced`): no
            // body of it reads one, and no outer test asks for one (`outer_tested`).
            if b != c && self.pure_interface(b) {
                continue;
            }
            let path = if b == c { None } else { self.parent_path_outer(c, b) };
            let Some(body) = path.or_else(|| self.outer_instance_for(c, o)) else { continue };
            let declared = self.outer_accessor(b);
            let s = if b == c {
                self.syms.sym_mut(declared).mods = mods::FINAL;
                declared
            } else {
                let (name, file, sig) = {
                    let info = self.syms.sym(declared);
                    (info.name, self.syms.class(c).file, info.sig.clone())
                };
                let s = self.syms.new_sym(name, SymKind::Def, mods::FINAL, Owner::Class(c), file, None, Span::default());
                let mut info = self.syms.sym_mut(s);
                info.sig = sig;
                info.state().set(Completion::Done);
                self.syms.add_member(c, s);
                s
            };
            let f = self.prog.add_fun(TFun { sym: s, params: Vec::new(), defaults: Vec::new(), body: Some(body) });
            self.fun_of_sym.insert(s, f);
            funs.push(f);
        }
        funs
    }

    /// The instance of `o` that encloses the class `c` being defined: the nearest enclosing
    /// instance deriving from it, or, for an object nested in a class (`Existential.Bounded`
    /// mixed into `Existentials.Impl`), that object read from the enclosing instance holding it.
    fn outer_instance_for(&mut self, c: ClassId, o: ClassId) -> Option<TExprId> {
        let in_frames = |t: &Self, k: ClassId| k != c && t.env.frames.iter().any(|f| matches!(f, Frame::Class(x) if *x == k));
        let holder = self.holder_of(o);
        if in_frames(self, holder) {
            return Some(self.this_ref(holder));
        }
        if holder == c {
            return None;
        }
        let val = self.syms.class(o).inner_object?;
        let Owner::Class(k) = self.syms.class(o).owner else { return None };
        let object_holder = self.holder_of(k);
        if !in_frames(self, object_holder) {
            return None;
        }
        let recv = self.this_ref(object_holder);
        Some(self.prog.add(TExpr::Field(recv, val)))
    }

    /// The enclosing instance the parents of `c` give the trait `b` nested in a class: the
    /// prefix of `c`'s base type of `b` (`object Z extends C.D`, `class I extends Test.a.Inner`,
    /// `new b.Inner {}`, an alias of one), its value read from inside the class, as dotty's
    /// `ExplicitOuter.outerPrefix` reads the parent type's prefix. None where the base type is
    /// the bare trait or the trait through an enclosing class's `this`, whose enclosing instance
    /// is the one in scope (`outer_instance_for`).
    fn parent_path_outer(&mut self, c: ClassId, b: ClassId) -> Option<TExprId> {
        let base = self.syms.class(c).base_types.iter().find(|&&(k, _)| k == b).map(|&(_, t)| t)?;
        let Type::Nested(p, _) = self.types.get(base) else { return None };
        if matches!(self.types.get(p), Type::This(_)) {
            return None;
        }
        self.prefix_value(p)
    }

    /// Whether the superclass of `c`, a class teq checks, mixes in the trait `b`, whose outer
    /// accessor it then defines for `c` as well.
    fn superclass_mixes_in(&self, c: ClassId, b: ClassId) -> bool {
        self.syms.class(c).superclass.is_some_and(|s| !self.is_library_class(s) && self.syms.class(s).base_types.iter().any(|&(x, _)| x == b))
    }

    /// The local that stands for `C.this` in the classes nested in `c`: one per class, a cell
    /// when the class is every worker's, since the class's check and
    /// the bodies that name the enclosing instance may run on different workers.
    pub(super) fn outer_this_sym(&mut self, c: ClassId) -> SymId {
        if let Some((o, p)) = self.ctor_outer.filter(|&(o, _)| o == c) {
            debug_assert_eq!(o, c);
            return p;
        }
        if let Some(&s) = self.outer_this.get(&c) {
            return s;
        }
        if self.shared_class(c) {
            return self.with_loader(|w| w.outer_this_sym_unlocked(c));
        }
        self.outer_this_sym_unlocked(c)
    }

    fn outer_this_sym_unlocked(&mut self, c: ClassId) -> SymId {
        if let Some(&s) = self.outer_this.get(&c) {
            return s;
        }
        let (span, file, def) = {
            let info = self.syms.class(c);
            (info.span, info.file, info.def)
        };
        // The local stands for `C.this`, so its type is that singleton: a member type reached
        // through it (`$this.Self`) is the one reached through `C.this`.
        let ty = self.types.mk(Type::This(c));
        // By the class's position, since its id depends on what was typed before it: on the
        // std files entered, and between a fresh build and a rebuild in watch mode; a product's
        // class by its place in its source.
        // A product's class not converted yet (a downstream's `new o.Inner` names it first) by
        // its definition's place in the product, where its conversion puts it.
        let place = self.product_position(file, def).or_else(|| self.unconverted_product_place(c));
        let position = match place {
            Some((token, offset)) => format!("{}_{}", crate::source::tag_text(token), offset),
            None => self.prog.position(file, span.start),
        };
        let name = format!("$this{}", position);
        let name = self.interner.intern(&name);
        let s = self.new_local(name, SymKind::Val, ty, span);
        self.outer_this.insert(c, s);
        s
    }
}

/// How a class takes part in `capture_through_nested`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    /// The lifted class whose captures the others take.
    Root,
    /// A class nested in it: they follow its outer instance.
    Nested,
    /// An anonymous or local class inside it: they follow what it captures.
    Lifted { anon: bool },
}
