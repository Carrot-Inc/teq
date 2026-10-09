//! Quotes, splices and macros' definitions as scalac 3.8.4 pickles them, before staging:
//! a quote `QUOTE` of its
//! body and type selected through `ContextFunction1.apply` and applied to the `Quotes` the search
//! found, a splice `SPLICE` of the context function scalac types its code as, a type quote
//! `Type.of[T](q)`.

use super::*;
use crate::tir::capture::Form;
use crate::tir::{TExpr, TExprId};

impl<'w, 'a> P<'w, 'a> {
    /// Whether a `Js` node is the `$quote` template of a quote.
    pub(super) fn is_quote_template(&self, node: TExpr) -> bool {
        matches!(node, TExpr::Js(t, _) if self.w.prog.strings[t.idx()] == "$quote")
    }

    /// A quote: `'{ body }.apply(q)` (`APPLY(SELECTin(apply, QUOTE(body, T), ContextFunction1),
    /// q)`), its holes the splices of their code; a type quote `Type.of[T](q)`. The `Type`
    /// values of the type parameters the body names are no part of it: scalac heals them after
    /// pickling.
    pub(super) fn quote(&mut self, e: TExprId) {
        let TExpr::Js(_, args) = self.w.prog.expr(e) else { return self.fail("a quote's template".to_string()) };
        let index = match self.w.prog.expr_list(args).first().map(|&a| self.w.prog.expr(a)) {
            Some(TExpr::Int(i)) => i as usize,
            _ => return self.fail("a quote's template".to_string()),
        };
        let q = self.w.prog.quotes[index].clone();
        let Some(quotes) = q.quotes else { return self.fail("a quote whose Quotes the typer did not keep".to_string()) };
        let Some(body) = q.body else { return self.type_of_call(q.ty, quotes) };
        // `'{ $x }`: scalac's typer cancels the pair to `x`, and its pickle never holds the
        // quote ("missed quote cancellation"); the code's `Quotes` is then the quote's. A hole
        // with layers written around it (`$x.asInstanceOf[T]`, `($x: T)`) is no such pair.
        let bare = { let r = self.records(body); r.wraps.is_empty() && r.form.is_none() };
        if let (TExpr::Local(h), true) = (self.w.prog.expr(body), bare) {
            if let Some(&(_, code)) = q.holes.iter().find(|&&(s, _)| s == h) {
                self.cancelled_quotes.push((self.splice_params.len(), quotes));
                self.term(code);
                self.cancelled_quotes.pop();
                return;
            }
        }
        let a = self.open(APPLY);
        self.term_at(Some(e));
        let s = self.open(SELECTIN);
        let n = self.names.signed("apply", None, &[SigParam::Type("java.lang.Object".to_string())], "java.lang.Object");
        self.buf.nat(n as u64);
        self.term_at(Some(e));
        let ql = self.open(QUOTE);
        self.open_holes.extend(q.holes.iter().copied());
        let depth = self.open_holes.len() - q.holes.len();
        self.quoted_depth += 1;
        self.term(body);
        self.quoted_depth -= 1;
        self.open_holes.truncate(depth);
        self.ty(q.ty);
        self.buf.end_length(ql);
        self.external_typeref("scala", "ContextFunction1");
        self.buf.end_length(s);
        self.term(quotes);
        self.buf.end_length(a);
    }

    /// `Type.of[T](q)`, a type quote `'[T]` or the `Type[T]` the search synthesized.
    fn type_of_call(&mut self, t: TypeId, quotes: TExprId) {
        let a = self.open(APPLY);
        self.term_at(None);
        let ta = self.open(TYPEAPPLY);
        self.term_at(None);
        let s = self.open(SELECTIN);
        let n = self.names.signed("of", None, &[SigParam::Types(1), SigParam::Type("scala.quoted.Quotes".to_string())], "scala.quoted.Type");
        self.buf.nat(n as u64);
        self.object_path("scala.quoted", "Type");
        self.object_class_ref("scala.quoted", "Type");
        self.buf.end_length(s);
        self.tpt(t);
        self.buf.end_length(ta);
        self.term(quotes);
        self.buf.end_length(a);
    }

