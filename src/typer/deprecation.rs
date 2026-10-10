//! The deprecation warning of a reference to a `@deprecated` definition, dotty's
//! `CrossVersionChecks.checkDeprecatedRef`: `method old is deprecated since 1: old`, at the
//! reference, a `DeprecationWarning` whose origin is the definition's full name, which the
//! reporting policy summarizes unless `--deprecation` (`warnings.rs`).
//!
//! A definition is deprecated when its symbol or class carries `mods::DEPRECATED`: set by the
//! parser for a program's definition annotated `@deprecated`, by the loader for a library's whose
//! pickle or class file says so. Its message and version are read where the warning is given,
//! from the annotation's arguments. What is checked is what `CrossVersionChecks` transforms:
//! an identifier or a selection (`transformIdent`, `transformSelect`: the head of what an
//! expression typed to), an instance's class (`transformNew`) and a type written in the source
//! (`transformTypeTree`), each at its place. The warning is skipped where a definition enclosing
//! the reference is deprecated itself, or where an enum's own body names one of its cases
//! (`skipDeprecation`), which for a program's code are the definitions whose ranges hold it. An
//! inline expansion checks nothing: the definition's own check types its body where it stands.

use super::referents::Referent;
use super::Worker;
use crate::ast::{self, mods};
use crate::source::Span;
use crate::symbols::{ClassKind, Owner, SymKind};
use crate::types::ClassId;
use std::sync::atomic::{AtomicU8, Ordering};

/// What the reference of a typed expression may need checked (`Worker::check_reference`): a
/// definition seen so far is deprecated (`DEPRECATED`), or a build of the process marks the
/// unused definitions (`UNUSED_DEFS`). One byte, which `type_expr` tests alone; the check sees
/// for itself what its own build asks.
static CHECKS: AtomicU8 = AtomicU8::new(0);
const DEPRECATED: u8 = 1;
const UNUSED_DEFS: u8 = 2;

/// A deprecated definition was entered or loaded.
pub fn note_deprecated() {
    if CHECKS.load(Ordering::Relaxed) & DEPRECATED == 0 {
        CHECKS.fetch_or(DEPRECATED, Ordering::Relaxed);
    }
}

/// A build marks the reads of definitions (`unused::DefKinds`).
pub fn note_unused_defs() {
    CHECKS.fetch_or(UNUSED_DEFS, Ordering::Relaxed);
}

/// Whether any definition seen so far is deprecated: until one is, nothing is checked.
#[inline]
pub fn possible() -> bool {
    CHECKS.load(Ordering::Relaxed) & DEPRECATED != 0
}

/// Whether the reference of a typed expression is checked at all.
#[inline]
pub fn references_checked() -> bool {
    CHECKS.load(Ordering::Relaxed) != 0
}

impl<'a> Worker<'a> {
    /// The reference the expression `e` typed as `te` makes: checked for deprecation, and marked
    /// for the unused definitions. Apart from `type_expr`, which tests whether either runs.
    #[inline(never)]
    pub(super) fn check_reference(&mut self, e: ast::ExprId, te: crate::tir::TExprId) {
        if possible() {
            self.check_deprecated_expr(e, te);
        }
        if self.unused.defs_on() {
            self.use_expr(e, te);
        }
    }

    /// The reference the expression `e` typed as `te` makes, where its head is a name: checked.
    #[inline(always)]
    pub(super) fn check_deprecated_expr(&mut self, e: ast::ExprId, te: crate::tir::TExprId) {
        if !self.may_name_deprecated(te) || self.inline.depth > 0 {
            return;
        }
        self.check_deprecated_reference(e, te);
    }

    #[inline(never)]
    fn check_deprecated_reference(&mut self, e: ast::ExprId, te: crate::tir::TExprId) {
        let Some(r) = self.head_reference(e, te) else { return };
        for referent in r.referents.into_iter().flatten() {
            self.check_deprecated(referent, r.span);
        }
    }

