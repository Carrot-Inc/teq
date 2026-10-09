//! JavaScript interop annotations: teq's `@jsImport` / `@jsExport` and the Scala.js facade
//! annotations (`@js.native`, `@JSImport`, `@JSGlobal`, `@JSGlobalScope`, `@JSName`,
//! `@JSBracketAccess`, `@JSExportTopLevel`), read while symbols are entered, and the calls into
//! native JS members and bindings.

use super::Worker;
use crate::ast::{self, mods, DefId, DefKind, Expr, ExprId, ListRef, TyExpr};
use crate::emit::is_js_identifier;
use crate::intern::Name;
use crate::names;
use crate::source::FileId;
use crate::symbols::*;
use crate::tir::*;
use crate::tir::capture::Form;
use crate::types::{ClassId, SymId};

/// What the interop annotations need to know about the annotated definition.
#[derive(Default)]
struct DefShape {
    is_def_or_val: bool,
    has_body: bool,
    /// The body is `js.native`, which a Scala.js facade writes in place of an implementation.
    native_body: bool,
    has_using: bool,
    has_default: bool,
    /// A default other than `js.native` or `js.undefined`, which a JS callee never sees.
    has_scala_default: bool,
    has_by_name: bool,
    repeated_before_last: bool,
}

fn def_shape(ast: &ast::Ast, def: &ast::Def) -> DefShape {
    match &def.kind {
        DefKind::Val { pat, rhs, .. } => DefShape {
            is_def_or_val: pat.is_none() && def.mods & mods::MUTABLE == 0,
            has_body: rhs.is_some(),
            native_body: rhs.map_or(false, |e| is_js_native(ast, e)),
            ..DefShape::default()
        },
        DefKind::Fun(f) => {
            let params = || f.clauses.iter().flat_map(|c| c.params.iter());
            let count = params().count();
            DefShape {
                is_def_or_val: !f.is_extension,
                has_body: f.body.is_some(),
                native_body: f.body.map_or(false, |e| is_js_native(ast, e)),
                has_using: f.clauses.iter().any(|c| c.is_using)
                    || f.tparams.iter().any(|tp| !tp.context_bounds.is_empty()),
                has_default: params().any(|p| p.default.is_some()),
                has_scala_default: params().any(|p| p.default.map_or(false, |d| !is_js_default(ast, d))),
                has_by_name: params().any(|p| matches!(ast.ty(p.ty), TyExpr::ByName(_))),
                repeated_before_last: params()
                    .take(count.saturating_sub(1))
                    .any(|p| matches!(ast.ty(p.ty), TyExpr::Repeated(_))),
            }
        }
        _ => DefShape::default(),
    }
}

/// The expression behind a body that may be written on the next line.
fn unwrap_block(ast: &ast::Ast, e: ExprId) -> ExprId {
    match ast.expr(e) {
        Expr::Block(stmts) if stmts.len == 1 => match ast.stmt_list(stmts)[0] {
            ast::Stmt::Expr(inner) => inner,
            _ => e,
        },
        Expr::Parens(inner) => inner,
        _ => e,
    }
}

fn is_named(ast: &ast::Ast, e: ExprId, name: Name) -> bool {
    matches!(ast.expr(unwrap_block(ast, e)), Expr::Ident(n) | Expr::Select(_, n) if n == name)
}

/// `js.native`, the body of a facade member.
pub fn is_js_native(ast: &ast::Ast, e: ExprId) -> bool {
    is_named(ast, e, names::NATIVE)
}

/// `js.undefined`, the only value a JS trait may give a val.
pub fn is_js_undefined(ast: &ast::Ast, e: ExprId) -> bool {
    is_named(ast, e, names::UNDEFINED)
}

/// A default that a call into JavaScript leaves out when the argument is omitted.
fn is_js_default(ast: &ast::Ast, e: ExprId) -> bool {
    is_js_native(ast, e) || is_js_undefined(ast, e)
}