    /// The code of a hole of the quote being written, which stands where its local is read:
    /// `${ (using contextual$N: Quotes) => code }`, the splice of type `T` of the hole's.
    pub(super) fn hole_splice(&mut self, hole: SymId, code: TExprId) {
        let t = self.w.sig_of(hole).ret;
        let start = self.w.prog.span_of(code).map(|(_, s)| s.start);
        let number = self.contextual_number(start, 1);
        let outer = std::mem::take(&mut self.open_holes);
        let quoted = std::mem::replace(&mut self.quoted_depth, 0);
        let l = self.open(SPLICE);
        self.splice_fun(Key::SpliceParam(hole), number, code, t);
        self.ty(t);
        self.buf.end_length(l);
        self.quoted_depth = quoted;
        self.open_holes = outer;
    }

    /// A macro's splice, `${ (using contextual$N: Quotes) => code }` of the context function
    /// the definition check kept (`TExpr::Splice` of a `Lambda` of its `Quotes`), never applied.
    pub(super) fn macro_splice(&mut self, e: TExprId, lambda: TExprId) {
        let TExpr::Lambda(params, code) = self.w.prog.expr(lambda) else { return self.fail("a macro's splice of no function".to_string()) };
        let &[q] = self.w.prog.sym_list(params) else { return self.fail("a macro's splice of no Quotes".to_string()) };
        let Some(t) = self.node_type(e) else { return self.fail("a macro's splice of no type".to_string()) };
        let start = self.w.prog.span_of(e).map(|(_, s)| s.start);
        let number = self.contextual_number(start, 1);
        // A macro of `Unit` whose code is another `Expr`: scalac discards the splice's value,
        // `{ ${ .. }; () }`, the splice of the code's own type.
        let held = self.held_type(lambda);
        if let Some(x) = held.filter(|&x| self.is_unit(t) && !self.is_unit(x)) {
            let b = self.open(BLOCK);
            self.term_at(None);
            self.buf.byte(UNITCONST);
            self.term_at(Some(e));
            let l = self.open(SPLICE);
            self.splice_fun(Key::Sym(q), number, code, x);
            self.ty(x);
            self.buf.end_length(l);
            self.buf.end_length(b);
            return;
        }
        let l = self.open(SPLICE);
        self.splice_fun(Key::Sym(q), number, code, t);
        self.ty(t);
        self.buf.end_length(l);
    }

    /// The type of the trees a macro's code gives, `X` of the `Quotes ?=> Expr[X]` the definition
    /// check typed it at.
    fn held_type(&mut self, lambda: TExprId) -> Option<TypeId> {
        let t = self.node_type(lambda)?;
        let Type::Class(_, args) = self.w.types.get(t) else { return None };
        let &[_, code] = self.w.types.items(args) else { return None };
        let Type::Class(_, held) = self.w.types.get(code) else { return None };
        self.w.types.items(held).first().copied()
    }

    /// Whether `s` is the std's `Type.of[T](using Quotes)(using Type[T])`, which scala-library
    /// declares with the first clause alone.
    pub(super) fn is_std_type_of(&self, s: SymId) -> bool {
        let info = self.w.syms.sym(s);
        let Owner::Class(o) = info.owner else { return false };
        let object = self.w.syms.class(o);
        self.is_std_class(o)
            && self.w.interner.get(info.name) == "of"
            && self.w.interner.get(object.name) == "Type"
            && matches!(object.owner, Owner::Package(p) if self.pkg_path(p) == "scala.quoted")
    }

    /// `Type.of[T](q)` of the std's `Type.of[T](q)(t)`.
    pub(super) fn std_type_of(&mut self, targs: Option<TList>, args: &[TExprId]) {
        let t = targs.and_then(|l| self.w.types.items(l).first().copied());
        match (t, args.first()) {
            (Some(t), Some(&q)) => self.type_of_call(t, q),
            _ => self.fail("Type.of without its type or its Quotes".to_string()),
        }
    }