    /// Whether a definition the typed node `te` names at its head, or the value it calls a
    /// member on, is deprecated: what `head_reference` would find, over-approximated from the
    /// node alone, so that the reference is worked out only where a warning may follow.
    #[inline(always)]
    fn may_name_deprecated(&self, te: crate::tir::TExprId) -> bool {
        use crate::tir::TExpr;
        let (mut id, mut node) = (te, self.prog.expr(te));
        loop {
            match node {
                TExpr::Block(_, res) => (id, node) = (res, self.prog.expr(res)),
                // The target of an assignment, a method's eta-expansion: what they call or name.
                TExpr::Assign(l, _) | TExpr::Lambda(_, l) => {
                    node = self.prog.expr(l);
                    while let TExpr::Block(_, res) = node {
                        node = self.prog.expr(res);
                    }
                    return self.names_deprecated(node);
                }
                // A constant member folded to its value.
                TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) => {
                    return self.folded_deprecated && self.folded_paths.get(&id).is_some_and(|&(_, s)| self.syms.sym(s).info.mods & mods::DEPRECATED != 0);
                }
                _ => return self.names_deprecated(node),
            }
        }
    }

    /// Whether the node names a deprecated definition, or calls a member on a value that is.
    #[inline(always)]
    fn names_deprecated(&self, node: crate::tir::TExpr) -> bool {
        use crate::tir::TExpr;
        let sym = |s: crate::types::SymId| self.syms.sym(s).info.mods & mods::DEPRECATED != 0;
        let class = |c: ClassId| self.syms.class(c).mods & mods::DEPRECATED != 0;
        match node {
            TExpr::CallMethod(recv, s, _) => {
                sym(s)
                    || match self.prog.expr(recv) {
                        TExpr::Local(v) | TExpr::Static(v) | TExpr::Field(_, v) => sym(v),
                        TExpr::Module(c) => class(c),
                        _ => false,
                    }
            }
            TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) | TExpr::CallStatic(s, _) => sym(s),
            TExpr::CallClosure(recv, _) => match self.prog.expr(recv) {
                TExpr::Local(v) | TExpr::Static(v) | TExpr::Field(_, v) => sym(v),
                TExpr::Module(c) => class(c),
                _ => false,
            },
            TExpr::Module(c) | TExpr::New(c, _) => class(c),
            TExpr::NewVia(s, _) => sym(s) || matches!(self.syms.sym(s).owner, Owner::Class(c) if class(c)),
            _ => false,
        }
    }

    /// The type the source writes at `span`, resolved to the class `c`.
    #[inline(always)]
    pub(super) fn check_deprecated_class(&mut self, c: ClassId, span: Span) {
        if self.syms.class(c).mods & mods::DEPRECATED != 0 && self.inline.depth == 0 {
            self.check_deprecated(Referent::Class(c), span);
        }
    }

    /// The warnings of the definition `d` of `file` itself: an implicit conversion's feature, its
    /// annotations' deprecation. One call per definition, apart from the paths that type it,
    /// which a definition neither implicit nor annotated leaves at once.
    #[inline(never)]
    pub(super) fn check_def_warnings(&mut self, file: crate::source::FileId, d: ast::DefId) {
        let ast = self.ast(file);
        let implicit = ast.def(d).mods & mods::IMPLICIT != 0;
        let annotated = ast.def_has_annotations(d);
        if implicit {
            self.check_conversion_def(file, d);
        }
        if annotated {
            self.check_deprecated_annotations(file, d);
        }
    }

    /// The annotations of the definition `d` of `file`, of its parameters and of its type
    /// parameters, each whose class is deprecated warned of at the annotation
    /// (`checkDeprecatedAnnots` of `transformValDef`, `transformDefDef`, `transformTypeDef`): the
    /// annotation's resolved class (`annot.symbol`), through an alias, loaded from its library
    /// where it is one, so that no global test of what was loaded before excludes it. The
    /// program's definitions alone: `CrossVersionChecks` transforms the compilation units,
    /// never a library's (the std's) definitions.
    #[inline]
    pub(super) fn check_deprecated_annotations(&mut self, file: crate::source::FileId, d: ast::DefId) {
        if self.inline.depth == 0 && self.ast(file).def_has_annotations(d) && self.program_source(file) {
            self.check_deprecated_annotations_of(file, d);
        }
    }

    #[cold]
    #[inline(never)]
    fn check_deprecated_annotations_of(&mut self, file: crate::source::FileId, d: ast::DefId) {
        let ast = self.ast(file);
        let def = ast.def(d);
        let (tparams, clauses): (&[ast::TypeParam], &[ast::ParamClause]) = match &def.kind {
            ast::DefKind::Fun(f) => (&f.tparams, &f.clauses),
            ast::DefKind::Class(c) => (&c.tparams, &c.clauses),
            ast::DefKind::Given(g) => (&g.tparams, &g.clauses),
            ast::DefKind::TypeAlias { tparams, .. } => (tparams, &[]),
            ast::DefKind::Val { .. } => (&[], &[]),
        };
        let annots: Vec<ast::ExprId> = def
            .annots
            .iter()
            .chain(clauses.iter().flat_map(|c| c.params.iter()).flat_map(|p| ast.param_annots(p).iter()))
            .chain(tparams.iter().flat_map(|tp| tp.annots.iter()))
            .map(|a| a.instance)
            .collect();
        for instance in annots {
            let ast::Expr::New(mut ty, _) = ast.expr(instance) else { continue };
            if let ast::TyExpr::Apply(f, _) = ast.ty(ty) {
                ty = f;
            }
            // The annotation's type resolved where the definition stands and dealiased: its
            // class. What resolving it reports at the annotation is the annotation's typing's,
            // not this check's; what the resolution does elsewhere on the way (the file's imports
            // resolved first) keeps its diagnostics.
            let mark = self.diags.items.len();
            let resolved = self.resolve_type_ctor(ty);
            let resolved = self.dealias(resolved);
            let span = ast.expr_span(instance);
            let own = |d: &crate::source::Diagnostic| d.file == file && d.span.start >= span.start && d.span.end <= span.end;
            let elsewhere: Vec<crate::source::Diagnostic> = self.diags.items.drain(mark..).filter(|d| !own(d)).collect();
            self.diags.items.extend(elsewhere);
            let class = match self.types.get(resolved) {
                crate::types::Type::Class(c, _) | crate::types::Type::Ctor(c) => c,
                crate::types::Type::Lambda(_, rhs) => {
                    let rhs = self.dealias(rhs);
                    match self.types.get(rhs) {
                        crate::types::Type::Class(c, _) => c,
                        _ => continue,
                    }
                }
                _ => continue,
            };
            self.check_deprecated_class(class, ast.expr_span(instance));
        }
    }

    /// A deprecated member `d` of `file` that overrides a concrete member none deprecated
    /// (`checkDeprecatedOvers`): its deprecation is meaningless, warned of at the definition.
    #[inline]
    pub(super) fn check_deprecated_overrides(&mut self, file: crate::source::FileId, d: ast::DefId, class: ClassId) {
        if self.ast(file).def(d).mods & mods::DEPRECATED != 0 && self.inline.depth == 0 && self.program_source(file) {
            self.check_deprecated_overrides_of(file, d, class);
        }
    }

    #[cold]
    #[inline(never)]
    fn check_deprecated_overrides_of(&mut self, file: crate::source::FileId, d: ast::DefId, class: ClassId) {
        let Some(&m) = self.def_syms.get(file.0 as usize, &d) else { return };
        if !matches!(self.syms.sym(m).kind, SymKind::Def | SymKind::Val | SymKind::Var) {
            return;
        }
        self.complete_class(class);
        let bases: Vec<ClassId> = self.syms.class(class).base_types.iter().map(|&(b, _)| b).filter(|&b| b != class).collect();
        let mut overridden: Vec<String> = Vec::new();
        for b in bases {
            self.complete_class(b);
            if let Some(s) = self.concrete_super_member(b, m) {
                if self.syms.sym(s).mods & mods::DEPRECATED == 0 {
                    overridden.push(self.name_str(self.syms.sym(s).name));
                }
            }
        }
        if overridden.is_empty() {
            return;
        }
        let (shown, _) = self.shown(Referent::Sym(m));
        let span = self.ast(file).def_range(d);
        let w = crate::source::Warning { id: crate::warnings::NO_ID, category: crate::warnings::Category::Deprecation, origin: Some("".into()), phase: 0, unplaced: false };
        let file_was = std::mem::replace(&mut self.env.file, file);
        self.warn_as(span, format!("{} overrides concrete, non-deprecated definition(s):\n    {}", shown, overridden.join(", ")), w);
        self.env.file = file_was;
    }

    /// The type alias `a` the source writes at `span`, before it is dealiased
    /// (`transformTypeTree`'s `TypeRef` of the alias).
    #[inline(always)]
    pub(super) fn check_deprecated_alias(&mut self, a: crate::types::AliasId, span: Span) {
        if self.syms.alias(a).deprecated && self.inline.depth == 0 {
            self.check_deprecated(Referent::Alias(a), span);
        }
    }

    fn check_deprecated(&mut self, r: Referent, span: Span) {
        let deprecated = match r {
            Referent::Sym(s) => self.syms.sym(s).mods & mods::DEPRECATED != 0,
            Referent::Class(c) => self.syms.class(c).mods & mods::DEPRECATED != 0,
            Referent::Alias(a) => self.syms.alias(a).deprecated,
        };
        if !deprecated || self.deprecation_skipped(r, span) {
            return;
        }
        let (message, since) = self.deprecation_arguments(r);
        let located = self.shown_located(r);
        let since = since.filter(|s| !s.is_empty()).map(|s| format!(" since {}", s)).unwrap_or_default();
        let message = message.filter(|m| !m.is_empty()).map(|m| format!(": {}", m)).unwrap_or_default();
        let origin = self.full_name(r);
        let w = crate::source::Warning { id: crate::warnings::NO_ID, category: crate::warnings::Category::Deprecation, origin: Some(origin.into_boxed_str()), phase: 0, unplaced: false };
        self.warn_as(span, format!("{} is deprecated{}{}", located, since, message), w);
    }

    /// `skipDeprecation`: a definition enclosing the reference is deprecated, or the referent
    /// is an enum's case named in the enum or its companion.
    fn deprecation_skipped(&self, r: Referent, span: Span) -> bool {
        let file = self.env.file;
        let ast = self.ast(file);
        let enum_case_owner = match r {
            Referent::Sym(s) => match (self.syms.sym(s).kind, self.syms.sym(s).owner) {
                (SymKind::EnumValue(_), Owner::Class(o)) => Some(o),
                _ => None,
            },
            Referent::Class(c) => match (self.syms.class(c).kind, self.syms.class(c).owner) {
                (ClassKind::EnumCase, Owner::Class(o)) => Some(o),
                _ => None,
            },
            Referent::Alias(_) => None,
        };
        let enum_ranges: Vec<Span> = enum_case_owner
            .into_iter()
            .flat_map(|o| [Some(o), self.syms.class(o).companion])
            .flatten()
            .filter(|&k| self.syms.class(k).file == file)
            .filter_map(|k| self.syms.class(k).def.map(|d| ast.def_range(d)))
            .collect();
        if enum_ranges.iter().any(|r| r.start <= span.start && span.end <= r.end) {
            return true;
        }
        ast.defs.iter().enumerate().any(|(i, d)| {
            if d.mods & mods::DEPRECATED == 0 {
                return false;
            }
            let range = ast.def_range(ast::DefId(i as u32));
            range.start <= span.start && span.end <= range.end
        })
    }

    /// The message and the version of the referent's `@deprecated`, its first and second
    /// arguments where they are string literals.
    fn deprecation_arguments(&mut self, r: Referent) -> (Option<String>, Option<String>) {
        let (file, def) = match r {
            Referent::Sym(s) => (self.syms.sym(s).file, self.syms.sym(s).def),
            Referent::Class(c) => (self.syms.class(c).file, self.syms.class(c).def),
            Referent::Alias(a) => (self.syms.alias(a).file, self.syms.alias(a).def),
        };
        if let Some(d) = def.filter(|_| (file.0 as usize) < self.files.len() || self.is_body_file(file)) {
            let ast = self.ast(file);
            let Some(a) = ast.def(d).annots.iter().find(|a| a.name == crate::names::DEPRECATED) else { return (None, None) };
            let args = ast.annot_args(a);
            return (args.first().map(|&s| ast.str(s).to_string()), args.get(1).map(|&s| ast.str(s).to_string()));
        }
        match r {
            Referent::Sym(s) => self.loaded_deprecation(Some(s), None, None),
            Referent::Class(c) => self.loaded_deprecation(None, Some(c), None),
            Referent::Alias(a) => self.loaded_deprecation(None, None, Some(a)),
        }
    }

    /// `method m in object Lib`, `class Old`: the referent as `showLocated` shows it, a member
    /// with the class it is a member of, a top-level definition of a named package with it.
    fn shown_located(&self, r: Referent) -> String {
        let (shown, owner) = self.shown(r);
        match owner {
            Owner::Class(c) if !self.is_package_object(c) => format!("{} in {}", shown, self.class_description(c)),
            Owner::Package(p) if p != crate::symbols::ROOT_PKG && self.syms.pkg(p).name != crate::names::EMPTY => format!("{} in package {}", shown, self.name_str(self.syms.pkg(p).name)),
            _ => shown,
        }
    }

    /// The referent as scalac shows a symbol, its kind and name (`method m`, `class Old`), with
    /// its owner.
    fn shown(&self, r: Referent) -> (String, Owner) {
        match r {
            Referent::Sym(s) => {
                let info = self.syms.sym(s);
                let kind = match info.kind {
                    _ if info.name == crate::names::INIT => "constructor",
                    SymKind::Object(_) => "object",
                    SymKind::Var => "variable",
                    SymKind::Given => "given instance",
                    SymKind::Param => "parameter",
                    SymKind::Val | SymKind::EnumValue(_) if info.mods & mods::LAZY != 0 => "lazy value",
                    SymKind::Val | SymKind::EnumValue(_) => "value",
                    _ if info.mods & mods::GIVEN != 0 => "given instance",
                    _ => "method",
                };
                let name = if info.name == crate::names::INIT { match info.owner {
                        Owner::Class(c) => self.name_str(self.syms.class(c).name),
                        _ => String::new(),
                    } } else { self.name_str(info.name) };
                (format!("{} {}", kind, name), info.owner)
            }
            Referent::Class(c) => {
                let info = self.syms.class(c);
                let kind = match info.kind {
                    ClassKind::Trait => "trait",
                    ClassKind::Object => "object",
                    ClassKind::Opaque => "type",
                    _ => "class",
                };
                (format!("{} {}", kind, self.name_str(info.name)), info.owner)
            }
            Referent::Alias(a) => {
                let info = self.syms.alias(a);
                (format!("type {}", self.name_str(info.name)), info.owner)
            }
        }
    }

    /// The referent's full name (`showFullName`): its owners' names and its own, joined by
    /// dots, the empty package and a package object left out.
    fn full_name(&self, r: Referent) -> String {
        let (name, mut owner) = match r {
            Referent::Sym(s) => {
                let info = self.syms.sym(s);
                let name = if info.name == crate::names::INIT { "<init>".to_string() } else { self.name_str(info.name) };
                (name, info.owner)
            }
            Referent::Class(c) => (self.name_str(self.syms.class(c).name), self.syms.class(c).owner),
            Referent::Alias(a) => (self.name_str(self.syms.alias(a).name), self.syms.alias(a).owner),
        };
        let mut parts = vec![name];
        for _ in 0..64 {
            match owner {
                Owner::Class(c) => {
                    if !self.is_package_object(c) {
                        parts.push(self.name_str(self.syms.class(c).name));
                    }
                    owner = self.syms.class(c).owner;
                }
                Owner::Package(p) => {
                    let info = self.syms.pkg(p);
                    if p == crate::symbols::ROOT_PKG || info.name == crate::names::EMPTY {
                        break;
                    }
                    parts.push(self.name_str(info.name));
                    match info.parent {
                        Some(q) => owner = Owner::Package(q),
                        None => break,
                    }
                }
                Owner::Local => break,
            }
        }
        parts.reverse();
        parts.join(".")
    }

    /// Whether `c` is a package object, which a location leaves out.
    fn is_package_object(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        info.kind == ClassKind::Object && (info.name == crate::names::PACKAGE || self.interner.get(info.name).ends_with("$package"))
    }
}


