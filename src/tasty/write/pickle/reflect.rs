//! The reflection API as scalac 3.8.4 pickles it: scala-library declares it in
//! `Quotes#reflectModule`, which a body reaches
//! through a `Quotes` value's `reflect` (`q.reflect.TypeReprMethods.typeSymbol(x)`), its trees and
//! types abstract types of that path (`q.reflect.Symbol`). teq's std declares it as the object
//! `scala.quoted.Reflect`, its objects and classes members of it; the typer keeps the path it
//! dropped (`Capture::receivers`), and `std_reflect.txt` (generated, `tests/tasty/stdshapes`)
//! names scala-library's member for each of the std's.

use super::*;
use crate::tir::{TExpr, TExprId};

/// A member of the std's reflection API as scala-library has it: the accessor of
/// `reflectModule` its owner stands for (`-` for `reflectModule` itself), the class that
/// declares it there, and its signature (`object` for an accessor, `-` for a member of no
/// signature).
pub(super) struct ReflectEntry {
    pub accessor: &'static str,
    pub owner: &'static str,
    pub sig: &'static str,
}

/// The table by the std's key, `owner<TAB>name<TAB>shape`.
fn reflect_table() -> &'static FxMap<String, ReflectEntry> {
    static TABLE: std::sync::OnceLock<FxMap<String, ReflectEntry>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        include_str!("../std_reflect.txt")
            .lines()
            .filter_map(|l| {
                let mut f = l.split('\t');
                let key = format!("{}\t{}\t{}", f.next()?, f.next()?, f.next()?);
                Some((key, ReflectEntry { accessor: f.next()?, owner: f.next()?, sig: f.next()? }))
            })
            .collect()
    })
}