    /// The context function of a splice of type `t`: `BLOCK(LAMBDA, DEFDEF $anonfun(contextual$N:
    /// Quotes): Expr[t] = code)`, its parameter defined under `param`, which the code's `Quotes`
    /// reads (a macro's) or the search found inside it (`Form::SpliceQuotes`) name.
    fn splice_fun(&mut self, param: Key, number: u32, code: TExprId, t: TypeId) {
        let Some(expr_class) = self.w.quoted_classes().expr else { return self.fail("scala.quoted.Expr".to_string()) };
        let Some(quotes_class) = self.w.quoted_classes().quotes else { return self.fail("scala.quoted.Quotes".to_string()) };
        let code_ty = self.w.types.class(expr_class, &[t]);
        let quotes_ty = self.w.types.class(quotes_class, &[]);
        self.term_at(Some(code));
        let b = self.open(BLOCK);
        self.term_at(Some(code));
        let l = self.open(LAMBDA);
        self.buf.byte(TERMREFDIRECT);
        let fwd = self.buf.forward_reference();
        self.buf.end_length(l);
        let d = self.buf.addr();
        self.buf.fill(fwd, d);
        self.term_at(Some(code));
        let dl = self.open(DEFDEF);
        let n = self.names.simple("$anonfun");
        self.buf.nat(n as u64);
        let addr = self.buf.addr();
        self.define(param, addr);
        self.term_at(None);
        let pl = self.open(PARAM);
        let pn = self.names.simple(&format!("contextual${}", number));
        self.buf.nat(pn as u64);
        self.tpt(quotes_ty);
        self.buf.byte(GIVEN);
        self.buf.end_length(pl);
        self.tpt(code_ty);
        if let Key::SpliceParam(hole) = param {
            self.splice_params.push((hole, self.params.len()));
            self.term_to(code, code_ty);
            self.splice_params.pop();
        } else {
            self.term_to(code, code_ty);
        }
        self.write_flags(&[SYNTHETIC, ARTIFACT]);
        self.buf.end_length(dl);
        self.buf.end_length(b);
    }

    /// The `Quotes` the search found inside a splice of a quote: the splice's own parameter.
    pub(super) fn splice_quotes(&mut self) {
        if let Some(&(depth, quotes)) = self.cancelled_quotes.last() {
            if depth == self.splice_params.len() {
                // The quote's `Quotes` is found outside it: an enclosing splice's or cancellation's.
                let inner = self.cancelled_quotes.pop();
                self.term(quotes);
                self.cancelled_quotes.extend(inner);
                return;
            }
        }
        match self.splice_params.last().copied() {
            Some((hole, _)) => {
                self.buf.byte(TERMREFDIRECT);
                self.def_ref(Key::SpliceParam(hole));
            }
            None => self.fail("a splice's Quotes outside its splice".to_string()),
        }
    }