/// The Scala.js annotations of one definition.
#[derive(Default)]
pub struct JsAnnots {
    pub native: bool,
    pub import: Option<(Name, Name)>,
    /// `@JSGlobal`, with the name when one is given.
    pub global: Option<Option<Name>>,
    pub global_scope: bool,
    pub js_name: Option<Name>,
    /// `js_name` names a well-known symbol (`js.Symbol.iterator`).
    pub js_symbol: bool,
    pub bracket: bool,
    pub export: Option<Name>,
    /// `@JSType`, which scalac puts on every JS type, native or not.
    pub js_type: bool,
}

impl<'a> Worker<'a> {
    /// Reads the Scala.js annotations of `annots`; the arguments have to be literals or the
    /// constants `JSImport.Default` and `JSImport.Namespace`.
    pub fn js_annots(&mut self, file: FileId, annots: &[ast::Annot], span: crate::source::Span) -> JsAnnots {
        let ast = self.ast(file);
        let mut out = JsAnnots::default();
        for a in annots {
            let strings: Vec<&str> = ast.annot_args(a).iter().map(|&s| ast.str(s)).collect();
            match a.name {
                names::NATIVE => out.native = true,
                names::JS_IMPORT_ANNOT => {
                    let Some((_, values)) = ast.annot_new(a) else { continue };
                    let args = ast.expr_list(values);
                    let mut parts = Vec::with_capacity(2);
                    for &v in args {
                        let part = match ast.expr(v) {
                            Expr::StringLit(s) => Some(self.interner.intern(ast.str(s))),
                            Expr::Ident(names::DEFAULT) | Expr::Select(_, names::DEFAULT) => {
                                Some(self.interner.intern("default"))
                            }
                            Expr::Ident(names::NAMESPACE) | Expr::Select(_, names::NAMESPACE) => Some(names::STAR),
                            _ => None,
                        };
                        parts.push(part);
                    }
                    match parts.as_slice() {
                        [Some(module), Some(name)] if *module != names::EMPTY && *name != names::EMPTY => {
                            out.import = Some((*module, *name));
                        }
                        _ => self.diags.error(
                            file,
                            span,
                            "@JSImport needs a module and a name: @JSImport(\"module\", \"name\"), JSImport.Default or JSImport.Namespace",
                        ),
                    }
                }
                names::JS_GLOBAL => {
                    let Some((_, values)) = ast.annot_new(a) else { continue };
                    match (ast.expr_list(values).len(), strings.first()) {
                        (0, _) => out.global = Some(None),
                        (1, Some(s)) if !s.is_empty() => out.global = Some(Some(self.interner.intern(s))),
                        _ => self.diags.error(file, span, "@JSGlobal takes a name as a string literal, or nothing"),
                    }
                }
                names::JS_GLOBAL_SCOPE => out.global_scope = true,
                names::JS_NAME => match strings.as_slice() {
                    [s] if !s.is_empty() => out.js_name = Some(self.interner.intern(s)),
                    _ => self.diags.error(file, span, "@JSName needs the name as a string literal: @JSName(\"name\")"),
                },
                names::JS_BRACKET_ACCESS => out.bracket = true,
                names::JS_EXPORT_TOP_LEVEL => match strings.as_slice() {
                    [s] if is_js_identifier(s) => out.export = Some(self.interner.intern(s)),
                    _ => self.diags.error(file, span, "@JSExportTopLevel needs a valid JS identifier: @JSExportTopLevel(\"name\")"),
                },
                _ => {}
            }
        }
        out
    }

