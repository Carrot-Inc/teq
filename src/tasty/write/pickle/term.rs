//! The terms of a body: the typed IR with the capture's records written as scalac 3.8.4's tree
//! after `PostTyper`. A form the encoder has no reading for
//! fails the build with the construct named, never a wrong body.

use super::*;
use super::shapes::{sig_of_text, Inverse};
use crate::tir::capture::{Form, NodeRecords, PatForm, Wrap};
use crate::tir::{PrimOp, TExpr, TExprId, TPat, TPatId, TStmt, UnOp};

/// What the receiver of a selection is.
#[derive(Clone, Copy)]
pub(super) enum Qual {
    /// A term of the body.
    Expr(TExprId),
    /// The path of the member's owner: its object, its file's `$package`, or `this` inside it.
    Owner,
}

/// Where scala-library has a lean std definition.
#[derive(Clone, Copy)]
pub(super) enum StdTarget {
    /// A member of an object: `Predef.println`, `scala.math.package.min`.
    Object { pkg: &'static str, object: &'static str },
    /// A member of the receiver's own class: `s.length()` of `java.lang.String`.
    Receiver { pkg: &'static str, class: &'static str, java: bool },
    /// A member of the class the receiver converts to through `Predef`'s conversion:
    /// `augmentString(s).nonEmpty`, `ArrowAssoc[A](a).->[B](b)`.
    Wrapped { conv: &'static str, targs: bool, param: &'static str, result: &'static str, class: (&'static str, &'static str) },
    /// A member of an array: the class's own, `ArrayOps`' through `Predef.xArrayOps` or
    /// `ArraySeq`'s through `Predef.wrapXArray`, whichever scala-library has it in first.
    Array,
    /// `summon[T](x)`: scalac's expansion, the given.
    Summon,
    /// `Predef.$conforms[A]`.
    Conforms,
}

/// How a node of an inline body is reducible: an `inline if` or `inline match`, or the match a
/// `summonFrom` was typed to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Reducible {
    Inline,
    SummonFrom,
}

pub(super) fn sig_params(params: Vec<crate::typer::loader::declared::PickledSigParam>) -> Vec<SigParam> {
    params
        .into_iter()
        .map(|p| match p {
            crate::typer::loader::declared::PickledSigParam::Types(n) => SigParam::Types(n),
            crate::typer::loader::declared::PickledSigParam::Term(t) => SigParam::Type(t),
        })
        .collect()
}

/// The signature of `Predef`'s conversion of a receiver (`augmentString`, `ArrowAssoc[A]`).
pub(super) fn conversion_sig(targs: bool, param: &str) -> Vec<SigParam> {
    if targs { vec![SigParam::Types(1), SigParam::Type(param.to_string())] } else { vec![SigParam::Type(param.to_string())] }
}

/// The kind of an array's elements Predef's conversions of arrays are chosen by.
#[derive(Clone, Copy)]
pub(super) enum ArrayElem {
    Prim(&'static str),
    Ref,
    Generic,
}

pub(super) const ARRAY_ELEMS: [ArrayElem; 11] = [
    ArrayElem::Prim("Int"),
    ArrayElem::Prim("Long"),
    ArrayElem::Prim("Double"),
    ArrayElem::Prim("Float"),
    ArrayElem::Prim("Char"),
    ArrayElem::Prim("Byte"),
    ArrayElem::Prim("Short"),
    ArrayElem::Prim("Boolean"),
    ArrayElem::Prim("Unit"),
    ArrayElem::Ref,
    ArrayElem::Generic,
];

/// A conversion of `Predef` and the class it converts to: its name, whether it takes the
/// element type, its parameter's erasure, its result's, the class's package and name.
pub(super) struct ArrayConv {
    pub name: String,
    pub targs: bool,
    pub param: String,
    pub result: String,
    pub class: (&'static str, String),
}

impl ArrayElem {
    fn array_erasure(self) -> String {
        match self {
            ArrayElem::Prim("Unit") => "scala.runtime.BoxedUnit[]".to_string(),
            ArrayElem::Prim(p) => format!("scala.{}[]", p),
            ArrayElem::Ref => "java.lang.Object[]".to_string(),
            ArrayElem::Generic => "java.lang.Object".to_string(),
        }
    }

    /// `intArrayOps`, `refArrayOps[T]`, `genericArrayOps[T]` to `ArrayOps`.
    pub(super) fn ops(self) -> ArrayConv {
        let name = match self {
            ArrayElem::Prim(p) => format!("{}ArrayOps", p.to_lowercase()),
            ArrayElem::Ref => "refArrayOps".to_string(),
            ArrayElem::Generic => "genericArrayOps".to_string(),
        };
        let targs = !matches!(self, ArrayElem::Prim(_));
        ArrayConv { name, targs, param: self.array_erasure(), result: "java.lang.Object".to_string(), class: ("scala.collection", "ArrayOps".to_string()) }
    }

    /// `wrapIntArray`, `wrapRefArray[T]`, `genericWrapArray[T]` to an `ArraySeq`.
    pub(super) fn wrap(self) -> ArrayConv {
        let (name, class) = match self {
            ArrayElem::Prim(p) => (format!("wrap{}Array", p), format!("ArraySeq$.of{}", p)),
            ArrayElem::Ref => ("wrapRefArray".to_string(), "ArraySeq$.ofRef".to_string()),
            ArrayElem::Generic => ("genericWrapArray".to_string(), "ArraySeq".to_string()),
        };
        let targs = !matches!(self, ArrayElem::Prim(_));
        let result = format!("scala.collection.mutable.{}", class);
        ArrayConv { name, targs, param: self.array_erasure(), result, class: ("scala.collection.mutable", class) }
    }
}

/// The members of `Any` and `AnyRef` the encoder writes: (owner's package, owner, type
/// parameters, parameters, result, an argument clause).
pub(super) const UNIVERSAL: &[&str] = crate::names::ANY_MEMBERS;

#[allow(clippy::type_complexity)]
pub(super) fn universal_member(name: &str) -> Option<(&'static str, &'static str, usize, &'static [&'static str], &'static str, bool)> {
    Some(match name {
        "==" | "!=" | "equals" => ("scala", "Any", 0, &["java.lang.Object"], "scala.Boolean", true),
        "hashCode" => ("scala", "Any", 0, &[], "scala.Int", true),
        "toString" => ("scala", "Any", 0, &[], "java.lang.String", true),
        "##" => ("scala", "Any", 0, &[], "scala.Int", false),
        "getClass" => ("scala", "Any", 1, &[], "java.lang.Class", true),
        "isInstanceOf" => ("scala", "Any", 1, &[], "scala.Boolean", false),
        "asInstanceOf" => ("scala", "Any", 1, &[], "java.lang.Object", false),
        "eq" | "ne" => ("java.lang", "Object", 0, &["java.lang.Object"], "scala.Boolean", true),
        "synchronized" => ("java.lang", "Object", 1, &["java.lang.Object"], "java.lang.Object", true),
        _ => return None,
    })
}

pub(super) fn universal_sig(tps: usize, params: &[&str]) -> Vec<SigParam> {
    let mut sig: Vec<SigParam> = Vec::new();
    if tps > 0 {
        sig.push(SigParam::Types(tps));
    }
    sig.extend(params.iter().map(|p| SigParam::Type(p.to_string())));
    sig
}

/// Whose default getter an omitted argument calls: a method's on the call's qualifier, a
/// constructor's on its class's companion, or the forwarder's of the method an export's object
/// (the term) has under the name, where the call is the forwarder's, as scalac's.
#[derive(Clone, Copy)]
pub(super) enum Getter {
    Method(SymId, Option<Qual>),
    Ctor(ClassId),
    Forwarder(SymId, TExprId, Name),
}

/// What an argument's parameter asks of it.
#[derive(Clone, Copy, Default)]
struct ParamUse {
    repeated: bool,
    elem: Option<TypeId>,
    /// The parameter's type as the call instantiates it, which a closure the typer gave no type
    /// (a comprehension's) takes its result type from.
    expected: Option<TypeId>,
    /// A primitive parameter's class, which an integer literal argument is a constant of.
    primitive: Option<ClassId>,
    /// The parameter's type as the call instantiates it.
    ty: Option<TypeId>,
}

impl<'w, 'a> P<'w, 'a> {
    /// The records the capture keeps of a node.
    pub(super) fn records(&self, e: TExprId) -> NodeRecords {
        match self.w.prog.capture.as_deref() {
            Some(c) => c.records_of(e),
            None => NodeRecords::default(),
        }
    }

    /// The type the typer gave a node, zonked.
    pub(super) fn node_type(&mut self, e: TExprId) -> Option<TypeId> {
        let t = self.w.prog.type_of(e)?;
        Some(self.w.zonk(t))
    }

    /// A node's type, or the declared type of the local, field or static it reads.
    fn term_type(&mut self, e: TExprId) -> Option<TypeId> {
        if let Some(t) = self.node_type(e) {
            return Some(t);
        }
        match self.w.prog.expr(e) {
            TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) => {
                let recorded = self.w.prog.capture.as_deref().and_then(|c| c.local_of(s)).and_then(|r| r.ty);
                let t = recorded.unwrap_or_else(|| self.w.sig_of(s).ret);
                Some(self.w.zonk(t))
            }
            _ => None,
        }
    }

    // ---- positions ------------------------------------------------------------------------

    /// The position of the tree that starts here, standing for the node `e`: its own span, or
    /// the span of the tree it stands in, synthetic.
    pub(super) fn term_at(&mut self, e: Option<TExprId>) -> Option<(FileId, Span)> {
        // A span of a source the products do not hold (a library's body, the std's) is none.
        let spanned = e.and_then(|e| self.w.prog.span_of(e));
        let own = spanned.filter(|&(f, _)| self.w.files.as_slice().get(f.0 as usize).map_or(false, |src| !src.is_std) && !self.w.in_jar(f));
        // A node of a body converted from a pickle (an upstream product's, a library's): the
        // place that pickle gives its tree, as scalac's pickle keeps an inlined body's.
        let pickled = if own.is_none() { e.and_then(|e| self.pickled_place_of(e)) } else { None };
        if pickled.is_some() {
            self.placed_tree = e.and_then(|e| self.records(e).tree).map(|t| (self.buf.addr(), t));
        }
        let own = own.or(pickled.map(|(f, s, _)| (f, s)));
        // The expansion of an upstream product's inline call, spanned in the product's body,
        // that neither places: the call's site, the span the whole build's typer gives the
        // expansion it makes. A node of no span (a search's result two sites share) keeps the
        // enclosing tree's, whatever site it was made at first.
        let own = own.or_else(|| spanned.and(e).and_then(|e| self.inline_site(e)));
        let addr = self.buf.addr();
        let (file, span, point) = match (own, self.span_ctx) {
            // A tree without a point of its own in the converted pickle has none here either,
            // as a node the typer made on the side has none in the whole build.
            (Some((f, s)), _) => (f, s, match pickled {
                Some((_, _, p)) => p,
                None => Some(s.start),
            }),
            (None, Some((f, s))) => (f, Span { start: s.start, end: s.start }, None),
            (None, None) => (self.file, Span { start: self.synth_span.start, end: self.synth_span.start }, None),
        };
        if file != self.file && !self.is_pickled_source(file) && !self.other_lines.contains_key(&file) {
            let lines = Lines::of(&self.w.files.as_slice()[file.0 as usize].text);
            self.other_lines.insert(file, lines);
        }
        let switch = file != self.src_ctx;
        self.positions.push(Pos { addr, file, span, point, def: false, switch });
        own
    }

    /// `within` for the trees that stand for an argument (its `NAMEDARG`, a reference to the val
    /// a call's block binds it to): an inline call of this file at its site, where scalac's
    /// pickle has the call, whatever span the expansion has (a quote's, in the macro's source).
    fn within_argument<R>(&mut self, e: TExprId, f: impl FnOnce(&mut Self) -> R) -> R {
        match self.inline_site(e).filter(|&(file, _)| file == self.file) {
            Some((file, span)) => self.within_at(file, span, f),
            None => self.within(Some(e), f),
        }
    }

    fn inline_site(&self, e: TExprId) -> Option<(FileId, Span)> {
        let site = self.w.prog.capture.as_deref()?.inline_site_of(e)?;
        self.is_own_source(site.0).then_some(site)
    }

    /// Writes `f` as the trees of the node `e`: their source and the span a node made without
    /// one takes.
    fn within<R>(&mut self, e: Option<TExprId>, f: impl FnOnce(&mut Self) -> R) -> R {
        let own = self.term_at(e);
        let (src, ctx) = (self.src_ctx, self.span_ctx);
        if let Some((file, span)) = own {
            self.src_ctx = file;
            self.span_ctx = Some((file, span));
        }
        let r = f(self);
        self.src_ctx = src;
        self.span_ctx = ctx;
        r
    }

    /// `within` for a tree that stands for more of the source than its node: a member the
    /// typer wrote as its receiver, whose selection's span the capture keeps.
    fn within_span<R>(&mut self, e: TExprId, span: Span, f: impl FnOnce(&mut Self) -> R) -> R {
        match self.w.prog.span_of(e).map(|(f, _)| f) {
            Some(file) if self.is_own_source(file) => self.within_at(file, span, f),
            _ => self.within(Some(e), f),
        }
    }

    /// `within` at a source and span the capture recorded for the node `e` stands in: an
    /// ordinary inline method's call, whose expansion `e` is.
    fn within_site<R>(&mut self, site: Option<(FileId, Span)>, e: TExprId, f: impl FnOnce(&mut Self) -> R) -> R {
        match site {
            Some((file, span)) if self.is_own_source(file) => self.within_at(file, span, f),
            _ => self.within(Some(e), f),
        }
    }

    /// Whether a source is one of the program's, which positions may name.
    pub(super) fn is_own_source(&self, file: FileId) -> bool {
        self.w.files.as_slice().get(file.0 as usize).map_or(false, |src| !src.is_std) && !self.w.in_jar(file)
    }

    fn within_at<R>(&mut self, file: FileId, span: Span, f: impl FnOnce(&mut Self) -> R) -> R {
        let addr = self.buf.addr();
        if file != self.file && !self.other_lines.contains_key(&file) {
            let lines = Lines::of(&self.w.files.as_slice()[file.0 as usize].text);
            self.other_lines.insert(file, lines);
        }
        let switch = file != self.src_ctx;
        self.positions.push(Pos { addr, file, span, point: Some(span.start), def: false, switch });
        let (src, ctx) = (self.src_ctx, self.span_ctx);
        self.src_ctx = file;
        self.span_ctx = Some((file, span));
        let r = f(self);
        self.src_ctx = src;
        self.span_ctx = ctx;
        r
    }

    /// A tree the encoder makes in the place of a node, positioned as it.
    pub(super) fn open(&mut self, tag: u8) -> crate::tasty::write::buf::Slot {
        self.buf.byte(tag);
        self.buf.begin_length()
    }

    // ---- the entry ------------------------------------------------------------------------

    /// A definition's right-hand side from its typed body, of the declared type `t`.
    pub(super) fn body_term(&mut self, e: TExprId, t: TypeId) {
        let (src, ctx) = (self.src_ctx, self.span_ctx);
        let scopes = self.reflect_scopes.len();
        self.src_ctx = self.file;
        self.term_to(e, t);
        self.src_ctx = src;
        self.span_ctx = ctx;
        self.reflect_scopes.truncate(scopes);
    }

    /// `e` where scalac's typer expects `t`: a value discarded where it expects `Unit`.
    pub(super) fn term_to(&mut self, e: TExprId, t: TypeId) {
        if let Some(why) = self.js_conversion(e, t) {
            return self.fail(why.to_string());
        }
        let zt = self.w.zonk(t);
        if matches!(self.w.prog.expr(e), TExpr::Lambda(..)) && self.js_alias(zt) == Some("Function") {
            let outer = std::mem::replace(&mut self.expected_fn, Some(t));
            self.term(e);
            self.expected_fn = outer;
            return;
        }
        let unit = self.is_unit(t);
        let outer = std::mem::replace(&mut self.expected_anon, Some((e, t)));
        self.term_in(e, unit);
        self.expected_anon = outer;
    }