    /// A quote pattern: `QUOTEPATTERN(body, q, Expr[T], bindings)`, the body's holes
    /// `SPLICEPATTERN`s of their binders' patterns, a type pattern's body `EXPLICITtpt` of its
    /// type, the type variables bound after the pattern's type (`BIND t (bounds) _`, synthetic
    /// where the source does not declare them), and the `Type` given each binds for the case
    /// written as scalac's `Type.of[t](q)`. The typer's form is the `$quoteMatch` extractor
    /// whose result binds the holes and the givens, in that order (`quote_match_pattern`).
    pub(super) fn quote_pattern(&mut self, call: TExprId, inner: crate::tir::TPatId) {
        use crate::tir::TPat;
        let TExpr::Js(_, args) = self.w.prog.expr(call) else { return self.fail("a quote pattern's template".to_string()) };
        let index = match self.w.prog.expr_list(args).get(1).map(|&a| self.w.prog.expr(a)) {
            Some(TExpr::Int(i)) => i as usize,
            _ => return self.fail("a quote pattern's template".to_string()),
        };
        let qp = self.w.prog.quote_pats[index].clone();
        let Some(quotes) = qp.quotes else { return self.fail("a quote pattern whose Quotes the typer did not keep".to_string()) };
        let value = match self.w.prog.pats[inner.idx()] {
            TPat::Class(_, _, _, subs) => self.w.prog.pat_lists[subs.range()].first().copied(),
            _ => None,
        };
        let count = qp.holes.len() + qp.type_params.len();
        let binders: Vec<crate::tir::TPatId> = match (count, value) {
            (0, _) => Vec::new(),
            (1, Some(v)) => vec![v],
            (_, Some(v)) => match self.w.prog.pats[v.idx()] {
                TPat::Class(_, _, _, subs) => self.w.prog.pat_lists[subs.range()].to_vec(),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        };
        if binders.len() != count {
            return self.fail("a quote pattern's binders".to_string());
        }
        let mut ids = Vec::with_capacity(qp.type_params.len());
        for (j, &t) in qp.type_params.iter().enumerate() {
            self.bindings += 1;
            self.tparams.push((t, TpRef::Direct(self.bindings)));
            ids.push(self.bindings);
            if let TPat::Bind(given, None) = self.w.prog.pats[binders[qp.holes.len() + j].idx()] {
                self.type_givens.push((given, t, quotes));
            }
        }
        let (Some(expr_class), Some(type_class)) = (self.w.quoted_classes().expr, self.w.quoted_classes().ty) else {
            return self.fail("scala.quoted's Expr and Type".to_string());
        };
        let l = self.open(QUOTEPATTERN);
        let pattern_ty = match qp.body {
            Some(body) => {
                let holes: Vec<(SymId, crate::tir::TPatId)> = qp.holes.iter().copied().zip(binders.iter().copied()).collect();
                let depth = self.pattern_holes.len();
                self.pattern_holes.extend(holes);
                self.quoted_depth += 1;
                self.term(body);
                self.quoted_depth -= 1;
                self.pattern_holes.truncate(depth);
                let t = self.node_type(body).unwrap_or(qp.ty);
                let t = self.w.widen_lit(t);
                self.w.types.class(expr_class, &[t])
            }
            None => {
                self.mark_tree();
                self.buf.byte(EXPLICITTPT);
                self.tpt(qp.ty);
                self.w.types.class(type_class, &[qp.ty])
            }
        };
        self.term(quotes);
        self.ty(pattern_ty);
        for (j, (&t, &id)) in qp.type_params.iter().zip(&ids).enumerate() {
            let info = self.w.syms.tparam(t).clone();
            let at = self.buf.addr();
            self.define(Key::Binding(id), at);
            self.mark_tree();
            let b = self.open(BIND);
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
            if j as u32 >= qp.declared {
                self.buf.byte(SYNTHETIC);
            }
            self.buf.end_length(b);
        }
        self.buf.end_length(l);
    }

    /// A hole of the quote pattern being written, where its local stands in the body:
    /// `SPLICEPATTERN(pattern, T)` of the binder its code is matched against.
    fn pattern_hole(&mut self, hole: SymId, binder: crate::tir::TPatId) {
        let t = self.w.sig_of(hole).ret;
        let Some(expr_class) = self.w.quoted_classes().expr else { return self.fail("scala.quoted.Expr".to_string()) };
        let code_ty = self.w.types.class(expr_class, &[t]);
        let l = self.open(SPLICEPATTERN);
        self.pattern(binder, code_ty);
        self.ty(t);
        self.buf.end_length(l);
    }

    /// The place a converted pickle gives the tree a node was typed from: a source of its own
    /// among the pickled sources (`is_pickled_source`), its span in that source's UTF-16 units,
    /// its point.
    pub(super) fn pickled_place_of(&mut self, e: TExprId) -> Option<(FileId, Span, Option<u32>)> {
        let (file, addr) = self.records(e).tree?;
        let (path, pos) = self.w.pickled_place(file, addr)?;
        Some((self.pickled_source(path), Span { start: pos.start, end: pos.end }, pos.point))
    }

    /// The file of the positions that stands for the source `path` of a converted pickle.
    pub(super) fn pickled_source(&mut self, path: String) -> FileId {
        let i = match self.pickled_sources.iter().position(|p| *p == path) {
            Some(i) => i,
            None => {
                self.pickled_sources.push(path);
                self.pickled_sources.len() - 1
            }
        };
        FileId(u32::MAX - i as u32)
    }

    /// Whether `f` stands for a source of a converted pickle (`pickled_place_of`), whose spans
    /// count UTF-16 units already and whose path the pickle named.
    pub(super) fn is_pickled_source(&self, f: FileId) -> bool {
        ((u32::MAX - f.0) as usize) < self.pickled_sources.len()
    }

    /// The hole of the quote being written that `s` is, with its code.
    pub(super) fn open_hole(&self, s: SymId) -> Option<TExprId> {
        self.open_holes.iter().rev().find(|&&(h, _)| h == s).map(|&(_, code)| code)
    }

    /// The number of the first `contextual$` parameter of the context closure at `start` of
    /// the pickle's file, of `count` parameters: scalac numbers them per compilation unit in the
    /// order it types the closures, which is their order in the source.
    pub(super) fn contextual_number(&mut self, start: Option<u32>, count: u32) -> u32 {
        let closures = self.index.contextual.get_or_init(|| contextual_closures(self.w));
        let found = start.and_then(|start| {
            let list = closures.get(&self.file)?;
            let i = list.iter().position(|&(s, _)| s == start)?;
            Some(1 + list[..i].iter().map(|&(_, n)| n).sum::<u32>())
        });
        match found {
            Some(n) => n,
            None => {
                let total: u32 = closures.get(&self.file).map_or(0, |l| l.iter().map(|&(_, n)| n).sum());
                self.contextual_extra += count;
                total + self.contextual_extra - count + 1
            }
        }
    }

    /// The form of a node the capture marks for the quotes, written in its place, if any.
    pub(super) fn quoted_form(&mut self, e: TExprId, node: TExpr, form: Option<Form>) -> bool {
        if let Some(Form::SpliceQuotes) = form {
            self.splice_quotes();
            return true;
        }
        match node {
            TExpr::Js(..) if self.is_quote_template(node) => {
                self.quote(e);
                true
            }
            TExpr::Splice(lambda) => {
                self.macro_splice(e, lambda);
                true
            }
            TExpr::Local(s) if !self.open_holes.is_empty() && self.open_hole(s).is_some() => {
                let code = self.open_hole(s).expect("an open hole");
                self.hole_splice(s, code);
                true
            }
            TExpr::Local(s) if self.pattern_holes.iter().any(|&(h, _)| h == s) => {
                let &(_, binder) = self.pattern_holes.iter().rev().find(|&&(h, _)| h == s).expect("a pattern hole");
                self.pattern_hole(s, binder);
                true
            }
            TExpr::Local(s) if self.type_givens.iter().any(|&(g, _, _)| g == s) => {
                let &(_, t, quotes) = self.type_givens.iter().rev().find(|&&(g, _, _)| g == s).expect("a type given");
                let ty = self.w.types.param(t);
                self.type_of_call(ty, quotes);
                true
            }
            _ => false,
        }
    }
}

/// The context closures of every file, each by its start (a hole's code's, a macro's splice's,
/// a closure's the typer made for an expected context function) with its number of
/// parameters, in the order of the source: what numbers their `contextual$` parameters.
fn contextual_closures(w: &Worker) -> FxMap<FileId, Vec<(u32, u32)>> {
    let mut out: FxMap<FileId, Vec<(u32, u32)>> = FxMap::default();
    let prog = &w.prog;
    let mut add = |at: Option<(FileId, Span)>, n: u32| {
        if let Some((file, span)) = at {
            out.entry(file).or_default().push((span.start, n));
        }
    };
    for q in prog.quotes.iter() {
        for &(_, code) in &q.holes {
            add(prog.span_of(code), 1);
        }
    }
    for i in 0..prog.exprs.len() {
        let e = TExprId(i as u32);
        match prog.expr(e) {
            TExpr::Splice(_) => add(prog.span_of(e), 1),
            TExpr::Lambda(params, _) => {
                let params = prog.sym_list(params);
                let contextual = params.first().map_or(false, |&p| w.interner.get(w.syms.sym(p).name).starts_with("contextual$"));
                if contextual {
                    add(prog.span_of(e), params.len() as u32);
                }
            }
            _ => {}
        }
    }
    for list in out.values_mut() {
        list.sort_unstable();
        list.dedup_by_key(|&mut (s, _)| s);
    }
    out
}