    /// Numbers the imports the bodies registered by their module and name, after the ones the
    /// signature phase registered: the bodies register an import where one of them first meets
    /// its definition (a library class completed on the way), which is the workers' order with
    /// several workers, and the numbers name the imports' bindings in the output.
    pub(super) fn settle_js_imports(&mut self) {
        let from = self.js_imports_at_bodies.min(self.prog.js_imports.len());
        let n = self.prog.js_imports.len();
        if n - from < 2 {
            return;
        }
        let mut order: Vec<u32> = (from as u32..n as u32).collect();
        order.sort_by(|&a, &b| {
            let (x, y) = (self.prog.js_imports[a as usize], self.prog.js_imports[b as usize]);
            (self.interner.get(x.module), self.interner.get(x.name)).cmp(&(self.interner.get(y.module), self.interner.get(y.name)))
        });
        if order.iter().enumerate().all(|(k, &i)| i as usize == from + k) {
            return;
        }
        let mut new_of: Vec<u32> = (0..n as u32).collect();
        for (k, &i) in order.iter().enumerate() {
            new_of[i as usize] = (from + k) as u32;
        }
        let imports: Vec<JsImport> = order.iter().map(|&i| self.prog.js_imports[i as usize]).collect();
        self.prog.js_imports.truncate(from);
        self.prog.js_imports.extend(imports);
        self.js_import_ids = self.prog.js_imports.iter().enumerate().map(|(i, &imp)| (imp, i as u32)).collect();
        let remap = |i: &mut u32| *i = new_of[*i as usize];
        for c in self.syms.classes.own_mut() {
            if let Some(JsBinding::Import(i)) = &mut c.js_binding {
                remap(i);
            }
        }
        for s in self.syms.syms.own_mut() {
            if let Some(i) = &mut s.js_import {
                remap(i);
            }
        }
        for e in self.prog.exprs.own_mut() {
            if let TExpr::JsImport(i) = e {
                remap(i);
            }
        }
    }

    pub(super) fn register_js_import(&mut self, module: Name, name: Name) -> u32 {
        let import = JsImport { module, name };
        if let Some(r) = &self.js_registry {
            return r.register(import);
        }
        let next = self.prog.js_imports.len() as u32;
        let id = *self.js_import_ids.entry(import).or_insert(next);
        if id == next {
            self.prog.js_imports.push(import);
        }
        id
    }

    fn register_js_export(&mut self, sym: SymId, name: Name, span: crate::source::Span) {
        if let Some(&(other, _)) = self.prog.js_exports.iter().find(|&&(_, n)| n == name) {
            let mut msg = format!("{} is exported twice", self.name_str(name));
            if self.syms.sym(other).name == self.syms.sym(sym).name && self.syms.sym(other).owner == self.syms.sym(sym).owner {
                msg.push_str(": JavaScript has one function of that name and nothing chooses between overloaded alternatives at run time, so each one needs a name of its own");
            }
            self.error(span, msg);
        }
        self.prog.js_exports.push((sym, name));
        self.js_exported.store(true, std::sync::atomic::Ordering::Release);
    }