    /// Where scalac converts a value to the JavaScript type expected (`js.UndefOr[A]` of an
    /// `A`, a `js.Function` of a Scala function), which the lean std's aliases make the value
    /// itself: the facade forms this stage leaves out.
    fn js_conversion(&mut self, e: TExprId, expected: TypeId) -> Option<&'static str> {
        let expected = self.w.zonk(expected);
        let kinds: Vec<&'static str> = self.union_parts(expected).into_iter().filter_map(|t| self.js_alias(t)).collect();
        let value = self.term_type(e)?;
        let value = self.w.zonk(value);
        let own: Vec<&'static str> = self.union_parts(value).into_iter().filter_map(|t| self.js_alias(t)).collect();
        // A JavaScript function where a Scala one is expected: scalac's `toFunctionN`.
        if kinds.is_empty() && own.contains(&"Function") && self.w.as_function(expected).is_some() {
            return Some("a js.Function scalac converts to a function");
        }
        if kinds.is_empty() {
            return None;
        }
        let unit_like = |p: &mut Self, t: TypeId| p.is_unit(t) || p.is_nothing(t);
        if kinds.contains(&"UndefOr") && own.is_empty() && !unit_like(self, value) {
            return Some("a value scalac converts to js.UndefOr");
        }
        if kinds.contains(&"Function") && !own.contains(&"Function") && self.w.as_function(value).is_some() {
            return Some("a function scalac converts to a js.Function");
        }
        None
    }

    /// The std's alias of Scala.js's `js.UndefOr` or of a `js.FunctionN`.
    fn js_alias(&self, t: TypeId) -> Option<&'static str> {
        let Type::Alias(a, _) = self.w.types.get(t) else { return None };
        let info = self.w.syms.alias(a);
        if !self.is_std_file(info.file) {
            return None;
        }
        let name = self.w.interner.get(info.name);
        if name == "UndefOr" {
            Some("UndefOr")
        } else if name.starts_with("Function") || name.starts_with("ThisFunction") {
            Some("Function")
        } else {
            None
        }
    }

    /// `e`, discarded as `{ e; () }` where the context is `Unit` and `e` is of another type.
    fn term_in(&mut self, e: TExprId, unit: bool) {
        let discard = unit && self.node_type(e).map_or(false, |t| !self.is_unit(t) && !self.is_nothing(t));
        if !discard {
            return self.term(e);
        }
        self.term_at(Some(e));
        let l = self.open(BLOCK);
        self.unit_literal();
        self.term(e);
        self.buf.end_length(l);
    }

    /// Whether the typer gave a node the type `Unit`.
    fn typed_unit(&mut self, e: TExprId) -> bool {
        self.node_type(e).map_or(false, |t| self.is_unit(t))
    }

    pub(super) fn is_unit(&mut self, t: TypeId) -> bool {
        let t = self.w.zonk(t);
        t == self.w.b.t_unit || matches!(self.w.types.get(t), Type::Class(c, _) if c == self.w.b.unit)
    }

    pub(super) fn is_nothing(&mut self, t: TypeId) -> bool {
        let t = self.w.zonk(t);
        t == NOTHING
    }

    pub(super) fn term(&mut self, e: TExprId) {
        // A tree an expansion took from its call site in another source: in an `INLINED` without
        // a call, in the expansion's source, as scalac writes an argument it puts for a
        // parameter, which gives the tree it stands in a span of that source.
        let from_site = !self.inlined_open.is_empty()
            && self.w.prog.span_of(e).map_or(false, |(f, _)| f != self.src_ctx && self.is_own_source(f))
            && self.records(e).inline_calls.is_empty();
        match self.span_ctx.filter(|&(f, _)| from_site && f == self.src_ctx) {
            Some((file, span)) => {
                let addr = self.buf.addr();
                let l = self.open(INLINED);
                self.positions.push(Pos { addr, file, span, point: None, def: false, switch: false });
                self.term_node(e);
                self.buf.end_length(l);
            }
            None => self.term_node(e),
        }
    }

    fn term_node(&mut self, e: TExprId) {
        if !self.lifted_terms.is_empty() && self.lifted_terms.contains_key(&e) {
            return self.lifted_ref(e);
        }
        if let Some(&inlined) = self.inlined_open.last() {
            self.note_expansion_leaf(inlined, e);
        }
        let recs = self.records(e);
        // A number the typer converted without a node of the conversion (`i.toDouble` of an
        // `Int`, which is a double on JavaScript): the conversion, as scalac selects it.
        if let Some(conv) = self.unwritten_conversion(e, &recs) {
            // Inside a named argument's name, outside an ascription.
            if let [Wrap::Named(name)] = recs.wraps.as_slice() {
                let name = *name;
                return self.within(Some(e), |p| {
                    p.buf.byte(NAMEDARG);
                    let n = p.names.simple(&p.name(name));
                    p.buf.nat(n as u64);
                    p.term_at(Some(e));
                    p.buf.byte(SELECT);
                    let c = p.names.simple(&conv);
                    p.buf.nat(c as u64);
                    p.unwrapped(e, &recs);
                });
            }
            self.term_at(Some(e));
            self.buf.byte(SELECT);
            let n = self.names.simple(&conv);
            self.buf.nat(n as u64);
            return self.wrapped(e, &recs.wraps, &recs);
        }
        // A number where a union of one numeric class is expected: the implicit conversion to
        // it, as scalac's typer applies it.
        if let Some((from, to)) = self.union_conversion(e, &recs) {
            self.term_at(Some(e));
            return self.widen_call(from, to, |p| p.wrapped(e, &recs.wraps, &recs));
        }
        self.wrapped(e, &recs.wraps, &recs);
    }

    fn union_conversion(&mut self, e: TExprId, recs: &NodeRecords) -> Option<(ClassId, ClassId)> {
        if !recs.wraps.is_empty() || recs.form.is_some() {
            return None;
        }
        let typed = self.node_type(e)?;
        if !matches!(self.w.types.get(typed), Type::Union(..)) {
            return None;
        }
        let members = self.union_parts(typed);
        let numeric: Vec<ClassId> = members.iter().filter_map(|&t| self.numeric_class(t)).collect();
        let [to] = numeric.as_slice() else { return None };
        let from = self.natural_type(e).and_then(|t| self.numeric_class(t))?;
        (from != *to).then_some((from, *to))
    }

    fn union_parts(&self, t: TypeId) -> Vec<TypeId> {
        match self.w.types.get(t) {
            Type::Union(a, b) => {
                let mut out = self.union_parts(a);
                out.extend(self.union_parts(b));
                out
            }
            _ => vec![t],
        }
    }

    /// `toT` where a read, a call or an ascription of a primitive type has another primitive
    /// type `T` as the typer gave the node.
    fn unwritten_conversion(&mut self, e: TExprId, recs: &NodeRecords) -> Option<String> {
        let declared = match recs.wraps.as_slice() {
            [.., Wrap::Ascribed(t)] => *t,
            [Wrap::Named(_)] | [] if recs.form.is_none() => self.natural_type(e)?,
            _ => return None,
        };
        let typed = self.node_type(e)?;
        let typed = self.w.widen_lit(typed);
        let declared = self.w.widen_lit(declared);
        let (Some(from), Some(to)) = (self.numeric_class(declared), self.numeric_class(typed)) else { return None };
        (from != to && to != self.w.b.boolean && from != self.w.b.boolean).then(|| format!("to{}", self.name(self.w.syms.class(to).name)))
    }

    /// The type of what the encoder writes for a node before a conversion the typer made in
    /// its place: a read's declared type, a call's result, a primitive operation's overload's.
    fn natural_type(&mut self, e: TExprId) -> Option<TypeId> {
        if let Some(&known) = self.natural_types.get(&e) {
            return known;
        }
        let t = self.natural_type_now(e);
        self.natural_types.insert(e, t);
        t
    }

    fn natural_type_now(&mut self, e: TExprId) -> Option<TypeId> {
        let (declared, recv) = match self.w.prog.expr(e) {
            TExpr::Local(s) | TExpr::Static(s) if !self.is_by_name(s) => {
                let recorded = self.w.prog.capture.as_deref().and_then(|c| c.local_of(s)).and_then(|r| r.ty);
                (recorded.unwrap_or_else(|| self.w.sig_of(s).ret), None)
            }
            TExpr::Field(r, s) if !self.is_by_name(s) => (self.w.sig_of(s).ret, Some((r, s))),
            TExpr::CallStatic(s, _) if self.w.sig_of(s).tparams.is_empty() => (self.w.sig_of(s).ret, None),
            TExpr::CallMethod(r, s, _) if self.w.sig_of(s).tparams.is_empty() => (self.w.sig_of(s).ret, Some((r, s))),
            TExpr::Prim(op, a, b) => {
                let (lt, rt) = (self.operand_type(a)?, self.operand_type(b)?);
                self.numeric_class(lt)?;
                (self.op_result(op, lt, rt), None)
            }
            // A conditional's or a block's as its values are written.
            TExpr::If(_, t, Some(f)) => {
                let (wt, wf) = (self.written_type(t)?, self.written_type(f)?);
                (self.numeric_class(wt).is_some() && self.numeric_class(wt) == self.numeric_class(wf)).then_some(wt)?;
                (wt, None)
            }
            TExpr::Block(_, res) => (self.written_type(res)?, None),
            _ => return None,
        };
        // A member's type as its receiver's type instantiates its class's parameters.
        let declared = match recv {
            Some((r, s)) => match (self.term_type(r), self.w.syms.sym(s).owner) {
                (Some(rt), Owner::Class(owner)) => match self.w.base_type(rt, owner) {
                    Some(base) => {
                        let subst = self.w.owner_subst(base);
                        self.w.types.subst(declared, &subst)
                    }
                    None => declared,
                },
                _ => declared,
            },
            None => declared,
        };
        Some(self.w.zonk(declared))
    }

    /// The type of the tree `term` writes for a node: its own where the encoder writes a
    /// conversion to it, its natural one otherwise.
    fn written_type(&mut self, x: TExprId) -> Option<TypeId> {
        let recs = self.records(x);
        let t = if self.unwritten_conversion(x, &recs).is_some() { self.node_type(x) } else { self.natural_type(x).or_else(|| self.node_type(x)) }?;
        Some(self.w.widen_lit(t))
    }

    fn is_type_constructor(&mut self, t: TypeId) -> bool {
        let t = self.w.zonk(t);
        match self.w.types.get(t) {
            Type::Ctor(_) | Type::Lambda(..) => true,
            Type::Param(p) => self.w.syms.tparam(p).arity > 0,
            Type::Class(c, args) => self.w.types.items(args).is_empty() && !self.w.syms.class(c).own_tparams().is_empty(),
            Type::Member(..) | Type::Decl(_) => true,
            _ => false,
        }
    }

    /// An operand's type as written, before the promotion to its operation's rank: a
    /// promotion the typer made in place leaves the operand of its own type.
    fn operand_type(&mut self, x: TExprId) -> Option<TypeId> {
        let u = self.unpromoted(x);
        let in_place = u == x && matches!(self.records(x).form, Some(Form::Promotion));
        let t = if in_place { self.natural_type(x).or_else(|| self.node_type(x))? } else { self.node_type(u)? };
        Some(self.numeric_seen_through(self.w.widen_lit(t)))
    }

    /// An opaque type of a primitive operation's operand as the primitive it stands for: its
    /// definition where the operation sees it, its bound elsewhere, which scalac selects the
    /// operator on; a stuck match type as its declared bound (`Tuple.Size[X] <: Int`).
    fn numeric_seen_through(&mut self, t: TypeId) -> TypeId {
        if self.is_match_type(t) {
            let bound = self.w.match_bound(t);
            let bound = self.w.widen_lit(bound);
            return if self.numeric_class(bound).is_some() { bound } else { t };
        }
        let Type::Class(c, _) = self.w.types.get(t) else { return t };
        self.w.complete_class(c);
        let info = self.w.syms.class(c);
        if info.kind != ClassKind::Opaque || !info.own_tparams().is_empty() {
            return t;
        }
        let candidates: Vec<TypeId> = info.underlying.into_iter().chain(info.parents.iter().copied()).collect();
        candidates.into_iter().map(|u| self.w.widen_lit(u)).find(|&u| self.numeric_class(u).is_some()).unwrap_or(t)
    }

    fn is_match_type(&mut self, t: TypeId) -> bool {
        match self.w.types.get(t) {
            Type::Match(..) => true,
            Type::Alias(a, args) => self.w.alias_expansion(a, args).is_some_and(|x| matches!(self.w.types.get(x), Type::Match(..))),
            _ => false,
        }
    }

    /// `e` inside the source's layers around it, innermost first.
    fn wrapped(&mut self, e: TExprId, wraps: &[Wrap], recs: &NodeRecords) {
        let Some((&outer, inner)) = wraps.split_last() else {
            return self.unwrapped(e, recs);
        };
        match outer {
            Wrap::Ascribed(t) => self.within(Some(e), |p| {
                let l = p.open(TYPED);
                // A literal is a constant of the type it is ascribed (`(0: Short)`).
                let literal = p.numeric_class(t).filter(|_| inner.is_empty() && matches!(p.w.prog.expr(e), TExpr::Int(_)));
                match (literal, p.w.prog.expr(e)) {
                    (Some(c), TExpr::Int(i)) => p.within(Some(e), |p| p.int_constant(i, c)),
                    _ => p.wrapped(e, inner, recs),
                }
                p.tpt(t);
                p.buf.end_length(l);
            }),
            Wrap::Cast(t) => self.within(Some(e), |p| p.cast_to(t, |p| p.wrapped(e, inner, recs))),
            Wrap::Splice(t) => self.within(Some(e), |p| {
                let l = p.open(TYPED);
                p.wrapped(e, inner, recs);
                p.repeated_tpt(t);
                p.buf.end_length(l);
            }),
            Wrap::Named(n) => self.within_argument(e, |p| {
                p.buf.byte(NAMEDARG);
                let name = p.names.simple(&p.name(n));
                p.buf.nat(name as u64);
                p.wrapped(e, inner, recs);
            }),
            Wrap::Unchecked => self.within(Some(e), |p| {
                let t = p.node_type(e).unwrap_or(ANY);
                let l = p.open(TYPED);
                p.wrapped(e, inner, recs);
                p.term_at(None);
                let a = p.open(ANNOTATEDTPT);
                p.tpt(t);
                p.annotation_tree("scala", "unchecked");
                p.buf.end_length(a);
                p.buf.end_length(l);
            }),
            Wrap::Member(n, whole) => self.within_span(e, whole, |p| {
                let recv_ty = p.node_type(e).unwrap_or(ANY);
                let name = p.name(n);
                let recv_ty = p.w.widen_lit(recv_ty);
                // A member of `Any` a val of the receiver's class overrides is the val.
                let own_val = p.w.find_member(recv_ty, n).map(|(s, _)| s).filter(|&s| matches!(p.w.syms.sym(s).kind, SymKind::Val | SymKind::Var) && !p.is_library_sym(s));
                if let Some(s) = own_val {
                    p.buf.byte(SELECT);
                    let k = p.names.simple(&p.name(p.w.syms.sym(s).name));
                    p.buf.nat(k as u64);
                    return p.wrapped(e, inner, recs);
                }
                if matches!(name.as_str(), "toString" | "hashCode" | "##" | "getClass") {
                    let targs: Vec<TypeId> = if name == "getClass" { vec![recv_ty] } else { Vec::new() };
                    return p.universal(&name, |p| p.wrapped(e, inner, recs), &targs, &[]);
                }
                if name.starts_with("unary_") && p.numeric_class(recv_ty).is_some() {
                    p.buf.byte(SELECT);
                    let k = p.names.simple(&name);
                    p.buf.nat(k as u64);
                    return p.wrapped(e, inner, recs);
                }
                match p.w.find_member(recv_ty, n) {
                    Some((s, _)) => p.call_with(s, |p| p.wrapped(e, inner, recs), None, &[], Some(e)),
                    None => p.fail(format!("a body's member {} the typer wrote as its receiver", name)),
                }
            }),
        }
    }

    fn unwrapped(&mut self, e: TExprId, recs: &NodeRecords) {
        if !recs.inline_calls.is_empty() {
            return self.inline_chain(e, recs);
        }
        if let Some(b) = &recs.builtin {
            let b = b.clone();
            // The application of the function a transparent expansion returns, which the typer
            // reduced to the function's body (its span the body's, in the callee's source): at
            // the expansion's call site, as scalac's application of the `INLINED` is.
            let site = self.records(b.recv).inline_calls.iter().rev().find(|c| c.transparent).and_then(|c| c.site);
            let own = self.w.prog.span_of(e).map(|(f, _)| f);
            if let Some((file, span)) = site.filter(|&(f, _)| self.is_own_source(f) && own != Some(f)) {
                return self.within_at(file, span, |p| p.builtin_call(e, &b));
            }
            return self.within(Some(e), |p| p.builtin_call(e, &b));
        }
        self.within(Some(e), |p| p.node(e, recs))
    }

    // ---- nodes ----------------------------------------------------------------------------

    fn node(&mut self, e: TExprId, recs: &NodeRecords) {
        let node = self.w.prog.expr(e);
        let form = recs.form;
        self.call_evidence = recs.evidence.clone();
        if self.quoted_form(e, node, form) {
            return;
        }
        if let Some(Form::Evidence(t)) = form {
            return self.evidence(t);
        }
        if self.inline_body.is_some() && !self.w.inline_accessor_syms.is_empty() && self.through_accessor(node, recs) {
            return;
        }
        match node {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null => match form {
                None | Some(Form::Written(_)) => self.literal(e, node),
                Some(Form::Cast(t)) => self.cast_to(t, |p| p.literal(e, node)),
                Some(Form::Folded(op)) => self.folded(e, node, op),
                Some(Form::Constant(s)) => self.constant_read(s),
                Some(f) => self.unsupported_form(f),
            },
            TExpr::ClassOf(c) => {
                self.buf.byte(CLASSCONST);
                let t = self.class_applied_wild(c);
                self.ty(t);
            }
            TExpr::Local(s) => self.local_ref(s),
            TExpr::This => self.this_ref(),
            TExpr::Super(t) => self.super_of(t),
            TExpr::Module(c) if self.is_reflect_class(c) => self.reflect_module(c, recs.receiver),
            TExpr::Module(c) => self.module_term(c),
            TExpr::Static(s) => self.static_ref(s, recs.targs),
            TExpr::Field(r, s) | TExpr::CallMethod(r, s, _) if !self.receivers.contains_key(&e) && self.wider_than_lean(s, r) => {
                self.fail(format!("the std's {}, whose scala-library result is wider than the lean std's, other than as a receiver", self.name(self.w.syms.sym(s).name)))
            }
            TExpr::Field(r, s) if self.is_quotes_reflect(s) => self.quotes_reflect(r),
            TExpr::Field(r, s) => {
                self.receive_by_name(e, r, s);
                // A named tuple's field, which scalac selects through `NamedTuple`'s members.
                let named = matches!(form, Some(Form::Op(_))) || self.term_type(r).map_or(false, |t| matches!(self.w.types.get(t), Type::Class(c, _) if self.w.interner.get(self.w.syms.class(c).name) == "NamedTuple"));
                if named {
                    return self.fail("a named tuple's field".to_string());
                }
                self.field(r, s, recs.targs)
            }
            TExpr::CallStatic(s, args) => match form {
                None => {
                    let args = self.w.prog.expr_list(args).to_vec();
                    if self.is_std_quotes(s) {
                        return self.quotes_call(e, &args);
                    }
                    if self.names_unqualified(e, s) {
                        return self.applied(s, recs.targs, &args, |p| p.identifier(s), Some(Qual::Owner));
                    }
                    self.call(s, Qual::Owner, recs.targs, &args)
                }
                Some(Form::Member(m)) => {
                    let args = self.w.prog.expr_list(args).to_vec();
                    self.member_helper(m, &args, recs.targs)
                }
                Some(f) => self.unsupported_form(f),
            },
            TExpr::CallMethod(r, s, args) => match form {
                None => {
                    // An assignment through an abstract var, which scalac's tree keeps as one.
                    if let (Some(var), [value]) = (crate::typer::setters::abstract_var_of_setter(&self.w.syms, self.w.interner, s), self.w.prog.expr_list(args)) {
                        let value = *value;
                        self.receive_by_name(e, r, var);
                        let l = self.open(ASSIGN);
                        self.field(r, var, None);
                        self.term(value);
                        self.buf.end_length(l);
                        return;
                    }
                    self.receive_by_name(e, r, s);
                    let args = self.w.prog.expr_list(args).to_vec();
                    if self.is_std_type_of(s) {
                        return self.std_type_of(recs.targs, &args);
                    }
                    if self.is_outer_accessor(s) {
                        return self.outer_this(e, s);
                    }
                    // A member called on the `this` the typer made for an unqualified name.
                    if matches!(self.w.prog.expr(r), TExpr::This) && self.w.prog.span_of(r).is_none() && self.names_unqualified(e, s) {
                        return self.applied(s, recs.targs, &args, |p| p.identifier(s), Some(Qual::Expr(r)));
                    }
                    // `super.m` of a member of `Any` the typer reaches through a symbol of its
                    // own: the member of `Any` or `AnyRef`, as scalac selects it.
                    let name = self.name(self.w.syms.sym(s).name);
                    if matches!(self.w.prog.expr(r), TExpr::Super(_)) && self.w.syms.sym(s).def.is_none() && universal_member(&name).is_some() {
                        let targs: Vec<TypeId> = recs.targs.map(|l| self.w.types.items(l).to_vec()).unwrap_or_default();
                        return self.universal(&name, |p| p.super_ref(), &targs, &args);
                    }
                    // A member only a refinement of a `Selectable` declares: the dynamic call
                    // cast to the member's type, as scalac's `Dynamic.handleStructural` has it.
                    if matches!(name.as_str(), "selectDynamic" | "applyDynamic") && self.structural_result(s, e).is_some() {
                        let t = self.structural_result(s, e).unwrap();
                        return self.dollar_cast(t, |p| p.call(s, Qual::Expr(r), recs.targs, &args));
                    }
                    self.call(s, Qual::Expr(r), recs.targs, &args)
                }
                Some(f) => self.unsupported_form(f),
            },
            TExpr::CallClosure(f, args) => match form {
                // A by-name parameter read: the parameter itself, which the typer evaluates.
                None if args.len == 0 && matches!(self.w.prog.expr(f), TExpr::Local(s) if self.is_by_name(s)) => {
                    let TExpr::Local(s) = self.w.prog.expr(f) else { unreachable!() };
                    self.local_ref(s)
                }
                None if args.len == 0 && matches!(self.w.prog.expr(f), TExpr::Field(_, s) if self.is_by_name(s)) => self.term(f),
                None => self.apply_function(f, args, e),
                Some(f) => self.unsupported_form(f),
            },
            TExpr::New(c, args) => match form {
                None => {
                    let args = self.w.prog.expr_list(args).to_vec();
                    self.new_instance(c, None, recs.targs, &args, e)
                }
                // A case class applied by its name: the companion's `apply`, as scalac's typer
                // selects it; a macro reading the tree, a quote's matching, tells it from `new`.
                Some(Form::CaseApply) => {
                    let args = self.w.prog.expr_list(args).to_vec();
                    self.case_apply(c, recs.targs, &args, e)
                }
                Some(Form::JavaAnnotation) => {
                    let args = self.w.prog.expr_list(args).to_vec();
                    self.java_annotation(c, &args)
                }
                // A given's class instantiated for the given: the call of the given, on the
                // enclosing instance the class of a class's or trait's given takes first.
                Some(Form::GivenCall(g)) => {
                    let args = self.w.prog.expr_list(args).to_vec();
                    let total: usize = self.w.sig_of(g).clauses.iter().map(|c| c.params.len()).sum();
                    match args.split_first() {
                        Some((&outer, rest)) if args.len() > total => self.call(g, Qual::Expr(outer), recs.targs, rest),
                        _ => self.call(g, Qual::Owner, recs.targs, &args),
                    }
                }
                Some(f) => self.unsupported_form(f),
            },
            TExpr::NewVia(s, args) => {
                let args = self.w.prog.expr_list(args).to_vec();
                let c = match self.w.syms.sym(s).owner {
                    Owner::Class(c) => c,
                    _ => return self.fail("a secondary constructor without a class".to_string()),
                };
                self.new_instance(c, Some(s), recs.targs, &args, e)
            }
            TExpr::Lambda(params, body) => match form {
                None => {
                    let params = self.w.prog.sym_list(params).to_vec();
                    self.closure(e, &params, body)
                }
                Some(Form::ByName) => self.term(body),
                Some(f) => self.unsupported_form(f),
            },
            TExpr::If(c, t, els) => {
                let unit = self.typed_unit(e);
                let l = self.open(IF);
                if self.reducible(e).is_some() {
                    self.buf.byte(INLINE);
                }
                self.term(c);
                self.term_in(t, unit);
                match els {
                    Some(x) => self.term_in(x, unit),
                    None => self.unit_literal(),
                }
                self.buf.end_length(l);
            }
            TExpr::While(c, b) => {
                let l = self.open(WHILE);
                self.term(c);
                self.term_in(b, true);
                self.buf.end_length(l);
            }
            TExpr::Block(stmts, res) => match form {
                None => self.block(e, stmts, res),
                Some(Form::Op(n)) => self.builtin_block(e, n, stmts, res),
                Some(Form::CaseCopy) => self.case_copy(e, stmts, res, recs.targs),
                Some(f) => self.unsupported_form(f),
            },
            TExpr::Assign(lhs, rhs) => {
                let l = self.open(ASSIGN);
                self.term(lhs);
                self.term(rhs);
                self.buf.end_length(l);
            }
            TExpr::Match(s, cases) => {
                let unit = self.typed_unit(e);
                let scrut = self.term_type(s).unwrap_or(ANY);
                let cases: Vec<crate::tir::TCase> = self.w.prog.case_list(cases).to_vec();
                if cases.iter().any(|c| self.refines_type_parameter(scrut, c.pat)) {
                    return self.fail("a match whose cases refine a type parameter".to_string());
                }
                let l = self.open(MATCH);
                match self.reducible(e) {
                    // `summonFrom`: no scrutinee, each case a search for its type.
                    Some(Reducible::SummonFrom) => {
                        self.buf.byte(IMPLICIT);
                        for c in cases {
                            self.summon_case(c, unit);
                        }
                        self.buf.end_length(l);
                        return;
                    }
                    Some(Reducible::Inline) => {
                        self.buf.byte(INLINE);
                    }
                    None => {}
                }
                self.term(s);
                for c in cases {
                    self.case(c, scrut, unit);
                }
                self.buf.end_length(l);
            }
            TExpr::Try(i) => {
                let unit = self.typed_unit(e);
                let t = self.w.prog.tries[i as usize].clone();
                let l = self.open(TRY);
                self.term_in(t.body, unit);
                let cases: Vec<crate::tir::TCase> = self.w.prog.case_list(t.cases).to_vec();
                let throwable = self.throwable_type();
                for c in cases {
                    self.case(c, throwable, unit);
                }
                if let Some(f) = t.finalizer {
                    self.term_in(f, true);
                }
                self.buf.end_length(l);
            }
            TExpr::Throw(a, _) => {
                self.buf.byte(THROW);
                self.term(a);
            }
            TExpr::Return(a) => match form {
                Some(Form::Return(m)) => {
                    // A `return` without a value returns `()`, which scalac's typer writes.
                    let l = self.open(RETURN);
                    self.def_ref(Key::Sym(m));
                    if matches!(self.w.prog.expr(a), TExpr::Unit) && self.w.prog.span_of(a).is_none() {
                        self.unit_literal();
                    } else {
                        let ret = self.w.sig_of(m).ret;
                        self.term_to(a, ret);
                    }
                    self.buf.end_length(l);
                }
                _ => self.fail("a return without its method".to_string()),
            },
            TExpr::Prim(op, a, b) => self.prim(e, op, a, b, form),
            TExpr::Unary(op, a) => match form {
                Some(Form::Promotion) => self.term(a),
                Some(Form::Widening) => self.widening(e, op, a),
                Some(Form::Cast(t)) => self.cast_to(t, |p| p.term(a)),
                None => self.unary(e, op, a),
                Some(f) => self.unsupported_form(f),
            },
            TExpr::StrConcat(l) => match form {
                None => {
                    let items = self.w.prog.expr_list(l).to_vec();
                    self.concat(&items)
                }
                Some(Form::Interp { kind, parts }) => {
                    let items = self.w.prog.expr_list(l).to_vec();
                    self.interpolation(kind, parts, &items)
                }
                Some(f) => self.unsupported_form(f),
            },
            TExpr::ToStr(a, conv) => {
                if conv.is_rendering() {
                    // A rendering outside a concatenation is the operand.
                    self.term(a)
                } else {
                    self.universal("toString", |p| p.term(a), &[], &[])
                }
            }
            TExpr::TypeTest(a, _) => match form {
                Some(Form::Test(t)) => self.universal("isInstanceOf", |p| p.term(a), &[t], &[]),
                _ => self.fail("a type test without its type".to_string()),
            },
            TExpr::Js(t, _) if self.w.prog.strings[t.idx()] == "$quoteMatch" => self.withhold(Withheld::Quote),
            TExpr::Splice(_) => self.fail("a splice that is no macro's".to_string()),
            TExpr::Js(t, args) => {
                let args = self.w.prog.expr_list(args).to_vec();
                match form {
                    None => self.template_call(t, &args, recs.targs, e, recs.receiver),
                    Some(Form::Member(m)) => self.member_helper(m, &args, recs.targs),
                    Some(Form::Op(n)) => self.builtin_member(e, n, &args, false),
                    Some(Form::SuperOp(n)) => self.builtin_member(e, n, &args, true),
                    Some(Form::EnumMember(n)) => self.enum_member(e, n, &args),
                    Some(Form::PartialFunction) => self.partial_function(e, &args),
                    Some(Form::Cast(t)) if args.len() == 1 => self.cast_to(t, |p| p.term(args[0])),
                    Some(f) => self.unsupported_form(f),
                }
            }
            TExpr::ObjLit(l) if matches!(form, Some(Form::DynamicLiteral(_))) => {
                let items = self.w.prog.expr_list(l).to_vec();
                self.dynamic_literal(&items)
            }
            TExpr::ArrayLit(l) => match form {
                Some(Form::EnumMember(n)) => self.enum_member(e, n, &[]),
                Some(f) => self.unsupported_form(f),
                None => {
                    let _ = l;
                    self.fail("a raw array".to_string())
                }
            },
            other => {
                let kind = format!("{:?}", other);
                let kind = kind.split('(').next().unwrap_or("").to_string();
                self.fail(format!("a body's {} node", kind));
            }
        }
    }

    /// A member the typer called through its `@js` or `@jvm` template: the member's call, its
    /// receiver the template's first argument where it has one.
    fn template_call(&mut self, t: crate::tir::StrRef, args: &[TExprId], targs: Option<TList>, e: TExprId, receiver: Option<TExprId>) {
        let Some(s) = self.w.prog.template_syms.get(&t).copied() else {
            let text = self.w.prog.strings[t.idx()].clone();
            return self.fail(format!("the template {}", text));
        };
        let total: usize = self.w.sig_of(s).clauses.iter().map(|c| c.params.len()).sum();
        // An extension of a class's instance, selected on the instance the typer left out.
        if let (Some(r), true, Owner::Class(_)) = (receiver, self.w.syms.sym(s).is_extension, self.w.syms.sym(s).owner) {
            return self.call_with(s, |p| p.term(r), targs, args, Some(e));
        }
        if args.len() == total + 1 && !self.w.syms.sym(s).is_extension {
            let r = args[0];
            self.call_with(s, |p| p.term(r), targs, &args[1..], Some(e))
        } else if args.len() == total {
            self.call(s, Qual::Owner, targs, args)
        } else {
            self.fail(format!("a template call of {} with {} arguments", self.name(self.w.syms.sym(s).name), args.len()))
        }
    }

    fn unsupported_form(&mut self, f: Form) {
        let kind = format!("{:?}", f);
        let kind = kind.split('(').next().unwrap_or("").to_string();
        self.fail(format!("a body's {} form", kind));
    }

    // ---- literals -------------------------------------------------------------------------

    fn literal(&mut self, e: TExprId, node: TExpr) {
        let ty = self.node_type(e);
        let class = ty.and_then(|t| match self.w.types.get(self.w.widen_lit(t)) {
            Type::Class(c, _) => Some(c),
            _ => None,
        });
        let b = (self.w.b.byte, self.w.b.short, self.w.b.float, self.w.b.char, self.w.b.long, self.w.b.double);
        match node {
            TExpr::Int(i) if class == Some(b.0) => {
                self.buf.byte(BYTECONST);
                self.buf.long_int(i as i64);
            }
            TExpr::Int(i) if class == Some(b.1) => {
                self.buf.byte(SHORTCONST);
                self.buf.long_int(i as i64);
            }
            TExpr::Int(i) if class == Some(b.3) => {
                self.buf.byte(CHARCONST);
                self.buf.nat(i as u32 as u64);
            }
            TExpr::Int(i) if class == Some(b.4) => {
                self.buf.byte(LONGCONST);
                self.buf.long_int(i as i64);
            }
            TExpr::Int(i) if class == Some(b.5) => {
                self.buf.byte(DOUBLECONST);
                self.buf.long_int((i as f64).to_bits() as i64);
            }
            TExpr::Int(i) => {
                self.buf.byte(INTCONST);
                self.buf.long_int(i as i64);
            }
            TExpr::Long(l) => {
                self.buf.byte(LONGCONST);
                self.buf.long_int(l);
            }
            TExpr::Double(d) if class == Some(b.2) => {
                self.buf.byte(FLOATCONST);
                self.buf.long_int((d as f32).to_bits() as i32 as i64);
            }
            TExpr::Double(d) => {
                self.buf.byte(DOUBLECONST);
                self.buf.long_int(d.to_bits() as i64);
            }
            TExpr::Bool(v) => self.buf.byte(if v { TRUECONST } else { FALSECONST }),
            TExpr::Char(c) => {
                self.buf.byte(CHARCONST);
                self.buf.nat(c as u64);
            }
            TExpr::Str(r) => {
                let text = self.w.prog.strings[r.idx()].clone();
                let n = self.names.simple(&text);
                self.buf.byte(STRINGCONST);
                self.buf.nat(n as u64);
            }
            TExpr::Unit => self.buf.byte(UNITCONST),
            TExpr::Null => self.buf.byte(NULLCONST),
            _ => unreachable!(),
        }
    }

    fn unit_literal(&mut self) {
        self.term_at(None);
        self.buf.byte(UNITCONST);
    }

    /// A class applied to wildcards, `classOf`'s argument.
    fn class_applied_wild(&mut self, c: ClassId) -> TypeId {
        let n = self.w.syms.class(c).own_tparams().len();
        let args: Vec<TypeId> = (0..n).map(|_| WILD).collect();
        self.w.types.class(c, &args)
    }

    // ---- references -----------------------------------------------------------------------

    pub(super) fn local_ref(&mut self, s: SymId) {
        if !self.retained_params.is_empty() && self.retained_params.contains(&s) {
            let params = std::mem::take(&mut self.retained_params);
            // The reference takes the read's place, as the `INLINED` around it does.
            let at = self.positions.last().filter(|p| p.addr == self.buf.addr()).map(|p| (p.file, p.span, p.point));
            let l = self.open(INLINED);
            if let Some((file, span, point)) = at {
                let switch = file != self.src_ctx;
                self.positions.push(Pos { addr: self.buf.addr(), file, span, point, def: false, switch });
            } else {
                self.term_at(None);
            }
            self.local_ref(s);
            self.buf.end_length(l);
            self.retained_params = params;
            return;
        }
        if let Some(&(_, at)) = self.case_binders.iter().rev().find(|(q, _)| *q == s) {
            self.buf.byte(TERMREFDIRECT);
            self.buf.reference(at);
            return;
        }
        if let Some(&(_, at)) = self.params.iter().rev().find(|(q, _)| *q == s) {
            if let Some(&read) = self.spelled_params.get(&s) {
                return self.dollar_cast(read, |p| {
                    p.buf.byte(TERMREFDIRECT);
                    p.buf.reference(at);
                });
            }
            self.buf.byte(TERMREFDIRECT);
            self.buf.reference(at);
            return;
        }
        // A class's constructor parameter: its accessor through the class's `this`, named as an
        // identifier in a parent's arguments.
        if let Owner::Class(c) = self.w.syms.sym(s).owner {
            if self.in_parent_args {
                return self.member_termref(s);
            }
            self.buf.byte(SELECT);
            let n = self.simple_name(self.w.syms.sym(s).name);
            self.buf.nat(n as u64);
            self.term_at(None);
            if self.enclosing.last() == Some(&c) {
                return self.this_ref();
            }
            return self.qual_this(c);
        }
        let name = self.w.interner.get(self.w.syms.sym(s).name).to_string();
        if name.starts_with("$this") {
            let t = self.w.sig_of(s).ret;
            let t = self.w.zonk(t);
            if let Type::This(c) = self.w.types.get(t) {
                return self.qual_this(c);
            }
            return self.fail(format!("the outer this {}", name));
        }
        self.buf.byte(TERMREFDIRECT);
        self.def_ref(Key::Sym(s));
    }

    /// `C.this` of an enclosing class.
    pub(super) fn qual_this(&mut self, c: ClassId) {
        self.buf.byte(QUALTHIS);
        self.term_at(None);
        let n = self.simple_name(self.w.syms.class(c).name);
        self.buf.byte(IDENTTPT);
        self.buf.nat(n as u64);
        self.class_typeref(c);
    }

    fn this_ref(&mut self) {
        // An anonymous class's parent arguments are its creation's, evaluated outside it (a
        // named class reads its constructor's parameters through its own `this` there).
        if let [.., outer, anon] = self.enclosing[..] {
            if self.in_parent_args && self.w.syms.class(anon).kind == ClassKind::Anon {
                return self.qual_this(outer);
            }
        }
        match self.enclosing.last().copied() {
            Some(c) => {
                self.buf.byte(THIS);
                self.class_typeref(c);
            }
            None => self.fail("`this` outside a class".to_string()),
        }
    }

    /// The outer instance an inner class's accessor reads, `C.this`.
    fn outer_this(&mut self, e: TExprId, accessor: SymId) {
        let t = match self.node_type(e) {
            Some(t) => t,
            None => {
                let r = self.w.sig_of(accessor).ret;
                self.w.zonk(r)
            }
        };
        match self.w.types.get(t) {
            Type::This(c) => self.qual_this(c),
            Type::Class(c, _) => self.qual_this(c),
            _ => self.fail("an outer accessor of an unknown class".to_string()),
        }
    }

    fn is_outer_accessor(&self, s: SymId) -> bool {
        self.w.interner.get(self.w.syms.sym(s).name).starts_with("$outer")
    }

    /// A top-level val, an object's val, an enum's value: its selection on its owner's path.
    fn static_ref(&mut self, s: SymId, targs: Option<TList>) {
        let info = self.w.syms.sym(s);
        if info.def.is_none() && matches!(info.owner, Owner::Package(_)) && !self.is_library_sym(s) {
            // A top-level val the typer makes, which no pickle defines: a mirror it derives
            // where scalac's is the class's companion.
            return self.withhold(Withheld::DerivedMirror);
        }
        if self.is_method(s) || targs.is_some() {
            return self.call(s, Qual::Owner, targs, &[]);
        }
        if !self.std_member_shape(s, &None, None) {
            return;
        }
        // A top-level val of the std: the object scala-library has it in, or none.
        if let Some(what) = self.std_helper(s) {
            let Some(StdTarget::Object { pkg, object }) = self.std_target(s) else {
                return self.fail(format!("the std's {}, which scala-library has under another shape", what));
            };
            let name = self.name(self.w.syms.sym(s).name);
            let ret = self.w.sig_of(s).ret;
            let r = self.result_erasure(ret);
            if !self.std_shape(&format!("{}.{}$", pkg, object), &name, &super::shapes::by_name_shape(&r)) {
                return;
            }
            self.buf.byte(SELECT);
            let n = self.names.simple(&name);
            self.buf.nat(n as u64);
            return self.object_path(pkg, object);
        }
        self.buf.byte(SELECT);
        let n = self.simple_name(self.w.syms.sym(s).name);
        self.buf.nat(n as u64);
        self.owner_path(s);
    }

    fn field(&mut self, r: TExprId, s: SymId, targs: Option<TList>) {
        if self.refined_member(r, s) {
            return self.fail("a member a refinement of its receiver's type types".to_string());
        }
        if self.is_method(s) || targs.is_some() {
            if let TExpr::Super(crate::tir::SuperTarget::Mixin(t)) = self.w.prog.expr(r) {
                return self.applied(s, targs, &[], |p| p.super_accessor_select(t, s), None);
            }
            self.select_receiver = self.receiver_of(r);
            return self.call_with(s, |p| p.term(r), targs, &[], None);
        }
        if let Some(e) = self.reflect_entry(s) {
            return self.reflect_select(s, e, |p| p.term(r));
        }
        let receiver = self.receiver_class(r);
        if !self.std_member_shape(s, &None, receiver) {
            return;
        }
        self.buf.byte(SELECT);
        let n = self.simple_name(self.w.syms.sym(s).name);
        self.buf.nat(n as u64);
        self.term(r);
    }

    /// Whether a member is a method: a def, or a given that takes parameters.
    fn is_method(&mut self, s: SymId) -> bool {
        match self.w.syms.sym(s).kind {
            SymKind::Def => true,
            SymKind::Given => {
                let sig = self.w.sig_of(s);
                !(sig.tparams.is_empty() && sig.clauses.is_empty())
            }
            _ => false,
        }
    }

    // ---- calls ----------------------------------------------------------------------------

    /// `s` applied to `args` on `qual`, clause by clause as its signature has them, with the type
    /// arguments the capture recorded.
    fn call(&mut self, s: SymId, qual: Qual, targs: Option<TList>, args: &[TExprId]) {
        if let Qual::Expr(r) = qual {
            if self.refined_member(r, s) {
                return self.fail("a member a refinement of its receiver's type types".to_string());
            }
        }
        match qual {
            Qual::Expr(r) => {
                // `super.m` in a trait: the super accessor `SuperAccessors` made, on `this`.
                if let TExpr::Super(crate::tir::SuperTarget::Mixin(t)) = self.w.prog.expr(r) {
                    return self.applied(s, targs, args, |p| p.super_accessor_select(t, s), Some(qual));
                }
                self.select_receiver = self.receiver_of(r);
                self.applied(s, targs, args, |p| p.select(s, |p| p.term(r)), Some(qual))
            }
            Qual::Owner => {
                if self.w.syms.sym(s).owner == Owner::Local {
                    return self.local_call(s, targs, args);
                }
                self.applied(s, targs, args, |p| p.select(s, |p| p.owner_path(s)), Some(qual))
            }
        }
    }

    /// The read of a constant the typer replaced by its value (`Form::Constant`): a local's
    /// reference (an `inline val`), a member's on its owner's `this` or its object's path.
    fn constant_read(&mut self, s: SymId) {
        let owner = self.w.syms.sym(s).owner;
        match owner {
            Owner::Local => self.local_ref(s),
            Owner::Class(_) | Owner::Package(_) if self.is_local(Key::Sym(s)) && self.static_owner(owner) => self.member_termref(s),
            _ => {
                self.buf.byte(SELECT);
                let n = self.simple_name(self.w.syms.sym(s).name);
                self.buf.nat(n as u64);
                self.owner_path(s);
            }
        }
    }

    /// A read, call or assignment of an inline body that goes through its class's accessor
    /// (`typer::accessors`): the accessor's call on the receiver, where one is.
    fn through_accessor(&mut self, node: TExpr, recs: &NodeRecords) -> bool {
        let Some(owner) = self.inline_owner else { return false };
        let acc = |p: &Self, t: SymId, setter: bool| p.w.inline_accessor_syms.get(&(owner, t, setter)).copied();
        // A companion object's member: its accessor is the inline method's class's, on its `this`.
        let target = match node {
            TExpr::Field(_, t) | TExpr::CallMethod(_, t, _) | TExpr::Static(t) | TExpr::CallStatic(t, _) => Some(t),
            TExpr::Assign(lhs, _) => match self.w.prog.expr(lhs) {
                TExpr::Field(_, t) | TExpr::Static(t) => Some(t),
                _ => None,
            },
            _ => None,
        };
        let companion = target.map_or(false, |t| matches!(self.w.syms.sym(t).owner, Owner::Class(o) if Some(o) == self.w.syms.class(owner).companion && o != owner));
        if companion {
            let t = target.unwrap();
            let (setter, args): (bool, Vec<TExprId>) = match node {
                TExpr::CallMethod(_, _, args) | TExpr::CallStatic(_, args) => (false, self.w.prog.expr_list(args).to_vec()),
                TExpr::Assign(_, v) => (true, vec![v]),
                _ => (false, Vec::new()),
            };
            let Some(a) = acc(self, t, setter) else { return false };
            self.select_receiver = Some((owner, self.w.types.mk(Type::This(owner))));
            let targs = if setter { None } else { recs.targs };
            self.applied(a, targs, &args, |p| p.select(a, |p| p.qual_this(owner)), None);
            return true;
        }
        match node {
            TExpr::Field(r, t) | TExpr::CallMethod(r, t, crate::ast::ListRef { len: 0, .. }) if acc(self, t, false).is_some() => {
                let a = acc(self, t, false).unwrap();
                self.call(a, Qual::Expr(r), None, &[]);
            }
            TExpr::CallMethod(r, t, args) if acc(self, t, false).is_some() => {
                let a = acc(self, t, false).unwrap();
                let args = self.w.prog.expr_list(args).to_vec();
                self.call(a, Qual::Expr(r), recs.targs, &args);
            }
            TExpr::Static(t) if acc(self, t, false).is_some() => {
                let a = acc(self, t, false).unwrap();
                self.call(a, Qual::Owner, None, &[]);
            }
            TExpr::CallStatic(t, args) if acc(self, t, false).is_some() => {
                let a = acc(self, t, false).unwrap();
                let args = self.w.prog.expr_list(args).to_vec();
                self.call(a, Qual::Owner, recs.targs, &args);
            }
            TExpr::Assign(lhs, v) => {
                let (q, t) = match self.w.prog.expr(lhs) {
                    TExpr::Field(r, t) => (Qual::Expr(r), t),
                    TExpr::Static(t) => (Qual::Owner, t),
                    _ => return false,
                };
                let Some(a) = acc(self, t, true) else { return false };
                self.call(a, q, None, &[v]);
            }
            _ => return false,
        }
        true
    }

    /// Whether the source names the method of the call `e` alone where scalac's typer keeps the
    /// identifier and teq's reader of a product reads it back as the source has it: `f(y)` of a
    /// top-level definition, a reference on its `$package`'s `this` that `codeOf` and a macro's
    /// trees tell from the selection; for a definition of this pickle, by the call's text. (A
    /// member of a static object is scalac's identifier too, which the reader would read as the
    /// object's path where the source calls on `this`: it stays the selection, the
    /// "identifiers" equivalence.)
    fn names_unqualified(&self, e: TExprId, s: SymId) -> bool {
        self.w.prog.span_of(e).map_or(false, |(file, span)| self.text_names(file, span, s))
    }

    /// Whether the text at `span` starts with the name of `s`, a definition of this pickle.
    fn text_names(&self, file: FileId, span: Span, s: SymId) -> bool {
        if !self.is_local(Key::Sym(s)) || !matches!(self.w.syms.sym(s).owner, Owner::Package(_)) {
            return false;
        }
        let Some(src) = self.w.files.as_slice().get(file.0 as usize) else { return false };
        let name = self.w.interner.get(self.w.syms.sym(s).name);
        match src.text.get(span.start as usize..) {
            Some(t) => t.starts_with(name) && !t[name.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '$'),
            None => false,
        }
    }

    /// The identifier of a member, placed as the tree it stands in.
    fn identifier(&mut self, s: SymId) {
        self.term_at(None);
        self.member_termref(s);
    }

    /// Whether the receiver's type refines the member `s` (`Ops { def m(x: Int): A }`), whose
    /// type scalac's selection takes from the refinement.
    fn refined_member(&mut self, r: TExprId, s: SymId) -> bool {
        let Some(t) = self.term_type(r) else { return false };
        let t = self.w.deref_alias(t);
        let (_, refinements) = self.w.types.refinements_of(t);
        let name = self.w.syms.sym(s).name;
        refinements.into_iter().any(|id| match self.w.types.refinement(id) {
            crate::types::Refinement::Term(n, ..) | crate::types::Refinement::Val(n, ..) => n == name,
            _ => false,
        })
    }

    /// The path a member of an object or a file is selected on.
    fn owner_path(&mut self, s: SymId) {
        let owner = self.w.syms.sym(s).owner;
        self.term_at(None);
        match owner {
            Owner::Class(o) if self.w.syms.class(o).kind == ClassKind::Object && !self.enclosing.contains(&o) => self.module_term(o),
            Owner::Class(o) if self.enclosing.contains(&o) => self.this_of(o),
            _ => self.term_owner_prefix(s, owner),
        }
    }

    /// Whether an owner's members are reached through a static path: a package, or an object of
    /// a static owner, which scalac's `ref` takes as an identifier and not as a selection of a
    /// `this` it makes without a position.
    pub(super) fn static_owner(&self, owner: Owner) -> bool {
        match owner {
            Owner::Package(_) => true,
            Owner::Class(c) => self.w.syms.class(c).kind == ClassKind::Object && self.static_owner(self.w.syms.class(c).owner),
            Owner::Local => false,
        }
    }

    /// An object as a term: its static path, or its selection on the enclosing instance's
    /// `this`, positioned, for an object of a class.
    pub(super) fn module_term(&mut self, o: ClassId) {
        if !self.std_object_shape(o) {
            return;
        }
        let owner = self.w.syms.class(o).owner;
        if self.static_owner(owner) || self.enclosing.contains(&o) {
            return self.module_termref(o);
        }
        let name = self.name(self.w.syms.class(o).name);
        self.member_by_name(&name, owner)
    }

    /// `C.this.name` of a member of an enclosing class.
    fn member_by_name(&mut self, name: &str, owner: Owner) {
        match owner {
            Owner::Class(c) => {
                self.buf.byte(SELECT);
                let n = self.names.simple(name);
                self.buf.nat(n as u64);
                self.term_at(None);
                self.this_of(c);
            }
            _ => self.fail(format!("the path of {}", name)),
        }
    }

    /// The `this` of an enclosing class: `this` of the innermost one, `C.this` of another.
    fn this_of(&mut self, c: ClassId) {
        if self.enclosing.last() == Some(&c) {
            self.this_ref()
        } else {
            self.qual_this(c)
        }
    }

    fn local_call(&mut self, s: SymId, targs: Option<TList>, args: &[TExprId]) {
        self.applied(
            s,
            targs,
            args,
            |p| {
                p.term_at(None);
                p.buf.byte(TERMREFDIRECT);
                p.def_ref(Key::Sym(s));
            },
            None,
        );
    }

    /// The selection of `s` on the qualifier `qual` writes, applied.
    pub(super) fn call_with(&mut self, s: SymId, qual: impl FnOnce(&mut Self), targs: Option<TList>, args: &[TExprId], _e: Option<TExprId>) {
        self.applied(s, targs, args, |p| p.select(s, qual), None);
    }

    /// `head` applied to the type arguments and to `args` split by the clauses of `s`.
    fn applied(&mut self, s: SymId, targs: Option<TList>, args: &[TExprId], head: impl FnOnce(&mut Self), qual: Option<Qual>) {
        // A call of an export's forwarder (`export_call`), which `head` selects by its name.
        let getter = match (self.forwarder_name, qual) {
            (Some(n), Some(Qual::Expr(o))) => Getter::Forwarder(s, o, n),
            _ => Getter::Method(s, qual),
        };
        let receiver = self.select_receiver.take();
        let evidence = std::mem::take(&mut self.call_evidence);
        let appended = self.appended_varargs(s, receiver.map(|(c, _)| c));
        if let Some(target) = self.std_target(s) {
            return self.std_call(s, target, targs, args, qual, &evidence);
        }
        // The evidence the lean std leaves out of the member's signature, scalac's last clause.
        let tags = self.erased_tag_count(s);
        if tags != evidence.len() {
            return self.fail(format!("the evidence of the std's {}, which the typer did not keep", self.name(self.w.syms.sym(s).name)));
        }
        let sig = self.w.sig_of(s).clone();
        let mut clauses: Vec<Vec<ParamSig>> = sig.clauses.iter().map(|c| c.params.clone()).collect();
        if clauses.is_empty() && self.is_java_method(s) {
            clauses.push(Vec::new());
        }
        let total: usize = clauses.iter().map(|c| c.len()).sum();
        // Arguments before the parameters are what a local class's constructor takes besides
        // them (its captures): the pickle's call does not pass them.
        let skip = args.len().saturating_sub(total);
        let args = &args[skip..];
        if args.len() != total {
            return self.fail(format!("a call of {} with {} arguments for {} parameters", self.name(self.w.syms.sym(s).name), args.len(), total));
        }
        let lens: Vec<usize> = clauses.iter().map(|c| c.len()).collect();
        // A by-name argument is not evaluated at the call, a repeated one is its sequence's.
        let by_name: Vec<bool> = clauses.iter().flatten().map(|p| p.by_name || p.repeated).collect();
        let targs: Vec<TypeId> = targs.map(|l| self.w.types.items(l).to_vec()).unwrap_or_default();
        // scalac's call of a polymorphic method has its type arguments, a type constructor
        // for a higher-kinded parameter.
        if targs.len() != sig.tparams.len() {
            return self.fail(format!("a call of {} with {} of its {} type arguments", self.name(self.w.syms.sym(s).name), targs.len(), sig.tparams.len()));
        }
        if let Some(i) = (0..targs.len()).find(|&i| self.w.syms.tparam(sig.tparams[i]).arity > 0 && !self.is_type_constructor(targs[i])) {
            let name = self.name(self.w.syms.tparam(sig.tparams[i]).name);
            let t = self.w.zonk(targs[i]);
            if matches!(self.w.types.get(t), Type::Var(_) | Type::AppVar(..)) {
                return self.fail(format!("a type argument of {} the typer leaves open", name));
            }
            return self.fail(format!("a type argument of {} that is no type constructor", name));
        }
        // The applications from the selection outwards, as the declaration's `paramss` order
        // them: an extension's type parameters and clauses before the method's own.
        let info = self.sym_info(s);
        // A right-associative extension takes its first own clause before the receiver's, as
        // its pickled signature has them (`method_params`).
        let mut order: Vec<usize> = (0..clauses.len()).collect();
        let name_text = self.name(info.name);
        // The left operand, which the swap would evaluate after the right one: bound first, as
        // scalac's typer lifts it (`{ val x$1 = a; f(b)(x$1) }`).
        let mut lifted: Option<TExprId> = None;
        if info.is_extension && name_text.ends_with(':') {
            let ext_clauses = info.ext_clauses as usize;
            let receiver = (0..clauses.len().min(ext_clauses)).find(|&i| !sig.clauses[i].is_using);
            // Of the clauses applied, where the method's first is among them.
            let swapped = sig.right_assoc_order(ext_clauses).filter(|_| clauses.len() > ext_clauses).map(|o| o.into_iter().filter(|&i| i < clauses.len()).collect::<Vec<usize>>());
            if let (Some(r), Some(swapped)) = (receiver, swapped) {
                order = swapped;
                let at: usize = clauses[..r].iter().map(|c| c.len()).sum();
                if clauses[r].len() == 1 && !self.is_pure_operand(args[at]) {
                    lifted = Some(args[at]);
                }
            }
        }
        let (ext_t, ext_c) = if info.is_extension && !targs.is_empty() { ((info.ext_tparams as usize).min(targs.len()), (info.ext_clauses as usize).min(clauses.len())) } else { (targs.len(), 0) };
        enum Layer {
            Types(std::ops::Range<usize>),
            Clause(usize),
            Evidence,
        }
        let mut layers: Vec<Layer> = Vec::new();
        if ext_t > 0 {
            layers.push(Layer::Types(0..ext_t));
        }
        for (pos, &ci) in order.iter().enumerate() {
            if pos == ext_c && ext_t < targs.len() {
                layers.push(Layer::Types(ext_t..targs.len()));
            }
            layers.push(Layer::Clause(ci));
        }
        if ext_c >= clauses.len() && ext_t < targs.len() {
            layers.push(Layer::Types(ext_t..targs.len()));
        }
        if !evidence.is_empty() {
            layers.push(Layer::Evidence);
        }
        // The parameters' types as the call instantiates them: the type arguments, the owner's
        // as the receiver's type has them.
        let mut subst: Subst = sig.tparams.iter().copied().zip(targs.iter().copied()).collect();
        if let (Some(Qual::Expr(r)), Owner::Class(owner)) = (qual, info.owner) {
            if let Some(rt) = self.node_type(r) {
                if let Some(base) = self.w.base_type(rt, owner) {
                    subst.extend(self.w.owner_subst(base));
                }
            }
        }
        // The block of a call whose defaults scalac evaluates after its arguments: its receiver
        // where it is no path, its arguments the call evaluates, then its defaults, each a val
        // of the block in that order (`Applications`' `liftArgs`).
        let receiver_expr = match qual {
            Some(Qual::Expr(r)) => Some(r),
            _ => None,
        };
        if self.hoisted_after_default(&lens, args) {
            return self.fail("a later clause's named arguments the typer hoists before an earlier clause's default".to_string());
        }
        let binds = self.lift_plan(&lens, args, &by_name, receiver_expr);
        let defaults_lifted = !binds.is_empty();
        if defaults_lifted {
            if lifted.is_some() {
                return self.fail("a right-associative call whose defaults scalac lifts".to_string());
            }
            for &(e, _) in &binds {
                self.lifted_terms.insert(e, crate::tasty::write::pickle::Lifted::Pending(Vec::new()));
            }
        }
        let block = (lifted.is_some() || defaults_lifted).then(|| {
            let b = self.open(BLOCK);
            self.term_at(None);
            b
        });
        let mut lifted_ref = None;
        let opens: Vec<crate::tasty::write::buf::Slot> = layers.iter().rev().map(|l| self.open(if matches!(l, Layer::Types(_)) { TYPEAPPLY } else { APPLY })).collect();
        self.select_receiver = receiver;
        self.varargs_head = appended.is_some();
        head(self);
        self.varargs_head = false;
        self.select_receiver = None;
        let starts: Vec<usize> = clauses.iter().scan(0, |acc, c| {
            let at = *acc;
            *acc += c.len();
            Some(at)
        }).collect();
        for (i, layer) in layers.iter().enumerate() {
            match layer {
                Layer::Types(r) => {
                    for &t in &targs[r.clone()] {
                        self.tpt(t);
                    }
                }
                Layer::Clause(ci) => {
                    for (k, p) in clauses[*ci].iter().enumerate() {
                        let at = starts[*ci] + k;
                        let expected = matches!(self.w.prog.expr(args[at]), TExpr::Lambda(..)).then(|| self.w.types.subst(p.ty, &subst));
                        let primitive = if p.by_name || p.repeated { None } else { self.numeric_class(p.ty) };
                        let ty = (!p.by_name && !p.repeated).then(|| self.w.types.subst(p.ty, &subst));
                        let u = ParamUse { repeated: p.repeated, elem: p.repeated.then_some(p.ty), expected, primitive, ty };
                        if lifted == Some(args[at]) {
                            self.term_at(Some(args[at]));
                            self.buf.byte(TERMREFDIRECT);
                            lifted_ref = Some(self.buf.forward_reference());
                            continue;
                        }
                        self.argument(args[at], u, getter, at, &targs, &args[..at]);
                    }
                    if let Some(elem) = appended.filter(|_| order.last() == Some(ci)) {
                        self.empty_varargs(elem);
                    }
                }
                Layer::Evidence => {
                    for &x in &evidence {
                        self.term(x);
                    }
                }
            }
            self.buf.end_length(opens[layers.len() - 1 - i]);
        }
        if let Some(b) = block.filter(|_| defaults_lifted) {
            let params: Vec<ParamSig> = clauses.iter().flatten().cloned().collect();
            for (e, at) in binds.clone() {
                let ty = match at {
                    Some(i) if matches!(self.records(e).form, Some(Form::Default)) => self.w.types.subst(params[i].ty, &subst),
                    _ => self.node_type(e).map(|t| self.w.widen_lit(t)).unwrap_or(ANY),
                };
                let name = match at {
                    Some(i) => format!("{}$1", self.name(params[i].name)),
                    None => "$1".to_string(),
                };
                self.lifted_val(e, &name, ty, |p| match at {
                    Some(i) if matches!(p.records(e).form, Some(Form::Default)) => {
                        p.within(Some(e), |p| p.default_arg(getter, i, &targs, &args[..i]))
                    }
                    _ => p.term_unnamed(e),
                });
            }
            for (e, _) in binds {
                self.lifted_terms.remove(&e);
            }
            self.buf.end_length(b);
        }
        if let (Some(b), Some(e), Some(slot)) = (block, lifted, lifted_ref) {
            let addr = self.buf.addr();
            self.buf.fill(slot, addr);
            self.term_at(None);
            let l = self.open(VALDEF);
            let n = self.names.simple("x$1");
            self.buf.nat(n as u64);
            let t = self.node_type(e).map(|t| self.w.widen_lit(t)).unwrap_or(ANY);
            self.tpt(t);
            self.term(e);
            self.write_flags(&[SYNTHETIC]);
            self.buf.end_length(l);
            self.buf.end_length(b);
        }
    }

    /// The element type of the Java varargs the inverse table appends to a call of the std member
    /// `s`, which the lean std declares without them.
    fn appended_varargs(&mut self, s: SymId, receiver: Option<ClassId>) -> Option<&'static str> {
        if !self.is_std_sym(s) {
            return None;
        }
        let sig = self.member_signature_on(s, receiver).ok()?;
        let sig = self.with_erased_tags(s, sig);
        self.std_inverse(s, &sig, receiver)?.appended
    }

    /// No arguments passed to a Java varargs parameter of the element class `elem`, as scalac
    /// writes them: `TYPED(REPEATED(E), <repeated>[E])`.
    fn empty_varargs(&mut self, elem: &str) {
        let (pkg, name) = elem.rsplit_once('.').unwrap_or(("", elem));
        self.term_at(None);
        let tl = self.open(TYPED);
        self.term_at(None);
        let r = self.open(REPEATED);
        self.external_tpt(pkg, name);
        self.buf.end_length(r);
        self.term_at(None);
        self.buf.byte(APPLIEDTYPE);
        let l = self.buf.begin_length();
        self.external_typeref("scala", "<repeated>");
        self.external_typeref(pkg, name);
        self.buf.end_length(l);
        self.buf.end_length(tl);
    }

    /// The member and the arguments of an export forwarder's call the typer wrote as the call
    /// of the exported member after the exporting object `o`.
    fn export_call(&mut self, o: TExprId, res: TExprId) -> Option<(SymId, Vec<TExprId>, Name)> {
        let TExpr::Module(oc) = self.w.prog.expr(o) else { return None };
        if !self.records(o).is_empty() {
            return None;
        }
        // The exports of the object or of a trait it inherits them from.
        self.w.complete_class(oc);
        let bases: Vec<ClassId> = std::iter::once(oc).chain(self.w.syms.class(oc).base_types.iter().map(|&(b, _)| b)).collect();
        let scopes: Vec<_> = bases.into_iter().filter_map(|b| self.w.exports_of(b)).collect();
        if scopes.is_empty() {
            return None;
        }
        let (m, args) = match self.w.prog.expr(res) {
            TExpr::CallMethod(r, m, args) if matches!(self.w.prog.expr(r), TExpr::Module(_)) => (m, self.w.prog.expr_list(args).to_vec()),
            TExpr::CallStatic(m, args) => (m, self.w.prog.expr_list(args).to_vec()),
            TExpr::Field(r, m) if matches!(self.w.prog.expr(r), TExpr::Module(_)) => (m, Vec::new()),
            TExpr::Static(m) => (m, Vec::new()),
            _ => return None,
        };
        if !self.records(res).form.is_none() {
            return None;
        }
        if self.w.syms.sym(m).owner == Owner::Class(oc) {
            return None;
        }
        // The forwarder's name, the member's or the one the export renames it to.
        let name = scopes.iter().find_map(|sc| sc.terms.iter().find(|(_, t)| t.sym() == Some(m)).map(|(&n, _)| n))?;
        Some((m, args, name))
    }

    /// What a call's block binds, in order, as scalac's typer lifts a call with defaults
    /// (`Applications`' `liftFun` and `liftArgs`), clause by clause: where a clause takes a
    /// default, the receiver when it is no stable path and the arguments of the clauses before
    /// it that the call evaluates, which the default's getter takes again; where a named argument
    /// of a clause follows a default, the clause's arguments the call evaluates, then its
    /// defaults. A by-name or repeated parameter's argument, and a by-name parameter's default,
    /// stay in place. `None` stands for the receiver.
    fn lift_plan(&self, lens: &[usize], args: &[TExprId], by_name: &[bool], receiver: Option<TExprId>) -> Vec<(TExprId, Option<usize>)> {
        let mut binds: Vec<(TExprId, Option<usize>)> = Vec::new();
        let bound = |binds: &mut Vec<(TExprId, Option<usize>)>, e: TExprId, at: Option<usize>| {
            if !binds.iter().any(|&(x, _)| x == e) {
                binds.push((e, at));
            }
        };
        let is_default = |a: TExprId| matches!(self.records(a).form, Some(Form::Default));
        let strict = |at: usize| !by_name.get(at).copied().unwrap_or(false);
        let mut from = 0;
        for &n in lens {
            let to = (from + n).min(args.len());
            let clause = &args[from.min(args.len())..to];
            if clause.iter().any(|&a| is_default(a)) {
                if let Some(r) = receiver.filter(|&r| !self.is_stable_path(r)) {
                    bound(&mut binds, r, None);
                }
                for at in 0..from.min(args.len()) {
                    let a = args[at];
                    if !is_default(a) && strict(at) && self.evaluated_argument(a) {
                        bound(&mut binds, a, Some(at));
                    }
                }
            }
            if self.names_after_default(&[n], clause) {
                for (i, &a) in clause.iter().enumerate() {
                    if !is_default(a) && strict(from + i) && self.evaluated_argument(a) {
                        bound(&mut binds, a, Some(from + i));
                    }
                }
                for (i, &a) in clause.iter().enumerate() {
                    if is_default(a) && strict(from + i) {
                        bound(&mut binds, a, Some(from + i));
                    }
                }
            }
            from += n;
        }
        binds
    }

    /// Whether a later clause's argument is a temporary the typer hoisted out of the call (a
    /// named argument out of order) while an earlier clause takes a default the block would bind
    /// after it: scalac evaluates the earlier clause's default first.
    fn hoisted_after_default(&self, lens: &[usize], args: &[TExprId]) -> bool {
        let is_default = |a: TExprId| matches!(self.records(a).form, Some(Form::Default));
        let hoisted = |a: TExprId| match self.w.prog.expr(a) {
            TExpr::Local(s) => self.w.interner.get(self.w.syms.sym(s).name).starts_with("h$"),
            _ => false,
        };
        let mut from = 0;
        let mut defaulted = false;
        for &n in lens {
            let clause = &args[from.min(args.len())..(from + n).min(args.len())];
            if defaulted && clause.iter().any(|&a| hoisted(a)) {
                return true;
            }
            defaulted |= clause.iter().any(|&a| is_default(a));
            from += n;
        }
        false
    }

    /// A path whose value no evaluation between two reads of it changes: a val, a parameter, an
    /// object, `this`, a literal and their stable selections, never a var's read.
    fn is_stable_path(&self, x: TExprId) -> bool {
        let stable = |s: SymId| !matches!(self.w.syms.sym(s).kind, SymKind::Var) && self.w.syms.sym(s).mods & mods::MUTABLE == 0;
        match self.w.prog.expr(x) {
            TExpr::Local(s) | TExpr::Static(s) => self.records(x).is_empty() && stable(s),
            TExpr::This | TExpr::Module(_) | TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null => self.records(x).is_empty(),
            TExpr::Field(r, s) => self.records(x).is_empty() && stable(s) && self.is_stable_path(r),
            _ => false,
        }
    }

    /// Whether a clause has a default before an argument the source names: scalac then lifts
    /// the arguments it evaluates and the defaults into a block, in that order (`{ val x1$1 =
    /// p(1); val x3$1 = p(3); val x2$1 = t$default$2; t(x1$1, x2$1, x3 = x3$1) }`), which a call
    /// of the arguments in the parameters' order would evaluate otherwise.
    fn names_after_default(&self, lens: &[usize], args: &[TExprId]) -> bool {
        let mut at = 0;
        lens.iter().any(|&n| {
            let clause = &args[at.min(args.len())..(at + n).min(args.len())];
            at += n;
            clause.iter().position(|&a| matches!(self.records(a).form, Some(Form::Default))).map_or(false, |d| clause[d..].iter().any(|&a| !matches!(self.records(a).form, Some(Form::Default))))
        })
    }

    /// Whether an argument's evaluation can be told from its place: neither a path, a literal
    /// nor a closure, which scalac's `isPureExpr` leaves in place, whatever name it is passed by.
    fn evaluated_argument(&self, a: TExprId) -> bool {
        let recs = self.records(a);
        let named_only = recs.form.is_none() && recs.targs.is_none() && recs.inline_calls.is_empty() && recs.builtin.is_none() && matches!(recs.wraps.as_slice(), [] | [Wrap::Named(_)]);
        // scalac's `isPureExpr`: a lazy val's and an object's reads are idempotent, not pure, and
        // an argument is evaluated where the source has it unless it is pure.
        let pure = named_only
            && match self.w.prog.expr(a) {
                TExpr::Local(s) => !matches!(self.w.syms.sym(s).kind, SymKind::Var) && self.w.syms.sym(s).mods & (mods::MUTABLE | mods::LAZY) == 0,
                TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null | TExpr::This => true,
                _ => false,
            };
        !pure && !matches!(self.w.prog.expr(a), TExpr::Lambda(..))
    }

    /// A reference to a term a call's block binds, its name around it where the argument is
    /// named.
    fn lifted_ref(&mut self, e: TExprId) {
        let named = match self.records(e).wraps.last() {
            Some(Wrap::Named(n)) => Some(*n),
            _ => None,
        };
        self.within_argument(e, |p| {
            match named {
                Some(n) => {
                    p.buf.byte(NAMEDARG);
                    let name = p.names.simple(&p.name(n));
                    p.buf.nat(name as u64);
                    p.within_argument(e, |p| p.buf.byte(TERMREFDIRECT));
                }
                None => {
                    p.term_at(Some(e));
                    p.buf.byte(TERMREFDIRECT);
                }
            }
            let at = match p.lifted_terms.get(&e) {
                Some(crate::tasty::write::pickle::Lifted::At(a)) => Some(*a),
                _ => None,
            };
            match at {
                Some(a) => p.buf.reference(a),
                None => {
                    let slot = p.buf.forward_reference();
                    if let Some(crate::tasty::write::pickle::Lifted::Pending(slots)) = p.lifted_terms.get_mut(&e) {
                        slots.push(slot);
                    }
                }
            }
        })
    }

    /// The val of a call's block a term is bound to, the references to it filled.
    fn lifted_val(&mut self, e: TExprId, name: &str, ty: TypeId, rhs: impl FnOnce(&mut Self)) {
        let addr = self.buf.addr();
        if let Some(crate::tasty::write::pickle::Lifted::Pending(slots)) = self.lifted_terms.insert(e, crate::tasty::write::pickle::Lifted::At(addr)) {
            for s in slots {
                self.buf.fill(s, addr);
            }
        }
        self.term_at(None);
        let l = self.open(VALDEF);
        let n = self.names.simple(name);
        self.buf.nat(n as u64);
        self.tpt(ty);
        // Its own value, not a reference to itself.
        let bound = self.lifted_terms.remove(&e);
        rhs(self);
        if let Some(b) = bound {
            self.lifted_terms.insert(e, b);
        }
        self.write_flags(&[SYNTHETIC]);
        self.buf.end_length(l);
    }

    /// A term without the name a named argument's wrap gives it.
    fn term_unnamed(&mut self, e: TExprId) {
        let recs = self.records(e);
        match recs.wraps.split_last() {
            Some((Wrap::Named(_), inner)) => {
                if self.unwritten_conversion(e, &recs).is_some() {
                    return self.fail("a converted named argument scalac lifts".to_string());
                }
                let inner = inner.to_vec();
                self.wrapped(e, &inner, &recs)
            }
            _ => self.term(e),
        }
    }

    /// Whether evaluating an operand later changes nothing: a path or a literal.
    fn is_pure_operand(&self, e: TExprId) -> bool {
        self.records(e).is_empty()
            && matches!(self.w.prog.expr(e), TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null | TExpr::Local(_) | TExpr::This | TExpr::Module(_))
    }

    fn argument(&mut self, a: TExprId, u: ParamUse, getter: Getter, index: usize, targs: &[TypeId], prior: &[TExprId]) {
        if !self.lifted_terms.is_empty() && self.lifted_terms.contains_key(&a) {
            return self.lifted_ref(a);
        }
        let recs = self.records(a);
        if let Some(why) = u.ty.and_then(|t| self.js_conversion(a, t)) {
            return self.fail(why.to_string());
        }
        match (self.w.prog.expr(a), recs.form) {
            (TExpr::Unit, Some(Form::Default)) => self.within(Some(a), |p| p.default_arg(getter, index, targs, prior)),
            (TExpr::Lambda(ps, body), Some(Form::ByName)) if ps.len == 0 => self.term(body),
            (TExpr::SeqLit(l), Some(Form::Repeated(t))) if recs.wraps.is_empty() => self.within(Some(a), |p| {
                let items = p.w.prog.expr_list(l).to_vec();
                let tl = p.open(TYPED);
                p.term_at(None);
                let r = p.open(REPEATED);
                p.tpt(t);
                for i in items {
                    p.term(i);
                }
                p.buf.end_length(r);
                p.repeated_tpt(t);
                p.buf.end_length(tl);
            }),
            // A sequence passed to a repeated parameter as its arguments, which scalac writes
            // `xs*` (an eta-expansion's `eta$0`).
            (_, None) if u.repeated && recs.wraps.is_empty() && !matches!(self.w.prog.expr(a), TExpr::SeqLit(_)) => self.within(Some(a), |p| {
                let tl = p.open(TYPED);
                p.term(a);
                p.repeated_tpt(u.elem.unwrap_or(ANY));
                p.buf.end_length(tl);
            }),
            // An integer literal is a constant of the primitive parameter's type, which the
            // typer's own may differ from (a literal it widened for an overload of the std).
            (TExpr::Int(i), None) if recs.wraps.is_empty() && u.primitive.is_some() => {
                let c = u.primitive.unwrap();
                self.within(Some(a), |p| p.int_constant(i, c))
            }
            _ => {
                let outer = std::mem::replace(&mut self.expected_fn, u.expected);
                self.term(a);
                self.expected_fn = outer;
            }
        }
    }

    /// The integer `i` as a constant of the primitive class `c`.
    fn int_constant(&mut self, i: i32, c: ClassId) {
        let b = &self.w.b;
        let (byte, short, char, long, float, double) = (b.byte, b.short, b.char, b.long, b.float, b.double);
        if c == byte {
            self.buf.byte(BYTECONST);
            self.buf.long_int(i as i64);
        } else if c == short {
            self.buf.byte(SHORTCONST);
            self.buf.long_int(i as i64);
        } else if c == char {
            self.buf.byte(CHARCONST);
            self.buf.nat(i as u32 as u64);
        } else if c == long {
            self.buf.byte(LONGCONST);
            self.buf.long_int(i as i64);
        } else if c == float {
            self.buf.byte(FLOATCONST);
            self.buf.long_int((i as f32).to_bits() as i32 as i64);
        } else if c == double {
            self.buf.byte(DOUBLECONST);
            self.buf.long_int((i as f64).to_bits() as i64);
        } else {
            self.buf.byte(INTCONST);
            self.buf.long_int(i as i64);
        }
    }

    /// `<repeated>[T]`, the type of a repeated argument.
    pub(super) fn repeated_tpt(&mut self, t: TypeId) {
        self.term_at(None);
        self.buf.byte(APPLIEDTYPE);
        let l = self.buf.begin_length();
        self.external_typeref("scala", "<repeated>");
        self.ty(t);
        self.buf.end_length(l);
    }

    /// An omitted argument: the parameter's default getter, applied to the call's type
    /// arguments and the arguments of the clauses before the parameter's.
    fn default_arg(&mut self, getter: Getter, index: usize, targs: &[TypeId], prior: &[TExprId]) {
        if let Getter::Method(m, _) = getter {
            if self.w.syms.sym(m).owner == Owner::Local {
                return self.fail("a default getter of a local method".to_string());
            }
        }
        let (name, tparams, original, order, owner_class) = match getter {
            Getter::Method(m, _) | Getter::Forwarder(m, ..) => {
                let sig = self.w.sig_of(m).clone();
                let info = self.w.syms.sym(m).clone();
                // A forwarder's getter is named after it, a member of the export's object.
                let (name, owner) = match (getter, info.owner) {
                    (Getter::Forwarder(_, o, n), _) => match self.w.prog.expr(o) {
                        TExpr::Module(oc) => (self.name(n), Some(oc)),
                        _ => return self.fail("a default of an export's forwarder on no object".to_string()),
                    },
                    (_, Owner::Class(c)) => (self.name(info.name), Some(c)),
                    _ => (self.name(info.name), None),
                };
                let order = super::declared_clause_order(self.w, &info, &sig);
                (name, sig.tparams.len(), sig.clauses, order, owner)
            }
            Getter::Ctor(c) => {
                self.w.complete_class(c);
                let info = self.w.syms.class(c);
                let n = info.ctor.len();
                ("<init>".to_string(), info.own_tparams().len(), info.ctor.clone(), (0..n).collect(), None)
            }
        };
        // The clauses in the declaration's order (a right-associative extension's swapped), the
        // getter's index among their parameters and the earlier clauses' arguments it takes, by
        // their places among the call's arguments, which are in the signature's order.
        let starts: Vec<usize> = original.iter().scan(0, |acc, c| {
            let at = *acc;
            *acc += c.params.len();
            Some(at)
        }).collect();
        let Some(ci) = (0..original.len()).find(|&ci| starts[ci] <= index && index < starts[ci] + original[ci].params.len()) else {
            return self.fail("a default of no clause".to_string());
        };
        let clause_of = order.iter().position(|&o| o == ci).unwrap_or(ci);
        let clauses: Vec<ClauseSig> = order.iter().map(|&o| original[o].clone()).collect();
        let earlier_groups: Vec<(usize, usize)> = order[..clause_of].iter().map(|&o| (starts[o], original[o].params.len())).collect();
        let before: usize = earlier_groups.iter().map(|&(_, n)| n).sum();
        let index = before + (index - starts[ci]);
        let earlier: Vec<usize> = earlier_groups.iter().map(|&(_, n)| n).collect();
        let takes = tparams > 0 || clause_of > 0;
        // The earlier clauses' arguments the getter takes: bound by the call's block, or stable
        // paths read again.
        let taken: Vec<TExprId> = earlier_groups.iter().flat_map(|&(at, n)| (at..at + n).map(|i| prior.get(i).copied())).collect::<Option<Vec<_>>>().unwrap_or_default();
        if taken.len() < before || taken.iter().any(|&x| !self.lifted_terms.contains_key(&x) && !self.is_stable_path(x)) {
            return self.fail("a default argument after arguments that are not paths".to_string());
        }
        if let Getter::Method(_, Some(Qual::Expr(r))) = getter {
            if !self.lifted_terms.contains_key(&r) && !self.is_stable_path(r) {
                return self.fail("a default of a call on a receiver that is no stable path".to_string());
            }
        }
        let std_owner = match getter {
            Getter::Method(m, _) => match (self.is_std_sym(m), owner_class) {
                (true, Some(o)) => Some(self.w.library_class_name(o, "")),
                _ => None,
            },
            Getter::Ctor(c) => self.is_std_class(c).then(|| format!("{}$", self.w.library_class_name(c, "").trim_end_matches('$'))),
            Getter::Forwarder(..) => None,
        };
        if let Some(owner) = std_owner {
            let ty = clauses[clause_of].params.get(index - before).map_or(ANY, |p| p.ty);
            if !self.getter_shape(&owner, &name, index, tparams, &clauses, clause_of, ty) {
                return;
            }
        }
        let n = self.names.default_getter(&name, index as u32);
        let opens: Vec<crate::tasty::write::buf::Slot> = earlier.iter().map(|_| self.open(APPLY)).collect();
        let ta = (tparams > 0 && !targs.is_empty()).then(|| self.open(TYPEAPPLY));
        if takes {
            let local = self.local_prefix();
            let Some((params, result)) = self.w.pickled_default_signature(tparams, &clauses, index, &local) else {
                return self.fail("a default getter's signature".to_string());
            };
            let params: Vec<SigParam> = params
                .into_iter()
                .map(|p| match p {
                    crate::typer::loader::declared::PickledSigParam::Types(n) => SigParam::Types(n),
                    crate::typer::loader::declared::PickledSigParam::Term(t) => SigParam::Type(t),
                })
                .collect();
            let sl = self.open(SELECTIN);
            let signed = self.names.signed_ref(n, &params, &result);
            self.buf.nat(signed as u64);
            self.getter_qualifier(getter);
            self.getter_owner(getter, owner_class);
            self.buf.end_length(sl);
        } else {
            self.buf.byte(SELECT);
            self.buf.nat(n as u64);
            self.getter_qualifier(getter);
        }
        if let Some(l) = ta {
            for &t in targs {
                self.tpt(t);
            }
            self.buf.end_length(l);
        }
        let mut at = 0;
        for (i, &k) in earlier.iter().enumerate() {
            for &x in &taken[at..at + k] {
                self.term(x);
            }
            at += k;
            self.buf.end_length(opens[earlier.len() - 1 - i]);
        }
    }

    /// What a default getter is selected on: the call's qualifier, or a constructor's companion.
    fn getter_qualifier(&mut self, getter: Getter) {
        match getter {
            Getter::Method(_, Some(Qual::Expr(r))) | Getter::Forwarder(_, r, _) => self.term(r),
            Getter::Method(m, Some(Qual::Owner)) => self.owner_path(m),
            Getter::Method(_, None) => self.this_ref(),
            Getter::Ctor(c) => self.companion_path(c),
        }
    }

    fn getter_owner(&mut self, getter: Getter, owner: Option<ClassId>) {
        match (getter, owner) {
            (Getter::Ctor(c), _) => self.companion_class_ref(c),
            (_, Some(o)) => self.owner_class_ref(o),
            // A top-level method's getter, a member of its file's `$package` object.
            (Getter::Method(m, _), None) if matches!(self.w.syms.sym(m).owner, Owner::Package(_)) => self.package_object_class_ref(m),
            _ => self.fail("a default getter of a local method".to_string()),
        }
    }

    /// The companion object of a class as a path: the program's, or the one scalac makes.
    pub(super) fn companion_path(&mut self, c: ClassId) {
        self.term_at(None);
        let info = self.class_info(c);
        match info.companion.filter(|&k| self.w.syms.class(k).kind == ClassKind::Object) {
            Some(co) => self.module_term(co),
            None if !self.static_owner(info.owner) => self.member_by_name(&self.name(info.name), info.owner),
            None if self.is_local(Key::SynthModule(c)) => self.def_termref(Key::SynthModule(c), info.owner),
            None => {
                let n = self.simple_name(info.name);
                self.buf.byte(TERMREF);
                self.buf.nat(n as u64);
                let (pkg, _) = self.external_class_name(c);
                match pkg {
                    Some(p) => self.package_path(&p),
                    None => self.owner_prefix(info.owner),
                }
            }
        }
    }

    /// The companion object's class of a class, as a type.
    pub(super) fn companion_class_ref(&mut self, c: ClassId) {
        let info = self.class_info(c);
        match info.companion.filter(|&k| self.w.syms.class(k).kind == ClassKind::Object) {
            Some(co) => self.module_class_typeref(co),
            None if self.is_local(Key::SynthModuleClass(c)) => {
                self.placed = true;
                self.buf.byte(TYPEREFSYMBOL);
                self.def_ref(Key::SynthModuleClass(c));
                self.owner_prefix(info.owner);
            }
            None => {
                let n = self.names.object_class(&self.name(info.name));
                self.buf.byte(TYPEREF);
                self.buf.nat(n as u64);
                let (pkg, _) = self.external_class_name(c);
                match pkg {
                    Some(p) => self.package_path(&p),
                    None => self.owner_prefix(info.owner),
                }
            }
        }
    }

    /// Whether a node is a path, which a tree may name twice: a local, `this`, an object, a
    /// static, a field of a path, a literal.
    fn is_path(&self, x: TExprId) -> bool {
        match self.w.prog.expr(x) {
            TExpr::Local(_) | TExpr::This | TExpr::Module(_) | TExpr::Static(_) | TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null => self.records(x).is_empty(),
            TExpr::Field(r, _) => self.records(x).is_empty() && self.is_path(r),
            _ => false,
        }
    }

    /// The selection of `s`: `SELECTin` with the declaration's signed name and the class that
    /// declares it for a method with a signature, `SELECT` by name otherwise (`TreePickler` 493
    /// to 508).
    pub(super) fn select(&mut self, s: SymId, qual: impl FnOnce(&mut Self)) {
        let s = self.through_parent(s);
        let info = self.sym_info(s);
        if let Some(e) = self.reflect_entry(s) {
            return self.reflect_select(s, e, qual);
        }
        if let Some(what) = self.std_helper(s) {
            return self.fail(format!("the std's {}, which scala-library has under another shape", what));
        }
        let receiver = self.select_receiver.take();
        let sig = match self.member_signature_on(s, receiver.map(|(c, _)| c)) {
            Ok(sig) => self.with_erased_tags(s, sig),
            Err(kind) => return self.fail(format!("the erasure of a {} in a signature", kind)),
        };
        let entry = self.std_inverse(s, &sig, receiver.map(|(c, _)| c));
        if entry.map_or(false, |e| e.appended.is_some()) && !std::mem::take(&mut self.varargs_head) {
            return self.fail(format!("the std's {} selected without its call, whose Java varargs scala-library's member takes", self.name(info.name)));
        }
        let sig = match entry {
            Some(e) if e.via.is_some() => return self.select_via(e, receiver.map(|(_, t)| t), qual),
            Some(e) if e.library.1 == e.lean.1 && sig.is_some() => match sig_of_text(e.library.2) {
                Some(lib) => Some(lib),
                None => return self.fail(format!("the std's {}, which scala-library has under another shape", e.lean.1)),
            },
            _ if !self.std_member_shape(s, &sig, receiver.map(|(c, _)| c)) => return,
            _ => sig,
        };
        let forwarder = self.forwarder_name.take();
        let name = forwarder.unwrap_or(info.name);
        let Some((params, result)) = sig else {
            self.buf.byte(SELECT);
            let n = self.simple_name(name);
            self.buf.nat(n as u64);
            qual(self);
            return;
        };
        let target = self.target_name(s);
        let l = self.open(SELECTIN);
        let n = self.names.signed(&self.name(name), target.as_deref(), &params, &result);
        self.buf.nat(n as u64);
        qual(self);
        // A std member is selected on its receiver's class, where scala-library has it; a
        // member an object exports, on the object, whose forwarder it is.
        let owner = match forwarder {
            Some(_) => receiver.map(|(c, _)| c),
            None if self.is_std_sym(s) => self.shape_owner(s, receiver.map(|(c, _)| c)),
            None => self.export_owner(s, receiver.map(|(c, _)| c)),
        };
        match (owner, info.owner) {
            (Some(c), _) | (None, Owner::Class(c)) => self.owner_class_ref(c),
            _ => self.package_object_class_ref(s),
        }
        self.buf.end_length(l);
    }

    /// A member of a local anonymous class selected outside the class's body, where scalac's
    /// typer avoids the class's type to its parents (`(new $anon(): Runnable).run()` of an
    /// expansion): the member of the nearest parent the selection reaches, else the member itself.
    fn through_parent(&mut self, s: SymId) -> SymId {
        let Owner::Class(c) = self.w.syms.sym(s).owner else { return s };
        if self.w.syms.class(c).kind != ClassKind::Anon || self.enclosing.contains(&c) {
            return s;
        }
        let name = self.w.syms.sym(s).name;
        let own = self.member_signature(s).ok().flatten().map(|(params, _)| super::shapes::sig_text(&params, ""));
        let bases: Vec<ClassId> = self.w.syms.class(c).base_types.iter().map(|&(b, _)| b).filter(|&b| b != c).collect();
        for b in bases {
            self.w.complete_class(b);
            let Some(&m) = self.w.syms.class(b).members.get(&name) else { continue };
            let alternatives: Vec<SymId> = self.w.syms.alternatives(m).map_or(vec![m], |a| a.to_vec());
            for a in alternatives {
                if self.member_signature(a).ok().flatten().map(|(params, _)| super::shapes::sig_text(&params, "")) == own {
                    return a;
                }
            }
        }
        s
    }

    /// The selection of a std member scala-library has on a conversion of its receiver, as
    /// scalac's implicit view writes it: `Conv.f[T..](receiver).member`, the conversion's type
    /// arguments those of the receiver's base type of the class it takes.
    fn select_via(&mut self, e: &'static Inverse, receiver: Option<TypeId>, qual: impl FnOnce(&mut Self)) {
        let Some((object, conv, conv_shape)) = e.via else { return };
        let (Some((conv_params, conv_result)), None) = (sig_of_text(conv_shape), sig_of_text(e.library.2)) else {
            return self.fail(format!("the std's {}, which scala-library has under another shape", e.lean.1));
        };
        let taken = conv_params.iter().find_map(|p| match p {
            SigParam::Type(t) => Some(t.clone()),
            SigParam::Types(_) => None,
        });
        let class = taken.and_then(|t| self.std_class_named(&t));
        let targs: Vec<TypeId> = match (class, receiver) {
            (Some(c), Some(t)) => match self.w.base_type(t, c).map(|b| self.w.types.get(b)) {
                Some(Type::Class(_, args)) => self.w.types.items(args).to_vec(),
                _ => return self.fail(format!("the receiver of the std's {}", e.lean.1)),
            },
            _ => return self.fail(format!("the receiver of the std's {}", e.lean.1)),
        };
        let (pkg, object) = object.rsplit_once('.').unwrap_or(("", object));
        self.buf.byte(SELECT);
        let n = self.names.simple(e.library.1);
        self.buf.nat(n as u64);
        self.term_at(None);
        let a = self.open(APPLY);
        let ta = (!targs.is_empty()).then(|| {
            self.term_at(None);
            self.open(TYPEAPPLY)
        });
        self.term_at(None);
        self.buf.byte(TERMREF);
        let c = self.names.signed(conv, None, &conv_params, &conv_result);
        self.buf.nat(c as u64);
        self.buf.byte(TERMREF);
        let o = self.names.simple(object.trim_end_matches('$'));
        self.buf.nat(o as u64);
        self.package_path(pkg);
        if let Some(ta) = ta {
            for t in targs {
                self.ty(t);
            }
            self.buf.end_length(ta);
        }
        qual(self);
        self.buf.end_length(a);
    }

    /// The object a member is selected on that has it as an export's forwarder: an object of
    /// none of the member's owner's classes.
    fn export_owner(&mut self, s: SymId, receiver: Option<ClassId>) -> Option<ClassId> {
        let rc = receiver?;
        let Owner::Class(o) = self.w.syms.sym(s).owner else { return None };
        if rc == o || self.w.syms.class(rc).kind != ClassKind::Object || self.w.exports_of(rc).is_none() {
            return None;
        }
        self.w.complete_class(rc);
        (!self.w.syms.class(rc).base_types.iter().any(|&(b, _)| b == o)).then_some(rc)
    }

    /// The class of a receiver's type, which a selection on it is resolved in.
    fn receiver_class(&mut self, r: TExprId) -> Option<ClassId> {
        self.receiver_of(r).map(|(c, _)| c)
    }

    /// The class of a receiver's type and the type.
    fn receiver_of(&mut self, r: TExprId) -> Option<(ClassId, TypeId)> {
        // `this` of no recorded type is the innermost class's (`ordinal` in an enum's method).
        if matches!(self.w.prog.expr(r), TExpr::This) && self.node_type(r).is_none() {
            let c = *self.enclosing.last()?;
            return Some((c, self.w.types.mk(Type::This(c))));
        }
        let t = self.term_type(r)?;
        let t = self.w.deref_alias(t);
        let t = self.w.widen_lit(t);
        match self.w.types.get(t) {
            Type::Class(c, _) | Type::This(c) => Some((c, t)),
            _ => None,
        }
    }

    /// The module class of the `<file>$package` object a top-level definition is a member of.
    pub(super) fn package_object_class_ref(&mut self, s: SymId) {
        let Owner::Package(p) = self.w.syms.sym(s).owner else { return self.fail("a top-level definition of no package".to_string()) };
        if let Some((class_at, pk)) = self.package_members_this.filter(|&(_, pk)| pk == p && self.is_local(Key::Sym(s))) {
            self.placed = true;
            self.buf.byte(TYPEREFSYMBOL);
            self.buf.reference(class_at);
            self.package_ref(pk);
            return;
        }
        let file = self.w.syms.sym(s).file;
        let stem = super::super::file_stem(&self.w.files.as_slice()[file.0 as usize].path);
        let n = self.names.object_class(&format!("{}$package", stem));
        self.buf.byte(TYPEREF);
        self.buf.nat(n as u64);
        self.package_ref(p);
    }

    /// The class that declares a member, as `SELECTin` names it: an object's module class.
    pub(super) fn owner_class_ref(&mut self, c: ClassId) {
        if self.w.syms.class(c).kind == ClassKind::Object {
            self.module_class_typeref(c)
        } else {
            self.class_typeref(c)
        }
    }

    /// A field of a Java class the std declares as a parameterless def, which its `@jvm`
    /// template reads (`getstatic`, `getfield`).
    fn is_java_field(&self, s: SymId) -> bool {
        let info = self.w.syms.sym(s);
        let Some(d) = info.def else { return false };
        if !self.is_std_file(info.file) {
            return false;
        }
        let ast = self.w.ast(info.file);
        ast.def(d).annots.iter().any(|a| self.w.interner.get(a.name) == "jvm" && ast.annot_args(a).first().map_or(false, |&arg| matches!(ast.str(arg).split_whitespace().next(), Some("getstatic" | "getfield"))))
    }

    /// A class of Java: one the loader read from a class file, or one the std defines in the
    /// JDK's packages.
    pub(super) fn is_java_class(&self, c: ClassId) -> bool {
        if self.w.loaded.as_ref().map_or(false, |l| l.java.classes.contains_key(&c)) {
            return true;
        }
        let name = scala_name(self.w, c);
        name.starts_with("java.") || name.starts_with("javax.")
    }

    /// A method of a Java class, which takes an argument clause where it takes no parameters.
    pub(super) fn is_java_method(&mut self, s: SymId) -> bool {
        let info = self.w.syms.sym(s);
        if info.kind != SymKind::Def || self.is_java_field(s) {
            return false;
        }
        let Owner::Class(c) = info.owner else { return false };
        if self.w.loaded.as_ref().map_or(false, |l| l.java.classes.contains_key(&c)) {
            return true;
        }
        let name = scala_name(self.w, c);
        name.starts_with("java.") || name.starts_with("javax.")
    }

    /// Where scala-library has a definition of the lean std's top level or one of its extensions:
    /// by the std's file and the receiver's class.
    pub(super) fn std_target(&mut self, s: SymId) -> Option<StdTarget> {
        let info = self.sym_info(s);
        let src = self.w.files.as_slice().get(info.file.0 as usize)?;
        if !src.is_std || self.w.in_jar(info.file) || self.std_helper(s).is_none() {
            return None;
        }
        let stem = super::super::file_stem(&src.path);
        let name = self.name(info.name);
        if !info.is_extension {
            let object = match (stem.as_str(), name.as_str()) {
                ("prelude", "println" | "print" | "???" | "identity" | "require" | "assert" | "implicitly" | "locally" | "valueOf") => ("scala", "Predef"),
                ("prelude", "summon") => return Some(StdTarget::Summon),
                ("evidence", "conforms") => return Some(StdTarget::Conforms),
                ("math", _) => ("scala.math", "package"),
                // `scala/compiletime/package.scala`'s top level, which an inline body calls.
                ("compiletime", _) => ("scala.compiletime", "package$package"),
                ("js", _) => ("scala.scalajs.js", "package"),
                ("timers", _) => ("scala.scalajs.js.timers", "package"),
                _ => return None,
            };
            return Some(StdTarget::Object { pkg: object.0, object: object.1 });
        }
        let recv = self.w.sig_of(s).clauses.first().and_then(|c| c.params.first()).map(|p| p.ty)?;
        let recv = self.w.widen_lit(recv);
        let class = match self.w.types.get(recv) {
            Type::Class(c, _) => self.name(self.w.syms.class(c).name),
            Type::Param(_) => "*".to_string(),
            _ => return None,
        };
        Some(match (stem.as_str(), class.as_str(), name.as_str()) {
            ("string", "String", _) => StdTarget::Receiver { pkg: "java.lang", class: "String", java: true },
            ("strings", "String", _) => StdTarget::Wrapped { conv: "augmentString", targs: false, param: "java.lang.String", result: "java.lang.String", class: ("scala.collection", "StringOps") },
            ("strings" | "collections", "Char", _) => StdTarget::Wrapped { conv: "charWrapper", targs: false, param: "scala.Char", result: "scala.Char", class: ("scala.runtime", "RichChar") },
            ("collections", "Int", _) => StdTarget::Wrapped { conv: "intWrapper", targs: false, param: "scala.Int", result: "scala.Int", class: ("scala.runtime", "RichInt") },
            ("collections", "Long", _) => StdTarget::Wrapped { conv: "longWrapper", targs: false, param: "scala.Long", result: "scala.Long", class: ("scala.runtime", "RichLong") },
            ("collections", "Double", _) => StdTarget::Wrapped { conv: "doubleWrapper", targs: false, param: "scala.Double", result: "scala.Double", class: ("scala.runtime", "RichDouble") },
            ("prelude", "*", "->") => StdTarget::Wrapped { conv: "ArrowAssoc", targs: true, param: "java.lang.Object", result: "java.lang.Object", class: ("scala.Predef", "ArrowAssoc") },
            ("collections" | "core", "Array", _) => StdTarget::Array,
            ("core", "Function1", _) => StdTarget::Receiver { pkg: "scala", class: "Function1", java: false },
            ("core", "Tuple2", _) => StdTarget::Receiver { pkg: "scala", class: "Tuple2", java: false },
            ("collections", "Option", _) => StdTarget::Receiver { pkg: "scala", class: "Option", java: false },
            _ => return None,
        })
    }

    /// A call of a lean std definition as scala-library has it.
    fn std_call(&mut self, s: SymId, target: StdTarget, targs: Option<TList>, args: &[TExprId], _qual: Option<Qual>, evidence: &[TExprId]) {
        let info = self.sym_info(s);
        let name = self.name(info.name);
        let sig = self.w.sig_of(s).clone();
        let targs: Vec<TypeId> = targs.map(|l| self.w.types.items(l).to_vec()).unwrap_or_default();
        let defaulted = args.iter().any(|&a| matches!(self.records(a).form, Some(Form::Default)));
        // An inline body keeps its calls, the transparent `summon`'s among them
        // (`Predef.summon[Show[Int]](given_Show_Int)`): the call of `Predef`'s member.
        let target = match target {
            StdTarget::Summon if self.inline_body.is_some() => StdTarget::Object { pkg: "scala", object: "Predef" },
            other => other,
        };
        match target {
            // scalac's expansion of the transparent `Predef.summon`: its argument in the
            // `INLINED` of the parameter's proxy, the call the class `Predef$`, by which the
            // reader of a product calls the std's `summon`.
            StdTarget::Summon => match args {
                [x] => {
                    if !self.std_shape("scala", "Predef", "object") {
                        return;
                    }
                    let l = self.open(INLINED);
                    self.term_at(None);
                    let arg = self.open(INLINED);
                    // A static given as its path, as scalac writes it: scalac makes a selection
                    // of a given object here an identifier without a span.
                    if self.is_summoned_path(*x) {
                        self.term_at(Some(*x));
                        self.summoned_path_type(*x);
                    } else {
                        self.term(*x);
                    }
                    self.buf.end_length(arg);
                    self.term_at(None);
                    self.buf.byte(IDENTTPT);
                    let n = self.names.object_class("Predef");
                    self.buf.nat(n as u64);
                    self.object_class_ref("scala", "Predef");
                    self.buf.end_length(l);
                }
                _ => self.fail("summon's evidence".to_string()),
            },
            StdTarget::Conforms => {
                // scala-library's `Predef.$conforms[A]`.
                if !self.predef_shape("$conforms", &[SigParam::Types(1)], "scala.Function1") {
                    return;
                }
                let t = targs.first().copied().unwrap_or(ANY);
                let ta = self.open(TYPEAPPLY);
                let sl = self.open(SELECTIN);
                let n = self.names.signed("$conforms", None, &[SigParam::Types(1)], "scala.Function1");
                self.buf.nat(n as u64);
                self.object_path("scala", "Predef");
                self.object_class_ref("scala", "Predef");
                self.buf.end_length(sl);
                self.tpt(t);
                self.buf.end_length(ta);
            }
            StdTarget::Object { pkg, object } => {
                if !self.std_shape(pkg, object, "object") {
                    return;
                }
                let owner = format!("{}.{}$", pkg, object);
                if defaulted {
                    // `println()`: the overload without the parameter, which the std's default stands for.
                    if name == "println" && args.len() == 1 {
                        if !self.std_shape(&owner, "println", ":scala.Unit") {
                            return;
                        }
                        let l = self.open(APPLY);
                        let sl = self.open(SELECTIN);
                        let n = self.names.signed("println", None, &[], "scala.Unit");
                        self.buf.nat(n as u64);
                        self.object_path(pkg, object);
                        self.object_class_ref(pkg, object);
                        self.buf.end_length(sl);
                        self.buf.end_length(l);
                        return;
                    }
                    return self.fail(format!("a default of the std's {}", name));
                }
                if sig.clauses.is_empty() && sig.tparams.is_empty() {
                    let r = self.result_erasure(sig.ret);
                    if !self.std_shape(&owner, &name, &super::shapes::by_name_shape(&r)) {
                        return;
                    }
                    self.buf.byte(SELECT);
                    let n = self.names.simple(&name);
                    self.buf.nat(n as u64);
                    return self.object_path(pkg, object);
                }
                let Some((params, kept, result)) = self.std_signature(s, &targs, args, (0, 0)) else { return self.fail(format!("the erasure of the std's {}", name)) };
                let clauses: Vec<usize> = sig.clauses.iter().map(|c| c.params.len()).collect();
                self.std_applied(&owner, &name, &params, &result, &clauses, &kept, args, |p| p.object_path(pkg, object), |p| p.object_class_ref(pkg, object));
            }
            StdTarget::Receiver { pkg, class, .. } | StdTarget::Wrapped { class: (pkg, class), .. } if !defaulted => {
                let (et, ec) = ((info.ext_tparams as usize).min(targs.len()), (info.ext_clauses as usize).min(sig.clauses.len()));
                let ext_args: usize = sig.clauses[..ec].iter().map(|c| c.params.len()).sum();
                if ec != 1 || ext_args != 1 || args.len() < 1 {
                    return self.fail(format!("the receiver of the std's {}", name));
                }
                let recv = args[0];
                let rest = &args[1..];
                let own = self.std_own_clauses(s, target);
                let Some((params, own_targs, result)) = self.std_signature(s, &targs, args, (et, ec)) else { return self.fail(format!("the erasure of the std's {}", name)) };
                let ext_targs: Vec<TypeId> = targs[..et].to_vec();
                let wrapped = match target {
                    StdTarget::Wrapped { conv, targs: conv_targs, param, result, .. } => Some((conv, conv_targs, param, result)),
                    _ => None,
                };
                if let Some((conv, conv_targs, param, result)) = wrapped {
                    let sig = conversion_sig(conv_targs, param);
                    if !self.std_shape("scala", "Predef", "object") || !self.predef_shape(conv, &sig, result) {
                        return;
                    }
                }
                let recv_ty = self.node_type(recv).map(|t| self.w.widen_lit(t));
                let owner = super::shapes::library_owner(pkg, class);
                let auto_applied = sig.clauses.len() == ec && own == [0];
                self.std_applied_as(
                    &owner,
                    &name,
                    &params,
                    &result,
                    &own,
                    &own_targs,
                    rest,
                    auto_applied,
                    |p| match wrapped {
                        None => p.term(recv),
                        Some((conv, conv_targs, param, result)) => {
                            p.term_at(None);
                            let l = p.open(APPLY);
                            let ta = conv_targs.then(|| p.open(TYPEAPPLY));
                            let sl = p.open(SELECTIN);
                            let sig = conversion_sig(conv_targs, param);
                            let n = p.names.signed(conv, None, &sig, result);
                            p.buf.nat(n as u64);
                            p.object_path("scala", "Predef");
                            p.object_class_ref("scala", "Predef");
                            p.buf.end_length(sl);
                            if let Some(ta) = ta {
                                let t = ext_targs.first().copied().or(recv_ty).unwrap_or(ANY);
                                p.tpt(t);
                                p.buf.end_length(ta);
                            }
                            p.term(recv);
                            p.buf.end_length(l);
                        }
                    },
                    |p| p.library_class_ref(pkg, class),
                );
            }
            StdTarget::Array if !defaulted => self.array_call(s, &targs, args, evidence),
            _ => self.fail(format!("a default of the std's {}", name)),
        }
    }

    /// A std extension of arrays as scala-library has the member: the array's own
    /// (`xs.length`), `ArrayOps`' (`refArrayOps[T](xs).map[B](f)(ct)`) or an `ArraySeq`'s
    /// (`wrapRefArray[T](xs).mkString(",")`), the first that has it as scalac's implicit search
    /// finds it.
    fn array_call(&mut self, s: SymId, targs: &[TypeId], args: &[TExprId], evidence: &[TExprId]) {
        let info = self.sym_info(s);
        let name = self.name(info.name);
        let sig = self.w.sig_of(s).clone();
        let (et, ec) = ((info.ext_tparams as usize).min(targs.len()), (info.ext_clauses as usize).min(sig.clauses.len()));
        let ext_args: usize = sig.clauses[..ec].iter().map(|c| c.params.len()).sum();
        if name == "unzip" {
            return self.fail("the std's ArrayOps.unzip, whose scala-library form takes `asPair` before its tags".to_string());
        }
        if ec != 1 || ext_args != 1 || args.is_empty() || et != 1 {
            return self.fail(format!("the receiver of the std's {}", name));
        }
        let (recv, rest) = (args[0], &args[1..]);
        let own = self.std_own_clauses(s, StdTarget::Array);
        let Some((mut params, own_targs, result)) = self.std_signature(s, targs, args, (et, ec)) else { return self.fail(format!("the erasure of the std's {}", name)) };
        // The `ClassTag`s of a `@jvmEvidence` member, erased from its signature here, are
        // scala-library's last clause, the evidence the typer selected for them.
        let tags = self.erased_tag_count(s);
        if tags != evidence.len() {
            return self.fail(format!("the evidence of the std's {}, which the typer did not keep", name));
        }
        params.extend((0..tags).map(|_| SigParam::Type("scala.reflect.ClassTag".to_string())));
        let elem_ty = self.w.zonk(targs[0]);
        let elem = self.array_elem(elem_ty);
        let by_name = params.is_empty() && own.is_empty() && own_targs.is_empty();
        let shape = if by_name { super::shapes::by_name_shape(&result) } else { super::shapes::sig_text(&params, &result) };
        let clause = (tags > 0).then(|| {
            self.term_at(None);
            self.open(APPLY)
        });
        if self.std_allowed("scala.Array", &name, &shape) {
            self.std_applied("scala.Array", &name, &params, &result, &own, &own_targs, rest, |p| p.term(recv), |p| p.external_typeref("scala", "Array"));
            return self.evidence_clause(clause, evidence);
        }
        let conv = [elem.ops(), elem.wrap()].into_iter().find(|c| self.std_allowed(&super::shapes::library_owner(c.class.0, &c.class.1), &name, &shape));
        let Some(conv) = conv else { return self.fail(format!("the std's {} of an array, which scala-library has under another shape", name)) };
        if !self.std_shape("scala", "Predef", "object") || !self.predef_shape(&conv.name, &conversion_sig(conv.targs, &conv.param), &conv.result) {
            return;
        }
        let owner = super::shapes::library_owner(conv.class.0, &conv.class.1);
        self.std_applied(
            &owner,
            &name,
            &params,
            &result,
            &own,
            &own_targs,
            rest,
            |p| {
                p.term_at(None);
                let l = p.open(APPLY);
                let ta = conv.targs.then(|| p.open(TYPEAPPLY));
                let sl = p.open(SELECTIN);
                let n = p.names.signed(&conv.name, None, &conversion_sig(conv.targs, &conv.param), &conv.result);
                p.buf.nat(n as u64);
                p.object_path("scala", "Predef");
                p.object_class_ref("scala", "Predef");
                p.buf.end_length(sl);
                if let Some(ta) = ta {
                    p.tpt(elem_ty);
                    p.buf.end_length(ta);
                }
                p.term(recv);
                p.buf.end_length(l);
            },
            |p| p.library_class_ref(conv.class.0, &conv.class.1),
        );
        self.evidence_clause(clause, evidence);
    }

    /// The evidence the typer selected, in the clause opened.
    fn evidence_clause(&mut self, clause: Option<crate::tasty::write::buf::Slot>, evidence: &[TExprId]) {
        let Some(l) = clause else { return };
        for &x in evidence {
            self.term(x);
        }
        self.buf.end_length(l);
    }

    /// A given the typer synthesized, as scalac's `Synthesizer` writes it: a `ClassTag`, `<:<`'s
    /// `refl`, `CanEqual.canEqualAny`, `NotGiven.value`, a `ValueOf` of a constant.
    fn evidence(&mut self, target: TypeId) {
        let target = self.w.zonk(target);
        let target = self.w.deref_alias(target);
        if let Some(c) = self.mirrored_case_class(target) {
            return self.product_mirror(c);
        }
        let Type::Class(c, args) = self.w.types.get(target) else { return self.fail("a synthesized given of no class".to_string()) };
        let args: Vec<TypeId> = self.w.types.items(args).to_vec();
        let class = scala_name(self.w, c);
        match (class.as_str(), args.as_slice()) {
            ("scala.reflect.ClassTag", [t]) => self.class_tag(*t),
            ("scala.<:<" | "scala.=:=", [a, _]) => {
                self.term_at(None);
                let l = self.open(TYPEAPPLY);
                self.buf.byte(TERMREF);
                let n = self.names.signed("refl", None, &[SigParam::Types(1)], "scala.=:=");
                self.buf.nat(n as u64);
                self.buf.byte(TERMREF);
                let o = self.names.simple("<:<");
                self.buf.nat(o as u64);
                self.package_path("scala");
                self.tpt(*a);
                self.buf.end_length(l);
            }
            ("scala.CanEqual", [a, b]) => {
                self.term_at(None);
                let l = self.open(TYPEAPPLY);
                self.buf.byte(TERMREF);
                let n = self.names.signed("canEqualAny", None, &[SigParam::Types(2)], "scala.CanEqual");
                self.buf.nat(n as u64);
                self.buf.byte(THIS);
                self.object_class_ref("scala", "CanEqual");
                self.tpt(*a);
                self.tpt(*b);
                self.buf.end_length(l);
            }
            ("scala.util.NotGiven", [_]) => {
                self.term_at(None);
                self.buf.byte(TERMREF);
                let n = self.names.simple("value");
                self.buf.nat(n as u64);
                self.buf.byte(THIS);
                self.object_class_ref("scala.util", "NotGiven");
            }
            _ => self.fail(format!("a synthesized given of {}", class)),
        }
    }

    /// The case class a synthesized `Mirror.Of`, `Mirror.ProductOf` or `Mirror.Product` is of,
    /// where scalac's `Synthesizer` takes its companion for the mirror (`productMirror`'s
    /// `companionPath`, `companion_mirror_of`). A generic class's, a sum's, a tuple's and an
    /// anonymous mirror are none of these.
    fn mirrored_case_class(&mut self, target: TypeId) -> Option<ClassId> {
        let mut t = target;
        let mut mirrored = None;
        let base = loop {
            match self.w.types.get(t) {
                Type::Refined(parent, r) => {
                    if let crate::types::Refinement::Alias(n, ty) = self.w.types.refinement(r) {
                        if self.w.interner.get(n) == "MirroredType" {
                            mirrored = Some(ty);
                        }
                    }
                    t = parent;
                }
                Type::Class(k, _) => break k,
                _ => return None,
            }
        };
        if !matches!(scala_name(self.w, base).as_str(), "scala.deriving.Mirror" | "scala.deriving.Mirror.Product" | "scala.deriving.Mirror$.Product") {
            return None;
        }
        let mirrored = self.w.deref_alias(mirrored?);
        let Type::Class(c, args) = self.w.types.get(mirrored) else { return None };
        (self.w.types.items(args).is_empty() && self.companion_mirror_of(c)).then_some(c)
    }

    /// Whether the companion of the case class `c`, the source's or the one written for it, is
    /// the class's mirror (`mirrored_case_class`): a case class of a package or of objects, of one
    /// parameter clause of no repeated or by-name parameter and of no type parameters.
    pub(super) fn companion_mirror_of(&mut self, c: ClassId) -> bool {
        self.w.complete_class(c);
        let info = self.w.syms.class(c);
        let static_owner = match info.owner {
            Owner::Package(_) => true,
            Owner::Class(o) => self.w.syms.class(o).kind == ClassKind::Object && self.static_owner(info.owner),
            Owner::Local => false,
        };
        info.tparams.is_empty()
            && info.mods & crate::ast::mods::CASE != 0
            && info.kind == ClassKind::Class
            && static_owner
            && info.ctor.len() == 1
            && info.mods & crate::ast::mods::ABSTRACT == 0
            && !info.ctor[0].params.iter().any(|p| p.repeated || p.by_name)
    }

    /// `C.$asInstanceOf$[Mirror.Product { type MirroredMonoType = C; type MirroredType = C;
    /// type MirroredLabel = "C"; type MirroredElemTypes = T1 *: .. *: EmptyTuple;
    /// type MirroredElemLabels = "f1" *: .. *: EmptyTuple }]`, the companion `C` as the mirror,
    /// as scalac's `Synthesizer` writes it.
    fn product_mirror(&mut self, c: ClassId) {
        self.term_at(None);
        let l = self.open(TYPEAPPLY);
        let sl = self.open(SELECTIN);
        let n = self.names.signed("$asInstanceOf$", None, &[SigParam::Types(1)], "java.lang.Object");
        self.buf.nat(n as u64);
        self.companion_path(c);
        self.external_typeref("scala", "Any");
        self.buf.end_length(sl);
        // The type argument is a tree.
        self.mark_tree();
        self.product_mirror_type(c);
        self.buf.end_length(l);
    }

    /// The refinement a case class's companion is its mirror at (`product_mirror`), its members
    /// nesting the first innermost, each written around the next.
    pub(super) fn product_mirror_type(&mut self, c: ClassId) {
        let fields: Vec<(crate::intern::Name, TypeId)> = self.w.syms.class(c).ctor[0].params.iter().map(|p| (p.name, p.ty)).collect();
        let label = self.name(self.w.syms.class(c).name).to_string();
        let members = ["MirroredElemLabels", "MirroredElemTypes", "MirroredLabel", "MirroredType", "MirroredMonoType"];
        let mut open = Vec::with_capacity(members.len());
        for m in members {
            self.buf.byte(REFINEDTYPE);
            open.push(self.buf.begin_length());
            let n = self.names.simple(m);
            self.buf.nat(n as u64);
        }
        self.mirror_parent("Product");
        for (i, len) in open.into_iter().rev().enumerate() {
            self.buf.byte(TYPEBOUNDS);
            let b = self.buf.begin_length();
            match i {
                0 | 1 => self.class_typeref(c),
                2 => {
                    let n = self.names.simple(&label);
                    self.buf.byte(STRINGCONST);
                    self.buf.nat(n as u64);
                }
                3 => {
                    let types: Vec<TypeId> = fields.iter().map(|&(_, t)| t).collect();
                    self.cons_chain(types.len(), &mut |p, k| p.ty(types[k]));
                }
                _ => {
                    let labels: Vec<String> = fields.iter().map(|&(n, _)| self.name(n).to_string()).collect();
                    self.cons_chain(labels.len(), &mut |p, k| {
                        let n = p.names.simple(&labels[k]);
                        p.buf.byte(STRINGCONST);
                        p.buf.nat(n as u64);
                    });
                }
            }
            self.buf.end_length(b);
            self.buf.end_length(len);
        }
    }

    /// `e0 *: e1 *: .. *: EmptyTuple` of `n` elements, each written by `elem`.
    fn cons_chain(&mut self, n: usize, elem: &mut dyn FnMut(&mut Self, usize)) {
        let mut opens = Vec::with_capacity(n);
        for k in 0..n {
            self.buf.byte(APPLIEDTYPE);
            opens.push(self.buf.begin_length());
            self.external_typeref("scala", "*:");
            elem(self, k);
        }
        self.external_typeref("scala", "EmptyTuple");
        for len in opens.into_iter().rev() {
            self.buf.end_length(len);
        }
    }

    /// The `ClassTag[T]` scalac's implicit search finds: the evidence parameter of a type
    /// parameter in scope, `ClassTag.apply[T](classOf[T])` of a class, `.wrap` of the element's
    /// for an array.
    fn class_tag(&mut self, t: TypeId) {
        let t = self.w.zonk(t);
        let t = self.w.deref_alias(t);
        let tag_class = self.std_class_named("scala.reflect.ClassTag");
        if let Type::Class(c, args) = self.w.types.get(t) {
            if c == self.w.b.array {
                let Some(&elem) = self.w.types.items(args).first() else { return self.fail("an array's element type".to_string()) };
                self.term_at(None);
                self.buf.byte(SELECT);
                let n = self.names.simple("wrap");
                self.buf.nat(n as u64);
                return self.class_tag(elem);
            }
        }
        match self.w.types.get(t) {
            Type::Param(_) => {
                let evidence = self.params.iter().rev().map(|&(q, at)| (q, at)).find(|&(q, _)| {
                    let pt = self.w.sig_of(q).ret;
                    matches!(self.w.types.get(pt), Type::Class(k, args) if Some(k) == tag_class && self.w.types.items(args).first() == Some(&t))
                });
                match evidence {
                    Some((_, at)) => {
                        self.term_at(None);
                        self.buf.byte(TERMREFDIRECT);
                        self.buf.reference(at);
                    }
                    None => self.fail("the ClassTag of a type parameter of no evidence in scope".to_string()),
                }
            }
            Type::Class(c, _) if !matches!(self.w.interner.get(self.w.syms.class(c).name), "Any" | "AnyVal" | "Nothing" | "Null" | "Matchable" | "Singleton") => {
                self.term_at(None);
                let l = self.open(APPLY);
                let ta = self.open(TYPEAPPLY);
                let sl = self.open(SELECTIN);
                let n = self.names.signed("apply", None, &[SigParam::Types(1), SigParam::Type("java.lang.Class".into())], "scala.reflect.ClassTag");
                self.buf.nat(n as u64);
                self.object_path("scala.reflect", "ClassTag");
                self.object_class_ref("scala.reflect", "ClassTag");
                self.buf.end_length(sl);
                self.tpt(t);
                self.buf.end_length(ta);
                self.term_at(None);
                self.buf.byte(CLASSCONST);
                let k = self.class_applied_wild(c);
                self.ty(k);
                self.buf.end_length(l);
            }
            _ => self.fail("the ClassTag of a type of no class".to_string()),
        }
    }

    /// `js.Dynamic.literal(k = v, ..)` as Scala.js's `literal` object takes it, applied
    /// dynamically: `literal.applyDynamicNamed("apply")(("k", v)*)`, each value as a `js.Any`.
    fn dynamic_literal(&mut self, items: &[TExprId]) {
        if items.len() % 2 != 0 {
            return self.fail("a literal's fields".to_string());
        }
        let string = self.w.b.t_string;
        // `(String, js.Any)`, scalajs-library's trait `js.Any` where the lean std has an alias.
        let pair = |p: &mut Self| {
            p.mark_tree();
            p.buf.byte(APPLIEDTYPE);
            let l = p.buf.begin_length();
            p.external_typeref("scala", "Tuple2");
            p.ty(string);
            p.external_typeref("scala.scalajs.js", "Any");
            p.buf.end_length(l);
        };
        let outer = self.open(APPLY);
        self.term_at(None);
        let inner = self.open(APPLY);
        let sl = self.open(SELECTIN);
        let n = self.names.signed("applyDynamicNamed", None, &[SigParam::Type("java.lang.String".into()), SigParam::Type("scala.collection.immutable.Seq".into())], "scala.scalajs.js.Object");
        self.buf.nat(n as u64);
        self.term_at(None);
        self.buf.byte(SELECT);
        let lit = self.names.simple("literal");
        self.buf.nat(lit as u64);
        self.term_at(None);
        self.buf.byte(SELECT);
        let dynamic = self.names.simple("Dynamic");
        self.buf.nat(dynamic as u64);
        self.term_at(None);
        self.package_path("scala.scalajs.js");
        self.buf.byte(TYPEREF);
        let lc = self.names.object_class("literal");
        self.buf.nat(lc as u64);
        self.buf.byte(THIS);
        self.object_class_ref("scala.scalajs.js", "Dynamic");
        self.buf.end_length(sl);
        self.term_at(None);
        self.buf.byte(STRINGCONST);
        let apply = self.names.simple("apply");
        self.buf.nat(apply as u64);
        self.buf.end_length(inner);
        self.term_at(None);
        let typed = self.open(TYPED);
        self.term_at(None);
        let rep = self.open(REPEATED);
        pair(self);
        for kv in items.chunks(2) {
            // Tuple2.apply[String, js.Any](k, v)
            self.term_at(Some(kv[1]));
            let a = self.open(APPLY);
            let ta = self.open(TYPEAPPLY);
            let s2 = self.open(SELECTIN);
            let n = self.names.signed("apply", None, &[SigParam::Types(2), SigParam::Type("java.lang.Object".into()), SigParam::Type("java.lang.Object".into())], "scala.Tuple2");
            self.buf.nat(n as u64);
            self.object_path("scala", "Tuple2");
            self.object_class_ref("scala", "Tuple2");
            self.buf.end_length(s2);
            self.tpt(string);
            self.mark_tree();
            self.external_typeref("scala.scalajs.js", "Any");
            self.buf.end_length(ta);
            self.term(kv[0]);
            self.js_any_value(kv[1]);
            self.buf.end_length(a);
        }
        self.buf.end_length(rep);
        self.term_at(None);
        self.buf.byte(APPLIEDTYPE);
        let l = self.buf.begin_length();
        self.external_typeref("scala", "<repeated>");
        pair(self);
        self.buf.end_length(l);
        self.buf.end_length(typed);
        self.buf.end_length(outer);
    }

    /// A value where a `js.Any` is expected, as Scala.js's implicit conversions make it one: a
    /// primitive or a string through `js.Any.fromX`, a JavaScript value as it is.
    fn js_any_value(&mut self, v: TExprId) {
        let Some(t) = self.node_type(v) else { return self.fail("a literal's value of no type".to_string()) };
        let t = self.w.widen_lit(t);
        let kind = if t == self.w.b.t_string {
            Some("String")
        } else if let Some(k) = self.numeric_class(t) {
            let name = self.name(self.w.syms.class(k).name);
            match name.as_str() {
                "Int" | "Double" | "Boolean" | "Float" | "Short" | "Byte" | "Long" => Some(match name.as_str() {
                    "Int" => "Int",
                    "Double" => "Double",
                    "Boolean" => "Boolean",
                    "Float" => "Float",
                    "Short" => "Short",
                    "Byte" => "Byte",
                    _ => "Long",
                }),
                _ => return self.fail(format!("a literal's value of {}", name)),
            }
        } else if t == ANY {
            // `Any` is the lean std's `js.Any`, the cast's target scalajs-library's.
            self.term_at(None);
            let l = self.open(TYPEAPPLY);
            let sl = self.open(SELECTIN);
            let n = self.names.signed("asInstanceOf", None, &[SigParam::Types(1)], "java.lang.Object");
            self.buf.nat(n as u64);
            self.term(v);
            self.external_typeref("scala", "Any");
            self.buf.end_length(sl);
            self.mark_tree();
            self.external_typeref("scala.scalajs.js", "Any");
            self.buf.end_length(l);
            return;
        } else if self.is_js_value_type(t) {
            None
        } else {
            return self.fail("a literal's value of a type no js.Any conversion takes".to_string());
        };
        let Some(kind) = kind else { return self.term(v) };
        self.term_at(None);
        let l = self.open(APPLY);
        self.buf.byte(TERMREF);
        let param = if kind == "String" { "java.lang.String".to_string() } else { format!("scala.{}", kind) };
        let n = self.names.signed(&format!("from{}", kind), None, &[SigParam::Type(param)], "scala.scalajs.js.Any");
        self.buf.nat(n as u64);
        self.buf.byte(TERMREF);
        let any = self.names.simple("Any");
        self.buf.nat(any as u64);
        self.package_path("scala.scalajs.js");
        self.term(v);
        self.buf.end_length(l);
    }

    /// Whether a value of the type is a `js.Any` as scalajs-library types it: `js.Any` itself, a
    /// class of `scala.scalajs.js` or one deriving `js.Object`.
    fn is_js_value_type(&mut self, t: TypeId) -> bool {
        if let Type::Alias(a, _) = self.w.types.get(t) {
            let info = self.w.syms.alias(a);
            if self.w.interner.get(info.name) == "Any" && self.is_std_file(info.file) {
                return true;
            }
        }
        let t = self.w.deref_alias(t);
        let Type::Class(c, _) = self.w.types.get(t) else { return false };
        let js_object = self.std_class_named("scala.scalajs.js.Object");
        let in_js = scala_name(self.w, c).starts_with("scala.scalajs.js.");
        self.w.complete_class(c);
        in_js || js_object.map_or(false, |o| self.w.syms.class(c).base_types.iter().any(|&(b, _)| b == o))
    }

    /// `x.ordinal` of a value of the enum `e`: `scala.reflect.Enum`'s parameterless member, or
    /// `java.lang.Enum`'s `ordinal()` for an enum over it.
    pub(super) fn ordinal_of(&mut self, e: ClassId, recv: impl FnOnce(&mut Self)) {
        let java_enum = self.is_java_enum(e);
        self.term_at(None);
        if !java_enum {
            self.buf.byte(SELECT);
            let k = self.names.simple("ordinal");
            self.buf.nat(k as u64);
            return recv(self);
        }
        let l = self.open(APPLY);
        let sl = self.open(SELECTIN);
        let k = self.names.signed("ordinal", None, &[], "scala.Int");
        self.buf.nat(k as u64);
        recv(self);
        self.external_typeref("java.lang", "Enum");
        self.buf.end_length(sl);
        self.buf.end_length(l);
    }

    /// The type the typer gave a dynamic call of a `Selectable` that is not its declared
    /// result: a structural member's.
    fn structural_result(&mut self, s: SymId, e: TExprId) -> Option<TypeId> {
        let Owner::Class(c) = self.w.syms.sym(s).owner else { return None };
        let selectable = self.w.b.selectable?;
        self.w.complete_class(c);
        if c != selectable && !self.w.syms.class(c).base_types.iter().any(|&(b, _)| b == selectable) {
            return None;
        }
        let t = self.node_type(e)?;
        let t = self.w.zonk(t);
        let declared = self.w.sig_of(s).ret;
        (t != declared && t != ANY).then_some(t)
    }

    /// `e.$asInstanceOf$[T]`, the cast scalac's typer inserts.
    fn dollar_cast(&mut self, t: TypeId, e: impl FnOnce(&mut Self)) {
        self.term_at(None);
        let l = self.open(TYPEAPPLY);
        let sl = self.open(SELECTIN);
        let n = self.names.signed("$asInstanceOf$", None, &[SigParam::Types(1)], "java.lang.Object");
        self.buf.nat(n as u64);
        e(self);
        self.external_typeref("scala", "Any");
        self.buf.end_length(sl);
        self.tpt(t);
        self.buf.end_length(l);
    }

    pub(super) fn is_java_enum(&mut self, e: ClassId) -> bool {
        self.w.complete_class(e);
        self.w.syms.class(e).base_types.iter().any(|&(b, _)| scala_name(self.w, b) == "java.lang.Enum")
    }

    /// The kind of an array's element type: a primitive, a reference type `refArrayOps` takes
    /// (a class or trait of no `AnyVal`, `String` and an array among them), any other.
    pub(super) fn array_elem(&mut self, t: TypeId) -> ArrayElem {
        let t = self.w.deref_alias(t);
        let t = self.w.widen_lit(t);
        if let Some(k) = self.numeric_class(t) {
            if let Some(&p) = ["Int", "Long", "Double", "Float", "Char", "Byte", "Short"].iter().find(|&&p| self.name(self.w.syms.class(k).name) == p) {
                return ArrayElem::Prim(p);
            }
        }
        if t == self.w.b.t_boolean {
            return ArrayElem::Prim("Boolean");
        }
        if self.is_unit(t) {
            return ArrayElem::Prim("Unit");
        }
        match self.w.types.get(t) {
            Type::Class(c, _) => {
                let info = self.w.syms.class(c);
                let top = matches!(self.w.interner.get(info.name), "Any" | "AnyVal" | "Matchable" | "Singleton") && self.is_std_class(c);
                let reference = matches!(self.w.interner.get(info.name), "String" | "Array") && self.is_std_class(c);
                if info.value_class || top || info.kind == ClassKind::Opaque || info.kind == ClassKind::Builtin && !reference {
                    ArrayElem::Generic
                } else {
                    ArrayElem::Ref
                }
            }
            _ => ArrayElem::Generic,
        }
    }

    /// The argument clauses of a std extension's own part as the library member takes them: a
    /// Java method takes one where the std's def takes none.
    pub(super) fn std_own_clauses(&mut self, s: SymId, target: StdTarget) -> Vec<usize> {
        let info = self.sym_info(s);
        let sig = self.w.sig_of(s).clone();
        let ec = (info.ext_clauses as usize).min(sig.clauses.len());
        let own: Vec<usize> = sig.clauses[ec..].iter().map(|c| c.params.len()).collect();
        let java = matches!(target, StdTarget::Receiver { java: true, .. });
        if java && own.is_empty() { vec![0] } else { own }
    }

    /// A member of `Predef`, checked.
    pub(super) fn predef_shape(&mut self, name: &str, params: &[SigParam], result: &str) -> bool {
        self.std_shape("scala.Predef$", name, &super::shapes::sig_text(params, result))
    }

    /// The signature of a std definition as scala-library's overload for the call: its own
    /// parameters (`skip` of them the receiver's) erased as declared; a method whose type
    /// parameters are bounded by unions stands for overloads without type parameters
    /// (`math.min[T <: Int | Long ..]` for `min(Int, Int)`), instantiated by the call's type
    /// arguments, and a parameter of a union type takes its argument's. Returns the signature,
    /// the type arguments the call keeps, the result.
    fn std_signature(&mut self, s: SymId, targs: &[TypeId], args: &[TExprId], skip: (usize, usize)) -> Option<(Vec<SigParam>, Vec<TypeId>, String)> {
        let skip_t = skip.0.min(targs.len());
        let arg_types: Vec<Option<TypeId>> = args.iter().map(|&a| self.node_type(a).map(|t| self.w.widen_lit(t))).collect();
        let (params, keeps, result) = self.std_signature_at(s, skip, &targs[skip_t..], &arg_types)?;
        let kept = if keeps { targs[skip_t..].to_vec() } else { Vec::new() };
        Some((params, kept, result))
    }

    /// `std_signature` for the instantiation `inst` of the own type parameters and the types of
    /// the arguments; whether the call keeps its type arguments.
    pub(super) fn std_signature_at(&mut self, s: SymId, skip: (usize, usize), inst: &[TypeId], arg_types: &[Option<TypeId>]) -> Option<(Vec<SigParam>, bool, String)> {
        let sig = self.w.sig_of(s).clone();
        let (skip_t, skip_c) = (skip.0.min(sig.tparams.len()), skip.1.min(sig.clauses.len()));
        let own_tparams: Vec<TParamId> = sig.tparams[skip_t..].to_vec();
        let overloads = !own_tparams.is_empty() && own_tparams.iter().all(|&tp| {
            let u = self.w.syms.tparam(tp).upper;
            matches!(self.w.types.get(u), Type::Union(..))
        });
        let subst: Subst = if overloads { own_tparams.iter().copied().zip(inst.iter().copied()).collect() } else { Subst::default() };
        let local = self.local_prefix();
        let mut params = Vec::new();
        if !overloads && !own_tparams.is_empty() {
            params.push(SigParam::Types(own_tparams.len()));
        }
        let mut at: usize = sig.clauses[..skip_c].iter().map(|c| c.params.len()).sum();
        for clause in &sig.clauses[skip_c..] {
            for p in &clause.params {
                let erased = if p.by_name {
                    "scala.Function0".to_string()
                } else if p.repeated {
                    "scala.collection.immutable.Seq".to_string()
                } else {
                    let mut t = self.w.types.subst(p.ty, &subst);
                    if matches!(self.w.types.get(t), Type::Union(..)) {
                        t = arg_types.get(at).copied().flatten()?;
                    }
                    self.w.library_sig_name(t, &local, false)?
                };
                params.push(SigParam::Type(erased));
                at += 1;
            }
        }
        let ret = self.w.types.subst(sig.ret, &subst);
        let result = self.w.library_sig_name(ret, &local, true)?;
        Some((params, !overloads, result))
    }

    /// `name[targs](args)` of clauses of `clauses` arguments, selected with this signature on the
    /// qualifier `qual` writes, declared by the class `owner` writes, which `owner_key` names.
    #[allow(clippy::too_many_arguments)]
    fn std_applied(&mut self, owner_key: &str, name: &str, params: &[SigParam], result: &str, clauses: &[usize], targs: &[TypeId], args: &[TExprId], qual: impl FnOnce(&mut Self), owner: impl FnOnce(&mut Self)) {
        self.std_applied_as(owner_key, name, params, result, clauses, targs, args, false, qual, owner)
    }

    /// The selection about to be written in the call at `call_at`, whose one empty clause the
    /// source did not write: it spans the call, which tells the reader so.
    fn mark_auto_applied(&mut self, call_at: usize) {
        let Some(&call) = self.positions.last().filter(|p| p.addr == call_at && p.point.is_some()) else { return };
        let unparenthesised = match self.is_pickled_source(call.file) {
            // A converted call: as the pickle it was converted from has it.
            true => self.placed_tree.filter(|&(at, _)| at == call_at).map_or(false, |(_, (f, a))| self.w.pickled_function_placed(f, a)),
            false => {
                let text = self.w.files.as_slice()[call.file.0 as usize].text.as_bytes();
                call.span.end > 0 && text.get(call.span.end as usize - 1) != Some(&b')')
            }
        };
        if unparenthesised {
            self.positions.push(Pos { addr: self.buf.addr(), point: None, def: false, switch: false, ..call });
        }
    }

    /// `std_applied`, where `auto_applied` says that its one empty clause is Java's `()` for a
    /// std member without one: where the source's call does not end in it either, the selection
    /// spans the call, as scalac's does, which tells the reader so.
    #[allow(clippy::too_many_arguments)]
    fn std_applied_as(
        &mut self,
        owner_key: &str,
        name: &str,
        params: &[SigParam],
        result: &str,
        clauses: &[usize],
        targs: &[TypeId],
        args: &[TExprId],
        auto_applied: bool,
        qual: impl FnOnce(&mut Self),
        owner: impl FnOnce(&mut Self),
    ) {
        if clauses.iter().sum::<usize>() != args.len() {
            return self.fail(format!("a call of the std's {} with {} arguments", name, args.len()));
        }
        let by_name = params.is_empty() && clauses.is_empty() && targs.is_empty();
        let shape = if by_name { super::shapes::by_name_shape(result) } else { super::shapes::sig_text(params, result) };
        // A key the inverse table remaps: scala-library's signature, the arguments as its form
        // has them.
        let remapped = (self.shapes_seen.is_none() && !by_name)
            .then(|| super::shapes::inverse_entry(owner_key, name, &shape))
            .flatten()
            .filter(|e| e.via.is_none() && e.appended.is_none() && e.library.1 == name && !e.written_alike());
        let library = remapped.and_then(|e| sig_of_text(e.library.2));
        let (params, result) = match &library {
            Some((p, r)) => (&p[..], &r[..]),
            None if remapped.is_some() => return self.fail(format!("the std's {}, which scala-library has under another shape", name)),
            None if !self.std_shape(owner_key, name, &shape) => return,
            None => (params, result),
        };
        let widened: Vec<bool> = match remapped.filter(|e| e.widened) {
            Some(_) => {
                let terms: Vec<&SigParam> = params.iter().filter(|p| matches!(p, SigParam::Type(_))).collect();
                args.iter().enumerate().map(|(i, &a)| matches!(terms.get(i), Some(SigParam::Type(t)) if t == "scala.Int") && self.is_char_valued(a)).collect()
            }
            None => vec![false; args.len()],
        };
        let call_at = self.buf.addr();
        let opens: Vec<crate::tasty::write::buf::Slot> = clauses.iter().map(|_| self.open(APPLY)).collect();
        let ta = (!targs.is_empty()).then(|| self.open(TYPEAPPLY));
        if auto_applied && ta.is_none() {
            self.mark_auto_applied(call_at);
        }
        if by_name {
            self.buf.byte(SELECT);
            let n = self.names.simple(name);
            self.buf.nat(n as u64);
            qual(self);
        } else {
            let sl = self.open(SELECTIN);
            let n = self.names.signed(name, None, params, result);
            self.buf.nat(n as u64);
            qual(self);
            owner(self);
            self.buf.end_length(sl);
        }
        if let Some(l) = ta {
            for &t in targs {
                self.tpt(t);
            }
            self.buf.end_length(l);
        }
        let mut at = 0;
        for (i, &k) in clauses.iter().enumerate() {
            for (j, &a) in args[at..at + k].iter().enumerate() {
                match (self.w.prog.expr(a), self.records(a).form) {
                    _ if widened[at + j] => self.char_widened(a),
                    (TExpr::Lambda(ps, body), Some(Form::ByName)) if ps.len == 0 => self.term(body),
                    _ => self.term(a),
                }
            }
            at += k;
            self.buf.end_length(opens[clauses.len() - 1 - i]);
        }
    }

    /// Whether an argument is a `Char`: of that class, or of a type conforming to it (a parameter
    /// bounded by `Char`, a union of characters).
    fn is_char_valued(&mut self, a: TExprId) -> bool {
        let Some(t) = self.node_type(a) else { return false };
        let char = self.w.b.t_char;
        let mark = self.w.snapshot();
        let sub = self.w.is_sub(t, char);
        self.w.rollback(mark);
        sub
    }

    /// A `Char` argument adapted to an `Int` parameter: `Char.char2int(c)`, as scalac's typer
    /// adapts it, a literal too, which scalac folds to its code, and which teq's reader reads back
    /// as the `Char` it was by that call (`product_widened`).
    fn char_widened(&mut self, a: TExprId) {
        let (char, int) = (self.w.b.char, self.w.b.int);
        self.within(Some(a), |p| p.widen_call(char, int, |p| p.term(a)));
    }

    /// The receiver `r` of the member `s` of the selection `e`, where a member of a wider
    /// scala-library result may be selected when `s` is selected by name (a `SELECT` scalac
    /// resolves on the receiver's own type, not at a signature declared for the lean receiver's
    /// class) and scala-library's `s` on the wider class returns the class teq's selection does:
    /// `m.values.toList` a `List` on both sides, where `m.values.tail` is scala-library's
    /// `Iterable` and no binding or selection of the lean `List` would take it.
    fn receive_by_name(&mut self, e: TExprId, r: TExprId, s: SymId) {
        if !matches!(self.member_signature(s), Ok(None)) {
            return;
        }
        if let Some(wider) = self.wider_receiver(r) {
            let name = self.name(self.w.syms.sym(s).name);
            let local = self.local_prefix();
            let lean = self.node_type(e).and_then(|t| self.w.library_sig_name(t, &local, true));
            if lean.is_none() || super::shapes::library_result_by_name(wider, &name) != lean.as_deref() {
                return;
            }
        }
        self.receivers.insert(r, ());
    }

    /// The class scala-library's result of the selection `r` is, where it is wider than the lean
    /// std's (`Map.values`'s `Iterable`).
    fn wider_receiver(&mut self, r: TExprId) -> Option<&'static str> {
        match self.w.prog.expr(r) {
            TExpr::Field(q, s) => self.wider_result(s, q),
            TExpr::CallMethod(q, s, args) if self.w.prog.expr_list(args).is_empty() => self.wider_result(s, q),
            _ => None,
        }
    }

    /// Whether the std member `s` has an entry whose scala-library result is wider than the lean
    /// std's (`Map.values` an `Iterable`, `sizeIs` a `SizeCompareOps`).
    fn wider_than_lean(&mut self, s: SymId, r: TExprId) -> bool {
        self.wider_entry(s, r).is_some()
    }

    /// The class of `wider_entry`'s scala-library result.
    fn wider_result(&mut self, s: SymId, r: TExprId) -> Option<&'static str> {
        self.wider_entry(s, r)?.library.2.rsplit_once(':').map(|(_, c)| c)
    }

    fn wider_entry(&mut self, s: SymId, r: TExprId) -> Option<&'static super::shapes::Inverse> {
        if !self.is_std_sym(s) || !super::shapes::may_be_receiver_only(&self.name(self.w.syms.sym(s).name)) {
            return None;
        }
        let receiver = self.receiver_class(r);
        let sig = self.member_signature_on(s, receiver).ok()?;
        let sig = self.with_erased_tags(s, sig);
        self.std_inverse(s, &sig, receiver).filter(|e| e.receiver_only())
    }

    /// The class the inverse table says scala-library's member, selected by `a`, returns, whose
    /// comparisons a comparison of `a`'s value selects: `IterableOps.SizeCompareOps` of `sizeIs`.
    fn compared_class(&mut self, a: TExprId) -> Option<&'static str> {
        let (s, recv) = match self.w.prog.expr(a) {
            TExpr::CallMethod(r, s, args) if self.w.prog.expr_list(args).is_empty() => (s, r),
            TExpr::Field(r, s) => (s, r),
            _ => return None,
        };
        if !self.is_std_sym(s) {
            return None;
        }
        let receiver = self.receiver_class(recv);
        let sig = self.member_signature_on(s, receiver).ok()?;
        let sig = self.with_erased_tags(s, sig);
        self.std_inverse(s, &sig, receiver)?.compared
    }

    /// `a op b` of a comparison with an `Int`, selected on the class scala-library's member
    /// returns (`l.sizeIs > 1`).
    fn compared_op(&mut self, class: &str, name: &str, a: TExprId, b: TExprId) {
        if !matches!(name, "<" | "<=" | ">" | ">=" | "==" | "!=") || !self.operand_type(b).map_or(false, |t| t == self.w.b.t_int) {
            return self.fail(format!("the comparison {} of a member scala-library has under another shape", name));
        }
        // The package, and the class with the objects it is nested in (`IterableOps$.SizeCompareOps`).
        let segments: Vec<&str> = class.split('.').collect();
        let at = segments.iter().position(|s| s.ends_with('$')).unwrap_or(segments.len() - 1);
        let (pkg, cls) = (segments[..at].join("."), segments[at..].join("."));
        let l = self.open(APPLY);
        let sl = self.open(SELECTIN);
        let n = self.names.signed(name, None, &[SigParam::Type("scala.Int".to_string())], "scala.Boolean");
        self.buf.nat(n as u64);
        self.term(a);
        self.library_class_ref(&pkg, &cls);
        self.buf.end_length(sl);
        self.term(b);
        self.buf.end_length(l);
    }

    /// An object of scala-library as a path: `Predef`, `scala.math.package`.
    pub(super) fn object_path(&mut self, pkg: &str, object: &str) {
        self.term_at(None);
        self.buf.byte(TERMREF);
        let n = self.names.simple(object);
        self.buf.nat(n as u64);
        self.package_path(pkg);
    }

    /// The class of an object of scala-library: `Predef$`.
    pub(super) fn object_class_ref(&mut self, pkg: &str, object: &str) {
        self.buf.byte(TYPEREF);
        let n = self.names.object_class(object);
        self.buf.nat(n as u64);
        self.package_path(pkg);
    }

    /// A class of scala-library by its package (or object) and name.
    fn library_class_ref(&mut self, pkg: &str, class: &str) {
        // A class of an object: `ArraySeq$.ofRef`.
        if let Some((object, inner)) = class.split_once("$.") {
            self.buf.byte(TYPEREF);
            let n = self.names.simple(inner);
            self.buf.nat(n as u64);
            self.buf.byte(TERMREF);
            let o = self.names.simple(object);
            self.buf.nat(o as u64);
            return self.package_path(pkg);
        }
        match pkg.strip_prefix("scala.").filter(|_| pkg == "scala.Predef") {
            Some(_) => {
                self.buf.byte(TYPEREF);
                let n = self.names.simple(class);
                self.buf.nat(n as u64);
                self.buf.byte(TERMREF);
                let o = self.names.simple("Predef");
                self.buf.nat(o as u64);
                self.package_path("scala");
            }
            None => self.external_typeref(pkg, class),
        }
    }

    /// A definition of the lean std that stands for scala-library's in another shape: a
    /// top-level definition or an extension of a std file (`println` for `Predef.println`,
    /// `s.length` for `String.length()`, `s * n` for `augmentString(s) * n`).
    pub(super) fn std_helper(&self, s: SymId) -> Option<String> {
        let info = self.w.syms.sym(s);
        let src = self.w.files.as_slice().get(info.file.0 as usize)?;
        if !src.is_std || self.w.in_jar(info.file) {
            return None;
        }
        let holder = match info.owner {
            Owner::Package(_) => true,
            Owner::Class(c) => self.w.interner.get(self.w.syms.class(c).name).ends_with("$package"),
            Owner::Local => false,
        };
        holder.then(|| format!("{}.{}", super::super::file_stem(&src.path), self.w.interner.get(info.name)))
    }

    /// `@targetName`'s name of a definition, where it has one.
    pub(super) fn target_name(&self, s: SymId) -> Option<String> {
        // A definition read from TASTy keeps the name its pickle gave it.
        if let Some(t) = self.w.loaded.as_ref().and_then(|l| l.target_names.get(&s).copied().or_else(|| l.product_target_names.get(&s).copied())) {
            return Some(self.w.interner.get(t).to_string());
        }
        let info = self.w.syms.sym(s);
        let d = info.def?;
        let ast = self.w.ast(info.file);
        let def = ast.def(d);
        def.annots.iter().find(|a| self.w.interner.get(a.name) == "targetName").and_then(|a| ast.annot_args(a).first().map(|&arg| ast.str(arg).to_string()))
    }

    /// The class a local class of the body being written is named under in a signature.
    pub(super) fn local_prefix(&mut self) -> String {
        match self.enclosing.last().copied() {
            Some(c) => self.w.library_class_name(c, ""),
            None => String::new(),
        }
    }

    /// `f.apply(args)` of a function value.
    /// `f.apply(args)`: the `apply` the function's type has, a context function's or one a
    /// class of functions overrides, as the typer of scalac selects it.
    fn apply_function(&mut self, f: TExprId, args: ListRef, e: TExprId) {
        let args = self.w.prog.expr_list(args).to_vec();
        if let Some(ft) = self.term_type(f) {
            let ft = self.w.deref_alias(ft);
            if let Type::Poly(ps, fun) = self.w.types.get(ft) {
                return self.poly_function_apply(f, ps, fun, &args, e);
            }
        }
        self.function_apply(f, &args)
    }

    /// `f[T..](args)` of a polymorphic function value, as scalac writes it: the `apply` of the
    /// `PolyFunction` refinement, selected by its signature, applied to the type arguments the
    /// typer took (a contextual one's arguments are its using clause's).
    fn poly_function_apply(&mut self, f: TExprId, ps: TList, fun: TypeId, args: &[TExprId], e: TExprId) {
        let ps: Vec<TypeId> = self.w.types.poly_params(ps).to_vec();
        let targs: Vec<TypeId> = self.records(e).targs.map(|l| self.w.types.items(l).to_vec()).unwrap_or_default();
        if targs.len() != ps.len() {
            return self.fail("a polymorphic function's application without its type arguments".to_string());
        }
        let Some((params, ret)) = self.w.as_function(fun).or_else(|| self.w.as_context_function(fun)) else {
            return self.fail("a polymorphic function of no function type".to_string());
        };
        if params.len() != args.len() {
            return self.fail("a polymorphic function's application of another arity".to_string());
        }
        let mut sig = vec![SigParam::Types(ps.len())];
        for &p in &params {
            sig.push(SigParam::Type(self.result_erasure(p)));
        }
        let result = self.result_erasure(ret);
        let a = self.open(APPLY);
        self.term_at(None);
        let ta = self.open(TYPEAPPLY);
        self.term_at(None);
        self.buf.byte(SELECT);
        let n = self.names.signed("apply", None, &sig, &result);
        self.buf.nat(n as u64);
        self.term(f);
        for &t in &targs {
            self.tpt(t);
        }
        self.buf.end_length(ta);
        for &x in args {
            self.term(x);
        }
        self.buf.end_length(a);
    }

    fn function_apply(&mut self, f: TExprId, args: &[TExprId]) {
        let n = args.len();
        // A call's node may hold no type: the function is its callee's result.
        let ft = self.term_type(f).or_else(|| match self.w.prog.expr(f) {
            TExpr::CallStatic(s, _) | TExpr::CallMethod(_, s, _) => Some(self.w.sig_of(s).ret),
            _ => None,
        });
        let context = ft.map_or(false, |t| self.w.as_context_function(t).is_some());
        let fc = if context { self.w.context_function_class(n) } else { self.w.function_class(n) };
        let own = ft.and_then(|t| {
            let t = self.w.deref_alias(t);
            let apply = self.w.interner.intern("apply");
            self.w.find_member(t, apply).map(|(s, _)| s)
        });
        let own = own.filter(|&s| self.w.syms.alternatives(s).is_none() && self.w.sig_of(s).clauses.first().map_or(false, |c| c.params.len() == n));
        // A context function's is `ContextFunctionN.apply`, which scalac selects, not the
        // `FunctionN.apply` the receiver's type may find first.
        let apply = if context { self.member_of_class(fc, "apply").or(own) } else { own.or_else(|| self.member_of_class(fc, "apply")) };
        match apply {
            Some(s) => self.call_with(s, |p| p.term(f), None, args, None),
            None => self.fail("Function.apply".to_string()),
        }
    }

    // ---- instances ------------------------------------------------------------------------

    /// `new C[T..](args)`: the constructor selected on `NEW` of the class's type.
    /// `C.apply[T..](args)` of a case class the source applies by its name: the companion's
    /// `apply` of the constructor's one clause; `new` where a default or a repeated parameter
    /// would take the apply's own shape.
    pub(super) fn case_apply(&mut self, c: ClassId, targs: Option<TList>, args: &[TExprId], e: TExprId) {
        self.w.complete_class(c);
        let ctor = self.w.syms.class(c).ctor.clone();
        let plain = ctor.len() == 1 && ctor[0].params.len() == args.len() && ctor[0].params.iter().all(|p| !p.repeated && !p.by_name)
            && args.iter().all(|&a| !matches!(self.records(a).form, Some(Form::Default)));
        // A local case class outside a quote: its construction, which is what scalac's local
        // companion's `apply` makes and which nothing but a quote's reflection tells from it; the
        // writer does not write the local companion.
        let local = self.w.syms.class(c).owner == Owner::Local && !self.is_local(Key::SynthModule(c)) && self.quoted_depth == 0;
        if !plain || local {
            return self.new_instance(c, None, targs, args, e);
        }
        let targs: Vec<TypeId> = targs.map(|l| self.w.types.items(l).to_vec()).unwrap_or_default();
        let (owner, sig, result) = self.case_apply_key(c);
        if self.is_std_class(c) && !self.std_shape(&owner, "apply", &super::shapes::sig_text(&sig, &result)) {
            return;
        }
        let a = self.open(APPLY);
        self.term_at(None);
        let ta = (!targs.is_empty()).then(|| self.open(TYPEAPPLY));
        if ta.is_some() {
            self.term_at(None);
        }
        let s = self.open(SELECTIN);
        let n = self.names.signed("apply", None, &sig, &result);
        self.buf.nat(n as u64);
        self.companion_path(c);
        self.companion_class_ref(c);
        self.buf.end_length(s);
        if let Some(ta) = ta {
            for &t in &targs {
                self.tpt(t);
            }
            self.buf.end_length(ta);
        }
        for &arg in args {
            self.term(arg);
        }
        self.buf.end_length(a);
    }

    /// A Java annotation's tree: its interface's unsigned `<init>` on `NEW`, applied to the
    /// elements given, each a `NAMEDARG`, as scalac's typer leaves a Java annotation's
    /// constructor call (`Applications.isJavaAnnotConstr`).
    fn java_annotation(&mut self, c: ClassId, args: &[TExprId]) {
        let app = self.open(APPLY);
        self.buf.byte(SELECT);
        let n = self.names.simple("<init>");
        self.buf.nat(n as u64);
        self.term_at(None);
        self.buf.byte(NEW);
        let ty = self.w.types.class(c, &[]);
        self.tpt(ty);
        for &a in args {
            self.term(a);
        }
        self.buf.end_length(app);
    }

    fn new_instance(&mut self, c: ClassId, via: Option<SymId>, targs: Option<TList>, args: &[TExprId], e: TExprId) {
        let info = self.class_info(c);
        if info.kind == ClassKind::Anon {
            return self.anon_instance(c, args, e);
        }
        let targs: Vec<TypeId> = targs.map(|l| self.w.types.items(l).to_vec()).unwrap_or_default();
        let ty = self.w.types.class(c, &targs);
        self.ctor_call(c, via, &targs, args, ty, None);
    }

    /// The call of a constructor of `c`, `via` a secondary one, on `NEW` of `new_ty`, the
    /// arguments after those a local class's captures take; `prelude` the temporaries of named
    /// arguments, before the call in a block.
    fn ctor_call(&mut self, c: ClassId, via: Option<SymId>, targs: &[TypeId], args: &[TExprId], new_ty: TypeId, prelude: Option<ListRef>) {
        let own = self.w.syms.class(c).own_tparams().len();
        if targs.len() != own && c != self.w.b.any_ref {
            return self.fail(format!("a constructor call of {} with {} of its {} type arguments", self.name(self.w.syms.class(c).name), targs.len(), own));
        }
        let ctor: Vec<ClauseSig> = match via {
            Some(s) => self.w.sig_of(s).clauses.clone(),
            None => {
                self.w.complete_class(c);
                self.w.syms.class(c).ctor.clone()
            }
        };
        let total: usize = ctor.iter().map(|c| c.params.len()).sum();
        let skip = args.len().saturating_sub(total);
        // The enclosing instance of a class of a class: the prefix of the instance's type, as
        // `ExplicitOuter` takes it (`new o.I` is `new (o.type)#I`).
        let outer = match self.w.syms.class(c).owner {
            Owner::Class(o) if skip >= 1 && self.w.syms.class(o).kind != ClassKind::Object => {
                // The enclosing instance itself: `this`, or its path from a class nested in it
                // (`Outer.this` reached from `object Helper`), whose type is `O.this`.
                let this = matches!(self.w.prog.expr(args[0]), TExpr::This) || self.term_type(args[0]).map_or(false, |t| matches!(self.w.types.get(t), Type::This(k) if k == o));
                // Inside the outer class teq's types name the instance `O.this.I` whatever its
                // outer instance: one of another instance there is not stated.
                if !this && self.enclosing.contains(&o) {
                    return self.fail("an instance of a class of another instance of its enclosing class".to_string());
                }
                (!this && self.is_simple_path(args[0])).then_some(args[0])
            }
            _ => None,
        };
        let mut args = &args[skip..];
        if args.len() != total {
            return self.fail(format!("a constructor call of {} with {} arguments for {} parameters", self.name(self.w.syms.class(c).name), args.len(), total));
        }
        // A Java class's constructor the std gives defaults stands for the JDK's overloads: the
        // one of the arguments given.
        let mut ctor = ctor;
        let java = self.is_java_class(c);
        if java && ctor.len() == 1 {
            let given = args.iter().rposition(|&a| !matches!(self.records(a).form, Some(Form::Default))).map_or(0, |i| i + 1);
            args = &args[..given];
            ctor[0].params.truncate(given);
        }
        let eliding = self.elide_args.is_some();
        let annotation = self.in_annotation;
        let block = prelude.filter(|l| l.len > 0 && !eliding).map(|l| {
            let b = self.open(BLOCK);
            (b, l)
        });
        // A default before a named argument: the arguments the call evaluates and the defaults
        // bound in a block first, as for a method's call (`applied`).
        let lens: Vec<usize> = ctor.iter().map(|c| c.params.len()).collect();
        let by_name: Vec<bool> = ctor.iter().flat_map(|c| c.params.iter().map(|p| p.by_name || p.repeated)).collect();
        if !eliding && !annotation && self.hoisted_after_default(&lens, args) {
            return self.fail("a later clause's named arguments the typer hoists before an earlier clause's default".to_string());
        }
        let binds: Vec<(TExprId, usize)> = if eliding || annotation { Vec::new() } else { self.lift_plan(&lens, args, &by_name, None).into_iter().filter_map(|(e, at)| Some((e, at?))).collect() };
        if !binds.is_empty() {
            if block.is_some() {
                return self.fail("a constructor call of named arguments out of order and after a default".to_string());
            }
            for &(e, _) in &binds {
                self.lifted_terms.insert(e, crate::tasty::write::pickle::Lifted::Pending(Vec::new()));
            }
        }
        let lifting = (!binds.is_empty()).then(|| {
            let b = self.open(BLOCK);
            self.term_at(None);
            b
        });
        let given_params = ctor.first().map_or(0, |cl| cl.params.len());
        // A constructor of using clauses only takes a trailing `()` (scalac's
        // `normalizeIfConstructor`), an empty one of none, one whose first clause is implicit
        // a leading one.
        let empty = ClauseSig { params: Vec::new(), is_using: false, is_implicit: false };
        let clauses = if ctor.is_empty() {
            vec![empty]
        } else if ctor[0].is_implicit {
            let mut c = vec![empty];
            c.extend(ctor);
            c
        } else if ctor.iter().all(|c| c.is_using) {
            let mut c = ctor;
            c.push(empty);
            c
        } else {
            ctor
        };
        let opens: Vec<crate::tasty::write::buf::Slot> = clauses.iter().map(|_| self.open(APPLY)).collect();
        let ta = (!targs.is_empty()).then(|| self.open(TYPEAPPLY));
        let local = self.local_prefix();
        let object = c == self.w.b.any_ref;
        let sig = match via {
            Some(s) => self.w.pickled_signature(s, &local),
            None if object => Some((Vec::new(), "java.lang.Object".to_string())),
            None => self.w.pickled_ctor_signature(c, &local).map(|(mut params, r)| {
                if java {
                    let n = given_params;
                    let types = params.iter().filter(|p| matches!(p, crate::typer::loader::declared::PickledSigParam::Types(_))).count();
                    params.truncate(types + n);
                }
                (params, r)
            }),
        };
        let Some((params, result)) = sig else {
            return self.fail(format!("the constructor's signature of {}", self.name(self.w.syms.class(c).name)));
        };
        let params = sig_params(params);
        if !object && self.is_std_class(c) {
            let owner = self.w.library_class_name(c, "");
            if !self.std_shape(&owner, "<init>", &super::shapes::sig_text(&params, &result)) {
                return;
            }
        }
        let sl = self.open(SELECTIN);
        let n = self.names.signed("<init>", None, &params, &result);
        self.buf.nat(n as u64);
        self.term_at(None);
        if std::mem::take(&mut self.self_init) {
            self.qual_this(c);
            self.class_typeref(c);
            self.buf.end_length(sl);
            self.ctor_args(c, via, targs, args, &clauses, ta, opens, block);
            return self.lifted_ctor_args(c, via, targs, args, &ctor_params(&clauses), binds, lifting);
        }
        self.buf.byte(NEW);
        if object {
            // A synthesized companion's parent is written `AnyRef()`, as `Desugar` makes it.
            if std::mem::take(&mut self.parent_as_anyref) {
                self.external_tpt("scala", "AnyRef");
            } else {
                self.external_tpt("java.lang", "Object");
            }
            self.external_typeref("java.lang", "Object");
        } else if let Some(o) = outer {
            self.mark_tree();
            let app = (!targs.is_empty()).then(|| {
                self.buf.byte(APPLIEDTYPE);
                self.buf.begin_length()
            });
            self.buf.byte(TYPEREF);
            let n = self.simple_name(self.w.syms.class(c).name);
            self.buf.nat(n as u64);
            self.path_type(o);
            if let Some(l) = app {
                for &t in targs {
                    self.ty(t);
                }
                self.buf.end_length(l);
            }
            self.class_typeref(c);
        } else {
            self.tpt(new_ty);
            self.class_typeref(c);
        }
        self.buf.end_length(sl);
        self.ctor_args(c, via, targs, args, &clauses, ta, opens, block);
        self.lifted_ctor_args(c, via, targs, args, &ctor_params(&clauses), binds, lifting)
    }

    /// Whether an expression is a stable path `path_type` writes: a local or parameter, `this`,
    /// an object, a val of a path.
    fn is_simple_path(&self, e: TExprId) -> bool {
        if !self.records(e).is_empty() {
            return false;
        }
        match self.w.prog.expr(e) {
            TExpr::This | TExpr::Module(_) => true,
            TExpr::Local(s) => self.params.iter().any(|&(q, _)| q == s) || self.w.syms.sym(s).kind == SymKind::Val && self.w.syms.sym(s).owner == Owner::Local,
            TExpr::Field(r, f) => self.w.syms.sym(f).kind == SymKind::Val && self.is_simple_path(r),
            _ => false,
        }
    }

    /// A stable given of a package or of a static object (a given object, a given alias), which
    /// scalac's `ref` makes an identifier of its path, as the evidence `summon` takes.
    fn is_summoned_path(&self, e: TExprId) -> bool {
        let given = |s: SymId| self.w.syms.sym(s).kind == SymKind::Given;
        if !self.records(e).is_empty() {
            return false;
        }
        match self.w.prog.expr(e) {
            TExpr::Field(r, f) => {
                given(f) && self.records(r).is_empty() && matches!(self.w.prog.expr(r), TExpr::Module(o) if self.static_owner(self.w.syms.class(o).owner))
            }
            TExpr::Static(s) => given(s) && matches!(self.w.syms.sym(s).owner, Owner::Package(_)),
            _ => false,
        }
    }

    /// The singleton type of a path `is_summoned_path` admits.
    fn summoned_path_type(&mut self, e: TExprId) {
        let (s, owner) = match self.w.prog.expr(e) {
            TExpr::Field(r, f) => (f, Some(r)),
            TExpr::Static(s) => (s, None),
            _ => return self.fail("a summoned path of no singleton type".to_string()),
        };
        self.buf.byte(TERMREF);
        let n = self.simple_name(self.w.syms.sym(s).name);
        self.buf.nat(n as u64);
        match owner {
            Some(r) => self.path_type(r),
            None => {
                let owner = self.w.syms.sym(s).owner;
                self.term_owner_prefix(s, owner);
            }
        }
    }

    /// The singleton type of a path `is_simple_path` admits.
    fn path_type(&mut self, e: TExprId) {
        match self.w.prog.expr(e) {
            TExpr::This => {
                let c = self.enclosing.last().copied().unwrap_or(ClassId(u32::MAX));
                self.buf.byte(THIS);
                self.class_typeref(c);
            }
            TExpr::Module(o) => self.module_termref(o),
            TExpr::Local(s) => self.local_ref(s),
            TExpr::Field(r, f) => {
                self.buf.byte(TERMREF);
                let n = self.simple_name(self.w.syms.sym(f).name);
                self.buf.nat(n as u64);
                self.path_type(r);
            }
            _ => self.fail("a path of no singleton type".to_string()),
        }
    }

    /// A constructor call's type arguments and argument clauses after its selection, and the
    /// block of the temporaries it was written in.
    #[allow(clippy::too_many_arguments)]
    /// The vals of a constructor call's block (`ctor_call`), its arguments' and then its
    /// defaults', the references to them filled.
    fn lifted_ctor_args(&mut self, c: ClassId, via: Option<SymId>, targs: &[TypeId], args: &[TExprId], params: &[ParamSig], binds: Vec<(TExprId, usize)>, lifting: Option<crate::tasty::write::buf::Slot>) {
        let Some(b) = lifting else { return };
        let own: Vec<TParamId> = self.w.syms.class(c).own_tparams().to_vec();
        let subst: Subst = own.iter().copied().zip(targs.iter().copied()).collect();
        for &(e, i) in &binds {
            let default = matches!(self.records(e).form, Some(Form::Default));
            let ty = if default { self.w.types.subst(params[i].ty, &subst) } else { self.node_type(e).map(|t| self.w.widen_lit(t)).unwrap_or(ANY) };
            let name = format!("{}$1", self.name(params[i].name));
            self.lifted_val(e, &name, ty, |p| {
                if default {
                    let getter = match via {
                        Some(s) => Getter::Method(s, None),
                        None => Getter::Ctor(c),
                    };
                    p.within(Some(e), |p| p.default_arg(getter, i, targs, &args[..i]))
                } else {
                    p.term_unnamed(e)
                }
            });
        }
        for (e, _) in binds {
            self.lifted_terms.remove(&e);
        }
        self.buf.end_length(b);
    }

    fn ctor_args(&mut self, c: ClassId, via: Option<SymId>, targs: &[TypeId], args: &[TExprId], clauses: &[ClauseSig], ta: Option<crate::tasty::write::buf::Slot>, opens: Vec<crate::tasty::write::buf::Slot>, block: Option<(crate::tasty::write::buf::Slot, ListRef)>) {
        if let Some(l) = ta {
            for &t in targs {
                self.tpt(t);
            }
            self.buf.end_length(l);
        }
        let mut at = 0;
        for (ci, clause) in clauses.iter().enumerate() {
            for p in &clause.params {
                let primitive = if p.by_name || p.repeated { None } else { self.numeric_class(p.ty) };
                let ty = (!p.by_name && !p.repeated).then_some(p.ty);
                let u = ParamUse { repeated: p.repeated, elem: p.repeated.then_some(p.ty), expected: None, primitive, ty };
                let getter = match via {
                    Some(s) => Getter::Method(s, None),
                    None => Getter::Ctor(c),
                };
                match self.elide_args.clone() {
                    Some(reason) => {
                        self.pending_withheld = Some(reason);
                        self.elided_bodies += 1;
                        self.mark_tree();
                        self.elided_byte();
                        if p.repeated {
                            self.repeated_type(p.ty);
                        } else {
                            self.ty(p.ty);
                        }
                    }
                    None => self.argument(args[at], u, getter, at, targs, &args[..at]),
                }
                at += 1;
            }
            self.buf.end_length(opens[clauses.len() - 1 - ci]);
        }
        if let Some((b, l)) = block {
            let stmts: Vec<TStmt> = self.w.prog.stmt_list(l).to_vec();
            self.statements(&stmts);
            self.buf.end_length(b);
        }
    }

    /// A secondary constructor's body: `{ this(args); stats; () }`, the typer's block of the
    /// call of the other constructor and the statements after it.
    pub(super) fn secondary_body(&mut self, c: ClassId, f: crate::tir::FunId) {
        let Some(body) = self.w.prog.funs[f.idx()].body else { return self.fail("a secondary constructor without a body".to_string()) };
        let TExpr::Block(stmts, res) = self.w.prog.expr(body) else { return self.fail("a secondary constructor's shape".to_string()) };
        let stmts: Vec<TStmt> = self.w.prog.stmt_list(stmts).to_vec();
        let Some((&TStmt::Expr(first), rest)) = stmts.split_first() else { return self.fail("a secondary constructor's call".to_string()) };
        let (via, args) = match self.w.prog.expr(first) {
            TExpr::New(k, args) if k == c => (None, args),
            TExpr::NewVia(s, args) if self.w.syms.sym(s).owner == Owner::Class(c) => (Some(s), args),
            _ => return self.fail("a secondary constructor's call".to_string()),
        };
        let args = self.w.prog.expr_list(args).to_vec();
        let own: Vec<TypeId> = self.w.syms.class(c).own_tparams().to_vec().into_iter().map(|tp| self.w.types.param(tp)).collect();
        let this_ty = self.w.types.class(c, &own);
        let (src, ctx) = (self.src_ctx, self.span_ctx);
        self.src_ctx = self.file;
        self.within(Some(body), |p| {
            let b = p.open(BLOCK);
            p.unit_literal();
            p.within(Some(first), |p| {
                p.self_init = true;
                p.ctor_call(c, via, &own, &args, this_ty, None);
                p.self_init = false;
            });
            p.statements(rest);
            match p.w.prog.expr(res) {
                TExpr::Block(more, last) if p.records(res).is_empty() => {
                    let more: Vec<TStmt> = p.w.prog.stmt_list(more).to_vec();
                    p.statements(&more);
                    if !(matches!(p.w.prog.expr(last), TExpr::Unit) && p.w.prog.span_of(last).is_none()) {
                        p.term(last);
                    }
                }
                TExpr::Unit if p.w.prog.span_of(res).is_none() => {}
                _ => p.term(res),
            }
            p.buf.end_length(b);
        });
        self.src_ctx = src;
        self.span_ctx = ctx;
    }

    /// The first parent of a class: its constructor call with what the class passes it.
    pub(super) fn parent_call(&mut self, c: ClassId, parent: TypeId) {
        let parent = self.w.zonk(parent);
        let (k, targs) = match self.w.types.get(parent) {
            Type::Class(k, args) => (k, self.w.types.items(args).to_vec()),
            _ => return self.fail("a parent that is no class".to_string()),
        };
        let tc = self.index.tclasses.get(&c).map(|&i| {
            let t = &self.w.prog.classes[i as usize];
            (t.parent_args, t.parent_via, t.parent_prelude)
        });
        let (args, via, prelude): (Vec<TExprId>, Option<SymId>, Option<ListRef>) = match self.anon_parent_args.take() {
            Some(args) => (args, tc.and_then(|t| t.1), None),
            None => match tc {
                Some((a, via, prelude)) => (a.map(|l| self.w.prog.expr_list(l).to_vec()).unwrap_or_default(), via, Some(prelude)),
                None => (Vec::new(), None, None),
            },
        };
        let outer_args = self.w.syms.class(k).outer_tparams as usize;
        let own: Vec<TypeId> = targs.iter().skip(outer_args).copied().collect();
        self.term_at(None);
        let outside = std::mem::replace(&mut self.in_parent_args, true);
        self.ctor_call(k, via, &own, &args, parent, prelude);
        self.in_parent_args = outside;
    }

    /// A trait parent's constructor call with what the class passes it.
    pub(super) fn trait_parent_call(&mut self, parent: TypeId, args: &[TExprId], via: Option<SymId>, prelude: Option<ListRef>) {
        let parent = self.w.zonk(parent);
        let (k, targs) = match self.w.types.get(parent) {
            Type::Class(k, args) => (k, self.w.types.items(args).to_vec()),
            _ => return self.fail("a trait parent that is no class".to_string()),
        };
        let outer_args = self.w.syms.class(k).outer_tparams as usize;
        let own: Vec<TypeId> = targs.iter().skip(outer_args).copied().collect();
        self.term_at(None);
        let outside = std::mem::replace(&mut self.in_parent_args, true);
        self.ctor_call(k, via, &own, args, parent, prelude);
        self.in_parent_args = outside;
    }

    /// `new C { .. }`: the anonymous class defined in a block whose value is its instance, typed
    /// as its parents (scalac's `{ final class $anon extends ..; (new $anon(): T) }`).
    fn anon_instance(&mut self, c: ClassId, args: &[TExprId], e: TExprId) {
        if self.w.sam_classes.contains_key(&c) {
            return self.sam_closure(c);
        }
        let captures = self.index.tclasses.get(&c).map_or(0, |&i| self.w.prog.classes[i as usize].ctor_params.len());
        let parent_args: Vec<TExprId> = args.iter().skip(captures).copied().collect();
        let parents: Vec<TypeId> = self.w.syms.class(c).parents.clone();
        let parents: Vec<TypeId> = parents.into_iter().filter(|&p| p != self.w.b.t_any_ref).collect();
        let ty = match parents.split_first() {
            Some((&first, rest)) => rest.iter().fold(first, |acc, &p| self.w.types.inter(acc, p)),
            None => self.w.b.t_any_ref,
        };
        // The instance's type as the typer gave it, the type members its body defines refining
        // the parents; a cast's target where the instance is cast.
        let cast = self.records(e).wraps.iter().any(|w| matches!(w, Wrap::Cast(_)));
        let ty = match self.node_type(e) {
            Some(t) if !cast && !matches!(self.w.types.get(t), Type::Class(k, _) if k == c) => t,
            _ => ty,
        };
        // A type the members refine: the context's, where it has one, which scalac ascribes the
        // instance where the refined parents are not one.
        let expected = self.expected_anon.filter(|&(x, _)| x == e && !cast).map(|(_, t)| self.w.zonk(t)).filter(|&t| !self.w.types.has_vars(t) && !self.is_unit(t));
        let ty = expected.unwrap_or(ty);
        if expected.is_none() && self.refining_anon_class(c, ty) {
            return self.fail("an anonymous class whose members refine its type".to_string());
        }
        let b = self.open(BLOCK);
        self.term_at(None);
        let t = self.open(TYPED);
        let this_ty = self.w.types.class(c, &[]);
        self.ctor_call(c, None, &[], &[], this_ty, None);
        self.tpt(ty);
        self.buf.end_length(t);
        self.anon_parent_args = Some(parent_args);
        self.local_class(c);
        self.anon_parent_args = None;
        self.buf.end_length(b);
    }

    /// Whether an anonymous class defines a type member, which scalac's type of the instance
    /// refines its parents with (its term members refine nothing, as scalac 3.8.4's typer has
    /// it), where the typer's type of the instance has no refinement.
    fn refining_anon_class(&mut self, c: ClassId, ty: TypeId) -> bool {
        self.w.complete_class(c);
        let info = self.class_info(c);
        !info.type_aliases.is_empty() && !matches!(self.w.types.get(ty), Type::Refined(..))
    }

    /// The lambda a SAM conversion made a class of: the closure over a local `$anonfun` of the
    /// lambda's parameters and body, with the trait as its target type.
    fn sam_closure(&mut self, c: ClassId) {
        // An inline method's body keeps its classes with its record, not among the program's.
        let methods = match self.index.tclasses.get(&c) {
            Some(&i) => Some(self.w.prog.classes[i as usize].methods.clone()),
            None => self.inline_body.as_ref().and_then(|d| d.classes.iter().find(|t| t.id == c)).map(|t| t.methods.clone()),
        };
        let Some(methods) = methods else { return self.fail("a SAM conversion's class".to_string()) };
        let Some(&f) = methods.first() else { return self.fail("a SAM conversion's method".to_string()) };
        let tf = self.w.prog.funs[f.idx()].clone();
        let sam = self.w.syms.class(c).parents.first().copied().unwrap_or(ANY);
        let ret = self.w.sig_of(tf.sym).ret;
        let Some(body) = tf.body else { return self.fail("a SAM conversion's method without a body".to_string()) };
        let b = self.open(BLOCK);
        self.term_at(None);
        let l = self.open(LAMBDA);
        self.buf.byte(TERMREFDIRECT);
        let fwd = self.buf.forward_reference();
        self.tpt(sam);
        self.buf.end_length(l);
        let d = self.buf.addr();
        self.buf.fill(fwd, d);
        // The class the SAM conversion makes, which the pickle holds as the lambda, is recorded
        // at its `$anonfun` (kind 2): what names it in a downstream.
        self.class_origins.push((d, c));
        self.term_at(None);
        let dl = self.open(DEFDEF);
        let n = self.names.simple("$anonfun");
        self.buf.nat(n as u64);
        if tf.params.is_empty() {
            self.buf.byte(EMPTYCLAUSE);
        }
        for &s in &tf.params {
            self.local_param(s);
        }
        self.tpt(ret);
        self.term(body);
        self.write_flags(&[SYNTHETIC, ARTIFACT]);
        self.buf.end_length(dl);
        self.buf.end_length(b);
    }

    /// `{ case .. }` as a partial function: the closure over the scrutinee the typer's
    /// `applyOrElse` takes, whose body is its match without the last case, the default's, with
    /// `PartialFunction[A, B]` as the target type.
    fn partial_function(&mut self, e: TExprId, args: &[TExprId]) {
        let Some(&apply_or_else) = args.first() else { return self.fail("a partial function's applyOrElse".to_string()) };
        let TExpr::Lambda(params, body) = self.w.prog.expr(apply_or_else) else { return self.fail("a partial function's applyOrElse".to_string()) };
        let Some(&x) = self.w.prog.sym_list(params).first() else { return self.fail("a partial function's parameter".to_string()) };
        let TExpr::Match(scrut, cases) = self.w.prog.expr(body) else { return self.fail("a partial function's match".to_string()) };
        let cases: Vec<crate::tir::TCase> = self.w.prog.case_list(cases).to_vec();
        let Some((_, own)) = cases.split_last() else { return self.fail("a partial function's cases".to_string()) };
        let own = own.to_vec();
        let pf = self.node_type(e).unwrap_or(ANY);
        let ret = match self.w.types.get(pf) {
            Type::Class(_, targs) => self.w.types.items(targs).get(1).copied(),
            _ => None,
        };
        let Some(ret) = ret else { return self.fail("a partial function's result type".to_string()) };
        let scrut_ty = self.node_type(scrut).unwrap_or(ANY);
        let b = self.open(BLOCK);
        self.term_at(None);
        let l = self.open(LAMBDA);
        self.buf.byte(TERMREFDIRECT);
        let fwd = self.buf.forward_reference();
        self.tpt(pf);
        self.buf.end_length(l);
        let d = self.buf.addr();
        self.buf.fill(fwd, d);
        self.term_at(None);
        let dl = self.open(DEFDEF);
        let n = self.names.simple("$anonfun");
        self.buf.nat(n as u64);
        self.local_param(x);
        self.tpt(ret);
        self.term_at(Some(body));
        let m = self.open(MATCH);
        self.term(scrut);
        let unit = self.is_unit(ret);
        for c in own {
            self.case(c, scrut_ty, unit);
        }
        self.buf.end_length(m);
        self.write_flags(&[SYNTHETIC, ARTIFACT]);
        self.buf.end_length(dl);
        self.buf.end_length(b);
    }

    /// A class defined in a body, written where its block defines it.
    fn local_class(&mut self, c: ClassId) {
        if self.written_locals.insert(c, ()).is_some() {
            return;
        }
        let outside = std::mem::replace(&mut self.in_parent_args, false);
        self.local_class_body(c);
        self.in_parent_args = outside;
    }

    fn local_class_body(&mut self, c: ClassId) {
        if self.bodies_open > 0 {
            self.undo.push(Undo::Written(c));
        }
        self.add_local(Key::Class(c));
        self.class_origins.push((self.buf.addr(), c));
        self.class_def(c);
    }

    // ---- closures and blocks --------------------------------------------------------------

    /// `(params) => body`: a local `$anonfun` and the closure over it.
    fn closure(&mut self, e: TExprId, params: &[SymId], body: TExprId) {
        let expected = self.expected_fn.take();
        // A closure where a `js.FunctionN` is expected: scalac's SAM closure, of that target.
        let js_target = expected.filter(|&t| self.js_alias(t) == Some("Function"));
        let fun_ty = js_target.or_else(|| self.node_type(e)).or(expected).unwrap_or(ANY);
        // A polymorphic function's closure: its method takes the type parameters
        // (`$anonfun[T](x: A): R`), the closure's type the `PolyFunction` refinement scalac infers.
        let head = self.w.deref_alias(fun_ty);
        let (poly, fun_ty): (Vec<TParamId>, TypeId) = match self.w.types.get(head) {
            // The lambda's own parameters, which its body names.
            Type::Poly(ps, fun) => (self.w.types.poly_params(ps).iter().filter_map(|&p| match self.w.types.get(p) {
                Type::Param(id) => Some(id),
                _ => None,
            }).collect(), fun),
            _ => (Vec::new(), fun_ty),
        };
        // A function type with named parameters: its `apply`'s result over the closure's own
        // parameters, which may name them (dotty's `Typer.decomposeProtoFunction`,
        // `typedPolyFunctionValue`: `restpe.substParams(mt, syms.map(_.termRef))`).
        let named = self.named_apply(fun_ty).filter(|sig| sig.clauses.len() == 1 && sig.clauses[0].params.len() == params.len());
        let ret = match named {
            Some(sig) => {
                let ret = sig.ret;
                let renamed: Vec<(SymId, TypeId)> = sig.clauses[0].params.iter().zip(params).filter(|(p, &s)| p.sym != s).map(|(p, &s)| (p.sym, self.w.types.mk(Type::Term(s)))).collect();
                Some(if renamed.is_empty() { ret } else { self.w.subst_paths(ret, &renamed) })
            }
            None => match self.w.types.get(fun_ty) {
                Type::Class(_, args) | Type::Alias(_, args) => self.w.types.items(args).last().copied(),
                _ => None,
            },
        };
        let Some(ret) = ret.or_else(|| self.node_type(body)) else {
            return self.fail("a closure of no known type".to_string());
        };
        // A context function's closure takes its parameters as a using clause.
        let context = self.w.as_context_function(fun_ty).is_some();
        let b = self.open(BLOCK);
        self.term_at(None);
        let l = self.open(LAMBDA);
        self.buf.byte(TERMREFDIRECT);
        let fwd = self.buf.forward_reference();
        if let Some(t) = js_target {
            self.ty(t);
        }
        self.buf.end_length(l);
        let d = self.buf.addr();
        self.buf.fill(fwd, d);
        self.term_at(None);
        let dl = self.open(DEFDEF);
        let n = self.names.simple("$anonfun");
        self.buf.nat(n as u64);
        let tmark = self.tparams.len();
        let ids = self.bind_tparams(&poly);
        self.write_bound_tparams(&poly, &ids, false);
        if params.is_empty() {
            self.buf.byte(EMPTYCLAUSE);
        }
        // The parameters of a closure the typer made for an expected context function are
        // `contextual$N`, numbered per file as scalac's unique names are.
        let contextual = params.first().map_or(false, |&p| self.w.interner.get(self.w.syms.sym(p).name).starts_with("contextual$"));
        let first = if contextual { Some(self.contextual_number(self.w.prog.span_of(e).map(|(_, s)| s.start), params.len() as u32)) } else { None };
        // The closure's parameters are in scope of its body: a `Quotes` among them is the one the
        // reflection API's paths there start from.
        let pmark = self.params.len();
        for (i, &s) in params.iter().enumerate() {
            let addr = self.buf.addr();
            self.local_param_named(s, context, first.map(|n| format!("contextual${}", n + i as u32)));
            self.params.push((s, addr));
            if context {
                self.using_params.insert(s, ());
            }
        }
        self.tpt(ret);
        self.term_to(body, ret);
        self.params.truncate(pmark);
        self.tparams.truncate(tmark);
        self.write_flags(&[SYNTHETIC, ARTIFACT]);
        self.buf.end_length(dl);
        self.buf.end_length(b);
    }

    /// A parameter of a local method or a closure.
    fn local_param(&mut self, s: SymId) {
        self.local_param_of(s, false)
    }

    /// A parameter of a local method or a closure, of a using clause where `given`.
    fn local_param_of(&mut self, s: SymId, given: bool) {
        self.local_param_named(s, given, None)
    }

    /// `local_param_of`, named `name` where given in the place of the symbol's name.
    fn local_param_named(&mut self, s: SymId, given: bool, name: Option<String>) {
        let addr = self.buf.addr();
        self.define(Key::Sym(s), addr);
        self.term_at(None);
        let l = self.open(PARAM);
        let n = match name {
            Some(name) => self.names.simple(&name),
            None => self.simple_name(self.w.syms.sym(s).name),
        };
        self.buf.nat(n as u64);
        let t = self.w.sig_of(s).ret;
        let zt = self.w.zonk(t);
        if matches!(self.w.types.get(zt), Type::Wild | Type::BoundedWild(..)) {
            return self.fail("a parameter of a wildcard type".to_string());
        }
        if self.w.syms.sym(s).by_name {
            self.term_at(None);
            self.buf.byte(BYNAMETPT);
        }
        self.tpt(t);
        if given {
            self.buf.byte(GIVEN);
        }
        self.buf.end_length(l);
    }

    /// Whether a local is a by-name parameter, which a read evaluates.
    fn is_by_name(&self, s: SymId) -> bool {
        self.w.syms.sym(s).by_name || self.by_name_params.contains_key(&s)
    }

    /// A block, the imports of the reflection API in it holding until its end.
    fn block(&mut self, e: TExprId, stmts: ListRef, res: TExprId) {
        let scopes = self.reflect_scopes.len();
        self.block_trees(e, stmts, res);
        self.reflect_scopes.truncate(scopes);
    }

    fn block_trees(&mut self, e: TExprId, stmts: ListRef, res: TExprId) {
        let stmts: Vec<TStmt> = self.w.prog.stmt_list(stmts).to_vec();
        // An object of the reflection API reached through a prefix that is no path
        // (`quotes.reflect.report`): the object selected on it, which evaluates it.
        if let ([TStmt::Expr(r)], TExpr::Module(c)) = (stmts.as_slice(), self.w.prog.expr(res)) {
            if self.records(res).receiver.is_some() && self.is_reflect_class(c) {
                return self.reflect_module(c, Some(*r));
            }
        }
        // `{ O; A.m(..) }`, the typer's call of an export forwarder of the object `O` (the object
        // touched, the exported member called): the forwarder's call, as scalac's.
        if let [TStmt::Expr(o)] = stmts.as_slice() {
            if let Some((m, args, name)) = self.export_call(*o, res) {
                let targs = self.records(res).targs;
                if matches!(self.w.prog.expr(res), TExpr::Field(..) | TExpr::Static(_)) && !self.is_method(m) {
                    self.buf.byte(SELECT);
                    let n = self.simple_name(name);
                    self.buf.nat(n as u64);
                    return self.term(*o);
                }
                self.forwarder_name = Some(name);
                self.call(m, Qual::Expr(*o), targs, &args);
                self.forwarder_name = None;
                return;
            }
        }
        let classes = self.block_classes(e, &stmts, res);
        let unit = self.typed_unit(e);
        let l = self.open(BLOCK);
        // The context's type is the result's.
        let expected = self.expected_anon.filter(|&(x, _)| x == e).map(|(_, t)| (res, t));
        let outer = std::mem::replace(&mut self.expected_anon, expected);
        self.term_in(res, unit);
        self.expected_anon = outer;
        // The block's imports, before the statements they stand before, as the search of an
        // inline expansion or a macro there sees them, and its local inline methods
        // (`Capture::block_stmts`).
        let extras: Vec<(u32, u32, crate::tir::capture::BlockStmt)> = match (self.w.prog.span_of(e), self.w.prog.capture.as_deref()) {
            (Some((file, span)), Some(c)) => c.block_stmts.get(&(file, span.start)).cloned().unwrap_or_default(),
            _ => Vec::new(),
        };
        // And the names an inline body's block binds to a value's members (`import v.{value as
        // g}`), which the definition check keeps as the selections they stand for.
        let aliases: Vec<(u32, SymId, TExprId)> = match &self.inline_body {
            Some(d) => d.aliases.iter().filter(|a| a.block == e).map(|a| (a.at, a.local, a.tree)).collect(),
            None => Vec::new(),
        };
        // The block's own classes stand among its statements in source order.
        let mut classes = classes.into_iter().peekable();
        for (i, st) in stmts.iter().enumerate() {
            for (_, from, x) in extras.iter().filter(|&&(at, _, _)| at as usize == i) {
                self.block_extra(e, *from, x);
            }
            for &(_, local, tree) in aliases.iter().filter(|&&(at, _, _)| at as usize == i) {
                self.alias_import(local, tree);
            }
            let at = self.stmt_start(st);
            while let Some(&(start, c)) = classes.peek() {
                if at.map_or(false, |a| start < a) {
                    self.local_class(c);
                    classes.next();
                } else {
                    break;
                }
            }
            self.statements(std::slice::from_ref(st));
        }
        for (_, c) in classes {
            self.local_class(c);
        }
        for (_, from, x) in extras.iter().filter(|&&(at, _, _)| at as usize >= stmts.len()) {
            self.block_extra(e, *from, x);
        }
        for &(_, local, tree) in aliases.iter().filter(|&&(at, _, _)| at as usize >= stmts.len()) {
            self.alias_import(local, tree);
        }
        self.buf.end_length(l);
    }

    /// A statement of the block `e` its typed form leaves out, at `from` of the block's source.
    fn block_extra(&mut self, e: TExprId, from: u32, x: &crate::tir::capture::BlockStmt) {
        match x {
            crate::tir::capture::BlockStmt::Import(import) => {
                self.import_at = self.w.prog.span_of(e).map(|(file, _)| (file, from));
                self.import_tree(None, import);
                self.import_at = None;
            }
            &crate::tir::capture::BlockStmt::InlineDef(s) => self.local_inline_def(s),
        }
    }

    /// A block's local inline method, from the record the definition check stored, as a
    /// member's (`inline_rhs`): the calls the block makes of it name its `DEFDEF`.
    fn local_inline_def(&mut self, s: SymId) {
        let info = self.sym_info(s);
        let sig = self.w.sig_of(s).clone();
        let addr = self.buf.addr();
        self.define(Key::Sym(s), addr);
        self.term_at(None);
        let l = self.open(DEFDEF);
        let n = self.simple_name(info.name);
        self.buf.nat(n as u64);
        let tmark = self.tparams.len();
        let pmark = self.params.len();
        self.def_params(&sig.tparams, &sig.clauses, None);
        let outer = self.result_tpt.replace(self.buf.addr());
        self.tpt(sig.ret);
        let record = self.w.inline_definitions.get(&s).cloned();
        match record.and_then(|d| d.body.map(|b| (d, b))) {
            Some((d, b)) => {
                let transparent = info.mods & mods::TRANSPARENT != 0;
                let saved = self.inline_body.replace(d);
                match self.result_tpt.filter(|_| !transparent) {
                    Some(at) => {
                        let t = self.open(TYPED);
                        self.term_to(b, sig.ret);
                        self.buf.byte(SHAREDTERM);
                        self.buf.reference(at);
                        self.buf.end_length(t);
                    }
                    None => self.term_to(b, sig.ret),
                }
                self.inline_body = saved;
            }
            None => self.fail("a local inline method the definition check stored no body of".to_string()),
        }
        self.result_tpt = outer;
        self.tparams.truncate(tmark);
        self.params.truncate(pmark);
        let mut flags = vec![INLINE];
        if info.mods & mods::TRANSPARENT != 0 {
            flags.push(TRANSPARENT);
        }
        self.write_flags(&flags);
        self.buf.end_length(l);
    }

    /// `import v.{m as a}` of an inline body's block: `IMPORT v (IMPORTED m RENAMED a)`, the
    /// selection `v.m` the definition check bound `a` to.
    fn alias_import(&mut self, local: SymId, tree: TExprId) {
        let (r, m) = match self.w.prog.expr(tree) {
            TExpr::Field(r, m) => (r, m),
            TExpr::CallMethod(r, m, args) if args.len == 0 => (r, m),
            _ => return,
        };
        self.mark_tree();
        let l = self.open(IMPORT);
        self.term(r);
        let member = self.w.syms.sym(m).name;
        let bound = self.w.syms.sym(local).name;
        self.mark_tree();
        self.buf.byte(IMPORTED);
        let n = self.simple_name(member);
        self.buf.nat(n as u64);
        if bound != member {
            self.mark_tree();
            self.buf.byte(RENAMED);
            let r = self.simple_name(bound);
            self.buf.nat(r as u64);
        }
        self.buf.end_length(l);
    }

    /// `val P = e`, as `Desugar` makes it: one variable `val v = (e: @unchecked) match { case
    /// P => v }`, several a synthetic tuple of them and a val per variable, none the match alone.
    fn pattern_definition(&mut self, p: TPatId, e: TExprId) {
        let vars: Vec<SymId> = self.pattern_vars(p);
        let scrut = self.node_type(e).unwrap_or(ANY);
        let types: Vec<TypeId> = vars.iter().map(|&v| {
            let t = self.w.sig_of(v).ret;
            self.w.zonk(t)
        }).collect();
        let tuple = (vars.len() > 1).then(|| self.w.tuple_of(&types));
        // The match: the case's binders apart from the vals the definition makes.
        let write_match = |p0: &mut Self, value: &dyn Fn(&mut Self)| {
            p0.term_at(Some(e));
            let m = p0.open(MATCH);
            p0.term_at(None);
            let t = p0.open(TYPED);
            p0.term(e);
            p0.term_at(None);
            let a = p0.open(ANNOTATEDTPT);
            p0.tpt(scrut);
            p0.annotation_tree("scala", "unchecked");
            p0.buf.end_length(a);
            p0.buf.end_length(t);
            p0.term_at(None);
            let c = p0.open(CASEDEF);
            let mark = p0.case_binders.len();
            let (tmark, bmark) = (p0.tparams.len(), p0.pattern_binds.len());
            let outer = std::mem::replace(&mut p0.binders_apart, true);
            p0.pattern(p, scrut);
            p0.binders_apart = outer;
            value(p0);
            p0.case_binders.truncate(mark);
            p0.tparams.truncate(tmark);
            p0.pattern_binds.truncate(bmark);
            p0.buf.end_length(c);
            p0.buf.end_length(m);
        };
        match vars.as_slice() {
            [] => write_match(self, &|p0| p0.unit_literal()),
            [v] => {
                let v = *v;
                let addr = self.buf.addr();
                self.define(Key::Sym(v), addr);
                self.term_at(None);
                let l = self.open(VALDEF);
                let n = self.simple_name(self.w.syms.sym(v).name);
                self.buf.nat(n as u64);
                self.tpt(types[0]);
                write_match(self, &|p0| {
                    p0.term_at(None);
                    p0.local_ref(v)
                });
                self.buf.end_length(l);
            }
            _ => {
                let tuple = tuple.unwrap();
                let at = self.buf.addr();
                self.term_at(None);
                let l = self.open(VALDEF);
                self.fresh_vals += 1;
                let n = self.names.simple(&format!("x${}", self.fresh_vals));
                self.buf.nat(n as u64);
                self.tpt(tuple);
                let k = vars.len();
                write_match(self, &|p0| {
                    p0.term_at(None);
                    let a = p0.open(APPLY);
                    let ta = p0.open(TYPEAPPLY);
                    let sl = p0.open(SELECTIN);
                    let mut sig = vec![SigParam::Types(k)];
                    sig.extend((0..k).map(|_| SigParam::Type("java.lang.Object".into())));
                    let tn = p0.names.signed("apply", None, &sig, &format!("scala.Tuple{}", k));
                    p0.buf.nat(tn as u64);
                    p0.object_path("scala", &format!("Tuple{}", k));
                    p0.object_class_ref("scala", &format!("Tuple{}", k));
                    p0.buf.end_length(sl);
                    for &t in &types {
                        p0.tpt(t);
                    }
                    p0.buf.end_length(ta);
                    for &v in &vars {
                        p0.term_at(None);
                        p0.local_ref(v);
                    }
                    p0.buf.end_length(a);
                });
                self.write_flags(&[SYNTHETIC]);
                self.buf.end_length(l);
                for (i, &v) in vars.iter().enumerate() {
                    let addr = self.buf.addr();
                    self.define(Key::Sym(v), addr);
                    self.term_at(None);
                    let vl = self.open(VALDEF);
                    let n = self.simple_name(self.w.syms.sym(v).name);
                    self.buf.nat(n as u64);
                    self.tpt(types[i]);
                    self.term_at(None);
                    self.buf.byte(SELECT);
                    let f = self.names.simple(&format!("_{}", i + 1));
                    self.buf.nat(f as u64);
                    self.term_at(None);
                    self.buf.byte(TERMREFDIRECT);
                    self.buf.reference(at);
                    self.buf.end_length(vl);
                }
            }
        }
    }

    /// The variables a pattern binds, in order.
    fn pattern_vars(&self, p: TPatId) -> Vec<SymId> {
        let mut out = Vec::new();
        let mut stack = vec![p];
        while let Some(p) = stack.pop() {
            match self.w.prog.pats[p.idx()] {
                TPat::Bind(s, inner) => {
                    out.push(s);
                    stack.extend(inner);
                }
                TPat::Test(_, _, inner) => stack.push(inner),
                TPat::Class(_, _, _, subs) | TPat::Alt(subs) => stack.extend(self.w.prog.pat_lists[subs.range()].iter().rev().copied()),
                TPat::Seq(items, rest) => {
                    stack.extend(rest);
                    stack.extend(self.w.prog.pat_lists[items.range()].iter().rev().copied());
                }
                TPat::Unapply(_, _, inner) => stack.push(inner),
                TPat::Wildcard | TPat::Equals(..) => {}
            }
        }
        out
    }

    fn statements(&mut self, stmts: &[TStmt]) {
        for &st in stmts {
            match st {
                TStmt::Expr(x) => self.term(x),
                TStmt::Val(s, x) => self.local_val(s, x),
                TStmt::Fun(f) => self.local_def(f),
                TStmt::Pat(p, e) => self.pattern_definition(p, e),
            }
        }
    }

    fn stmt_start(&self, st: &TStmt) -> Option<u32> {
        match *st {
            TStmt::Expr(x) | TStmt::Val(_, x) | TStmt::Pat(_, x) => self.w.prog.span_of(x).map(|(_, s)| s.start),
            TStmt::Fun(f) => Some(self.w.syms.sym(self.w.prog.funs[f.idx()].sym).span.start),
        }
    }

    /// The named classes the source defines directly in the block `e`: inside its span and
    /// inside none of its statements', in source order.
    fn block_classes(&mut self, e: TExprId, stmts: &[TStmt], res: TExprId) -> Vec<(u32, ClassId)> {
        let Some((file, span)) = self.w.prog.span_of(e) else { return Vec::new() };
        let Some(candidates) = self.index.local_classes.get(&file) else { return Vec::new() };
        let inner: Vec<Span> = stmts
            .iter()
            .filter_map(|st| match *st {
                TStmt::Expr(x) | TStmt::Val(_, x) | TStmt::Pat(_, x) => self.w.prog.span_of(x).map(|(_, s)| s),
                TStmt::Fun(f) => {
                    let s = self.w.prog.funs[f.idx()].sym;
                    let info = self.w.syms.sym(s);
                    Some(self.def_span(info.file, info.def, info.span))
                }
            })
            .chain(self.w.prog.span_of(res).map(|(_, s)| s))
            .collect();
        // The copies an expansion makes of the callee's classes share their spans: a definition's
        // body defines the originals, an expansion's block the copies it creates.
        let copies = !self.inlined_open.is_empty();
        let mut out: Vec<(u32, ClassId)> = candidates
            .iter()
            .filter(|&&(s, _)| span.start <= s.start && s.end <= span.end && !inner.iter().any(|i| i.start <= s.start && s.end <= i.end && *i != span))
            .map(|&(s, c)| (s.start, c))
            .filter(|&(_, c)| !self.written_locals.contains_key(&c) && self.w.syms.class(c).made_at.is_some() == copies)
            .collect();
        if copies && out.len() > 1 {
            let made = self.classes_made_in(e);
            out.retain(|(_, c)| made.contains(c));
        }
        out.sort();
        out
    }

    /// The classes the tree `e` creates, with the classes each extends.
    fn classes_made_in(&self, e: TExprId) -> Vec<ClassId> {
        let mut made: Vec<ClassId> = Vec::new();
        for x in self.w.prog.descendants(e) {
            let c = match self.w.prog.expr(x) {
                TExpr::New(c, _) => c,
                TExpr::NewVia(s, _) => match self.w.syms.sym(s).owner {
                    Owner::Class(c) => c,
                    _ => continue,
                },
                _ => continue,
            };
            let mut pending = vec![c];
            while let Some(k) = pending.pop() {
                if made.contains(&k) {
                    continue;
                }
                made.push(k);
                for &p in &self.w.syms.class(k).parents {
                    if let Type::Class(pc, _) = self.w.types.get(p) {
                        pending.push(pc);
                    }
                }
            }
        }
        made
    }

    fn local_val(&mut self, s: SymId, init: TExprId) {
        let info = self.sym_info(s);
        let addr = self.buf.addr();
        self.define(Key::Sym(s), addr);
        self.term_at(None);
        let l = self.open(VALDEF);
        let n = self.simple_name(info.name);
        self.buf.nat(n as u64);
        let recorded = self.w.prog.capture.as_deref().and_then(|c| c.local_of(s));
        let t = self.inferred_from_inline_call(s, init).or_else(|| recorded.and_then(|r| r.ty)).unwrap_or_else(|| self.w.sig_of(s).ret);
        let t = self.w.zonk(t);
        if matches!(self.w.types.get(t), Type::Wild | Type::BoundedWild(..)) {
            return self.fail("a local of a wildcard type".to_string());
        }
        self.tpt(t);
        self.term_to(init, t);
        let mut flags = Vec::new();
        if info.kind == SymKind::Var {
            flags.push(MUTABLE);
        }
        // A local object's val, which scalac flags as the object it holds.
        if self.w.local_module_of_sym(s).is_some() {
            flags.push(OBJECT);
        } else if info.mods & mods::LAZY != 0 {
            flags.push(LAZY);
        }
        if recorded.map_or(false, |r| r.mods & mods::INLINE != 0) {
            flags.push(INLINE);
        }
        // A local given, which a search the body's expansion makes finds (`{ given Int = 7;
        // summonInline[Int] }`).
        if info.kind == SymKind::Given || info.mods & mods::GIVEN != 0 {
            flags.push(GIVEN);
        }
        let synthetic = {
            let name = self.w.interner.get(info.name);
            name.contains('$')
        };
        if synthetic {
            flags.push(SYNTHETIC);
        }
        self.write_flags(&flags);
        self.buf.end_length(l);
    }

    /// The type scalac infers for a local of no declared type whose initializer the pickle holds
    /// as an ordinary inline call: the call's result type, which the call's expansion after the
    /// pickler does not narrow (`val m = summonInline[Mirror.Of[Box]]` in a transparent
    /// expansion is a `Mirror.Of[Box]`, where teq's expansion gave the summoned mirror's type).
    fn inferred_from_inline_call(&mut self, s: SymId, init: TExprId) -> Option<TypeId> {
        let info = self.w.syms.sym(s);
        // A local an expansion copied has no definition of its own: its type is the call's
        // where the expansion narrowed it, its own where it is wider (a declared one).
        let copied = info.def.is_none();
        if info.def.map_or(false, |d| matches!(&self.w.ast(info.file).def(d).kind, DefKind::Val { ty: Some(_), .. })) {
            return None;
        }
        let call = self.records(init).inline_calls.last().cloned().filter(|c| !c.transparent)?;
        let sig = self.w.sig_of(call.callee).clone();
        let targs: Vec<TypeId> = self.w.types.items(call.targs).to_vec();
        if targs.len() != sig.tparams.len() {
            return None;
        }
        let subst: crate::types::Subst = sig.tparams.iter().copied().zip(targs).collect();
        let t = self.w.types.subst(sig.ret, &subst);
        // A result naming the type parameters of the callee's class, which the call's receiver
        // instantiates, is left as the local's own type.
        let owner_tparams: Vec<TParamId> = match self.w.syms.sym(call.callee).owner {
            Owner::Class(c) => self.w.syms.class(c).own_tparams().to_vec(),
            _ => Vec::new(),
        };
        if self.w.mentions_tparam_of(t, Some(&owner_tparams)) {
            return None;
        }
        let own = self.w.sig_of(s).ret;
        (!copied || self.w.is_sub(own, t)).then_some(t)
    }

    fn local_def(&mut self, f: crate::tir::FunId) {
        let tf = self.w.prog.funs[f.idx()].clone();
        let s = tf.sym;
        let info = self.sym_info(s);
        let sig = self.w.sig_of(s).clone();
        let addr = self.buf.addr();
        self.define(Key::Sym(s), addr);
        self.term_at(None);
        let l = self.open(DEFDEF);
        let n = self.simple_name(info.name);
        self.buf.nat(n as u64);
        let tmark = self.tparams.len();
        let pmark = self.params.len();
        self.def_params(&sig.tparams, &sig.clauses, None);
        self.tpt(sig.ret);
        match tf.body {
            Some(b) => self.term_to(b, sig.ret),
            None => self.fail("a local method without a body".to_string()),
        }
        self.tparams.truncate(tmark);
        self.params.truncate(pmark);
        let mut flags = Vec::new();
        if info.mods & mods::INLINE != 0 {
            flags.push(INLINE);
        }
        if info.kind == SymKind::Given || info.mods & mods::GIVEN != 0 {
            flags.push(GIVEN);
        }
        self.write_flags(&flags);
        self.buf.end_length(l);
    }

    // ---- patterns -------------------------------------------------------------------------

    /// Whether a case's class fixes an argument the scrutinee's type has as a type parameter
    /// (`Expr[T]` matched by `IntLit extends Expr[Int]`), which scalac reasons about as a GADT.
    fn refines_type_parameter(&mut self, scrut: TypeId, p: TPatId) -> bool {
        let t = match self.w.prog.pats[p.idx()] {
            TPat::Class(_, t, _, _) | TPat::Test(_, t, _) => t,
            TPat::Bind(_, Some(inner)) => return self.refines_type_parameter(scrut, inner),
            TPat::Equals(x, _) => match self.term_type(x) {
                Some(t) => t,
                None => return false,
            },
            TPat::Alt(subs) => {
                let subs: Vec<TPatId> = self.w.prog.pat_lists[subs.range()].to_vec();
                return subs.into_iter().any(|q| self.refines_type_parameter(scrut, q));
            }
            _ => return false,
        };
        let scrut = self.w.zonk(scrut);
        let Type::Class(sc, sargs) = self.w.types.get(scrut) else { return false };
        let sargs: Vec<TypeId> = self.w.types.items(sargs).to_vec();
        if !sargs.iter().any(|&a| matches!(self.w.types.get(a), Type::Param(_))) {
            return false;
        }
        let t = self.w.zonk(t);
        let Some(base) = self.w.base_type(t, sc) else { return false };
        let Type::Class(_, bargs) = self.w.types.get(base) else { return false };
        let bargs: Vec<TypeId> = self.w.types.items(bargs).to_vec();
        sargs.iter().zip(&bargs).any(|(&s, &b)| matches!(self.w.types.get(s), Type::Param(_)) && s != b)
    }

    /// How the inline body being written has `e` among its reducible nodes.
    fn reducible(&self, e: TExprId) -> Option<Reducible> {
        let d = self.inline_body.as_ref()?;
        let i = d.reducible.iter().position(|&r| r == e)?;
        Some(match d.reducible_sources.get(i) {
            Some(crate::tir::ReducibleSource::SummonFrom { .. }) => Reducible::SummonFrom,
            _ => Reducible::Inline,
        })
    }

    /// A case of `summonFrom` as scalac pickles it: a type's search binds a given, `x: T` its
    /// binder, `_: T` a fresh `_$N`, the pattern `TYPED(IDENT _, T)`; `_` the wildcard of the
    /// implicit scrutinee's type, `scala.implicit`.
    fn summon_case(&mut self, c: crate::tir::TCase, unit: bool) {
        self.term_at(Some(c.body));
        let l = self.open(CASEDEF);
        self.term_at(None);
        match self.w.prog.pats[c.pat.idx()] {
            TPat::Wildcard => {
                self.buf.byte(IDENT);
                let n = self.names.simple("_");
                self.buf.nat(n as u64);
                self.external_typeref("scala", "implicit");
            }
            TPat::Bind(s, Some(test)) => {
                let TPat::Test(_, t, _) = self.w.prog.pats[test.idx()] else { return self.fail("a summonFrom case of that shape".to_string()) };
                let addr = self.buf.addr();
                self.define(Key::Sym(s), addr);
                let name = self.name(self.w.syms.sym(s).name);
                self.given_bind(&name, t);
            }
            TPat::Test(_, t, _) => {
                self.summon_wildcards += 1;
                let name = format!("_${}", self.summon_wildcards);
                self.given_bind(&name, t);
            }
            _ => return self.fail("a summonFrom case of that shape".to_string()),
        }
        self.term_in(c.body, unit);
        if let Some(g) = c.guard {
            self.term(g);
        }
        self.buf.end_length(l);
    }

    /// `BIND name T TYPED(IDENT _, T) GIVEN`.
    fn given_bind(&mut self, name: &str, t: TypeId) {
        let l = self.open(BIND);
        let n = self.names.simple(name);
        self.buf.nat(n as u64);
        let t = self.w.zonk(t);
        self.ty(t);
        self.term_at(None);
        let typed = self.open(TYPED);
        self.wildcard(t);
        self.tpt(t);
        self.buf.end_length(typed);
        self.buf.byte(GIVEN);
        self.buf.end_length(l);
    }

    fn case(&mut self, c: crate::tir::TCase, scrut: TypeId, unit: bool) {
        self.term_at(Some(c.body));
        let l = self.open(CASEDEF);
        let (tmark, bmark, gmark) = (self.tparams.len(), self.pattern_binds.len(), self.type_givens.len());
        self.pattern(c.pat, scrut);
        self.term_in(c.body, unit);
        if let Some(g) = c.guard {
            self.term(g);
        }
        self.tparams.truncate(tmark);
        self.pattern_binds.truncate(bmark);
        self.type_givens.truncate(gmark);
        self.buf.end_length(l);
    }

    pub(super) fn pattern(&mut self, p: TPatId, scrut: TypeId) {
        self.term_at(None);
        match self.w.prog.pats[p.idx()] {
            TPat::Wildcard => self.wildcard(scrut),
            TPat::Bind(s, inner) => {
                let (l, t) = self.bind_head(s);
                match inner {
                    Some(i) => self.pattern(i, t),
                    None => {
                        self.term_at(None);
                        self.wildcard(t)
                    }
                }
                self.buf.end_length(l);
            }
            TPat::Test(test, t, inner) if matches!(self.pat_form(inner), Some(PatForm::Seq(_))) => self.seq_pattern(test, t, inner, scrut),
            // A tuple pattern past 22 elements is scalac's `TupleXXL(p1, ..., pn)` alone, which the
            // reader types as the tuple pattern it stands for (`typer/pat.rs`).
            TPat::Test(_, _, inner) if self.is_xxl_tuple_extraction(inner) => self.pattern(inner, scrut),
            TPat::Test(test, t, inner) => {
                let zt = self.w.zonk(t);
                if let Type::Class(c, _) = self.w.types.get(zt) {
                    if self.is_reflect_class(c) {
                        return self.reflect_type_test(c, t, inner, scrut);
                    }
                }
                if let Some(&inlined) = self.inlined_open.last() {
                    if self.w.prog.leaf_tests.contains_key(&test) {
                        self.inlined_leaf_tests.push((inlined, self.buf.addr()));
                    }
                }
                // A type variable (`case _: (h *: t)`) is the case's: `BIND` at its first place
                // in the type tree, which its references in the case name.
                let mut vars = Vec::new();
                self.pattern_type_vars(t, &mut vars);
                for p in vars {
                    self.bindings += 1;
                    self.tparams.push((p, TpRef::Direct(self.bindings)));
                    self.pattern_binds.push((p, self.bindings, false));
                }
                let l = self.open(TYPED);
                self.pattern(inner, t);
                self.pattern_tpt(t);
                self.buf.end_length(l);
            }
            // `_: p.type`, which the typer compares by identity: a path of a val.
            TPat::Equals(e, true) if self.singleton_path(e) => {
                let l = self.open(TYPED);
                self.wildcard(scrut);
                self.mark_tree();
                self.buf.byte(SINGLETONTPT);
                self.term(e);
                self.buf.end_length(l);
            }
            TPat::Equals(e, _) => self.term(e),
            TPat::Alt(subs) => {
                let subs: Vec<TPatId> = self.w.prog.pat_lists[subs.range()].to_vec();
                let l = self.open(ALTERNATIVE);
                for s in subs {
                    self.pattern(s, scrut);
                }
                self.buf.end_length(l);
            }
            TPat::Class(c, t, fields, subs) => {
                // A case class of a repeated last field matches it through `unapplySeq`.
                self.w.complete_class(c);
                let repeated = self.w.syms.class(c).ctor.first().and_then(|cl| cl.params.last()).map_or(false, |p| p.repeated);
                if repeated {
                    return self.repeated_class_pattern(c, t, fields, subs, scrut);
                }
                self.class_pattern(c, t, fields, subs, scrut)
            }
            TPat::Seq(..) => self.fail("a sequence pattern".to_string()),
            TPat::Unapply(_, call, inner) if matches!(self.w.prog.expr(call), TExpr::Js(t, _) if self.w.prog.strings[t.idx()] == "$quoteMatch") => self.quote_pattern(call, inner),
            TPat::Unapply(_, call, inner) => self.extractor_pattern(call, inner),
        }
    }

    /// A binder's `BIND`, its name and its type written, defined at its address; the type.
    fn bind_head(&mut self, s: SymId) -> (crate::tasty::write::buf::Slot, TypeId) {
        let addr = self.buf.addr();
        if self.binders_apart {
            // A pattern definition's binder: the case's own, apart from the val.
            self.case_binders.push((s, addr));
        } else {
            self.define(Key::Sym(s), addr);
        }
        let l = self.open(BIND);
        let n = self.simple_name(self.w.syms.sym(s).name);
        self.buf.nat(n as u64);
        let t = self.w.sig_of(s).ret;
        let t = self.w.zonk(t);
        self.ty(t);
        (l, t)
    }

    fn pat_form(&self, p: TPatId) -> Option<PatForm> {
        self.w.prog.capture.as_deref().and_then(|c| c.pat_of(p))
    }

    /// `List(a, b, rest*)`, the type test of the class `t` before the elements: `UNAPPLY` of the
    /// companion's `unapplySeq` at its own signature applied to the element type, the matched
    /// type, the elements' patterns and the rest's; ascribed `t` where the scrutinee is no `t`,
    /// as `typedUnApply` has it.
    fn seq_pattern(&mut self, test: crate::tir::TestId, t: TypeId, seq: TPatId, scrut: TypeId) {
        let t = self.w.zonk(t);
        let Type::Class(c, args) = self.w.types.get(t) else { return self.fail("a sequence pattern of no class".to_string()) };
        let Some(elem) = self.w.types.items(args).first().copied() else { return self.fail("a sequence pattern of no element type".to_string()) };
        let elem = self.w.zonk(elem);
        let scrut = self.w.zonk(scrut);
        let tested = self.tested_against(scrut, t);
        // `case Array(..)` on a scrutinee it tests: scalac's test is of `Array[T$1]`, a fresh
        // element type, which any array passes; `Array[Any]` would let no primitive array through.
        if tested && c == self.w.b.array {
            return self.fail("a sequence pattern of an array on a scrutinee it tests".to_string());
        }
        if tested {
            if let Some(&inlined) = self.inlined_open.last() {
                if self.w.prog.leaf_tests.contains_key(&test) {
                    self.inlined_leaf_tests.push((inlined, self.buf.addr()));
                }
            }
        }
        let typed = tested.then(|| self.open(TYPED));
        let l = self.open(UNAPPLY);
        if !self.unapply_seq_extractor(c, elem) {
            return;
        }
        self.ty(if tested { t } else { scrut });
        self.seq_elements(seq, elem);
        self.buf.end_length(l);
        if let Some(x) = typed {
            self.tpt(t);
            self.buf.end_length(x);
        }
    }

    /// Whether a scrutinee is tested against a pattern's type `t` it does not conform to.
    fn tested_against(&mut self, scrut: TypeId, t: TypeId) -> bool {
        !self.w.types.has_vars(scrut) && !self.w.types.has_vars(t) && {
            let mark = self.w.snapshot();
            let sub = self.w.is_sub(scrut, t);
            self.w.rollback(mark);
            !sub
        }
    }

    /// The `unapplySeq` of the sequence class `c`'s companion applied to the element type:
    /// scala-library's for a std class (`SeqFactory`'s as seen from the companion, `Array`'s),
    /// checked as a std member's key; the member the companion declares or inherits otherwise,
    /// selected as declared.
    fn unapply_seq_extractor(&mut self, c: ClassId, elem: TypeId) -> bool {
        let name = self.w.interner.intern("unapplySeq");
        if self.is_std_class(c) {
            let (owner, shape) = self.std_unapply_seq_key(c);
            let Some((params, result)) = sig_of_text(&shape).filter(|_| self.std_shape(&owner, "unapplySeq", &shape)) else { return false };
            let ta = self.open(TYPEAPPLY);
            let sl = self.open(SELECTIN);
            let n = self.names.signed("unapplySeq", None, &params, &result);
            self.buf.nat(n as u64);
            self.companion_path(c);
            self.companion_class_ref(c);
            self.buf.end_length(sl);
            self.tpt(elem);
            self.buf.end_length(ta);
            return true;
        }
        let member = self.class_info(c).companion.and_then(|o| {
            self.w.complete_class(o);
            let bases: Vec<ClassId> = self.w.syms.class(o).base_types.iter().map(|&(b, _)| b).collect();
            bases.into_iter().find_map(|b| {
                self.w.complete_class(b);
                self.w.syms.class(b).members.get(&name).copied()
            })
        });
        let Some(m) = member.filter(|&m| self.w.syms.alternatives(m).is_none() && self.w.sig_of(m).tparams.len() <= 1) else {
            self.fail("the unapplySeq of a sequence class's companion".to_string());
            return false;
        };
        let ta = (!self.w.sig_of(m).tparams.is_empty()).then(|| self.open(TYPEAPPLY));
        self.select(m, |p| p.companion_path(c));
        if let Some(x) = ta {
            self.tpt(elem);
            self.buf.end_length(x);
        }
        true
    }

    /// `C(a, xs*)` of a case class of a repeated last field: `UNAPPLY` of the companion's
    /// `unapplySeq`, the fixed fields' patterns, then the elements' and the rest's.
    fn repeated_class_pattern(&mut self, c: ClassId, t: TypeId, fields: ListRef, subs: ListRef, scrut: TypeId) {
        let fields: Vec<SymId> = self.w.prog.sym_list(fields).to_vec();
        let subs: Vec<TPatId> = self.w.prog.pat_lists[subs.range()].to_vec();
        let t = self.w.zonk(t);
        let targs: Vec<TypeId> = match self.w.types.get(t) {
            Type::Class(k, args) if k == c => self.w.types.items(args).to_vec(),
            _ => Vec::new(),
        };
        let (Some((&last_sub, fixed_subs)), Some(last)) = (subs.split_last(), self.w.syms.class(c).ctor.first().and_then(|cl| cl.params.last()).cloned()) else {
            return self.fail("a pattern of a case class of a repeated field".to_string());
        };
        if fields.len() != subs.len() || !matches!(self.pat_form(last_sub), Some(PatForm::Elements)) {
            return self.fail("a pattern of a case class of a repeated field".to_string());
        }
        let local = self.local_prefix();
        let cls = self.w.library_class_name(c, &local);
        // A program's case class has the identity `unapplySeq` of Scala 3; a product's and a
        // library's declare theirs.
        let result = if self.is_std_class(c) {
            return self.fail("a pattern of a std case class of a repeated field".to_string());
        } else if self.w.syms.class(c).def.is_none() || self.w.in_jar(self.w.syms.class(c).file) {
            let name = self.w.interner.intern("unapplySeq");
            let m = self.w.syms.class(c).companion.and_then(|o| {
                self.w.complete_class(o);
                self.w.syms.class(o).members.get(&name).copied()
            });
            match m.map(|m| self.member_signature(m)) {
                Some(Ok(Some((_, r)))) => r,
                None => cls.clone(),
                _ => return self.fail("the unapplySeq of a library's case class".to_string()),
            }
        } else {
            cls.clone()
        };
        let subst: Subst = {
            let own = self.w.syms.class(c).own_tparams().to_vec();
            own.into_iter().zip(targs.iter().copied()).collect()
        };
        let scrut = self.w.zonk(scrut);
        let tested = self.tested_against(scrut, t);
        let typed = tested.then(|| self.open(TYPED));
        let l = self.open(UNAPPLY);
        let ta = (!targs.is_empty()).then(|| self.open(TYPEAPPLY));
        let sl = self.open(SELECTIN);
        let mut sig = Vec::new();
        if !targs.is_empty() {
            sig.push(SigParam::Types(targs.len()));
        }
        sig.push(SigParam::Type(cls.clone()));
        let n = self.names.signed("unapplySeq", None, &sig, &result);
        self.buf.nat(n as u64);
        self.companion_path(c);
        self.companion_class_ref(c);
        self.buf.end_length(sl);
        if let Some(x) = ta {
            for &a in &targs {
                self.tpt(a);
            }
            self.buf.end_length(x);
        }
        self.ty(t);
        for (f, &p) in fields.iter().zip(fixed_subs) {
            let ft = self.w.sig_of(*f).ret;
            let ft = self.w.types.subst(ft, &subst);
            self.pattern(p, ft);
        }
        let elem = self.w.types.subst(last.ty, &subst);
        self.seq_elements(last_sub, elem);
        self.buf.end_length(l);
        if let Some(x) = typed {
            self.tpt(t);
            self.buf.end_length(x);
        }
    }

    /// The patterns of a sequence's elements, each against `elem`, then of its rest as scalac
    /// spells it: a named one `BIND(xs, Seq[T], TYPED(IDENT _*, <repeated>[T]))`, an anonymous
    /// one `TYPED(IDENT _, <repeated>[T])`.
    fn seq_elements(&mut self, seq: TPatId, elem: TypeId) {
        let TPat::Seq(items, rest) = self.w.prog.pats[seq.idx()] else { return self.fail("a sequence pattern".to_string()) };
        let items: Vec<TPatId> = self.w.prog.pat_lists[items.range()].to_vec();
        for p in items {
            self.pattern(p, elem);
        }
        let Some(r) = rest else { return };
        self.term_at(None);
        match self.w.prog.pats[r.idx()] {
            TPat::Wildcard => {
                let Some(seq_class) = self.w.seq_class() else { return self.fail("a rest pattern without the std's Seq".to_string()) };
                let seq_ty = self.w.types.class(seq_class, &[elem]);
                self.rest_ident("_", seq_ty, elem);
            }
            TPat::Bind(s, inner) if inner.map_or(true, |i| matches!(self.w.prog.pats[i.idx()], TPat::Wildcard)) => {
                let (l, t) = self.bind_head(s);
                self.rest_ident("_*", t, elem);
                self.buf.end_length(l);
            }
            _ => self.fail("a rest pattern of that shape".to_string()),
        }
    }

    fn rest_ident(&mut self, name: &str, seq_ty: TypeId, elem: TypeId) {
        self.term_at(None);
        let l = self.open(TYPED);
        self.term_at(None);
        self.buf.byte(IDENT);
        let n = self.names.simple(name);
        self.buf.nat(n as u64);
        self.ty(seq_ty);
        self.repeated_tpt(elem);
        self.buf.end_length(l);
    }

    /// The element type of a sequence type, `T` of a `Seq[T]` or what it extends.
    fn seq_elem(&mut self, t: TypeId) -> Option<TypeId> {
        let t = self.w.zonk(t);
        let seq = self.w.seq_class()?;
        match self.w.base_type(t, seq).map(|b| self.w.types.get(b)) {
            Some(Type::Class(_, args)) => self.w.types.items(args).first().copied(),
            _ => None,
        }
    }

    /// The type parameters `t` names that no definition in scope binds: a type pattern's
    /// variables, in the order they first appear.
    fn pattern_type_vars(&self, t: TypeId, out: &mut Vec<TParamId>) {
        match self.w.types.get(t) {
            Type::Param(p) if !self.tparams.iter().any(|&(q, _)| q == p) && !out.contains(&p) => out.push(p),
            Type::Class(_, args) => {
                for &a in self.w.types.items(args) {
                    self.pattern_type_vars(a, out);
                }
            }
            _ => {}
        }
    }

    fn binds_pattern_var(&self, t: TypeId) -> bool {
        match self.w.types.get(t) {
            Type::Param(p) => self.pattern_binds.iter().any(|&(q, _, written)| q == p && !written),
            Type::Class(_, args) => self.w.types.items(args).iter().any(|&a| self.binds_pattern_var(a)),
            _ => false,
        }
    }

    /// A type pattern's type tree, each of its variables' `BIND` at its first place
    /// (`APPLIEDtpt(*:, BIND h (TYPEBOUNDS ..) _, ..)`, as scalac's typer leaves it).
    fn pattern_tpt(&mut self, t: TypeId) {
        if !self.binds_pattern_var(t) {
            return self.tpt(t);
        }
        match self.w.types.get(t) {
            Type::Param(p) => {
                let i = self.pattern_binds.iter().position(|&(q, _, written)| q == p && !written).expect("an unwritten variable");
                self.pattern_binds[i].2 = true;
                let id = self.pattern_binds[i].1;
                let info = self.w.syms.tparam(p).clone();
                let at = self.buf.addr();
                self.define(Key::Binding(id), at);
                self.mark_tree();
                let l = self.open(BIND);
                let n = self.simple_name(info.name);
                self.buf.nat(n as u64);
                let bounds = self.buf.addr();
                self.bounds(&info);
                self.mark_tree();
                self.buf.byte(IDENT);
                let w = self.names.simple("_");
                self.buf.nat(w as u64);
                self.buf.byte(SHAREDTYPE);
                self.buf.reference(bounds);
                self.buf.end_length(l);
            }
            Type::Class(c, args) => {
                let args = self.w.types.items(args).to_vec();
                self.mark_tree();
                let l = self.open(APPLIEDTPT);
                self.mark_tree();
                self.class_typeref(c);
                for a in args {
                    self.pattern_tpt(a);
                }
                self.buf.end_length(l);
            }
            _ => self.tpt(t),
        }
    }

    /// Whether an identity pattern is a singleton type's: a val's path (an object's and an enum
    /// value's are stable identifiers the typer compares by identity as well).
    fn singleton_path(&self, e: TExprId) -> bool {
        match self.w.prog.expr(e) {
            TExpr::Local(s) | TExpr::Field(_, s) => self.w.syms.sym(s).kind == SymKind::Val,
            TExpr::Static(s) => self.w.syms.sym(s).kind == SymKind::Val,
            _ => false,
        }
    }

    pub(super) fn wildcard(&mut self, t: TypeId) {
        self.term_at(None);
        self.buf.byte(IDENT);
        let n = self.names.simple("_");
        self.buf.nat(n as u64);
        self.ty(t);
    }

    pub(super) fn throwable_type(&mut self) -> TypeId {
        match self.w.b.throwable {
            Some(c) => self.w.types.class(c, &[]),
            None => ANY,
        }
    }

    // ---- operators ------------------------------------------------------------------------

    fn prim(&mut self, e: TExprId, op: PrimOp, a: TExprId, b: TExprId, form: Option<Form>) {
        let (a, b, name) = match form {
            Some(Form::SuperOp(n)) => {
                let name = self.name(n);
                return self.universal(&name, |p| p.super_ref(), &[], &[b]);
            }
            Some(Form::Op(n)) => (a, b, self.name(n)),
            Some(Form::SwappedOp(n)) => (b, a, self.name(n)),
            None => (a, b, prim_name(op).to_string()),
            Some(f) => return self.unsupported_form(f),
        };
        let (lt, rt) = (self.operand_type(a), self.operand_type(b));
        let a = self.unpromoted(a);
        let b = self.unpromoted(b);
        if let Some(class) = self.compared_class(a) {
            self.receivers.insert(a, ());
            return self.compared_op(class, &name, a, b);
        }
        let (Some(lt), Some(rt)) = (lt, rt) else {
            return self.fail(format!("the operands' types of {}", name));
        };
        let numeric = self.numeric_class(lt);
        // A primitive operation's result is its overload's, a conversion of it the typer made
        // in place written around it (`unwritten_conversion`).
        let result = match (numeric, self.node_type(e)) {
            (Some(_), _) | (None, None) => self.op_result(op, lt, rt),
            (None, Some(t)) => self.w.widen_lit(t),
        };
        match (numeric, name.as_str()) {
            (Some(c), _) => self.builtin_op(c, &name, a, rt, b, result),
            (None, "==" | "!=" | "equals" | "eq" | "ne") => self.universal(&name, |p| p.term(a), &[], &[b]),
            (None, _) => self.fail(format!("the operator {} of a receiver", name)),
        }
    }

    /// The type of a primitive operation the typer made without one (`n += 1`'s `n + 1`): a
    /// comparison's `Boolean`, an arithmetic operation's the wider operand's, `Int` at least.
    fn op_result(&self, op: PrimOp, lt: TypeId, rt: TypeId) -> TypeId {
        use PrimOp::*;
        let b = &self.w.b;
        match op {
            Lt | Le | Gt | Ge | RefEq | RefNe | Eq | Ne | BoolAnd | BoolOr | BoolXor | BoolStrictAnd | BoolStrictOr => b.t_boolean,
            IntShl | IntShr | IntUshr | LongShl | LongShr | LongUshr => if lt == b.t_long { b.t_long } else { b.t_int },
            _ => {
                let rank = |t: TypeId| if t == b.t_double { 4 } else if t == b.t_float { 3 } else if t == b.t_long { 2 } else { 1 };
                match rank(lt).max(rank(rt)) {
                    4 => b.t_double,
                    3 => b.t_float,
                    2 => b.t_long,
                    _ => b.t_int,
                }
            }
        }
    }

    fn unpromoted(&self, x: TExprId) -> TExprId {
        match (self.w.prog.expr(x), self.records(x).form) {
            (TExpr::Unary(_, inner), Some(Form::Promotion)) => inner,
            _ => x,
        }
    }

    /// The class of a primitive type of scala: `Int`, `Boolean`, ...
    pub(super) fn numeric_class(&self, t: TypeId) -> Option<ClassId> {
        let b = &self.w.b;
        match self.w.types.get(t) {
            Type::Class(c, _) if [b.int, b.long, b.double, b.float, b.char, b.byte, b.short, b.boolean].contains(&c) => Some(c),
            _ => None,
        }
    }

    /// `a.op(b)` of a primitive class, its signature by the operand types: `Int.<(Long)`.
    fn builtin_op(&mut self, c: ClassId, name: &str, a: TExprId, rt: TypeId, b: TExprId, result: TypeId) {
        let Some(param) = self.primitive_sig_name(rt) else {
            return self.fail(format!("the operand of {}", name));
        };
        let Some(res) = self.primitive_sig_name(result) else {
            return self.fail(format!("the result of {}", name));
        };
        let l = self.open(APPLY);
        let sl = self.open(SELECTIN);
        let n = self.names.signed(name, None, &[SigParam::Type(param)], &res);
        self.buf.nat(n as u64);
        self.term(a);
        self.class_typeref(c);
        self.buf.end_length(sl);
        self.term(b);
        self.buf.end_length(l);
    }

    fn primitive_sig_name(&mut self, t: TypeId) -> Option<String> {
        let local = self.local_prefix();
        let c = match self.w.types.get(t) {
            Type::Class(c, _) => c,
            _ => return None,
        };
        Some(self.w.library_class_name(c, &local))
    }

    fn unary(&mut self, e: TExprId, op: UnOp, a: TExprId) {
        // A conversion the typer made of fewer steps than scalac's (`c.toDouble` of a `Char`
        // as its `toInt`, a double on JavaScript): the one to the node's type.
        let converts = !matches!(op, UnOp::IntNeg | UnOp::LongNeg | UnOp::DoubleNeg | UnOp::FloatNeg | UnOp::BoolNot | UnOp::IntNot | UnOp::LongNot);
        if converts {
            if let Some(to) = self.node_type(e).map(|t| self.w.widen_lit(t)).and_then(|t| self.numeric_class(t)) {
                let name = format!("to{}", self.name(self.w.syms.class(to).name));
                self.buf.byte(SELECT);
                let n = self.names.simple(&name);
                self.buf.nat(n as u64);
                return self.term(a);
            }
        }
        let name = match op {
            UnOp::IntNeg | UnOp::LongNeg | UnOp::DoubleNeg | UnOp::FloatNeg => "unary_-",
            UnOp::BoolNot => "unary_!",
            UnOp::IntNot | UnOp::LongNot => "unary_~",
            UnOp::IntToLong | UnOp::CharToLong | UnOp::DoubleToLong | UnOp::FloatToLong => "toLong",
            UnOp::LongToDouble | UnOp::FloatToDouble | UnOp::IntToDouble => "toDouble",
            UnOp::LongToInt | UnOp::DoubleToInt | UnOp::CharToInt | UnOp::FloatToInt | UnOp::ByteToInt | UnOp::ShortToInt => "toInt",
            UnOp::IntToChar => "toChar",
            UnOp::IntToByte => "toByte",
            UnOp::IntToShort | UnOp::ByteToShort => "toShort",
            UnOp::IntToFloat | UnOp::LongToFloat | UnOp::DoubleToFloat => "toFloat",
        };
        self.buf.byte(SELECT);
        let n = self.names.simple(name);
        self.buf.nat(n as u64);
        self.term(a);
    }

    /// `Int.int2long(i)`: the companion's conversion named by the two types.
    fn widening(&mut self, e: TExprId, _op: UnOp, a: TExprId) {
        // A chain of conversions captured as one widening (`IntToDouble(CharToInt(c))` for a
        // `Char` widened to a `Double`) is the one conversion of its innermost operand,
        // `Char.char2double(c)`, as scalac writes it: the node between has no type of its own. A
        // widening of a widened value (`(i: Float)` widened to a `Double`) keeps both, with the
        // inner one's rounding: the inner node has its type.
        let mut a = a;
        while let TExpr::Unary(_, inner) = self.w.prog.expr(a) {
            if !matches!(self.records(a).form, Some(Form::Widening)) || self.node_type(a).is_some() {
                break;
            }
            a = inner;
        }
        let from = self.node_type(a).map(|t| self.w.widen_lit(t));
        let to = self.node_type(e).map(|t| self.w.widen_lit(t));
        let (Some(from), Some(to)) = (from, to) else { return self.fail("a widening's types".to_string()) };
        let (Some(fc), Some(tc)) = (self.numeric_class(from), self.numeric_class(to)) else {
            return self.fail("a widening of no primitive".to_string());
        };
        self.widen_call(fc, tc, |p| p.term(a));
    }

    /// `From.from2to(x)`, the conversion of a number in the companion of its class.
    fn widen_call(&mut self, fc: ClassId, tc: ClassId, a: impl FnOnce(&mut Self)) {
        let (fname, tname) = (self.name(self.w.syms.class(fc).name), self.name(self.w.syms.class(tc).name));
        let conv = format!("{}2{}", fname.to_lowercase(), tname.to_lowercase());
        let l = self.open(APPLY);
        let sl = self.open(SELECTIN);
        let n = self.names.signed(&conv, None, &[SigParam::Type(format!("scala.{}", fname))], &format!("scala.{}", tname));
        self.buf.nat(n as u64);
        self.term_at(None);
        self.buf.byte(TERMREF);
        let m = self.names.simple(&fname);
        self.buf.nat(m as u64);
        self.package_path("scala");
        self.buf.byte(TYPEREF);
        let mc = self.names.object_class(&fname);
        self.buf.nat(mc as u64);
        self.package_path("scala");
        self.buf.end_length(sl);
        a(self);
        self.buf.end_length(l);
    }

    /// `e.asInstanceOf[T]`.
    fn cast_to(&mut self, t: TypeId, e: impl FnOnce(&mut Self)) {
        self.universal("asInstanceOf", e, &[t], &[]);
    }

    /// A member of `Any` or of `AnyRef` (`java.lang.Object`), which teq's types hold no symbol
    /// of: `recv.name[targs](args)` as scalac 3.8.4 declares it.
    fn universal(&mut self, name: &str, recv: impl FnOnce(&mut Self), targs: &[TypeId], args: &[TExprId]) {
        let Some((pkg, owner, tps, params, result, clause)) = universal_member(name) else { return self.fail(format!("the member {} of Any", name)) };
        if args.len() != params.len() || targs.len() != tps {
            return self.fail(format!("{} of Any with {} arguments", name, args.len()));
        }
        let sig = universal_sig(tps, params);
        let shape = if !clause && tps == 0 { super::shapes::by_name_shape(result) } else { super::shapes::sig_text(&sig, result) };
        if !self.std_shape(&format!("{}.{}", pkg, owner), name, &shape) {
            return;
        }
        let call_at = self.buf.addr();
        let app = clause.then(|| self.open(APPLY));
        let ta = (tps > 0).then(|| self.open(TYPEAPPLY));
        if clause && tps == 0 && args.is_empty() {
            self.mark_auto_applied(call_at);
        }
        if !clause && tps == 0 {
            self.buf.byte(SELECT);
            let n = self.names.simple(name);
            self.buf.nat(n as u64);
            recv(self);
        } else {
            let sl = self.open(SELECTIN);
            let n = self.names.signed(name, None, &sig, result);
            self.buf.nat(n as u64);
            recv(self);
            self.external_typeref(pkg, owner);
            self.buf.end_length(sl);
        }
        if let Some(l) = ta {
            for &t in targs {
                self.tpt(t);
            }
            self.buf.end_length(l);
        }
        if let Some(l) = app {
            for &a in args {
                self.term(a);
            }
            self.buf.end_length(l);
        }
    }

    /// A chain of `+` of strings, left associated, each rendering the operand it renders.
    /// `a + b` of a concatenation: `String.+(Any)` where the left operand is a string, the
    /// number's or char's `+(String)` where it renders one.
    fn concat(&mut self, items: &[TExprId]) {
        let [a, b] = items else {
            return self.fail(format!("a concatenation of {} operands", items.len()));
        };
        let (a, b) = (self.rendered(*a), self.rendered(*b));
        let lt = self.node_type(a).map(|t| self.w.widen_lit(t));
        let string = self.w.b.t_string;
        let left_string = lt.map_or(false, |t| t == string || self.w.is_same(t, string));
        let (owner_pkg, owner, param): (String, String, &str) = if left_string {
            ("java.lang".into(), "String".into(), "java.lang.Object")
        } else {
            match lt.and_then(|t| self.numeric_class(t)) {
                Some(c) if c != self.w.b.boolean => ("scala".into(), self.name(self.w.syms.class(c).name), "java.lang.String"),
                _ => return self.fail("a concatenation whose left operand is no string or number".to_string()),
            }
        };
        let l = self.open(APPLY);
        let sl = self.open(SELECTIN);
        let n = self.names.signed("+", None, &[SigParam::Type(param.to_string())], "java.lang.String");
        self.buf.nat(n as u64);
        self.term(a);
        self.external_typeref(&owner_pkg, &owner);
        self.buf.end_length(sl);
        self.term(b);
        self.buf.end_length(l);
    }

    fn rendered(&self, x: TExprId) -> TExprId {
        match self.w.prog.expr(x).rendering() {
            Some((inner, _)) => inner,
            None => x,
        }
    }

    // ---- helpers --------------------------------------------------------------------------

    /// The member `name` a class declares or inherits, the first alternative.
    fn member_of_class(&mut self, c: ClassId, name: &str) -> Option<SymId> {
        let n = self.w.interner.intern(name);
        let t = self.w.types.class(c, &[]);
        self.w.find_member(t, n).map(|(s, _)| s).or_else(|| self.w.syms.class(c).members.get(&n).copied())
    }

    /// An expansion of inline calls, innermost first: an ordinary call is pickled as the call
    /// (scalac's `Pickler` runs before `Inlining`), a transparent one's expansion as `INLINED`,
    /// a library's or an upstream product's as a program's own.
    fn inline_chain(&mut self, e: TExprId, recs: &NodeRecords) {
        let calls = recs.inline_calls.clone();
        self.inline_level(e, &calls, recs);
    }

    /// The outermost of `calls` (innermost first) the expansion `e` stands for: an ordinary
    /// call as the call, a transparent one as `INLINED(expansion, origin, bindings)`, the origin
    /// the top-level class of the inlined method (`PostTyper` 565 to 570).
    fn inline_level(&mut self, e: TExprId, calls: &[crate::tir::capture::InlineCall], recs: &NodeRecords) {
        let Some((outer, inner)) = calls.split_last() else {
            let rest = NodeRecords { inline_calls: Vec::new(), ..recs.clone() };
            return self.within(Some(e), |p| p.node(e, &rest));
        };
        let outer = outer.clone();
        if !outer.transparent {
            return self.within_site(outer.site, e, |p| {
                let targs = (!p.w.types.items(outer.targs).is_empty()).then_some(outer.targs);
                let unqualified = outer.site.map_or(false, |(f, s)| p.text_names(f, s, outer.callee));
                match outer.recv {
                    Some(r) if unqualified && matches!(p.w.prog.expr(r), TExpr::This) && p.w.prog.span_of(r).is_none() => {
                        p.applied(outer.callee, targs, &outer.args, |p| p.identifier(outer.callee), Some(Qual::Expr(r)))
                    }
                    Some(r) => p.applied(outer.callee, targs, &outer.args, |p| p.select(outer.callee, |p| p.term(r)), Some(Qual::Expr(r))),
                    None if unqualified => p.applied(outer.callee, targs, &outer.args, |p| p.identifier(outer.callee), Some(Qual::Owner)),
                    None => p.call(outer.callee, Qual::Owner, targs, &outer.args),
                }
            });
        }
        // The `INLINED` at the call's place, its expansion at the body's (a macro's: the
        // splice's), as scalac's pickle has them.
        self.within_site(outer.site, e, |p| {
            p.inlined_open.push(p.inlined_callees.len());
            p.inlined_callees.push((p.buf.addr(), outer.callee));
            let l = p.open(INLINED);
            // The expansion, its bindings the block the typer made of them.
            let bindings: Vec<TStmt> = match (outer.binds, inner.is_empty(), p.w.prog.expr(e)) {
                (true, true, TExpr::Block(stmts, res)) => {
                    let stmts = p.w.prog.stmt_list(stmts).to_vec();
                    p.term(res);
                    stmts
                }
                // The expansion is in the callee's source, at its stored body's place, where the
                // `INLINED` around it is at the call's (one node of teq's stands for both); a
                // macro's at its quote's, but a constant, or a tree reflection made without a
                // place, at the splice's, where scalac's folding of the splice's own `INLINED`
                // leaves the one and its expansion's position puts the other.
                _ if inner.is_empty() => {
                    let rest = NodeRecords { inline_calls: Vec::new(), ..recs.clone() };
                    let body = p.w.inline_definitions.get(&outer.callee).and_then(|d| d.body);
                    let placed = p.w.prog.span_of(e).map_or(false, |at| Some(at) != outer.site);
                    let quoted = body.map_or(false, |b| matches!(p.w.prog.expr(b), TExpr::Splice(_))) && !p.w.prog.expr(e).is_constant() && placed;
                    let body_at = body.and_then(|b| p.w.prog.span_of(b));
                    // A root the expansion took from its call site (`id(x)`'s `x`) keeps the
                    // call site's place, which a downstream's expansion over a converted body
                    // gives it too (scalac's call-less `INLINED` at the body's around it is not
                    // written).
                    let from_site = p.w.prog.span_of(e).map_or(false, |at| p.is_own_source(at.0) && body_at.map(|(g, _)| g) != Some(at.0) && Some(at) != outer.site);
                    match body_at {
                        Some((file, span)) if p.is_own_source(file) && !quoted && !from_site => p.within_at(file, span, |p| p.node(e, &rest)),
                        _ => p.within(Some(e), |p| p.node(e, &rest)),
                    }
                    Vec::new()
                }
                _ => {
                    p.inline_level(e, inner, recs);
                    Vec::new()
                }
            };
            p.inline_origin(outer.callee);
            for st in &bindings {
                match *st {
                    TStmt::Val(v, x) => p.local_val(v, x),
                    TStmt::Fun(f) => p.local_def(f),
                    _ => p.fail("an inline expansion's binding that is no definition".to_string()),
                }
            }
            p.inlined_open.pop();
            p.buf.end_length(l);
        })
    }

    /// Records where `e`, about to be written inside the `INLINED` `inlined`, is what the
    /// expansion took from its call site (kind 8, version 2): a leaf, or a type test of the
    /// call's type arguments.
    fn note_expansion_leaf(&mut self, inlined: usize, e: TExprId) {
        if self.w.prog.is_leaf(e) {
            self.inlined_leaves.push((inlined, self.buf.addr()));
        }
        if let TExpr::TypeTest(_, test) = self.w.prog.expr(e) {
            if self.w.prog.leaf_tests.contains_key(&test) {
                self.inlined_leaf_tests.push((inlined, self.buf.addr()));
            }
        }
    }

    /// The top-level class an inlined method is a member of, as `INLINED` names its origin.
    pub(super) fn inline_origin(&mut self, m: SymId) {
        let mut owner = self.w.syms.sym(m).owner;
        let mut top: Option<ClassId> = None;
        while let Owner::Class(c) = owner {
            top = Some(c);
            owner = self.w.syms.class(c).owner;
        }
        self.term_at(None);
        match top {
            Some(c) => {
                let info = self.class_info(c);
                let name = if info.kind == ClassKind::Object { self.names.object_class(&self.name(info.name)) } else { self.simple_name(info.name) };
                self.buf.byte(IDENTTPT);
                self.buf.nat(name as u64);
                if info.kind == ClassKind::Object {
                    self.module_class_typeref(c);
                } else {
                    self.class_typeref(c);
                }
            }
            None => {
                let file = self.w.syms.sym(m).file;
                let stem = super::super::file_stem(&self.w.files.as_slice()[file.0 as usize].path);
                let holder = format!("{}$package", stem);
                let n = self.names.object_class(&holder);
                self.buf.byte(IDENTTPT);
                self.buf.nat(n as u64);
                self.package_object_class_ref(m);
            }
        }
    }

    /// A call the typer replaced by what it computes: a tuple's member, a function applied.
    fn builtin_call(&mut self, e: TExprId, b: &crate::tir::capture::BuiltinCall) {
        let name = self.name(b.name);
        let recv_ty = match self.node_type(b.recv) {
            Some(t) => self.w.widen_lit(t),
            None => return self.fail(format!("the receiver of the builtin {}", name)),
        };
        if name == "apply" && self.w.types.get(recv_ty) != Type::Any {
            if let Type::Class(c, _) = self.w.types.get(recv_ty) {
                if self.w.is_function_class(c) || self.w.is_context_function_class(c) {
                    return self.function_apply(b.recv, &b.args);
                }
            }
        }
        // The members of scala-library's `Tuple`, inline methods over `This >: this.type`.
        let arg_ty = b.args.first().and_then(|&a| self.node_type(a)).map(|t| self.w.widen_lit(t));
        let (tparams, params, result): (Vec<TypeId>, &[&str], &str) = match (name.as_str(), b.args.len()) {
            ("head", 0) => (vec![recv_ty], &[], "java.lang.Object"),
            ("tail", 0) => (vec![recv_ty], &[], "scala.Product"),
            ("size", 0) => (vec![recv_ty], &[], "scala.Int"),
            ("++", 1) => (vec![recv_ty], &["scala.Product"], "scala.Product"),
            ("*:", 1) => match arg_ty {
                Some(h) => (vec![h, recv_ty], &["java.lang.Object"], "scala.Product"),
                None => return self.fail("the element of *:".to_string()),
            },
            (":*", 1) => match arg_ty {
                Some(l) => (vec![recv_ty, l], &["java.lang.Object"], "scala.Product"),
                None => return self.fail("the element of :*".to_string()),
            },
            ("last", 0) => (vec![recv_ty], &[], "java.lang.Object"),
            ("init" | "reverse", 0) => (vec![recv_ty], &[], "scala.Product"),
            ("take" | "drop", 1) => (vec![recv_ty], &["scala.Int"], "scala.Product"),
            ("splitAt", 1) => (vec![recv_ty], &["scala.Int"], "scala.Tuple2"),
            _ => return self.fail(format!("the tuple's builtin {}", name)),
        };
        let _ = e;
        let app = (!params.is_empty()).then(|| self.open(APPLY));
        let ta = self.open(TYPEAPPLY);
        let sl = self.open(SELECTIN);
        let mut sig = vec![SigParam::Types(tparams.len())];
        sig.extend(params.iter().map(|p| SigParam::Type(p.to_string())));
        let n = self.names.signed(&name, None, &sig, result);
        self.buf.nat(n as u64);
        self.term(b.recv);
        self.external_typeref("scala", "Tuple");
        self.buf.end_length(sl);
        for t in tparams {
            self.tpt(t);
        }
        self.buf.end_length(ta);
        if let Some(l) = app {
            for &a in &b.args {
                self.term(a);
            }
            self.buf.end_length(l);
        }
    }

    /// `super` of the class being written.
    fn super_ref(&mut self) {
        let l = self.open(SUPER);
        self.term_at(None);
        self.this_ref();
        self.buf.end_length(l);
    }

    /// `super`, `super[C]` with its qualifier; the superclass chain's (teq's `super[C]` of the
    /// superclass) as `super[C]` of the superclass, which a plain `super` of a class mixing in
    /// traits is not.
    fn super_of(&mut self, t: crate::tir::SuperTarget) {
        let c = match t {
            crate::tir::SuperTarget::Class(c) => c,
            crate::tir::SuperTarget::Chain => match self.superclass_of_enclosing() {
                Some(c) => c,
                None => return self.super_ref(),
            },
            crate::tir::SuperTarget::Mixin(_) => return self.super_ref(),
        };
        let l = self.open(SUPER);
        self.term_at(None);
        self.this_ref();
        self.mark_tree();
        self.buf.byte(IDENTTPT);
        let n = self.simple_name(self.w.syms.class(c).name);
        self.buf.nat(n as u64);
        self.class_typeref(c);
        self.buf.end_length(l);
    }

    /// The superclass of the class being written where it mixes in traits, which a plain
    /// `super` would pass over.
    fn superclass_of_enclosing(&mut self) -> Option<ClassId> {
        let &c = self.enclosing.last()?;
        let parents = self.w.syms.class(c).parents.clone();
        let first = parents.first().copied()?;
        let Type::Class(k, _) = self.w.types.get(first) else { return None };
        let traits = parents.iter().skip(1).any(|&p| matches!(self.w.types.get(p), Type::Class(t, _) if self.w.syms.class(t).kind == ClassKind::Trait));
        (traits && self.w.syms.class(k).kind != ClassKind::Trait).then_some(k)
    }

    /// The selection of the super accessor of the trait `t` for its member `s` on the trait's
    /// `this`: `QUALTHIS(T).super$T$$m`.
    fn super_accessor_select(&mut self, t: ClassId, s: SymId) {
        let full = self.owner_full_name(self.w.syms.class(t).owner, &self.name(self.w.syms.class(t).name));
        let member = self.name(self.w.syms.sym(s).name);
        let base = self.super_accessor_name(&full, &member);
        match self.member_signature(s) {
            Ok(None) => {
                self.buf.byte(SELECT);
                self.buf.nat(base as u64);
                self.term_at(None);
                self.qual_this(t);
            }
            Ok(Some((params, result))) => {
                let l = self.open(SELECTIN);
                let k = self.names.signed_ref(base, &params, &result);
                self.buf.nat(k as u64);
                self.term_at(None);
                self.qual_this(t);
                self.class_typeref(t);
                self.buf.end_length(l);
            }
            Err(kind) => self.fail(format!("the erasure of a {} in a signature", kind)),
        }
    }

    /// `!true` folded to `false`: the operator applied to the literal it inverts.
    fn folded(&mut self, e: TExprId, node: TExpr, op: Name) {
        let name = self.name(op);
        match (name.as_str(), node) {
            ("unary_!", TExpr::Bool(v)) => {
                self.buf.byte(SELECT);
                let n = self.names.simple("unary_!");
                self.buf.nat(n as u64);
                self.term_at(None);
                self.buf.byte(if v { FALSECONST } else { TRUECONST });
            }
            ("unary_-", TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_)) => {
                let negated = match node {
                    TExpr::Int(i) => TExpr::Int(i.wrapping_neg()),
                    TExpr::Long(l) => TExpr::Long(l.wrapping_neg()),
                    TExpr::Double(d) => TExpr::Double(-d),
                    other => other,
                };
                self.buf.byte(SELECT);
                let n = self.names.simple("unary_-");
                self.buf.nat(n as u64);
                self.term_at(None);
                self.literal(e, negated);
            }
            _ => self.fail(format!("a folded {}", name)),
        }
    }

    /// A member a std helper stands for, which takes the receiver first: `c.productIterator`.
    fn member_helper(&mut self, m: SymId, args: &[TExprId], targs: Option<TList>) {
        let Some((&r, rest)) = args.split_first() else { return self.fail("a member's helper without a receiver".to_string()) };
        self.applied(m, targs, rest, |p| p.select(m, |p| p.term(r)), Some(Qual::Expr(r)));
    }

    /// A builtin member of `Any` or `AnyRef` the typer wrote as a template: `x.getClass[T]()`.
    fn builtin_member(&mut self, e: TExprId, n: Name, args: &[TExprId], on_super: bool) {
        let name = self.name(n);
        let Some((&r, rest)) = args.split_first() else { return self.fail(format!("{} without a receiver", name)) };
        if n == crate::names::ORDINAL && rest.is_empty() && !on_super {
            let enum_class = self.receiver_class(r);
            return match enum_class {
                Some(e) => self.ordinal_of(e, |p| p.term(r)),
                None => self.fail("the enum of ordinal".to_string()),
            };
        }
        let targs: Vec<TypeId> = match name.as_str() {
            "getClass" => {
                let t = self.node_type(r).map(|t| self.w.widen_lit(t)).unwrap_or(ANY);
                vec![t]
            }
            "synchronized" => vec![self.node_type(e).unwrap_or(ANY)],
            _ => Vec::new(),
        };
        if on_super {
            self.universal(&name, |p| p.super_ref(), &targs, rest)
        } else {
            self.universal(&name, |p| p.term(r), &targs, rest)
        }
    }

    /// `x.synchronized { body }`, which the typer wrote as the block of its receiver and body,
    /// and a primitive's `x.getClass`, the block of its receiver and the class.
    fn builtin_block(&mut self, e: TExprId, n: Name, stmts: ListRef, res: TExprId) {
        let name = self.name(n);
        let stmts: Vec<TStmt> = self.w.prog.stmt_list(stmts).to_vec();
        match (name.as_str(), stmts.as_slice()) {
            ("synchronized", [TStmt::Expr(r)]) => {
                let r = *r;
                let t = self.node_type(e).unwrap_or(ANY);
                self.universal("synchronized", |p| p.term(r), &[t], &[res])
            }
            ("getClass", [TStmt::Expr(r)]) => {
                let r = *r;
                let t = self.node_type(r).map(|t| self.w.widen_lit(t)).unwrap_or(ANY);
                self.universal("getClass", |p| p.term(r), &[t], &[])
            }
            _ => self.fail(format!("the block of {}", name)),
        }
    }

    /// The member of an enum's companion the typer synthesizes: `Color.values`,
    /// `Color.valueOf(name)`, the enum the node's type or its array's element.
    fn enum_member(&mut self, e: TExprId, n: Name, args: &[TExprId]) {
        let name = self.name(n);
        let t = self.node_type(e).map(|t| self.w.widen_lit(t));
        let mut enum_class = None;
        if let Some(t) = t {
            if let Type::Class(c, targs) = self.w.types.get(t) {
                enum_class = Some(c);
                if c == self.w.b.array {
                    if let Some(&el) = self.w.types.items(targs).first() {
                        if let Type::Class(k, _) = self.w.types.get(el) {
                            enum_class = Some(k);
                        }
                    }
                }
            }
        }
        let Some(ec) = enum_class.filter(|&c| self.w.syms.class(c).kind == ClassKind::Enum) else {
            return self.fail(format!("the enum of {}", name));
        };
        let enum_ty = self.enum_wild_type(ec);
        let string = self.w.b.t_string;
        let int = self.w.b.t_int;
        let (params, arity): (Vec<TypeId>, usize) = match name.as_str() {
            "values" => (Vec::new(), 0),
            "valueOf" => (vec![string], 1),
            "fromOrdinal" => (vec![int], 1),
            _ => return self.fail(format!("the enum's member {}", name)),
        };
        // The arguments follow teq's array of the values.
        let given: Vec<TExprId> = args.iter().skip(1).take(arity).copied().collect();
        if given.len() != arity {
            return self.fail(format!("the arguments of {}", name));
        }
        let local = self.local_prefix();
        let res = self.w.library_class_name(ec, &local);
        let app = (arity > 0).then(|| self.open(APPLY));
        if arity > 0 {
            let sl = self.open(SELECTIN);
            let ps: Vec<SigParam> = params.iter().map(|&p| SigParam::Type(self.primitive_or_class_name(p))).collect();
            let k = self.names.signed(&name, None, &ps, &res);
            self.buf.nat(k as u64);
            self.companion_path(ec);
            self.companion_class_ref(ec);
            self.buf.end_length(sl);
        } else if self.is_java_enum(ec) && self.w.syms.class(ec).def.is_none() {
            // A library's enum over `java.lang.Enum`, whose companion scalac's unpickler gives a
            // Java enum's `values()`.
            let l = self.open(APPLY);
            let sl = self.open(SELECTIN);
            let k = self.names.signed(&name, None, &[], &format!("{}[]", res));
            self.buf.nat(k as u64);
            self.companion_path(ec);
            self.companion_class_ref(ec);
            self.buf.end_length(sl);
            self.buf.end_length(l);
        } else {
            self.buf.byte(SELECT);
            let k = self.names.simple(&name);
            self.buf.nat(k as u64);
            self.companion_path(ec);
        }
        let _ = enum_ty;
        if let Some(l) = app {
            for g in given {
                self.term(g);
            }
            self.buf.end_length(l);
        }
    }

    fn primitive_or_class_name(&mut self, t: TypeId) -> String {
        let local = self.local_prefix();
        match self.w.types.get(t) {
            Type::Class(c, _) => self.w.library_class_name(c, &local),
            _ => "java.lang.Object".to_string(),
        }
    }

    /// `StringContext.apply(parts*).kind(args*)`, the parts as written.
    fn interpolation(&mut self, kind: Name, parts: TList, items: &[TExprId]) {
        let kind = self.name(kind);
        if !matches!(kind.as_str(), "s" | "raw") {
            return self.fail(format!("the interpolator {}", kind));
        }
        // The parts are the strings the typer typed none of; the rest are the arguments.
        let args: Vec<TExprId> = items.iter().filter(|&&x| !(matches!(self.w.prog.expr(x), TExpr::Str(_)) && self.w.prog.type_of(x).is_none())).map(|&x| self.rendered(x)).collect();
        let parts: Vec<TypeId> = self.w.types.items(parts).to_vec();
        if args.len() + 1 != parts.len() {
            return self.fail("an interpolation's arguments".to_string());
        }
        let string = self.w.b.t_string;
        let l = self.open(APPLY);
        let sl = self.open(SELECTIN);
        let n = self.names.signed(&kind, None, &[SigParam::Type("scala.collection.immutable.Seq".into())], "java.lang.String");
        self.buf.nat(n as u64);
        self.term_at(None);
        let a = self.open(APPLY);
        let al = self.open(SELECTIN);
        let m = self.names.signed("apply", None, &[SigParam::Type("scala.collection.immutable.Seq".into())], "scala.StringContext");
        self.buf.nat(m as u64);
        self.term_at(None);
        self.buf.byte(TERMREF);
        let sc = self.names.simple("StringContext");
        self.buf.nat(sc as u64);
        self.package_path("scala");
        self.buf.byte(TYPEREF);
        let scm = self.names.object_class("StringContext");
        self.buf.nat(scm as u64);
        self.package_path("scala");
        self.buf.end_length(al);
        self.repeated_of(string, |p| {
            for &part in &parts {
                p.term_at(None);
                p.ty(part);
            }
        });
        self.buf.end_length(a);
        self.external_typeref("scala", "StringContext");
        self.buf.end_length(sl);
        self.repeated_of(ANY, |p| {
            for &x in &args {
                p.term(x);
            }
        });
        self.buf.end_length(l);
    }

    /// `(elems*: T*)`: a sequence literal passed to a repeated parameter.
    fn repeated_of(&mut self, elem: TypeId, elems: impl FnOnce(&mut Self)) {
        self.term_at(None);
        let tl = self.open(TYPED);
        self.term_at(None);
        let r = self.open(REPEATED);
        self.tpt(elem);
        elems(self);
        self.buf.end_length(r);
        self.repeated_tpt(elem);
        self.buf.end_length(tl);
    }

    /// `c.copy[T..](given, c.copy$default$N[T..])` of a case class: the receiver the block's
    /// first value, the arguments the `new`'s, the fields read off the receiver its defaults;
    /// a receiver that is no path bound to the block's val first, as scalac's `liftApp` has it.
    fn case_copy(&mut self, _e: TExprId, stmts: ListRef, res: TExprId, _targs: Option<TList>) {
        let stmts: Vec<TStmt> = self.w.prog.stmt_list(stmts).to_vec();
        let (TStmt::Val(binder, recv), TExpr::New(c, args)) = (stmts.first().copied().unwrap_or(TStmt::Expr(res)), self.w.prog.expr(res)) else {
            return self.fail("a case class copy's shape".to_string());
        };
        let lifted = !self.is_path(recv);
        let receiver = |p: &mut Self| if lifted { p.term_at(Some(recv)); p.local_ref(binder) } else { p.term(recv) };
        let args = self.w.prog.expr_list(args).to_vec();
        // The temporaries of named arguments, before the copy in evaluation order.
        let temporaries: Vec<TStmt> = if lifted { stmts.clone() } else { stmts[1..].to_vec() };
        let block = (!temporaries.is_empty()).then(|| self.open(BLOCK));
        let new_recs = self.records(res);
        let targs: Vec<TypeId> = new_recs.targs.map(|l| self.w.types.items(l).to_vec()).unwrap_or_default();
        self.w.complete_class(c);
        let info = self.class_info(c);
        let Some(first) = info.ctor.first().cloned() else { return self.fail("a copy of a class without parameters".to_string()) };
        // `copy` takes the constructor's clauses: the first the copy's own, the later ones
        // (a context bound's evidence) as the constructor call passes them.
        let sizes: Vec<usize> = info.ctor.iter().map(|cl| cl.params.len()).collect();
        if args.len() != sizes.iter().sum::<usize>() {
            return self.fail("a copy's arguments".to_string());
        }
        let local = self.local_prefix();
        let Some((params, result)) = self.w.pickled_ctor_signature(c, &local) else { return self.fail("a copy's signature".to_string()) };
        let params = sig_params(params);
        let later: Vec<crate::tasty::write::buf::Slot> = sizes[1..].iter().map(|_| self.open(APPLY)).collect();
        let l = self.open(APPLY);
        let ta = (!targs.is_empty()).then(|| self.open(TYPEAPPLY));
        let sl = self.open(SELECTIN);
        let n = self.names.signed("copy", None, &params, &result);
        self.buf.nat(n as u64);
        receiver(self);
        self.class_typeref(c);
        self.buf.end_length(sl);
        if let Some(t) = ta {
            for &x in &targs {
                self.tpt(x);
            }
            self.buf.end_length(t);
        }
        for (i, &a) in args.iter().enumerate().take(first.params.len()) {
            let reads_binder = matches!(self.w.prog.expr(a), TExpr::Field(r, _) if matches!(self.w.prog.expr(r), TExpr::Local(b) if b == binder));
            if !reads_binder {
                self.term(a);
                continue;
            }
            // The receiver's field: the copy's default getter.
            self.term_at(Some(a));
            let ga = (!targs.is_empty()).then(|| self.open(TYPEAPPLY));
            let g = self.names.default_getter("copy", i as u32);
            if targs.is_empty() {
                self.buf.byte(SELECT);
                self.buf.nat(g as u64);
                receiver(self);
            } else {
                let gl = self.open(SELECTIN);
                let ty = first.params[i].ty;
                let r = match self.w.pickled_default_signature(targs.len(), &[first.clone()], i, &local) {
                    Some((_, r)) => r,
                    None => return self.fail("a copy default's signature".to_string()),
                };
                let _ = ty;
                let k = self.names.signed_ref(g, &[SigParam::Types(targs.len())], &r);
                self.buf.nat(k as u64);
                receiver(self);
                self.class_typeref(c);
                self.buf.end_length(gl);
            }
            if let Some(t) = ga {
                for &x in &targs {
                    self.tpt(x);
                }
                self.buf.end_length(t);
            }
        }
        self.buf.end_length(l);
        let mut at = first.params.len();
        for (i, &k) in sizes[1..].iter().enumerate() {
            for &a in &args[at..at + k] {
                self.term(a);
            }
            at += k;
            self.buf.end_length(later[later.len() - 1 - i]);
        }
        if let Some(b) = block {
            self.statements(&temporaries);
            self.buf.end_length(b);
        }
    }

    /// `C(subs)` of a case class: the companion's `unapply` (synthesized), the pattern's type,
    /// the fields' patterns.
    /// An extractor pattern `Obj(p..)` of an object's `unapply` (`Varargs(elems)`, the
    /// reflection API's `Apply(f, args)`): `UNAPPLY` of the selection with its type arguments,
    /// the using arguments as `IMPLICITarg`s, the type the extractor takes, and the patterns of
    /// the tuple or the option's value it gives.
    /// The extraction of a tuple pattern past 22 elements: `scala.runtime.TupleXXL.unapplySeq`.
    fn is_xxl_tuple_extraction(&self, p: TPatId) -> bool {
        let TPat::Unapply(_, call, _) = self.w.prog.pats[p.idx()] else { return false };
        let TExpr::CallMethod(r, _, _) = self.w.prog.expr(call) else { return false };
        let TExpr::Module(m) = self.w.prog.expr(r) else { return false };
        let info = self.w.syms.class(m);
        self.owner_full_name(info.owner, &self.name(info.name)) == "scala.runtime.TupleXXL"
    }

    fn extractor_pattern(&mut self, call: TExprId, inner: TPatId) {
        let (sym, recv, using) = match self.w.prog.expr(call) {
            TExpr::Js(t, args) => {
                let items = self.w.prog.expr_list(args).to_vec();
                match (self.w.prog.template_syms.get(&t).copied(), items.as_slice()) {
                    (Some(s), [r, _, rest @ ..]) if !self.w.syms.sym(s).is_extension => (s, Some(*r), rest.to_vec()),
                    _ => return self.fail("an extractor pattern".to_string()),
                }
            }
            TExpr::CallMethod(r, s, args) => (s, Some(r), self.w.prog.expr_list(args).get(1..).map(|a| a.to_vec()).unwrap_or_default()),
            TExpr::CallStatic(s, args) => (s, None, self.w.prog.expr_list(args).get(1..).map(|a| a.to_vec()).unwrap_or_default()),
            _ => return self.fail("an extractor pattern".to_string()),
        };
        let targs: Vec<TypeId> = self.records(call).targs.map(|l| self.w.types.items(l).to_vec()).unwrap_or_default();
        let sig = self.w.sig_of(sym).clone();
        let subst: Subst = sig.tparams.iter().copied().zip(targs.iter().copied()).collect();
        let Some(param) = sig.clauses.first().and_then(|c| c.params.first()).map(|p| self.w.types.subst(p.ty, &subst)) else {
            return self.fail("an extractor pattern of no parameter".to_string());
        };
        // The elements the result gives: a tuple's, or an option's value's.
        let mut subs: Vec<(TPatId, TypeId)> = Vec::new();
        let mut at = inner;
        loop {
            let TPat::Class(c, t, _, ps) = self.w.prog.pats[at.idx()] else { return self.fail("an extractor pattern".to_string()) };
            let ps: Vec<TPatId> = self.w.prog.pat_lists[ps.range()].to_vec();
            let zt = self.w.zonk(t);
            let args: Vec<TypeId> = match self.w.types.get(zt) {
                Type::Class(_, a) => self.w.types.items(a).to_vec(),
                _ => Vec::new(),
            };
            if self.w.is_tuple_class(c) {
                subs.extend(ps.into_iter().zip(args));
                break;
            }
            match (ps.as_slice(), args.as_slice()) {
                (&[p], &[_]) if matches!(self.w.prog.pats[p.idx()], TPat::Class(k, ..) if self.w.is_tuple_class(k)) => at = p,
                (&[p], &[v]) => {
                    subs.push((p, v));
                    break;
                }
                _ => return self.fail("an extractor pattern".to_string()),
            }
        }
        let l = self.open(UNAPPLY);
        self.term_at(None);
        let ta = (!targs.is_empty()).then(|| self.open(TYPEAPPLY));
        if ta.is_some() {
            self.term_at(None);
        }
        match recv {
            Some(r) => self.select(sym, |p| p.term(r)),
            None => self.select(sym, |p| p.owner_path(sym)),
        }
        if let Some(ta) = ta {
            for &t in &targs {
                self.tpt(t);
            }
            self.buf.end_length(ta);
        }
        for a in using {
            self.buf.byte(IMPLICITARG);
            self.term(a);
        }
        self.ty(param);
        for (p, t) in subs {
            if !matches!(self.pat_form(p), Some(PatForm::Elements)) {
                self.pattern(p, t);
                continue;
            }
            // The sequence an `unapplySeq` gives: its elements and its rest, the extractor's.
            match self.seq_elem(t) {
                Some(elem) => self.seq_elements(p, elem),
                None => return self.fail("an unapplySeq's sequence of no element type".to_string()),
            }
        }
        self.buf.end_length(l);
    }

    fn class_pattern(&mut self, c: ClassId, t: TypeId, fields: ListRef, subs: ListRef, scrut: TypeId) {
        let fields: Vec<SymId> = self.w.prog.sym_list(fields).to_vec();
        let subs: Vec<TPatId> = self.w.prog.pat_lists[subs.range()].to_vec();
        let t = self.w.zonk(t);
        let targs: Vec<TypeId> = match self.w.types.get(t) {
            Type::Class(k, args) if k == c => self.w.types.items(args).to_vec(),
            _ => Vec::new(),
        };
        let local = self.local_prefix();
        let cls = self.w.library_class_name(c, &local);
        // A program's case class has the identity `unapply` of Scala 3; scala-library's and a
        // library's declare theirs, `Option` of scala-library's.
        let result = if self.is_std_class(c) {
            let (key, shape) = self.std_unapply_key(c);
            if !self.std_shape(&key, "unapply", &shape) {
                return;
            }
            "scala.Option".to_string()
        } else if self.w.syms.class(c).def.is_none() || self.w.in_jar(self.w.syms.class(c).file) {
            let un = crate::names::UNAPPLY;
            let m = self.w.syms.class(c).companion.and_then(|o| {
                self.w.complete_class(o);
                self.w.syms.class(o).members.get(&un).copied()
            });
            // A Scala 3 class read without its companion's members has the identity `unapply`.
            match m.map(|m| self.member_signature(m)) {
                Some(Ok(Some((_, r)))) => r,
                None => cls.clone(),
                _ => return self.fail("the unapply of a library's case class".to_string()),
            }
        } else {
            cls.clone()
        };
        // A scrutinee the pattern's type does not contain is tested first, as `typedUnApply`'s
        // `Typed` has it.
        let scrut = self.w.zonk(scrut);
        let tested = !self.w.types.has_vars(scrut) && !self.w.types.has_vars(t) && {
            let mark = self.w.snapshot();
            let sub = self.w.is_sub(scrut, t);
            self.w.rollback(mark);
            !sub
        };
        let typed = tested.then(|| self.open(TYPED));
        let l = self.open(UNAPPLY);
        let ta = (!targs.is_empty()).then(|| self.open(TYPEAPPLY));
        let sl = self.open(SELECTIN);
        let mut sig = Vec::new();
        if !targs.is_empty() {
            sig.push(SigParam::Types(targs.len()));
        }
        sig.push(SigParam::Type(cls.clone()));
        let n = self.names.signed("unapply", None, &sig, &result);
        self.buf.nat(n as u64);
        self.companion_path(c);
        self.companion_class_ref(c);
        self.buf.end_length(sl);
        if let Some(x) = ta {
            for &a in &targs {
                self.tpt(a);
            }
            self.buf.end_length(x);
        }
        self.ty(t);
        let subst: Subst = {
            let own = self.w.syms.class(c).own_tparams().to_vec();
            own.into_iter().zip(targs.iter().copied()).collect()
        };
        for (f, p) in fields.iter().zip(subs) {
            let ft = self.w.sig_of(*f).ret;
            let ft = self.w.types.subst(ft, &subst);
            self.pattern(p, ft);
        }
        self.buf.end_length(l);
        if let Some(x) = typed {
            self.tpt(t);
            self.buf.end_length(x);
        }
    }
}

fn prim_name(op: PrimOp) -> &'static str {
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
        RefEq | Eq => "==",
        RefNe | Ne => "!=",
        BoolAnd => "&&",
        BoolOr => "||",
    }
}

/// The parameters of a constructor's clauses in order.
fn ctor_params(clauses: &[ClauseSig]) -> Vec<ParamSig> {
    clauses.iter().flat_map(|c| c.params.iter().cloned()).collect()
}
