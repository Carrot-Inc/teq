//! `scala.quoted` over the typed IR: the builtins behind `std/quoted/`, keyed by the qualified
//! name of each member, and the two templates the typer makes for quotes, `$quote` (a quote
//! run: its body copied with the holes filled) and `$quoteMatch` (a quote pattern matched
//! against a tree). An `Expr` and a `quotes.reflect.Term` are one value, `Value::Tree`, seen
//! through two APIs; the tree kinds of the reflect API are views over the IR nodes.

use super::builtins::{reg, Table};
use super::value::*;
use super::*;
use crate::intern::FxMap;
use crate::source::{FileId, Span};
use crate::symbols::{ClassKind, ClauseSig, MethodSig, ParamSig, ROOT_PKG};
use std::sync::Arc;
use crate::tir::*;
use crate::typer::site::SiteOwner;
use crate::typer::{ArgList, ArgSrc, TypeRef};
use crate::types::*;

/// The macro expansion an interpreter runs: what `scala.quoted` reads the call site from.
pub struct MacroCtx {
    /// The innermost inline call, where the trees and symbols the macro makes stand and its
    /// messages are reported, carried to the outermost call with their `inlined from` notes.
    pub site_file: FileId,
    pub site_span: Span,
    /// `Position.ofMacroExpansion`: the outermost inline call, as scalac's `Inliner` positions
    /// the expansion, for a call nested in inline methods and one a macro's quote made alike.
    pub expansion_file: FileId,
    pub expansion_span: Span,
    /// The outermost inline call, in the file being compiled: what names the expansion.
    pub unit_file: FileId,
    pub unit_span: Span,
    /// The number of this expansion in the session (`Interp::next_run`), which the symbols of
    /// its site carry: the owner chain below is this run's and no other's.
    pub run: u32,
    /// The definitions around the call, outermost first, and the class around it.
    pub owners: Vec<SiteOwner>,
    pub owner_class: Option<ClassId>,
    /// The argument each proxy of the inline method's parameters stands for.
    pub params: FxMap<SymId, TExprId>,
    /// The positions of the inline calls under expansion, the outermost first, and the info
    /// messages the run gave so far: what orders its messages among the build's
    /// (`Worker::flush_infos`).
    pub inline_path: Vec<(FileId, u32, u32)>,
    pub infos: std::cell::Cell<u32>,
}

/// A selection or reference that awaits its arguments, which `Apply` completes.
#[derive(Clone)]
pub struct Pending {
    pub recv: Option<TreeRef>,
    pub kind: PendingKind,
    pub targs: Vec<TypeId>,
}

/// The shapes a package takes as a `TypeRepr` (`Interp::pkg_type`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PkgForm {
    /// A `TermRef` over the path of the enclosing packages: the prefix of a class.
    Path = 0,
    /// A `TermRef` over the enclosing package's `ThisType`: `Symbol.termRef`.
    Sym = 1,
    ThisType = 2,
    /// The `TypeRef(NoPrefix, name)` inside a package's `ThisType`.
    ClassRef = 3,
    /// A `TypeRef` over the enclosing package's `ThisType`: `Symbol.typeRef`.
    SymClassRef = 4,
}

#[derive(Clone)]
pub enum PendingKind {
    /// A method resolved to its symbol.
    Sym(SymId),
    /// A member by name, resolved by the typer when the arguments are known.
    Named(Name),
    /// A builtin operator of the typed program (`1 + 1`) by name, applied as `Named` is, with
    /// the method type of the alternative the typer chose: what `tpe` of scalac's `Select`
    /// gives.
    Operator(Name, TypeId),
    /// `New(tpt)`.
    New(TypeId),
    /// `Select(New(tpt), <init>)`.
    Ctor(TypeId),
    /// A bare reference to a class or a package, `Ident(Tuple2)` in the tree of a tuple literal.
    Ref(SymRef),
}

type A<'v> = &'v [Value];

fn arg(a: A, i: usize) -> Value {
    a.get(i).cloned().unwrap_or(Value::Null)
}

/// The diagnostics of a record that is gone (`Records`).
const STALE_TREE: &str = "a tree made by a macro in an earlier build was used again: the trees a macro makes last the build";
const STALE_METHOD: &str = "a method type made by a macro in an earlier build was used again: the types a macro makes of a method last the build";
const STALE_SITE: &str = "a symbol of another expansion's site was used again: Symbol.spliceOwner and the owners it reaches belong to their expansion";

macro_rules! q {
    ($it:ident, $name:expr, |$i:ident, $a:ident| $body:expr) => {
        reg!($it, concat!("scala.quoted.", $name), |$i, $a| $body);
    };
}

pub(super) fn install(it: &mut Table) {
    install_quotes(it);
    install_trees(it);
    install_definition_parts(it);
    install_types(it);
    install_symbols(it);
    install_context(it);
    install_concurrent(it);
}

/// The stand-in the std's `java.util.concurrent` layer names for an absent value; the file
/// system behind `java.nio.file` is `files::install`'s.
fn install_concurrent(it: &mut Table) {
    reg!(it, "java.util.concurrent.absent", |_it, _a| Ok(Value::Null));
}

impl<'a, 't> Interp<'a, 't> {
    /// Fresh locals for the binders of the quote whose index is the template's first argument.
    pub(super) fn enter_quote(&mut self, args: crate::ast::ListRef) -> R<()> {
        let first = self.prog().expr_list(args).first().copied();
        let index = match first.map(|e| self.prog().expr(e)) {
            Some(TExpr::Int(i)) => i as usize,
            _ => return self.unsupported("a quote without its index"),
        };
        let binders = self.prog().quotes[index].binders.clone();
        let mut renames = FxMap::default();
        for b in binders {
            let fresh = self.typer.clone_local(b);
            renames.insert(b, fresh);
        }
        self.quote_renames.push(renames);
        Ok(())
    }

    /// The renames of every quote being run, innermost first.
    fn active_renames(&self) -> FxMap<SymId, SymId> {
        let mut out = FxMap::default();
        for map in &self.quote_renames {
            for (&k, &v) in map {
                out.insert(k, v);
            }
        }
        out
    }

    // ---- values ----

    pub(super) fn tree_arg(&mut self, a: A, i: usize) -> R<TreeRef> {
        match a.get(i) {
            Some(Value::Tree(t)) => Ok(*t),
            Some(other) => {
                let shown = self.to_str(other)?;
                self.unsupported(format!("argument {} is no tree: {}", i, shown))
            }
            None => self.unsupported(format!("argument {} is missing", i)),
        }
    }

    fn expr_arg(&mut self, a: A, i: usize) -> R<TExprId> {
        let t = self.tree_arg(a, i)?;
        self.materialize(t)
    }

    /// The val whose `Symbol.typeRef` `t` is: a path the store keeps unshared, which it makes
    /// for a typeRef alone (`TypeStore::term_unshared`) and whose provenance survives an import,
    /// an export and the merge, whichever run or view made it.
    fn val_type_ref(&self, t: TypeId) -> Option<SymId> {
        match self.typer.types.get(t) {
            Type::Term(sym) if self.typer.types.is_unshared(t) => Some(sym),
            _ => None,
        }
    }

    pub(super) fn type_arg(&mut self, a: A, i: usize) -> R<TypeId> {
        match a.get(i) {
            Some(Value::Type(t)) => Ok(*t),
            Some(other) => {
                let shown = self.to_str(other)?;
                self.unsupported(format!("argument {} is no type: {}", i, shown))
            }
            None => self.unsupported(format!("argument {} is missing", i)),
        }
    }

    fn sym_arg(&mut self, a: A, i: usize) -> R<SymRef> {
        match a.get(i) {
            Some(Value::Sym(s)) => self.live_sym(*s),
            Some(other) => {
                let shown = self.to_str(other)?;
                self.unsupported(format!("argument {} is no symbol: {}", i, shown))
            }
            None => self.unsupported(format!("argument {} is missing", i)),
        }
    }

    pub(super) fn str_arg(&mut self, a: A, i: usize) -> R<Rc<str>> {
        match a.get(i) {
            Some(Value::Str(s)) => Ok(s.clone()),
            Some(other) => {
                let s = self.to_str(other)?;
                Ok(Rc::from(s))
            }
            None => self.unsupported(format!("argument {} is missing", i)),
        }
    }

    fn int_arg(&mut self, a: A, i: usize) -> R<i32> {
        match a.get(i).and_then(|v| v.as_i32()) {
            Some(v) => Ok(v),
            None => self.unsupported(format!("argument {} is no Int", i)),
        }
    }

    pub(super) fn known_scala_class(&mut self, name: &'static str) -> R<ClassId> {
        match self.known_class(name) {
            Some(c) => Ok(c),
            None => self.unsupported(format!("{} is not in the program", name)),
        }
    }

    pub(super) fn make_list(&mut self, items: Vec<Value>) -> R {
        let cons = self.known_scala_class("::")?;
        let nil = self.known_scala_class("Nil")?;
        let mut out = self.module(nil)?;
        for item in items.into_iter().rev() {
            out = self.construct_new(cons, vec![item, out], &Frame::new(None))?;
        }
        Ok(out)
    }

    pub(super) fn list_items(&mut self, v: &Value) -> R<Vec<Value>> {
        let mut out = Vec::new();
        let mut cur = v.clone();
        loop {
            match self.call_by_name(cur.clone(), "isEmpty", Vec::new())? {
                Value::Bool(true) => return Ok(out),
                Value::Bool(false) => {}
                other => {
                    let shown = self.to_str(&other)?;
                    return self.unsupported(format!("a List was expected, found {}", shown));
                }
            }
            out.push(self.call_by_name(cur.clone(), "head", Vec::new())?);
            cur = self.call_by_name(cur, "tail", Vec::new())?;
        }
    }

    fn tree_list(&mut self, v: &Value) -> R<Vec<TreeRef>> {
        let items = self.list_items(v)?;
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            match item {
                Value::Tree(t) => out.push(t),
                other => {
                    let shown = self.to_str(&other)?;
                    return self.unsupported(format!("a tree was expected in the list, found {}", shown));
                }
            }
        }
        Ok(out)
    }

    fn expr_list(&mut self, v: &Value) -> R<Vec<TExprId>> {
        let trees = self.tree_list(v)?;
        trees.into_iter().map(|t| self.materialize(t)).collect()
    }

    fn type_list(&mut self, v: &Value) -> R<Vec<TypeId>> {
        let items = self.list_items(v)?;
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            match item {
                Value::Type(t) => out.push(t),
                Value::Tree(TreeRef::Type(t)) => out.push(t),
                other => {
                    let shown = self.to_str(&other)?;
                    return self.unsupported(format!("a type was expected in the list, found {}", shown));
                }
            }
        }
        Ok(out)
    }

    pub(super) fn make_some(&mut self, v: Value) -> R {
        let some = self.known_scala_class("Some")?;
        self.construct_new(some, vec![v], &Frame::new(None))
    }

    pub(super) fn make_none(&mut self) -> R {
        let none = self.known_scala_class("None")?;
        self.module(none)
    }

    fn make_option(&mut self, v: Option<Value>) -> R {
        match v {
            Some(v) => self.make_some(v),
            None => self.make_none(),
        }
    }

    fn option_item(&mut self, v: &Value) -> R<Option<Value>> {
        match self.call_by_name(v.clone(), "isEmpty", Vec::new())? {
            Value::Bool(true) => Ok(None),
            _ => Ok(Some(self.call_by_name(v.clone(), "get", Vec::new())?)),
        }
    }

    pub(super) fn make_tuple(&mut self, items: Vec<Value>) -> R {
        match items.len() {
            0 => {
                let c = self.known_scala_class("EmptyTuple")?;
                self.module(c)
            }
            1 => {
                let c = self.known_scala_class("Tuple1")?;
                self.construct_new(c, items, &Frame::new(None))
            }
            n => {
                let name: &'static str = match n {
                    2 => "Tuple2",
                    3 => "Tuple3",
                    4 => "Tuple4",
                    5 => "Tuple5",
                    6 => "Tuple6",
                    7 => "Tuple7",
                    8 => "Tuple8",
                    _ => return self.unsupported("a tuple of more than eight results"),
                };
                let c = self.known_scala_class(name)?;
                self.construct_new(c, items, &Frame::new(None))
            }
        }
    }

    fn tree_values(&mut self, trees: Vec<TreeRef>) -> R {
        let items = trees.into_iter().map(Value::Tree).collect();
        self.make_list(items)
    }

    fn sym_values(&mut self, syms: Vec<SymRef>) -> R {
        let items = syms.into_iter().map(Value::Sym).collect();
        self.make_list(items)
    }

    fn type_values(&mut self, types: Vec<TypeId>) -> R {
        let items = types.into_iter().map(Value::Type).collect();
        self.make_list(items)
    }

    fn str_values(&mut self, items: Vec<String>) -> R {
        let items = items.into_iter().map(Value::string).collect();
        self.make_list(items)
    }

    /// The class `scala.quoted.Reflect.<name>`.
    pub(super) fn reflect_class(&mut self, name: &'static str) -> Option<ClassId> {
        if let Some(c) = self.reflect_classes.get(name) {
            return *c;
        }
        let found = self.typer.class_at(&["scala", "quoted", "Reflect"]).and_then(|r| {
            let n = self.typer.interner.intern(name);
            self.typer.complete_class(r);
            self.syms().class(r).nested.get(&n).copied()
        });
        self.reflect_classes.insert(name, found);
        found
    }

    pub(super) fn reflect_class_named(&mut self, c: ClassId, name: &'static str) -> bool {
        self.reflect_class(name) == Some(c)
    }

    fn quoted_class(&mut self, path: &[&str]) -> R<ClassId> {
        let key = path.join(".");
        let found = match self.path_classes.get(&key) {
            Some(c) => *c,
            None => {
                let c = self.typer.class_at(path);
                self.path_classes.insert(key.clone(), c);
                c
            }
        };
        match found {
            Some(c) => Ok(c),
            None => self.unsupported(format!("{} is missing from the standard library", key)),
        }
    }

    fn constant_value(&mut self, v: &Value) -> R<Value> {
        let c = self.quoted_class(&["scala", "quoted", "Reflect", "Constant"])?;
        let field = self.syms().class(c).ctor_syms.first().and_then(|f| f.first().copied());
        match (v, field) {
            (Value::Obj(o), Some(f)) if o.class == c => self.get_field(v.clone(), f),
            _ => {
                let shown = self.to_str(v)?;
                self.unsupported(format!("a Constant was expected, found {}", shown))
            }
        }
    }

    fn make_constant(&mut self, v: Value) -> R {
        let c = self.quoted_class(&["scala", "quoted", "Reflect", "Constant"])?;
        self.construct_new(c, vec![v], &Frame::new(None))
    }

    fn macro_ctx(&mut self) -> R<Rc<MacroCtx>> {
        match &self.macro_ctx {
            Some(c) => Ok(c.clone()),
            None => self.unsupported("scala.quoted is only available while a macro expands"),
        }
    }

    // ---- diagnostics of a macro ----

    fn report(&mut self, a: A, warning: bool) -> R<()> {
        let msg = self.str_arg(a, 1)?;
        let ctx = self.macro_ctx()?;
        let (file, span) = match a.get(2) {
            Some(Value::Pos(f, s, e)) => {
                let text = &self.typer.source(*f).text;
                (*f, Span::new(boundary(text, *s).0 as u32, boundary(text, *e).0 as u32))
            }
            _ => (ctx.site_file, ctx.site_span),
        };
        // A message at the expansion's position stands at the macro's own call, which the
        // expansions around it carry to the outermost with their `inlined from` notes, as
        // scalac's position of an inlined call keeps its inline stack trace.
        let (file, span) = if (file, span) == (ctx.expansion_file, ctx.expansion_span) { (ctx.site_file, ctx.site_span) } else { (file, span) };
        if warning {
            self.typer.diags.warn(file, span, msg.to_string());
        } else {
            self.typer.diags.error(file, span, msg.to_string());
        }
        Ok(())
    }

    fn stop_expansion<T>(&mut self) -> R<T> {
        let c = self.quoted_class(&["scala", "quoted", "runtime", "StopMacroExpansion"])?;
        let e = self.construct_new(c, Vec::new(), &Frame::new(None))?;
        Err(Control::Throw(e))
    }

    // ---- positions ----

    // A position is a byte offset of the source's text, or, between the two units of a
    // supplementary character, the byte after the character's first, where no character starts
    // (`boundary`). A macro reads it as scalac's, the index of a UTF-16 unit of the file's
    // content, and a column counts units likewise: the text's own units (`source_units`), where
    // no character stands in for a lone surrogate.

    pub(super) fn show_position(&mut self, f: FileId, start: u32, _end: u32) -> String {
        let (line, col) = self.line_col(f, start);
        format!("{}:{}:{}", self.typer.source(f).path, line + 1, col + 1)
    }

    fn line_col(&self, f: FileId, offset: u32) -> (i32, i32) {
        let text = &self.typer.source(f).text;
        let (at, inside) = boundary(text, offset);
        let (line, col, _) = crate::source::locate(text, at);
        (line as i32 - 1, source_units(&text[at + 1 - col..at]) as i32 + inside as i32)
    }

    fn unit_offset(&self, f: FileId, offset: u32) -> i32 {
        let text = &self.typer.source(f).text;
        let (at, inside) = boundary(text, offset);
        source_units(&text[..at]) as i32 + inside as i32
    }

    fn byte_offset(&self, f: FileId, unit: i32) -> u32 {
        let text = &self.typer.source(f).text;
        let unit = unit.max(0) as usize;
        if text.is_ascii() {
            return unit.min(text.len()) as u32;
        }
        let mut units = 0;
        for (at, c) in text.char_indices() {
            if units == unit {
                return at as u32;
            }
            let n = c.len_utf16();
            if units + n > unit {
                return at as u32 + 1;
            }
            units += n;
        }
        text.len() as u32
    }

    /// The units of `f`'s content between two positions, a cut pair's half as its lone surrogate.
    fn source_between(&self, f: FileId, start: u32, end: u32) -> Option<Value> {
        let text = &self.typer.source(f).text;
        let ((a, a_inside), (b, b_inside)) = (boundary(text, start), boundary(text, end));
        if !a_inside && !b_inside {
            return text.get(a..b).map(source_string);
        }
        let (from, to) = (self.unit_offset(f, start) as usize, self.unit_offset(f, end) as usize);
        (from <= to).then(|| Value::string(crate::text::from_units(text.encode_utf16().skip(from).take(to - from))))
    }
}

/// Source text as a string of the program: a character of the stand-ins' range is the two units
/// it is in the file, not the lone surrogate it would stand for.
fn source_string(text: &str) -> Value {
    if !text.as_bytes().contains(&0xF4) || !text.chars().any(|c| crate::text::surrogate_of(c).is_some()) {
        return Value::str(text);
    }
    let mut out = String::with_capacity(text.len() + 4);
    for c in text.chars() {
        match crate::text::surrogate_of(c) {
            Some(_) => out.push_str(&crate::text::code_point(c as u32).unwrap_or_default()),
            None => out.push(c),
        }
    }
    Value::string(out)
}

/// The UTF-16 units of source text.
fn source_units(text: &str) -> usize {
    if text.is_ascii() { text.len() } else { text.encode_utf16().count() }
}

/// The character boundary a position stands at, and whether it is the middle of the pair that
/// starts there.
fn boundary(text: &str, offset: u32) -> (usize, bool) {
    let mut at = (offset as usize).min(text.len());
    let inside = !text.is_char_boundary(at);
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    (at, inside)
}

// ---- Quotes, Expr and Type ----

fn install_quotes(it: &mut Table) {
    reg!(it, "$quote", |it, a| {
        let index = it.int_arg(a, 0)? as usize;
        let (body, ty, hole_syms, type_params) = {
            let q = &it.prog().quotes[index];
            (q.body, q.ty, q.holes.iter().map(|&(h, _)| h).collect::<Vec<_>>(), q.types.iter().map(|&(p, _)| p).collect::<Vec<_>>())
        };
        let mut holes: FxMap<SymId, TExprId> = FxMap::default();
        for (i, h) in hole_syms.iter().enumerate() {
            let t = it.tree_arg(a, 1 + i)?;
            let e = it.materialize(t)?;
            holes.insert(*h, e);
        }
        let mut subst: Subst = Vec::new();
        for (i, p) in type_params.iter().enumerate() {
            let t = it.type_arg(a, 1 + hole_syms.len() + i)?;
            subst.push((*p, t));
        }
        match body {
            Some(b) => {
                let params = it.macro_ctx.as_ref().map(|c| c.params.clone()).unwrap_or_default();
                let mut renames = it.active_renames();
                let binders = it.prog().quotes[index].binders.clone();
                for b in binders {
                    if !renames.contains_key(&b) {
                        let fresh = it.typer.clone_local(b);
                        renames.insert(b, fresh);
                    }
                    if !subst.is_empty() {
                        it.typer.subst_local_type(renames[&b], &subst);
                    }
                }
                let copied = it.typer.instantiate_quote(b, &holes, &params, &subst, &renames);
                Ok(Value::Tree(TreeRef::Expr(copied)))
            }
            None => {
                let ty = it.typer.types.in_view_here(ty);
                Ok(Value::Type(it.typer.types.subst(ty, &subst)))
            }
        }
    });
    reg!(it, "$quoteMatch", |it, a| {
        let index = it.int_arg(a, 1)? as usize;
        let (body, pat_ty, holes, tparams, from_above, type_syms) = {
            let p = &it.prog().quote_pats[index];
            (p.body, p.ty, p.holes.clone(), p.type_params.clone(), p.from_above.clone(), p.types.iter().map(|&(q, _)| q).collect::<Vec<_>>())
        };
        let mut subst: Subst = Vec::new();
        for (i, q) in type_syms.iter().enumerate() {
            subst.push((*q, it.type_arg(a, 2 + i)?));
        }
        let pat_ty = it.typer.types.in_view_here(pat_ty);
        let pat_ty = it.typer.types.subst(pat_ty, &subst);
        let mut m = Matcher { holes: &holes, tparams: &tparams, hole_vals: vec![None; holes.len()], type_bounds: vec![HoleBounds::default(); tparams.len()], type_vals: vec![None; tparams.len()], locals: Vec::new(), subst };
        let matched = match (arg(a, 0), body) {
            (Value::Tree(t), Some(b)) => {
                let s = it.materialize(t)?;
                let ok = m.expr(it, b, s);
                if it.trace {
                    let (ps, ss) = (it.show_tree(TreeRef::Expr(b)), it.show_tree(TreeRef::Expr(s)));
                    let show = |it: &mut Interp, ts: &[TypeId]| ts.iter().map(|&t| it.typer.show(t)).collect::<Vec<_>>().join(" | ");
                    let bound: Vec<String> = m.type_bounds.clone().iter().map(|b| format!("{} .. {}", show(it, &b.lo), show(it, &b.hi))).collect();
                    eprintln!("quote pattern {} against {}: {} {:?}", ps, ss, ok, bound);
                }
                ok
            }
            (Value::Type(t), None) => m.unify(it, pat_ty, t),
            _ => false,
        };
        if !matched {
            return it.make_none();
        }
        m.approximate(it, &from_above);
        let mut results: Vec<Value> = Vec::new();
        for (i, v) in m.hole_vals.iter().enumerate() {
            let _ = i;
            match v {
                Some(e) => results.push(Value::Tree(TreeRef::Expr(*e))),
                None => return it.make_none(),
            }
        }
        for v in &m.type_vals {
            match v {
                Some(t) => results.push(Value::Type(*t)),
                None => return it.make_none(),
            }
        }
        let value = match results.len() {
            0 => Value::Unit,
            1 => results.pop().unwrap(),
            _ => it.make_tuple(results)?,
        };
        it.make_some(value)
    });
    q!(it, "Quotes.Expr.show", |it, a| {
        let t = it.tree_arg(a, 0)?;
        Ok(Value::string(it.show_tree(t)))
    });
    q!(it, "Quotes.Expr.matches", |it, a| {
        let x = it.expr_arg(a, 0)?;
        let y = it.expr_arg(a, 1)?;
        let mut m = Matcher { holes: &[], tparams: &[], hole_vals: Vec::new(), type_bounds: Vec::new(), type_vals: Vec::new(), locals: Vec::new(), subst: Vec::new() };
        Ok(Value::Bool(m.expr(it, x, y)))
    });
    q!(it, "Quotes.Expr.isExprOf", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let target = it.type_arg(a, 1)?;
        let actual = it.tree_type(t)?;
        if it.conforms(actual, target) {
            return Ok(Value::Bool(true));
        }
        // A stable reference is of its singleton type too, as scalac's `Ref` of a val is a
        // `TermRef` (chimney's `Ref(caseVal).asExprOf[Enum.Case.type]`).
        let path = match t {
            TreeRef::Expr(e) => it.typer.path_of(e),
            _ => None,
        };
        Ok(Value::Bool(path.map_or(false, |p| it.conforms(p, target))))
    });
    q!(it, "Reflect.Expr.asTerm", |_it, a| Ok(arg(a, 0)));
    q!(it, "Reflect.asExprOf", |it, a| {
        let t = it.tree_arg(a, 1)?;
        let e = it.materialize(t)?;
        Ok(Value::Tree(TreeRef::Expr(e)))
    });
    q!(it, "Reflect.tupleOf", |it, a| {
        let elems = it.expr_list(&arg(a, 1))?;
        let n = elems.len();
        let c = it.typer.tuple_class(n);
        let tys: Vec<TypeId> = elems.iter().map(|&e| it.tree_type(TreeRef::Expr(e)).unwrap_or(ANY)).collect();
        let l = it.typer.prog.list(&elems);
        let te = it.typer.prog.add(TExpr::New(c, l));
        if it.typer.capturing() {
            it.typer.capture_call_targs(te, &tys);
        }
        let ty = it.typer.tuple_type(&tys);
        it.typer.prog.set_type(te, ty);
        Ok(Value::Tree(TreeRef::Expr(te)))
    });
}

// ---- the matcher of quote patterns ----

/// A method type of the reflect API, which the typer's types do not have: the parameter
/// names and types of one clause, the result (a method type for the next clause), and the class
/// parameters an `appliedTo` fills.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct MethodTypeRepr {
    names: Vec<crate::intern::Name>,
    types: Vec<TypeId>,
    result: TypeId,
    open: Vec<TParamId>,
    kind: ClauseKind,
}

/// The kind of a method type's clause, which scalac's `MethodType` keeps
/// (`isContextual`, `isImplicit`).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum ClauseKind {
    Plain,
    Using,
    Implicit,
}

impl ClauseKind {
    fn of(c: &ClauseSig) -> ClauseKind {
        match (c.is_using, c.is_implicit) {
            (_, true) => ClauseKind::Implicit,
            (true, false) => ClauseKind::Using,
            _ => ClauseKind::Plain,
        }
    }
}

/// A generic method's type before its type arguments, `[A](x: A): A`: its type parameters and
/// the method type over them.
#[derive(Clone)]
pub(super) struct PolyTypeRepr {
    params: Vec<TParamId>,
    result: TypeId,
}

struct Matcher<'m> {
    holes: &'m [SymId],
    tparams: &'m [TParamId],
    hole_vals: Vec<Option<TExprId>>,
    /// The bounds the match gathers for each type variable (`tparams`), dotty's GADT constraint
    /// over the pattern's type holes (`QuoteMatcher.instrumentTypeHoles`): what the scrutinee's
    /// types put below it and above it, beside the bounds the pattern declares.
    type_bounds: Vec<HoleBounds>,
    /// Each type variable's type, chosen from its bounds once the whole tree has matched
    /// (`typeHoleApproximation`).
    type_vals: Vec<Option<TypeId>>,
    /// Binders of the pattern paired with those of the scrutinee.
    locals: Vec<(SymId, SymId)>,
    /// The `Type` values of the type parameters the pattern mentions.
    subst: Subst,
}

impl<'m> Matcher<'m> {
    fn expr(&mut self, it: &mut Interp, p: TExprId, s: TExprId) -> bool {
        let prog = it.prog();
        let pe = prog.expr(p);
        if let TExpr::Local(h) = pe {
            if let Some(i) = self.holes.iter().position(|&x| x == h) {
                let hole_ty = it.typer.syms.sym(h).sig.as_ref().map(|sig| sig.ret).unwrap_or(ANY);
                let hole_ty = it.typer.types.subst(hole_ty, &self.subst);
                // A hole whose type binds the pattern's type variables reads them off the
                // tree's type where the IR records none.
                let actual = match it.prog().type_of(s) {
                    Some(t) => Some(t),
                    None if self.binds_type_vars(it, hole_ty) => it.tree_type(TreeRef::Expr(s)).ok(),
                    None => None,
                };
                if let Some(actual) = actual {
                    if !self.unify(it, hole_ty, actual) {
                        return false;
                    }
                }
                return match self.hole_vals[i] {
                    Some(prev) => {
                        let mut inner = Matcher { holes: &[], tparams: &[], hole_vals: Vec::new(), type_bounds: Vec::new(), type_vals: Vec::new(), locals: Vec::new(), subst: Vec::new() };
                        inner.expr(it, prev, s)
                    }
                    None => {
                        self.hole_vals[i] = Some(s);
                        true
                    }
                };
            }
        }
        let se = it.prog().expr(s);
        match (pe, se) {
            (TExpr::Local(x), TExpr::Local(y)) => x == y || self.locals.iter().any(|&(a, b)| a == x && b == y),
            (TExpr::Int(x), TExpr::Int(y)) => x == y,
            (TExpr::Long(x), TExpr::Long(y)) => x == y,
            (TExpr::Double(x), TExpr::Double(y)) => x.to_bits() == y.to_bits(),
            (TExpr::Bool(x), TExpr::Bool(y)) => x == y,
            (TExpr::Char(x), TExpr::Char(y)) => x == y,
            (TExpr::Str(x), TExpr::Str(y)) => it.prog().strings[x.idx()] == it.prog().strings[y.idx()],
            (TExpr::Unit, TExpr::Unit) | (TExpr::Null, TExpr::Null) | (TExpr::This, TExpr::This) => true,
            (TExpr::Static(x), TExpr::Static(y)) => x == y,
            (TExpr::Module(x), TExpr::Module(y)) => x == y,
            (TExpr::Field(r1, s1), TExpr::Field(r2, s2)) | (TExpr::Field(r1, s1), TExpr::CallMethod(r2, s2, _)) | (TExpr::CallMethod(r1, s1, _), TExpr::Field(r2, s2)) => {
                it.same_member(s1, s2) && self.expr(it, r1, r2)
            }
            (TExpr::CallStatic(s1, a1), TExpr::CallStatic(s2, a2)) => s1 == s2 && self.list(it, a1, a2),
            (TExpr::CallMethod(r1, s1, a1), TExpr::CallMethod(r2, s2, a2)) => it.same_member(s1, s2) && self.expr(it, r1, r2) && self.list(it, a1, a2),
            (TExpr::CallClosure(f1, a1), TExpr::CallClosure(f2, a2)) => self.expr(it, f1, f2) && self.list(it, a1, a2),
            (TExpr::New(c1, a1), TExpr::New(c2, a2)) => c1 == c2 && self.list(it, a1, a2),
            (TExpr::NewVia(s1, a1), TExpr::NewVia(s2, a2)) => s1 == s2 && self.list(it, a1, a2),
            (TExpr::Lambda(p1, b1), TExpr::Lambda(p2, b2)) => {
                let (ps1, ps2) = (it.prog().sym_list(p1).to_vec(), it.prog().sym_list(p2).to_vec());
                if ps1.len() != ps2.len() {
                    return false;
                }
                let mark = self.locals.len();
                self.locals.extend(ps1.iter().copied().zip(ps2.iter().copied()));
                let ok = self.expr(it, b1, b2);
                self.locals.truncate(mark);
                ok
            }
            (TExpr::If(c1, t1, e1), TExpr::If(c2, t2, e2)) => {
                self.expr(it, c1, c2)
                    && self.expr(it, t1, t2)
                    && match (e1, e2) {
                        (Some(x), Some(y)) => self.expr(it, x, y),
                        (None, None) => true,
                        _ => false,
                    }
            }
            (TExpr::While(c1, b1), TExpr::While(c2, b2)) | (TExpr::Assign(c1, b1), TExpr::Assign(c2, b2)) => self.expr(it, c1, c2) && self.expr(it, b1, b2),
            (TExpr::Prim(o1, a1, b1), TExpr::Prim(o2, a2, b2)) => o1 == o2 && self.expr(it, a1, a2) && self.expr(it, b1, b2),
            (TExpr::Unary(o1, a1), TExpr::Unary(o2, a2)) => o1 == o2 && self.expr(it, a1, a2),
            (TExpr::StrConcat(l1), TExpr::StrConcat(l2)) | (TExpr::SeqLit(l1), TExpr::SeqLit(l2)) | (TExpr::ArrayLit(l1), TExpr::ArrayLit(l2)) => self.list(it, l1, l2),
            (TExpr::ToStr(a1, _), TExpr::ToStr(a2, _)) | (TExpr::Spread(a1), TExpr::Spread(a2)) | (TExpr::Return(a1), TExpr::Return(a2)) | (TExpr::Throw(a1, _), TExpr::Throw(a2, _)) => self.expr(it, a1, a2),
            (TExpr::Js(s1, a1), TExpr::Js(s2, a2)) => it.prog().strings[s1.idx()] == it.prog().strings[s2.idx()] && self.list(it, a1, a2),
            (TExpr::TypeTest(a1, t1), TExpr::TypeTest(a2, t2)) => self.expr(it, a1, a2) && it.prog().tests[t1.idx()] == it.prog().tests[t2.idx()],
            (TExpr::Cast(a1, _, t1), TExpr::Cast(a2, _, t2)) => t1 == t2 && self.expr(it, a1, a2),
            (TExpr::Index(a1, i1), TExpr::Index(a2, i2)) => i1 == i2 && self.expr(it, a1, a2),
            (TExpr::Block(st1, r1), TExpr::Block(st2, r2)) => {
                if st1.len != st2.len {
                    return false;
                }
                let mark = self.locals.len();
                let (items1, items2) = (it.prog().stmts[st1.range()].to_vec(), it.prog().stmts[st2.range()].to_vec());
                let mut ok = true;
                for (x, y) in items1.into_iter().zip(items2) {
                    ok = ok
                        && match (x, y) {
                            (TStmt::Expr(a), TStmt::Expr(b)) => self.expr(it, a, b),
                            (TStmt::Val(v1, a), TStmt::Val(v2, b)) => {
                                let same = self.expr(it, a, b);
                                self.locals.push((v1, v2));
                                same
                            }
                            (TStmt::Fun(f1), TStmt::Fun(f2)) => {
                                let (s1, ps1, b1) = { let f = &it.prog().funs[f1.idx()]; (f.sym, f.params.clone(), f.body) };
                                let (s2, ps2, b2) = { let f = &it.prog().funs[f2.idx()]; (f.sym, f.params.clone(), f.body) };
                                self.locals.push((s1, s2));
                                if ps1.len() != ps2.len() {
                                    false
                                } else {
                                    self.locals.extend(ps1.into_iter().zip(ps2));
                                    match (b1, b2) {
                                        (Some(x), Some(y)) => self.expr(it, x, y),
                                        (None, None) => true,
                                        _ => false,
                                    }
                                }
                            }
                            _ => false,
                        };
                }
                let ok = ok && self.expr(it, r1, r2);
                self.locals.truncate(mark);
                ok
            }
            (TExpr::Match(s1, c1), TExpr::Match(s2, c2)) => self.expr(it, s1, s2) && self.cases(it, c1, c2),
            (TExpr::Try(i1), TExpr::Try(i2)) => {
                let (b1, c1, f1) = { let t = &it.prog().tries[i1 as usize]; (t.body, t.cases, t.finalizer) };
                let (b2, c2, f2) = { let t = &it.prog().tries[i2 as usize]; (t.body, t.cases, t.finalizer) };
                self.expr(it, b1, b2)
                    && self.cases(it, c1, c2)
                    && match (f1, f2) {
                        (Some(x), Some(y)) => self.expr(it, x, y),
                        (None, None) => true,
                        _ => false,
                    }
            }
            _ => false,
        }
    }

    fn list(&mut self, it: &mut Interp, a: crate::ast::ListRef, b: crate::ast::ListRef) -> bool {
        if a.len != b.len {
            return false;
        }
        let (xs, ys) = (it.prog().expr_list(a).to_vec(), it.prog().expr_list(b).to_vec());
        xs.into_iter().zip(ys).all(|(x, y)| self.expr(it, x, y))
    }

    fn cases(&mut self, it: &mut Interp, a: crate::ast::ListRef, b: crate::ast::ListRef) -> bool {
        if a.len != b.len {
            return false;
        }
        let (xs, ys) = (it.prog().cases[a.range()].to_vec(), it.prog().cases[b.range()].to_vec());
        for (x, y) in xs.into_iter().zip(ys) {
            let mark = self.locals.len();
            let ok = self.pat(it, x.pat, y.pat)
                && match (x.guard, y.guard) {
                    (Some(g1), Some(g2)) => self.expr(it, g1, g2),
                    (None, None) => true,
                    _ => false,
                }
                && self.expr(it, x.body, y.body);
            self.locals.truncate(mark);
            if !ok {
                return false;
            }
        }
        true
    }

    fn pat(&mut self, it: &mut Interp, a: TPatId, b: TPatId) -> bool {
        let (pa, pb) = (it.prog().pats[a.idx()], it.prog().pats[b.idx()]);
        match (pa, pb) {
            (TPat::Wildcard, TPat::Wildcard) => true,
            (TPat::Bind(x, i1), TPat::Bind(y, i2)) => {
                self.locals.push((x, y));
                match (i1, i2) {
                    (Some(p), Some(q)) => self.pat(it, p, q),
                    (None, None) => true,
                    _ => false,
                }
            }
            (TPat::Test(_, t1, i1), TPat::Test(_, t2, i2)) => self.unify(it, t1, t2) && self.pat(it, i1, i2),
            (TPat::Equals(e1, _), TPat::Equals(e2, _)) => self.expr(it, e1, e2),
            (TPat::Class(c1, _, _, s1), TPat::Class(c2, _, _, s2)) => c1 == c2 && self.pats(it, s1, s2),
            (TPat::Alt(s1), TPat::Alt(s2)) => self.pats(it, s1, s2),
            (TPat::Unapply(x, c1, i1), TPat::Unapply(y, c2, i2)) => {
                self.locals.push((x, y));
                self.expr(it, c1, c2) && self.pat(it, i1, i2)
            }
            _ => false,
        }
    }

    fn pats(&mut self, it: &mut Interp, a: crate::ast::ListRef, b: crate::ast::ListRef) -> bool {
        if a.len != b.len {
            return false;
        }
        let (xs, ys) = (it.prog().pat_lists[a.range()].to_vec(), it.prog().pat_lists[b.range()].to_vec());
        xs.into_iter().zip(ys).all(|(x, y)| self.pat(it, x, y))
    }

    /// Whether the scrutinee type fits the pattern type, binding the pattern's type variables.
    fn binds_type_vars(&self, it: &mut Interp, t: TypeId) -> bool {
        if self.tparams.is_empty() {
            return false;
        }
        let mut params = Vec::new();
        it.typer.collect_type_params(t, &mut params);
        params.iter().any(|q| self.tparams.contains(q))
    }

    /// Whether the scrutinee's type `s` fits the pattern's `p` (`scType <:< pattern.tpe`, dotty's
    /// `isSubTypeUnderEnv`), constraining the pattern's type variables.
    fn unify(&mut self, it: &mut Interp, p: TypeId, s: TypeId) -> bool {
        self.constrain(it, p, s, 1)
    }

    /// Whether the scrutinee's type `s` and the pattern's `p` relate as `variance` says (`1`: `s
    /// <: p`, `-1`: `p <: s`, `0`: both), each type variable `p` holds taking the bound the
    /// relation puts on it, as the subtype check under the match's GADT constraint adds it
    /// (an intersection or a union decomposed as dotty's `TypeComparer` does, one of two
    /// alternatives tried with the other's constraint rolled back): both in the reader's view
    /// first, a pattern's type being its record's (a peer's or the base's) and a scrutinee's
    /// what its tree records, so that the comparisons below are of two ids of one view.
    fn constrain(&mut self, it: &mut Interp, p: TypeId, s: TypeId, variance: i8) -> bool {
        if variance == 0 {
            return self.constrain(it, p, s, 1) && self.constrain(it, p, s, -1);
        }
        let below = variance > 0;
        let (p, s) = (it.typer.types.in_view_here(p), it.typer.types.in_view_here(s));
        let p = it.typer.zonk(p);
        let p = it.typer.types.subst(p, &self.subst);
        let s = it.typer.zonk(s);
        if let Type::Param(tp) = it.typer.types.get(p) {
            if let Some(i) = self.tparams.iter().position(|&x| x == tp) {
                return if below { self.add_lower(it, i, s) } else { self.add_upper(it, i, s) };
            }
        }
        let relate = |it: &mut Interp, p: TypeId, s: TypeId| if below { it.conforms(s, p) } else { it.conforms(p, s) };
        if self.tparams.is_empty() {
            return relate(it, p, s);
        }
        let mut params = Vec::new();
        it.typer.collect_type_params(p, &mut params);
        if !params.iter().any(|q| self.tparams.contains(q)) {
            return relate(it, p, s);
        }
        let sd = it.typer.dealias(s);
        // An alias in the pattern (`'[Lens[f, t]]`) matches what it stands for.
        let p = it.typer.deref_alias(p);
        let (pt, st) = (it.typer.types.get(p), it.typer.types.get(sd));
        // The side that has to be below both parts of an intersection, or above both parts of a
        // union, is compared with each; the side that has one of two parts to compare is tried
        // with each in turn (`Computed[..] & TransformerOverrides` against `'[Computed[path,
        // tail]]`; `t & Tuple` against a tuple).
        match (pt, st, below) {
            (Type::Inter(a, b), _, true) | (Type::Union(a, b), _, false) => return self.constrain(it, a, s, variance) && self.constrain(it, b, s, variance),
            (_, Type::Union(a, b), true) | (_, Type::Inter(a, b), false) => return self.constrain(it, p, a, variance) && self.constrain(it, p, b, variance),
            (Type::Union(a, b), _, true) | (Type::Inter(a, b), _, false) => return self.either(it, |m, it| m.constrain(it, a, s, variance), |m, it| m.constrain(it, b, s, variance)),
            (_, Type::Inter(a, b), true) | (_, Type::Union(a, b), false) => return self.either(it, |m, it| m.constrain(it, p, a, variance), |m, it| m.constrain(it, p, b, variance)),
            _ => {}
        }
        match (pt, st) {
            // `Mirror.Product { type MirroredElemLabels = labels } & Mirror.ProductOf[A]`: the
            // parent against the scrutinee's type, a refinement's member as the scrutinee's.
            (Type::Refined(parent, r), _) => {
                let Refinement::Alias(name, rhs) = it.typer.types.refinement(r) else { return self.constrain(it, parent, s, variance) };
                let Some(member) = it.typer.member_type(s, name) else { return false };
                self.constrain(it, parent, s, variance) && self.constrain(it, rhs, member, 0)
            }
            // `h *: t` against a `TupleN`: its first element and the rest as a cons chain, as
            // scalac's subtype check reads a tuple class.
            (Type::Class(c1, a1), Type::Class(c2, a2)) if Some(c1) == it.typer.b.cons_tuple && it.typer.is_tuple_class(c2) => {
                let (pat, elems) = (it.typer.types.items(a1).to_vec(), it.typer.types.items(a2).to_vec());
                let [h, t] = pat[..] else { return false };
                let mut rest = it.typer.empty_tuple_type();
                for &e in elems[1..].iter().rev() {
                    rest = it.typer.types.class(c1, &[e, rest]);
                }
                self.constrain(it, h, elems[0], variance) && self.constrain(it, t, rest, variance)
            }
            // The arguments of a class by its parameters' variances: the scrutinee's base type at
            // the pattern's class (`s <: p`), or the pattern's at the scrutinee's (`p <: s`).
            (Type::Class(c1, a1), _) => {
                let (class, xs, ys) = if below {
                    let base = match st {
                        Type::Class(c2, _) if c1 == c2 => Some(sd),
                        _ => it.typer.base_type(sd, c1),
                    };
                    let Some(base) = base else { return false };
                    let Type::Class(_, a2) = it.typer.types.get(base) else { return false };
                    (c1, it.typer.types.items(a1).to_vec(), it.typer.types.items(a2).to_vec())
                } else {
                    let Type::Class(c2, a2) = st else { return relate(it, p, s) };
                    let base = if c1 == c2 { Some(p) } else { it.typer.base_type(p, c2) };
                    let Some(base) = base else { return false };
                    let Type::Class(_, pa) = it.typer.types.get(base) else { return false };
                    (c2, it.typer.types.items(pa).to_vec(), it.typer.types.items(a2).to_vec())
                };
                if xs.len() != ys.len() {
                    return false;
                }
                let variances: Vec<i8> = it.typer.syms.class(class).tparams.iter().map(|&q| it.typer.syms.tparam(q).variance).collect();
                xs.into_iter().zip(ys).enumerate().all(|(k, (x, y))| {
                    let v = variances.get(k).copied().unwrap_or(0);
                    self.constrain(it, x, y, variance * v)
                })
            }
            _ => relate(it, p, s),
        }
    }

    /// The first alternative, or the second with what the first constrained rolled back.
    fn either(&mut self, it: &mut Interp, first: impl FnOnce(&mut Self, &mut Interp) -> bool, second: impl FnOnce(&mut Self, &mut Interp) -> bool) -> bool {
        let before = self.type_bounds.clone();
        if first(self, it) {
            return true;
        }
        self.type_bounds = before;
        second(self, it)
    }

    /// `s` below the type variable `i`: below every bound gathered above it, and below what it
    /// declares above itself, which puts `s` below another variable a bound names (`type b <:
    /// a`), as the GADT constraint keeps a variable's dependencies.
    fn add_lower(&mut self, it: &mut Interp, i: usize, s: TypeId) -> bool {
        if self.type_bounds[i].lo.iter().any(|&l| l == s || it.typer.is_same(l, s)) {
            return true;
        }
        self.type_bounds[i].lo.push(s);
        let uppers = self.type_bounds[i].hi.clone();
        if !uppers.into_iter().all(|h| it.conforms(s, h)) {
            return false;
        }
        let declared = it.typer.syms.tparam(self.tparams[i]).upper;
        self.constrain(it, declared, s, 1)
    }

    /// `s` above the type variable `i`: above every bound gathered below it and what it declares
    /// below itself.
    fn add_upper(&mut self, it: &mut Interp, i: usize, s: TypeId) -> bool {
        if self.type_bounds[i].hi.iter().any(|&h| h == s || it.typer.is_same(h, s)) {
            return true;
        }
        self.type_bounds[i].hi.push(s);
        let lowers = self.type_bounds[i].lo.clone();
        if !lowers.into_iter().all(|l| it.conforms(l, s)) {
            return false;
        }
        let declared = it.typer.syms.tparam(self.tparams[i]).lower;
        self.constrain(it, declared, s, -1)
    }

    /// Each type variable's type once the tree has matched, from its full bounds
    /// (`QuoteMatcher.typeHoleApproximation`): the join of what is below it, its declared lower
    /// bound among them, another variable a bound names standing for that one's type.
    fn approximate(&mut self, it: &mut Interp, from_above: &[bool]) {
        let mut done: Vec<Option<TypeId>> = vec![None; self.tparams.len()];
        let mut visiting = vec![false; self.tparams.len()];
        for i in 0..self.tparams.len() {
            self.approximate_one(it, i, from_above, &mut done, &mut visiting);
        }
        self.type_vals = done;
    }

    /// A variable marked `@fromAbove` takes the meet of what is above it, its declared upper
    /// bound among them; any other the join of what is below it.
    fn approximate_one(&mut self, it: &mut Interp, i: usize, from_above: &[bool], done: &mut Vec<Option<TypeId>>, visiting: &mut Vec<bool>) -> TypeId {
        if let Some(t) = done[i] {
            return t;
        }
        let param = it.typer.types.param(self.tparams[i]);
        if visiting[i] {
            return param;
        }
        visiting[i] = true;
        let above = from_above.get(i).copied().unwrap_or(false);
        let info = it.typer.syms.tparam(self.tparams[i]);
        let declared = it.typer.types.subst(if above { info.upper } else { info.lower }, &self.subst);
        let mut named = Vec::new();
        it.typer.collect_type_params(declared, &mut named);
        let mut by_hole: Subst = Vec::new();
        for q in named {
            if let Some(j) = self.tparams.iter().position(|&x| x == q) {
                let t = self.approximate_one(it, j, from_above, done, visiting);
                by_hole.push((q, t));
            }
        }
        let declared = it.typer.types.subst(declared, &by_hole);
        let (gathered, trivial) = if above { (self.type_bounds[i].hi.clone(), ANY) } else { (self.type_bounds[i].lo.clone(), NOTHING) };
        let mut bounds: Vec<TypeId> = Vec::new();
        for t in std::iter::once(declared).chain(gathered) {
            if t == trivial || bounds.iter().any(|&l| l == t || it.typer.is_same(l, t)) {
                continue;
            }
            bounds.push(t);
        }
        // The join of the lower bounds (the meet of the upper ones): a bound below (above)
        // another is subsumed by it, the rest their union (intersection), in order.
        let subsumed = |it: &mut Interp, t: TypeId, u: TypeId| if above { it.conforms(u, t) } else { it.conforms(t, u) };
        let mut kept: Vec<TypeId> = Vec::new();
        for (k, &t) in bounds.iter().enumerate() {
            let gone = bounds.iter().enumerate().any(|(j, &u)| j != k && subsumed(it, t, u) && !(j > k && subsumed(it, u, t)));
            if !gone {
                kept.push(t);
            }
        }
        let t = kept.into_iter().reduce(|a, b| if above { it.typer.types.inter(a, b) } else { it.typer.types.union(a, b) }).unwrap_or(trivial);
        done[i] = Some(t);
        t
    }
}

/// What a match has put below and above a pattern's type variable.
#[derive(Clone, Default)]
struct HoleBounds {
    lo: Vec<TypeId>,
    hi: Vec<TypeId>,
}

impl<'a, 't> Interp<'a, 't> {
    /// Whether a member selected by a pattern is the one the scrutinee selects, or is
    /// overridden by it: the same name, declared in a class the scrutinee's owner extends.
    fn same_member(&mut self, pattern: SymId, scrutinee: SymId) -> bool {
        if pattern == scrutinee {
            return true;
        }
        let (p, s) = (self.syms().sym(pattern), self.syms().sym(scrutinee));
        if p.name != s.name {
            return false;
        }
        match (p.owner, s.owner) {
            (Owner::Class(pc), Owner::Class(sc)) => {
                self.typer.complete_class(sc);
                self.is_subclass(sc, pc)
            }
            _ => false,
        }
    }

    /// `a <: b` without leaving bounds on the typer's variables.
    /// `<:<` as the call site sees it: an opaque type is transparent there only when the site
    /// is inside its scope, whatever the scope of the inline method that expands.
    pub(super) fn conforms(&mut self, a: TypeId, b: TypeId) -> bool {
        // `=> A` conforms to `=> B` as `A` to `B`, and to no type that is not by-name.
        match (self.by_name_underlying(a).unwrap_or(None), self.by_name_underlying(b).unwrap_or(None)) {
            (Some(x), Some(y)) => self.conforms(x, y),
            (None, None) => self.at_site(|t| t.is_sub(a, b)),
            _ => false,
        }
    }

    fn same_type(&mut self, a: TypeId, b: TypeId) -> bool {
        match (self.by_name_underlying(a).unwrap_or(None), self.by_name_underlying(b).unwrap_or(None)) {
            (Some(x), Some(y)) => self.same_type(x, y),
            (None, None) => self.at_site(|t| t.is_same(a, b)),
            _ => false,
        }
    }

    fn at_site(&mut self, check: impl FnOnce(&mut Worker) -> bool) -> bool {
        let mark = self.typer.snapshot();
        let ok = match self.typer.inline.site_env.clone() {
            Some(env) => self.typer.with_env((*env).clone(), check),
            None => check(self.typer),
        };
        self.typer.rollback(mark);
        ok
    }
}

// ---- tree views ----

/// The kind of the reflect API a tree shows as, and its parts.
pub(super) enum View {
    Literal(Value),
    /// A reference to a definition: `Ident` for a local or a static, `Select` with a receiver.
    Ident(SymRef),
    Select(TreeRef, SymRef, Name),
    Apply(TreeRef, Vec<TreeRef>),
    TypeApply(TreeRef, Vec<TypeId>),
    New(TypeId),
    This,
    Super,
    Typed(TreeRef, TypeId),
    Repeated(Vec<TreeRef>, TypeId),
    Assign(TreeRef, TreeRef),
    Block(Vec<TreeRef>, TreeRef),
    Lambda(Vec<SymId>, TreeRef),
    Closure(TreeRef),
    If(TreeRef, TreeRef, TreeRef),
    Match(TreeRef, Vec<TreeRef>),
    Try(TreeRef, Vec<TreeRef>, Option<TreeRef>),
    Return(TreeRef),
    While(TreeRef, TreeRef),
    ValDef(SymId, Option<TreeRef>),
    DefDef(SymId, Vec<Vec<SymRef>>, Option<TreeRef>),
    ClassDef(ClassId),
    TypeDef(SymRef),
    CaseDef(TreeRef, Option<TreeRef>, TreeRef),
    Bind(SymId, TreeRef),
    Unapply(Vec<TreeRef>),
    Alternatives(Vec<TreeRef>),
    Wildcard,
    TypeTree(TypeId),
    TypedPattern(TreeRef, TypeId),
    NamedArg(Name, TreeRef),
}

impl<'a, 't> Interp<'a, 't> {
    /// Whether a method symbol takes arguments: a selection of it awaits an `Apply`.
    fn takes_arguments(&mut self, s: SymId) -> bool {
        let info = self.syms().sym(s);
        match info.kind {
            SymKind::Def | SymKind::Given => {
                let sig = self.typer.sig_of(s);
                !sig.clauses.is_empty()
            }
            _ => false,
        }
    }

    fn pending(&mut self, p: Pending) -> TreeRef {
        TreeRef::Pending(self.records.pending.push(p))
    }

    fn pending_of(&self, i: u32) -> R<Pending> {
        match self.records.pending.get(i) {
            Some(p) => Ok(p.clone()),
            None => self.stale(STALE_TREE),
        }
    }

    fn named_arg_of(&self, i: u32) -> R<(Name, TreeRef)> {
        match self.records.named_args.get(i) {
            Some(&entry) => Ok(entry),
            None => self.stale(STALE_TREE),
        }
    }

    fn partial_of(&self, i: u32) -> R<(TreeRef, Vec<TreeRef>)> {
        match self.records.partials.get(i) {
            Some(entry) => Ok(entry.clone()),
            None => self.stale(STALE_TREE),
        }
    }

    fn sym_of_template(&mut self, s: StrRef) -> Option<SymId> {
        self.prog().template_syms.get(&s).copied()
    }

    /// The view of a tree in the shapes of the reflect API.
    pub(super) fn view(&mut self, t: TreeRef) -> R<View> {
        Ok(match t {
            TreeRef::Expr(e) => match self.prog().expr(e) {
                TExpr::Int(v) => View::Literal(match self.prog().type_of(e).map(|ty| self.typer.types.get(ty)) {
                    Some(Type::Class(c, _)) if c == self.typer.b.byte => Value::Byte(v as i8),
                    Some(Type::Class(c, _)) if c == self.typer.b.short => Value::Short(v as i16),
                    _ => Value::Int(v),
                }),
                TExpr::Long(v) => View::Literal(Value::Long(v)),
                TExpr::Double(v) => View::Literal(match self.prog().type_of(e).map(|ty| self.typer.types.get(ty)) {
                    Some(Type::Class(c, _)) if c == self.typer.b.float => Value::Float(v as f32),
                    _ => Value::Double(v),
                }),
                TExpr::Bool(v) => View::Literal(Value::Bool(v)),
                TExpr::Char(v) => View::Literal(Value::Char(v)),
                TExpr::Str(s) => View::Literal(Value::str(&self.prog().strings[s.idx()])),
                TExpr::Unit => View::Literal(Value::Unit),
                TExpr::Null => View::Literal(Value::Null),
                TExpr::ClassOf(c) => {
                    let n = self.syms().class(c).tparams.len();
                    let ty = self.typer.types.class(c, &vec![ANY; n]);
                    View::Literal(Value::Type(ty))
                }
                TExpr::Local(s) | TExpr::Static(s) => View::Ident(SymRef::Term(s)),
                TExpr::Module(c) => View::Ident(self.module_sym(c)),
                TExpr::This => View::This,
                TExpr::Super(_) => View::Super,
                TExpr::Field(r, s) if matches!(self.prog().expr(r), TExpr::This) => View::Ident(SymRef::Term(s)),
                TExpr::Field(r, s) => View::Select(TreeRef::Expr(r), SymRef::Term(s), self.syms().sym(s).name),
                TExpr::CallMethod(r, s, args) if self.assigned_var(s).is_some() && args.len == 1 => {
                    // An assignment through an abstract var, which scalac's tree keeps as one.
                    let var = self.assigned_var(s).expect("an abstract var's setter");
                    let value = self.prog().expr_list(args)[0];
                    let target = self.typer.prog.add(TExpr::Field(r, var));
                    let ty = self.expr_type_or_any(value);
                    self.typer.prog.set_type(target, ty);
                    View::Assign(TreeRef::Expr(target), TreeRef::Expr(value))
                }
                TExpr::CallMethod(r, s, args) => {
                    let exprs = self.prog().expr_list(args).to_vec();
                    let items: Vec<TreeRef> = exprs.iter().map(|&x| TreeRef::Expr(x)).collect();
                    let recv = if matches!(self.prog().expr(r), TExpr::This) { None } else { Some(TreeRef::Expr(r)) };
                    match (recv, items.is_empty() && !self.takes_arguments(s)) {
                        (Some(r), true) => View::Select(r, SymRef::Term(s), self.syms().sym(s).name),
                        (None, true) => View::Ident(SymRef::Term(s)),
                        (recv, false) => {
                            let result = self.prog().type_of(e);
                            self.call_view(recv, Some(r), s, &exprs, items, result)
                        }
                    }
                }
                TExpr::CallStatic(s, args) => {
                    let items: Vec<TreeRef> = self.prog().expr_list(args).iter().map(|&x| TreeRef::Expr(x)).collect();
                    let exprs = self.prog().expr_list(args).to_vec();
                    if items.is_empty() && !self.takes_arguments(s) {
                        View::Ident(SymRef::Term(s))
                    } else if exprs.len() == 2 && self.is_arrow(s) {
                        // `a -> b` in scalac's shape, `ArrowAssoc[A](a).->[B](b)`: the receiver
                        // is the extension's own application to `a`.
                        let (ta, tb) = (self.expr_type_or_any(exprs[0]), self.expr_type_or_any(exprs[1]));
                        let l = self.typer.prog.list(&exprs[..1]);
                        let wrapped = self.typer.prog.add(TExpr::CallStatic(s, l));
                        self.typer.prog.set_type(wrapped, ta);
                        let fun = self.pending(Pending { recv: Some(TreeRef::Expr(wrapped)), kind: PendingKind::Sym(s), targs: vec![tb] });
                        View::Apply(fun, vec![items[1]])
                    } else if exprs.len() == 1 && self.is_arrow(s) {
                        let ta = self.expr_type_or_any(exprs[0]);
                        let fun = self.pending(Pending { recv: None, kind: PendingKind::Sym(s), targs: vec![ta] });
                        View::Apply(fun, items)
                    } else {
                        let result = self.prog().type_of(e);
                        self.call_view(None, None, s, &exprs, items, result)
                    }
                }
                TExpr::CallClosure(f, args) => {
                    let items: Vec<TreeRef> = self.prog().expr_list(args).iter().map(|&x| TreeRef::Expr(x)).collect();
                    let apply = self.typer.interner.intern("apply");
                    let fun = self.pending(Pending { recv: Some(TreeRef::Expr(f)), kind: PendingKind::Named(apply), targs: Vec::new() });
                    View::Apply(fun, items)
                }
                TExpr::New(c, args) if self.typer.is_tuple_class(c) => {
                    // A tuple literal is `TupleN.apply[..](..)` under scalac, never a `new`.
                    let items: Vec<TreeRef> = self.prog().expr_list(args).iter().map(|&x| TreeRef::Expr(x)).collect();
                    let targs: Vec<TypeId> = match self.prog().type_of(e).map(|t| self.typer.types.get(t)) {
                        Some(Type::Class(_, l)) => self.typer.types.items(l).to_vec(),
                        _ => Vec::new(),
                    };
                    let module = self.pending(Pending { recv: None, kind: PendingKind::Ref(SymRef::Class(c)), targs: Vec::new() });
                    let apply = self.typer.interner.intern("apply");
                    let fun = self.pending(Pending { recv: Some(module), kind: PendingKind::Named(apply), targs });
                    View::Apply(fun, items)
                }
                TExpr::New(c, args) => {
                    let items: Vec<TreeRef> = self.prog().expr_list(args).iter().map(|&x| TreeRef::Expr(x)).collect();
                    // The class constructed, which the expression's type may widen (an enum
                    // case's `T.B(1)` is a `T`).
                    let ty = match self.prog().type_of(e) {
                        Some(t) if matches!(self.typer.types.get(t), Type::Class(k, _) if k == c) => t,
                        Some(t) => {
                            let n = self.syms().class(c).tparams.len();
                            let (args, _) = self.typer.solve_pattern_class(c, n, t);
                            self.typer.types.class(c, &args)
                        }
                        None => self.typer.types.class(c, &[]),
                    };
                    let fun = self.pending(Pending { recv: None, kind: PendingKind::Ctor(ty), targs: Vec::new() });
                    View::Apply(fun, items)
                }
                TExpr::NewVia(s, args) => {
                    let items: Vec<TreeRef> = self.prog().expr_list(args).iter().map(|&x| TreeRef::Expr(x)).collect();
                    let ty = self.prog().type_of(e).unwrap_or(ANY);
                    let _ = s;
                    let fun = self.pending(Pending { recv: None, kind: PendingKind::Ctor(ty), targs: Vec::new() });
                    View::Apply(fun, items)
                }
                TExpr::Prim(op, l, r) => {
                    let primitive = self.prog().type_of(l).map_or(false, |t| self.typer.is_primitive(t));
                    let text = match op {
                        PrimOp::RefEq if primitive => "==",
                        PrimOp::RefNe if primitive => "!=",
                        _ => prim_name(op),
                    };
                    let name = self.typer.interner.intern(text);
                    let method = self.operator_type(op, l, r, e);
                    let fun = self.pending(Pending { recv: Some(TreeRef::Expr(l)), kind: PendingKind::Operator(name, method), targs: Vec::new() });
                    View::Apply(fun, vec![TreeRef::Expr(r)])
                }
                TExpr::Unary(op, x) => {
                    let name = self.typer.interner.intern(unary_name(op));
                    View::Select(TreeRef::Expr(x), SymRef::None, name)
                }
                TExpr::StrConcat(l) => {
                    let items = self.prog().expr_list(l).to_vec();
                    self.concat_view(&items)
                }
                TExpr::ToStr(x, _) => View::Select(TreeRef::Expr(x), SymRef::None, self.typer.interner.intern("toString")),
                TExpr::Js(s, args) => {
                    let items: Vec<TreeRef> = self.prog().expr_list(args).iter().map(|&x| TreeRef::Expr(x)).collect();
                    match self.sym_of_template(s) {
                        Some(sym) => {
                            let info = self.syms().sym(sym);
                            let member = matches!(info.owner, Owner::Class(_)) && !info.is_extension;
                            if member && !items.is_empty() {
                                let recv = items[0];
                                let rest = items[1..].to_vec();
                                if rest.is_empty() && !self.takes_arguments(sym) {
                                    View::Select(recv, SymRef::Term(sym), self.syms().sym(sym).name)
                                } else {
                                    let fun = self.pending(Pending { recv: Some(recv), kind: PendingKind::Sym(sym), targs: Vec::new() });
                                    View::Apply(fun, rest)
                                }
                            } else if items.is_empty() && !self.takes_arguments(sym) {
                                View::Ident(SymRef::Term(sym))
                            } else {
                                let fun = self.pending(Pending { recv: None, kind: PendingKind::Sym(sym), targs: Vec::new() });
                                View::Apply(fun, items)
                            }
                        }
                        None => {
                            let text = self.prog().strings[s.idx()].clone();
                            let head = text.trim_start_matches('$').split('(').next().unwrap_or("template").to_string();
                            let name = self.typer.interner.intern(&head);
                            let fun = self.pending(Pending { recv: None, kind: PendingKind::Named(name), targs: Vec::new() });
                            View::Apply(fun, items)
                        }
                    }
                }
                TExpr::TypeTest(x, test) => {
                    let ty = self.test_type(test);
                    let name = self.typer.interner.intern("isInstanceOf");
                    let fun = self.pending(Pending { recv: Some(TreeRef::Expr(x)), kind: PendingKind::Named(name), targs: vec![ty] });
                    View::TypeApply(fun, vec![ty])
                }
                TExpr::Cast(x, _, ty) => {
                    let name = self.typer.interner.intern("asInstanceOf");
                    let fun = self.pending(Pending { recv: Some(TreeRef::Expr(x)), kind: PendingKind::Named(name), targs: vec![ty] });
                    View::TypeApply(fun, vec![ty])
                }
                TExpr::SeqLit(l) | TExpr::ArrayLit(l) => {
                    let items: Vec<TreeRef> = self.prog().expr_list(l).iter().map(|&x| TreeRef::Expr(x)).collect();
                    let elem = self.seq_elem_type(e);
                    View::Repeated(items, elem)
                }
                TExpr::Spread(x) => View::Typed(TreeRef::Expr(x), self.prog().type_of(e).unwrap_or(ANY)),
                TExpr::Index(x, _) | TExpr::JsSelect(x, _) => View::Select(TreeRef::Expr(x), SymRef::None, self.typer.interner.intern("apply")),
                TExpr::Lambda(params, body) => View::Lambda(self.prog().sym_list(params).to_vec(), TreeRef::Expr(body)),
                TExpr::If(c, t, els) => {
                    let els = match els {
                        Some(x) => TreeRef::Expr(x),
                        None => TreeRef::Expr(self.unit_expr()),
                    };
                    View::If(TreeRef::Expr(c), TreeRef::Expr(t), els)
                }
                TExpr::While(c, b) => View::While(TreeRef::Expr(c), TreeRef::Expr(b)),
                TExpr::Block(stmts, res) => {
                    let items: Vec<TreeRef> = (stmts.start..stmts.start + stmts.len).map(TreeRef::Stmt).collect();
                    View::Block(items, TreeRef::Expr(res))
                }
                TExpr::Assign(l, r) => View::Assign(TreeRef::Expr(l), TreeRef::Expr(r)),
                TExpr::Match(scrut, cases) => {
                    let items: Vec<TreeRef> = (cases.start..cases.start + cases.len).map(TreeRef::Case).collect();
                    View::Match(TreeRef::Expr(scrut), items)
                }
                TExpr::Return(x) => View::Return(TreeRef::Expr(x)),
                TExpr::Throw(x, _) => {
                    let name = self.typer.interner.intern("throw");
                    let fun = self.pending(Pending { recv: None, kind: PendingKind::Named(name), targs: Vec::new() });
                    View::Apply(fun, vec![TreeRef::Expr(x)])
                }
                TExpr::Try(i) => {
                    let (body, cases, finalizer) = {
                        let tr = &self.prog().tries[i as usize];
                        (tr.body, tr.cases, tr.finalizer)
                    };
                    let items: Vec<TreeRef> = (cases.start..cases.start + cases.len).map(TreeRef::Case).collect();
                    View::Try(TreeRef::Expr(body), items, finalizer.map(TreeRef::Expr))
                }
                TExpr::Splice(_) => return self.unsupported("a splice of an inline body typed at its definition is no tree of an expansion"),
                TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::ObjLit(_) => View::Ident(SymRef::None),
            },
            TreeRef::Stmt(i) => match self.prog().stmts[i as usize] {
                TStmt::Expr(x) => return self.view(TreeRef::Expr(x)),
                TStmt::Val(v, init) => View::ValDef(v, Some(TreeRef::Expr(init))),
                TStmt::Fun(f) => return self.view(TreeRef::Fun(f)),
                TStmt::Pat(p, init) => {
                    let binder = self.pattern_binder(p);
                    match binder {
                        Some(v) => View::ValDef(v, Some(TreeRef::Expr(init))),
                        None => return self.view(TreeRef::Expr(init)),
                    }
                }
            },
            TreeRef::Fun(f) => {
                let (sym, params, body) = {
                    let fun = &self.prog().funs[f.idx()];
                    (fun.sym, fun.params.clone(), fun.body)
                };
                let clauses = self.param_clauses(sym, &params);
                View::DefDef(sym, clauses, body.map(TreeRef::Expr))
            }
            TreeRef::Class(c) => View::ClassDef(c),
            TreeRef::Def(s) => {
                let kind = self.syms().sym(s).kind;
                match kind {
                    SymKind::Def | SymKind::Given if kind == SymKind::Def || self.takes_arguments(s) => {
                        self.typer.ensure_body(s);
                        let f = self.typer.fun_of_sym.get(&s).copied();
                        match f {
                            Some(f) => return self.view(TreeRef::Fun(f)),
                            None => {
                                let sig = self.typer.sig_of(s);
                                let params: Vec<SymId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
                                let clauses = self.param_clauses(s, &params);
                                View::DefDef(s, clauses, None)
                            }
                        }
                    }
                    SymKind::Object(_) => View::ValDef(s, None),
                    _ => {
                        let init = self.val_init_of(s);
                        View::ValDef(s, init.map(TreeRef::Expr))
                    }
                }
            }
            TreeRef::Case(i) => {
                let case = self.prog().cases[i as usize];
                View::CaseDef(TreeRef::Pat(case.pat), case.guard.map(TreeRef::Expr), TreeRef::Expr(case.body))
            }
            TreeRef::Pat(p) => match self.prog().pats[p.idx()] {
                TPat::Wildcard => View::Wildcard,
                TPat::Bind(s, inner) => match inner {
                    Some(i) => View::Bind(s, TreeRef::Pat(i)),
                    None => View::Bind(s, TreeRef::Pat(self.wildcard_pat())),
                },
                TPat::Test(_, ty, inner) => View::TypedPattern(TreeRef::Pat(inner), ty),
                TPat::Equals(e, _) => return self.view(TreeRef::Expr(e)),
                TPat::Class(_, _, _, subs) => View::Unapply(self.prog().pat_lists[subs.range()].iter().map(|&x| TreeRef::Pat(x)).collect()),
                TPat::Alt(subs) => View::Alternatives(self.prog().pat_lists[subs.range()].iter().map(|&x| TreeRef::Pat(x)).collect()),
                TPat::Seq(items, rest) => {
                    let mut subs: Vec<TreeRef> = self.prog().pat_lists[items.range()].iter().map(|&x| TreeRef::Pat(x)).collect();
                    if let Some(r) = rest {
                        subs.push(TreeRef::Pat(r));
                    }
                    View::Unapply(subs)
                }
                TPat::Unapply(_, _, inner) => View::Unapply(vec![TreeRef::Pat(inner)]),
            },
            TreeRef::Type(ty) => View::TypeTree(ty),
            TreeRef::LambdaDef(e) => {
                let TExpr::Lambda(params, body) = self.prog().expr(e) else { return self.unsupported("a closure without a lambda") };
                let ps: Vec<SymId> = self.prog().sym_list(params).to_vec();
                View::DefDef(SymId(u32::MAX), vec![ps.into_iter().map(SymRef::Term).collect()], Some(TreeRef::Expr(body)))
            }
            TreeRef::LambdaClosure(e) => View::Closure(TreeRef::LambdaRef(e)),
            TreeRef::LambdaRef(_) => View::Ident(SymRef::None),
            TreeRef::Ctor(c) => {
                let clauses = self.param_symss(SymRef::Ctor(c));
                View::DefDef(SymId(u32::MAX), clauses, None)
            }
            TreeRef::Alias(a) => View::TypeDef(SymRef::Alias(a)),
            TreeRef::Default(c, i) => {
                let init = self.default_init(c, i)?;
                View::DefDef(SymId(u32::MAX), Vec::new(), Some(TreeRef::Expr(init)))
            }
            TreeRef::NamedArg(i) => {
                let (name, arg) = self.named_arg_of(i)?;
                View::NamedArg(name, arg)
            }
            TreeRef::Partial(i) => {
                let (fun, args) = self.partial_of(i)?;
                View::Apply(fun, args)
            }
            TreeRef::Pending(i) => {
                let p = self.pending_of(i)?;
                if !p.targs.is_empty() {
                    let bare = self.pending(Pending { targs: Vec::new(), ..p.clone() });
                    return Ok(View::TypeApply(bare, p.targs));
                }
                match (p.recv, p.kind) {
                    (Some(r), PendingKind::Sym(s)) => View::Select(r, SymRef::Term(s), self.syms().sym(s).name),
                    (Some(r), PendingKind::Named(n) | PendingKind::Operator(n, _)) => View::Select(r, SymRef::None, n),
                    (None, PendingKind::Sym(s)) => View::Ident(SymRef::Term(s)),
                    (None, PendingKind::Named(n) | PendingKind::Operator(n, _)) => {
                        let _ = n;
                        View::Ident(SymRef::None)
                    }
                    (_, PendingKind::New(ty)) => View::New(ty),
                    (_, PendingKind::Ctor(ty)) => {
                        let inner = self.pending(Pending { recv: None, kind: PendingKind::New(ty), targs: Vec::new() });
                        View::Select(inner, self.ctor_sym(ty), self.typer.interner.intern("<init>"))
                    }
                    (_, PendingKind::Ref(r)) => View::Ident(r),
                }
            }
        })
    }

    /// The std's `->`, which scalac spells through `ArrowAssoc`.
    fn is_arrow(&mut self, s: SymId) -> bool {
        let arrow = self.typer.interner.intern("->");
        let info = self.syms().sym(s);
        info.name == arrow && info.is_extension && self.typer.source(info.file).is_std
    }

    /// The abstract var whose setter `s` is, which an assignment through it calls.
    fn assigned_var(&self, s: SymId) -> Option<SymId> {
        crate::typer::setters::abstract_var_of_setter(self.syms(), self.typer.interner, s)
    }

    fn expr_type_or_any(&mut self, e: TExprId) -> TypeId {
        self.prog().type_of(e).unwrap_or(ANY)
    }

    fn concat_view(&mut self, items: &[TExprId]) -> View {
        let n = items.len();
        let plus = self.typer.interner.intern("+");
        let left = if n <= 2 {
            TreeRef::Expr(items[0])
        } else {
            let l = self.typer.prog.list(&items[..n - 1]);
            let e = self.typer.prog.add(TExpr::StrConcat(l));
            let s = self.typer.b.t_string;
            self.typer.prog.set_type(e, s);
            TreeRef::Expr(e)
        };
        // `String.+(x$0: Any)`, or a number's `+(x: String)` where the chain starts with one.
        let lty = match left {
            TreeRef::Expr(l) => self.operand_type(l),
            _ => self.typer.b.t_string,
        };
        let (name, param) = if self.typer.is_numeric(lty).is_some() { ("x", self.typer.b.t_string) } else { ("x$0", ANY) };
        let names = vec![self.typer.interner.intern(name)];
        let result = self.typer.b.t_string;
        let method = self.method_type(MethodTypeRepr { names, types: vec![param], result, open: Vec::new(), kind: ClauseKind::Plain });
        let fun = self.pending(Pending { recv: Some(left), kind: PendingKind::Operator(plus, method), targs: Vec::new() });
        View::Apply(fun, vec![TreeRef::Expr(items[n - 1])])
    }

    /// The tree `e` as the end of a chain of `+`, where it is a `String`: what an ascription or
    /// a selection that the typer writes as its operand makes of it. The mark goes on a node of
    /// its own, so that the tree the macro holds stays what it was where the macro uses it
    /// again; a call a quote left for its site to expand expands as its copy.
    fn chain_end(&mut self, e: TExprId, ty: TypeId) -> TExprId {
        if !self.typer.is_string(ty) {
            return e;
        }
        let node = self.prog().expr(e);
        let id = self.typer.prog.add(node);
        if let Some(ty) = self.prog().type_of(e) {
            self.typer.prog.set_type(id, ty);
        }
        self.typer.prog.copy_span(e, id);
        self.typer.prog.copy_chain_marks(e, id);
        self.typer.copy_deferred(e, id);
        self.typer.prog.mark_chain_end(id);
        id
    }

    fn unit_expr(&mut self) -> TExprId {
        let e = self.typer.prog.add(TExpr::Unit);
        let u = self.typer.b.t_unit;
        self.typer.prog.set_type(e, u);
        e
    }

    fn wildcard_pat(&mut self) -> TPatId {
        self.typer.prog.add_pat(TPat::Wildcard)
    }

    fn pattern_binder(&self, p: TPatId) -> Option<SymId> {
        match self.prog().pats[p.idx()] {
            TPat::Bind(s, _) => Some(s),
            TPat::Test(_, _, inner) => self.pattern_binder(inner),
            _ => None,
        }
    }

    fn module_sym(&self, c: ClassId) -> SymRef {
        match self.syms().class(c).module_sym {
            Some(s) => SymRef::Term(s),
            None => SymRef::Class(c),
        }
    }

    /// The companion of a case class that declares none: scalac synthesizes one holding the
    /// default getters; teq keeps those on the class, which stands for it.
    fn implied_companion(&self, c: ClassId) -> SymRef {
        if self.syms().class(c).mods & crate::ast::mods::CASE != 0 {
            SymRef::Class(c)
        } else {
            SymRef::None
        }
    }

    fn ctor_sym(&mut self, ty: TypeId) -> SymRef {
        match self.typer.class_of(ty) {
            Some(c) => SymRef::Ctor(c),
            None => SymRef::None,
        }
    }

    /// The parameter clauses of a method as symbols: the type parameters first when it has any.
    fn param_clauses(&mut self, sym: SymId, params: &[SymId]) -> Vec<Vec<SymRef>> {
        let sig = self.typer.sig_of(sym);
        let mut out = Vec::new();
        if !sig.tparams.is_empty() {
            out.push(sig.tparams.iter().map(|&p| SymRef::TParam(p)).collect());
        }
        let mut i = 0;
        for clause in &sig.clauses {
            let n = clause.params.len();
            let syms: Vec<SymRef> = params.iter().skip(i).take(n).map(|&s| SymRef::Term(s)).collect();
            i += n;
            out.push(syms);
        }
        out
    }

    fn val_init_of(&mut self, s: SymId) -> Option<TExprId> {
        self.typer.ensure_body(s);
        if let Some(&e) = self.typer.val_init.get(&s) {
            crate::typer::bundle::entered_at(&self.typer.prog.exprs, e.0, crate::typer::bundle::Entry::Val);
            return Some(e);
        }
        if let Owner::Class(c) = self.syms().sym(s).owner {
            let idx = self.tclass_index(c)?;
            for init in &self.prog().classes[idx].init {
                if let TInit::Field(f, e) = init {
                    if *f == s {
                        return Some(*e);
                    }
                }
            }
        }
        None
    }

    fn test_type(&mut self, test: TestId) -> TypeId {
        match self.prog().tests[test.idx()] {
            TypeTest::Class(c) | TypeTest::Trait(c) => {
                let n = self.syms().class(c).tparams.len();
                let args = vec![ANY; n];
                self.typer.types.class(c, &args)
            }
            TypeTest::Int => self.typer.b.t_int,
            TypeTest::Number => self.typer.b.t_double,
            TypeTest::Long => self.typer.b.t_long,
            TypeTest::Str => self.typer.b.t_string,
            TypeTest::Bool => self.typer.b.t_boolean,
            TypeTest::Char => self.typer.b.t_char,
            TypeTest::Unit => self.typer.b.t_unit,
            TypeTest::Byte => self.typer.b.t_byte,
            TypeTest::Short => self.typer.b.t_short,
            TypeTest::Float => self.typer.b.t_float,
            TypeTest::AnyRef => self.typer.b.t_any_ref,
            TypeTest::AnyVal => self.typer.b.t_any_val,
            TypeTest::Null => self.typer.b.t_null,
            _ => ANY,
        }
    }

    fn seq_elem_type(&mut self, e: TExprId) -> TypeId {
        let Some(t) = self.prog().type_of(e) else { return ANY };
        let t = self.typer.dealias(t);
        match self.typer.types.get(t) {
            Type::Class(_, args) => self.typer.types.items(args).first().copied().unwrap_or(ANY),
            _ => ANY,
        }
    }

    /// A tree as an expression of the IR: a pending selection becomes the call it stands for.
    pub(super) fn materialize(&mut self, t: TreeRef) -> R<TExprId> {
        match t {
            TreeRef::Expr(e) => Ok(e),
            TreeRef::Stmt(i) => match self.prog().stmts[i as usize] {
                TStmt::Expr(e) => Ok(e),
                _ => self.unsupported("a definition where a term was expected"),
            },
            TreeRef::Pending(i) => {
                let p = self.pending_of(i)?;
                self.apply_pending(p, Vec::new(), false)
            }
            TreeRef::NamedArg(i) => {
                let (_, arg) = self.named_arg_of(i)?;
                self.materialize(arg)
            }
            TreeRef::Type(ty) => {
                let shown = self.show_type_repr(ty);
                self.unsupported(format!("a type tree where a term was expected: {}", shown))
            }
            // A closure evaluates to the function its method makes: the lambda.
            TreeRef::LambdaClosure(e) => Ok(e),
            other => {
                let kind = self.tree_kind(other)?;
                self.unsupported(format!("a {} where a term was expected", kind))
            }
        }
    }

    /// The application of a pending selection to arguments; without an `Apply` around it a
    /// method that takes none is called with none.
    /// A named argument among the trees of an application: its parameter's name and the
    /// argument.
    fn named_arg(&self, t: TreeRef) -> R<Option<(Name, TreeRef)>> {
        match t {
            TreeRef::NamedArg(i) => self.named_arg_of(i).map(Some),
            _ => Ok(None),
        }
    }

    /// The arguments of a call to `s` in the order of its first parameter clause, where some
    /// are named.
    fn ordered_args(&mut self, s: SymId, args: &[TreeRef]) -> R<Vec<TExprId>> {
        let mut any_named = false;
        for &a in args {
            any_named |= self.named_arg(a)?.is_some();
        }
        if !any_named {
            return args.iter().map(|&a| self.materialize(a)).collect();
        }
        let params: Vec<Name> = self.typer.sig_of(s).clauses.first().map_or(Vec::new(), |c| c.params.iter().map(|p| p.name).collect());
        let mut slots: Vec<Option<TreeRef>> = vec![None; params.len()];
        let mut next = 0;
        for &a in args {
            match self.named_arg(a)? {
                Some((name, arg)) => match params.iter().position(|&p| p == name) {
                    Some(i) => slots[i] = Some(arg),
                    None => {
                        let shown = self.typer.name_ref(name).to_string();
                        return self.unsupported(format!("no parameter {} to name in the call", shown));
                    }
                },
                None => {
                    while next < slots.len() && slots[next].is_some() {
                        next += 1;
                    }
                    if next >= slots.len() {
                        return self.unsupported("more arguments than the method takes");
                    }
                    slots[next] = Some(a);
                }
            }
        }
        let mut out = Vec::with_capacity(slots.len());
        for (slot, name) in slots.into_iter().zip(params) {
            match slot {
                Some(t) => out.push(self.materialize(t)?),
                None => {
                    let shown = self.typer.name_ref(name).to_string();
                    return self.unsupported(format!("the parameter {} gets no argument", shown));
                }
            }
        }
        Ok(out)
    }

    fn apply_pending(&mut self, p: Pending, args: Vec<TreeRef>, applied: bool) -> R<TExprId> {
        let span = self.macro_ctx.as_ref().map_or(Span::default(), |c| c.site_span);
        // A member with a template (`Product.productPrefix`) is applied as the typer applies
        // it, by name, so that the call becomes the template and not a method of the receiver.
        let p = match p.kind {
            PendingKind::Sym(s) if p.recv.is_some() && self.syms().sym(s).intrinsic.is_some() => {
                let name = self.syms().sym(s).name;
                Pending { kind: PendingKind::Named(name), ..p }
            }
            PendingKind::Operator(name, _) => Pending { kind: PendingKind::Named(name), ..p },
            _ => p,
        };
        match p.kind {
            PendingKind::Sym(s) => {
                let recv = match p.recv {
                    Some(r) => Some(self.materialize(r)?),
                    None => None,
                };
                let (kind, owner, is_ext, name) = {
                    let info = self.syms().sym(s);
                    (info.kind, info.owner, info.is_extension, info.name)
                };
                let takes = self.takes_arguments(s);
                if !applied && takes {
                    let shown = self.typer.name_ref(name).to_string();
                    return self.unsupported(format!("the method {} is selected without its arguments", shown));
                }
                let mut args = self.ordered_args(s, &args)?;
                // A by-name parameter takes the thunk of the expression a tree holds for it.
                let by_name: Vec<bool> = self.typer.sig_of(s).clauses.iter().flat_map(|c| c.params.iter().map(|p| p.by_name)).collect();
                for (a, _) in args.iter_mut().zip(&by_name).filter(|(_, &b)| b) {
                    *a = self.typer.by_name_thunk(*a);
                }
                let l = self.typer.prog.list(&args);
                let te = match (recv, kind, owner) {
                    (Some(r), SymKind::Val | SymKind::Var | SymKind::Param, _) => self.typer.prog.add(TExpr::Field(r, s)),
                    (Some(r), SymKind::Object(c), _) => {
                        let _ = r;
                        self.typer.prog.add(TExpr::Module(c))
                    }
                    (Some(r), _, _) if !is_ext => self.typer.prog.add(TExpr::CallMethod(r, s, l)),
                    (Some(r), _, _) => {
                        let mut all = vec![r];
                        all.extend(args.iter().copied());
                        let l = self.typer.prog.list(&all);
                        self.typer.prog.add(TExpr::CallStatic(s, l))
                    }
                    (None, SymKind::Object(c), _) => self.typer.prog.add(TExpr::Module(c)),
                    // An enum case is a static of its own, as a reference in source is typed.
                    (None, SymKind::EnumValue(_), _) => self.typer.prog.add(TExpr::Static(s)),
                    (None, SymKind::Val | SymKind::Var, Owner::Class(c)) if self.syms().class(c).kind == ClassKind::Object => {
                        let m = self.typer.prog.add(TExpr::Module(c));
                        self.typer.prog.add(TExpr::Field(m, s))
                    }
                    (None, SymKind::Val | SymKind::Var | SymKind::Param, Owner::Local) => self.typer.prog.add(TExpr::Local(s)),
                    (None, SymKind::Val | SymKind::Var, _) => self.typer.prog.add(TExpr::Static(s)),
                    (None, _, Owner::Class(c)) if !is_ext => {
                        let m = self.typer.prog.add(TExpr::Module(c));
                        self.typer.prog.add(TExpr::CallMethod(m, s, l))
                    }
                    (None, _, _) => self.typer.prog.add(TExpr::CallStatic(s, l)),
                };
                let ty = self.result_type(s, recv, &args, &p.targs);
                self.typer.prog.set_type(te, ty);
                if self.typer.capturing() && !p.targs.is_empty() {
                    self.typer.capture_call_targs(te, &p.targs);
                }
                Ok(te)
            }
            PendingKind::Named(name) | PendingKind::Operator(name, _) => {
                let Some(r) = p.recv else {
                    return self.unsupported(format!("a reference to {} without a definition", self.typer.name_ref(name)));
                };
                // `Companion.apply(args)` of a case class that declares no companion: the
                // constructor call it stands for.
                if let TreeRef::Pending(i) = r {
                    if let PendingKind::Ref(SymRef::Class(c)) = self.pending_of(i)?.kind {
                        if name == crate::names::APPLY && self.syms().class(c).kind != ClassKind::Object {
                            let args: Vec<TExprId> = args.into_iter().map(|a| self.materialize(a)).collect::<R<_>>()?;
                            return self.new_instance_tree(c, &p.targs, args);
                        }
                    }
                }
                let recv = self.materialize(r)?;
                let recv_ty = self.tree_type(TreeRef::Expr(recv))?;
                let mut lists = Vec::new();
                if applied {
                    let mut srcs = Vec::new();
                    for &a in &args {
                        let (name, a) = match self.named_arg(a)? {
                            Some((name, arg)) => (Some(name), arg),
                            None => (None, a),
                        };
                        let e = self.materialize(a)?;
                        let ty = self.tree_type(TreeRef::Expr(e))?;
                        srcs.push(match name {
                            Some(n) => ArgSrc::Named(n, e, ty),
                            None => ArgSrc::Typed(e, ty),
                        });
                    }
                    lists.push(ArgList { args: srcs, using: false, span });
                }
                // `!b` rebuilt by a macro: the builtin's prefix operator.
                if lists.is_empty() {
                    if let Some((te, ty)) = self.typer.prim_op_on(recv, recv_ty, name, None, &[], span, None) {
                        self.typer.prog.set_type(te, ty);
                        return Ok(te);
                    }
                }
                // `n * 2` rebuilt by a macro: the builtin operator, as the typer reads it.
                if let [ArgList { args, .. }] = lists.as_slice() {
                    if let [ArgSrc::Typed(tr, rty)] = args.as_slice() {
                        if name == crate::names::EQEQ || name == crate::names::NEQ {
                            let (te, ty) = self.typer.typed_equality(recv, recv_ty, *tr, *rty, name == crate::names::NEQ, span);
                            self.typer.prog.set_type(te, ty);
                            return Ok(te);
                        }
                        if let Some((te, ty)) = self.typer.typed_binop(name, recv, recv_ty, *tr, *rty, span) {
                            self.typer.prog.set_type(te, ty);
                            return Ok(te);
                        }
                    }
                }
                let mark = self.typer.diags.items.len();
                self.typer.macro_targs = (!p.targs.is_empty()).then(|| (name, p.targs.clone()));
                let (te, ty) = self.typer.apply_member(recv, recv_ty, name, None, lists, span, None);
                self.typer.macro_targs = None;
                if self.typer.diags.items.len() > mark {
                    let msgs: Vec<String> = self.typer.diags.items.drain(mark..).map(|d| d.msg).collect();
                    let shown_name = self.typer.name_ref(name).to_string();
                    let shown_ty = self.show_type_repr(recv_ty);
                    return self.unsupported(format!("the selection of {} on {} does not type: {}", shown_name, shown_ty, msgs.join("; ")));
                }
                // A selection the typer wrote as its receiver (`toString` of a string) ends a
                // chain of `+` as the call it is to scalac.
                if te == recv {
                    return Ok(self.chain_end(te, ty));
                }
                // A member the compiler synthesizes (`values` of an enum's companion) is a
                // template whose type only the application records.
                if self.prog().type_of(te).is_none() {
                    self.typer.prog.set_type(te, ty);
                }
                Ok(te)
            }
            PendingKind::Ref(r) => {
                let shown = self.symbol_name(r);
                self.unsupported(format!("a reference to {} where a term was expected", shown))
            }
            PendingKind::New(ty) | PendingKind::Ctor(ty) => {
                let Some(c) = self.typer.class_of(ty) else {
                    let shown = self.show_type_repr(ty);
                    return self.unsupported(format!("new of {}, which is no class", shown));
                };
                let args: Vec<TExprId> = args.into_iter().map(|a| self.materialize(a)).collect::<R<_>>()?;
                let l = self.typer.prog.list(&args);
                let te = self.typer.new_instance(c, l, crate::source::Span::default());
                self.typer.prog.set_type(te, ty);
                Ok(te)
            }
        }
    }

    /// A call as scalac's trees show it: the type arguments the typer inferred restored on a
    /// `TypeApply`, and one `Apply` per parameter clause, the first innermost.
    fn call_view(&mut self, recv: Option<TreeRef>, recv_expr: Option<TExprId>, s: SymId, exprs: &[TExprId], items: Vec<TreeRef>, result: Option<TypeId>) -> View {
        let targs = if self.typer.sig_of(s).tparams.is_empty() { Vec::new() } else { self.inferred_targs(s, recv_expr, exprs, result) };
        let items = self.by_name_args_unwrapped(s, items);
        let mut fun = self.pending(Pending { recv, kind: PendingKind::Sym(s), targs });
        let sizes: Vec<usize> = self.typer.sig_of(s).clauses.iter().map(|c| c.params.len()).collect();
        if sizes.len() < 2 || sizes.iter().sum::<usize>() != items.len() {
            return View::Apply(fun, items);
        }
        let mut rest = items;
        for &n in &sizes[..sizes.len() - 1] {
            let clause: Vec<TreeRef> = rest.drain(..n).collect();
            fun = TreeRef::Partial(self.records.partials.push((fun, clause)));
        }
        View::Apply(fun, rest)
    }

    /// The arguments of by-name parameters as scalac's trees hold them: the expression, where
    /// the typer passes a thunk of it.
    fn by_name_args_unwrapped(&mut self, s: SymId, items: Vec<TreeRef>) -> Vec<TreeRef> {
        let by_name: Vec<bool> = self.typer.sig_of(s).clauses.iter().flat_map(|c| c.params.iter().map(|p| p.by_name)).collect();
        items
            .into_iter()
            .enumerate()
            .map(|(i, t)| match t {
                TreeRef::Expr(e) if by_name.get(i) == Some(&true) => match self.prog().expr(e) {
                    TExpr::Lambda(params, body) if params.len == 0 => TreeRef::Expr(body),
                    _ => t,
                },
                _ => t,
            })
            .collect()
    }

    /// The type arguments of a call the typer inferred, read back from the arguments' types
    /// and the result's, in the order of the method's type parameters; one nothing fixes is
    /// `Any`.
    fn inferred_targs(&mut self, s: SymId, recv: Option<TExprId>, args: &[TExprId], result: Option<TypeId>) -> Vec<TypeId> {
        let sig = self.typer.sig_arc(s);
        let mut subst: Subst = Vec::new();
        if let (Some(r), Owner::Class(owner)) = (recv, self.syms().sym(s).owner) {
            if let Some(rty) = self.prog().type_of(r) {
                if let Some(base) = self.typer.base_type(rty, owner) {
                    if let Type::Class(_, bargs) = self.typer.types.get(base) {
                        let items = self.typer.types.items(bargs).to_vec();
                        let tparams = self.syms().class(owner).tparams.clone();
                        subst.extend(tparams.into_iter().zip(items));
                    }
                }
            }
        }
        let params: Vec<(TypeId, TExprId)> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).zip(args.iter().copied()).collect();
        for (pty, a) in params {
            if let Some(aty) = self.prog().type_of(a) {
                self.infer_simple(pty, aty, &sig.tparams, &mut subst);
            }
        }
        if let Some(r) = result {
            self.infer_simple(sig.ret, r, &sig.tparams, &mut subst);
        }
        sig.tparams.iter().map(|&tp| subst.iter().find(|&&(q, _)| q == tp).map_or(ANY, |&(_, t)| self.typer.zonk(t))).collect()
    }

    /// `new C[targs](args)`, the type arguments inferred from the arguments where none are given.
    fn new_instance_tree(&mut self, c: ClassId, targs: &[TypeId], args: Vec<TExprId>) -> R<TExprId> {
        self.typer.complete_class(c);
        let tparams = self.syms().class(c).tparams.clone();
        let targs: Vec<TypeId> = if targs.len() == tparams.len() {
            targs.to_vec()
        } else {
            let mut subst: Subst = Vec::new();
            let params: Vec<TypeId> = self.syms().class(c).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.ty)).collect();
            for (pty, &a) in params.iter().zip(&args) {
                if let Some(aty) = self.prog().type_of(a) {
                    self.infer_simple(*pty, aty, &tparams, &mut subst);
                }
            }
            tparams.iter().map(|&tp| subst.iter().find(|&&(q, _)| q == tp).map_or(ANY, |&(_, t)| t)).collect()
        };
        let ty = self.typer.types.class(c, &targs);
        let l = self.typer.prog.list(&args);
        // A class nested in a class takes the enclosing instance the expansion site holds, as
        // a `new` written there does (a derived codec's `new Response(..)` in `object Response`
        // of a class).
        let te = self.typer.new_instance(c, l, crate::source::Span::default());
        self.typer.prog.set_type(te, ty);
        if self.typer.capturing() && !targs.is_empty() {
            self.typer.capture_call_targs(te, &targs);
        }
        Ok(te)
    }

    /// The result type of a call: the method's declared result with the class's type parameters
    /// taken from the receiver and its own from the type arguments, or from the arguments'
    /// types where none were given.
    fn result_type(&mut self, s: SymId, recv: Option<TExprId>, args: &[TExprId], targs: &[TypeId]) -> TypeId {
        let sig = self.typer.sig_arc(s);
        let mut subst: Subst = Vec::new();
        if let (Some(r), Owner::Class(owner)) = (recv, self.syms().sym(s).owner) {
            if let Some(rty) = self.prog().type_of(r) {
                if let Some(base) = self.typer.base_type(rty, owner) {
                    if let Type::Class(_, bargs) = self.typer.types.get(base) {
                        let items = self.typer.types.items(bargs).to_vec();
                        let tparams = self.syms().class(owner).tparams.clone();
                        subst.extend(tparams.into_iter().zip(items));
                    }
                }
            }
        }
        if !sig.tparams.is_empty() {
            if targs.len() == sig.tparams.len() {
                subst.extend(sig.tparams.iter().copied().zip(targs.iter().copied()));
            } else {
                let params: Vec<(TypeId, TExprId)> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).zip(args.iter().copied()).collect();
                for (pty, a) in params {
                    if let Some(aty) = self.prog().type_of(a) {
                        self.infer_simple(pty, aty, &sig.tparams, &mut subst);
                    }
                }
            }
        }
        let ret = self.typer.types.subst(sig.ret, &subst);
        self.typer.zonk(ret)
    }

    fn infer_simple(&mut self, p: TypeId, a: TypeId, tparams: &[TParamId], subst: &mut Subst) {
        match self.typer.types.get(p) {
            Type::Param(tp) if tparams.contains(&tp) => {
                if !subst.iter().any(|&(q, _)| q == tp) {
                    let a = self.typer.widen_lit(a);
                    subst.push((tp, a));
                }
            }
            Type::Class(c, pargs) => {
                let a = self.typer.dealias(a);
                if let Some(base) = self.typer.base_type(a, c) {
                    if let Type::Class(_, aargs) = self.typer.types.get(base) {
                        let (xs, ys) = (self.typer.types.items(pargs).to_vec(), self.typer.types.items(aargs).to_vec());
                        for (x, y) in xs.into_iter().zip(ys) {
                            self.infer_simple(x, y, tparams, subst);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// The static type of a tree.
    pub(super) fn tree_type(&mut self, t: TreeRef) -> R<TypeId> {
        Ok(match t {
            TreeRef::Expr(e) => {
                // A stable val's reference has its singleton type, as scalac's `Ident` has,
                // which `widen` takes to the declared type.
                if let TExpr::Local(s) | TExpr::Static(s) = self.prog().expr(e) {
                    if matches!(self.syms().sym(s).kind, SymKind::Val | SymKind::Param) && self.typer.sig_of(s).tparams.is_empty() {
                        let declared = self.typer.sig_of(s).ret;
                        if self.prog().type_of(e).map_or(true, |ty| self.typer.zonk(ty) == declared) {
                            return Ok(self.typer.types.mk(Type::Term(s)));
                        }
                    }
                }
                if let Some(ty) = self.prog().type_of(e) {
                    return Ok(self.typer.zonk(ty));
                }
                match self.prog().expr(e) {
                    TExpr::Int(_) => self.typer.b.t_int,
                    TExpr::Long(_) => self.typer.b.t_long,
                    TExpr::Double(_) => self.typer.b.t_double,
                    TExpr::Bool(_) => self.typer.b.t_boolean,
                    TExpr::Char(_) => self.typer.b.t_char,
                    TExpr::Str(_) => self.typer.b.t_string,
                    TExpr::Unit => self.typer.b.t_unit,
                    TExpr::Null => self.typer.b.t_null,
                    TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) => self.typer.sig_of(s).ret,
                    TExpr::Module(c) => self.typer.types.class(c, &[]),
                    TExpr::CallStatic(s, args) => {
                        let items = self.prog().expr_list(args).to_vec();
                        self.result_type(s, None, &items, &[])
                    }
                    TExpr::CallMethod(r, s, args) => {
                        let items = self.prog().expr_list(args).to_vec();
                        self.result_type(s, Some(r), &items, &[])
                    }
                    TExpr::New(c, _) => {
                        let n = self.syms().class(c).tparams.len();
                        self.typer.types.class(c, &vec![ANY; n])
                    }
                    TExpr::StrConcat(_) | TExpr::ToStr(..) => self.typer.b.t_string,
                    TExpr::TypeTest(..) => self.typer.b.t_boolean,
                    TExpr::Cast(_, _, ty) => ty,
                    TExpr::Block(_, r) => return self.tree_type(TreeRef::Expr(r)),
                    _ => ANY,
                }
            }
            TreeRef::Pending(i) => {
                let p = self.pending_of(i)?;
                match p.kind {
                    PendingKind::Sym(s) => {
                        let recv = match p.recv {
                            Some(r) => Some(self.materialize(r)?),
                            None => None,
                        };
                        self.result_type(s, recv, &[], &p.targs)
                    }
                    PendingKind::New(ty) | PendingKind::Ctor(ty) => ty,
                    PendingKind::Ref(SymRef::Class(c)) => self.typer.types.class(c, &[]),
                    PendingKind::Ref(_) => ANY,
                    PendingKind::Named(_) => ANY,
                    PendingKind::Operator(_, method) => method,
                }
            }
            TreeRef::Def(s) => self.typer.sig_of(s).ret,
            TreeRef::Type(ty) => ty,
            TreeRef::Class(c) => self.typer.types.class(c, &[]),
            TreeRef::LambdaDef(e) | TreeRef::LambdaClosure(e) | TreeRef::LambdaRef(e) => return self.tree_type(TreeRef::Expr(e)),
            TreeRef::NamedArg(i) => return self.tree_type(self.named_arg_of(i)?.1),
            TreeRef::Default(c, i) => {
                let init = self.default_init(c, i)?;
                return self.tree_type(TreeRef::Expr(init));
            }
            TreeRef::Alias(a) => self.type_def_rhs(SymRef::Alias(a)),
            _ => ANY,
        })
    }

    // ---- kinds ----

    /// The name of the reflect API class a tree shows as.
    pub(super) fn tree_kind(&mut self, t: TreeRef) -> R<&'static str> {
        Ok(match t {
            TreeRef::Expr(e) => match self.prog().expr(e) {
                TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null | TExpr::ClassOf(_) => "Literal",
                TExpr::Local(_) | TExpr::Static(_) | TExpr::Module(_) | TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::ObjLit(_) => "Ident",
                TExpr::This => "This",
                TExpr::Super(_) => "Super",
                // A member of `this` named unqualified, which the view shows as an `Ident`.
                TExpr::Field(r, _) if matches!(self.prog().expr(r), TExpr::This) => "Ident",
                TExpr::Field(..) | TExpr::Unary(..) | TExpr::ToStr(..) | TExpr::Index(..) | TExpr::JsSelect(..) => "Select",
                TExpr::CallMethod(_, s, args) if args.len == 1 && self.assigned_var(s).is_some() => "Assign",
                TExpr::CallMethod(_, s, args) | TExpr::CallStatic(s, args) => {
                    if args.is_empty() && !self.takes_arguments(s) {
                        if matches!(self.prog().expr(e), TExpr::CallMethod(..)) { "Select" } else { "Ident" }
                    } else {
                        "Apply"
                    }
                }
                TExpr::Js(s, args) => match self.sym_of_template(s) {
                    Some(sym) => {
                        let info = self.syms().sym(sym);
                        let member = matches!(info.owner, Owner::Class(_)) && !info.is_extension;
                        let n = args.len as usize - member as usize;
                        if n == 0 && !self.takes_arguments(sym) {
                            if member { "Select" } else { "Ident" }
                        } else {
                            "Apply"
                        }
                    }
                    None => "Apply",
                },
                TExpr::CallClosure(..) | TExpr::New(..) | TExpr::NewVia(..) | TExpr::Prim(..) | TExpr::StrConcat(_) | TExpr::Throw(..) => "Apply",
                TExpr::TypeTest(..) | TExpr::Cast(..) => "TypeApply",
                TExpr::SeqLit(_) | TExpr::ArrayLit(_) => "Repeated",
                TExpr::Spread(_) => "Typed",
                TExpr::Lambda(..) | TExpr::Block(..) => "Block",
                TExpr::If(..) => "If",
                TExpr::While(..) => "While",
                TExpr::Assign(..) => "Assign",
                TExpr::Match(..) => "Match",
                TExpr::Return(_) => "Return",
                TExpr::Try(_) => "Try",
                TExpr::Splice(_) => return self.unsupported("a splice of an inline body typed at its definition is no tree of an expansion"),
            },
            TreeRef::Stmt(i) => match self.prog().stmts[i as usize] {
                TStmt::Expr(x) => return self.tree_kind(TreeRef::Expr(x)),
                TStmt::Val(..) | TStmt::Pat(..) => "ValDef",
                TStmt::Fun(_) => "DefDef",
            },
            TreeRef::Fun(_) => "DefDef",
            TreeRef::Class(_) => "ClassDef",
            TreeRef::Def(s) => {
                let kind = self.syms().sym(s).kind;
                match kind {
                    SymKind::Def => "DefDef",
                    SymKind::Given if self.takes_arguments(s) => "DefDef",
                    _ => "ValDef",
                }
            }
            TreeRef::Case(_) => "CaseDef",
            TreeRef::Pat(p) => match self.prog().pats[p.idx()] {
                TPat::Wildcard => "Wildcard",
                TPat::Bind(..) => "Bind",
                TPat::Test(..) => "Typed",
                TPat::Equals(e, _) => return self.tree_kind(TreeRef::Expr(e)),
                TPat::Class(..) | TPat::Seq(..) | TPat::Unapply(..) => "Unapply",
                TPat::Alt(_) => "Alternatives",
            },
            TreeRef::Type(_) => "Inferred",
            TreeRef::LambdaDef(_) | TreeRef::Ctor(_) | TreeRef::Default(..) => "DefDef",
            TreeRef::LambdaClosure(_) => "Closure",
            TreeRef::LambdaRef(_) => "Ident",
            TreeRef::Alias(_) => "TypeDef",
            TreeRef::NamedArg(_) => "NamedArg",
            TreeRef::Partial(_) => "Apply",
            TreeRef::Pending(i) => {
                let p = self.pending_of(i)?;
                if !p.targs.is_empty() {
                    "TypeApply"
                } else {
                    match (p.recv, p.kind) {
                        (_, PendingKind::New(_)) => "New",
                        (_, PendingKind::Ctor(_)) | (Some(_), _) => "Select",
                        (None, _) => "Ident",
                    }
                }
            }
        })
    }

    pub(super) fn tree_is_instance(&mut self, t: TreeRef, c: ClassId) -> bool {
        let Ok(kind) = self.tree_kind(t) else { return false };
        let Some(k) = self.reflect_class(kind) else { return false };
        self.typer.complete_class(k);
        self.is_subclass(k, c)
    }

    fn type_repr_kind(&mut self, t: TypeId) -> &'static str {
        let t = self.typer.zonk(t);
        match self.typer.types.get(t) {
            Type::Class(c, _) if Some(c) == self.typer.b.by_name => "ByNameType",
            Type::Class(c, args) => {
                if args != EMPTY_LIST {
                    "AppliedType"
                } else if self.syms().class(c).kind == ClassKind::Object {
                    "TermRef"
                } else {
                    "TypeRef"
                }
            }
            Type::Union(..) => "OrType",
            Type::Inter(..) => "AndType",
            Type::Lit(_) => "ConstantType",
            Type::Wild | Type::BoundedWild(..) => "TypeBounds",
            Type::This(_) => "ThisType",
            Type::Term(_) | Type::Select(..) => "TermRef",
            Type::Refined(..) => "Refinement",
            Type::Match(..) => "MatchType",
            Type::Lambda(..) => "TypeLambda",
            Type::Param(p) if self.lambda_of.contains_key(&p) => "ParamRef",
            Type::AppMember(..) => "AppliedType",
            Type::Alias(_, args) if args != EMPTY_LIST => "AppliedType",
            Type::Blocked(_) if self.pkg_form(t).is_some() => match self.pkg_form(t).unwrap().0 {
                PkgForm::Path | PkgForm::Sym => "TermRef",
                PkgForm::ThisType => "ThisType",
                PkgForm::ClassRef | PkgForm::SymClassRef => "TypeRef",
            },
            Type::Blocked(b) if self.typer.types.blocked_description(b) == "<noprefix>" => "NoPrefix",
            Type::Blocked(b) if self.typer.types.blocked_description(b).starts_with("<method>") => "MethodType",
            Type::Blocked(b) if self.typer.types.blocked_description(b).starts_with("<poly>") => "PolyType",
            _ => "TypeRef",
        }
    }

    pub(super) fn type_repr_is_instance(&mut self, t: TypeId, c: ClassId) -> bool {
        // `case o: Constant` on a `TypeRepr` has no `TypeTest` in scalac and erases to a test
        // that every value passes.
        if self.reflect_class("Constant") == Some(c) {
            return true;
        }
        let kind = self.type_repr_kind(t);
        let Some(k) = self.reflect_class(kind) else { return false };
        self.typer.complete_class(k);
        self.is_subclass(k, c)
    }
}

pub(crate) fn prim_name(op: PrimOp) -> &'static str {
    use PrimOp::*;
    match op {
        IntAdd | LongAdd | DoubleAdd | FloatAdd => "+",
        IntSub | LongSub | DoubleSub | FloatSub => "-",
        IntMul | LongMul | DoubleMul | FloatMul => "*",
        IntDiv | LongDiv | DoubleDiv | FloatDiv => "/",
        IntRem | LongRem | DoubleRem | FloatRem => "%",
        IntAnd | LongAnd | BoolStrictAnd => "&",
        IntOr | LongOr | BoolStrictOr => "|",
        IntXor | LongXor | BoolXor => "^",
        IntShl | LongShl => "<<",
        IntShr | LongShr => ">>",
        IntUshr | LongUshr => ">>>",
        Lt => "<",
        Le => "<=",
        Gt => ">",
        Ge => ">=",
        RefEq => "eq",
        RefNe => "ne",
        Eq => "==",
        Ne => "!=",
        BoolAnd => "&&",
        BoolOr => "||",
    }
}

pub(crate) fn unary_name(op: UnOp) -> &'static str {
    use UnOp::*;
    match op {
        IntNeg | LongNeg | DoubleNeg | FloatNeg => "unary_-",
        BoolNot => "unary_!",
        IntNot | LongNot => "unary_~",
        IntToLong | CharToLong | FloatToLong | DoubleToLong => "toLong",
        LongToDouble | FloatToDouble | IntToDouble => "toDouble",
        LongToInt | DoubleToInt | CharToInt | FloatToInt | ByteToInt | ShortToInt => "toInt",
        IntToChar => "toChar",
        IntToByte => "toByte",
        IntToShort | ByteToShort => "toShort",
        IntToFloat | LongToFloat | DoubleToFloat => "toFloat",
    }
}

// ---- trees ----

impl<'a, 't> Interp<'a, 't> {
    pub(super) fn literal_expr(&mut self, v: &Value) -> R<TExprId> {
        let (te, ty) = match v {
            Value::Int(i) => (TExpr::Int(*i), self.typer.b.t_int),
            Value::Byte(b) => (TExpr::Int(*b as i32), self.typer.b.t_byte),
            Value::Short(s) => (TExpr::Int(*s as i32), self.typer.b.t_short),
            Value::Long(l) => (TExpr::Long(*l), self.typer.b.t_long),
            Value::Double(d) => (TExpr::Double(*d), self.typer.b.t_double),
            Value::Float(f) => (TExpr::Double(*f as f64), self.typer.b.t_float),
            Value::Bool(b) => (TExpr::Bool(*b), self.typer.b.t_boolean),
            Value::Char(c) => (TExpr::Char(*c), self.typer.b.t_char),
            Value::Str(s) => {
                let r = self.typer.prog.add_str(s);
                (TExpr::Str(r), self.typer.b.t_string)
            }
            Value::Unit => (TExpr::Unit, self.typer.b.t_unit),
            Value::Null => (TExpr::Null, self.typer.b.t_null),
            Value::Type(t) => {
                let span = self.macro_ctx.as_ref().map_or(Span::default(), |c| c.site_span);
                let (te, ty) = self.typer.type_class_of(*t, span);
                self.typer.prog.set_type(te, ty);
                return Ok(te);
            }
            other => {
                let shown = self.to_str(other)?;
                return self.unsupported(format!("no literal holds {}", shown));
            }
        };
        let e = self.typer.prog.add(te);
        self.typer.prog.set_type(e, ty);
        Ok(e)
    }

    fn literal_value(&mut self, t: TreeRef) -> R<Option<Value>> {
        match self.view(t)? {
            View::Literal(v) => Ok(Some(v)),
            _ => Ok(None),
        }
    }

    fn tree_symbol(&mut self, t: TreeRef) -> R<SymRef> {
        Ok(match t {
            TreeRef::Expr(e) => match self.prog().expr(e) {
                TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) | TExpr::CallMethod(_, s, _) | TExpr::CallStatic(s, _) => SymRef::Term(s),
                TExpr::Module(c) => self.module_sym(c),
                TExpr::New(c, _) => SymRef::Ctor(c),
                TExpr::Js(s, _) => self.sym_of_template(s).map_or(SymRef::None, SymRef::Term),
                TExpr::Lambda(..) => SymRef::Lambda(e),
                TExpr::This => self.macro_ctx.as_ref().and_then(|c| c.owner_class).map_or(SymRef::None, SymRef::Class),
                _ => SymRef::None,
            },
            TreeRef::Stmt(i) => match self.prog().stmts[i as usize] {
                TStmt::Val(v, _) => SymRef::Term(v),
                TStmt::Fun(f) => SymRef::Term(self.prog().funs[f.idx()].sym),
                TStmt::Pat(p, _) => self.pattern_binder(p).map_or(SymRef::None, SymRef::Term),
                TStmt::Expr(x) => return self.tree_symbol(TreeRef::Expr(x)),
            },
            TreeRef::Fun(f) => SymRef::Term(self.prog().funs[f.idx()].sym),
            TreeRef::Class(c) => SymRef::Class(c),
            TreeRef::Def(s) => SymRef::Term(s),
            TreeRef::Pat(p) => match self.prog().pats[p.idx()] {
                TPat::Bind(s, _) => SymRef::Term(s),
                _ => SymRef::None,
            },
            TreeRef::Type(ty) => self.type_symbol(ty),
            TreeRef::Pending(i) => match self.pending_of(i)?.kind {
                PendingKind::Sym(s) => SymRef::Term(s),
                PendingKind::New(ty) | PendingKind::Ctor(ty) => self.ctor_sym(ty),
                PendingKind::Named(_) | PendingKind::Operator(..) => SymRef::None,
                PendingKind::Ref(r) => r,
            },
            TreeRef::Case(_) | TreeRef::NamedArg(_) | TreeRef::Partial(_) => SymRef::None,
            TreeRef::LambdaDef(e) | TreeRef::LambdaClosure(e) | TreeRef::LambdaRef(e) => SymRef::Lambda(e),
            TreeRef::Ctor(c) => SymRef::Ctor(c),
            TreeRef::Alias(a) => SymRef::Alias(a),
            TreeRef::Default(c, i) => SymRef::Default(c, i),
        })
    }

    fn apply_tree(&mut self, fun: TreeRef, args: Vec<TreeRef>) -> R<TExprId> {
        match fun {
            TreeRef::Pending(i) => {
                let p = self.pending_of(i)?;
                self.apply_pending(p, args, true)
            }
            other => {
                let f = self.materialize(other)?;
                let fty = self.tree_type(TreeRef::Expr(f))?;
                let args: Vec<TExprId> = args.into_iter().map(|a| self.materialize(a)).collect::<R<_>>()?;
                let l = self.typer.prog.list(&args);
                let te = self.typer.prog.add(TExpr::CallClosure(f, l));
                let ret = self.typer.as_function(fty).map_or(ANY, |(_, r)| r);
                self.typer.prog.set_type(te, ret);
                Ok(te)
            }
        }
    }

    /// `Select(qualifier, sym)`: a member reached through a receiver.
    fn select_tree(&mut self, qual: TreeRef, sym: SymRef) -> R<TreeRef> {
        let s = match sym {
            SymRef::Term(s) => s,
            SymRef::Ctor(c) => {
                let recv = self.materialize(qual)?;
                let ty = self.tree_type(TreeRef::Expr(recv))?;
                let _ = c;
                return Ok(self.pending(Pending { recv: None, kind: PendingKind::Ctor(ty), targs: Vec::new() }));
            }
            SymRef::Default(c, i) => {
                let init = self.default_init(c, i)?;
                return Ok(TreeRef::Expr(init));
            }
            other => {
                let shown = self.symbol_name(other);
                return self.unsupported(format!("a selection of {}, which is no term", shown));
            }
        };
        if let TreeRef::Pending(i) = qual {
            if let PendingKind::New(ty) = self.pending_of(i)?.kind {
                return Ok(self.pending(Pending { recv: None, kind: PendingKind::Ctor(ty), targs: Vec::new() }));
            }
        }
        let pending = self.pending(Pending { recv: Some(qual), kind: PendingKind::Sym(s), targs: Vec::new() });
        if self.takes_arguments(s) {
            Ok(pending)
        } else {
            Ok(TreeRef::Expr(self.materialize(pending)?))
        }
    }

    fn ref_tree(&mut self, sym: SymRef) -> R<TreeRef> {
        match sym {
            SymRef::Term(s) => {
                let pending = self.pending(Pending { recv: None, kind: PendingKind::Sym(s), targs: Vec::new() });
                if self.takes_arguments(s) {
                    Ok(pending)
                } else {
                    Ok(TreeRef::Expr(self.materialize(pending)?))
                }
            }
            SymRef::Class(c) if self.syms().class(c).kind == ClassKind::Object => {
                let te = self.typer.prog.add(TExpr::Module(c));
                let ty = self.typer.types.class(c, &[]);
                self.typer.prog.set_type(te, ty);
                Ok(TreeRef::Expr(te))
            }
            SymRef::Default(c, i) => {
                let init = self.default_init(c, i)?;
                Ok(TreeRef::Expr(init))
            }
            // A case class standing for its implied companion: the qualifier of a default
            // getter's selection, which needs no term.
            SymRef::Class(c) if self.syms().class(c).mods & crate::ast::mods::CASE != 0 => {
                Ok(self.pending(Pending { recv: None, kind: PendingKind::Ref(sym), targs: Vec::new() }))
            }
            other => {
                let shown = self.symbol_name(other);
                self.unsupported(format!("Ref of {}, which is no term", shown))
            }
        }
    }

    fn default_init(&mut self, c: ClassId, i: u32) -> R<TExprId> {
        self.typer.complete_class(c);
        let idx = self.tclass_index(c);
        let found = idx.and_then(|k| self.prog().classes[k].ctor_defaults.get(i as usize).copied().flatten());
        match found {
            Some(e) => Ok(e),
            None => {
                let name = self.typer.name_ref(self.syms().class(c).name).to_string();
                self.unsupported(format!("the default of parameter {} of {} is not available", i + 1, name))
            }
        }
    }

    fn opt_tree(&mut self, t: Option<TreeRef>) -> R {
        self.make_option(t.map(Value::Tree))
    }

    fn opt_term_arg(&mut self, a: A, i: usize) -> R<Option<TExprId>> {
        let v = arg(a, i);
        match self.option_item(&v)? {
            Some(Value::Tree(t)) => Ok(Some(self.materialize(t)?)),
            _ => Ok(None),
        }
    }

    fn type_tree_arg(&mut self, a: A, i: usize) -> R<TypeId> {
        match a.get(i) {
            Some(Value::Tree(TreeRef::Type(t))) => Ok(*t),
            Some(Value::Type(t)) => Ok(*t),
            Some(Value::Tree(t)) => {
                let t = *t;
                self.tree_type(t)
            }
            _ => self.unsupported(format!("argument {} is no type tree", i)),
        }
    }

    fn block_tree(&mut self, stats: Vec<TreeRef>, expr: TreeRef) -> R<TreeRef> {
        if let [TreeRef::LambdaDef(e)] = stats[..] {
            return Ok(TreeRef::Expr(e));
        }
        let mut items: Vec<TStmt> = Vec::new();
        for s in stats {
            items.push(match s {
                TreeRef::Stmt(i) => self.prog().stmts[i as usize],
                TreeRef::Fun(f) => TStmt::Fun(f),
                TreeRef::Def(v) => match self.val_init_of(v) {
                    Some(init) => TStmt::Val(v, init),
                    None => return self.unsupported("a definition without a right-hand side in a block"),
                },
                other => TStmt::Expr(self.materialize(other)?),
            });
        }
        let res = self.materialize(expr)?;
        let ty = self.tree_type(TreeRef::Expr(res))?;
        let l = self.typer.prog.stmts.push_slice(&items);
        let te = self.typer.prog.add(TExpr::Block(l, res));
        self.typer.prog.set_type(te, ty);
        Ok(TreeRef::Expr(te))
    }

    fn case_tree(&mut self, pat: TreeRef, guard: Option<TExprId>, body: TExprId) -> R<TreeRef> {
        let TreeRef::Pat(p) = pat else { return self.unsupported("a case whose pattern is no pattern tree") };
        self.typer.prog.cases.push(TCase { pat: p, guard, body });
        Ok(TreeRef::Case(self.typer.prog.cases.len() as u32 - 1))
    }

    fn cases_of(&mut self, v: &Value) -> R<Vec<u32>> {
        let trees = self.tree_list(v)?;
        let mut out = Vec::new();
        for t in trees {
            match t {
                TreeRef::Case(i) => out.push(i),
                _ => return self.unsupported("a list of cases holds something else"),
            }
        }
        Ok(out)
    }

    fn case_list(&mut self, indices: &[u32]) -> crate::ast::ListRef {
        let items: Vec<TCase> = indices.iter().map(|&i| self.prog().cases[i as usize]).collect();
        self.typer.prog.cases.push_slice(&items)
    }
}

fn install_trees(it: &mut Table) {
    q!(it, "Reflect.TreeMethods.Tree.pos", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let ctx = it.macro_ctx()?;
        let pos = match t {
            TreeRef::Def(s) => {
                let info = it.syms().sym(s);
                Value::Pos(info.file, info.span.start, info.span.end)
            }
            TreeRef::Class(c) => {
                let info = it.syms().class(c);
                Value::Pos(info.file, info.span.start, info.span.end)
            }
            TreeRef::Expr(e) => match it.prog().span_of(e) {
                Some((f, s)) => Value::Pos(f, s.start, s.end),
                None => Value::Pos(ctx.site_file, ctx.site_span.start, ctx.site_span.end),
            },
            _ => Value::Pos(ctx.site_file, ctx.site_span.start, ctx.site_span.end),
        };
        Ok(pos)
    });
    q!(it, "Reflect.TreeMethods.Tree.symbol", |it, a| {
        let t = it.tree_arg(a, 0)?;
        Ok(Value::Sym(it.tree_symbol(t)?))
    });
    q!(it, "Reflect.TreeMethods.Tree.changeOwner", |_it, a| Ok(arg(a, 0)));
    q!(it, "Reflect.DefinitionMethods.Definition.name", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let s = it.tree_symbol(t)?;
        Ok(Value::string(it.symbol_name(s)))
    });
    q!(it, "Reflect.TermMethods.Term.tpe", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.selection_method_type(t)? {
            Some(method) => Ok(Value::Type(method)),
            None => Ok(Value::Type(it.tree_type(t)?)),
        }
    });
    q!(it, "Reflect.TermMethods.Term.underlyingArgument", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let mut e = it.materialize(t)?;
        if let Some(ctx) = it.macro_ctx.clone() {
            while let TExpr::Local(s) = it.prog().expr(e) {
                match ctx.params.get(&s) {
                    Some(&arg) if arg != e => e = arg,
                    _ => break,
                }
            }
        }
        Ok(Value::Tree(TreeRef::Expr(e)))
    });
    q!(it, "Reflect.TermMethods.Term.underlying", |it, a| {
        let t = it.tree_arg(a, 0)?;
        Ok(Value::Tree(TreeRef::Expr(it.materialize(t)?)))
    });
    q!(it, "Reflect.TermMethods.Term.etaExpand", |_it, a| Ok(arg(a, 0)));
    q!(it, "Reflect.Term.betaReduce", |it, a| {
        let t = it.tree_arg(a, 1)?;
        let e = it.materialize(t)?;
        let reduced = match it.prog().expr(e) {
            TExpr::CallClosure(f, args) => match it.prog().expr(f) {
                TExpr::Lambda(params, body) => {
                    let (ps, xs) = (it.prog().sym_list(params).to_vec(), it.prog().expr_list(args).to_vec());
                    if ps.len() != xs.len() {
                        None
                    } else {
                        let stmts: Vec<TStmt> = ps.iter().zip(&xs).map(|(&p, &x)| TStmt::Val(p, x)).collect();
                        let l = it.typer.prog.stmts.push_slice(&stmts);
                        let te = it.typer.prog.add(TExpr::Block(l, body));
                        let ty = it.tree_type(TreeRef::Expr(body))?;
                        it.typer.prog.set_type(te, ty);
                        Some(te)
                    }
                }
                _ => None,
            },
            _ => None,
        };
        it.make_option(reduced.map(|e| Value::Tree(TreeRef::Expr(e))))
    });

    // Literal
    q!(it, "Reflect.Literal.apply", |it, a| {
        let c = it.constant_value(&arg(a, 1))?;
        Ok(Value::Tree(TreeRef::Expr(it.literal_expr(&c)?)))
    });
    q!(it, "Reflect.Literal.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.literal_value(t)? {
            Some(v) => {
                let c = it.make_constant(v)?;
                it.make_some(c)
            }
            None => it.unsupported("Literal.unapply on a tree that is no literal"),
        }
    });
    q!(it, "Reflect.LiteralMethods.Literal.constant", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.literal_value(t)? {
            Some(v) => it.make_constant(v),
            None => it.unsupported("constant of a tree that is no literal"),
        }
    });

    // Ident, Ref, Select
    q!(it, "Reflect.Ident.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        let s = it.tree_symbol(t)?;
        let name = match (s, it.view(t)?) {
            (SymRef::None, View::Select(_, _, n)) => it.typer.name_ref(n).to_string(),
            _ => it.symbol_name(s),
        };
        it.make_some(Value::string(name))
    });
    q!(it, "Reflect.IdentMethods.Ident.name", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let s = it.tree_symbol(t)?;
        Ok(Value::string(it.symbol_name(s)))
    });
    q!(it, "Reflect.Ident.copy", |_it, a| Ok(arg(a, 1)));
    q!(it, "Reflect.Wildcard.apply", |it, _a| Ok(Value::Tree(TreeRef::Pat(it.wildcard_pat()))));
    q!(it, "Reflect.Ref.apply", |it, a| {
        let s = it.sym_arg(a, 1)?;
        Ok(Value::Tree(it.ref_tree(s)?))
    });
    q!(it, "Reflect.Ref.term", |it, a| {
        let t = it.type_arg(a, 1)?;
        let s = it.term_symbol(t);
        Ok(Value::Tree(it.ref_tree(s)?))
    });
    q!(it, "Reflect.Select.apply", |it, a| {
        let q = it.tree_arg(a, 1)?;
        let s = it.sym_arg(a, 2)?;
        Ok(Value::Tree(it.select_tree(q, s)?))
    });
    q!(it, "Reflect.Select.unique", |it, a| {
        let q = it.tree_arg(a, 1)?;
        let name = it.str_arg(a, 2)?;
        let n = it.typer.interner.intern(&name);
        let recv = it.materialize(q)?;
        let ty = it.tree_type(TreeRef::Expr(recv))?;
        match it.typer.find_member(ty, n) {
            Some((s, _)) if !matches!(it.syms().sym(s).kind, SymKind::Overloaded(_)) => Ok(Value::Tree(it.select_tree(TreeRef::Expr(recv), SymRef::Term(s))?)),
            _ => {
                let p = it.pending(Pending { recv: Some(TreeRef::Expr(recv)), kind: PendingKind::Named(n), targs: Vec::new() });
                Ok(Value::Tree(p))
            }
        }
    });
    q!(it, "Reflect.Select.overloaded", |it, a| {
        let q = it.tree_arg(a, 1)?;
        let name = it.str_arg(a, 2)?;
        let n = it.typer.interner.intern(&name);
        let targs = it.type_list(&arg(a, 3))?;
        let args = it.tree_list(&arg(a, 4))?;
        let p = Pending { recv: Some(q), kind: PendingKind::Named(n), targs };
        Ok(Value::Tree(TreeRef::Expr(it.apply_pending(p, args, true)?)))
    });
    q!(it, "Reflect.Select.copy", |it, a| {
        let q = it.tree_arg(a, 2)?;
        let name = it.str_arg(a, 3)?;
        let n = it.typer.interner.intern(&name);
        let original = it.tree_arg(a, 1)?;
        match it.view(original)? {
            View::Select(_, SymRef::Term(s), _) => Ok(Value::Tree(it.select_tree(q, SymRef::Term(s))?)),
            // `Select(New(tpt), <init>)` again over the `New` a `TreeMap` made anew.
            View::Select(_, SymRef::Ctor(_), _) if name.as_ref() == "<init>" => match it.view(q)? {
                View::New(ty) => Ok(Value::Tree(it.pending(Pending { recv: None, kind: PendingKind::Ctor(ty), targs: Vec::new() }))),
                _ => it.unsupported("a constructor selected on a tree that is no new"),
            },
            _ => {
                let recv = it.materialize(q)?;
                // The operator selected again under its name keeps the alternative's type.
                let kind = match original {
                    TreeRef::Pending(i) => match it.pending_of(i)?.kind {
                        PendingKind::Operator(was, method) if was == n => PendingKind::Operator(n, method),
                        _ => PendingKind::Named(n),
                    },
                    _ => PendingKind::Named(n),
                };
                let p = it.pending(Pending { recv: Some(TreeRef::Expr(recv)), kind, targs: Vec::new() });
                Ok(Value::Tree(p))
            }
        }
    });
    q!(it, "Reflect.Select.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Select(q, _, n) => {
                let name = it.typer.name_ref(n).to_string();
                it.make_tuple(vec![Value::Tree(q), Value::string(name)])
            }
            _ => it.unsupported("Select.unapply on a tree that is no selection"),
        }
    });
    q!(it, "Reflect.SelectMethods.Select.qualifier", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Select(q, _, _) => Ok(Value::Tree(q)),
            _ => it.unsupported("qualifier of a tree that is no selection"),
        }
    });
    q!(it, "Reflect.SelectMethods.Select.name", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Select(_, _, n) => Ok(Value::string(it.typer.name_ref(n).to_string())),
            _ => it.unsupported("name of a tree that is no selection"),
        }
    });
    q!(it, "Reflect.SelectMethods.Select.signature", |it, _a| it.make_none());

    // Apply, TypeApply, New
    q!(it, "Reflect.Apply.apply", |it, a| {
        let fun = it.tree_arg(a, 1)?;
        let args = it.tree_list(&arg(a, 2))?;
        Ok(Value::Tree(TreeRef::Expr(it.apply_tree(fun, args)?)))
    });
    q!(it, "Reflect.Apply.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Apply(f, args) => {
                let list = it.tree_values(args)?;
                it.make_tuple(vec![Value::Tree(f), list])
            }
            _ => it.unsupported("Apply.unapply on a tree that is no application"),
        }
    });
    q!(it, "Reflect.ApplyMethods.Apply.fun", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Apply(f, _) => Ok(Value::Tree(f)),
            _ => it.unsupported("fun of a tree that is no application"),
        }
    });
    q!(it, "Reflect.ApplyMethods.Apply.args", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Apply(_, args) => it.tree_values(args),
            _ => it.unsupported("args of a tree that is no application"),
        }
    });
    q!(it, "Reflect.TypeApply.apply", |it, a| {
        let fun = it.tree_arg(a, 1)?;
        let targs = it.type_list(&arg(a, 2))?;
        match fun {
            TreeRef::Pending(i) => {
                let mut p = it.pending_of(i)?;
                p.targs = targs;
                Ok(Value::Tree(it.pending(p)))
            }
            other => Ok(Value::Tree(other)),
        }
    });
    q!(it, "Reflect.TypeApply.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::TypeApply(f, targs) => {
                let list = it.make_list(targs.into_iter().map(|t| Value::Tree(TreeRef::Type(t))).collect())?;
                it.make_tuple(vec![Value::Tree(f), list])
            }
            _ => it.unsupported("TypeApply.unapply on a tree that is no type application"),
        }
    });
    q!(it, "Reflect.TypeApplyMethods.TypeApply.fun", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::TypeApply(f, _) => Ok(Value::Tree(f)),
            _ => it.unsupported("fun of a tree that is no type application"),
        }
    });
    q!(it, "Reflect.TypeApplyMethods.TypeApply.args", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::TypeApply(_, targs) => it.make_list(targs.into_iter().map(|t| Value::Tree(TreeRef::Type(t))).collect()),
            _ => it.unsupported("args of a tree that is no type application"),
        }
    });
    q!(it, "Reflect.New.apply", |it, a| {
        let ty = it.type_tree_arg(a, 1)?;
        Ok(Value::Tree(it.pending(Pending { recv: None, kind: PendingKind::New(ty), targs: Vec::new() })))
    });
    q!(it, "Reflect.New.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::New(ty) => it.make_some(Value::Tree(TreeRef::Type(ty))),
            _ => it.unsupported("New.unapply on a tree that is no new"),
        }
    });
    q!(it, "Reflect.NewMethods.New.tpt", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::New(ty) => Ok(Value::Tree(TreeRef::Type(ty))),
            _ => it.unsupported("tpt of a tree that is no new"),
        }
    });
    q!(it, "Reflect.This.apply", |it, a| {
        let s = it.sym_arg(a, 1)?;
        let te = match s {
            SymRef::Class(c) if it.syms().class(c).kind == ClassKind::Object => {
                let e = it.typer.prog.add(TExpr::Module(c));
                let ty = it.typer.types.class(c, &[]);
                it.typer.prog.set_type(e, ty);
                e
            }
            _ => it.typer.prog.add(TExpr::This),
        };
        Ok(Value::Tree(TreeRef::Expr(te)))
    });
    q!(it, "Reflect.This.unapply", |it, _a| {
        let none = it.make_none()?;
        it.make_some(none)
    });
    q!(it, "Reflect.ThisMethods.This.id", |it, _a| it.make_none());
    q!(it, "Reflect.Super.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        let e = it.materialize(t)?;
        let this = it.typer.prog.add(TExpr::This);
        let _ = e;
        let none = it.make_none()?;
        it.make_tuple(vec![Value::Tree(TreeRef::Expr(this)), none])
    });

    // Typed, Repeated, Inlined, NamedArg, Assign, Block, Closure, Lambda, If, Match, Try, Return, While
    q!(it, "Reflect.Typed.apply", |it, a| {
        // `Typed(Wildcard(), tpt)` in a pattern, as a 3.3 macro spells a type test: the
        // pattern `_: T`.
        if let Some(Value::Tree(TreeRef::Pat(p))) = a.get(1) {
            let p = *p;
            let ty = it.type_tree_arg(a, 2)?;
            let test = it.typer.test_for(ty, ANY, Span::default(), true);
            let np = it.typer.prog.add_pat(TPat::Test(test, ty, p));
            return Ok(Value::Tree(TreeRef::Pat(np)));
        }
        let e = it.expr_arg(a, 1)?;
        let ty = it.type_tree_arg(a, 2)?;
        Ok(Value::Tree(TreeRef::Expr(it.chain_end(e, ty))))
    });
    q!(it, "Reflect.Typed.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Typed(e, ty) => it.make_tuple(vec![Value::Tree(e), Value::Tree(TreeRef::Type(ty))]),
            _ => it.unsupported("Typed.unapply on a tree that is no ascription"),
        }
    });
    q!(it, "Reflect.TypedMethods.Typed.expr", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Typed(e, _) => Ok(Value::Tree(e)),
            _ => it.unsupported("expr of a tree that is no ascription"),
        }
    });
    q!(it, "Reflect.TypedMethods.Typed.tpt", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Typed(_, ty) => Ok(Value::Tree(TreeRef::Type(ty))),
            _ => it.unsupported("tpt of a tree that is no ascription"),
        }
    });
    q!(it, "Reflect.TypedOrTest.apply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        let ty = it.type_tree_arg(a, 2)?;
        match t {
            TreeRef::Pat(p) => {
                let span = Span::default();
                let test = it.typer.test_for(ty, ANY, span, true);
                let np = it.typer.prog.add_pat(TPat::Test(test, ty, p));
                Ok(Value::Tree(TreeRef::Pat(np)))
            }
            other => Ok(Value::Tree(other)),
        }
    });
    q!(it, "Reflect.TypedOrTest.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::TypedPattern(inner, ty) | View::Typed(inner, ty) => it.make_tuple(vec![Value::Tree(inner), Value::Tree(TreeRef::Type(ty))]),
            _ => it.unsupported("TypedOrTest.unapply on a tree that is no ascription"),
        }
    });
    q!(it, "Reflect.TypedOrTestMethods.TypedOrTest.tree", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::TypedPattern(inner, _) | View::Typed(inner, _) => Ok(Value::Tree(inner)),
            _ => it.unsupported("tree of a tree that is no ascription"),
        }
    });
    q!(it, "Reflect.TypedOrTestMethods.TypedOrTest.tpt", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::TypedPattern(_, ty) | View::Typed(_, ty) => Ok(Value::Tree(TreeRef::Type(ty))),
            _ => it.unsupported("tpt of a tree that is no ascription"),
        }
    });
    q!(it, "Reflect.Repeated.apply", |it, a| {
        let elems = it.expr_list(&arg(a, 1))?;
        let elem_ty = it.type_tree_arg(a, 2)?;
        let l = it.typer.prog.list(&elems);
        let te = it.typer.prog.add(TExpr::SeqLit(l));
        if it.typer.capturing() {
            it.typer.capture_form(te, crate::tir::capture::Form::Repeated(elem_ty));
        }
        let ty = match it.typer.seq_class() {
            Some(seq) => it.typer.types.class(seq, &[elem_ty]),
            None => ANY,
        };
        it.typer.prog.set_type(te, ty);
        Ok(Value::Tree(TreeRef::Expr(te)))
    });
    q!(it, "Reflect.Repeated.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Repeated(elems, ty) => {
                let list = it.tree_values(elems)?;
                it.make_tuple(vec![list, Value::Tree(TreeRef::Type(ty))])
            }
            _ => it.unsupported("Repeated.unapply on a tree that is no vararg"),
        }
    });
    q!(it, "Reflect.RepeatedMethods.Repeated.elems", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Repeated(elems, _) => it.tree_values(elems),
            _ => it.unsupported("elems of a tree that is no vararg"),
        }
    });
    q!(it, "Reflect.RepeatedMethods.Repeated.elemtpt", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Repeated(_, ty) => Ok(Value::Tree(TreeRef::Type(ty))),
            _ => it.unsupported("elemtpt of a tree that is no vararg"),
        }
    });
    q!(it, "Reflect.Inlined.apply", |it, a| {
        let e = it.expr_arg(a, 3)?;
        Ok(Value::Tree(TreeRef::Expr(e)))
    });
    // The trees of this compiler carry no `Inlined` nodes; the argument of a macro, which
    // scalac wraps in one, is taken apart once: as the reference to the proxy that holds it or
    // as the argument itself, each answered with the argument's tree.
    q!(it, "Reflect.Inlined.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        let e = it.materialize(t)?;
        let Some(ctx) = it.macro_ctx.clone() else { return it.unsupported("Inlined.unapply outside a macro") };
        let inner = match it.prog().expr(e) {
            TExpr::Local(s) if ctx.params.contains_key(&s) => Some(ctx.params[&s]),
            _ if ctx.params.values().any(|&root| root == e) => Some(e),
            _ => None,
        };
        let Some(inner) = inner else { return it.unsupported("Inlined.unapply on a tree that is no inlined argument") };
        let holes = FxMap::default();
        let params = FxMap::default();
        let copied = it.typer.instantiate_quote(inner, &holes, &params, &Vec::new(), &FxMap::default());
        let none = it.make_none()?;
        let nil = it.make_list(Vec::new())?;
        it.make_tuple(vec![none, nil, Value::Tree(TreeRef::Expr(copied))])
    });
    q!(it, "Reflect.NamedArg.apply", |it, a| {
        let name = it.str_arg(a, 1)?;
        let n = it.typer.interner.intern(&name);
        let t = it.tree_arg(a, 2)?;
        Ok(Value::Tree(TreeRef::NamedArg(it.records.named_args.push((n, t)))))
    });
    q!(it, "Reflect.NamedArg.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::NamedArg(n, arg) => {
                let name = it.typer.name_ref(n).to_string();
                it.make_tuple(vec![Value::string(name), Value::Tree(arg)])
            }
            _ => it.unsupported("NamedArg.unapply on a tree that is no named argument"),
        }
    });
    q!(it, "Reflect.NamedArgMethods.NamedArg.name", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::NamedArg(n, _) => Ok(Value::string(it.typer.name_ref(n).to_string())),
            _ => it.unsupported("name of a tree that is no named argument"),
        }
    });
    q!(it, "Reflect.NamedArgMethods.NamedArg.value", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::NamedArg(_, arg) => Ok(Value::Tree(arg)),
            _ => it.unsupported("value of a tree that is no named argument"),
        }
    });
    q!(it, "Reflect.Assign.apply", |it, a| {
        let l = it.expr_arg(a, 1)?;
        let r = it.expr_arg(a, 2)?;
        let te = it.typer.assignment(l, r);
        let u = it.typer.b.t_unit;
        it.typer.prog.set_type(te, u);
        Ok(Value::Tree(TreeRef::Expr(te)))
    });
    q!(it, "Reflect.Assign.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Assign(l, r) => it.make_tuple(vec![Value::Tree(l), Value::Tree(r)]),
            _ => it.unsupported("Assign.unapply on a tree that is no assignment"),
        }
    });
    q!(it, "Reflect.AssignMethods.Assign.lhs", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Assign(l, _) => Ok(Value::Tree(l)),
            _ => it.unsupported("lhs of a tree that is no assignment"),
        }
    });
    q!(it, "Reflect.AssignMethods.Assign.rhs", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Assign(_, r) => Ok(Value::Tree(r)),
            _ => it.unsupported("rhs of a tree that is no assignment"),
        }
    });
    q!(it, "Reflect.Block.apply", |it, a| {
        let stats = it.tree_list(&arg(a, 1))?;
        let expr = it.tree_arg(a, 2)?;
        Ok(Value::Tree(it.block_tree(stats, expr)?))
    });
    q!(it, "Reflect.Block.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Block(stats, expr) => {
                let list = it.tree_values(stats)?;
                it.make_tuple(vec![list, Value::Tree(expr)])
            }
            View::Lambda(_, _) => {
                let e = it.materialize(t)?;
                let list = it.make_list(vec![Value::Tree(TreeRef::LambdaDef(e))])?;
                it.make_tuple(vec![list, Value::Tree(TreeRef::LambdaClosure(e))])
            }
            _ => it.unsupported("Block.unapply on a tree that is no block"),
        }
    });
    q!(it, "Reflect.BlockMethods.Block.statements", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Block(stats, _) => it.tree_values(stats),
            View::Lambda(..) => {
                let e = it.materialize(t)?;
                it.make_list(vec![Value::Tree(TreeRef::LambdaDef(e))])
            }
            _ => it.unsupported("statements of a tree that is no block"),
        }
    });
    q!(it, "Reflect.BlockMethods.Block.expr", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Block(_, expr) => Ok(Value::Tree(expr)),
            View::Lambda(..) => {
                let e = it.materialize(t)?;
                Ok(Value::Tree(TreeRef::LambdaClosure(e)))
            }
            _ => it.unsupported("expr of a tree that is no block"),
        }
    });
    q!(it, "Reflect.Closure.apply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        Ok(Value::Tree(t))
    });
    q!(it, "Reflect.Closure.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match t {
            TreeRef::LambdaClosure(e) => {
                let none = it.make_none()?;
                it.make_tuple(vec![Value::Tree(TreeRef::LambdaRef(e)), none])
            }
            _ => it.unsupported("Closure.unapply on a tree that is no closure"),
        }
    });
    q!(it, "Reflect.ClosureMethods.Closure.meth", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match t {
            TreeRef::LambdaClosure(e) => Ok(Value::Tree(TreeRef::LambdaRef(e))),
            _ => it.unsupported("meth of a tree that is no closure"),
        }
    });
    q!(it, "Reflect.ClosureMethods.Closure.tpeOpt", |it, _a| it.make_none());
    q!(it, "Reflect.Lambda.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Lambda(params, body) => {
                let list = it.make_list(params.into_iter().map(|p| Value::Tree(TreeRef::Def(p))).collect())?;
                let tuple = it.make_tuple(vec![list, Value::Tree(body)])?;
                it.make_some(tuple)
            }
            _ => it.make_none(),
        }
    });
    q!(it, "Reflect.Lambda.apply", |it, a| {
        let mt = it.type_arg(a, 2)?;
        let f = arg(a, 3);
        let (param_tys, ret) = match it.typer.types.get(mt) {
            Type::Class(c, args) if it.typer.is_function_class(c) => {
                let items = it.typer.types.items(args).to_vec();
                let n = items.len() - 1;
                (items[..n].to_vec(), items[n])
            }
            _ => return it.unsupported("Lambda.apply takes the method type as a function type"),
        };
        let span = it.macro_ctx.as_ref().map_or(Span::default(), |c| c.site_span);
        let params: Vec<SymId> = param_tys.iter().map(|&t| it.typer.fresh_local("x", t, span)).collect();
        let owner = arg(a, 1);
        let refs: Vec<Value> = params
            .iter()
            .map(|&p| {
                let e = it.typer.prog.add(TExpr::Local(p));
                let ty = it.typer.sig_of(p).ret;
                it.typer.prog.set_type(e, ty);
                Value::Tree(TreeRef::Expr(e))
            })
            .collect();
        let refs = it.make_list(refs)?;
        let body = it.apply_value(f, vec![owner, refs])?;
        let Value::Tree(bt) = body else { return it.unsupported("the body of Lambda.apply is no tree") };
        let body = it.materialize(bt)?;
        let pl = it.typer.prog.syms(&params);
        let te = it.typer.prog.add(TExpr::Lambda(pl, body));
        let ty = it.typer.fun_type(&param_tys, ret);
        it.typer.prog.set_type(te, ty);
        Ok(Value::Tree(TreeRef::Expr(te)))
    });
    q!(it, "Reflect.If.apply", |it, a| {
        let c = it.expr_arg(a, 1)?;
        let t = it.expr_arg(a, 2)?;
        let e = it.expr_arg(a, 3)?;
        let te = it.typer.prog.add(TExpr::If(c, t, Some(e)));
        let (tt, et) = (it.tree_type(TreeRef::Expr(t))?, it.tree_type(TreeRef::Expr(e))?);
        let ty = if it.conforms(et, tt) { tt } else if it.conforms(tt, et) { et } else { it.typer.types.union(tt, et) };
        it.typer.prog.set_type(te, ty);
        it.typer.mark_taken_branch(c, te, ty);
        Ok(Value::Tree(TreeRef::Expr(te)))
    });
    q!(it, "Reflect.If.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::If(c, x, y) => it.make_tuple(vec![Value::Tree(c), Value::Tree(x), Value::Tree(y)]),
            _ => it.unsupported("If.unapply on a tree that is no conditional"),
        }
    });
    q!(it, "Reflect.IfMethods.If.cond", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::If(c, _, _) => Ok(Value::Tree(c)),
            _ => it.unsupported("cond of a tree that is no conditional"),
        }
    });
    q!(it, "Reflect.IfMethods.If.thenp", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::If(_, x, _) => Ok(Value::Tree(x)),
            _ => it.unsupported("thenp of a tree that is no conditional"),
        }
    });
    q!(it, "Reflect.IfMethods.If.elsep", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::If(_, _, y) => Ok(Value::Tree(y)),
            _ => it.unsupported("elsep of a tree that is no conditional"),
        }
    });
    q!(it, "Reflect.Match.apply", |it, a| {
        let scrut = it.expr_arg(a, 1)?;
        let cases = it.cases_of(&arg(a, 2))?;
        let l = it.case_list(&cases);
        let te = it.typer.prog.add(TExpr::Match(scrut, l));
        let ty = match cases.first() {
            Some(&c) => {
                let body = it.prog().cases[c as usize].body;
                it.tree_type(TreeRef::Expr(body))?
            }
            None => ANY,
        };
        it.typer.prog.set_type(te, ty);
        Ok(Value::Tree(TreeRef::Expr(te)))
    });
    q!(it, "Reflect.Match.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Match(s, cases) => {
                let list = it.tree_values(cases)?;
                it.make_tuple(vec![Value::Tree(s), list])
            }
            _ => it.unsupported("Match.unapply on a tree that is no match"),
        }
    });
    q!(it, "Reflect.MatchMethods.Match.scrutinee", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Match(s, _) => Ok(Value::Tree(s)),
            _ => it.unsupported("scrutinee of a tree that is no match"),
        }
    });
    q!(it, "Reflect.MatchMethods.Match.cases", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Match(_, cases) => it.tree_values(cases),
            _ => it.unsupported("cases of a tree that is no match"),
        }
    });
    q!(it, "Reflect.CaseDef.apply", |it, a| {
        let pat = it.tree_arg(a, 1)?;
        let guard = it.opt_term_arg(a, 2)?;
        let body = it.expr_arg(a, 3)?;
        Ok(Value::Tree(it.case_tree(pat, guard, body)?))
    });
    q!(it, "Reflect.CaseDef.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::CaseDef(p, g, b) => {
                let guard = it.opt_tree(g)?;
                it.make_tuple(vec![Value::Tree(p), guard, Value::Tree(b)])
            }
            _ => it.unsupported("CaseDef.unapply on a tree that is no case"),
        }
    });
    q!(it, "Reflect.CaseDefMethods.CaseDef.pattern", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::CaseDef(p, _, _) => Ok(Value::Tree(p)),
            _ => it.unsupported("pattern of a tree that is no case"),
        }
    });
    q!(it, "Reflect.CaseDefMethods.CaseDef.guard", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::CaseDef(_, g, _) => it.opt_tree(g),
            _ => it.unsupported("guard of a tree that is no case"),
        }
    });
    q!(it, "Reflect.CaseDefMethods.CaseDef.rhs", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::CaseDef(_, _, b) => Ok(Value::Tree(b)),
            _ => it.unsupported("rhs of a tree that is no case"),
        }
    });
    q!(it, "Reflect.Try.apply", |it, a| {
        let body = it.expr_arg(a, 1)?;
        let cases = it.cases_of(&arg(a, 2))?;
        let finalizer = it.opt_term_arg(a, 3)?;
        let l = it.case_list(&cases);
        it.typer.prog.tries.push(TTry { body, cases: l, finalizer, wraps: false });
        let te = it.typer.prog.add(TExpr::Try(it.typer.prog.tries.len() as u32 - 1));
        let ty = it.tree_type(TreeRef::Expr(body))?;
        it.typer.prog.set_type(te, ty);
        Ok(Value::Tree(TreeRef::Expr(te)))
    });
    q!(it, "Reflect.Try.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Try(b, cases, f) => {
                let list = it.tree_values(cases)?;
                let fin = it.opt_tree(f)?;
                it.make_tuple(vec![Value::Tree(b), list, fin])
            }
            _ => it.unsupported("Try.unapply on a tree that is no try"),
        }
    });
    q!(it, "Reflect.TryMethods.Try.body", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Try(b, _, _) => Ok(Value::Tree(b)),
            _ => it.unsupported("body of a tree that is no try"),
        }
    });
    q!(it, "Reflect.TryMethods.Try.cases", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Try(_, cases, _) => it.tree_values(cases),
            _ => it.unsupported("cases of a tree that is no try"),
        }
    });
    q!(it, "Reflect.TryMethods.Try.finalizer", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Try(_, _, f) => it.opt_tree(f),
            _ => it.unsupported("finalizer of a tree that is no try"),
        }
    });
    q!(it, "Reflect.Return.apply", |it, a| {
        let e = it.expr_arg(a, 1)?;
        let te = it.typer.prog.add(TExpr::Return(e));
        it.typer.prog.set_type(te, NOTHING);
        Ok(Value::Tree(TreeRef::Expr(te)))
    });
    q!(it, "Reflect.Return.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Return(e) => {
                let owner = it.innermost_site_sym().map_or(SymRef::None, SymRef::Term);
                it.make_tuple(vec![Value::Tree(e), Value::Sym(owner)])
            }
            _ => it.unsupported("Return.unapply on a tree that is no return"),
        }
    });
    q!(it, "Reflect.ReturnMethods.Return.expr", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Return(e) => Ok(Value::Tree(e)),
            _ => it.unsupported("expr of a tree that is no return"),
        }
    });
    q!(it, "Reflect.While.apply", |it, a| {
        let c = it.expr_arg(a, 1)?;
        let b = it.expr_arg(a, 2)?;
        let te = it.typer.prog.add(TExpr::While(c, b));
        let u = it.typer.b.t_unit;
        it.typer.prog.set_type(te, u);
        Ok(Value::Tree(TreeRef::Expr(te)))
    });
    q!(it, "Reflect.While.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::While(c, b) => it.make_tuple(vec![Value::Tree(c), Value::Tree(b)]),
            _ => it.unsupported("While.unapply on a tree that is no loop"),
        }
    });
    q!(it, "Reflect.WhileMethods.While.cond", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::While(c, _) => Ok(Value::Tree(c)),
            _ => it.unsupported("cond of a tree that is no loop"),
        }
    });
    q!(it, "Reflect.WhileMethods.While.body", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::While(_, b) => Ok(Value::Tree(b)),
            _ => it.unsupported("body of a tree that is no loop"),
        }
    });

    // definitions
    q!(it, "Reflect.ValDef.apply", |it, a| {
        let s = it.sym_arg(a, 1)?;
        let SymRef::Term(sym) = s else { return it.unsupported("ValDef.apply takes a term symbol") };
        match it.opt_term_arg(a, 2)? {
            Some(rhs) => {
                it.typer.prog.stmts.push(TStmt::Val(sym, rhs));
                Ok(Value::Tree(TreeRef::Stmt(it.typer.prog.stmts.len() as u32 - 1)))
            }
            None => Ok(Value::Tree(TreeRef::Def(sym))),
        }
    });
    q!(it, "Reflect.ValDef.copy", |it, a| {
        let original = it.tree_arg(a, 1)?;
        let s = it.tree_symbol(original)?;
        let SymRef::Term(sym) = s else { return it.unsupported("ValDef.copy of a tree without a symbol") };
        match it.opt_term_arg(a, 4)? {
            Some(rhs) => {
                it.typer.prog.stmts.push(TStmt::Val(sym, rhs));
                Ok(Value::Tree(TreeRef::Stmt(it.typer.prog.stmts.len() as u32 - 1)))
            }
            None => Ok(Value::Tree(TreeRef::Def(sym))),
        }
    });
    q!(it, "Reflect.ValDef.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::ValDef(s, rhs) => {
                let name = it.symbol_name(SymRef::Term(s));
                let ty = it.typer.sig_of(s).ret;
                let rhs = it.opt_tree(rhs)?;
                it.make_tuple(vec![Value::string(name), Value::Tree(TreeRef::Type(ty)), rhs])
            }
            _ => it.unsupported("ValDef.unapply on a tree that is no val"),
        }
    });
    q!(it, "Reflect.ValOrDefDefMethods.ValOrDefDef.tpt", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::ValDef(s, _) => Ok(Value::Tree(TreeRef::Type(it.typer.sig_of(s).ret))),
            View::DefDef(..) => Ok(Value::Tree(TreeRef::Type(it.def_parts(t)?.2))),
            _ => it.unsupported("tpt of a tree that is no definition"),
        }
    });
    q!(it, "Reflect.ValOrDefDefMethods.ValOrDefDef.rhs", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::ValDef(_, rhs) => it.opt_tree(rhs),
            View::DefDef(..) => {
                let rhs = it.def_parts(t)?.3;
                it.opt_tree(rhs)
            }
            _ => it.unsupported("rhs of a tree that is no definition"),
        }
    });
    q!(it, "Reflect.DefDef.apply", |it, a| {
        let s = it.sym_arg(a, 1)?;
        let SymRef::Term(sym) = s else { return it.unsupported("DefDef.apply takes a method symbol") };
        let sig = it.typer.sig_arc(sym);
        let span = it.macro_ctx.as_ref().map_or(Span::default(), |c| c.site_span);
        let mut params: Vec<SymId> = Vec::new();
        let mut clause_values: Vec<Value> = Vec::new();
        for clause in &sig.clauses {
            let mut refs = Vec::new();
            for p in &clause.params {
                let local = it.typer.new_local(p.name, SymKind::Val, p.ty, span);
                params.push(local);
                let e = it.typer.prog.add(TExpr::Local(local));
                it.typer.prog.set_type(e, p.ty);
                refs.push(Value::Tree(TreeRef::Expr(e)));
            }
            clause_values.push(it.make_list(refs)?);
        }
        let clauses = it.make_list(clause_values)?;
        let body = it.apply_value(arg(a, 2), vec![clauses])?;
        let body = match it.option_item(&body)? {
            Some(Value::Tree(t)) => Some(it.materialize(t)?),
            _ => None,
        };
        let n = params.len();
        let f = it.typer.prog.add_fun(TFun { sym, params, defaults: vec![None; n], body });
        Ok(Value::Tree(TreeRef::Fun(f)))
    });
    q!(it, "Reflect.DefDef.copy", |it, a| {
        let original = it.tree_arg(a, 1)?;
        let rhs = it.opt_term_arg(a, 5)?;
        match original {
            TreeRef::LambdaDef(e) => {
                let TExpr::Lambda(params, _) = it.prog().expr(e) else { return it.unsupported("DefDef.copy of a closure") };
                let Some(body) = rhs else { return it.unsupported("a lambda without a body") };
                let te = it.typer.prog.add(TExpr::Lambda(params, body));
                if let Some(ty) = it.prog().type_of(e) {
                    it.typer.prog.set_type(te, ty);
                }
                Ok(Value::Tree(TreeRef::LambdaDef(te)))
            }
            _ => {
                let s = it.tree_symbol(original)?;
                let SymRef::Term(sym) = s else { return it.unsupported("DefDef.copy of a tree without a symbol") };
                let (params, defaults) = match original {
                    TreeRef::Fun(f) => {
                        let fun = &it.prog().funs[f.idx()];
                        (fun.params.clone(), fun.defaults.clone())
                    }
                    _ => {
                        let sig = it.typer.sig_of(sym);
                        let ps: Vec<SymId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
                        let n = ps.len();
                        (ps, vec![None; n])
                    }
                };
                let f = it.typer.prog.add_fun(TFun { sym, params, defaults, body: rhs });
                Ok(Value::Tree(TreeRef::Fun(f)))
            }
        }
    });
    q!(it, "Reflect.DefDef.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        let (name, clauses, ret, rhs) = it.def_parts(t)?;
        let paramss = it.param_clause_values(&clauses)?;
        let rhs = it.opt_tree(rhs)?;
        it.make_tuple(vec![Value::string(name), paramss, Value::Tree(TreeRef::Type(ret)), rhs])
    });
    q!(it, "Reflect.DefDefMethods.DefDef.paramss", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let (_, clauses, _, _) = it.def_parts(t)?;
        it.param_clause_values(&clauses)
    });
    q!(it, "Reflect.DefDefMethods.DefDef.returnTpt", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let (_, _, ret, _) = it.def_parts(t)?;
        Ok(Value::Tree(TreeRef::Type(ret)))
    });
    q!(it, "Reflect.TypeDef.apply", |it, a| {
        let s = it.sym_arg(a, 1)?;
        Ok(Value::Tree(it.sym_tree(s)))
    });
    q!(it, "Reflect.TypeDef.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        let s = it.tree_symbol(t)?;
        let name = it.symbol_name(s);
        let rhs = it.type_def_rhs(s);
        it.make_tuple(vec![Value::string(name), Value::Tree(TreeRef::Type(rhs))])
    });
    // A type definition's tree is its symbol's: a copy with the same name and right-hand side
    // is the tree itself, the one a `TreeMap` that leaves type trees alone makes.
    q!(it, "Reflect.TypeDef.copy", |it, a| {
        let original = it.tree_arg(a, 1)?;
        let s = it.tree_symbol(original)?;
        let name = it.str_arg(a, 2)?;
        let rhs = it.tree_arg(a, 3)?;
        let same_rhs = matches!(rhs, TreeRef::Type(t) if t == it.type_def_rhs(s));
        if *name != *it.symbol_name(s) || !same_rhs {
            return it.unsupported("TypeDef.copy with another name or right-hand side");
        }
        Ok(Value::Tree(original))
    });
    q!(it, "Reflect.TypeDefMethods.TypeDef.rhs", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let s = it.tree_symbol(t)?;
        Ok(Value::Tree(TreeRef::Type(it.type_def_rhs(s))))
    });
    q!(it, "Reflect.ClassDef.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        let TreeRef::Class(c) = t else { return it.unsupported("ClassDef.unapply on a tree that is no class") };
        let (name, parents, body) = it.class_parts(c)?;
        let none = it.make_none()?;
        it.make_tuple(vec![Value::string(name), Value::Tree(TreeRef::Ctor(c)), parents, none, body])
    });
    q!(it, "Reflect.ClassDefMethods.ClassDef.constructor", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let TreeRef::Class(c) = t else { return it.unsupported("constructor of a tree that is no class") };
        Ok(Value::Tree(TreeRef::Ctor(c)))
    });
    q!(it, "Reflect.ClassDefMethods.ClassDef.parents", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let TreeRef::Class(c) = t else { return it.unsupported("parents of a tree that is no class") };
        Ok(it.class_parts(c)?.1)
    });
    q!(it, "Reflect.ClassDefMethods.ClassDef.self", |it, _a| it.make_none());
    q!(it, "Reflect.ClassDefMethods.ClassDef.body", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let TreeRef::Class(c) = t else { return it.unsupported("body of a tree that is no class") };
        Ok(it.class_parts(c)?.2)
    });

    // patterns
    q!(it, "Reflect.Bind.apply", |it, a| {
        let s = it.sym_arg(a, 1)?;
        let SymRef::Term(sym) = s else { return it.unsupported("Bind.apply takes a term symbol") };
        // A term is a stable identifier pattern (`Bind(x, Ref(caseVal))` of chimney's cases).
        let inner = match it.tree_arg(a, 2)? {
            TreeRef::Pat(p) => p,
            TreeRef::Expr(e) => it.typer.prog.add_pat(TPat::Equals(e, false)),
            _ => return it.unsupported("Bind.apply takes a pattern"),
        };
        let inner = if matches!(it.prog().pats[inner.idx()], TPat::Wildcard) { None } else { Some(inner) };
        let p = it.typer.prog.add_pat(TPat::Bind(sym, inner));
        Ok(Value::Tree(TreeRef::Pat(p)))
    });
    q!(it, "Reflect.Bind.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Bind(s, inner) => {
                let name = it.symbol_name(SymRef::Term(s));
                it.make_tuple(vec![Value::string(name), Value::Tree(inner)])
            }
            _ => it.unsupported("Bind.unapply on a tree that is no binder"),
        }
    });
    q!(it, "Reflect.BindMethods.Bind.name", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Bind(s, _) => Ok(Value::string(it.symbol_name(SymRef::Term(s)))),
            _ => it.unsupported("name of a tree that is no binder"),
        }
    });
    q!(it, "Reflect.BindMethods.Bind.pattern", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Bind(_, inner) => Ok(Value::Tree(inner)),
            _ => it.unsupported("pattern of a tree that is no binder"),
        }
    });
    q!(it, "Reflect.Unapply.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Unapply(subs) => {
                let fun = it.pattern_fun(t)?;
                let list = it.tree_values(subs)?;
                let nil = it.make_list(Vec::new())?;
                it.make_tuple(vec![Value::Tree(fun), nil, list])
            }
            _ => it.unsupported("Unapply.unapply on a tree that is no extractor pattern"),
        }
    });
    q!(it, "Reflect.UnapplyMethods.Unapply.fun", |it, a| {
        let t = it.tree_arg(a, 0)?;
        Ok(Value::Tree(it.pattern_fun(t)?))
    });
    q!(it, "Reflect.UnapplyMethods.Unapply.implicits", |it, _a| it.make_list(Vec::new()));
    q!(it, "Reflect.UnapplyMethods.Unapply.patterns", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Unapply(subs) => it.tree_values(subs),
            _ => it.unsupported("patterns of a tree that is no extractor pattern"),
        }
    });
    q!(it, "Reflect.Alternatives.apply", |it, a| {
        let pats = it.tree_list(&arg(a, 1))?;
        let mut items = Vec::new();
        for p in pats {
            let TreeRef::Pat(p) = p else { return it.unsupported("Alternatives.apply takes patterns") };
            items.push(p);
        }
        let l = it.typer.prog.pat_lists.push_slice(&items);
        let p = it.typer.prog.add_pat(TPat::Alt(l));
        Ok(Value::Tree(TreeRef::Pat(p)))
    });
    q!(it, "Reflect.Alternatives.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        match it.view(t)? {
            View::Alternatives(subs) => {
                let list = it.tree_values(subs)?;
                it.make_some(list)
            }
            _ => it.unsupported("Alternatives.unapply on a tree that is no alternative"),
        }
    });
    q!(it, "Reflect.AlternativesMethods.Alternatives.patterns", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::Alternatives(subs) => it.tree_values(subs),
            _ => it.unsupported("patterns of a tree that is no alternative"),
        }
    });

    // type trees
    q!(it, "Reflect.TypeTree.of", |it, a| {
        let t = it.type_arg(a, 1)?;
        Ok(Value::Tree(TreeRef::Type(t)))
    });
    q!(it, "Reflect.TypeTree.ref", |it, a| {
        let s = it.sym_arg(a, 1)?;
        let t = it.type_ref_of(s)?;
        Ok(Value::Tree(TreeRef::Type(t)))
    });
    q!(it, "Reflect.TypeTreeMethods.TypeTree.tpe", |it, a| {
        let t = it.tree_arg(a, 0)?;
        Ok(Value::Type(it.tree_type(t)?))
    });
    q!(it, "Reflect.Inferred.apply", |it, a| {
        let t = it.type_arg(a, 1)?;
        Ok(Value::Tree(TreeRef::Type(t)))
    });
    q!(it, "Reflect.TypeIdent.apply", |it, a| {
        let s = it.sym_arg(a, 1)?;
        // scalac asserts it, which a macro may catch.
        if !matches!(s, SymRef::Class(_) | SymRef::TParam(_) | SymRef::Alias(_) | SymRef::Any | SymRef::Nothing | SymRef::PkgClass(_) | SymRef::Root | SymRef::FilePackage(_)) {
            let shown = it.symbol_name(s);
            return it.throw_named("AssertionError", &format!("assertion failed: Expected a type symbol, but got {}", shown));
        }
        let t = it.type_ref_of(s)?;
        Ok(Value::Tree(TreeRef::Type(t)))
    });
    q!(it, "Reflect.TypeIdent.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        let ty = it.tree_type(t)?;
        let s = it.type_symbol(ty);
        let name = it.symbol_name(s);
        it.make_some(Value::string(name))
    });
    q!(it, "Reflect.TypeIdentMethods.TypeIdent.name", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let ty = it.tree_type(t)?;
        let s = it.type_symbol(ty);
        Ok(Value::string(it.symbol_name(s)))
    });
    q!(it, "Reflect.Applied.apply", |it, a| {
        let tycon = it.type_tree_arg(a, 1)?;
        let args = it.type_list(&arg(a, 2))?;
        Ok(Value::Tree(TreeRef::Type(it.applied_type(tycon, &args)?)))
    });
    q!(it, "Reflect.Applied.unapply", |it, a| {
        let t = it.tree_arg(a, 1)?;
        let ty = it.tree_type(t)?;
        let ty = it.typer.zonk(ty);
        match it.typer.types.get(ty) {
            Type::Class(c, args) if args != EMPTY_LIST => {
                let items = it.typer.types.items(args).to_vec();
                let ctor = it.typer.types.mk(Type::Ctor(c));
                let list = it.make_list(items.into_iter().map(|x| Value::Tree(TreeRef::Type(x))).collect())?;
                it.make_tuple(vec![Value::Tree(TreeRef::Type(ctor)), list])
            }
            _ => it.unsupported("Applied.unapply on a type tree that is no application"),
        }
    });
    q!(it, "Reflect.Singleton.apply", |it, a| {
        let e = it.expr_arg(a, 1)?;
        let ty = match it.prog().expr(e) {
            TExpr::Local(s) | TExpr::Static(s) => it.typer.types.mk(Type::Term(s)),
            _ => it.tree_type(TreeRef::Expr(e))?,
        };
        Ok(Value::Tree(TreeRef::Type(ty)))
    });
    // A bounds tree is the bounded wildcard of its two bounds.
    q!(it, "Reflect.TypeBoundsTree.apply", |it, a| it.bounds_tree(a, 1));
    q!(it, "Reflect.TypeBoundsTree.copy", |it, a| it.bounds_tree(a, 2));
    q!(it, "Reflect.TypeBoundsTreeMethods.TypeBoundsTree.tpe", |it, a| {
        let t = it.tree_arg(a, 0)?;
        Ok(Value::Type(it.tree_type(t)?))
    });
    q!(it, "Reflect.TypeBoundsTreeMethods.TypeBoundsTree.low", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let ty = it.tree_type(t)?;
        Ok(Value::Tree(TreeRef::Type(it.typer.types.wild_bounds(ty).map_or(NOTHING, |b| b.0))))
    });
    q!(it, "Reflect.TypeBoundsTreeMethods.TypeBoundsTree.hi", |it, a| {
        let t = it.tree_arg(a, 0)?;
        let ty = it.tree_type(t)?;
        Ok(Value::Tree(TreeRef::Type(it.typer.types.wild_bounds(ty).map_or(ANY, |b| b.1))))
    });
    q!(it, "Reflect.WildcardTypeTree.apply", |it, a| {
        let t = it.type_arg(a, 1)?;
        Ok(Value::Tree(TreeRef::Type(t)))
    });
    q!(it, "Reflect.WildcardTypeTreeMethods.WildcardTypeTree.tpe", |it, a| {
        let t = it.tree_arg(a, 0)?;
        Ok(Value::Type(it.tree_type(t)?))
    });
}

impl<'a, 't> Interp<'a, 't> {
    fn sym_tree(&mut self, s: SymRef) -> TreeRef {
        match s {
            SymRef::Term(sym) => TreeRef::Def(sym),
            SymRef::Class(c) => TreeRef::Class(c),
            SymRef::Ctor(c) => TreeRef::Ctor(c),
            SymRef::TParam(p) => TreeRef::Type(self.typer.types.param(p)),
            SymRef::Alias(a) => TreeRef::Alias(a),
            SymRef::Default(c, i) => TreeRef::Default(c, i),
            _ => TreeRef::Type(ANY),
        }
    }

    /// The parameter clauses of a method as scalac's reflection API holds them: a list of lists,
    /// the `ValDef`s of a term clause or the `TypeDef`s of a type clause.
    fn param_clause_values(&mut self, clauses: &[Vec<SymRef>]) -> R {
        let mut items = Vec::with_capacity(clauses.len());
        for c in clauses {
            let params: Vec<Value> = c.iter().map(|&s| Value::Tree(self.sym_tree(s))).collect();
            items.push(self.make_list(params)?);
        }
        self.make_list(items)
    }

    /// Whether a list is a parameter clause of the kind `c` names, by its first element as
    /// scalac's type tests decide: an empty clause is a term clause.
    pub(super) fn clause_is_instance(&mut self, v: &Value, c: ClassId) -> bool {
        if !self.name(self.syms().class(c).name).ends_with("ParamClause") {
            return false;
        }
        let term = self.reflect_class_named(c, "TermParamClause");
        let ty = self.reflect_class_named(c, "TypeParamClause");
        if !term && !ty && !self.reflect_class_named(c, "ParamClause") {
            return false;
        }
        let Ok(items) = self.list_items(v) else { return false };
        let Some(Value::Tree(head)) = items.first() else { return items.is_empty() && !ty };
        let is_type = match *head {
            TreeRef::Type(t) => matches!(self.typer.types.get(t), Type::Param(_)),
            t => matches!(self.tree_kind(t), Ok("TypeDef")),
        };
        if term { !is_type && matches!(self.tree_kind(*head), Ok("ValDef")) } else if ty { is_type } else { true }
    }

    /// The name, the parameter clauses, the result type and the body of a method tree.
    fn def_parts(&mut self, t: TreeRef) -> R<(String, Vec<Vec<SymRef>>, TypeId, Option<TreeRef>)> {
        match t {
            TreeRef::LambdaDef(e) => {
                let TExpr::Lambda(params, body) = self.prog().expr(e) else { return self.unsupported("a closure without a lambda") };
                let ps: Vec<SymRef> = self.prog().sym_list(params).iter().map(|&p| SymRef::Term(p)).collect();
                let ret = self.tree_type(TreeRef::Expr(body))?;
                Ok(("$anonfun".to_string(), vec![ps], ret, Some(TreeRef::Expr(body))))
            }
            TreeRef::Ctor(c) => {
                let (tparams, clauses) = {
                    let info = self.syms().class(c);
                    (info.tparams.clone(), info.ctor_syms.clone())
                };
                let mut out: Vec<Vec<SymRef>> = Vec::new();
                if !tparams.is_empty() {
                    out.push(tparams.into_iter().map(SymRef::TParam).collect());
                }
                for clause in clauses {
                    out.push(clause.into_iter().map(SymRef::Term).collect());
                }
                let unit = self.typer.b.t_unit;
                Ok(("<init>".to_string(), out, unit, None))
            }
            // scalac's tree of a default getter has no right-hand side at expansion time
            // (Magnolia's `defaultValue` answers `None` without `-Yretain-trees`).
            TreeRef::Default(c, i) => {
                let init = self.default_init(c, i)?;
                let ret = self.tree_type(TreeRef::Expr(init))?;
                Ok((format!("$lessinit$greater$default${}", i + 1), Vec::new(), ret, None))
            }
            _ => match self.view(t)? {
                View::DefDef(s, clauses, body) if s != SymId(u32::MAX) => {
                    let name = self.symbol_name(SymRef::Term(s));
                    let ret = self.typer.sig_of(s).ret;
                    Ok((name, clauses, ret, body))
                }
                _ => self.unsupported("a method was expected"),
            },
        }
    }

    fn type_def_rhs(&mut self, s: SymRef) -> TypeId {
        match s {
            SymRef::Alias(a) => {
                self.typer.complete_alias(a);
                let info = &self.typer.syms.aliases[a.idx()];
                match info.bounds {
                    Some(_) => WILD,
                    None => info.rhs,
                }
            }
            SymRef::TParam(_) => WILD,
            SymRef::Class(c) => self.typer.types.class(c, &[]),
            _ => ANY,
        }
    }

    /// The name, the parents as type trees and the members of a class.
    fn class_parts(&mut self, c: ClassId) -> R<(String, Value, Value)> {
        self.typer.complete_class(c);
        let (name, parents, members) = {
            let info = self.syms().class(c);
            (self.typer.name_ref(info.name).to_string(), info.parents.clone(), info.member_order.clone())
        };
        let parents = self.make_list(parents.into_iter().map(|p| Value::Tree(TreeRef::Type(p))).collect())?;
        let mut body = Vec::new();
        for m in members {
            let info = self.syms().sym(m);
            if matches!(info.kind, SymKind::Overloaded(_)) || info.def.is_none() && !self.typer.is_library_member(m) {
                continue;
            }
            body.push(Value::Tree(TreeRef::Def(m)));
        }
        let body = self.make_list(body)?;
        Ok((name, parents, body))
    }

    /// The extractor a class or sequence pattern calls, as a reference to `unapply`.
    fn pattern_fun(&mut self, t: TreeRef) -> R<TreeRef> {
        let TreeRef::Pat(p) = t else { return self.unsupported("a pattern was expected") };
        match self.prog().pats[p.idx()] {
            TPat::Class(c, _, _, _) => {
                let companion = self.syms().class(c).companion;
                let unapply = self.typer.interner.intern("unapply");
                let sym = companion.and_then(|k| self.syms().class(k).members.get(&unapply).copied());
                match (companion, sym) {
                    (Some(k), Some(s)) => {
                        let m = self.typer.prog.add(TExpr::Module(k));
                        Ok(self.pending(Pending { recv: Some(TreeRef::Expr(m)), kind: PendingKind::Sym(s), targs: Vec::new() }))
                    }
                    (Some(k), None) => {
                        let m = self.typer.prog.add(TExpr::Module(k));
                        Ok(self.pending(Pending { recv: Some(TreeRef::Expr(m)), kind: PendingKind::Named(unapply), targs: Vec::new() }))
                    }
                    _ => {
                        let m = self.typer.prog.add(TExpr::Unit);
                        Ok(TreeRef::Expr(m))
                    }
                }
            }
            TPat::Unapply(_, call, _) => match self.prog().expr(call) {
                TExpr::CallMethod(r, s, _) => Ok(self.pending(Pending { recv: Some(TreeRef::Expr(r)), kind: PendingKind::Sym(s), targs: Vec::new() })),
                TExpr::CallStatic(s, _) => Ok(self.pending(Pending { recv: None, kind: PendingKind::Sym(s), targs: Vec::new() })),
                _ => Ok(TreeRef::Expr(call)),
            },
            _ => {
                let m = self.typer.prog.add(TExpr::Unit);
                Ok(TreeRef::Expr(m))
            }
        }
    }
}

// ---- types ----

impl<'a, 't> Interp<'a, 't> {
    /// The class or type parameter a type stands for.
    pub(super) fn type_symbol(&mut self, t: TypeId) -> SymRef {
        let t = self.typer.zonk(t);
        match self.typer.types.get(t) {
            Type::Class(c, _) | Type::Ctor(c) => SymRef::Class(c),
            Type::Param(p) | Type::AppParam(p, _) => SymRef::TParam(p),
            Type::Alias(a, _) | Type::Decl(a) => SymRef::Alias(a),
            Type::Lit(l) => {
                let c = match self.typer.types.lit_val(l) {
                    LitVal::Int(_) => self.typer.b.int,
                    LitVal::Long(_) => self.typer.b.long,
                    LitVal::Double(_) => self.typer.b.double,
                    LitVal::Char(_) => self.typer.b.char,
                    LitVal::Bool(_) => self.typer.b.boolean,
                    LitVal::Str(_) => self.typer.b.string,
                };
                SymRef::Class(c)
            }
            Type::Any => SymRef::Any,
            Type::Nothing => SymRef::Nothing,
            Type::This(c) => SymRef::Class(c),
            Type::Term(s) | Type::Select(_, s) => {
                let ret = self.typer.sig_of(s).ret;
                if ret == t { SymRef::None } else { self.type_symbol(ret) }
            }
            Type::Lambda(_, b) | Type::Poly(_, b) => self.type_symbol(b),
            Type::Refined(p, _) | Type::AppMember(p, _) => self.type_symbol(p),
            Type::Blocked(_) => self.pkg_form(t).map_or(SymRef::None, |(_, p)| p),
            Type::Member(..)
            | Type::Union(..)
            | Type::Inter(..)
            | Type::Var(_)
            | Type::AppVar(..)
            | Type::Wild
            | Type::BoundedWild(..)
            | Type::Error
            | Type::Match(..) => SymRef::None,
        }
    }

    pub(super) fn term_symbol(&mut self, t: TypeId) -> SymRef {
        let t = self.typer.zonk(t);
        match self.typer.types.get(t) {
            Type::Term(s) | Type::Select(_, s) => SymRef::Term(s),
            Type::Class(c, _) if self.syms().class(c).kind == ClassKind::Object => self.module_sym(c),
            Type::This(c) if self.syms().class(c).kind == ClassKind::Object => self.module_sym(c),
            Type::Class(c, _) if self.syms().class(c).mods & crate::ast::mods::CASE != 0 => SymRef::Class(c),
            Type::Blocked(_) => match self.pkg_form(t) {
                Some((PkgForm::Path | PkgForm::Sym | PkgForm::ThisType, SymRef::PkgClass(p))) => SymRef::Pkg(p),
                Some((PkgForm::ThisType, SymRef::Root)) => SymRef::Root,
                _ => SymRef::None,
            },
            _ => SymRef::None,
        }
    }

    /// The type a symbol names: a class applied to nothing, a parameter, an alias.
    fn type_ref_of(&mut self, s: SymRef) -> R<TypeId> {
        Ok(match s {
            SymRef::Class(c) => {
                if self.syms().class(c).tparams.is_empty() {
                    self.typer.types.class(c, &[])
                } else {
                    self.typer.types.mk(Type::Ctor(c))
                }
            }
            SymRef::TParam(p) => self.typer.types.param(p),
            SymRef::Alias(a) => {
                self.typer.complete_alias(a);
                let info = &self.typer.syms.aliases[a.idx()];
                if info.tparams.is_empty() { info.rhs } else { self.typer.types.mk(Type::Alias(a, EMPTY_LIST)) }
            }
            SymRef::Any => ANY,
            SymRef::Nothing => NOTHING,
            SymRef::PkgClass(_) | SymRef::Pkg(_) => self.pkg_type(PkgForm::SymClassRef, s),
            SymRef::Root => self.pkg_type(PkgForm::ClassRef, s),
            // scalac's `NoSymbol.typeRef` is a type that equals nothing else, not an error
            // (`defn.MatchableClass.typeRef` where the std has no `Matchable`).
            SymRef::None => self.typer.types.blocked("<nosymbol>"),
            SymRef::Term(s) if matches!(self.syms().sym(s).kind, SymKind::Object(_)) => {
                let SymKind::Object(c) = self.syms().sym(s).kind else { unreachable!() };
                self.typer.types.class(c, &[])
            }
            other => {
                let shown = self.symbol_name(other);
                return self.unsupported(format!("{} is no type", shown));
            }
        })
    }

    fn applied_type(&mut self, tycon: TypeId, args: &[TypeId]) -> R<TypeId> {
        let tycon = self.typer.zonk(tycon);
        if let Some(m) = self.method_type_parts(tycon)? {
            return Ok(self.applied_method_type(m, args));
        }
        Ok(match self.typer.types.get(tycon) {
            Type::Ctor(c) | Type::Class(c, _) => self.typer.types.class(c, args),
            Type::Lambda(..) => self.typer.types.apply_ctor(tycon, args),
            Type::Alias(a, _) => {
                let l = self.typer.types.list(args);
                self.typer.types.mk(Type::Alias(a, l))
            }
            Type::Param(p) => {
                let l = self.typer.types.list(args);
                self.typer.types.mk(Type::AppParam(p, l))
            }
            _ => {
                let shown = self.show_type_repr(tycon);
                return self.unsupported(format!("{} takes no type arguments", shown));
            }
        })
    }

    /// The prefix of a named type as the reflect API shows it: the owner package or object.
    fn prefix_of(&mut self, owner: Owner) -> TypeId {
        match owner {
            Owner::Package(p) if p == ROOT_PKG => self.pkg_type(PkgForm::ThisType, SymRef::PkgClass(p)),
            Owner::Package(p) => self.pkg_type(PkgForm::Path, SymRef::PkgClass(p)),
            Owner::Class(c) => {
                if self.syms().class(c).kind == ClassKind::Object {
                    self.typer.types.class(c, &[])
                } else {
                    self.typer.types.mk(Type::This(c))
                }
            }
            Owner::Local => self.typer.types.blocked("<noprefix>"),
        }
    }

    /// A type over a package, which `Type` has no case for: kept as a blocked type whose
    /// description names the form and the package (`SymRef::PkgClass` or `SymRef::Root`).
    fn pkg_type(&mut self, form: PkgForm, pkg: SymRef) -> TypeId {
        let id = match pkg {
            SymRef::PkgClass(p) | SymRef::Pkg(p) => p.0.to_string(),
            _ => "root".to_string(),
        };
        let desc = format!("<pkg{}>{}", form as u8, id);
        self.typer.types.blocked(&desc)
    }

    pub(super) fn pkg_form(&self, t: TypeId) -> Option<(PkgForm, SymRef)> {
        let Type::Blocked(b) = self.typer.types.get(t) else { return None };
        let rest = self.typer.types.blocked_description(b).strip_prefix("<pkg")?;
        let (form, id) = rest.split_once('>')?;
        let form = match form {
            "0" => PkgForm::Path,
            "1" => PkgForm::Sym,
            "2" => PkgForm::ThisType,
            "3" => PkgForm::ClassRef,
            "4" => PkgForm::SymClassRef,
            _ => return None,
        };
        let pkg = if id == "root" { SymRef::Root } else { SymRef::PkgClass(PkgId(id.parse().ok()?)) };
        Some((form, pkg))
    }

    /// The package class a package is a member of: the root for the empty package and the
    /// top-level ones.
    fn pkg_parent(&self, pkg: SymRef) -> SymRef {
        match pkg {
            SymRef::PkgClass(p) if p != ROOT_PKG => match self.syms().pkg(p).parent {
                Some(parent) if parent != ROOT_PKG => SymRef::PkgClass(parent),
                _ => SymRef::Root,
            },
            _ => SymRef::Root,
        }
    }

    /// scalac's shapes: a path `TermRef(TermRef(ThisType(<root>), scala), collection)`, a
    /// symbol's `termRef` and `typeRef` over the owner's `ThisType`, and the `ThisType` of a
    /// package class over `TypeRef(NoPrefix, name)`.
    fn pkg_named_parts(&mut self, form: PkgForm, pkg: SymRef) -> (TypeId, String) {
        let name = self.symbol_name(pkg);
        let parent = self.pkg_parent(pkg);
        let qual = match form {
            PkgForm::Path => match parent {
                SymRef::Root => self.pkg_type(PkgForm::ThisType, SymRef::Root),
                p => self.pkg_type(PkgForm::Path, p),
            },
            PkgForm::Sym | PkgForm::SymClassRef => self.pkg_type(PkgForm::ThisType, parent),
            PkgForm::ThisType | PkgForm::ClassRef => self.typer.types.blocked("<noprefix>"),
        };
        (qual, name)
    }

    fn this_tref(&mut self, t: TypeId) -> R<TypeId> {
        match self.typer.types.get(t) {
            Type::This(c) => Ok(self.typer.types.class(c, &[])),
            _ => match self.pkg_form(t) {
                Some((PkgForm::ThisType, pkg)) => Ok(self.pkg_type(PkgForm::ClassRef, pkg)),
                _ => self.unsupported("ThisType.unapply on a type that is no this"),
            },
        }
    }

    /// The base type of `t` for the class `c`: of an intersection the first part's that has
    /// one, as scalac's `baseType` takes it.
    fn base_type_of(&mut self, t: TypeId, c: ClassId) -> Option<TypeId> {
        let t = self.typer.dealias(t);
        match self.typer.types.get(t) {
            Type::Inter(a, b) => self.base_type_of(a, c).or_else(|| self.base_type_of(b, c)),
            // `AnyRef` is a base of every class `baseClasses` lists it for.
            _ if c == self.typer.b.any_ref => Some(self.typer.types.class(c, &[])),
            _ => self.typer.base_type(t, c).or_else(|| self.product_base(t, c)),
        }
    }

    /// `Product`, `Equals` and `Serializable` of a case class or an enum, which `baseClasses` lists.
    fn product_base(&mut self, t: TypeId, c: ClassId) -> Option<TypeId> {
        let serializable = self.typer.class_at(&["java", "io", "Serializable"]);
        if (Some(c) == self.typer.b.product || Some(c) == self.typer.equals_class() || Some(c) == serializable) && self.typer.is_product(t) {
            return Some(self.typer.types.class(c, &[]));
        }
        None
    }

    fn widen_type(&mut self, t: TypeId) -> TypeId {
        // A val's `typeRef` is scalac's `TypeRef`, no singleton to widen.
        if self.val_type_ref(t).is_some() {
            return t;
        }
        // `=> T` widens to `T` widened, as scalac's `ExprType` does.
        if let Some(u) = self.by_name_underlying(t).unwrap_or(None) {
            return self.widen_type(u);
        }
        let t = self.typer.zonk(t);
        let t = match self.typer.types.get(t) {
            Type::Term(s) | Type::Select(_, s) => {
                let ret = self.typer.sig_of(s).ret;
                if ret == t { t } else { self.widen_type(ret) }
            }
            _ => t,
        };
        self.typer.widen_lit(t)
    }

    fn member_type_of(&mut self, t: TypeId, s: SymRef) -> R<TypeId> {
        match s {
            SymRef::Term(sym) => {
                let ret = self.typer.sig_of(sym).ret;
                let mut subst: Subst = Vec::new();
                if let Owner::Class(owner) = self.syms().sym(sym).owner {
                    if let Some(base) = self.typer.base_type(t, owner) {
                        if let Type::Class(_, args) = self.typer.types.get(base) {
                            let items = self.typer.types.items(args).to_vec();
                            let tparams = self.syms().class(owner).tparams.clone();
                            subst.extend(tparams.into_iter().zip(items));
                        }
                    }
                }
                Ok(self.typer.types.subst(ret, &subst))
            }
            SymRef::Default(c, i) => {
                let init = self.default_init(c, i)?;
                self.tree_type(TreeRef::Expr(init))
            }
            SymRef::Ctor(c) => Ok(self.ctor_method_type(t, c)),
            other => self.type_ref_of(other),
        }
    }

    /// The primary constructor of `c` as a member of `t`: a method type per parameter clause,
    /// the class's parameters taken from `t`, as scalac's `memberType` gives it. Where `t` does
    /// not apply the class, the parameters stay for `appliedTo` to fill.
    fn ctor_method_type(&mut self, t: TypeId, c: ClassId) -> TypeId {
        self.typer.complete_class(c);
        let tparams = self.syms().class(c).tparams.clone();
        let mut subst: Subst = Vec::new();
        if let Some(base) = self.typer.base_type(t, c) {
            if let Type::Class(_, args) = self.typer.types.get(base) {
                let items = self.typer.types.items(args).to_vec();
                if items.len() == tparams.len() {
                    subst.extend(tparams.iter().copied().zip(items));
                }
            }
        }
        let open = if subst.is_empty() { tparams.clone() } else { Vec::new() };
        let result = match self.typer.base_type(t, c) {
            Some(b) if !subst.is_empty() => b,
            _ => {
                let ps: Vec<TypeId> = tparams.iter().map(|&p| self.typer.types.param(p)).collect();
                self.typer.types.class(c, &ps)
            }
        };
        let clauses: Vec<(Vec<crate::intern::Name>, Vec<TypeId>, ClauseKind)> = self
            .syms()
            .class(c)
            .ctor
            .iter()
            .map(|clause| (clause.params.iter().map(|p| p.name).collect(), clause.params.iter().map(|p| p.ty).collect(), ClauseKind::of(clause)))
            .collect();
        let mut result = result;
        for (names, types, kind) in clauses.into_iter().rev() {
            let types = types.into_iter().map(|ty| self.typer.types.subst(ty, &subst)).collect();
            result = self.method_type(MethodTypeRepr { names, types, result, open: open.clone(), kind });
        }
        result
    }

    /// The method type of the builtin operator `op` applied to `r` in `e`, as scalac's typed
    /// `Select` carries it: an operator of a number or a `Boolean` is overloaded by the
    /// argument's type, which the chosen alternative takes (the operand's type before the
    /// typer widened it, where the typed program keeps it); `eq` and `ne` take `AnyRef`, and
    /// `==` and `!=` of anything else `Any`.
    fn operator_type(&mut self, op: PrimOp, l: TExprId, r: TExprId, e: TExprId) -> TypeId {
        let lty = self.operand_type(l);
        let overloaded = self.typer.is_numeric(lty).is_some() || self.typer.dealias(lty) == self.typer.b.t_boolean;
        let (name, param) = if overloaded {
            ("x", self.operand_type(r))
        } else {
            match op {
                PrimOp::RefEq | PrimOp::RefNe | PrimOp::Eq | PrimOp::Ne if lty == self.typer.b.t_string => ("x$0", ANY),
                PrimOp::RefEq | PrimOp::RefNe => ("x$0", self.typer.b.t_any_ref),
                _ => ("x$0", ANY),
            }
        };
        let result = self.prog().type_of(e).unwrap_or(ANY);
        let names = vec![self.typer.interner.intern(name)];
        self.method_type(MethodTypeRepr { names, types: vec![param], result, open: Vec::new(), kind: ClauseKind::Plain })
    }

    /// The type of a selection of a method that awaits its arguments, as scalac's typed tree
    /// has it: the method type of its parameter clauses over the receiver's type arguments, and
    /// under a `PolyType` where a generic method is selected without its type arguments; the
    /// same of the method behind a closure. None for a tree that is no such selection.
    fn selection_method_type(&mut self, t: TreeRef) -> R<Option<TypeId>> {
        let (s, recv, targs) = match t {
            TreeRef::Pending(i) => {
                let p = self.pending_of(i)?;
                match p.kind {
                    PendingKind::Operator(_, method) => return Ok(Some(method)),
                    PendingKind::Sym(s) if self.takes_arguments(s) => {
                        let recv = match p.recv {
                            Some(r) => Some(self.materialize(r)?),
                            None => None,
                        };
                        (s, recv, p.targs)
                    }
                    _ => return Ok(None),
                }
            }
            TreeRef::LambdaRef(e) => {
                let TExpr::Lambda(params, body) = self.prog().expr(e) else { return Ok(None) };
                let params = self.prog().sym_list(params).to_vec();
                let names = params.iter().map(|&p| self.syms().sym(p).name).collect();
                let types = params.iter().map(|&p| self.typer.sig_of(p).ret).collect();
                let result = self.tree_type(TreeRef::Expr(body))?;
                return Ok(Some(self.method_type(MethodTypeRepr { names, types, result, open: Vec::new(), kind: ClauseKind::Plain })));
            }
            _ => return Ok(None),
        };
        let sig = self.typer.sig_arc(s);
        let mut subst: Subst = Vec::new();
        if let (Some(r), Owner::Class(owner)) = (recv, self.syms().sym(s).owner) {
            if let Some(rty) = self.prog().type_of(r) {
                if let Some(base) = self.typer.base_type(rty, owner) {
                    if let Type::Class(_, bargs) = self.typer.types.get(base) {
                        let items = self.typer.types.items(bargs).to_vec();
                        let tparams = self.syms().class(owner).tparams.clone();
                        subst.extend(tparams.into_iter().zip(items));
                    }
                }
            }
        }
        let poly = !sig.tparams.is_empty() && targs.is_empty();
        if targs.len() == sig.tparams.len() {
            subst.extend(sig.tparams.iter().copied().zip(targs.iter().copied()));
        }
        let mut result = self.typer.types.subst(sig.ret, &subst);
        for clause in sig.clauses.iter().rev() {
            let names = clause.params.iter().map(|p| p.name).collect();
            let types = clause
                .params
                .iter()
                .map(|p| {
                    let t = self.typer.types.subst(p.ty, &subst);
                    if p.by_name { self.typer.by_name_type(t) } else { t }
                })
                .collect();
            result = self.method_type(MethodTypeRepr { names, types, result, open: Vec::new(), kind: ClauseKind::of(clause) });
        }
        if poly {
            let desc = format!("<poly>{}", self.records.poly_types.push(PolyTypeRepr { params: sig.tparams.to_vec(), result }));
            result = self.typer.types.blocked(&desc);
        }
        Ok(Some(result))
    }

    /// Whether the tree is a selection of a method that awaits its arguments, which is no
    /// expression (`Tree.isExpr`): what `selection_method_type` gives a type.
    pub(super) fn awaits_arguments(&mut self, t: TreeRef) -> R<bool> {
        Ok(match t {
            TreeRef::Pending(i) => match self.pending_of(i)?.kind {
                PendingKind::Operator(..) => true,
                PendingKind::Sym(s) => self.takes_arguments(s),
                _ => false,
            },
            TreeRef::LambdaRef(_) => true,
            _ => false,
        })
    }

    fn poly_type_parts(&self, t: TypeId) -> R<Option<PolyTypeRepr>> {
        let Type::Blocked(b) = self.typer.types.get(t) else { return Ok(None) };
        let Some(id) = self.typer.types.blocked_description(b).strip_prefix("<poly>").and_then(|n| n.parse::<u32>().ok()) else { return Ok(None) };
        match self.records.poly_types.get(id) {
            Some(p) => Ok(Some(p.clone())),
            None => self.stale(STALE_METHOD),
        }
    }

    /// The type of an operand of a builtin operator before the typer widened it to the
    /// operator's rank.
    fn operand_type(&mut self, x: TExprId) -> TypeId {
        let operand = match self.prog().expr(x) {
            TExpr::Unary(UnOp::IntToLong | UnOp::CharToInt | UnOp::CharToLong | UnOp::IntToFloat | UnOp::IntToDouble | UnOp::ByteToShort | UnOp::ByteToInt | UnOp::ShortToInt | UnOp::LongToFloat | UnOp::LongToDouble | UnOp::FloatToDouble, inner) => inner,
            _ => x,
        };
        let ty = self.prog().type_of(operand).unwrap_or(ANY);
        self.typer.widen_lit(ty)
    }

    fn method_type(&mut self, m: MethodTypeRepr) -> TypeId {
        if let Some(&t) = self.records.method_type_ids.get(&m) {
            #[cfg(debug_assertions)]
            crate::types::view::cached("Records' method types", t);
            return t;
        }
        #[cfg(debug_assertions)]
        for &x in m.types.iter().chain([&m.result]) {
            crate::types::view::cached("Records' method types", x);
        }
        let desc = format!("<method>{}", self.records.method_types.push(m.clone()));
        let t = self.typer.types.blocked(&desc);
        self.records.method_type_ids.insert(m, t);
        t
    }

    /// `t` of a by-name parameter's type `=> t` (`ByNameType`).
    fn by_name_underlying(&mut self, t: TypeId) -> R<Option<TypeId>> {
        Ok(self.typer.by_name_arg(t))
    }

    fn method_type_parts(&self, t: TypeId) -> R<Option<MethodTypeRepr>> {
        let Type::Blocked(b) = self.typer.types.get(t) else { return Ok(None) };
        let Some(id) = self.typer.types.blocked_description(b).strip_prefix("<method>").and_then(|n| n.parse::<u32>().ok()) else { return Ok(None) };
        match self.records.method_types.get(id) {
            Some(m) => Ok(Some(m.clone())),
            None => self.stale(STALE_METHOD),
        }
    }

    /// A constructor's method type over its class's parameters applied to arguments, as
    /// `memberType(ctor).appliedTo(typeArgs)` reads a generic class's parameter types.
    fn applied_method_type(&mut self, m: MethodTypeRepr, args: &[TypeId]) -> TypeId {
        let subst: Subst = m.open.iter().copied().zip(args.iter().copied()).collect();
        let types = m.types.iter().map(|&t| self.typer.types.subst(t, &subst)).collect();
        // The result of a method type is made with it: no build lies between them.
        let result = match self.method_type_parts(m.result).unwrap_or(None) {
            Some(inner) => self.applied_method_type(inner, args),
            None => self.typer.types.subst(m.result, &subst),
        };
        self.method_type(MethodTypeRepr { names: m.names, types, result, open: Vec::new(), kind: m.kind })
    }

    fn base_classes(&mut self, t: TypeId) -> Vec<SymRef> {
        let t = self.typer.dealias(t);
        // An opaque type outside its scope is an abstract type with its bound's classes (`Any`
        // without one), within it an alias with the underlying type's, as scalac lists them.
        if let Type::Class(c, _) = self.typer.types.get(t) {
            if self.syms().class(c).kind == ClassKind::Opaque {
                let seen = self.typer.opaque_underlying(t).or_else(|| self.typer.opaque_bound(t)).unwrap_or(ANY);
                return self.base_classes(seen);
            }
        }
        // Of an intersection, the second part's classes the first lacks, then the first's, as
        // scalac merges them (`Counter, Greeter, Object, ..` for `Greeter & Counter`).
        if let Type::Inter(a, b) = self.typer.types.get(t) {
            let (first, second) = (self.base_classes(a), self.base_classes(b));
            let mut out: Vec<SymRef> = second.iter().copied().filter(|s| !first.contains(s)).collect();
            for s in first {
                if !out.contains(&s) {
                    out.push(s);
                }
            }
            return out;
        }
        match self.typer.class_of(t) {
            Some(c) => {
                self.typer.complete_class(c);
                let mut out: Vec<SymRef> = self.syms().class(c).base_types.iter().map(|&(b, _)| SymRef::Class(b)).collect();
                // A case class or an enum extends `Product` and `Serializable`, which scalac adds
                // to its parents and teq's typer knows by its kind.
                if let Some(product) = self.typer.b.product.filter(|&p| !out.contains(&SymRef::Class(p))) {
                    let own = self.typer.types.class(c, &[]);
                    if self.typer.is_product(own) {
                        let at = out.iter().position(|s| *s == SymRef::Class(self.typer.b.any_ref)).unwrap_or(out.len());
                        let mut added = Vec::new();
                        if let Some(ser) = self.typer.class_at(&["java", "io", "Serializable"]).filter(|&k| !out.contains(&SymRef::Class(k))) {
                            added.push(SymRef::Class(ser));
                        }
                        added.push(SymRef::Class(product));
                        if let Some(equals) = self.typer.equals_class().filter(|&k| !out.contains(&SymRef::Class(k))) {
                            added.push(SymRef::Class(equals));
                        }
                        out.splice(at..at, added);
                    }
                }
                if !out.contains(&SymRef::Class(self.typer.b.any_ref)) && !self.syms().class(c).value_class {
                    let any_ref = self.typer.b.any_ref;
                    out.push(SymRef::Class(any_ref));
                }
                out.push(SymRef::Any);
                out
            }
            None => vec![SymRef::Any],
        }
    }

    /// `TypeBoundsTree(low, hi)` from the two trees at `a[i]` and `a[i + 1]`.
    fn bounds_tree(&mut self, a: A, i: usize) -> R {
        let (low, hi) = (self.tree_arg(a, i)?, self.tree_arg(a, i + 1)?);
        let (lo, hi) = (self.tree_type(low)?, self.tree_type(hi)?);
        Ok(Value::Tree(TreeRef::Type(self.typer.types.bounded_wild(lo, hi))))
    }

    fn bounds_of(&mut self, a: A, i: usize) -> R<(TypeId, TypeId)> {
        let t = self.type_arg(a, i)?;
        Ok(self.typer.types.wild_bounds(t).unwrap_or((NOTHING, ANY)))
    }
}

fn install_types(it: &mut Table) {
    q!(it, "Reflect.TypeRepr.of", |it, a| Ok(Value::Type(it.type_arg(a, 1)?)));
    q!(it, "Reflect.TypeRepr.typeConstructorOf", |it, _a| it.unsupported("TypeRepr.typeConstructorOf needs a runtime class"));
    q!(it, "Reflect.TypeReprMethods.TypeRepr.asType", |it, a| Ok(Value::Type(it.type_arg(a, 0)?)));
    q!(it, "Reflect.TypeReprMethods.TypeRepr.=:=", |it, a| {
        let (x, y) = (it.type_arg(a, 0)?, it.type_arg(a, 1)?);
        Ok(Value::Bool(it.same_type(x, y)))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.<:<", |it, a| {
        let (x, y) = (it.type_arg(a, 0)?, it.type_arg(a, 1)?);
        Ok(Value::Bool(it.conforms(x, y)))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.widen", |it, a| {
        let t = it.type_arg(a, 0)?;
        Ok(Value::Type(it.widen_type(t)))
    });
    // A term reference is widened to what it refers to, a constant type is none and stays.
    q!(it, "Reflect.TypeReprMethods.TypeRepr.widenTermRefByName", |it, a| {
        let t = it.type_arg(a, 0)?;
        if it.val_type_ref(t).is_some() {
            return Ok(Value::Type(t));
        }
        let t = it.typer.zonk(t);
        match it.typer.types.get(t) {
            Type::Term(_) | Type::Select(..) => Ok(Value::Type(it.typer.widen_path(t))),
            _ => Ok(Value::Type(t)),
        }
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.widenByName", |it, a| {
        let t = it.type_arg(a, 0)?;
        Ok(Value::Type(it.by_name_underlying(t)?.unwrap_or(t)))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.dealias", |it, a| {
        let t = it.type_arg(a, 0)?;
        let t = it.typer.zonk(t);
        // A constant type and a path stay what they are, as in scalac; the typer's dealias
        // widens them to their class.
        if matches!(it.typer.types.get(t), Type::Lit(_)) || it.typer.types.is_path(t) {
            return Ok(Value::Type(t));
        }
        Ok(Value::Type(it.typer.dealias(t)))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.simplified", |it, a| {
        let t = it.type_arg(a, 0)?;
        let t = it.typer.zonk(t);
        Ok(Value::Type(it.typer.normalize(t)))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.classSymbol", |it, a| {
        let t = it.type_arg(a, 0)?;
        let t = it.typer.dealias(t);
        let s = it.type_symbol(t);
        match s {
            SymRef::Class(_) => it.make_some(Value::Sym(s)),
            _ => it.make_none(),
        }
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.typeSymbol", |it, a| {
        let t = it.type_arg(a, 0)?;
        if let Some(sym) = it.val_type_ref(t) {
            return Ok(Value::Sym(SymRef::Term(sym)));
        }
        Ok(Value::Sym(it.type_symbol(t)))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.termSymbol", |it, a| {
        let t = it.type_arg(a, 0)?;
        if it.val_type_ref(t).is_some() {
            return Ok(Value::Sym(SymRef::None));
        }
        Ok(Value::Sym(it.term_symbol(t)))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.isSingleton", |it, a| {
        let t = it.type_arg(a, 0)?;
        let t = it.typer.zonk(t);
        let single = match it.typer.types.get(t) {
            Type::Term(_) | Type::Select(..) | Type::Lit(_) | Type::This(_) => true,
            Type::Class(c, _) => it.syms().class(c).kind == ClassKind::Object,
            _ => false,
        };
        Ok(Value::Bool(single))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.memberType", |it, a| {
        let t = it.type_arg(a, 0)?;
        let s = it.sym_arg(a, 1)?;
        Ok(Value::Type(it.member_type_of(t, s)?))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.baseClasses", |it, a| {
        let t = it.type_arg(a, 0)?;
        let syms = it.base_classes(t);
        it.sym_values(syms)
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.baseType", |it, a| {
        let t = it.type_arg(a, 0)?;
        let s = it.sym_arg(a, 1)?;
        let t = it.typer.dealias(t);
        Ok(Value::Type(match s {
            SymRef::Class(c) => it.base_type_of(t, c).unwrap_or(ERROR),
            SymRef::Any => ANY,
            _ => ERROR,
        }))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.derivesFrom", |it, a| {
        let t = it.type_arg(a, 0)?;
        let s = it.sym_arg(a, 1)?;
        let t = it.typer.dealias(t);
        Ok(Value::Bool(match s {
            SymRef::Class(c) => it.typer.base_type(t, c).is_some(),
            SymRef::Any => true,
            _ => false,
        }))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.isFunctionType", |it, a| {
        let t = it.type_arg(a, 0)?;
        let t = it.typer.dealias(t);
        Ok(Value::Bool(matches!(it.typer.types.get(t), Type::Class(c, _) if it.typer.is_function_class(c))))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.isTupleN", |it, a| {
        let t = it.type_arg(a, 0)?;
        let t = it.typer.dealias(t);
        Ok(Value::Bool(matches!(it.typer.types.get(t), Type::Class(c, _) if it.typer.is_tuple_class(c))))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.select", |it, a| {
        let t = it.type_arg(a, 0)?;
        let s = it.sym_arg(a, 1)?;
        match s {
            SymRef::Term(sym) => Ok(Value::Type(it.typer.types.mk(Type::Select(t, sym)))),
            other => Ok(Value::Type(it.member_type_of(t, other)?)),
        }
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.appliedTo", |it, a| {
        let t = it.type_arg(a, 0)?;
        let args = match a.get(1) {
            Some(Value::Type(x)) => vec![*x],
            Some(other) => {
                let other = other.clone();
                it.type_list(&other)?
            }
            None => Vec::new(),
        };
        Ok(Value::Type(it.applied_type(t, &args)?))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.substituteTypes", |it, a| {
        let t = it.type_arg(a, 0)?;
        let from = it.list_items(&arg(a, 1))?;
        let to = it.type_list(&arg(a, 2))?;
        let mut subst: Subst = Vec::new();
        for (f, ty) in from.iter().zip(to) {
            if let Value::Sym(SymRef::TParam(p)) = f {
                subst.push((*p, ty));
            }
        }
        Ok(Value::Type(it.typer.types.subst(t, &subst)))
    });
    q!(it, "Reflect.TypeReprMethods.TypeRepr.typeArgs", |it, a| {
        let t = it.type_arg(a, 0)?;
        let t = it.typer.zonk(t);
        let args = match it.typer.types.get(t) {
            Type::Class(_, args) | Type::AppMember(_, args) | Type::Alias(_, args) | Type::AppParam(_, args) => it.typer.types.items(args).to_vec(),
            _ => Vec::new(),
        };
        it.type_values(args)
    });

    q!(it, "Reflect.ConstantType.apply", |it, a| {
        let c = it.constant_value(&arg(a, 1))?;
        let lit = match c {
            Value::Int(i) => LitVal::Int(i),
            Value::Long(l) => LitVal::Long(l),
            Value::Double(d) => LitVal::Double(d.to_bits()),
            Value::Bool(b) => LitVal::Bool(b),
            Value::Char(ch) => LitVal::Char(ch),
            Value::Str(s) => LitVal::Str(it.typer.interner.intern(&s)),
            other => {
                let shown = it.to_str(&other)?;
                return it.unsupported(format!("no literal type holds {}", shown));
            }
        };
        Ok(Value::Type(it.typer.types.lit(lit)))
    });
    q!(it, "Reflect.ConstantType.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let t = it.typer.zonk(t);
        let Type::Lit(l) = it.typer.types.get(t) else { return it.unsupported("ConstantType.unapply on a type that is no constant") };
        let v = it.literal(it.typer.types.lit_val(l));
        let c = it.make_constant(v)?;
        it.make_some(c)
    });
    q!(it, "Reflect.ConstantTypeMethods.ConstantType.constant", |it, a| {
        let t = it.type_arg(a, 0)?;
        let t = it.typer.zonk(t);
        let Type::Lit(l) = it.typer.types.get(t) else { return it.unsupported("constant of a type that is no constant") };
        let v = it.literal(it.typer.types.lit_val(l));
        it.make_constant(v)
    });
    q!(it, "Reflect.NamedTypeMethods.NamedType.qualifier", |it, a| {
        let t = it.type_arg(a, 0)?;
        Ok(Value::Type(it.named_parts(t)?.0))
    });
    q!(it, "Reflect.NamedTypeMethods.NamedType.name", |it, a| {
        let t = it.type_arg(a, 0)?;
        Ok(Value::string(it.named_parts(t)?.1))
    });
    q!(it, "Reflect.TermRef.apply", |it, a| {
        let q = it.type_arg(a, 1)?;
        let name = it.str_arg(a, 2)?;
        let n = it.typer.interner.intern(&name);
        match it.typer.find_member(q, n) {
            Some((s, _)) => Ok(Value::Type(it.typer.types.mk(Type::Select(q, s)))),
            None => {
                let shown = it.show_type_repr(q);
                it.unsupported(format!("{} has no member {}", shown, name))
            }
        }
    });
    q!(it, "Reflect.TermRef.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let (q, name) = it.named_parts(t)?;
        it.make_tuple(vec![Value::Type(q), Value::string(name)])
    });
    q!(it, "Reflect.TypeRef.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let (q, name) = it.named_parts(t)?;
        it.make_tuple(vec![Value::Type(q), Value::string(name)])
    });
    q!(it, "Reflect.TypeRefMethods.TypeRef.isOpaqueAlias", |it, a| {
        let t = it.type_arg(a, 0)?;
        Ok(Value::Bool(matches!(it.typer.types.get(t), Type::Class(c, _) if it.syms().class(c).kind == ClassKind::Opaque)))
    });
    q!(it, "Reflect.TypeRefMethods.TypeRef.translucentSuperType", |it, a| {
        let t = it.type_arg(a, 0)?;
        Ok(Value::Type(match it.typer.types.get(t) {
            Type::Class(c, _) if it.syms().class(c).kind == ClassKind::Opaque => it.syms().class(c).underlying.unwrap_or(t),
            _ => t,
        }))
    });
    q!(it, "Reflect.AppliedType.apply", |it, a| {
        let tycon = it.type_arg(a, 1)?;
        let args = it.type_list(&arg(a, 2))?;
        Ok(Value::Type(it.applied_type(tycon, &args)?))
    });
    q!(it, "Reflect.AppliedType.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let (tycon, args) = it.applied_parts(t)?;
        let list = it.type_values(args)?;
        it.make_tuple(vec![Value::Type(tycon), list])
    });
    q!(it, "Reflect.AppliedTypeMethods.AppliedType.tycon", |it, a| {
        let t = it.type_arg(a, 0)?;
        Ok(Value::Type(it.applied_parts(t)?.0))
    });
    q!(it, "Reflect.AppliedTypeMethods.AppliedType.args", |it, a| {
        let t = it.type_arg(a, 0)?;
        let args = it.applied_parts(t)?.1;
        it.type_values(args)
    });
    // An annotation changes no type for this compiler: `@nowarn` on a macro's val is its type.
    q!(it, "Reflect.AnnotatedType.apply", |it, a| Ok(Value::Type(it.type_arg(a, 1)?)));
    q!(it, "Reflect.AnnotatedType.unapply", |it, _a| it.unsupported("AnnotatedType.unapply: the types of this compiler carry no annotations"));
    q!(it, "Reflect.AndType.apply", |it, a| {
        let (x, y) = (it.type_arg(a, 1)?, it.type_arg(a, 2)?);
        Ok(Value::Type(it.typer.types.inter(x, y)))
    });
    q!(it, "Reflect.OrType.apply", |it, a| {
        let (x, y) = (it.type_arg(a, 1)?, it.type_arg(a, 2)?);
        Ok(Value::Type(it.typer.types.union(x, y)))
    });
    q!(it, "Reflect.AndType.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let Type::Inter(x, y) = it.typer.types.get(t) else { return it.unsupported("AndType.unapply on a type that is no intersection") };
        it.make_tuple(vec![Value::Type(x), Value::Type(y)])
    });
    q!(it, "Reflect.OrType.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let Type::Union(x, y) = it.typer.types.get(t) else { return it.unsupported("OrType.unapply on a type that is no union") };
        it.make_tuple(vec![Value::Type(x), Value::Type(y)])
    });
    q!(it, "Reflect.AndOrTypeMethods.AndOrType.left", |it, a| {
        let t = it.type_arg(a, 0)?;
        match it.typer.types.get(t) {
            Type::Inter(x, _) | Type::Union(x, _) => Ok(Value::Type(x)),
            _ => it.unsupported("left of a type that is no intersection or union"),
        }
    });
    q!(it, "Reflect.AndOrTypeMethods.AndOrType.right", |it, a| {
        let t = it.type_arg(a, 0)?;
        match it.typer.types.get(t) {
            Type::Inter(_, y) | Type::Union(_, y) => Ok(Value::Type(y)),
            _ => it.unsupported("right of a type that is no intersection or union"),
        }
    });
    q!(it, "Reflect.Refinement.unapply", |it, a| {
        let (parent, r) = it.refinement_arg(a, 1)?;
        let name = it.typer.name_ref(refinement_name(r)).to_string();
        let info = it.refinement_info(r)?;
        it.make_tuple(vec![Value::Type(parent), Value::string(name), Value::Type(info)])
    });
    q!(it, "Reflect.RefinementMethods.Refinement.parent", |it, a| Ok(Value::Type(it.refinement_arg(a, 0)?.0)));
    q!(it, "Reflect.RefinementMethods.Refinement.name", |it, a| {
        let (_, r) = it.refinement_arg(a, 0)?;
        Ok(Value::string(it.typer.name_ref(refinement_name(r)).to_string()))
    });
    q!(it, "Reflect.RefinementMethods.Refinement.info", |it, a| {
        let (_, r) = it.refinement_arg(a, 0)?;
        Ok(Value::Type(it.refinement_info(r)?))
    });
    q!(it, "Reflect.Refinement.apply", |it, a| {
        let parent = it.type_arg(a, 1)?;
        let name = it.str_arg(a, 2)?;
        let info = it.type_arg(a, 3)?;
        let name = it.typer.interner.intern(&name);
        let r = it.refinement_of_info(name, info)?;
        let r = it.typer.types.refine(r);
        Ok(Value::Type(it.typer.types.mk(Type::Refined(parent, r))))
    });
    q!(it, "Reflect.ByNameType.apply", |it, a| {
        let t = it.type_arg(a, 1)?;
        Ok(Value::Type(it.typer.by_name_type(t)))
    });
    q!(it, "Reflect.ByNameType.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let Some(u) = it.typer.by_name_arg(t) else { return it.unsupported("ByNameType.unapply on a type that is no by-name type") };
        it.make_some(Value::Type(u))
    });
    q!(it, "Reflect.ByNameTypeMethods.ByNameType.underlying", |it, a| {
        let t = it.type_arg(a, 0)?;
        let Some(u) = it.typer.by_name_arg(t) else { return it.unsupported("underlying of a type that is no by-name type") };
        Ok(Value::Type(u))
    });
    q!(it, "Reflect.ThisType.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let tref = it.this_tref(t)?;
        it.make_some(Value::Type(tref))
    });
    q!(it, "Reflect.ThisTypeMethods.ThisType.tref", |it, a| {
        let t = it.type_arg(a, 0)?;
        Ok(Value::Type(it.this_tref(t)?))
    });
    q!(it, "Reflect.TypeBounds.apply", |it, a| {
        let (lo, hi) = (it.type_arg(a, 1)?, it.type_arg(a, 2)?);
        Ok(Value::Type(it.typer.types.bounded_wild(lo, hi)))
    });
    q!(it, "Reflect.TypeBounds.unapply", |it, a| {
        let (lo, hi) = it.bounds_of(a, 1)?;
        it.make_tuple(vec![Value::Type(lo), Value::Type(hi)])
    });
    q!(it, "Reflect.TypeBoundsMethods.TypeBounds.low", |it, a| Ok(Value::Type(it.bounds_of(a, 0)?.0)));
    q!(it, "Reflect.TypeBoundsMethods.TypeBounds.hi", |it, a| Ok(Value::Type(it.bounds_of(a, 0)?.1)));
    q!(it, "Reflect.PolyType.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let Some(p) = it.poly_type_parts(t)? else { return it.unsupported("PolyType.unapply on a type that is no polymorphic method type") };
        let names = p.params.iter().map(|&tp| it.typer.name_ref(it.syms().tparam(tp).name).to_string()).collect();
        let names = it.str_values(names)?;
        let bounds = p.params.iter().map(|&tp| Value::Type(it.tparam_bounds(tp))).collect();
        let bounds = it.make_list(bounds)?;
        it.make_tuple(vec![names, bounds, Value::Type(p.result)])
    });
    q!(it, "Reflect.MethodType.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let Some(m) = it.method_type_parts(t)? else { return it.unsupported("MethodType.unapply on a type that is no method type") };
        let names = m.names.iter().map(|&n| it.typer.name_ref(n).to_string()).collect();
        let names = it.str_values(names)?;
        let types = it.make_list(m.types.into_iter().map(Value::Type).collect())?;
        it.make_tuple(vec![names, types, Value::Type(m.result)])
    });
    q!(it, "Reflect.MethodTypeMethods.MethodType.isImplicit", |it, a| {
        let t = it.type_arg(a, 0)?;
        let Some(m) = it.method_type_parts(t)? else { return it.unsupported("isImplicit of a type that is no method type") };
        Ok(Value::Bool(m.kind != ClauseKind::Plain))
    });
    q!(it, "Reflect.MethodTypeMethods.MethodType.isContextual", |it, a| {
        let t = it.type_arg(a, 0)?;
        let Some(m) = it.method_type_parts(t)? else { return it.unsupported("isContextual of a type that is no method type") };
        Ok(Value::Bool(m.kind == ClauseKind::Using))
    });
    q!(it, "Reflect.TypeLambda.apply", |it, a| {
        let names = it.list_items(&arg(a, 1))?;
        let (bounds_fn, body_fn) = (arg(a, 2), arg(a, 3));
        let mut params = Vec::with_capacity(names.len());
        for n in &names {
            let name = it.to_str(n)?;
            let name = it.typer.interner.intern(&name);
            let p = it.typer.syms.new_tparam(name, 0);
            params.push(it.typer.types.param(p));
        }
        let pl = it.typer.types.list(&params);
        let open = it.typer.types.mk(Type::Lambda(pl, ANY));
        let bounds = it.apply_value(bounds_fn, vec![Value::Type(open)])?;
        for (&p, b) in params.iter().zip(it.list_items(&bounds)?) {
            let (Type::Param(tp), Value::Type(b)) = (it.typer.types.get(p), b) else { continue };
            let (lo, hi) = it.typer.types.wild_bounds(b).unwrap_or((b, b));
            it.typer.syms.tparams[tp.idx()].lower = lo;
            it.typer.syms.tparams[tp.idx()].upper = hi;
        }
        let body = it.apply_value(body_fn, vec![Value::Type(open)])?;
        let Value::Type(body) = body else { return it.unsupported("a TypeLambda body that is no type") };
        let lambda = it.typer.types.mk(Type::Lambda(pl, body));
        it.note_lambda(lambda);
        Ok(Value::Type(lambda))
    });
    q!(it, "Reflect.TypeLambda.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let Type::Lambda(params, body) = it.typer.types.get(t) else { return it.unsupported("TypeLambda.unapply on a type that is no lambda") };
        it.note_lambda(t);
        let items = it.typer.types.items(params).to_vec();
        let mut names = Vec::new();
        let mut bounds = Vec::new();
        for p in &items {
            if let Type::Param(tp) = it.typer.types.get(*p) {
                names.push(it.typer.name_ref(it.syms().tparam(tp).name).to_string());
                bounds.push(Value::Type(it.tparam_bounds(tp)));
            }
        }
        let names = it.str_values(names)?;
        let bounds = it.make_list(bounds)?;
        it.make_tuple(vec![names, bounds, Value::Type(body)])
    });
    q!(it, "Reflect.TypeLambdaMethods.TypeLambda.param", |it, a| {
        let t = it.type_arg(a, 0)?;
        let Value::Int(i) = arg(a, 1) else { return it.unsupported("TypeLambda.param of that index") };
        let Type::Lambda(params, _) = it.typer.types.get(t) else { return it.unsupported("param of a type that is no lambda") };
        match it.typer.types.items(params).get(i as usize).copied() {
            Some(p) => Ok(Value::Type(p)),
            None => it.throw_named("IndexOutOfBoundsException", &i.to_string()),
        }
    });
    q!(it, "Reflect.TypeLambdaMethods.TypeLambda.paramBounds", |it, a| {
        let t = it.type_arg(a, 0)?;
        let bounds = it.lambda_param_bounds(t)?;
        it.make_list(bounds.into_iter().map(Value::Type).collect())
    });
    q!(it, "Reflect.LambdaTypeMethods.LambdaType.paramTypes", |it, a| {
        let t = it.type_arg(a, 0)?;
        if let Some(m) = it.method_type_parts(t)? {
            return it.make_list(m.types.into_iter().map(Value::Type).collect());
        }
        let bounds = it.lambda_param_bounds(t)?;
        it.make_list(bounds.into_iter().map(Value::Type).collect())
    });
    q!(it, "Reflect.TypeLambdaMethods.TypeLambda.paramVariances", |it, a| {
        let t = it.type_arg(a, 0)?;
        let Type::Lambda(params, body) = it.typer.types.get(t) else { return it.unsupported("paramVariances of a type that is no lambda") };
        let params = it.lambda_params(params);
        let mut out = Vec::new();
        for v in it.lambda_param_variances(&params, body) {
            out.push(it.make_flags(v)?);
        }
        it.make_list(out)
    });
    q!(it, "Reflect.ParamRef.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let (binder, n) = it.param_ref_parts(t)?;
        it.make_tuple(vec![Value::Type(binder), Value::Int(n)])
    });
    q!(it, "Reflect.ParamRefMethods.ParamRef.binder", |it, a| {
        let t = it.type_arg(a, 0)?;
        Ok(Value::Type(it.param_ref_parts(t)?.0))
    });
    q!(it, "Reflect.ParamRefMethods.ParamRef.paramNum", |it, a| {
        let t = it.type_arg(a, 0)?;
        Ok(Value::Int(it.param_ref_parts(t)?.1))
    });
    q!(it, "Reflect.LambdaTypeMethods.LambdaType.paramNames", |it, a| {
        let t = it.type_arg(a, 0)?;
        if let Some(m) = it.method_type_parts(t)? {
            let names = m.names.iter().map(|&n| it.typer.name_ref(n).to_string()).collect();
            return it.str_values(names);
        }
        let Type::Lambda(params, _) = it.typer.types.get(t) else { return it.unsupported("paramNames of a type that is no lambda") };
        let items = it.typer.types.items(params).to_vec();
        let mut names = Vec::new();
        for p in &items {
            if let Type::Param(tp) = it.typer.types.get(*p) {
                names.push(it.typer.name_ref(it.syms().tparam(tp).name).to_string());
            }
        }
        it.str_values(names)
    });
    q!(it, "Reflect.LambdaTypeMethods.LambdaType.resType", |it, a| {
        let t = it.type_arg(a, 0)?;
        if let Some(m) = it.method_type_parts(t)? {
            return Ok(Value::Type(m.result));
        }
        let Type::Lambda(_, body) = it.typer.types.get(t) else { return it.unsupported("resType of a type that is no lambda") };
        it.note_lambda(t);
        Ok(Value::Type(body))
    });
    q!(it, "Reflect.MatchType.unapply", |it, a| {
        let t = it.type_arg(a, 1)?;
        let Type::Match(s, m) = it.typer.types.get(t) else { return it.unsupported("MatchType.unapply on a type that is no match type") };
        let info = it.typer.types.match_info(m).clone();
        let cases: Vec<Value> = info.cases.iter().map(|c| Value::Type(c.body)).collect();
        let list = it.make_list(cases)?;
        it.make_tuple(vec![Value::Type(info.bound), Value::Type(s), list])
    });
}

impl<'a, 't> Interp<'a, 't> {
    /// The prefix and the name of a named type.
    fn named_parts(&mut self, t: TypeId) -> R<(TypeId, String)> {
        let t = self.typer.zonk(t);
        Ok(match self.typer.types.get(t) {
            Type::Class(c, _) | Type::Ctor(c) => {
                let (owner, name) = {
                    let info = self.syms().class(c);
                    (info.owner, info.name)
                };
                let prefix = self.prefix_of(owner);
                (prefix, self.typer.name_ref(name).to_string())
            }
            Type::Param(p) | Type::AppParam(p, _) => {
                let name = self.typer.name_ref(self.syms().tparam(p).name).to_string();
                (self.typer.types.blocked("<noprefix>"), name)
            }
            Type::Term(s) => {
                let (owner, name) = {
                    let info = self.syms().sym(s);
                    (info.owner, info.name)
                };
                let prefix = self.prefix_of(owner);
                (prefix, self.typer.name_ref(name).to_string())
            }
            Type::Select(p, s) => (p, self.typer.name_ref(self.syms().sym(s).name).to_string()),
            Type::Member(p, n) => (p, self.typer.name_ref(n).to_string()),
            Type::Alias(a, _) | Type::Decl(a) => {
                let (owner, name) = {
                    let info = &self.typer.syms.aliases[a.idx()];
                    (info.owner, info.name)
                };
                let prefix = self.prefix_of(owner);
                (prefix, self.typer.name_ref(name).to_string())
            }
            Type::Blocked(_) if matches!(self.pkg_form(t), Some((f, _)) if f != PkgForm::ThisType) => {
                let (form, pkg) = self.pkg_form(t).unwrap();
                self.pkg_named_parts(form, pkg)
            }
            Type::Any => (self.prefix_of(Owner::Package(self.typer.b.scala_pkg)), "Any".to_string()),
            Type::Nothing => (self.prefix_of(Owner::Package(self.typer.b.scala_pkg)), "Nothing".to_string()),
            Type::Lit(_) => {
                let s = self.type_symbol(t);
                let SymRef::Class(c) = s else { return self.unsupported("a literal type without a class") };
                let ct = self.typer.types.class(c, &[]);
                return self.named_parts(ct);
            }
            _ => {
                let shown = self.show_type_repr(t);
                return self.unsupported(format!("{} is no named type", shown));
            }
        })
    }

    fn applied_parts(&mut self, t: TypeId) -> R<(TypeId, Vec<TypeId>)> {
        let t = self.typer.zonk(t);
        match self.typer.types.get(t) {
            Type::Class(c, args) if args != EMPTY_LIST => Ok((self.typer.types.mk(Type::Ctor(c)), self.typer.types.items(args).to_vec())),
            Type::AppMember(m, args) => Ok((m, self.typer.types.items(args).to_vec())),
            Type::Alias(a, args) if args != EMPTY_LIST => Ok((self.typer.types.mk(Type::Alias(a, EMPTY_LIST)), self.typer.types.items(args).to_vec())),
            Type::AppParam(p, args) => Ok((self.typer.types.param(p), self.typer.types.items(args).to_vec())),
            _ => {
                let shown = self.show_type_repr(t);
                self.unsupported(format!("{} is no applied type", shown))
            }
        }
    }
}

// ---- scalac's internal methods ----

impl<'a, 't> Interp<'a, 't> {
    /// The methods of scalac's own classes that a library calls reflectively on the reflection
    /// API's values, for what the public API does not give (izumi-reflect's `_underlying` and
    /// `_info`), by the class name `getClass` gives the value.
    pub(super) fn internal_methods(qname: &str) -> &'static [&'static str] {
        match qname {
            "scala.quoted.Type" => &["underlying", "typeParams"],
            "scala.quoted.Quotes.reflectModule.Symbol" => &["denot", "info", "paramVariance"],
            _ => &[],
        }
    }

    fn lambda_params(&self, params: TList) -> Vec<TParamId> {
        self.typer.types.items(params).iter().filter_map(|&p| match self.typer.types.get(p) {
            Type::Param(p) => Some(p),
            _ => None,
        }).collect()
    }

    fn refinement_arg(&mut self, a: A, i: usize) -> R<(TypeId, Refinement)> {
        let t = self.type_arg(a, i)?;
        let t = self.typer.zonk(t);
        let Type::Refined(parent, r) = self.typer.types.get(t) else { return self.unsupported("a Refinement that is no refinement") };
        Ok((parent, self.typer.types.refinement(r)))
    }

    /// A refinement's member as scalac's `Refinement.info` has it: a type member as its
    /// `TypeBounds` (an alias's two alike), a `val` as its type, a parameterless `def` as a
    /// `ByNameType`, a method as its `MethodType`.
    fn refinement_info(&mut self, r: Refinement) -> R<TypeId> {
        match r {
            Refinement::Alias(_, ty) => Ok(self.typer.types.mk(Type::BoundedWild(ty, ty))),
            Refinement::Bounds(_, lo, hi) => Ok(self.typer.types.mk(Type::BoundedWild(lo, hi))),
            Refinement::Val(_, _, ty) => Ok(ty),
            Refinement::Term(_, sym, l) => {
                let items = self.typer.types.items(l).to_vec();
                let sig = self.typer.sig_arc(sym);
                if !sig.tparams.is_empty() {
                    return self.unsupported("the PolyType of a refinement's polymorphic method");
                }
                let mut result = *items.last().unwrap();
                if sig.clauses.is_empty() {
                    return Ok(self.typer.by_name_type(result));
                }
                let mut end = items.len() - 1;
                for clause in sig.clauses.iter().rev() {
                    let start = end - clause.params.len();
                    let names = clause.params.iter().map(|p| p.name).collect();
                    // A by-name parameter's type is a `ByNameType`, a repeated one's scalac's
                    // `<repeated>[A]`, a by-name repeated one's both (`=> <repeated>[A]`).
                    let types = clause
                        .params
                        .iter()
                        .zip(&items[start..end])
                        .map(|(p, &t)| {
                            let t = if p.repeated { self.typer.repeated_type(t) } else { t };
                            if p.by_name { self.typer.by_name_type(t) } else { t }
                        })
                        .collect();
                    result = self.method_type(MethodTypeRepr { names, types, result, open: Vec::new(), kind: ClauseKind::of(clause) });
                    end = start;
                }
                Ok(result)
            }
        }
    }

    /// The refinement `Refinement(parent, name, info)` makes: `refinement_info` read backwards.
    fn refinement_of_info(&mut self, name: crate::intern::Name, info: TypeId) -> R<Refinement> {
        if let Some((lo, hi)) = self.typer.types.wild_bounds(info) {
            return Ok(if lo == hi { Refinement::Alias(name, hi) } else { Refinement::Bounds(name, lo, hi) });
        }
        let span = self.macro_ctx.as_ref().map_or(Span::default(), |c| c.site_span);
        let file = self.typer.env.file;
        let mut clauses = Vec::new();
        let mut ret = info;
        let kind = if let Some(u) = self.typer.by_name_arg(info) {
            ret = u;
            SymKind::Def
        } else if self.method_type_parts(info)?.is_some() {
            while let Some(m) = self.method_type_parts(ret)? {
                let mut clause = ClauseSig { is_using: m.kind == ClauseKind::Using, is_implicit: m.kind == ClauseKind::Implicit, ..ClauseSig::default() };
                for (&pname, &written) in m.names.iter().zip(&m.types) {
                    let (ty, by_name) = match self.typer.by_name_arg(written) {
                        Some(t) => (t, true),
                        None => (written, false),
                    };
                    let (ty, repeated) = match self.typer.repeated_arg(ty) {
                        Some(t) => (t, true),
                        None => (ty, false),
                    };
                    let local_ty = match (repeated, self.typer.seq_class()) {
                        (true, Some(seq)) => self.typer.types.class(seq, &[ty]),
                        _ => ty,
                    };
                    let p = self.typer.syms.new_sym(pname, SymKind::Param, 0, Owner::Local, file, None, span);
                    let psig = self.typer.value_sig(local_ty);
                    let mut pinfo = self.typer.syms.sym_mut(p);
                    pinfo.sig = Some(psig);
                    pinfo.state().done_fresh();
                    pinfo.by_name = by_name;
                    clause.params.push(ParamSig { name: pname, ty, by_name, repeated, has_default: false, sym: p });
                }
                clauses.push(clause);
                ret = m.result;
            }
            SymKind::Def
        } else {
            SymKind::Val
        };
        let sym = self.typer.syms.new_sym(name, kind, crate::ast::mods::ABSTRACT, Owner::Local, file, None, span);
        let sig = Arc::new(MethodSig { tparams: Vec::new(), clauses, ret });
        let mut s = self.typer.syms.sym_mut(sym);
        s.sig = Some(sig.clone());
        s.state().done_fresh();
        Ok(if kind == SymKind::Val { Refinement::Val(name, sym, ret) } else { Refinement::Term(name, sym, self.typer.sig_types(&sig)) })
    }

    fn lambda_param_bounds(&mut self, t: TypeId) -> R<Vec<TypeId>> {
        let Type::Lambda(params, _) = self.typer.types.get(t) else { return self.unsupported("the parameters of a type that is no lambda") };
        let params = self.lambda_params(params);
        Ok(params.into_iter().map(|p| self.tparam_bounds(p)).collect())
    }

    /// The parameters of `lambda` are `ParamRef`s bound by it from here on.
    fn note_lambda(&mut self, lambda: TypeId) {
        let Type::Lambda(params, _) = self.typer.types.get(lambda) else { return };
        #[cfg(debug_assertions)]
        crate::types::view::cached("InterpCaches::lambda_of", lambda);
        for p in self.lambda_params(params) {
            self.lambda_of.insert(p, lambda);
        }
    }

    fn param_ref_parts(&mut self, t: TypeId) -> R<(TypeId, i32)> {
        let t = self.typer.zonk(t);
        let Type::Param(p) = self.typer.types.get(t) else { return self.unsupported("a ParamRef that is no lambda parameter") };
        let Some(&lambda) = self.lambda_of.get(&p) else { return self.unsupported("a ParamRef that is no lambda parameter") };
        #[cfg(debug_assertions)]
        crate::types::view::cached("InterpCaches::lambda_of", lambda);
        let Type::Lambda(params, _) = self.typer.types.get(lambda) else { return self.unsupported("a ParamRef that is no lambda parameter") };
        let n = self.lambda_params(params).iter().position(|&q| q == p).unwrap_or(0);
        Ok((lambda, n as i32))
    }

    fn tparam_bounds(&mut self, p: TParamId) -> TypeId {
        let (lo, hi) = (self.syms().tparam(p).lower, self.syms().tparam(p).upper);
        self.typer.types.bounded_wild(lo, hi)
    }

    /// The variances of a lambda's parameters: those declared, else the structural ones of
    /// scalac's `setStructuralVariances`, each starting bivariant and narrowed by every
    /// occurrence in the body.
    fn lambda_param_variances(&mut self, params: &[TParamId], body: TypeId) -> Vec<u64> {
        if params.iter().any(|&p| self.syms().tparam(p).variance != 0) {
            return params.iter().map(|&p| variance_flags(self.syms().tparam(p).variance)).collect();
        }
        let mut out = vec![F_COVARIANT | F_CONTRAVARIANT; params.len()];
        self.narrow_variances(body, 1, params, &mut out, 0);
        out
    }

    fn narrow_variances(&mut self, t: TypeId, v: i8, params: &[TParamId], out: &mut [u64], depth: u32) {
        if depth > 64 {
            return;
        }
        let t = self.typer.zonk(t);
        let occurs = |out: &mut [u64], p: TParamId, v: i8| {
            if let Some(i) = params.iter().position(|&q| q == p) {
                out[i] &= variance_flags(v);
            }
        };
        match self.typer.types.get(t) {
            Type::Param(p) => occurs(out, p, v),
            Type::Class(c, args) => {
                self.typer.complete_class_tparams(c);
                let variances: Vec<i8> = self.syms().class(c).tparams.iter().map(|&q| self.syms().tparam(q).variance).collect();
                let args: Vec<TypeId> = self.typer.types.items(args).to_vec();
                for (i, a) in args.into_iter().enumerate() {
                    self.narrow_variances(a, v * variances.get(i).copied().unwrap_or(0), params, out, depth + 1);
                }
            }
            Type::AppParam(p, args) => {
                occurs(out, p, v);
                let variances = self.syms().tparam(p).hk_variances.clone();
                let args: Vec<TypeId> = self.typer.types.items(args).to_vec();
                for (i, a) in args.into_iter().enumerate() {
                    self.narrow_variances(a, v * variances.get(i).copied().unwrap_or(0), params, out, depth + 1);
                }
            }
            Type::Union(a, b) | Type::Inter(a, b) => {
                self.narrow_variances(a, v, params, out, depth + 1);
                self.narrow_variances(b, v, params, out, depth + 1);
            }
            Type::BoundedWild(lo, hi) => {
                self.narrow_variances(lo, -v, params, out, depth + 1);
                self.narrow_variances(hi, v, params, out, depth + 1);
            }
            Type::Lambda(_, b) | Type::Poly(_, b) => self.narrow_variances(b, v, params, out, depth + 1),
            Type::Refined(parent, _) => self.narrow_variances(parent, v, params, out, depth + 1),
            Type::AppVar(_, args) | Type::AppMember(_, args) | Type::Alias(_, args) => {
                let args: Vec<TypeId> = self.typer.types.items(args).to_vec();
                for a in args {
                    self.narrow_variances(a, 0, params, out, depth + 1);
                }
            }
            _ => {}
        }
    }

    /// `TypeRef.underlying(ctx)`: a class's own type stands for its `ClassInfo`, an abstract
    /// type or a type parameter is its bounds, an alias what it names. `Symbol.denot(ctx)` is
    /// the symbol, whose `info(ctx)` is its declared type, the bounds for a type parameter.
    pub(super) fn internal_call(&mut self, recv: &Value, name: &str, _args: &[Value]) -> Option<R> {
        match (recv, name) {
            (Value::Type(t), "underlying") => {
                let t = self.typer.zonk(*t);
                // A term's singleton is its declared type, which may be another singleton.
                if let Type::Term(s) | Type::Select(_, s) = self.typer.types.get(t) {
                    return Some(Ok(Value::Type(self.typer.sig_of(s).ret)));
                }
                // An object's type stands for its module val's `TermRef` here; the val's
                // declared type is the module class, whose `ThisType` stands for it.
                if let Type::Class(c, _) = self.typer.types.get(t) {
                    if self.syms().class(c).kind == ClassKind::Object {
                        return Some(Ok(Value::Type(self.typer.types.mk(Type::This(c)))));
                    }
                }
                let under = match self.type_symbol(t) {
                    SymRef::Alias(a) => {
                        self.typer.complete_alias(a);
                        let info = &self.typer.syms.aliases[a.idx()];
                        if info.is_abstract() { WILD } else { self.typer.dealias(t) }
                    }
                    SymRef::TParam(p) => self.tparam_bounds(p),
                    _ => t,
                };
                Some(Ok(Value::Type(under)))
            }
            // `HKTypeLambda.typeParams`: its parameters as their symbols; no other type has it.
            (Value::Type(t), "typeParams") => {
                let t = self.typer.zonk(*t);
                let Type::Lambda(params, body) = self.typer.types.get(t) else {
                    return Some(self.throw_named("NoSuchMethodException", "typeParams"));
                };
                let params: Vec<TParamId> = self.typer.types.items(params).iter().filter_map(|&p| match self.typer.types.get(p) {
                    Type::Param(p) => Some(p),
                    _ => None,
                }).collect();
                self.note_lambda(t);
                let variances = self.lambda_param_variances(&params, body);
                let mut syms = Vec::with_capacity(params.len());
                for (&p, v) in params.iter().zip(variances) {
                    self.lambda_variances.insert(p, v);
                    syms.push(Value::Sym(SymRef::TParam(p)));
                }
                Some(self.make_list(syms))
            }
            (Value::Sym(SymRef::TParam(p)), "paramVariance") => {
                let bits = match self.lambda_variances.get(p) {
                    Some(&v) => v,
                    None => variance_flags(self.syms().tparam(*p).variance),
                };
                Some(self.make_flags(bits))
            }
            (Value::Sym(_), "denot") => Some(Ok(recv.clone())),
            (Value::Sym(s), "info") => Some(match self.live_sym(*s) {
                Err(stale) => Err(stale),
                Ok(SymRef::TParam(p)) => Ok(Value::Type(self.tparam_bounds(p))),
                Ok(SymRef::Term(sym)) => Ok(Value::Type(self.typer.sig_of(sym).ret)),
                Ok(other) => self.type_ref_of(other).map(Value::Type),
            }),
            _ => None,
        }
    }
}

// ---- symbols ----

const F_ABSTRACT: u64 = 1 << 0;
const F_CASE: u64 = 1 << 2;
const F_CASE_ACCESSOR: u64 = 1 << 3;
/// A declared variance as scalac's flags: `Covariant`, `Contravariant`, or none.
fn variance_flags(v: i8) -> u64 {
    match v {
        1 => F_COVARIANT,
        -1 => F_CONTRAVARIANT,
        _ => 0,
    }
}

const F_CONTRAVARIANT: u64 = 1 << 4;
const F_COVARIANT: u64 = 1 << 5;
const F_DEFERRED: u64 = 1 << 6;
const F_ENUM: u64 = 1 << 7;
const F_EXTENSION: u64 = 1 << 10;
const F_FIELD_ACCESSOR: u64 = 1 << 11;
const F_FINAL: u64 = 1 << 12;
const F_GIVEN: u64 = 1 << 13;
const F_HAS_DEFAULT: u64 = 1 << 14;
const F_IMPLICIT: u64 = 1 << 15;
const F_INFIX: u64 = 1 << 16;
const F_INLINE: u64 = 1 << 17;
const F_INVARIANT: u64 = 1 << 18;
const F_JAVA_DEFINED: u64 = 1 << 19;
const F_LAZY: u64 = 1 << 22;
const F_LOCAL: u64 = 1 << 23;
const F_MACRO: u64 = 1 << 24;
const F_METHOD: u64 = 1 << 25;
const F_MODULE: u64 = 1 << 26;
const F_MUTABLE: u64 = 1 << 27;
const F_OPAQUE: u64 = 1 << 29;
const F_OPEN: u64 = 1 << 30;
const F_OVERRIDE: u64 = 1 << 31;
const F_PACKAGE: u64 = 1 << 32;
const F_PARAM: u64 = 1 << 33;
const F_PARAM_ACCESSOR: u64 = 1 << 34;
const F_PRIVATE: u64 = 1 << 35;
const F_PROTECTED: u64 = 1 << 37;
const F_SEALED: u64 = 1 << 39;
const F_STABLE: u64 = 1 << 40;
const F_SYNTHETIC: u64 = 1 << 42;
const F_TRAIT: u64 = 1 << 43;
const F_TRANSPARENT: u64 = 1 << 44;

impl<'a, 't> Interp<'a, 't> {
    /// `s`, or the diagnostic of a symbol of another expansion's site, which names an entry of an
    /// owner chain that is gone: whatever reads a symbol's entry asks here first, rendering
    /// included.
    pub(super) fn live_sym(&self, s: SymRef) -> R<SymRef> {
        match s {
            SymRef::Site(run, _) | SymRef::Splice(run) if !self.is_current_run(run) => self.stale(STALE_SITE),
            _ => Ok(s),
        }
    }

    /// Whether `run` numbers the expansion under way, whose owner chain the symbols of a site
    /// index; a symbol of another expansion's site names an entry that is gone.
    fn is_current_run(&self, run: u32) -> bool {
        self.macro_ctx.as_ref().is_some_and(|c| c.run == run)
    }

    fn current_run(&self) -> u32 {
        self.macro_ctx.as_ref().map_or(0, |c| c.run)
    }

    /// The chain entry of the expansion under way that `s` is, with its index.
    fn site_index(&self, s: SymRef) -> Option<usize> {
        let ctx = self.macro_ctx.as_ref()?;
        let owners = &ctx.owners;
        match s {
            SymRef::Site(run, i) => (run == ctx.run).then_some(i as usize),
            SymRef::Term(sym) => owners.iter().rposition(|&o| o == SiteOwner::Sym(sym)),
            SymRef::Class(c) => owners.iter().rposition(|&o| o == SiteOwner::Dummy(c)),
            _ => None,
        }
    }

    fn site_entry(&self, run: u32, i: usize) -> Option<SiteOwner> {
        let ctx = self.macro_ctx.as_ref()?;
        if ctx.run != run {
            return None;
        }
        ctx.owners.get(i).copied()
    }

    fn site_ref(&self, i: usize) -> SymRef {
        match self.site_entry(self.current_run(), i) {
            Some(SiteOwner::Sym(s)) => SymRef::Term(s),
            Some(_) => SymRef::Site(self.current_run(), i as u32),
            None => SymRef::None,
        }
    }

    /// Where the type of an anonymous given starts, which scalac places its symbol at.
    fn after_given_keyword(&self, file: FileId, start: u32) -> u32 {
        let text = self.typer.source(file).text.as_bytes();
        let mut at = start as usize;
        if text.get(at..at + 5) == Some(&b"given"[..]) {
            at += 5;
            while text.get(at).is_some_and(|b| b.is_ascii_whitespace()) {
                at += 1;
            }
        }
        at as u32
    }

    /// The first of the top-level definitions that scalac wraps in the `file$package` object.
    fn file_package_start(&self, f: FileId) -> Option<u32> {
        let ast = &self.typer.asts[f.0 as usize];
        let syms = self.typer.def_syms.entries_in(f.0 as usize);
        let classes = self.typer.def_classes.entries_in(f.0 as usize);
        let terms = syms.iter().filter(|(_, s)| matches!(self.syms().sym(*s).owner, Owner::Package(_))).map(|(d, _)| *d);
        let givens = classes
            .iter()
            .filter(|(_, c)| self.syms().class(*c).kind == ClassKind::GivenImpl && matches!(self.syms().class(*c).owner, Owner::Package(_)))
            .map(|(d, _)| *d);
        terms.chain(givens).map(|d| ast.def(d).span.start).min()
    }

    /// The `$1$`, `$2$` of scalac's pattern temporaries, counted along the chain.
    fn pattern_temp_number(&self, i: usize) -> usize {
        let owners = self.macro_ctx.as_ref().map_or(&[][..], |c| &c.owners[..]);
        owners[..=i.min(owners.len().saturating_sub(1))].iter().filter(|o| matches!(o, SiteOwner::PatternTemp(_))).count()
    }

    fn innermost_site_sym(&self) -> Option<SymId> {
        self.macro_ctx.as_ref()?.owners.iter().rev().find_map(|&o| match o {
            SiteOwner::Sym(s) => Some(s),
            _ => None,
        })
    }

    /// The name a definition has in scalac: an anonymous given keeps the name it is made up
    /// from where it is one of several overloads.
    fn source_name(&self, sym: SymId) -> Name {
        let info = self.syms().sym(sym);
        if info.kind == SymKind::Given && info.mods & crate::ast::mods::ANONYMOUS != 0 {
            if let Some(d) = info.def {
                return self.typer.asts[info.file.0 as usize].def(d).name;
            }
        }
        info.name
    }

    pub(super) fn symbol_name(&mut self, s: SymRef) -> String {
        match s {
            SymRef::Term(sym) => self.typer.name_ref(self.source_name(sym)).to_string(),
            SymRef::Class(c) => {
                let info = self.syms().class(c);
                let name = self.typer.name_ref(info.name).to_string();
                if matches!(info.kind, ClassKind::Object | ClassKind::GivenImpl) { format!("{}$", name) } else { name }
            }
            SymRef::Splice(_) => "macro".to_string(),
            SymRef::Site(run, i) => match self.site_entry(run, i as usize) {
                Some(SiteOwner::Dummy(c)) => format!("<local {}>", self.symbol_name(SymRef::Class(c))),
                Some(SiteOwner::Lambda(_)) => "$anonfun".to_string(),
                Some(SiteOwner::Binder(n, _)) => self.typer.name_ref(n).to_string(),
                Some(SiteOwner::PatternTemp(_)) => format!("${}$", self.pattern_temp_number(i as usize)),
                Some(SiteOwner::Sym(sym)) => self.symbol_name(SymRef::Term(sym)),
                None => "<none>".to_string(),
            },
            SymRef::Pkg(p) | SymRef::PkgClass(p) => {
                if p == ROOT_PKG { "<empty>".to_string() } else { self.typer.name_ref(self.syms().pkg(p).name).to_string() }
            }
            SymRef::Root => "<root>".to_string(),
            SymRef::TParam(p) => self.typer.name_ref(self.syms().tparam(p).name).to_string(),
            SymRef::Alias(a) => self.typer.name_ref(self.typer.syms.aliases[a.idx()].name).to_string(),
            SymRef::Ctor(_) => "<init>".to_string(),
            SymRef::Default(_, i) => format!("$lessinit$greater$default${}", i + 1),
            SymRef::Lambda(_) => "$anonfun".to_string(),
            SymRef::FilePackage(f) => {
                let path = &self.typer.source(f).path;
                let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
                format!("{}$package$", name.strip_suffix(".scala").unwrap_or(name))
            }
            SymRef::Any => "Any".to_string(),
            SymRef::Nothing => "Nothing".to_string(),
            SymRef::None => "<none>".to_string(),
        }
    }

    /// The owner of a local definition: the entry before it in the owner chain of the
    /// expansion, or for one off the chain, a parameter say, the innermost definition on it.
    fn local_owner(&self, s: SymRef) -> SymRef {
        match self.site_index(s) {
            Some(0) => self.macro_ctx.as_ref().and_then(|c| c.owner_class).map_or(SymRef::None, SymRef::Class),
            Some(i) => self.site_ref(i - 1),
            None => match self.innermost_site_sym() {
                Some(o) if SymRef::Term(o) != s => SymRef::Term(o),
                _ => SymRef::None,
            },
        }
    }

    fn symbol_owner(&mut self, s: SymRef) -> SymRef {
        match s {
            SymRef::Term(sym) => {
                let info = self.syms().sym(sym);
                match info.owner {
                    Owner::Class(c) => SymRef::Class(c),
                    // A top-level object is a member of its package, as its class is.
                    Owner::Package(p) if matches!(info.kind, SymKind::Object(_)) => SymRef::PkgClass(p),
                    Owner::Package(_) => SymRef::FilePackage(info.file),
                    Owner::Local => self.local_owner(s),
                }
            }
            SymRef::Class(c) => {
                let info = self.syms().class(c);
                match info.owner {
                    Owner::Class(o) => SymRef::Class(o),
                    Owner::Package(_) if matches!(info.kind, ClassKind::GivenImpl | ClassKind::Opaque) => SymRef::FilePackage(info.file),
                    Owner::Package(p) => SymRef::PkgClass(p),
                    Owner::Local => self.local_owner(s),
                }
            }
            SymRef::Splice(run) if !self.is_current_run(run) => SymRef::None,
            SymRef::Splice(_) => {
                let n = self.macro_ctx.as_ref().map_or(0, |c| c.owners.len());
                match n {
                    0 => self.macro_ctx.as_ref().and_then(|c| c.owner_class).map_or(SymRef::None, SymRef::Class),
                    n => self.site_ref(n - 1),
                }
            }
            SymRef::Site(run, i) => match self.site_entry(run, i as usize) {
                Some(SiteOwner::Dummy(c)) => SymRef::Class(c),
                _ => self.local_owner(s),
            },
            // Packages hang off the root; the empty package is the root's too.
            SymRef::Pkg(p) => match self.syms().pkg(p).parent {
                Some(parent) if parent != ROOT_PKG => SymRef::PkgClass(parent),
                _ => SymRef::Root,
            },
            SymRef::PkgClass(p) => match self.syms().pkg(p).parent {
                Some(parent) if parent != ROOT_PKG => SymRef::PkgClass(parent),
                _ => SymRef::Root,
            },
            SymRef::Root => SymRef::None,
            SymRef::Alias(a) => {
                let info = &self.typer.syms.aliases[a.idx()];
                match info.owner {
                    Owner::Class(o) => SymRef::Class(o),
                    Owner::Package(_) => SymRef::FilePackage(info.file),
                    Owner::Local => SymRef::None,
                }
            }
            SymRef::Ctor(c) | SymRef::Default(c, _) => SymRef::Class(c),
            SymRef::FilePackage(f) => {
                let p = self.typer.file_pkgs.get(f.0 as usize).copied().unwrap_or(ROOT_PKG);
                SymRef::PkgClass(p)
            }
            SymRef::Lambda(_) => self.innermost_site_sym().map_or(SymRef::None, SymRef::Term),
            SymRef::TParam(_) | SymRef::Any | SymRef::Nothing => SymRef::PkgClass(self.typer.b.scala_pkg),
            SymRef::None => SymRef::None,
        }
    }

    fn is_class_like(s: SymRef) -> bool {
        matches!(s, SymRef::Class(_) | SymRef::PkgClass(_) | SymRef::Pkg(_) | SymRef::FilePackage(_) | SymRef::Root | SymRef::None)
    }

    /// scalac's full name: the owners' up to the innermost class, then the name, with a `_$`
    /// in front of it for each term owner skipped.
    /// scalac's `Signature` of a method: a type parameter clause as its length, a parameter
    /// as the name of its erased type, and the erased result; `NotAMethod` for anything else.
    fn symbol_signature(&mut self, s: SymRef) -> (Vec<Value>, String) {
        let SymRef::Term(sym) = s else { return (Vec::new(), String::new()) };
        let info = self.syms().sym(sym);
        if !matches!(info.kind, SymKind::Def) {
            return (Vec::new(), String::new());
        }
        // The method whose body the macro expands in, its result being inferred, has its
        // parameters' signature already; any other is completed, or waited for in another worker.
        let completing_here = info.state().get() == crate::symbols::Completion::InProgress && info.state().mine();
        let sig = if completing_here { info.sig.clone() } else { Some(self.typer.sig_arc(sym)) };
        let Some(sig) = sig.filter(|sig| !sig.clauses.is_empty() || !sig.tparams.is_empty()) else {
            return (Vec::new(), String::new());
        };
        let mut params = Vec::new();
        if !sig.tparams.is_empty() {
            params.push(Value::Int(sig.tparams.len() as i32));
        }
        for clause in &sig.clauses {
            for p in &clause.params {
                let name = if p.repeated {
                    "scala.collection.immutable.Seq".to_string()
                } else if p.by_name {
                    "scala.Function0".to_string()
                } else {
                    self.erased_sig_name(p.ty)
                };
                params.push(Value::string(name));
            }
        }
        let result = self.erased_sig_name(sig.ret);
        (params, result)
    }

    /// The name scalac's signatures give the erasure of a type.
    fn erased_sig_name(&mut self, t: TypeId) -> String {
        let t = self.typer.dealias(t);
        match self.typer.types.get(t) {
            Type::Class(c, args) if c == self.typer.b.array => match self.typer.types.items(args).first().copied() {
                Some(elem) => format!("{}[]", self.erased_sig_name(elem)),
                None => "java.lang.Object".to_string(),
            },
            Type::Class(c, _) if c == self.typer.b.any_ref => "java.lang.Object".to_string(),
            Type::Class(c, _) => {
                let name = self.symbol_full_name(SymRef::Class(c));
                if self.syms().class(c).kind == ClassKind::Object { format!("{}$", name) } else { name }
            }
            Type::Nothing => "scala.runtime.Nothing$".to_string(),
            _ if t == self.typer.b.t_null => "scala.runtime.Null$".to_string(),
            Type::Param(p) => {
                let hi = self.syms().tparam(p).upper;
                if hi != t && hi != ANY { self.erased_sig_name(hi) } else { "java.lang.Object".to_string() }
            }
            _ => "java.lang.Object".to_string(),
        }
    }

    fn symbol_full_name(&mut self, s: SymRef) -> String {
        match s {
            SymRef::Pkg(p) | SymRef::PkgClass(p) => {
                if p == ROOT_PKG {
                    return "<empty>".to_string();
                }
                let path = self.typer.class_path_prefix(Owner::Package(p));
                path.trim_end_matches('.').to_string()
            }
            SymRef::Root => "<root>".to_string(),
            SymRef::Any => "scala.Any".to_string(),
            SymRef::Nothing => "scala.Nothing".to_string(),
            SymRef::None => "<none>".to_string(),
            SymRef::TParam(_) | SymRef::Lambda(_) => self.symbol_name(s),
            SymRef::Class(c) if self.is_std_scala_class(c) => {
                let name = self.symbol_name(s);
                let pkg = if name == "String" { "java.lang" } else { Self::scala_library_package(&name) };
                format!("{}.{}", pkg, name)
            }
            other => {
                let mut name = self.symbol_name(other);
                if let SymRef::Site(run, i) = other {
                    if let Some(SiteOwner::PatternTemp(_)) = self.site_entry(run, i as usize) {
                        name.pop();
                    }
                }
                let mut encl = self.symbol_owner(other);
                let mut filler = String::new();
                for _ in 0..64 {
                    if Self::is_class_like(encl) {
                        break;
                    }
                    encl = self.symbol_owner(encl);
                    filler.push_str("_$");
                }
                match encl {
                    SymRef::None | SymRef::Root | SymRef::Pkg(ROOT_PKG) | SymRef::PkgClass(ROOT_PKG) => filler + &name,
                    o => format!("{}.{}{}", self.symbol_full_name(o), filler, name),
                }
            }
        }
    }

    fn symbol_flags(&mut self, s: SymRef) -> u64 {
        use crate::ast::mods;
        let mut f = 0u64;
        match s {
            SymRef::Term(sym) => {
                let info = self.syms().sym(sym);
                let m = info.mods;
                if m & mods::PRIVATE != 0 { f |= F_PRIVATE }
                if m & mods::PROTECTED != 0 { f |= F_PROTECTED }
                if m & mods::FINAL != 0 { f |= F_FINAL }
                if m & mods::LAZY != 0 { f |= F_LAZY }
                if m & mods::OVERRIDE != 0 { f |= F_OVERRIDE }
                if m & mods::INLINE != 0 { f |= F_INLINE }
                if m & mods::IMPLICIT != 0 { f |= F_IMPLICIT }
                if m & mods::GIVEN != 0 || info.kind == SymKind::Given { f |= F_GIVEN }
                if m & mods::INFIX != 0 { f |= F_INFIX }
                if m & mods::TRANSPARENT != 0 { f |= F_TRANSPARENT }
                if m & mods::CASE != 0 { f |= F_CASE }
                if m & mods::ENUM != 0 { f |= F_ENUM }
                if info.is_extension { f |= F_EXTENSION }
                if info.java_defined { f |= F_JAVA_DEFINED }
                match info.kind {
                    SymKind::Given if m & mods::LAZY != 0 => f |= F_STABLE,
                    SymKind::Def | SymKind::Given => f |= F_METHOD,
                    SymKind::Var => f |= F_MUTABLE,
                    SymKind::Param => {
                        f |= F_PARAM;
                        if let Owner::Class(c) = info.owner {
                            f |= F_PARAM_ACCESSOR;
                            if self.syms().class(c).mods & mods::CASE != 0 {
                                f |= F_CASE_ACCESSOR;
                            }
                        }
                    }
                    SymKind::Object(_) => f |= F_MODULE | F_STABLE | F_FINAL,
                    SymKind::EnumValue(_) => f |= F_ENUM | F_CASE | F_STABLE,
                    SymKind::Val => f |= F_STABLE,
                    SymKind::Overloaded(_) => {}
                }
                if matches!(info.kind, SymKind::Val | SymKind::Var) && matches!(info.owner, Owner::Class(_)) { f |= F_FIELD_ACCESSOR }
                if info.owner == Owner::Local { f |= F_LOCAL }
                if info.mods & mods::INLINE != 0 && info.def.is_some() {
                    if let Some(def_id) = info.def {
                        let file = info.file;
                        if let crate::ast::DefKind::Fun(fun) = &self.typer.asts[file.0 as usize].def(def_id).kind {
                            if let Some(body) = fun.body {
                                if matches!(self.typer.asts[file.0 as usize].expr(body), crate::ast::Expr::Splice(_)) {
                                    f |= F_MACRO;
                                }
                            }
                        }
                    }
                }
                // A constructor parameter as `paramSymss` hands it out stands for the field.
                let field = self.typer.reflect_ctor_params.get(&sym).copied().unwrap_or(sym);
                let has_default = matches!(info.kind, SymKind::Param) && {
                    let sig_default = match info.owner {
                        Owner::Class(c) => self.syms().class(c).ctor.iter().flat_map(|cl| cl.params.iter()).any(|p| (p.sym == sym || p.sym == field) && p.has_default),
                        _ => false,
                    };
                    sig_default
                };
                if has_default { f |= F_HAS_DEFAULT }
            }
            SymRef::Class(c) => {
                let info = self.syms().class(c);
                let m = info.mods;
                if m & mods::PRIVATE != 0 { f |= F_PRIVATE }
                if m & mods::PROTECTED != 0 { f |= F_PROTECTED }
                if m & mods::FINAL != 0 { f |= F_FINAL }
                if m & mods::SEALED != 0 { f |= F_SEALED }
                if m & mods::ABSTRACT != 0 { f |= F_ABSTRACT }
                if m & mods::CASE != 0 { f |= F_CASE }
                if m & mods::OPEN != 0 { f |= F_OPEN }
                match info.kind {
                    ClassKind::Trait => f |= F_TRAIT | F_ABSTRACT,
                    ClassKind::Object | ClassKind::GivenImpl => f |= F_MODULE | F_FINAL,
                    ClassKind::Enum => f |= F_ENUM | F_SEALED | F_ABSTRACT,
                    ClassKind::EnumCase => f |= F_ENUM | F_CASE | F_FINAL,
                    ClassKind::Opaque => f |= F_OPAQUE,
                    ClassKind::Anon => f |= F_FINAL | F_SYNTHETIC,
                    _ => {}
                }
                if info.kind == ClassKind::Object && info.def.is_some() {
                    let is_case = m & mods::CASE != 0;
                    if is_case { f |= F_CASE }
                }
            }
            SymRef::Pkg(_) | SymRef::PkgClass(_) | SymRef::Root => f |= F_PACKAGE | F_MODULE,
            SymRef::Splice(_) => f |= F_SYNTHETIC | F_MACRO,
            SymRef::Site(run, i) => match self.site_entry(run, i as usize) {
                Some(SiteOwner::Lambda(_)) => f |= F_METHOD | F_SYNTHETIC,
                Some(SiteOwner::PatternTemp(_)) => f |= F_SYNTHETIC,
                _ => {}
            },
            SymRef::TParam(p) => {
                f |= F_PARAM | F_DEFERRED;
                match self.syms().tparam(p).variance {
                    1 => f |= F_COVARIANT,
                    -1 => f |= F_CONTRAVARIANT,
                    _ => f |= F_INVARIANT,
                }
            }
            SymRef::Alias(a) => {
                if self.typer.syms.aliases[a.idx()].is_abstract() { f |= F_DEFERRED }
            }
            SymRef::Ctor(_) | SymRef::Default(..) => f |= F_METHOD | F_SYNTHETIC,
            SymRef::Lambda(_) => f |= F_METHOD | F_SYNTHETIC,
            SymRef::FilePackage(_) => f |= F_MODULE | F_SYNTHETIC | F_FINAL,
            SymRef::Any | SymRef::Nothing => f |= F_ABSTRACT,
            SymRef::None => {}
        }
        f
    }

    fn make_flags(&mut self, bits: u64) -> R {
        let c = self.quoted_class(&["scala", "quoted", "Reflect", "Flags"])?;
        self.construct_new(c, vec![Value::Long(bits as i64)], &Frame::new(None))
    }

    fn flags_bits(&mut self, v: &Value) -> R<u64> {
        let c = self.quoted_class(&["scala", "quoted", "Reflect", "Flags"])?;
        let field = self.syms().class(c).ctor_syms.first().and_then(|f| f.first().copied());
        match (v, field) {
            (Value::Obj(o), Some(f)) if o.class == c => match self.get_field(v.clone(), f)? {
                Value::Long(l) => Ok(l as u64),
                _ => Ok(0),
            },
            _ => self.unsupported("Flags were expected"),
        }
    }

    /// The class a path names: `scala.List`, `scala.collection.immutable.List`, `a.B.C` for a
    /// nested class, `scala.None$` for a module class.
    fn class_by_path(&mut self, path: &str) -> Option<SymRef> {
        let want_module = path.ends_with('$');
        let path = path.trim_end_matches('$');
        let segs: Vec<&str> = path.split('.').collect();
        let mut p = ROOT_PKG;
        let mut i = 0;
        while i < segs.len() - 1 {
            let n = self.typer.interner.intern(segs[i]);
            match self.typer.demand_pkg(p, n) {
                Some(sub) => {
                    p = sub;
                    i += 1;
                }
                None => break,
            }
        }
        let n = self.typer.interner.intern(segs[i]);
        let mut class = match self.typer.pkg_type(p, n) {
            Some(TypeRef::Class(c)) => c,
            _ => match self.typer.pkg_term(p, n) {
                Some(r) => match r.sym().map(|s| self.syms().sym(s).kind) {
                    Some(SymKind::Object(c)) => c,
                    _ => return None,
                },
                None => return None,
            },
        };
        i += 1;
        while i < segs.len() {
            let n = self.typer.interner.intern(segs[i]);
            self.typer.complete_class(class);
            let nested = self.syms().class(class).nested.get(&n).copied();
            let companion = self.syms().class(class).companion.and_then(|k| self.syms().class(k).nested.get(&n).copied());
            class = nested.or(companion)?;
            i += 1;
        }
        if want_module && self.syms().class(class).kind != ClassKind::Object {
            class = self.syms().class(class).companion?;
        }
        Some(SymRef::Class(class))
    }

    fn module_by_path(&mut self, path: &str) -> Option<SymRef> {
        match self.class_by_path(&format!("{}$", path))? {
            SymRef::Class(c) if self.syms().class(c).kind == ClassKind::Object => Some(self.module_sym(c)),
            _ => None,
        }
    }

    fn package_by_path(&mut self, path: &str) -> Option<SymRef> {
        let mut p = ROOT_PKG;
        for seg in path.split('.').filter(|s| !s.is_empty()) {
            let n = self.typer.interner.intern(seg);
            p = self.typer.demand_pkg(p, n)?;
        }
        Some(SymRef::Pkg(p))
    }

    /// The class whose members a symbol's member lookups read: the class itself, or the class
    /// of a module, as scalac reads a module's members through its module class.
    fn member_holder(&self, s: SymRef) -> Option<ClassId> {
        match s {
            SymRef::Class(c) => Some(c),
            SymRef::Term(sym) => match self.syms().sym(sym).kind {
                SymKind::Object(c) => Some(c),
                _ => None,
            },
            _ => None,
        }
    }

    fn class_members(&mut self, c: ClassId, inherited: bool) -> Vec<SymId> {
        self.typer.complete_class(c);
        let mut out: Vec<SymId> = Vec::new();
        let classes: Vec<ClassId> = if inherited {
            self.syms().class(c).base_types.iter().map(|&(b, _)| b).collect()
        } else {
            vec![c]
        };
        for k in classes {
            self.typer.complete_class(k);
            for &m in &self.syms().class(k).member_order {
                let info = self.syms().sym(m);
                if matches!(info.kind, SymKind::Overloaded(_)) {
                    if let Some(alts) = self.syms().alternatives(m) {
                        for &a in alts {
                            if !out.contains(&a) {
                                out.push(a);
                            }
                        }
                    }
                    continue;
                }
                if !out.iter().any(|&x| self.syms().sym(x).name == info.name && k != c) && !out.contains(&m) {
                    out.push(m);
                }
            }
        }
        out
    }

    fn is_field_sym(&self, s: SymId) -> bool {
        let info = self.syms().sym(s);
        matches!(info.kind, SymKind::Val | SymKind::Var | SymKind::Param | SymKind::Object(_) | SymKind::EnumValue(_))
            || self.is_given_val(s)
    }

    /// A given without parameters, which scalac makes a lazy val.
    fn is_given_val(&self, s: SymId) -> bool {
        let info = self.syms().sym(s);
        info.kind == SymKind::Given && info.mods & crate::ast::mods::LAZY != 0
    }

    fn is_method_sym_of(&self, s: SymId) -> bool {
        matches!(self.syms().sym(s).kind, SymKind::Def | SymKind::Given) && !self.is_given_val(s)
    }

    fn param_symss(&mut self, s: SymRef) -> Vec<Vec<SymRef>> {
        match s {
            SymRef::Term(sym) => {
                let sig = self.typer.sig_of(sym);
                let mut out = Vec::new();
                if !sig.tparams.is_empty() {
                    out.push(sig.tparams.iter().map(|&p| SymRef::TParam(p)).collect());
                }
                for clause in &sig.clauses {
                    out.push(clause.params.iter().map(|p| SymRef::Term(p.sym)).collect());
                }
                out
            }
            SymRef::Ctor(c) => {
                self.typer.complete_class(c);
                let (tparams, clauses) = {
                    let info = self.syms().class(c);
                    (info.tparams.clone(), info.ctor_syms.clone())
                };
                let mut out: Vec<Vec<SymRef>> = Vec::new();
                if !tparams.is_empty() {
                    out.push(tparams.into_iter().map(SymRef::TParam).collect());
                }
                for clause in clauses {
                    out.push(clause.into_iter().map(|p| SymRef::Term(self.reflect_ctor_param(p))).collect());
                }
                out
            }
            _ => Vec::new(),
        }
    }

    /// The annotations of a definition as the trees that instantiate them.
    fn annotation_trees(&mut self, s: SymRef) -> R<Vec<TreeRef>> {
        let (file, owner, annots): (FileId, Owner, Vec<crate::ast::Annot>) = match s {
            SymRef::Term(sym) => {
                let info = self.syms().sym(sym);
                let (file, owner, def) = (info.file, info.owner, info.def);
                let annots = match (def, info.kind) {
                    (Some(d), _) => self.typer.asts[file.0 as usize].def(d).annots.clone(),
                    (None, SymKind::Param) => match owner {
                        Owner::Class(c) => self.ctor_param_annots(c, sym),
                        _ => Vec::new(),
                    },
                    _ => Vec::new(),
                };
                (file, owner, annots)
            }
            SymRef::Class(c) => {
                let info = self.syms().class(c);
                let (file, owner, def) = (info.file, info.owner, info.def);
                let annots = match def {
                    Some(d) => self.typer.asts[file.0 as usize].def(d).annots.clone(),
                    None => Vec::new(),
                };
                (file, owner, annots)
            }
            _ => return Ok(Vec::new()),
        };
        let mut out = Vec::new();
        for a in annots {
            let ast = &self.typer.asts[file.0 as usize];
            let Some((path, _)) = ast.annot_new(&a) else { continue };
            let env = self.typer.env_at(file, owner, u32::MAX);
            let mark = self.typer.diags.items.len();
            let known = self.typer.with_env(env, |t| {
                let class = match t.cur_ast().ty(path) {
                    crate::ast::TyExpr::Name(n) => t.lookup_type(n),
                    crate::ast::TyExpr::Select(q, n) => t.lookup_type_in_path(q, n),
                    _ => None,
                };
                let is_class = matches!(class, Some(TypeRef::Class(c)) if t.syms.class(c).kind == ClassKind::Class);
                if !is_class {
                    return None;
                }
                Some(t.type_expr(a.instance, None).0)
            });
            if self.typer.diags.items.len() > mark {
                self.typer.diags.items.truncate(mark);
                continue;
            }
            if let Some(te) = known {
                out.push(TreeRef::Expr(te));
            }
        }
        Ok(out)
    }

    /// A class parameter is also the field of its name; the constructor's parameter is a
    /// symbol of its own, which the parameter's annotations belong to.
    fn reflect_ctor_param(&mut self, field: SymId) -> SymId {
        if self.syms().sym(field).kind == SymKind::Param {
            return field;
        }
        if let Some(&p) = self.typer.reflect_param_of.get(&field) {
            return p;
        }
        let mut info = self.syms().sym(field).info.clone();
        info.kind = SymKind::Param;
        let p = SymId(self.typer.syms.syms.len() as u32);
        self.typer.syms.syms.push(info);
        self.typer.reflect_ctor_params.insert(p, field);
        self.typer.reflect_param_of.insert(field, p);
        p
    }

    fn ctor_param_annots(&mut self, c: ClassId, sym: SymId) -> Vec<crate::ast::Annot> {
        let sym = self.typer.reflect_ctor_params.get(&sym).copied().unwrap_or(sym);
        let info = self.syms().class(c);
        let (file, def) = (info.file, info.def);
        let Some(d) = def else { return Vec::new() };
        let ast = &self.typer.asts[file.0 as usize];
        let crate::ast::DefKind::Class(cls) = &ast.def(d).kind else { return Vec::new() };
        let mut index = 0;
        for clause in &cls.clauses {
            for p in &clause.params {
                if self.syms().class(c).ctor_syms.iter().flatten().nth(index) == Some(&sym) {
                    return ast.param_annots(p).to_vec();
                }
                index += 1;
            }
        }
        Vec::new()
    }
}

fn install_symbols(it: &mut Table) {
    q!(it, "Reflect.Symbol.spliceOwner", |it, _a| {
        let run = it.macro_ctx()?.run;
        Ok(Value::Sym(SymRef::Splice(run)))
    });
    q!(it, "Reflect.Symbol.noSymbol", |_it, _a| Ok(Value::Sym(SymRef::None)));
    q!(it, "Reflect.Symbol.classSymbol", |it, a| {
        let path = it.str_arg(a, 1)?;
        match it.class_by_path(&path) {
            Some(s) => Ok(Value::Sym(s)),
            None => match &*path {
                "scala.Any" => Ok(Value::Sym(SymRef::Any)),
                "scala.Nothing" => Ok(Value::Sym(SymRef::Nothing)),
                _ => Ok(Value::Sym(SymRef::None)),
            },
        }
    });
    q!(it, "Reflect.Symbol.requiredClass", |it, a| {
        let path = it.str_arg(a, 1)?;
        match it.class_by_path(&path) {
            Some(s) => Ok(Value::Sym(s)),
            None => it.throw_named("NoSuchElementException", &format!("class {} not found", path)),
        }
    });
    q!(it, "Reflect.Symbol.requiredModule", |it, a| {
        let path = it.str_arg(a, 1)?;
        match it.module_by_path(&path) {
            Some(s) => Ok(Value::Sym(s)),
            None => it.throw_named("NoSuchElementException", &format!("object {} not found", path)),
        }
    });
    q!(it, "Reflect.Symbol.requiredPackage", |it, a| {
        let path = it.str_arg(a, 1)?;
        match it.package_by_path(&path) {
            Some(s) => Ok(Value::Sym(s)),
            None => it.throw_named("NoSuchElementException", &format!("package {} not found", path)),
        }
    });
    q!(it, "Reflect.Symbol.requiredMethod", |it, a| {
        let path = it.str_arg(a, 1)?;
        let (owner, name) = match path.rfind('.') {
            Some(i) => (path[..i].to_string(), path[i + 1..].to_string()),
            None => return it.throw_named("NoSuchElementException", &format!("method {} not found", path)),
        };
        let n = it.typer.interner.intern(&name);
        let found = match it.module_by_path(&owner).or_else(|| it.class_by_path(&owner)) {
            Some(SymRef::Term(s)) => match it.syms().sym(s).kind {
                SymKind::Object(c) => it.class_members(c, true).into_iter().find(|&m| it.syms().sym(m).name == n),
                _ => None,
            },
            Some(SymRef::Class(c)) => it.class_members(c, true).into_iter().find(|&m| it.syms().sym(m).name == n),
            _ => it.package_by_path(&owner).and_then(|p| match p {
                SymRef::Pkg(p) => it.typer.pkg_term(p, n).and_then(|r| r.sym()),
                _ => None,
            }),
        };
        match found {
            Some(s) => Ok(Value::Sym(SymRef::Term(s))),
            None => it.throw_named("NoSuchElementException", &format!("method {} not found", path)),
        }
    });
    q!(it, "Reflect.Symbol.newVal", |it, a| {
        let name = it.str_arg(a, 2)?;
        let ty = it.type_arg(a, 3)?;
        let bits = it.flags_bits(&arg(a, 4))?;
        let n = it.typer.interner.intern(&name);
        let span = it.macro_ctx.as_ref().map_or(Span::default(), |c| c.site_span);
        let kind = if bits & F_MUTABLE != 0 { SymKind::Var } else { SymKind::Val };
        let s = it.typer.new_local(n, kind, ty, span);
        Ok(Value::Sym(SymRef::Term(s)))
    });
    q!(it, "Reflect.Symbol.newBind", |it, a| {
        let name = it.str_arg(a, 2)?;
        let ty = it.type_arg(a, 4)?;
        let n = it.typer.interner.intern(&name);
        let span = it.macro_ctx.as_ref().map_or(Span::default(), |c| c.site_span);
        let s = it.typer.new_local(n, SymKind::Val, ty, span);
        Ok(Value::Sym(SymRef::Term(s)))
    });
    q!(it, "Reflect.Symbol.newMethod", |it, a| {
        let name = it.str_arg(a, 2)?;
        let ty = it.type_arg(a, 3)?;
        let n = it.typer.interner.intern(&name);
        let span = it.macro_ctx.as_ref().map_or(Span::default(), |c| c.site_span);
        let (params, ret) = match it.typer.types.get(ty) {
            Type::Class(c, args) if it.typer.is_function_class(c) => {
                let items = it.typer.types.items(args).to_vec();
                let k = items.len() - 1;
                (items[..k].to_vec(), items[k])
            }
            _ => (Vec::new(), ty),
        };
        let s = it.typer.new_local_method(n, &params, ret, span);
        Ok(Value::Sym(SymRef::Term(s)))
    });
    // The number is a hash of the expansion's call site (its file's tag and its offset) and the
    // count of fresh names the expansion made before, so that it is a function of the program:
    // neither the interpreter's call count (which a native standing in for a body changes) nor
    // the order of the build's expansions (which a watch rebuild of one file changes) reaches it.
    // Seven digits at most: the output-stability checks fold runs of eight hex digits.
    q!(it, "Reflect.Symbol.freshName", |it, a| {
        let prefix = it.str_arg(a, 1)?;
        let (tag, offset, count) = match &it.macro_ctx {
            Some(c) => {
                let n = it.typer.quote.fresh.entry((c.unit_file, c.unit_span.start)).or_insert(0);
                *n += 1;
                (it.typer.prog.file_tags.get(c.unit_file.0 as usize).copied().unwrap_or(0), c.unit_span.start, *n)
            }
            None => {
                it.fresh_names += 1;
                (0, 0, it.fresh_names)
            }
        };
        // A program file's tag has 32 bits, a library body's 64.
        let mut bytes: Vec<u8> = if tag >> 32 == 0 { (tag as u32).to_le_bytes().to_vec() } else { tag.to_le_bytes().to_vec() };
        bytes.extend(offset.to_le_bytes());
        bytes.extend(count.to_le_bytes());
        let number = bytes.into_iter().fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193)) % 10_000_000;
        Ok(Value::string(format!("{}${}", prefix, number)))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.owner", |it, a| {
        let s = it.sym_arg(a, 0)?;
        match it.symbol_owner(s) {
            SymRef::None => it.throw_named("NoSuchElementException", "no owner"),
            o => Ok(Value::Sym(o)),
        }
    });
    q!(it, "Reflect.SymbolMethods.Symbol.maybeOwner", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Sym(it.symbol_owner(s)))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.flags", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let bits = it.symbol_flags(s);
        it.make_flags(bits)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.privateWithin", |it, _a| it.make_none());
    q!(it, "Reflect.SymbolMethods.Symbol.protectedWithin", |it, _a| it.make_none());
    q!(it, "Reflect.SymbolMethods.Symbol.name", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::string(it.symbol_name(s)))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.fullName", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::string(it.symbol_full_name(s)))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.pos", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let point = |file: FileId, at: u32| Some(Value::Pos(file, at, at));
        let site_file = it.macro_ctx.as_ref().map(|c| c.site_file);
        let pos = match s {
            SymRef::Term(sym) => {
                let info = it.syms().sym(sym);
                let anonymous = info.mods & crate::ast::mods::ANONYMOUS != 0;
                let (file, start) = (info.file, info.span.start);
                point(file, if anonymous { it.after_given_keyword(file, start) } else { start })
            }
            SymRef::Class(c) => {
                let info = it.syms().class(c);
                let (file, start, given) = (info.file, info.span.start, info.kind == ClassKind::GivenImpl);
                point(file, if given { it.after_given_keyword(file, start) } else { start })
            }
            SymRef::Site(run, i) => match it.site_entry(run, i as usize) {
                Some(SiteOwner::Dummy(c)) => point(it.syms().class(c).file, 0),
                Some(SiteOwner::Lambda(span) | SiteOwner::Binder(_, span) | SiteOwner::PatternTemp(span)) => site_file.and_then(|f| point(f, span.start)),
                _ => None,
            },
            SymRef::Splice(run) => it.macro_ctx.as_ref().filter(|c| c.run == run).and_then(|c| point(c.site_file, c.site_span.start)),
            SymRef::FilePackage(f) => it.file_package_start(f).and_then(|at| point(f, at)),
            _ => None,
        };
        it.make_option(pos)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.tree", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Tree(match s {
            SymRef::Term(sym) => {
                let kind = it.syms().sym(sym).kind;
                match kind {
                    SymKind::Def | SymKind::Given => {
                        it.typer.ensure_body(sym);
                        match it.typer.fun_of_sym.get(&sym).copied() {
                            Some(f) => TreeRef::Fun(f),
                            None => TreeRef::Def(sym),
                        }
                    }
                    _ => TreeRef::Def(sym),
                }
            }
            SymRef::Class(c) => TreeRef::Class(c),
            other => it.sym_tree(other),
        }))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.hasAnnotation", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let annot = it.sym_arg(a, 1)?;
        let trees = it.annotation_trees(s)?;
        for t in trees {
            let ty = it.tree_type(t)?;
            if it.type_symbol(ty) == annot {
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.getAnnotation", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let annot = it.sym_arg(a, 1)?;
        let trees = it.annotation_trees(s)?;
        for t in trees {
            let ty = it.tree_type(t)?;
            if it.type_symbol(ty) == annot {
                return it.make_some(Value::Tree(t));
            }
        }
        it.make_none()
    });
    q!(it, "Reflect.SymbolMethods.Symbol.annotations", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let trees = it.annotation_trees(s)?;
        it.tree_values(trees)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isDefinedInCurrentRun", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(match s {
            SymRef::Term(sym) => !it.typer.source(it.syms().sym(sym).file).is_std,
            SymRef::Class(c) => !it.typer.source(it.syms().class(c).file).is_std,
            _ => false,
        }))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isLocalDummy", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(match s {
            SymRef::Site(run, i) => matches!(it.site_entry(run, i as usize), Some(SiteOwner::Dummy(_))),
            _ => false,
        }))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isRefinementClass", |_it, _a| Ok(Value::Bool(false)));
    q!(it, "Reflect.SymbolMethods.Symbol.isAliasType", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(matches!(s, SymRef::Alias(a) if !it.typer.syms.aliases[a.idx()].is_abstract())))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isAnonymousClass", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(matches!(s, SymRef::Class(c) if matches!(it.syms().class(c).kind, ClassKind::Anon | ClassKind::GivenImpl))))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isAnonymousFunction", |_it, a| Ok(Value::Bool(matches!(a.first(), Some(Value::Sym(SymRef::Lambda(_)))))));
    q!(it, "Reflect.SymbolMethods.Symbol.isAbstractType", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(match s {
            SymRef::Alias(a) => it.typer.syms.aliases[a.idx()].is_abstract(),
            SymRef::TParam(_) => true,
            _ => false,
        }))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isClassConstructor", |_it, a| Ok(Value::Bool(matches!(a.first(), Some(Value::Sym(SymRef::Ctor(_)))))));
    q!(it, "Reflect.SymbolMethods.Symbol.isSuperAccessor", |_it, _a| Ok(Value::Bool(false)));
    q!(it, "Reflect.SymbolMethods.Symbol.isType", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(matches!(s, SymRef::Class(_) | SymRef::TParam(_) | SymRef::Alias(_) | SymRef::Any | SymRef::Nothing | SymRef::FilePackage(_) | SymRef::PkgClass(_) | SymRef::Root)))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isTerm", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(matches!(s, SymRef::Term(_) | SymRef::Pkg(_) | SymRef::Ctor(_) | SymRef::Default(..) | SymRef::Lambda(_) | SymRef::Splice(_) | SymRef::Site(..))))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isPackageDef", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(matches!(s, SymRef::Pkg(_) | SymRef::PkgClass(_) | SymRef::Root)))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isClassDef", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(matches!(s, SymRef::Class(_) | SymRef::Any | SymRef::Nothing | SymRef::FilePackage(_) | SymRef::PkgClass(_) | SymRef::Root)))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isTypeDef", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(matches!(s, SymRef::Alias(_) | SymRef::TParam(_))))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isValDef", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(match s {
            SymRef::Term(sym) => it.is_field_sym(sym),
            SymRef::Splice(_) => true,
            SymRef::Site(run, i) => !matches!(it.site_entry(run, i as usize), Some(SiteOwner::Lambda(_))),
            _ => false,
        }))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isDefDef", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(match s {
            SymRef::Term(sym) => it.is_method_sym_of(sym),
            SymRef::Ctor(_) | SymRef::Default(..) | SymRef::Lambda(_) => true,
            SymRef::Site(run, i) => matches!(it.site_entry(run, i as usize), Some(SiteOwner::Lambda(_))),
            _ => false,
        }))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isBind", |_it, _a| Ok(Value::Bool(false)));
    q!(it, "Reflect.SymbolMethods.Symbol.isNoSymbol", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(s == SymRef::None))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.isTypeParam", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Bool(matches!(s, SymRef::TParam(_))))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.declaredField", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let name = it.str_arg(a, 1)?;
        let n = it.typer.interner.intern(&name);
        let Some(c) = it.member_holder(s) else { return Ok(Value::Sym(SymRef::None)) };
        let found = it.class_members(c, false).into_iter().find(|&m| it.syms().sym(m).name == n && it.is_field_sym(m));
        Ok(Value::Sym(found.map_or(SymRef::None, SymRef::Term)))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.declaredFields", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let Some(c) = it.member_holder(s) else { return it.make_list(Vec::new()) };
        let members: Vec<SymRef> = it.class_members(c, false).into_iter().filter(|&m| it.is_field_sym(m)).map(SymRef::Term).collect();
        it.sym_values(members)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.fieldMember", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let name = it.str_arg(a, 1)?;
        let n = it.typer.interner.intern(&name);
        let Some(c) = it.member_holder(s) else { return Ok(Value::Sym(SymRef::None)) };
        let found = it.class_members(c, true).into_iter().find(|&m| it.syms().sym(m).name == n && it.is_field_sym(m));
        Ok(Value::Sym(found.map_or(SymRef::None, SymRef::Term)))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.fieldMembers", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let Some(c) = it.member_holder(s) else { return it.make_list(Vec::new()) };
        let members: Vec<SymRef> = it.class_members(c, true).into_iter().filter(|&m| it.is_field_sym(m)).map(SymRef::Term).collect();
        it.sym_values(members)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.declaredMethod", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let name = it.str_arg(a, 1)?;
        let n = it.typer.interner.intern(&name);
        let Some(c) = it.member_holder(s) else { return it.make_list(Vec::new()) };
        if let Some(rest) = name.strip_prefix("$lessinit$greater$default$") {
            let i: u32 = rest.parse().unwrap_or(0);
            let companion = it.syms().class(c).companion;
            let target = if it.syms().class(c).kind == ClassKind::Object { companion } else { Some(c) };
            if let (Some(k), true) = (target, i > 0) {
                it.typer.complete_class(k);
                let has_default = it.syms().class(k).ctor.iter().flat_map(|cl| cl.params.iter()).nth(i as usize - 1).map_or(false, |p| p.has_default);
                if has_default {
                    return it.sym_values(vec![SymRef::Default(k, i - 1)]);
                }
            }
            return it.make_list(Vec::new());
        }
        let members: Vec<SymRef> = it.class_members(c, false).into_iter().filter(|&m| it.syms().sym(m).name == n && it.is_method_sym_of(m)).map(SymRef::Term).collect();
        it.sym_values(members)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.declaredMethods", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let Some(c) = it.member_holder(s) else { return it.make_list(Vec::new()) };
        let members: Vec<SymRef> = it.class_members(c, false).into_iter().filter(|&m| it.is_method_sym_of(m)).map(SymRef::Term).collect();
        it.sym_values(members)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.methodMember", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let name = it.str_arg(a, 1)?;
        let n = it.typer.interner.intern(&name);
        let Some(c) = it.member_holder(s) else { return it.make_list(Vec::new()) };
        let members: Vec<SymRef> = it.class_members(c, true).into_iter().filter(|&m| it.syms().sym(m).name == n && it.is_method_sym_of(m)).map(SymRef::Term).collect();
        it.sym_values(members)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.methodMembers", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let Some(c) = it.member_holder(s) else { return it.make_list(Vec::new()) };
        let members: Vec<SymRef> = it.class_members(c, true).into_iter().filter(|&m| it.is_method_sym_of(m)).map(SymRef::Term).collect();
        it.sym_values(members)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.declaredType", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let name = it.str_arg(a, 1)?;
        let n = it.typer.interner.intern(&name);
        let Some(c) = it.member_holder(s) else { return it.make_list(Vec::new()) };
        it.typer.complete_class(c);
        let mut out = Vec::new();
        if let Some(&k) = it.syms().class(c).nested.get(&n) {
            out.push(SymRef::Class(k));
        }
        if let Some(&al) = it.syms().class(c).type_aliases.get(&n) {
            out.push(SymRef::Alias(al));
        }
        it.sym_values(out)
    });
    // A class's type parameters are its first declared types, as scalac enters them.
    q!(it, "Reflect.SymbolMethods.Symbol.declaredTypes", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let Some(c) = it.member_holder(s) else { return it.make_list(Vec::new()) };
        it.typer.complete_class(c);
        let mut out: Vec<SymRef> = it.syms().class(c).own_tparams().iter().map(|&p| SymRef::TParam(p)).collect();
        out.extend(it.syms().class(c).nested.values().map(|&k| SymRef::Class(k)));
        out.extend(it.syms().class(c).type_aliases.values().map(|&al| SymRef::Alias(al)));
        it.sym_values(out)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.typeMember", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let name = it.str_arg(a, 1)?;
        let n = it.typer.interner.intern(&name);
        let Some(c) = it.member_holder(s) else { return Ok(Value::Sym(SymRef::None)) };
        it.typer.complete_class(c);
        let found = it.syms().class(c).nested.get(&n).map(|&k| SymRef::Class(k)).or_else(|| it.syms().class(c).type_aliases.get(&n).map(|&al| SymRef::Alias(al)));
        Ok(Value::Sym(found.unwrap_or(SymRef::None)))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.typeMembers", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let Some(c) = it.member_holder(s) else { return it.make_list(Vec::new()) };
        it.typer.complete_class(c);
        let mut out: Vec<SymRef> = it.syms().class(c).nested.values().map(|&k| SymRef::Class(k)).collect();
        out.extend(it.syms().class(c).type_aliases.values().map(|&al| SymRef::Alias(al)));
        it.sym_values(out)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.declarations", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let Some(c) = it.member_holder(s) else { return it.make_list(Vec::new()) };
        let mut out: Vec<SymRef> = it.class_members(c, false).into_iter().map(SymRef::Term).collect();
        out.extend(it.syms().class(c).nested.values().map(|&k| SymRef::Class(k)));
        it.sym_values(out)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.paramSymss", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let clauses = it.param_symss(s);
        let mut lists = Vec::new();
        for c in clauses {
            lists.push(it.sym_values(c)?);
        }
        it.make_list(lists)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.allOverriddenSymbols", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let mut out = Vec::new();
        if let SymRef::Term(sym) = s {
            let info = it.syms().sym(sym);
            if let (Owner::Class(c), name) = (info.owner, info.name) {
                it.typer.complete_class(c);
                let bases: Vec<ClassId> = it.syms().class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
                for b in bases {
                    it.typer.complete_class(b);
                    if let Some(&m) = it.syms().class(b).members.get(&name) {
                        out.push(Value::Sym(SymRef::Term(m)));
                    }
                }
            }
        }
        let list = it.make_list(out)?;
        it.call_by_name(list, "iterator", Vec::new())
    });
    q!(it, "Reflect.SymbolMethods.Symbol.overridingSymbol", |_it, _a| Ok(Value::Sym(SymRef::None)));
    q!(it, "Reflect.SymbolMethods.Symbol.primaryConstructor", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Sym(match s {
            SymRef::Class(c) => SymRef::Ctor(c),
            _ => SymRef::None,
        }))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.caseFields", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let Some(c) = it.member_holder(s) else { return it.make_list(Vec::new()) };
        it.typer.complete_class(c);
        let fields: Vec<SymRef> = if it.syms().class(c).mods & crate::ast::mods::CASE != 0 {
            it.syms().class(c).ctor_syms.first().map_or(Vec::new(), |f| f.iter().map(|&p| SymRef::Term(p)).collect())
        } else {
            Vec::new()
        };
        it.sym_values(fields)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.paramVariance", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let bits = match s {
            SymRef::TParam(p) => match it.syms().tparam(p).variance {
                1 => F_COVARIANT,
                -1 => F_CONTRAVARIANT,
                _ => 0,
            },
            _ => 0,
        };
        it.make_flags(bits)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.signature", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let (params, result) = it.symbol_signature(s);
        let params = it.make_list(params)?;
        let Some(c) = it.reflect_class("Signature") else { return it.unsupported("Signature is not in the program") };
        it.construct_new(c, vec![params, Value::string(result)], &Frame::new(None))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.moduleClass", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Sym(match s {
            SymRef::Term(sym) => match it.syms().sym(sym).kind {
                SymKind::Object(c) => SymRef::Class(c),
                _ => SymRef::None,
            },
            SymRef::Class(c) if it.syms().class(c).kind == ClassKind::Object => SymRef::Class(c),
            SymRef::Pkg(p) | SymRef::PkgClass(p) => SymRef::PkgClass(p),
            _ => SymRef::None,
        }))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.companionClass", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Sym(match s {
            SymRef::Class(c) => {
                it.typer.complete_class(c);
                match it.syms().class(c).companion {
                    Some(k) => SymRef::Class(k),
                    None => it.implied_companion(c),
                }
            }
            SymRef::Term(sym) => match it.syms().sym(sym).kind {
                SymKind::Object(c) => {
                    it.typer.complete_class(c);
                    it.syms().class(c).companion.map_or(SymRef::None, SymRef::Class)
                }
                _ => SymRef::None,
            },
            _ => SymRef::None,
        }))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.companionModule", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Sym(match s {
            SymRef::Class(c) => {
                it.typer.complete_class(c);
                if it.syms().class(c).kind == ClassKind::Object {
                    it.module_sym(c)
                } else {
                    match it.syms().class(c).companion {
                        Some(k) => it.module_sym(k),
                        None => it.implied_companion(c),
                    }
                }
            }
            SymRef::Term(sym) if matches!(it.syms().sym(sym).kind, SymKind::Object(_)) => s,
            SymRef::Pkg(p) | SymRef::PkgClass(p) => SymRef::Pkg(p),
            _ => SymRef::None,
        }))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.children", |it, a| {
        let s = it.sym_arg(a, 0)?;
        let Some(c) = it.member_holder(s) else { return it.make_list(Vec::new()) };
        it.typer.complete_class(c);
        let mut kids: Vec<SymRef> = Vec::new();
        for &k in &it.syms().class(c).children {
            let info = it.syms().class(k);
            kids.push(match (info.kind, info.singleton) {
                (ClassKind::EnumCase, Some(v)) => SymRef::Term(v),
                (ClassKind::Object, _) => it.module_sym(k),
                _ => SymRef::Class(k),
            });
        }
        it.sym_values(kids)
    });
    q!(it, "Reflect.SymbolMethods.Symbol.typeRef", |it, a| {
        let s = it.sym_arg(a, 0)?;
        // A val's `typeRef` is scalac's `TypeRef` to the term, whose `underlying` is the val's
        // declared type (izumi-reflect's name of an enum case's singleton) and whose
        // `typeSymbol` the term (chimney's children of a sealed enum).
        if let SymRef::Term(sym) = s {
            if !matches!(it.syms().sym(sym).kind, SymKind::Object(_)) {
                let t = match it.val_type_refs.get(&sym) {
                    Some(&t) => t,
                    None => {
                        let t = it.typer.types.term_unshared(sym);
                        it.val_type_refs.insert(sym, t);
                        t
                    }
                };
                return Ok(Value::Type(t));
            }
        }
        Ok(Value::Type(it.type_ref_of(s)?))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.termRef", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Type(match s {
            SymRef::Term(sym) => match it.syms().sym(sym).kind {
                SymKind::Object(c) => it.typer.types.class(c, &[]),
                _ => it.typer.types.mk(Type::Term(sym)),
            },
            SymRef::Class(c) if it.syms().class(c).kind == ClassKind::Object => it.typer.types.class(c, &[]),
            SymRef::Class(c) if it.syms().class(c).mods & crate::ast::mods::CASE != 0 => it.typer.types.class(c, &[]),
            SymRef::Pkg(_) | SymRef::PkgClass(_) => it.pkg_type(PkgForm::Sym, s),
            // scalac makes a `TermRef` to any symbol; the one of a class serves as its prefix
            // source, which the class type gives as well.
            SymRef::Class(c) => it.typer.types.class(c, &[]),
            SymRef::Any => ANY,
            SymRef::Nothing => NOTHING,
            // scalac's `NoSymbol.termRef` is a reference to nothing, as its `typeRef` is.
            SymRef::None => it.typer.types.blocked("<nosymbol>"),
            other => {
                let shown = it.symbol_name(other);
                return it.unsupported(format!("{} has no term reference", shown));
            }
        }))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.info", |it, a| {
        let s = it.sym_arg(a, 0)?;
        Ok(Value::Type(match s {
            SymRef::Term(sym) => it.typer.sig_of(sym).ret,
            other => it.type_ref_of(other)?,
        }))
    });
    q!(it, "Reflect.SymbolMethods.Symbol.asQuotes", |it, _a| {
        let c = it.quoted_class(&["scala", "quoted", "QuotesImpl"])?;
        it.module(c)
    });
}

// ---- positions, reports, printers, definitions, implicits ----

fn install_context(it: &mut Table) {
    q!(it, "Reflect.Position.ofMacroExpansion", |it, _a| {
        let ctx = it.macro_ctx()?;
        Ok(Value::Pos(ctx.expansion_file, ctx.expansion_span.start, ctx.expansion_span.end))
    });
    q!(it, "Reflect.Position.apply", |it, a| {
        let Value::Src(f) = arg(a, 1) else { return it.unsupported("Position.apply takes a SourceFile") };
        let start = it.int_arg(a, 2)?;
        let end = it.int_arg(a, 3)?;
        Ok(Value::Pos(f, it.byte_offset(f, start), it.byte_offset(f, end)))
    });
    q!(it, "Reflect.PositionMethods.Position.start", |it, a| match arg(a, 0) {
        Value::Pos(f, s, _) => Ok(Value::Int(it.unit_offset(f, s))),
        _ => it.unsupported("a Position was expected"),
    });
    q!(it, "Reflect.PositionMethods.Position.end", |it, a| match arg(a, 0) {
        Value::Pos(f, _, e) => Ok(Value::Int(it.unit_offset(f, e))),
        _ => it.unsupported("a Position was expected"),
    });
    q!(it, "Reflect.PositionMethods.Position.sourceFile", |it, a| match arg(a, 0) {
        Value::Pos(f, _, _) => Ok(Value::Src(f)),
        _ => it.unsupported("a Position was expected"),
    });
    q!(it, "Reflect.PositionMethods.Position.startLine", |it, a| match arg(a, 0) {
        Value::Pos(f, s, _) => Ok(Value::Int(it.line_col(f, s).0)),
        _ => it.unsupported("a Position was expected"),
    });
    q!(it, "Reflect.PositionMethods.Position.endLine", |it, a| match arg(a, 0) {
        Value::Pos(f, _, e) => Ok(Value::Int(it.line_col(f, e).0)),
        _ => it.unsupported("a Position was expected"),
    });
    q!(it, "Reflect.PositionMethods.Position.startColumn", |it, a| match arg(a, 0) {
        Value::Pos(f, s, _) => Ok(Value::Int(it.line_col(f, s).1)),
        _ => it.unsupported("a Position was expected"),
    });
    q!(it, "Reflect.PositionMethods.Position.endColumn", |it, a| match arg(a, 0) {
        Value::Pos(f, _, e) => Ok(Value::Int(it.line_col(f, e).1)),
        _ => it.unsupported("a Position was expected"),
    });
    q!(it, "Reflect.PositionMethods.Position.sourceCode", |it, a| match arg(a, 0) {
        Value::Pos(f, s, e) => {
            let text = it.source_between(f, s, e);
            it.make_option(text)
        }
        _ => it.unsupported("a Position was expected"),
    });
    q!(it, "Reflect.SourceFile.current", |it, _a| {
        let ctx = it.macro_ctx()?;
        Ok(Value::Src(ctx.expansion_file))
    });
    q!(it, "Reflect.SourceFileMethods.SourceFile.getJPath", |it, a| match arg(a, 0) {
        Value::Src(f) => {
            let path: Rc<str> = Rc::from(it.typer.source(f).path.as_str());
            let c = it.quoted_class(&["java", "nio", "file", "Path"])?;
            let value = it.construct_new(c, vec![Value::Str(path)], &Frame::new(None))?;
            it.make_some(value)
        }
        _ => it.unsupported("a SourceFile was expected"),
    });
    // The older spelling of `getJPath`, which scalactic's `Position` macro still calls.
    q!(it, "Reflect.SourceFileMethods.SourceFile.jpath", |it, a| match arg(a, 0) {
        Value::Src(f) => {
            let path: Rc<str> = Rc::from(it.typer.source(f).path.as_str());
            let c = it.quoted_class(&["java", "nio", "file", "Path"])?;
            it.construct_new(c, vec![Value::Str(path)], &Frame::new(None))
        }
        _ => it.unsupported("a SourceFile was expected"),
    });
    q!(it, "Reflect.SourceFileMethods.SourceFile.name", |it, a| match arg(a, 0) {
        Value::Src(f) => {
            let path = &it.typer.source(f).path;
            let name = path.rsplit(['/', '\\']).next().unwrap_or(path).to_string();
            Ok(Value::string(name))
        }
        _ => it.unsupported("a SourceFile was expected"),
    });
    // The path as the compiler was given it, relative where it was (dotty's
    // `SourceFileMethods.path`, the source's `path`), as `jpath` and `getJPath` are.
    q!(it, "Reflect.SourceFileMethods.SourceFile.path", |it, a| match arg(a, 0) {
        Value::Src(f) => {
            let path = it.typer.source(f).path.clone();
            Ok(Value::string(path))
        }
        _ => it.unsupported("a SourceFile was expected"),
    });
    q!(it, "Reflect.SourceFileMethods.SourceFile.content", |it, a| match arg(a, 0) {
        Value::Src(f) => {
            let text = source_string(&it.typer.source(f).text);
            it.make_some(text)
        }
        _ => it.unsupported("a SourceFile was expected"),
    });

    q!(it, "Reflect.report.error", |it, a| {
        it.report(a, false)?;
        Ok(Value::Unit)
    });
    q!(it, "Reflect.report.errorAndAbort", |it, a| {
        it.report(a, false)?;
        it.stop_expansion()
    });
    q!(it, "Reflect.report.warning", |it, a| {
        it.report(a, true)?;
        Ok(Value::Unit)
    });
    q!(it, "Reflect.report.info", |it, a| {
        let msg = it.str_arg(a, 1)?;
        // Held with the expansion's place until the build is accepted, and printed then in the
        // order of the places, whichever worker ran it.
        let (path, ordinal) = match &it.macro_ctx {
            Some(cx) => {
                cx.infos.set(cx.infos.get() + 1);
                (cx.inline_path.clone(), cx.infos.get())
            }
            None => (Vec::new(), 0),
        };
        // A message of a run under an attempt goes with the attempt.
        it.typer.note_info();
        match &mut it.typer.infos {
            Some(infos) => infos.push(crate::typer::InfoMessage { path, ordinal, text: msg.to_string() }),
            None => eprintln!("info: {}", msg),
        }
        Ok(Value::Unit)
    });

    q!(it, "Reflect.Printer.TreeCode.show", |it, a| {
        let t = it.tree_arg(a, 1)?;
        Ok(Value::string(it.show_tree(t)))
    });
    q!(it, "Reflect.Printer.TreeShortCode.show", |it, a| {
        let t = it.tree_arg(a, 1)?;
        Ok(Value::string(it.show_tree(t)))
    });
    q!(it, "Reflect.Printer.TreeStructure.show", |it, a| {
        let t = it.tree_arg(a, 1)?;
        let mut out = String::new();
        it.show_structure(t, &mut out)?;
        Ok(Value::string(out))
    });
    q!(it, "Reflect.Printer.TypeReprCode.show", |it, a| {
        let t = it.type_arg(a, 1)?;
        Ok(Value::string(it.show_type_repr(t)))
    });
    q!(it, "Reflect.Printer.TypeReprShortCode.show", |it, a| {
        let t = it.type_arg(a, 1)?;
        let t = it.typer.zonk(t);
        Ok(Value::string(it.typer.show(t)))
    });
    q!(it, "Reflect.Printer.TypeReprStructure.show", |it, a| {
        let t = it.type_arg(a, 1)?;
        Ok(Value::string(it.show_type_structure(t)))
    });
    q!(it, "Reflect.Printer.ConstantCode.show", |it, a| {
        let c = it.constant_value(&arg(a, 1))?;
        Ok(Value::string(it.show_constant(&c)?))
    });

    q!(it, "Reflect.defn.RootPackage", |_it, _a| Ok(Value::Sym(SymRef::Root)));
    q!(it, "Reflect.defn.RootClass", |_it, _a| Ok(Value::Sym(SymRef::Root)));
    q!(it, "Reflect.defn.EmptyPackageClass", |_it, _a| Ok(Value::Sym(SymRef::PkgClass(ROOT_PKG))));
    q!(it, "Reflect.defn.ScalaPackage", |it, _a| Ok(Value::Sym(SymRef::Pkg(it.typer.b.scala_pkg))));
    q!(it, "Reflect.defn.ScalaPackageClass", |it, _a| Ok(Value::Sym(SymRef::PkgClass(it.typer.b.scala_pkg))));
    q!(it, "Reflect.defn.RepeatedParamClass", |it, _a| Ok(Value::Sym(it.known_class("Seq").map_or(SymRef::None, SymRef::Class))));
    q!(it, "Reflect.defn.RepeatedAnnot", |_it, _a| Ok(Value::Sym(SymRef::None)));
    q!(it, "Reflect.defn.FunctionClass", |it, a| {
        let n = it.int_arg(a, 1)? as usize;
        let c = it.typer.b.functions.get(n).copied().flatten();
        Ok(Value::Sym(c.map_or(SymRef::None, SymRef::Class)))
    });
    q!(it, "Reflect.defn.PolyFunctionClass", |_it, _a| Ok(Value::Sym(SymRef::None)));
    q!(it, "Reflect.defn.TupleClass", |it, a| {
        let n = it.int_arg(a, 1)? as usize;
        Ok(Value::Sym(SymRef::Class(it.typer.tuple_class(n))))
    });
    q!(it, "Reflect.defn.isTupleClass", |it, a| {
        let s = it.sym_arg(a, 1)?;
        Ok(Value::Bool(matches!(s, SymRef::Class(c) if it.typer.is_tuple_class(c))))
    });
    q!(it, "Reflect.defn.EmptyTupleClass", |it, _a| Ok(Value::Sym(it.known_class("EmptyTuple").map_or(SymRef::None, SymRef::Class))));
    q!(it, "Reflect.defn.NonEmptyTupleClass", |it, _a| Ok(Value::Sym(it.known_class("NonEmptyTuple").map_or(SymRef::None, SymRef::Class))));

    q!(it, "Reflect.Implicits.search", |it, a| {
        let t = it.type_arg(a, 1)?;
        let span = it.macro_ctx.as_ref().map_or(Span::default(), |c| c.site_span);
        let mark = it.typer.diags.items.len();
        let found = it.typer.summon_at_site(t, span);
        let msgs: Vec<String> = it.typer.diags.items.drain(mark..).map(|d| d.msg).collect();
        match found {
            Some((te, ty)) => {
                let te = if it.prog().type_of(te).is_none() { it.typer.prog.typed(te, ty) } else { te };
                let c = it.quoted_class(&["scala", "quoted", "Reflect", "ImplicitSearchSuccess"])?;
                it.construct_new(c, vec![Value::Tree(TreeRef::Expr(te))], &Frame::new(None))
            }
            None => {
                let shown = it.show_type_repr(t);
                let explanation = if msgs.is_empty() { format!("No given instance of type {} was found", shown) } else { msgs.join("\n") };
                let c = it.quoted_class(&["scala", "quoted", "Reflect", "NoMatchingImplicits"])?;
                it.construct_new(c, vec![Value::string(explanation)], &Frame::new(None))
            }
        }
    });
    q!(it, "Reflect.ClassOfConstant.apply", |it, a| {
        let t = it.type_arg(a, 1)?;
        it.make_constant(Value::Type(t))
    });
}

/// A character inside a literal as scalac prints it: the escapes of the language, `\uXXXX`
/// for the other control characters.
fn escape_char(c: char, quote: char, out: &mut String) {
    match c {
        '\\' => out.push_str("\\\\"),
        '\n' => out.push_str("\\n"),
        '\t' => out.push_str("\\t"),
        '\r' => out.push_str("\\r"),
        '\u{8}' => out.push_str("\\b"),
        '\u{c}' => out.push_str("\\f"),
        c if c == quote => {
            out.push('\\');
            out.push(c);
        }
        c if (c as u32) < 0x20 || (0x7f..0xa0).contains(&(c as u32)) => out.push_str(&format!("\\u{:04x}", c as u32)),
        c => out.push(c),
    }
}

// ---- printers ----

impl<'a, 't> Interp<'a, 't> {
    fn show_constant(&mut self, v: &Value) -> R<String> {
        Ok(match v {
            Value::Str(s) => {
                let mut out = String::from("\"");
                for c in s.chars() {
                    escape_char(c, '"', &mut out);
                }
                out.push('"');
                out
            }
            Value::Char(c) => {
                let mut out = String::from("'");
                escape_char(char::from_u32(*c as u32).unwrap_or('\u{fffd}'), '\'', &mut out);
                out.push('\'');
                out
            }
            Value::Long(l) => format!("{}L", l),
            Value::Float(f) => format!("{}f", java_float(*f)),
            Value::Double(d) => java_double(*d),
            Value::Unit => "()".to_string(),
            Value::Null => "null".to_string(),
            Value::Type(t) => format!("classOf[{}]", self.show_type_repr(*t)),
            other => self.to_str(other)?,
        })
    }

    /// The tree as Scala source, as `Printer.TreeCode` shows it.
    pub(super) fn show_tree(&mut self, t: TreeRef) -> String {
        let mut out = String::new();
        if let Err(_) = self.show_tree_into(t, &mut out, 0) {
            out.push_str("<tree>");
        }
        out
    }

    fn show_tree_into(&mut self, t: TreeRef, out: &mut String, indent: usize) -> R<()> {
        match self.view(t)? {
            View::Literal(v) => out.push_str(&self.show_constant(&v)?),
            View::Ident(s) => out.push_str(&self.symbol_name(s).trim_end_matches('$').to_string()),
            View::Select(q, _, n) => {
                self.show_tree_into(q, out, indent)?;
                out.push('.');
                out.push_str(self.typer.name_ref(n));
            }
            View::Apply(f, args) => {
                self.show_tree_into(f, out, indent)?;
                out.push('(');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.show_tree_into(*a, out, indent)?;
                }
                out.push(')');
            }
            View::TypeApply(f, targs) => {
                self.show_tree_into(f, out, indent)?;
                out.push('[');
                for (i, a) in targs.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&self.show_type_repr(*a));
                }
                out.push(']');
            }
            View::New(ty) => {
                out.push_str("new ");
                out.push_str(&self.show_type_repr(ty));
            }
            View::This => out.push_str("this"),
            View::Super => out.push_str("super"),
            View::Typed(e, ty) => {
                out.push('(');
                self.show_tree_into(e, out, indent)?;
                out.push_str(": ");
                out.push_str(&self.show_type_repr(ty));
                out.push(')');
            }
            View::Repeated(items, _) => {
                for (i, a) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.show_tree_into(*a, out, indent)?;
                }
            }
            View::Assign(l, r) => {
                self.show_tree_into(l, out, indent)?;
                out.push_str(" = ");
                self.show_tree_into(r, out, indent)?;
            }
            View::Block(stats, res) => {
                out.push_str("{\n");
                let pad = "  ".repeat(indent + 1);
                for s in stats {
                    out.push_str(&pad);
                    self.show_tree_into(s, out, indent + 1)?;
                    out.push('\n');
                }
                out.push_str(&pad);
                self.show_tree_into(res, out, indent + 1)?;
                out.push('\n');
                out.push_str(&"  ".repeat(indent));
                out.push('}');
            }
            View::Lambda(params, body) => {
                out.push_str("((");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&self.symbol_name(SymRef::Term(*p)));
                    out.push_str(": ");
                    let ty = self.typer.sig_of(*p).ret;
                    out.push_str(&self.show_type_repr(ty));
                }
                out.push_str(") => ");
                self.show_tree_into(body, out, indent)?;
                out.push(')');
            }
            View::Closure(m) => self.show_tree_into(m, out, indent)?,
            View::If(c, a, b) => {
                out.push_str("if (");
                self.show_tree_into(c, out, indent)?;
                out.push_str(") ");
                self.show_tree_into(a, out, indent)?;
                out.push_str(" else ");
                self.show_tree_into(b, out, indent)?;
            }
            View::Match(s, cases) => {
                self.show_tree_into(s, out, indent)?;
                out.push_str(" match {\n");
                for c in cases {
                    out.push_str(&"  ".repeat(indent + 1));
                    self.show_tree_into(c, out, indent + 1)?;
                    out.push('\n');
                }
                out.push_str(&"  ".repeat(indent));
                out.push('}');
            }
            View::CaseDef(p, g, b) => {
                out.push_str("case ");
                self.show_tree_into(p, out, indent)?;
                if let Some(g) = g {
                    out.push_str(" if ");
                    self.show_tree_into(g, out, indent)?;
                }
                out.push_str(" =>\n");
                out.push_str(&"  ".repeat(indent + 1));
                self.show_tree_into(b, out, indent + 1)?;
            }
            View::Try(b, cases, f) => {
                out.push_str("try ");
                self.show_tree_into(b, out, indent)?;
                if !cases.is_empty() {
                    out.push_str(" catch {\n");
                    for c in cases {
                        out.push_str(&"  ".repeat(indent + 1));
                        self.show_tree_into(c, out, indent + 1)?;
                        out.push('\n');
                    }
                    out.push_str(&"  ".repeat(indent));
                    out.push('}');
                }
                if let Some(f) = f {
                    out.push_str(" finally ");
                    self.show_tree_into(f, out, indent)?;
                }
            }
            View::Return(e) => {
                out.push_str("return ");
                self.show_tree_into(e, out, indent)?;
            }
            View::While(c, b) => {
                out.push_str("while (");
                self.show_tree_into(c, out, indent)?;
                out.push_str(") ");
                self.show_tree_into(b, out, indent)?;
            }
            View::ValDef(s, rhs) => {
                let info = self.syms().sym(s);
                out.push_str(if info.kind == SymKind::Var { "var " } else { "val " });
                out.push_str(&self.symbol_name(SymRef::Term(s)));
                out.push_str(": ");
                let ty = self.typer.sig_of(s).ret;
                out.push_str(&self.show_type_repr(ty));
                if let Some(r) = rhs {
                    out.push_str(" = ");
                    self.show_tree_into(r, out, indent)?;
                }
            }
            View::DefDef(..) => {
                let (name, clauses, ret, body) = self.def_parts(t)?;
                out.push_str("def ");
                out.push_str(&name);
                self.show_clauses(&clauses, out);
                out.push_str(": ");
                out.push_str(&self.show_type_repr(ret));
                if let Some(b) = body {
                    out.push_str(" = ");
                    self.show_tree_into(b, out, indent)?;
                }
            }
            View::ClassDef(c) => {
                let info = self.syms().class(c);
                out.push_str(match info.kind {
                    ClassKind::Trait => "trait ",
                    ClassKind::Object => "object ",
                    _ => "class ",
                });
                out.push_str(self.typer.name_ref(info.name));
            }
            View::TypeDef(s) => {
                out.push_str("type ");
                out.push_str(&self.symbol_name(s));
            }
            View::Bind(s, p) => {
                out.push_str(&self.symbol_name(SymRef::Term(s)));
                if !matches!(self.view(p)?, View::Wildcard) {
                    out.push_str(" @ ");
                    self.show_tree_into(p, out, indent)?;
                }
            }
            View::Unapply(subs) => {
                let fun = self.pattern_fun(t)?;
                match self.view(fun)? {
                    View::Select(q, _, _) => self.show_tree_into(q, out, indent)?,
                    _ => out.push_str("_"),
                }
                out.push('(');
                for (i, a) in subs.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.show_tree_into(*a, out, indent)?;
                }
                out.push(')');
            }
            View::Alternatives(subs) => {
                for (i, a) in subs.iter().enumerate() {
                    if i > 0 {
                        out.push_str(" | ");
                    }
                    self.show_tree_into(*a, out, indent)?;
                }
            }
            View::Wildcard => out.push('_'),
            View::TypeTree(ty) => out.push_str(&self.show_type_repr(ty)),
            View::TypedPattern(p, ty) => {
                self.show_tree_into(p, out, indent)?;
                out.push_str(": ");
                out.push_str(&self.show_type_repr(ty));
            }
            View::NamedArg(n, arg) => {
                out.push_str(self.typer.name_ref(n));
                out.push_str(" = ");
                self.show_tree_into(arg, out, indent)?;
            }
        }
        Ok(())
    }

    fn show_clauses(&mut self, clauses: &[Vec<SymRef>], out: &mut String) {
        for clause in clauses {
            let types = clause.iter().all(|s| matches!(s, SymRef::TParam(_)));
            out.push(if types { '[' } else { '(' });
            for (i, s) in clause.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&self.symbol_name(*s));
                if let SymRef::Term(p) = s {
                    out.push_str(": ");
                    let ty = self.typer.sig_of(*p).ret;
                    out.push_str(&self.show_type_repr(ty));
                }
            }
            out.push(if types { ']' } else { ')' });
        }
    }

    /// The tree as its constructors, as `Printer.TreeStructure` shows it.
    fn show_structure(&mut self, t: TreeRef, out: &mut String) -> R<()> {
        let list = |it: &mut Self, items: &[TreeRef], out: &mut String| -> R<()> {
            out.push_str("List(");
            for (i, a) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                it.show_structure(*a, out)?;
            }
            out.push(')');
            Ok(())
        };
        match self.view(t)? {
            View::Literal(v) => {
                let kind = match v {
                    Value::Int(_) => "IntConstant",
                    Value::Long(_) => "LongConstant",
                    Value::Double(_) => "DoubleConstant",
                    Value::Float(_) => "FloatConstant",
                    Value::Bool(_) => "BooleanConstant",
                    Value::Char(_) => "CharConstant",
                    Value::Str(_) => "StringConstant",
                    Value::Byte(_) => "ByteConstant",
                    Value::Short(_) => "ShortConstant",
                    Value::Unit => "UnitConstant",
                    Value::Null => "NullConstant",
                    _ => "ClassOfConstant",
                };
                let shown = self.show_constant(&v)?;
                if matches!(v, Value::Unit | Value::Null) {
                    out.push_str(&format!("Literal({}())", kind));
                } else {
                    out.push_str(&format!("Literal({}({}))", kind, shown));
                }
            }
            View::Ident(s) => out.push_str(&format!("Ident(\"{}\")", self.symbol_name(s))),
            View::Select(q, _, n) => {
                out.push_str("Select(");
                self.show_structure(q, out)?;
                out.push_str(&format!(", \"{}\")", self.typer.name_ref(n)));
            }
            View::Apply(f, args) => {
                out.push_str("Apply(");
                self.show_structure(f, out)?;
                out.push_str(", ");
                list(self, &args, out)?;
                out.push(')');
            }
            View::TypeApply(f, targs) => {
                out.push_str("TypeApply(");
                self.show_structure(f, out)?;
                out.push_str(", List(");
                for (i, a) in targs.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!("Inferred({})", self.show_type_structure(*a)));
                }
                out.push_str("))");
            }
            View::New(ty) => out.push_str(&format!("New(Inferred({}))", self.show_type_structure(ty))),
            View::This => out.push_str("This(None)"),
            View::Super => out.push_str("Super(This(None), None)"),
            View::Typed(e, ty) => {
                out.push_str("Typed(");
                self.show_structure(e, out)?;
                out.push_str(&format!(", Inferred({}))", self.show_type_structure(ty)));
            }
            View::Repeated(items, ty) => {
                out.push_str("Repeated(");
                list(self, &items, out)?;
                out.push_str(&format!(", Inferred({}))", self.show_type_structure(ty)));
            }
            View::Assign(l, r) => {
                out.push_str("Assign(");
                self.show_structure(l, out)?;
                out.push_str(", ");
                self.show_structure(r, out)?;
                out.push(')');
            }
            View::Block(stats, res) => {
                out.push_str("Block(");
                list(self, &stats, out)?;
                out.push_str(", ");
                self.show_structure(res, out)?;
                out.push(')');
            }
            View::Lambda(params, body) => {
                out.push_str("Block(List(DefDef(\"$anonfun\", List(TermParamClause(List(");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    let ty = self.typer.sig_of(*p).ret;
                    out.push_str(&format!("ValDef(\"{}\", Inferred({}), None)", self.symbol_name(SymRef::Term(*p)), self.show_type_structure(ty)));
                }
                out.push_str("))), Inferred(), Some(");
                self.show_structure(body, out)?;
                out.push_str("))), Closure(Ident(\"$anonfun\"), None))");
            }
            View::Closure(m) => {
                out.push_str("Closure(");
                self.show_structure(m, out)?;
                out.push_str(", None)");
            }
            View::If(c, a, b) => {
                out.push_str("If(");
                self.show_structure(c, out)?;
                out.push_str(", ");
                self.show_structure(a, out)?;
                out.push_str(", ");
                self.show_structure(b, out)?;
                out.push(')');
            }
            View::Match(s, cases) => {
                out.push_str("Match(");
                self.show_structure(s, out)?;
                out.push_str(", ");
                list(self, &cases, out)?;
                out.push(')');
            }
            View::CaseDef(p, g, b) => {
                out.push_str("CaseDef(");
                self.show_structure(p, out)?;
                out.push_str(", ");
                match g {
                    Some(g) => {
                        out.push_str("Some(");
                        self.show_structure(g, out)?;
                        out.push(')');
                    }
                    None => out.push_str("None"),
                }
                out.push_str(", ");
                self.show_structure(b, out)?;
                out.push(')');
            }
            View::Try(b, cases, f) => {
                out.push_str("Try(");
                self.show_structure(b, out)?;
                out.push_str(", ");
                list(self, &cases, out)?;
                out.push_str(", ");
                match f {
                    Some(f) => {
                        out.push_str("Some(");
                        self.show_structure(f, out)?;
                        out.push(')');
                    }
                    None => out.push_str("None"),
                }
                out.push(')');
            }
            View::Return(e) => {
                out.push_str("Return(");
                self.show_structure(e, out)?;
                out.push_str(", <owner>)");
            }
            View::While(c, b) => {
                out.push_str("While(");
                self.show_structure(c, out)?;
                out.push_str(", ");
                self.show_structure(b, out)?;
                out.push(')');
            }
            View::ValDef(s, rhs) => {
                let ty = self.typer.sig_of(s).ret;
                out.push_str(&format!("ValDef(\"{}\", Inferred({}), ", self.symbol_name(SymRef::Term(s)), self.show_type_structure(ty)));
                match rhs {
                    Some(r) => {
                        out.push_str("Some(");
                        self.show_structure(r, out)?;
                        out.push(')');
                    }
                    None => out.push_str("None"),
                }
                out.push(')');
            }
            View::DefDef(..) => {
                let (name, clauses, ret, body) = self.def_parts(t)?;
                out.push_str(&format!("DefDef(\"{}\", List(", name));
                for (i, clause) in clauses.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    let types = clause.iter().all(|s| matches!(s, SymRef::TParam(_)));
                    out.push_str(if types { "TypeParamClause(List(" } else { "TermParamClause(List(" });
                    for (j, p) in clause.iter().enumerate() {
                        if j > 0 {
                            out.push_str(", ");
                        }
                        match p {
                            SymRef::Term(sym) => {
                                let ty = self.typer.sig_of(*sym).ret;
                                out.push_str(&format!("ValDef(\"{}\", Inferred({}), None)", self.symbol_name(*p), self.show_type_structure(ty)));
                            }
                            other => out.push_str(&format!("TypeDef(\"{}\", TypeBoundsTree(Inferred(TypeRef(ThisType(TypeRef(NoPrefix(), \"scala\")), \"Nothing\")), Inferred(TypeRef(ThisType(TypeRef(NoPrefix(), \"scala\")), \"Any\"))))", self.symbol_name(*other))),
                        }
                    }
                    out.push_str("))");
                }
                out.push_str(&format!("), Inferred({}), ", self.show_type_structure(ret)));
                match body {
                    Some(b) => {
                        out.push_str("Some(");
                        self.show_structure(b, out)?;
                        out.push(')');
                    }
                    None => out.push_str("None"),
                }
                out.push(')');
            }
            View::ClassDef(c) => out.push_str(&format!("ClassDef(\"{}\", ...)", self.typer.name_ref(self.syms().class(c).name))),
            View::TypeDef(s) => out.push_str(&format!("TypeDef(\"{}\", ...)", self.symbol_name(s))),
            View::Bind(s, p) => {
                out.push_str(&format!("Bind(\"{}\", ", self.symbol_name(SymRef::Term(s))));
                self.show_structure(p, out)?;
                out.push(')');
            }
            View::Unapply(subs) => {
                let fun = self.pattern_fun(t)?;
                out.push_str("Unapply(");
                self.show_structure(fun, out)?;
                out.push_str(", Nil, ");
                list(self, &subs, out)?;
                out.push(')');
            }
            View::Alternatives(subs) => {
                out.push_str("Alternatives(");
                list(self, &subs, out)?;
                out.push(')');
            }
            View::Wildcard => out.push_str("Wildcard()"),
            View::TypeTree(ty) => out.push_str(&format!("Inferred({})", self.show_type_structure(ty))),
            View::TypedPattern(p, ty) => {
                out.push_str("TypedOrTest(");
                self.show_structure(p, out)?;
                out.push_str(&format!(", Inferred({}))", self.show_type_structure(ty)));
            }
            View::NamedArg(n, arg) => {
                out.push_str(&format!("NamedArg(\"{}\", ", self.typer.name_ref(n)));
                self.show_structure(arg, out)?;
                out.push(')');
            }
        }
        Ok(())
    }

    /// A type as `Printer.TypeReprCode` shows it: qualified as scalac qualifies it.
    pub(super) fn show_type_repr(&mut self, t: TypeId) -> String {
        let t = self.typer.zonk(t);
        let mut out = String::new();
        self.show_type_into(t, &mut out);
        out
    }

    /// The package scala-library declares a class of the std's `scala` package in.
    fn scala_library_package(name: &str) -> &'static str {
        match name {
            "List" | "Nil" | "::" | "Vector" | "Map" | "Set" | "Seq" | "IndexedSeq" | "Iterable" | "LazyList" | "Range" | "NumericRange" | "ArraySeq" | "ListMap" | "SortedMap" | "SortedSet" | "TreeMap" | "TreeSet" => {
                "scala.collection.immutable"
            }
            "Iterator" | "IterableOnce" | "Factory" | "BuildFrom" | "IterableOps" | "SeqOps" | "SetOps" | "IndexedSeqOps" | "View" | "MapView" | "IterableFactory" | "IterableFactoryDefaults" | "SeqFactory" | "MapFactory" | "StrictOptimizedIterableOps" | "StrictOptimizedSeqOps" | "StrictOptimizedSeqFactory" => "scala.collection",
            "ArrayBuffer" | "ListBuffer" | "Builder" | "StringBuilder" => "scala.collection.mutable",
            "Either" | "Left" | "Right" => "scala.util",
            "Ordering" | "Ordered" | "Numeric" | "Integral" | "Fractional" => "scala.math",
            _ => "scala",
        }
    }

    fn is_std_scala_class(&self, c: ClassId) -> bool {
        let info = self.syms().class(c);
        info.kind == ClassKind::Builtin || (matches!(info.owner, Owner::Package(p) if p == self.typer.b.scala_pkg) && self.typer.source(info.file).is_std)
    }

    fn scalac_class_name(&mut self, c: ClassId) -> String {
        let info = self.syms().class(c);
        let name = self.typer.name_ref(info.name).to_string();
        if self.is_std_scala_class(c) {
            if name == "String" {
                return "scala.Predef.String".to_string();
            }
            return format!("{}.{}", Self::scala_library_package(&name), name);
        }
        let path = self.typer.class_path(c);
        match info.owner {
            Owner::Package(p) if p == ROOT_PKG => path,
            Owner::Class(_) | Owner::Package(_) => path,
            Owner::Local => name,
        }
    }

    fn show_type_into(&mut self, t: TypeId, out: &mut String) {
        match self.typer.types.get(t) {
            Type::Any => out.push_str("scala.Any"),
            Type::Nothing => out.push_str("scala.Nothing"),
            Type::Error => out.push_str("<error>"),
            Type::Wild => out.push_str("?"),
            Type::BoundedWild(lo, hi) => {
                out.push('?');
                if lo != NOTHING {
                    out.push_str(" >: ");
                    self.show_type_into(lo, out);
                }
                if hi != ANY {
                    out.push_str(" <: ");
                    self.show_type_into(hi, out);
                }
            }
            Type::Class(c, args) => {
                let items = self.typer.types.items(args).to_vec();
                let name = self.scalac_class_name(c);
                out.push_str(&name);
                if self.syms().class(c).kind == ClassKind::Object {
                    out.push_str(".type");
                }
                if !items.is_empty() {
                    out.push('[');
                    for (i, a) in items.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        self.show_type_into(*a, out);
                    }
                    out.push(']');
                }
            }
            Type::Ctor(c) => {
                let name = self.scalac_class_name(c);
                out.push_str(&name);
            }
            Type::Alias(a, args) => {
                let name = self.typer.name_ref(self.typer.syms.aliases[a.idx()].name).to_string();
                match self.symbol_owner(SymRef::Alias(a)) {
                    SymRef::Class(c) => {
                        out.push_str(&self.scalac_class_name(c));
                        out.push('.');
                    }
                    SymRef::FilePackage(f) => {
                        let full = self.symbol_full_name(SymRef::FilePackage(f));
                        out.push_str(full.trim_end_matches('$'));
                        out.push('.');
                    }
                    _ => {}
                }
                out.push_str(&name);
                let items = self.typer.types.items(args).to_vec();
                if !items.is_empty() {
                    out.push('[');
                    for (i, a) in items.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        self.show_type_into(*a, out);
                    }
                    out.push(']');
                }
            }
            Type::Param(p) | Type::AppParam(p, _) => {
                out.push_str(self.typer.name_ref(self.syms().tparam(p).name));
                if let Type::AppParam(_, args) = self.typer.types.get(t) {
                    let items = self.typer.types.items(args).to_vec();
                    out.push('[');
                    for (i, a) in items.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        self.show_type_into(*a, out);
                    }
                    out.push(']');
                }
            }
            Type::Lit(l) => {
                let v = self.literal(self.typer.types.lit_val(l));
                out.push_str(&self.show_constant(&v).unwrap_or_default());
            }
            Type::Union(a, b) => {
                self.show_type_into(a, out);
                out.push_str(" | ");
                self.show_type_into(b, out);
            }
            Type::Inter(a, b) => {
                self.show_type_into(a, out);
                out.push_str(" & ");
                self.show_type_into(b, out);
            }
            Type::Term(s) => {
                // A local's reference has no prefix and is shown by its name alone.
                if self.syms().sym(s).owner == Owner::Local {
                    out.push_str(self.typer.name_ref(self.syms().sym(s).name));
                    return;
                }
                let full = self.symbol_full_name(SymRef::Term(s));
                out.push_str(&full);
                out.push_str(".type");
            }
            Type::Select(p, s) => {
                self.show_type_into(p, out);
                let name = self.typer.name_ref(self.syms().sym(s).name).to_string();
                out.push('.');
                out.push_str(&name);
                out.push_str(".type");
            }
            Type::This(c) => {
                let name = self.scalac_class_name(c);
                out.push_str(&name);
                out.push_str(".this.type");
            }
            Type::Blocked(b) => match self.pkg_form(t) {
                Some((form, pkg)) => {
                    let path = if pkg == SymRef::Root { "_root_".to_string() } else { self.symbol_full_name(pkg) };
                    out.push_str(&path);
                    if form == PkgForm::ThisType {
                        out.push_str(".type");
                    }
                }
                None => match self.by_name_underlying(t).unwrap_or(None) {
                    Some(u) => {
                        out.push_str("=> ");
                        self.show_type_into(u, out);
                    }
                    None => out.push_str(self.typer.types.blocked_description(b)),
                },
            },
            _ => {
                let shown = self.typer.show(t);
                out.push_str(&shown);
            }
        }
    }

    fn show_type_structure(&mut self, t: TypeId) -> String {
        let t = self.typer.zonk(t);
        match self.typer.types.get(t) {
            Type::Class(c, args) => {
                let items = self.typer.types.items(args).to_vec();
                let (owner, name) = {
                    let info = self.syms().class(c);
                    (info.owner, self.typer.name_ref(info.name).to_string())
                };
                let prefix = match owner {
                    Owner::Package(p) => {
                        let path = self.typer.class_path_prefix(Owner::Package(p));
                        let pkg = path.trim_end_matches('.');
                        format!("ThisType(TypeRef(NoPrefix(), \"{}\"))", pkg.rsplit('.').next().unwrap_or(pkg))
                    }
                    Owner::Class(o) => {
                        let ot = self.typer.types.class(o, &[]);
                        self.show_type_structure(ot)
                    }
                    Owner::Local => "NoPrefix()".to_string(),
                };
                let is_object = self.syms().class(c).kind == ClassKind::Object;
                let base = if is_object { format!("TermRef({}, \"{}\")", prefix, name) } else { format!("TypeRef({}, \"{}\")", prefix, name) };
                if items.is_empty() {
                    base
                } else {
                    let args: Vec<String> = items.iter().map(|a| self.show_type_structure(*a)).collect();
                    format!("AppliedType({}, List({}))", base, args.join(", "))
                }
            }
            Type::Lit(l) => {
                let v = self.literal(self.typer.types.lit_val(l));
                let shown = self.show_constant(&v).unwrap_or_default();
                let kind = match v {
                    Value::Int(_) => "IntConstant",
                    Value::Long(_) => "LongConstant",
                    Value::Double(_) => "DoubleConstant",
                    Value::Bool(_) => "BooleanConstant",
                    Value::Char(_) => "CharConstant",
                    _ => "StringConstant",
                };
                format!("ConstantType({}({}))", kind, shown)
            }
            Type::Union(a, b) => format!("OrType({}, {})", self.show_type_structure(a), self.show_type_structure(b)),
            Type::Inter(a, b) => format!("AndType({}, {})", self.show_type_structure(a), self.show_type_structure(b)),
            Type::Param(p) => format!("TypeRef(NoPrefix(), \"{}\")", self.typer.name_ref(self.syms().tparam(p).name)),
            Type::Any => "TypeRef(ThisType(TypeRef(NoPrefix(), \"scala\")), \"Any\")".to_string(),
            Type::Nothing => "TypeRef(ThisType(TypeRef(NoPrefix(), \"scala\")), \"Nothing\")".to_string(),
            _ => {
                let shown = self.show_type_repr(t);
                format!("TypeRef(NoPrefix(), \"{}\")", shown)
            }
        }
    }
}

/// `tpt` and `rhs` as scalac's `ValDefMethods` and `DefDefMethods` also declare them, which
/// a jar's macro names directly (`ValDefMethods.tpt(v)`); the `ValOrDefDefMethods` answers.
fn install_definition_parts(it: &mut Table) {
    q!(it, "Reflect.ValDefMethods.ValDef.tpt", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::ValDef(s, _) => Ok(Value::Tree(TreeRef::Type(it.typer.sig_of(s).ret))),
            _ => it.unsupported("tpt of a tree that is no val"),
        }
    });
    q!(it, "Reflect.ValDefMethods.ValDef.rhs", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::ValDef(_, rhs) => it.opt_tree(rhs),
            _ => it.unsupported("rhs of a tree that is no val"),
        }
    });
    q!(it, "Reflect.DefDefMethods.DefDef.rhs", |it, a| {
        let t = it.tree_arg(a, 0)?;
        match it.view(t)? {
            View::DefDef(..) => {
                let rhs = it.def_parts(t)?.3;
                it.opt_tree(rhs)
            }
            _ => it.unsupported("rhs of a tree that is no def"),
        }
    });
}

fn refinement_name(r: Refinement) -> crate::intern::Name {
    match r {
        Refinement::Alias(n, _) | Refinement::Bounds(n, ..) | Refinement::Val(n, ..) | Refinement::Term(n, ..) => n,
    }
}