    /// `sym` is the symbol of a def or val; every other kind of definition passes `None`.
    pub fn enter_interop_annots(&mut self, file: FileId, owner: Owner, def: &'a ast::Def, sym: Option<SymId>) {
        for a in &def.annots {
            let is_import = a.name == names::JS_IMPORT;
            if !is_import && a.name != names::JS_EXPORT {
                continue;
            }
            let label = if is_import { "@jsImport" } else { "@jsExport" };
            let ast = self.ast(file);
            let shape = def_shape(ast, def);
            let args: Vec<&'a str> = ast.annot_args(a).iter().map(|&s| ast.str(s)).collect();
            let first_import = def.annots.iter().find(|b| b.name == names::JS_IMPORT).unwrap_or(a);
            let also_imported = first_import.name == names::JS_IMPORT;
            let problem = if !matches!(owner, Owner::Package(_)) {
                Some(format!("{} is only allowed on top-level definitions", label))
            } else if !shape.is_def_or_val || sym.is_none() {
                Some(format!("{} is only allowed on a def or a val", label))
            } else if is_import && (args.len() != 2 || args.iter().any(|s| s.is_empty())) {
                Some("@jsImport needs a module and a name: @jsImport(\"module\", \"name\")".to_string())
            } else if !is_import && args.len() != 1 {
                Some("@jsExport needs the exported name: @jsExport(\"name\")".to_string())
            } else if shape.has_using {
                Some(format!("a {} definition cannot have using clauses", label))
            } else if is_import && shape.has_body {
                Some("a @jsImport definition cannot have a body".to_string())
            } else if is_import && shape.has_default {
                Some("default arguments are not supported on @jsImport definitions".to_string())
            } else if is_import && shape.has_by_name {
                Some("by-name parameters are not supported on @jsImport definitions".to_string())
            } else if is_import && def.annots.iter().any(|b| b.name == names::JS) {
                Some("@js and @jsImport cannot be combined".to_string())
            } else if is_import && !std::ptr::eq(a, first_import) {
                Some("a definition can only have one @jsImport".to_string())
            } else if !is_import && !is_js_identifier(args[0]) {
                Some(format!("\"{}\" is not a valid export name", args[0]))
            } else if !is_import && !shape.has_body && !also_imported {
                Some("a @jsExport definition needs a body".to_string())
            } else if !is_import && shape.repeated_before_last {
                Some("the repeated parameter of a @jsExport def has to come last".to_string())
            } else {
                None
            };
            if let Some(msg) = problem {
                self.error(def.span, msg);
                continue;
            }
            let Some(sym) = sym else { continue };
            if is_import {
                let (module, name) = (self.interner.intern(args[0]), self.interner.intern(args[1]));
                let id = self.register_js_import(module, name);
                self.syms.sym_mut(sym).js_import = Some(id);
            } else {
                let name = self.interner.intern(args[0]);
                self.register_js_export(sym, name, def.span);
            }
        }
        if let Some(sym) = sym {
            self.enter_scalajs_member_annots(file, owner, def, sym);
        }
    }

    /// The Scala.js annotations of a def or val: a binding to an import or a global, a JS name,
    /// bracket access and a top-level export.
    fn enter_scalajs_member_annots(&mut self, file: FileId, owner: Owner, def: &'a ast::Def, sym: SymId) {
        let scalajs = |a: &ast::Annot| {
            matches!(
                a.name,
                names::NATIVE
                    | names::JS_IMPORT_ANNOT
                    | names::JS_GLOBAL
                    | names::JS_GLOBAL_SCOPE
                    | names::JS_NAME
                    | names::JS_BRACKET_ACCESS
                    | names::JS_EXPORT_TOP_LEVEL
            )
        };
        if !def.annots.iter().any(scalajs) {
            return;
        }
        let annots = self.js_annots(file, &def.annots, def.span);
        let shape = def_shape(self.ast(file), def);
        let in_js_class = match owner {
            Owner::Class(c) => self.syms.class(c).js != JsKind::Scala,
            _ => false,
        };
        let bound = annots.import.is_some() || annots.global.is_some();
        let problem = if annots.global_scope {
            Some("@JSGlobalScope is only allowed on an object".to_string())
        } else if annots.import.is_some() && annots.global.is_some() {
            Some("a definition cannot have both @JSImport and @JSGlobal".to_string())
        } else if bound && !shape.is_def_or_val {
            Some("@JSImport and @JSGlobal go on a def, a val, an object or a class".to_string())
        } else if bound && in_js_class {
            Some("a member of a JS type cannot have @JSImport or @JSGlobal".to_string())
        } else if bound && shape.has_body && !shape.native_body {
            Some("the body of a @JSImport or @JSGlobal definition has to be js.native".to_string())
        } else if bound && shape.has_using {
            Some("a @JSImport or @JSGlobal definition cannot have using clauses".to_string())
        } else if bound && shape.has_by_name {
            Some("by-name parameters are not supported on @JSImport or @JSGlobal definitions".to_string())
        } else if bound && shape.has_scala_default {
            Some("a default argument of a native JS method has to be js.native or js.undefined".to_string())
        } else if bound && def.annots.iter().any(|b| b.name == names::JS || b.name == names::JS_IMPORT) {
            Some("@JSImport and @JSGlobal cannot be combined with @js or @jsImport".to_string())
        } else if annots.js_name.is_some() && !in_js_class {
            Some("@JSName is only allowed on a member of a JS type".to_string())
        } else if annots.bracket && !in_js_class {
            Some("@JSBracketAccess is only allowed on a member of a native JS type".to_string())
        } else if annots.bracket && !matches!(def.name, names::APPLY | names::UPDATE) {
            Some("@JSBracketAccess goes on apply (a read) or update (a write)".to_string())
        } else if annots.export.is_some() && !matches!(owner, Owner::Package(_)) && !self.is_object_member(owner) {
            Some("@JSExportTopLevel is only allowed on a top-level definition or a member of an object".to_string())
        } else if annots.export.is_some() && !shape.has_body {
            Some("a @JSExportTopLevel definition needs a body".to_string())
        } else {
            None
        };
        if let Some(msg) = problem {
            self.error(def.span, msg);
            return;
        }
        self.mark_js_member(sym, &annots);
        if let Some(name) = annots.export {
            self.register_js_export(sym, name, def.span);
        }
    }