impl<'w, 'a> P<'w, 'a> {
    /// The std's object `scala.quoted.Reflect`, where it is entered: no class of it is in a
    /// program that enters none.
    fn reflect_object(&mut self) -> Option<ClassId> {
        if let Some(found) = self.reflect_object {
            return found;
        }
        let mut p = ROOT_PKG;
        let mut found = None;
        'walk: {
            for seg in ["scala", "quoted"] {
                let Some(n) = self.w.interner.lookup(seg) else { break 'walk };
                let Some(sub) = self.w.syms.entry_at(p, n).and_then(|e| e.pkg) else { break 'walk };
                p = sub;
            }
            let Some(n) = self.w.interner.lookup("Reflect") else { break 'walk };
            found = match self.w.syms.scoped(self.w.interner, p, n).and_then(|e| e.term).map(|s| self.w.syms.sym(s).kind) {
                Some(SymKind::Object(c)) => Some(c),
                _ => None,
            };
        }
        self.reflect_object = Some(found);
        found
    }

    /// Whether `c` is the std's `scala.quoted.Reflect` or a class or object of it.
    pub(super) fn is_reflect_class(&mut self, c: ClassId) -> bool {
        let Some(r) = self.reflect_object() else { return false };
        let mut at = c;
        loop {
            if at == r {
                return true;
            }
            match self.w.syms.class(at).owner {
                Owner::Class(o) => at = o,
                _ => return false,
            }
        }
    }

    /// Whether `s` is the std's `Quotes.reflect`.
    pub(super) fn is_quotes_reflect(&mut self, s: SymId) -> bool {
        let quotes = self.w.quoted_classes().quotes;
        matches!(self.w.syms.sym(s).owner, Owner::Class(c) if Some(c) == quotes) && self.w.interner.get(self.w.syms.sym(s).name) == "reflect"
    }

    /// `q.reflect`: the val of scala-library's `Quotes`, selected by name.
    pub(super) fn quotes_reflect(&mut self, r: TExprId) {
        self.buf.byte(SELECT);
        let n = self.names.simple("reflect");
        self.buf.nat(n as u64);
        self.term(r);
    }

    /// An object of the reflection API on the path the typer reached it through: its accessor
    /// selected on `q.reflect` (`q.reflect.TypeRepr`), or on the object it is nested in
    /// (`q.reflect.Printer.TreeCode`).
    pub(super) fn reflect_module(&mut self, c: ClassId, receiver: Option<TExprId>) {
        // `Reflect` itself is the `reflect` of the `Quotes` value it was read on.
        if Some(c) == self.reflect_object() {
            return match receiver {
                Some(q) => self.quotes_reflect(q),
                None => self.innermost_reflect("reflect"),
            };
        }
        let name = self.name(self.w.syms.class(c).name);
        let owner = match self.w.syms.class(c).owner {
            Owner::Class(o) => self.w.library_class_name(o, ""),
            _ => String::new(),
        };
        if !reflect_table().contains_key(&format!("{}\t{}\tobject", owner, name)) {
            return self.fail(format!("the reflection API's {}, which scala-library has under another shape", name));
        }
        self.buf.byte(SELECT);
        let n = self.names.simple(&name);
        self.buf.nat(n as u64);
        match receiver {
            Some(r) => self.term(r),
            // An object the typer found no path for (an extension's, which an import of
            // `q.reflect.*` makes visible): the innermost `Quotes` parameter's `reflect`.
            None => self.innermost_reflect(&name),
        }
    }

    /// The `Quotes` the reflection API's paths start from: the one the source names where it
    /// does (an import `q.reflect.*` before them in the block, a parameter declared
    /// `t: q.reflect.Term`), else the innermost given `Quotes` parameter, else the innermost
    /// `Quotes` parameter, or the splice's own where it is nearer.
    fn innermost_quotes(&mut self) -> Option<QuotesRef> {
        if let Some(&(q, splices)) = self.reflect_scopes.last() {
            if splices == self.splice_params.len() {
                return Some(q);
            }
        }
        let quotes = self.w.quoted_classes().quotes?;
        let params: Vec<(usize, SymId)> = self.params.iter().enumerate().rev().map(|(i, &(s, _))| (i, s)).collect();
        let candidates: Vec<(usize, SymId)> = params
            .into_iter()
            .filter(|&(_, s)| {
                let t = self.w.sig_of(s).ret;
                matches!(self.w.types.get(t), Type::Class(k, _) if k == quotes)
            })
            .collect();
        let param = candidates.iter().find(|(_, s)| self.using_params.contains_key(s)).or(candidates.first()).copied();
        // A splice entered inside the method of that parameter is the nearer: its own `Quotes`.
        match (param, self.splice_params.last().copied()) {
            (Some((i, _)), Some((hole, open))) if open > i => Some(QuotesRef::Splice(hole)),
            (None, Some((hole, _))) => Some(QuotesRef::Splice(hole)),
            (Some((_, s)), _) => Some(QuotesRef::Param(s)),
            (None, None) => None,
        }
    }

    /// The `Quotes` parameter the written type of the parameter whose name starts at `start` of `file` selects
    /// `reflect` on (`t: q.reflect.Term`, a method's or a function literal's parameter), among the parameters in
    /// scope: the name before the type tree's `reflect` selection (`Parsers.simpleType1`'s path).
    pub(super) fn declared_quotes(&mut self, file: FileId, start: u32) -> Option<QuotesRef> {
        self.w.files.as_slice().get(file.0 as usize)?;
        if !self.param_types.contains_key(&file) {
            let index = self.index;
            let ast = self.w.ast(file);
            let m = index.param_types.borrow_mut().entry(file).or_insert_with(|| {
                let mut m: FxMap<u32, crate::ast::TyExprId> = FxMap::default();
                for d in &ast.defs {
                    let clauses: &[crate::ast::ParamClause] = match &d.kind {
                        crate::ast::DefKind::Fun(f) => &f.clauses,
                        crate::ast::DefKind::Class(c) => &c.clauses,
                        crate::ast::DefKind::Given(g) => &g.clauses,
                        _ => &[],
                    };
                    for c in clauses {
                        for p in &c.params {
                            m.insert(p.span.start, p.ty);
                        }
                    }
                }
                for p in &ast.lambda_params {
                    if let Some(t) = p.ty {
                        m.insert(p.span.start, t);
                    }
                }
                std::rc::Rc::new(m)
            }).clone();
            self.param_types.insert(file, m);
        }
        let t = *self.param_types[&file].get(&start)?;
        let name = reflect_prefix(self.w.ast(file), self.w.interner, t)?;
        let name = self.w.interner.get(name).to_string();
        self.quotes_named(&name)
    }

    /// The `Quotes` a call on an object of the reflection API takes from its receiver, its first argument (an
    /// extension's, `TreeMethods.asExprOf(t)`): the one a parameter's written type names (`declared_quotes`).
    pub(super) fn receiver_quotes(&mut self, qual: TExprId, args: &[TExprId]) -> Option<QuotesRef> {
        let TExpr::Module(c) = self.w.prog.expr(qual) else { return None };
        if !self.is_reflect_class(c) {
            return None;
        }
        let TExpr::Local(p) = self.w.prog.expr(*args.first()?) else { return None };
        let info = self.w.syms.sym(p);
        let (file, start) = (info.file, info.span.start);
        self.declared_quotes(file, start)
    }

    /// The `Quotes` parameter the import clause starting at `start` of `file` selects `reflect` on (`import
    /// q.reflect.*`): the name before `reflect` in its qualifier, as the parser read it (`Parsers.importExpr`).
    pub(super) fn import_quotes(&mut self, file: FileId, start: u32) -> Option<QuotesRef> {
        self.w.files.as_slice().get(file.0 as usize)?;
        let ast = self.w.ast(file);
        let imp = ast.imports.iter().chain(&ast.local_imports).find(|i| i.tree_start == start)?;
        let name = reflect_qualifier(self.w.interner, &imp.path)?;
        let name = self.w.interner.get(name).to_string();
        self.quotes_named(&name)
    }

    /// The `Quotes` parameter the first import of the reflection API in the definition of `s`
    /// names (`import r.reflect.*`): the first import clause the parser read in its range whose qualifier selects
    /// `reflect` (a block's `Import` tree, Parsers.scala 5024).
    pub(super) fn body_reflect_import(&mut self, s: SymId) -> Option<QuotesRef> {
        let info = self.w.syms.sym(s);
        let (file, def) = (info.file, info.def?);
        self.w.files.as_slice().get(file.0 as usize)?;
        let ast = self.w.ast(file);
        let range = ast.def_range(def);
        let name = ast
            .local_imports
            .iter()
            .filter(|i| i.tree_start >= range.start && i.tree_start < range.end)
            .filter_map(|i| reflect_qualifier(self.w.interner, &i.path).map(|n| (i.tree_start, n)))
            .min_by_key(|&(at, _)| at)?
            .1;
        let name = self.w.interner.get(name).to_string();
        self.quotes_named(&name)
    }

    /// The innermost `Quotes` parameter in scope named `name`.
    fn quotes_named(&mut self, name: &str) -> Option<QuotesRef> {
        let quotes = self.w.quoted_classes().quotes?;
        let params: Vec<SymId> = self.params.iter().rev().map(|&(s, _)| s).collect();
        params
            .into_iter()
            .find(|&s| {
                let t = self.w.sig_of(s).ret;
                self.w.interner.get(self.w.syms.sym(s).name) == name && matches!(self.w.types.get(t), Type::Class(k, _) if k == quotes)
            })
            .map(QuotesRef::Param)
    }

    /// The path of the `Quotes` value `q`.
    fn quotes_path(&mut self, q: QuotesRef) {
        match q {
            QuotesRef::Param(s) => self.local_ref(s),
            QuotesRef::Splice(hole) => {
                self.buf.byte(TERMREFDIRECT);
                self.def_ref(Key::SpliceParam(hole));
            }
        }
    }

    /// The qualifier of an import of the reflection API, which the typer resolved to the std's
    /// object: `q.reflect` of the innermost `Quotes` parameter, an object of it on that.
    pub(super) fn reflect_import(&mut self, c: ClassId) {
        if Some(c) == self.reflect_object() {
            return self.innermost_reflect("import");
        }
        self.reflect_module(c, None)
    }

    /// `q.reflect` of the innermost `Quotes` parameter, for the reflection API's `what`.
    fn innermost_reflect(&mut self, what: &str) {
        let Some(q) = self.innermost_quotes() else { return self.fail(format!("the reflection API's {} outside a method of a Quotes", what)) };
        self.term_at(None);
        self.buf.byte(SELECT);
        let r = self.names.simple("reflect");
        self.buf.nat(r as u64);
        self.term_at(None);
        self.quotes_path(q);
    }

    /// scala-library's member for the std's member `s` of the reflection API, where the table
    /// has one.
    pub(super) fn reflect_entry(&mut self, s: SymId) -> Option<&'static ReflectEntry> {
        let Owner::Class(c) = self.w.syms.sym(s).owner else { return None };
        if !self.is_reflect_class(c) {
            return None;
        }
        let sig = self.member_signature(s).ok()?;
        let (owner, name, shape) = self.std_member_key(s, &sig, None)?;
        reflect_table().get(&format!("{}\t{}\t{}", owner, name, shape))
    }

    /// The selection of a member of the reflection API: scala-library's signature, on the
    /// qualifier `qual` writes (the accessor on `q.reflect`), declared by the class the table
    /// names.
    pub(super) fn reflect_select(&mut self, s: SymId, e: &'static ReflectEntry, qual: impl FnOnce(&mut Self)) {
        let name = self.name(self.w.syms.sym(s).name);
        // A member of a class of teq's std that scala-library declares as an extension:
        // `q.reflect.XMethods.m(receiver)`.
        if let Some(accessor) = e.accessor.strip_prefix("ext:") {
            let Some((params, result)) = super::shapes::sig_of_text(e.sig) else { return self.fail(format!("the reflection API's {}", name)) };
            let a = self.open(APPLY);
            self.term_at(None);
            let l = self.open(SELECTIN);
            let n = self.names.signed(&name, None, &params, &result);
            self.buf.nat(n as u64);
            self.term_at(None);
            self.buf.byte(SELECT);
            let m = self.names.simple(accessor);
            self.buf.nat(m as u64);
            self.innermost_reflect(accessor);
            self.reflect_owner_ref(e.owner);
            self.buf.end_length(l);
            qual(self);
            self.buf.end_length(a);
            return;
        }
        let Some((params, result)) = (e.sig != "-").then(|| super::shapes::sig_of_text(e.sig)).flatten() else {
            self.buf.byte(SELECT);
            let n = self.names.simple(&name);
            self.buf.nat(n as u64);
            return qual(self);
        };
        let l = self.open(SELECTIN);
        let n = self.names.signed(&name, None, &params, &result);
        self.buf.nat(n as u64);
        qual(self);
        self.reflect_owner_ref(e.owner);
        self.buf.end_length(l);
    }

    /// The class that declares a member of the reflection API in scala-library: a trait of
    /// `reflectModule` as scalac's `Quotes.this.reflectModule.this.X`, another class (`TypeTest`)
    /// by its package.
    fn reflect_owner_ref(&mut self, owner: &str) {
        match owner.strip_prefix("scala.quoted.Quotes.reflectModule") {
            Some("") => {
                self.buf.byte(TYPEREF);
                let n = self.names.simple("reflectModule");
                self.buf.nat(n as u64);
                self.buf.byte(THIS);
                self.external_typeref("scala.quoted", "Quotes");
            }
            Some(rest) => {
                self.buf.byte(TYPEREF);
                let n = self.names.simple(rest.trim_start_matches('.'));
                self.buf.nat(n as u64);
                self.buf.byte(THIS);
                self.buf.byte(TYPEREF);
                let m = self.names.simple("reflectModule");
                self.buf.nat(m as u64);
                self.buf.byte(THIS);
                self.external_typeref("scala.quoted", "Quotes");
            }
            None => {
                let (pkg, class) = owner.rsplit_once('.').unwrap_or(("", owner));
                self.external_typeref(pkg, class);
            }
        }
    }

    /// A class of the reflection API as a type: scala-library's abstract type of the innermost
    /// `Quotes` parameter's `reflect` (`q.reflect.Symbol`).
    pub(super) fn reflect_typeref(&mut self, c: ClassId) {
        let name = self.name(self.w.syms.class(c).name);
        let Some(q) = self.innermost_quotes() else { return self.fail(format!("the reflection API's type {} outside a method of a Quotes", name)) };
        self.placed = true;
        self.buf.byte(TYPEREF);
        let n = self.names.simple(&name);
        self.buf.nat(n as u64);
        self.buf.byte(TERMREF);
        let r = self.names.simple("reflect");
        self.buf.nat(r as u64);
        self.quotes_path(q);
    }

    /// Whether `s` is the std's `scala.quoted.quotes`, scala-library's transparent inline
    /// `Quotes$package.quotes(using q): q.type`.
    pub(super) fn is_std_quotes(&mut self, s: SymId) -> bool {
        if !self.is_std_sym(s) || self.w.interner.get(self.w.syms.sym(s).name) != "quotes" {
            return false;
        }
        match self.w.syms.sym(s).owner {
            Owner::Package(p) => self.pkg_path(p) == "scala.quoted",
            Owner::Class(c) => self.w.library_class_name(c, "").starts_with("scala.quoted.") && self.name(self.w.syms.class(c).name).ends_with("$package"),
            Owner::Local => false,
        }
    }

    /// `quotes(using q)` as scalac's expansion pickles it: `q` inside the `INLINED` of
    /// `Quotes$package` (`(inlined(Quotes$package.type) inlined q)`).
    pub(super) fn quotes_call(&mut self, e: TExprId, args: &[TExprId]) {
        let [q] = args else { return self.fail("quotes of no Quotes".to_string()) };
        let l = self.open(INLINED);
        self.term_at(Some(e));
        let inner = self.open(INLINED);
        self.term(*q);
        self.buf.end_length(inner);
        self.mark_tree();
        self.buf.byte(IDENTTPT);
        let n = self.names.object_class("Quotes$package");
        self.buf.nat(n as u64);
        self.buf.byte(TYPEREF);
        self.buf.nat(n as u64);
        self.package_path("scala.quoted");
        self.buf.end_length(l);
    }

    /// The typer's expansion `e` of `quotes(using q)` (the given `q` itself, the body's parameter
    /// reference) as scalac's pickle has it: `q` in the call-less `INLINED` of the parameter's
    /// proxy (`Inliner.inlinedFromOutside`), inside the `INLINED` whose call is scala-library's
    /// `Quotes$package` (`(inlined(Quotes$package.type) inlined q)`).
    pub(super) fn quotes_expansion(&mut self, e: TExprId, rest: &crate::tir::capture::NodeRecords) {
        let l = self.open(INLINED);
        self.term_at(Some(e));
        let inner = self.open(INLINED);
        self.within(Some(e), |p| p.node(e, rest));
        self.buf.end_length(inner);
        self.mark_tree();
        self.buf.byte(IDENTTPT);
        let n = self.names.object_class("Quotes$package");
        self.buf.nat(n as u64);
        self.buf.byte(TYPEREF);
        self.buf.nat(n as u64);
        self.package_path("scala.quoted");
        self.buf.end_length(l);
    }

    /// A type test of a class of the reflection API (`case l: Literal`), an unchecked test of an
    /// abstract type in scala-library: the `TypeTest` given's extractor scalac selects,
    /// `q.reflect.LiteralTypeTest(l)`, against the scrutinee's type.
    pub(super) fn reflect_type_test(&mut self, c: ClassId, t: TypeId, inner: crate::tir::TPatId, scrut: TypeId) {
        let name = format!("{}TypeTest", self.name(self.w.syms.class(c).name));
        let Some(e) = reflect_table().get(&format!("scala.quoted.Reflect$.{}$\tunapply\tscala.quoted.Reflect$.Tree:scala.Option", name))
            .or_else(|| reflect_table().get(&format!("scala.quoted.Reflect$.{}$\tunapply\tscala.quoted.Reflect$.TypeRepr:scala.Option", name))) else {
            return self.fail(format!("the reflection API's {}, which scala-library has under another shape", name));
        };
        let Some((params, result)) = super::shapes::sig_of_text(e.sig) else { return self.fail(format!("the reflection API's {}", name)) };
        let l = self.open(UNAPPLY);
        self.term_at(None);
        let s = self.open(SELECTIN);
        let n = self.names.signed("unapply", None, &params, &result);
        self.buf.nat(n as u64);
        self.term_at(None);
        self.buf.byte(SELECT);
        let m = self.names.simple(&name);
        self.buf.nat(m as u64);
        self.innermost_reflect(&name);
        self.reflect_owner_ref(e.owner);
        self.buf.end_length(s);
        self.ty(scrut);
        self.pattern(inner, t);
        self.buf.end_length(l);
    }
}

/// A `Quotes` value the reflection API's paths start from: a method's parameter, or the
/// parameter of the splice being written, which has no symbol.
#[derive(Clone, Copy)]
pub(super) enum QuotesRef {
    Param(SymId),
    Splice(SymId),
}

/// The name before the first `reflect` a type tree selects, in the order the source writes it (`q.reflect.Term`,
/// `List[q.reflect.Term]`): the prefix's name, an identifier's or a selection's last one.
fn reflect_prefix(ast: &crate::ast::Ast, interner: &crate::intern::Interner, t: crate::ast::TyExprId) -> Option<crate::intern::Name> {
    use crate::ast::TyExpr;
    let is_reflect = |n: crate::intern::Name| interner.get(n) == "reflect";
    match ast.ty(t) {
        TyExpr::Select(q, n) if is_reflect(n) => match ast.ty(q) {
            TyExpr::Name(x) | TyExpr::Select(_, x) => Some(x),
            _ => reflect_prefix(ast, interner, q),
        },
        TyExpr::Select(q, _) | TyExpr::Singleton(q) | TyExpr::ByName(q) | TyExpr::Repeated(q) | TyExpr::Annotated(q, _) => reflect_prefix(ast, interner, q),
        TyExpr::Apply(f, args) => reflect_prefix(ast, interner, f).or_else(|| ast.ty_list(args).iter().find_map(|&a| reflect_prefix(ast, interner, a))),
        TyExpr::Tuple(l) => ast.ty_list(l).iter().find_map(|&a| reflect_prefix(ast, interner, a)),
        TyExpr::Fun(l, r) | TyExpr::CtxFun(l, r) => ast.ty_list(l).iter().find_map(|&a| reflect_prefix(ast, interner, a)).or_else(|| reflect_prefix(ast, interner, r)),
        _ => None,
    }
}

/// The name before `reflect` in an import's qualifier (`import q.reflect.*`, `import q.reflect.{..}`).
fn reflect_qualifier(interner: &crate::intern::Interner, path: &[crate::intern::Name]) -> Option<crate::intern::Name> {
    let i = path.iter().position(|&n| interner.get(n) == "reflect")?;
    i.checked_sub(1).map(|j| path[j])
}