    /// What the Scala.js annotations of a def or val make of it, read from source or from TASTy:
    /// the binding to an import or a global, the JS name, bracket access.
    pub(super) fn mark_js_member(&mut self, sym: SymId, annots: &JsAnnots) {
        if let Some((module, name)) = annots.import {
            let id = self.register_js_import(module, name);
            self.syms.sym_mut(sym).js_import = Some(id);
        }
        if let Some(global) = annots.global {
            let name = global.unwrap_or(self.syms.sym(sym).name);
            self.syms.sym_mut(sym).js_global = Some(name);
        }
        if let Some(name) = annots.js_name {
            self.syms.sym_mut(sym).js_name = Some(name);
            self.syms.sym_mut(sym).js_symbol = annots.js_symbol;
        }
        if annots.bracket {
            self.syms.sym_mut(sym).js_bracket = true;
        }
    }

    /// Where JavaScript keeps a native class or object: its import, its global (the Scala name
    /// when the annotation gives none) or the global scope.
    pub(super) fn js_class_binding(&mut self, annots: &JsAnnots, name: Name) -> Option<JsBinding> {
        match (annots.import, annots.global, annots.global_scope) {
            (Some((module, name)), _, _) => Some(JsBinding::Import(self.register_js_import(module, name))),
            (None, Some(global), _) => Some(JsBinding::Global(global.unwrap_or(name))),
            (None, None, true) => Some(JsBinding::GlobalScope),
            (None, None, false) => None,
        }
    }

    fn is_object_member(&self, owner: Owner) -> bool {
        matches!(owner, Owner::Class(c) if self.syms.class(c).kind == ClassKind::Object)
    }

    /// `@js.native` makes a class, trait or object a native JS type; `@JSImport`, `@JSGlobal`
    /// and `@JSGlobalScope` say where JavaScript keeps it.
    pub fn enter_class_annots(&mut self, file: FileId, def: &'a ast::Def, c: ClassId) {
        if def.annots.is_empty() {
            return;
        }
        let annots = self.js_annots(file, &def.annots, def.span);
        let kind = self.syms.class(c).kind;
        if let Some(a) = def.annots.iter().find(|a| a.name == names::JS_IMPORT || a.name == names::JS_EXPORT) {
            let label = if a.name == names::JS_IMPORT { "@jsImport" } else { "@jsExport" };
            let msg = format!("{} is only allowed on a def or a val", label);
            self.error(def.span, msg);
        }
        let bindings = [annots.import.is_some(), annots.global.is_some(), annots.global_scope].iter().filter(|&&b| b).count();
        if bindings > 1 {
            self.error(def.span, "a class or object can only have one of @JSImport, @JSGlobal and @JSGlobalScope");
        }
        let binding = self.js_class_binding(&annots, def.name);
        let problem = if annots.js_name.is_some() || annots.bracket || annots.export.is_some() {
            Some("@JSName, @JSBracketAccess and @JSExportTopLevel go on a def or a val")
        } else if binding.is_some() && !annots.native {
            Some("@JSImport, @JSGlobal and @JSGlobalScope need @js.native on the class or object")
        } else if binding.is_some() && kind == ClassKind::Trait {
            Some("a native JS trait has no location; @JSImport, @JSGlobal and @JSGlobalScope go on a class or an object")
        } else if annots.global_scope && kind != ClassKind::Object {
            Some("@JSGlobalScope is only allowed on an object")
        } else if annots.native && !matches!(kind, ClassKind::Class | ClassKind::Trait | ClassKind::Object) {
            Some("@js.native is only allowed on a class, a trait or an object")
        } else {
            None
        };
        if let Some(msg) = problem {
            self.error(def.span, msg);
            return;
        }
        if annots.native {
            let mut info = self.syms.class_mut(c);
            info.js = JsKind::Native;
            info.js_binding = binding;
        }
    }

    /// `@JSName` on a constructor parameter of a JS class.
    pub fn enter_param_annots(&mut self, file: FileId, p: &ast::Param, sym: SymId) {
        if p.annots.is_empty() {
            return;
        }
        let annots: Vec<ast::Annot> = self.ast(file).param_annots(p).to_vec();
        let read = self.js_annots(file, &annots, p.span);
        if let Some(name) = read.js_name {
            self.syms.sym_mut(sym).js_name = Some(name);
        }
    }

    /// Whether the native JS type `c` has the members a body of a native type may declare: no
    /// definitions with an implementation other than `js.native`, no givens, no nested classes.
    pub fn check_native_body(&mut self, c: ClassId) {
        let (file, def, kind) = {
            let i = self.syms.class(c);
            (i.file, i.def, i.kind)
        };
        let Some(def) = def else { return };
        let ast = self.ast(file);
        let DefKind::Class(cls) = &ast.def(def).kind else { return };
        let needs_binding = kind != ClassKind::Trait && self.syms.class(c).js_binding.is_none();
        if needs_binding {
            let msg = format!(
                "the native JS {} {} needs @JSImport, @JSGlobal or @JSGlobalScope to say where JavaScript defines it",
                if kind == ClassKind::Object { "object" } else { "class" },
                self.name_str(self.syms.class(c).name)
            );
            self.diags.error(file, self.syms.class(c).span, msg);
        }
        for stmt in &cls.body {
            let d: DefId = match stmt {
                ast::Stmt::Def(d) => *d,
                ast::Stmt::Expr(e) => {
                    self.diags.error(file, ast.expr_span(*e), "a native JS type has no initialiser statements");
                    continue;
                }
                ast::Stmt::Import(_) => continue,
            };
            let member = ast.def(d);
            let (body, defaults): (Option<ExprId>, Vec<ExprId>) = match &member.kind {
                DefKind::Val { rhs, .. } => (*rhs, Vec::new()),
                DefKind::Fun(f) => {
                    (f.body, f.clauses.iter().flat_map(|c| c.params.iter().filter_map(|p| p.default)).collect())
                }
                DefKind::TypeAlias { .. } => continue,
                DefKind::Class(_) => {
                    let msg = format!(
                        "{} is nested in a native JS type, which cannot define classes or objects",
                        self.name_str(member.name)
                    );
                    self.diags.error(file, member.span, msg);
                    continue;
                }
                DefKind::Given(_) => {
                    self.diags.error(file, member.span, "a native JS type cannot define givens");
                    continue;
                }
            };
            if let Some(b) = body.filter(|&b| !is_js_native(ast, b)) {
                let msg = format!(
                    "{} is a member of a native JS type: it has to be abstract or have the body js.native",
                    self.name_str(member.name)
                );
                self.diags.error(file, ast.expr_span(b), msg);
            }
            for d in defaults {
                if !is_js_default(ast, d) {
                    self.diags.error(
                        file,
                        ast.expr_span(d),
                        "a default argument of a native JS method has to be js.native or js.undefined",
                    );
                }
            }
            if let Some(&sym) = self.def_syms.get(file.0 as usize, &d) {
                self.sig_of(sym);
            }
        }
    }

    /// Whether the body of `sym` is left to JavaScript: a member of a native type or a binding to
    /// an import or a global, whose `js.native` body is dropped.
    pub fn body_is_native(&self, sym: SymId) -> bool {
        let info = self.syms.sym(sym);
        if info.js_import.is_some() || info.js_global.is_some() {
            return true;
        }
        matches!(info.owner, Owner::Class(c) if self.syms.class(c).js == JsKind::Native)
    }

    /// The arguments of a call into JavaScript: repeated ones are spread, and trailing omitted
    /// defaults are not passed, so that the callee sees its own defaults.
    fn js_call_args(&mut self, params: &[ParamSig], args: ListRef) -> ListRef {
        let mut flat = self.prog.expr_list(args).to_vec();
        while let Some((&last, p)) = flat.last().zip(params.get(flat.len().wrapping_sub(1))) {
            if p.has_default && matches!(self.prog.expr(last), TExpr::Unit) {
                flat.pop();
            } else {
                break;
            }
        }
        let mut changed = flat.len() != args.len as usize;
        for (slot, p) in flat.iter_mut().zip(params) {
            if p.repeated {
                *slot = self.prog.add(TExpr::Spread(*slot));
                changed = true;
            }
        }
        if changed { self.prog.list(&flat) } else { args }
    }

    fn flat_params(&mut self, sym: SymId) -> Vec<ParamSig> {
        let sig = self.sig_of(sym);
        sig.clauses.iter().flat_map(|c| c.params.iter().cloned()).collect()
    }

    /// A call of a def bound to an import or a global passes its arguments positionally over all
    /// parameter clauses.
    pub fn build_import_call(&mut self, sym: SymId, callee: TExpr, args: ListRef) -> TExprId {
        let callee = self.prog.add(callee);
        let params = self.flat_params(sym);
        if params.is_empty() && self.sig_of(sym).clauses.is_empty() {
            if self.capturing() {
                self.capture_form(callee, Form::Member(sym));
            }
            return callee;
        }
        let args = self.js_call_args(&params, args);
        let te = self.prog.add(TExpr::CallClosure(callee, args));
        if self.capturing() {
            self.capture_form(te, Form::Member(sym));
        }
        te
    }

    /// The constructor arguments of a native JS class, with the omitted trailing defaults left out.
    pub fn native_ctor_args(&mut self, c: ClassId, args: ListRef) -> ListRef {
        let params: Vec<ParamSig> = self.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().cloned()).collect();
        self.js_call_args(&params, args)
    }

    /// A call of a member of a JS type: a parameterless def is a property read, `apply` and
    /// `@JSBracketAccess` members index the receiver, and every other def is a JS method call.
    pub fn build_js_member_call(&mut self, recv: TExprId, sym: SymId, args: ListRef) -> TExprId {
        if self.sig_of(sym).clauses.is_empty() {
            return self.prog.add(TExpr::Field(recv, sym));
        }
        let Owner::Class(owner) = self.syms.sym(sym).owner else { unreachable!() };
        if self.syms.class(owner).js != JsKind::Native {
            return self.prog.add(TExpr::CallMethod(recv, sym, args));
        }
        let params = self.flat_params(sym);
        let args = self.js_call_args(&params, args);
        let info = self.syms.sym(sym);
        if info.js_bracket {
            let items = self.prog.expr_list(args).to_vec();
            let template = if info.name == names::APPLY { "($0)[$1]" } else { "(($0)[$1] = $2)" };
            let s = self.prog.add_str(template);
            let mut all = vec![recv];
            all.extend(items);
            let l = self.prog.list(&all);
            let te = self.prog.add(TExpr::Js(s, l));
            if self.capturing() {
                self.capture_form(te, Form::Member(sym));
            }
            return te;
        }
        if info.name == names::APPLY && info.js_name.is_none() {
            let te = self.prog.add(TExpr::CallClosure(recv, args));
            if self.capturing() {
                self.capture_form(te, Form::Member(sym));
            }
            return te;
        }
        self.prog.add(TExpr::CallMethod(recv, sym, args))
    }
}
