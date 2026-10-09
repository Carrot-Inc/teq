//! The completeness census of the capture (`TEQ_CAPTURE_CENSUS=<file>`): a walk of every
//! body, constructor and initialiser of the owned sources as their
//! pickle will see them (an ordinary inline call as the call, a transparent one's expansion
//! with its origin, a quote's body at its level), which counts per construct the nodes whose
//! typed form follows from the node, its symbol and its recorded type (derived), the nodes a
//! record of the capture completes (captured), those it should complete and does not
//! (missing), and those the capture cannot state (unsupported). The records no walk reaches
//! (an alternative's tried and discarded) are the orphans. A line per construct is appended to
//! the file; `tests/support/capture-census.sh` sums the files of many builds.

use super::Worker;
use crate::intern::FxMap;
use crate::source::FileId;
use crate::symbols::*;
use crate::tir::capture::{Capture, Form, Key, PatForm, Wrap};
use crate::tir::*;
use crate::types::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
enum Status {
    Derived = 0,
    Captured = 1,
    Missing = 2,
    Unsupported = 3,
}

/// What the parameter an argument is passed to asks of it.
#[derive(Clone, Copy, Default)]
struct Param {
    by_name: bool,
    repeated: bool,
    has_default: bool,
}

#[derive(Clone, Copy)]
struct Ctx {
    /// Whether the node is part of the pickle: not inside an ordinary inline call's expansion.
    pickled: bool,
    /// The quotes around the node less the splices.
    level: u32,
    param: Option<Param>,
    /// Inside a block that unpacks a comprehension's variables.
    unpacking: bool,
    /// The innermost quote around the node, whose holes are its splices.
    quote: Option<u32>,
}

#[derive(Default)]
pub struct Census {
    /// Per construct: derived, captured, missing, unsupported, and the pickled nodes without a
    /// source position.
    rows: BTreeMap<&'static str, [u64; 5]>,
    /// A few places of the missing ones, per construct.
    places: BTreeMap<&'static str, Vec<String>>,
    pub orphans: u64,
    /// The records the compaction dropped: of typings no body holds.
    pub discarded: u64,
    /// A few of the orphans: the record's kind, the node's and its place.
    pub orphan_places: Vec<String>,
    pub unsolved: u64,
    /// With `TEQ_CAPTURE_PARTS`, the parts each interpolation captured, by the definition it is
    /// in, written as the TASTy printer writes a `StringContext`'s (`teq tasty --body`).
    pub parts: Vec<String>,
}

/// A set of expression ids: as bits for the merged program's, whose ids are dense, and in a
/// table for a worker's own, which stand far above them.
#[derive(Default)]
struct Seen {
    bits: Vec<u64>,
    far: FxMap<u32, ()>,
}

impl Seen {
    /// Adds `i`, and says whether it was there.
    #[inline]
    fn insert(&mut self, i: u32) -> bool {
        if i >= crate::arena::LOCAL_BASE {
            return self.far.insert(i, ()).is_some();
        }
        let (w, b) = ((i / 64) as usize, i % 64);
        if self.bits.len() <= w {
            self.bits.resize(w + 1, 0);
        }
        let had = self.bits[w] & (1 << b) != 0;
        self.bits[w] |= 1 << b;
        had
    }

    #[inline]
    fn has(&self, i: u32) -> bool {
        if i >= crate::arena::LOCAL_BASE {
            return self.far.contains_key(&i);
        }
        self.bits.get((i / 64) as usize).map_or(false, |w| w & (1 << (i % 64)) != 0)
    }
}

struct Walk<'w, 'a> {
    w: &'w Worker<'a>,
    c: &'w Capture,
    census: Census,
    seen: Seen,
    seen_pats: FxMap<TPatId, ()>,
    seen_locals: FxMap<SymId, ()>,
    file: FileId,
    /// The nodes the walk is inside, and the definition, for the places the census gives.
    stack: Vec<TExprId>,
    def: Option<SymId>,
    /// The binder a count without a node is of, for its place.
    binder_named: Option<SymId>,
    /// Whether the walk is the census's, which counts, or the finalisation's, which only
    /// marks what it reaches.
    counting: bool,
    /// The holes of every quote, which no code outside a quote names, and the nodes that
    /// named one outside a quote and were counted.
    quote_holes: FxMap<SymId, ()>,
    reported_scope: FxMap<TExprId, ()>,
    /// The local classes a walked body makes whose file no walked root is of (a quote's copy of
    /// a converted pickle's class, written in the expansion that makes it), to walk once.
    unowned: Vec<(ClassId, Ctx)>,
    unowned_seen: FxMap<ClassId, ()>,
    /// The walk's deepest node and the lowest stack address it reached, which `TEQ_CAPTURE_FINISH`
    /// reports.
    pub deepest: u32,
    pub stack_low: usize,
}

impl<'w, 'a> Walk<'w, 'a> {
    fn count(&mut self, construct: &'static str, status: Status, e: Option<TExprId>, ctx: Ctx) {
        if !ctx.pickled || !self.counting {
            return;
        }
        let row = self.census.rows.entry(construct).or_default();
        row[status as usize] += 1;
        let placed = e.and_then(|e| self.w.prog.span_of(e));
        if e.is_some() && placed.is_none() {
            row[4] += 1;
        }
        if matches!(status, Status::Missing) && self.census.places.get(construct).map_or(true, |p| p.len() < 5) {
            let (file, start) = match placed {
                Some((f, s)) => (f, s.start),
                None => (self.file, 0),
            };
            let src = self.w.source(file);
            let (line, _, _) = crate::source::locate(&src.text, start as usize);
            let node = match (e, self.binder_named) {
                (Some(e), _) => self.describe(e),
                (None, Some(b)) => format!("binder {}", self.w.name_ref(self.w.syms.sym(b).name)),
                (None, None) => String::new(),
            };
            let within: Vec<String> = self.stack.iter().rev().filter(|&&x| Some(x) != e).take(3).map(|&x| self.describe(x)).collect();
            let def = self.def.map_or(String::new(), |d| self.w.name_str(self.w.syms.sym(d).name).to_string());
            let place = format!("{}:{}\t{}\t{} < {}", src.path, line, node, def, within.join(" < "));
            if std::env::var_os("TEQ_CAPTURE_TRACE").is_some() {
                for &x in self.stack.iter().rev().take(4) {
                    eprintln!("  {:?} {:?} {:?} {:?} builtin {:?} inline {:?}", x, self.w.prog.expr(x), self.c.forms.get(&x), self.c.targs.get(&x).map(|l| self.w.types.items(*l).len()), self.c.builtin_calls.get(&x).map(|b| b.name), self.c.inline_calls.get(&x).map(|c| c.len()));
                }
                eprintln!("missing {} at {}", construct, place);
            }
            self.census.places.entry(construct).or_default().push(place);
        }
    }

    /// The node's kind and the name of the symbol it names, for the places the census gives.
    /// The scope of a tree a record names, which the walk may have reached from elsewhere
    /// first (a quote's body that the record of a copy of it names still): every quote's hole
    /// it names outside a quote counts as missing, once.
    fn named_by_record(&mut self, e: TExprId, ctx: Ctx) {
        if !self.counting || ctx.level != 0 {
            return;
        }
        let outside: Vec<TExprId> = self.w.prog.descendants(e).filter(|&x| self.out_of_scope(x)).collect();
        for x in outside {
            if !self.reported_scope.contains_key(&x) {
                self.reported_scope.insert(x, ());
                self.seen.insert(x.0);
                self.count("local out of its scope", Status::Missing, Some(x), ctx);
            }
        }
    }

    /// Whether `e` names a quote's hole, which code outside the quote cannot.
    fn out_of_scope(&self, e: TExprId) -> bool {
        matches!(self.w.prog.expr(e), TExpr::Local(s) if self.quote_holes.contains_key(&s))
    }

    fn describe(&self, e: TExprId) -> String {
        let node = self.w.prog.expr(e);
        let kind = format!("{:?}", node);
        let kind = kind.split('(').next().unwrap_or("").to_string();
        let sym = match node {
            TExpr::CallMethod(_, s, _) | TExpr::CallStatic(s, _) | TExpr::Field(_, s) | TExpr::Static(s) | TExpr::Local(s) | TExpr::NewVia(s, _) => Some(s),
            _ => None,
        };
        match (sym, node) {
            (Some(s), _) => format!("{} {}", kind, self.w.name_str(self.w.syms.sym(s).name)),
            (None, TExpr::New(c, _)) => format!("{} {}", kind, self.w.name_str(self.w.syms.class(c).name)),
            (None, TExpr::Js(t, _)) => format!("{} {}", kind, self.w.prog.strings[t.idx()]),
            _ => kind,
        }
    }

    fn has_tparams(&self, s: SymId) -> bool {
        self.w.syms.sym(s).sig.as_ref().map_or(false, |sig| !sig.tparams.is_empty())
    }

    fn params_of(&self, s: SymId) -> Vec<Param> {
        match &self.w.syms.sym(s).sig {
            Some(sig) => sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| Param { by_name: p.by_name, repeated: p.repeated, has_default: p.has_default })).collect(),
            None => Vec::new(),
        }
    }

    fn targs_status(&self, e: TExprId, generic: bool) -> Status {
        match (generic, self.c.targs.contains_key(&e)) {
            (false, _) => Status::Derived,
            (true, true) => Status::Captured,
            (true, false) => Status::Missing,
        }
    }

    /// An inner class's accessor of its enclosing instance: `C.this` of the class its result is.
    fn outer_accessor(&self, s: SymId) -> bool {
        self.w.name_str(self.w.syms.sym(s).name).starts_with("$outer")
    }

    fn list(&mut self, l: crate::ast::ListRef, ctx: Ctx) {
        for &a in self.w.prog.expr_list(l) {
            self.expr(a, Ctx { param: None, ..ctx });
        }
    }

    /// The arguments of a call of `s`, each with the parameter it is passed to.
    fn args(&mut self, s: SymId, l: crate::ast::ListRef, ctx: Ctx) {
        let params = self.params_of(s);
        self.aligned(&params, l, ctx);
    }

    /// Arguments for `params`; where there are more, the first ones are what a local or inner
    /// class's constructor takes before them (the captured locals, the enclosing instance),
    /// which the pickle's `new` does not pass.
    fn aligned(&mut self, params: &[Param], l: crate::ast::ListRef, ctx: Ctx) {
        let items = self.w.prog.expr_list(l);
        let extra = items.len().saturating_sub(params.len());
        for (i, &a) in items.iter().enumerate() {
            if i < extra {
                self.expr(a, Ctx { pickled: false, param: None, ..ctx });
                continue;
            }
            let param = params.get(i - extra).copied();
            self.expr(a, Ctx { param, ..ctx });
        }
    }

    fn ctor_params(&self, c: ClassId) -> Vec<Param> {
        self.w.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| Param { by_name: p.by_name, repeated: p.repeated, has_default: p.has_default })).collect()
    }

    fn stmts(&mut self, l: crate::ast::ListRef, ctx: Ctx) {
        let w = self.w;
        for &s in w.prog.stmt_list(l) {
            match s {
                TStmt::Expr(e) => self.expr(e, ctx),
                TStmt::Val(v, e) => {
                    self.binder(v, ctx);
                    self.expr(e, ctx);
                }
                TStmt::Fun(f) => {
                    self.count("local def", Status::Derived, None, ctx);
                    self.local_annotations(self.w.prog.funs[f.idx()].sym, ctx);
                    self.fun(f, ctx);
                }
                TStmt::Pat(p, e) => {
                    self.count("pattern definition", Status::Derived, None, ctx);
                    self.pat(p, ctx);
                    self.expr(e, ctx);
                }
            }
        }
    }

    /// A local val's definition: a temporary the typer made needs the type it gave `Any`.
    fn binder(&mut self, v: SymId, ctx: Ctx) {
        self.seen_locals.insert(v, ());
        if !self.counting || !ctx.pickled {
            return;
        }
        // The temporaries typed `Any` in the place of their value's type: a hoisted operand's
        // and argument's (`h$`), a packed comprehension's (`u$`).
        let (synthetic, ty) = {
            let info = self.w.syms.sym(v);
            let name = self.w.name_ref(info.name);
            (name.starts_with("h$") || name.starts_with("u$"), info.sig.as_ref().map_or(ERROR, |s| s.ret))
        };
        let local = self.c.locals.get(&v);
        if local.map_or(false, |l| l.mods != 0) {
            self.count("local modifiers", Status::Captured, None, ctx);
        }
        let status = match (synthetic && ty == ANY, local.and_then(|l| l.ty)) {
            (false, _) => Status::Derived,
            (true, Some(_)) => Status::Captured,
            (true, None) => Status::Missing,
        };
        self.binder_named = Some(v);
        self.count(if synthetic { "temporary" } else { "local val" }, status, None, ctx);
        self.binder_named = None;
    }

    /// The annotations of a local definition, which the typer resolves none of.
    fn local_annotations(&mut self, s: SymId, ctx: Ctx) {
        if !self.counting {
            return;
        }
        let info = self.w.syms.sym(s);
        let Some(d) = info.def else { return };
        let n = self.w.ast(info.file).def(d).annots.len();
        for _ in 0..n {
            self.count("local annotation", Status::Unsupported, None, ctx);
        }
    }

    /// Whether `s` is a definition the typer made rather than read (a mirror's val, a case
    /// class's members, an accessor), whose right-hand side is stage 3's inventory and not
    /// the capture's.
    fn synthesized(&self, s: SymId) -> bool {
        self.w.syms.sym(s).def.is_none()
    }

    fn fun(&mut self, f: FunId, ctx: Ctx) {
        let w = self.w;
        let tf = &w.prog.funs[f.idx()];
        let (defaults, body, sym) = (&tf.defaults, tf.body, tf.sym);
        let ctx = if ctx.pickled && self.synthesized(sym) && self.w.syms.sym(sym).owner != Owner::Local {
            self.count("synthesized definition", Status::Unsupported, None, ctx);
            Ctx { pickled: false, ..ctx }
        } else {
            ctx
        };
        let outer = self.def.replace(sym);
        for &d in defaults.iter().flatten() {
            self.expr(d, Ctx { param: None, ..ctx });
        }
        if let Some(b) = body {
            self.expr(b, Ctx { param: None, ..ctx });
        }
        self.def = outer;
    }

    fn cases(&mut self, l: crate::ast::ListRef, ctx: Ctx) {
        let w = self.w;
        for &c in w.prog.case_list(l) {
            self.pat(c.pat, ctx);
            if let Some(g) = c.guard {
                self.expr(g, ctx);
            }
            self.expr(c.body, ctx);
        }
    }

    fn pat(&mut self, p: TPatId, ctx: Ctx) {
        self.seen_pats.insert(p, ());
        match self.w.prog.pats[p.idx()] {
            TPat::Wildcard => self.count("wildcard pattern", Status::Derived, None, ctx),
            TPat::Bind(_, inner) => {
                self.count("binder pattern", Status::Derived, None, ctx);
                if let Some(i) = inner {
                    self.pat(i, ctx);
                }
            }
            TPat::Test(_, _, inner) => {
                self.count("typed pattern", Status::Derived, None, ctx);
                self.pat(inner, ctx);
            }
            TPat::Equals(e, _) => {
                self.count("literal or stable pattern", Status::Derived, None, ctx);
                self.expr(e, ctx);
            }
            TPat::Class(_, _, _, subs) => {
                self.count("case class pattern", Status::Derived, None, ctx);
                for &s in &self.w.prog.pat_lists[subs.range()] {
                    self.pat(s, ctx);
                }
            }
            TPat::Alt(subs) => {
                self.count("alternative pattern", Status::Derived, None, ctx);
                for &s in &self.w.prog.pat_lists[subs.range()] {
                    self.pat(s, ctx);
                }
            }
            TPat::Seq(items, rest) => {
                let status = match self.c.pats.get(&p) {
                    Some(PatForm::Seq(_) | PatForm::Elements) => Status::Captured,
                    None => Status::Missing,
                };
                self.count("sequence pattern", status, None, ctx);
                for &s in &self.w.prog.pat_lists[items.range()] {
                    self.pat(s, ctx);
                }
                if let Some(r) = rest {
                    self.pat(r, ctx);
                }
            }
            TPat::Unapply(_, call, inner) => {
                self.count("extractor pattern", Status::Derived, None, ctx);
                self.expr(call, ctx);
                self.pat(inner, ctx);
            }
        }
    }

    fn wraps(&mut self, e: TExprId, ctx: Ctx) {
        let Some(ws) = self.c.wraps.get(&e) else { return };
        for w in ws.clone() {
            let construct = match w {
                Wrap::Ascribed(_) => "ascription",
                Wrap::Cast(_) => "cast",
                Wrap::Unchecked => "@unchecked",
                Wrap::Splice(_) => "sequence argument",
                Wrap::Named(_) => "named argument",
                Wrap::Member(..) => "builtin member call",
            };
            self.count(construct, Status::Captured, Some(e), ctx);
        }
    }

    fn expr(&mut self, e: TExprId, ctx: Ctx) {
        if self.seen.insert(e.0) {
            // A node reached twice: an inline call's argument that its expansion holds too.
            return;
        }
        if self.counting {
            self.wraps(e, ctx);
        }
        // The path a node reached its member through and leaves out, which the pickle writes.
        if let Some(&r) = self.c.receivers.get(&e) {
            self.expr(r, Ctx { param: None, ..ctx });
        }
        // The evidence a call passes where the signature erases it, which the pickle writes.
        if let Some(evidence) = self.c.evidence.get(&e) {
            for a in evidence.clone() {
                self.expr(a, Ctx { param: None, ..ctx });
            }
        }
        self.stack.push(e);
        let here = 0u8;
        self.stack_low = self.stack_low.min(&here as *const u8 as usize);
        self.deepest = self.deepest.max(self.stack.len() as u32);
        if let Some(calls) = self.c.inline_calls.get(&e) {
            let calls = calls.clone();
            self.inline_chain(e, &calls, ctx);
        } else if let Some(call) = self.c.builtin_calls.get(&e) {
            let call = call.clone();
            let construct = match call.name {
                crate::names::APPLY => "reduced function application",
                crate::names::PRODUCT_PREFIX => "builtin member call",
                _ => "tuple member",
            };
            self.count(construct, Status::Captured, Some(e), ctx);
            self.named_by_record(call.recv, ctx);
            self.expr(call.recv, Ctx { param: None, ..ctx });
            for &a in &call.args {
                self.named_by_record(a, ctx);
                self.expr(a, Ctx { param: None, ..ctx });
            }
            self.node(e, Ctx { pickled: false, param: None, ..ctx });
        } else {
            self.node(e, ctx);
        }
        self.stack.pop();
    }

    /// The expansion `e` of the inline calls `calls`, innermost first: an ordinary call is
    /// pickled as the call and its expansion is not; a transparent call's expansion is, with
    /// the call's origin.
    fn inline_chain(&mut self, e: TExprId, calls: &[crate::tir::capture::InlineCall], ctx: Ctx) {
        let Some((outer, inner)) = calls.split_last() else {
            self.node(e, ctx);
            return;
        };
        let generic = self.has_tparams(outer.callee);
        if outer.transparent {
            self.count("transparent inline expansion", Status::Captured, Some(e), ctx);
            if outer.binds {
                self.count("transparent expansion's bindings", Status::Captured, None, ctx);
            }
            let off = Ctx { pickled: false, param: None, ..ctx };
            for &r in outer.recv.iter().chain(&outer.args) {
                self.named_by_record(r, ctx);
                self.expr(r, off);
            }
            self.inline_chain(e, inner, Ctx { param: None, ..ctx });
        } else {
            self.count("inline call", Status::Captured, Some(e), ctx);
            let status = if generic { Status::Captured } else { Status::Derived };
            self.count("inline call type arguments", status, None, ctx);
            for &r in outer.recv.iter().chain(&outer.args) {
                self.named_by_record(r, ctx);
            }
            if let Some(r) = outer.recv {
                self.expr(r, Ctx { param: None, ..ctx });
            }
            let params = self.params_of(outer.callee);
            let aligned = params.len() == outer.args.len();
            for (i, &a) in outer.args.iter().enumerate() {
                let param = if aligned { Some(params[i]) } else { None };
                self.expr(a, Ctx { param, ..ctx });
            }
            self.inline_chain(e, inner, Ctx { pickled: false, param: None, ..ctx });
        }
    }

    fn node(&mut self, e: TExprId, ctx: Ctx) {
        // The finalisation's walk only marks what it reaches, which the forms do not change.
        let form = if self.counting { self.c.forms.get(&e).copied() } else { None };
        match form {
            Some(Form::Evidence(_)) => {
                self.count("synthesized given", Status::Captured, Some(e), ctx);
                self.node_of(e, None, Ctx { pickled: false, ..ctx });
            }
            _ => self.node_of(e, form, ctx),
        }
    }

    fn node_of(&mut self, e: TExprId, form: Option<Form>, ctx: Ctx) {
        let prog = &self.w.prog;
        let inner = Ctx { param: None, ..ctx };
        match prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Null => {
                match form {
                    Some(Form::Cast(_)) => self.count("cast", Status::Captured, Some(e), ctx),
                    Some(Form::Folded(_)) => self.count("folded operator", Status::Captured, Some(e), ctx),
                    Some(Form::Constant(_)) => self.count("constant read", Status::Captured, Some(e), ctx),
                    Some(Form::Written(_)) => self.count("interpolation part", Status::Captured, Some(e), ctx),
                    _ => self.count("literal", Status::Derived, Some(e), ctx),
                }
            }
            TExpr::Unit => match (form, ctx.param) {
                (Some(Form::Default), _) => self.count("default argument", Status::Captured, Some(e), ctx),
                (_, Some(p)) if p.has_default => self.count("default argument", Status::Missing, Some(e), ctx),
                _ => self.count("literal", Status::Derived, Some(e), ctx),
            },
            TExpr::Local(_) if !self.counting => {}
            TExpr::Local(s) => {
                let hole = ctx.quote.and_then(|q| prog.quotes.try_get(q)).map_or(false, |q| q.holes.iter().any(|&(h, _)| h == s));
                let outer = self.w.name_str(self.w.syms.sym(s).name).starts_with("$this");
                let construct = if hole {
                    "splice in a quote"
                } else if outer {
                    "outer this"
                } else {
                    "local reference"
                };
                self.count(construct, Status::Derived, Some(e), ctx);
            }
            TExpr::This => self.count("this", Status::Derived, Some(e), ctx),
            TExpr::Super(_) => self.count("super", Status::Derived, Some(e), ctx),
            TExpr::Static(s) => {
                let status = self.targs_status(e, self.has_tparams(s));
                self.count("static reference", status, Some(e), ctx);
            }
            TExpr::Module(_) => self.count("module reference", Status::Derived, Some(e), ctx),
            TExpr::Field(r, _) if matches!(form, Some(Form::Op(_))) => {
                self.count("named tuple member", Status::Captured, Some(e), ctx);
                self.expr(r, inner);
            }
            TExpr::Field(r, s) => {
                let status = self.targs_status(e, self.has_tparams(s));
                self.count("field selection", status, Some(e), ctx);
                self.expr(r, inner);
            }
            TExpr::CallStatic(s, args) => {
                let (construct, status) = match form {
                    Some(Form::Member(_)) => ("helper for a member", Status::Captured),
                    _ => ("call", self.targs_status(e, self.has_tparams(s))),
                };
                self.count(construct, status, Some(e), ctx);
                self.args(s, args, ctx);
            }
            TExpr::CallMethod(r, s, _) if self.outer_accessor(s) => {
                self.count("outer this", Status::Derived, Some(e), ctx);
                self.expr(r, Ctx { pickled: false, ..inner });
            }
            TExpr::CallMethod(r, s, args) => {
                let status = self.targs_status(e, self.has_tparams(s));
                self.count("method call", status, Some(e), ctx);
                self.expr(r, inner);
                self.args(s, args, ctx);
            }
            TExpr::CallClosure(f, args) => match form {
                Some(Form::Dynamic(_) | Form::Op(_)) => {
                    self.count("JavaScript dynamic call", Status::Captured, Some(e), ctx);
                    match prog.expr(f) {
                        TExpr::JsSelect(r, _) if matches!(form, Some(Form::Dynamic(_))) => {
                            self.seen.insert(f.0);
                            self.expr(r, inner);
                        }
                        _ => self.expr(f, inner),
                    }
                    self.list(args, ctx);
                }
                Some(Form::Member(s)) => {
                    self.count("JavaScript call", Status::Captured, Some(e), ctx);
                    self.expr(f, Ctx { pickled: false, ..inner });
                    let params = self.params_of(s);
                    let items = prog.expr_list(args);
                    for (i, &a) in items.iter().enumerate() {
                        self.expr(a, Ctx { param: params.get(i).copied(), ..ctx });
                    }
                }
                _ => {
                    self.count("function application", Status::Derived, Some(e), ctx);
                    self.expr(f, inner);
                    self.list(args, ctx);
                }
            },
            TExpr::New(c, args) => {
                let info = self.w.syms.class(c);
                if info.owner == Owner::Local && !self.c.owns(info.file) && self.unowned_seen.insert(c, ()).is_none() {
                    self.unowned.push((c, ctx));
                }
                let generic = !self.w.syms.class(c).tparams.is_empty();
                match form {
                    Some(Form::GivenCall(s)) => {
                        let status = self.targs_status(e, self.has_tparams(s));
                        self.count("given call", status, Some(e), ctx);
                        // The enclosing instance a class's or trait's given class takes first is
                        // the call's qualifier in the pickle.
                        let params = self.params_of(s);
                        let items = self.w.prog.expr_list(args).to_vec();
                        let extra = items.len().saturating_sub(params.len());
                        for (i, &a) in items.iter().enumerate() {
                            let param = if i < extra { None } else { params.get(i - extra).copied() };
                            self.expr(a, Ctx { param, ..ctx });
                        }
                    }
                    _ => {
                        let status = self.targs_status(e, generic);
                        self.count("constructor call", status, Some(e), ctx);
                        let params = self.ctor_params(c);
                        self.aligned(&params, args, ctx);
                    }
                }
            }
            TExpr::NewVia(s, args) => {
                let generic = match self.w.syms.sym(s).owner {
                    Owner::Class(c) => !self.w.syms.class(c).tparams.is_empty(),
                    _ => false,
                };
                let status = self.targs_status(e, generic);
                self.count("secondary constructor call", status, Some(e), ctx);
                self.args(s, args, ctx);
            }
            TExpr::Lambda(params, body) => {
                match (form, ctx.param) {
                    (Some(Form::ByName), _) => self.count("by-name argument", Status::Captured, Some(e), ctx),
                    (_, Some(p)) if p.by_name && params.is_empty() => self.count("by-name argument", Status::Missing, Some(e), ctx),
                    _ => self.count("closure", Status::Derived, Some(e), ctx),
                }
                let unpacks = self.counting && matches!(self.c.forms.get(&body), Some(Form::Unpack(_)));
                for &p in prog.sym_list(params) {
                    if unpacks {
                        self.seen_locals.insert(p, ());
                        self.count("comprehension parameter", Status::Derived, None, ctx);
                    } else {
                        self.binder(p, ctx);
                    }
                }
                self.expr(body, inner);
            }
            TExpr::If(c, t, els) => {
                self.count("if", Status::Derived, Some(e), ctx);
                self.expr(c, inner);
                self.expr(t, inner);
                if let Some(x) = els {
                    self.expr(x, inner);
                }
            }
            TExpr::While(c, b) => {
                self.count("while", Status::Derived, Some(e), ctx);
                self.expr(c, inner);
                self.expr(b, inner);
            }
            TExpr::Block(stmts, res) => {
                let unpacking = matches!(form, Some(Form::Unpack(_)));
                match form {
                    Some(Form::CaseCopy) => self.count("case class copy", Status::Captured, Some(e), ctx),
                    Some(Form::Unpack(_)) => self.count("comprehension unpacking", Status::Captured, Some(e), ctx),
                    Some(Form::Op(_)) => self.count("builtin member call", Status::Captured, Some(e), ctx),
                    _ => self.count("block", Status::Derived, Some(e), ctx),
                }
                let within = Ctx { unpacking, ..inner };
                self.stmts(stmts, within);
                self.expr(res, inner);
            }
            TExpr::Assign(l, r) => {
                self.count("assignment", Status::Derived, Some(e), ctx);
                self.expr(l, inner);
                self.expr(r, inner);
            }
            TExpr::Match(s, cases) => {
                self.count("match", Status::Derived, Some(e), ctx);
                self.expr(s, inner);
                self.cases(cases, inner);
            }
            TExpr::Prim(op, a, b) => {
                let ambiguous = matches!(op, PrimOp::RefEq | PrimOp::RefNe | PrimOp::Eq | PrimOp::Ne);
                let status = match (form, ambiguous) {
                    (Some(Form::Op(_) | Form::SwappedOp(_) | Form::SuperOp(_)), _) => Status::Captured,
                    (_, true) => Status::Missing,
                    (_, false) => Status::Derived,
                };
                self.count("operator", status, Some(e), ctx);
                self.expr(a, inner);
                self.expr(b, inner);
            }
            TExpr::Unary(_, a) => {
                let construct = match form {
                    Some(Form::Promotion) => "operand promotion",
                    Some(Form::Widening) => "numeric widening",
                    Some(Form::Cast(_)) => "cast",
                    _ => "unary operator",
                };
                self.count(construct, if form.is_some() { Status::Captured } else { Status::Derived }, Some(e), ctx);
                self.expr(a, inner);
            }
            TExpr::StrConcat(l) => {
                if let (Some(Form::Interp { parts, .. }), true) = (form, ctx.pickled && std::env::var_os("TEQ_CAPTURE_PARTS").is_some()) {
                    let texts: Vec<String> = self.w.types.items(parts).iter().map(|&t| match self.w.types.get(t) {
                        Type::Lit(l) => match self.w.types.lit_val(l) {
                            LitVal::Str(n) => format!("{:?}", self.w.name_str(n)),
                            _ => "?".to_string(),
                        },
                        _ => "?".to_string(),
                    }).collect();
                    let def = self.def.map_or(String::new(), |d| self.w.name_str(self.w.syms.sym(d).name).to_string());
                    self.census.parts.push(format!("{}\t[{}]", def, texts.join(", ")));
                }
                let status = match form {
                    Some(Form::Interp { .. }) => Status::Captured,
                    _ if l.len == 2 => Status::Derived,
                    _ => Status::Missing,
                };
                self.count(if matches!(form, Some(Form::Interp { .. })) { "interpolation" } else { "string concatenation" }, status, Some(e), ctx);
                self.list(l, ctx);
            }
            TExpr::ToStr(a, conv) => {
                self.count(if conv.is_rendering() { "concatenation operand" } else { "toString call" }, Status::Derived, Some(e), ctx);
                self.expr(a, inner);
            }
            TExpr::Js(s, args) => self.template(e, s, args, form, ctx),
            TExpr::TypeTest(a, _) => {
                let status = if matches!(form, Some(Form::Test(_))) { Status::Captured } else { Status::Missing };
                self.count("type test", status, Some(e), ctx);
                self.expr(a, inner);
            }
            TExpr::ClassOf(_) => self.count("class literal", Status::Derived, Some(e), ctx),
            TExpr::SeqLit(l) => {
                let status = if matches!(form, Some(Form::Repeated(_))) { Status::Captured } else { Status::Missing };
                self.count("repeated arguments", status, Some(e), ctx);
                self.list(l, ctx);
            }
            TExpr::ArrayLit(l) => match form {
                Some(Form::Pack(_)) => {
                    self.count("comprehension packing", Status::Captured, Some(e), ctx);
                    self.list(l, ctx);
                }
                Some(Form::EnumMember(_)) => {
                    self.count("enum companion member", Status::Captured, Some(e), ctx);
                    self.list(l, Ctx { pickled: false, ..ctx });
                }
                _ => {
                    self.count("array literal", Status::Missing, Some(e), ctx);
                    self.list(l, ctx);
                }
            },
            TExpr::Index(a, _) => {
                let status = if ctx.unpacking { Status::Derived } else { Status::Missing };
                self.count("packed read", status, Some(e), ctx);
                self.expr(a, inner);
            }
            TExpr::JsImport(_) | TExpr::JsGlobal(..) => {
                let status = if matches!(form, Some(Form::Member(_))) { Status::Captured } else { Status::Missing };
                self.count("JavaScript reference", status, Some(e), ctx);
            }
            TExpr::JsSelect(a, _) => {
                let status = if matches!(form, Some(Form::Member(_) | Form::Dynamic(_))) { Status::Captured } else { Status::Missing };
                self.count("JavaScript selection", status, Some(e), ctx);
                self.expr(a, inner);
            }
            TExpr::ObjLit(l) => {
                let status = if matches!(form, Some(Form::DynamicLiteral(_) | Form::JsObject(_))) { Status::Captured } else { Status::Missing };
                self.count("JavaScript object literal", status, Some(e), ctx);
                self.list(l, ctx);
            }
            TExpr::Spread(a) => {
                let status = if ctx.param.map_or(false, |p| p.repeated) { Status::Derived } else { Status::Missing };
                self.count("JavaScript spread", status, Some(e), ctx);
                self.expr(a, inner);
            }
            TExpr::Return(a) => {
                let status = if matches!(form, Some(Form::Return(_))) { Status::Captured } else { Status::Missing };
                self.count("return", status, Some(e), ctx);
                self.expr(a, inner);
            }
            TExpr::Throw(a, _) => {
                self.count("throw", Status::Derived, Some(e), ctx);
                self.expr(a, inner);
            }
            TExpr::Try(i) => {
                self.count("try", Status::Derived, Some(e), ctx);
                let (body, cases, finalizer) = {
                    let t = &prog.tries[i as usize];
                    (t.body, t.cases, t.finalizer)
                };
                self.expr(body, inner);
                self.cases(cases, inner);
                if let Some(f) = finalizer {
                    self.expr(f, inner);
                }
            }
            TExpr::Splice(a) => {
                self.count("macro splice", Status::Derived, Some(e), ctx);
                self.expr(a, Ctx { level: ctx.level.saturating_sub(1), ..inner });
            }
        }
    }

    /// A `Js` template call: a member's, a quote, or a lowering the capture names.
    fn template(&mut self, e: TExprId, s: StrRef, args: crate::ast::ListRef, form: Option<Form>, ctx: Ctx) {
        let prog = &self.w.prog;
        let text = prog.strings[s.idx()].as_str();
        if text == "$quoteMatch" {
            let items = prog.expr_list(args);
            let index = items.get(1).and_then(|&i| match prog.expr(i) {
                TExpr::Int(n) => Some(n as u32),
                _ => None,
            });
            self.count("quote pattern", Status::Derived, Some(e), ctx);
            let pat = index.and_then(|i| prog.quote_pats.try_get(i));
            let (body, quotes) = (pat.and_then(|q| q.body), pat.and_then(|q| q.quotes));
            if let Some(body) = body {
                self.expr(body, Ctx { level: ctx.level + 1, param: None, quote: None, ..ctx });
            }
            // The pattern's `Quotes`, which the pickle writes as its operand.
            if let Some(q) = quotes {
                self.expr(q, Ctx { param: None, ..ctx });
            }
            for (i, &a) in items.iter().enumerate() {
                if i != 1 {
                    self.expr(a, Ctx { param: None, ..ctx });
                }
            }
            return;
        }
        if text == "$quote" {
            let items = prog.expr_list(args);
            let index = items.first().and_then(|&i| match prog.expr(i) {
                TExpr::Int(n) => Some(n as usize),
                _ => None,
            });
            self.count("quote", Status::Derived, Some(e), ctx);
            let record = index.and_then(|i| prog.quotes.try_get(i as u32));
            let (quote, quotes) = (record.and_then(|q| q.body), record.and_then(|q| q.quotes));
            if let Some(body) = quote {
                self.expr(body, Ctx { level: ctx.level + 1, param: None, quote: index.map(|i| i as u32), ..ctx });
            }
            // The quote's `Quotes`, which the pickle applies it to.
            if let Some(q) = quotes {
                self.expr(q, Ctx { param: None, ..ctx });
            }
            for &a in items.iter().skip(1) {
                self.expr(a, Ctx { param: None, ..ctx });
            }
            return;
        }
        let status = match form {
            Some(Form::Member(_) | Form::Cast(_) | Form::PartialFunction | Form::Op(_) | Form::SuperOp(_) | Form::EnumMember(_)) => Status::Captured,
            _ if prog.template_syms.get(&s).is_some() => Status::Derived,
            _ => Status::Missing,
        };
        let construct = match form {
            Some(Form::Cast(_)) => "cast",
            Some(Form::PartialFunction) => "partial function literal",
            Some(Form::Op(_) | Form::SuperOp(_)) => "builtin member call",
            Some(Form::EnumMember(_)) => "enum companion member",
            _ => "template call",
        };
        self.count(construct, status, Some(e), ctx);
        let items = prog.expr_list(args);
        let values = matches!(form, Some(Form::EnumMember(_)));
        for (i, &a) in items.iter().enumerate() {
            let pickled = ctx.pickled && !(values && i == 0);
            self.expr(a, Ctx { pickled, param: None, ..ctx });
        }
    }

    fn class(&mut self, tc: &TClass, ctx: Ctx) {
        for d in tc.ctor_defaults.iter().flatten() {
            self.count("constructor default", Status::Derived, None, ctx);
            self.expr(*d, ctx);
        }
        if let Some(args) = tc.parent_args {
            self.count("parent constructor call", Status::Derived, None, ctx);
            match tc.parent_via {
                Some(via) => self.args(via, args, ctx),
                None => self.list(args, ctx),
            }
        }
        self.stmts(tc.parent_prelude, ctx);
        for init in &tc.init {
            match *init {
                TInit::Field(s, e) if self.synthesized(s) => {
                    self.count("synthesized definition", Status::Unsupported, None, ctx);
                    self.expr(e, Ctx { pickled: false, ..ctx });
                }
                TInit::Field(_, e) => {
                    self.count("field initialiser", Status::Derived, None, ctx);
                    self.expr(e, ctx);
                }
                TInit::Stmt(e) => {
                    self.count("template statement", Status::Derived, None, ctx);
                    self.expr(e, ctx);
                }
                TInit::Parent(_, pc) => {
                    self.count("trait parent call", Status::Derived, None, ctx);
                    self.stmts(pc.prelude, ctx);
                    self.list(pc.args, ctx);
                }
            }
        }
        for &f in tc.methods.iter().chain(&tc.ctors) {
            self.fun(f, ctx);
        }
    }
}

impl<'a> Worker<'a> {
    /// The walk of the bodies of the owned files, or of `only` among them.
    fn walk_owned<'w>(&'w self, c: &'w Capture, only: Option<&[FileId]>, counting: bool) -> Walk<'w, 'a> {
        let mut quote_holes: FxMap<SymId, ()> = FxMap::default();
        // The classes a quote's body makes (an anonymous class of it), whose bodies are at the
        // quote's level.
        let mut quoted_classes: FxMap<ClassId, u32> = FxMap::default();
        if counting {
            for (i, q) in self.prog.quotes.iter().enumerate() {
                quote_holes.extend(q.holes.iter().map(|&(h, _)| (h, ())));
                for e in q.body.into_iter().flat_map(|b| self.prog.descendants(b)) {
                    if let TExpr::New(c, _) = self.prog.expr(e) {
                        if self.syms.class(c).kind == ClassKind::Anon {
                            quoted_classes.insert(c, i as u32);
                        }
                    }
                }
            }
            for q in self.prog.quote_pats.iter() {
                quote_holes.extend(q.holes.iter().map(|&h| (h, ())));
            }
        }
        let mut walk = Walk { w: self, c, census: Census::default(), seen: Seen::default(), seen_pats: FxMap::default(), seen_locals: FxMap::default(), file: FileId(0), stack: Vec::new(), def: None, binder_named: None, counting, quote_holes, reported_scope: FxMap::default(), unowned: Vec::new(), unowned_seen: FxMap::default(), deepest: 0, stack_low: usize::MAX };
        let walked = |f: FileId| c.owns(f) && only.map_or(true, |files| files.contains(&f));
        let root = Ctx { pickled: true, level: 0, param: None, unpacking: false, quote: None };
        for tc in self.prog.classes.iter() {
            let file = self.syms.class(tc.id).file;
            if walked(file) {
                walk.file = file;
                let expanded = c.expansion_classes.contains_key(&tc.id);
                let at = match quoted_classes.get(&tc.id) {
                    Some(&q) => Ctx { level: 1, quote: Some(q), ..root },
                    None => root,
                };
                walk.class(tc, if expanded { Ctx { pickled: false, ..at } } else { at });
            }
        }
        let funs: Vec<FunId> = self.prog.top_funs.iter().copied().collect();
        for f in funs {
            let file = self.syms.sym(self.prog.funs[f.idx()].sym).file;
            if walked(file) {
                walk.file = file;
                walk.fun(f, root);
            }
        }
        let vals: Vec<(SymId, TExprId)> = self.prog.top_vals.iter().copied().collect();
        for (s, e) in vals {
            let file = self.syms.sym(s).file;
            if walked(file) {
                walk.file = file;
                if walk.synthesized(s) {
                    walk.count("synthesized definition", Status::Unsupported, None, root);
                    walk.expr(e, Ctx { pickled: false, ..root });
                } else {
                    walk.count("top-level val", Status::Derived, None, root);
                    walk.expr(e, root);
                }
            }
        }
        let mut annotations: Vec<((FileId, crate::ast::ExprId), TExprId)> = c.annotations.iter().map(|(&k, &e)| (k, e)).collect();
        annotations.sort_unstable_by_key(|&(k, _)| k);
        for ((file, _), e) in annotations {
            if c.annotates(file) && only.map_or(true, |files| files.contains(&file)) {
                walk.file = file;
                walk.count("annotation", Status::Captured, Some(e), root);
                walk.expr(e, root);
            }
        }
        let mut stored: Vec<(SymId, Option<TExprId>, DefinitionState, std::sync::Arc<crate::tir::InlineDefinition>)> =
            self.inline_definitions.local.iter().map(|(&s, d)| (s, d.body, d.state, d.clone())).collect();
        stored.sort_by_key(|&(s, _, _, _)| s);
        // An inline method whose body the definition check did not store.
        if counting {
            for (i, info) in self.syms.syms.iter().enumerate() {
                let s = SymId(i as u32);
                let inline_def = info.mods & crate::ast::mods::INLINE != 0 && info.kind == SymKind::Def && info.def.is_some();
                if inline_def && walked(info.file) && self.inline_definitions.get(&s).is_none() {
                    walk.count("inline method body", Status::Unsupported, None, root);
                }
            }
        }
        for (s, body, state, record) in stored {
            let file = self.syms.sym(s).file;
            if !walked(file) {
                continue;
            }
            walk.file = file;
            match (state, body) {
                (DefinitionState::Held(_), _) | (_, None) => walk.count("inline method body", Status::Unsupported, None, root),
                (_, Some(b)) => {
                    walk.count("inline method body", Status::Captured, None, root);
                    walk.expr(b, root);
                    // What the pickle writes of the record beside the body: the defaults' getters,
                    // the imports' selections, the classes the body makes.
                    for &d in record.defaults.iter().flatten() {
                        walk.expr(d, root);
                    }
                    for a in &record.aliases {
                        walk.expr(a.tree, root);
                    }
                    for tc in &record.classes {
                        walk.class(tc, root);
                    }
                }
            }
        }
        // The bodies of the quotes the pickle does not reach through an owned body (a library's,
        // which a macro's run copies), whose records the copies take.
        for q in self.prog.quotes.iter() {
            if let Some(b) = q.body {
                walk.expr(b, Ctx { pickled: false, level: 1, ..root });
            }
        }
        if !walk.unowned.is_empty() {
            let index: FxMap<ClassId, usize> = self.prog.classes.iter().enumerate().map(|(i, tc)| (tc.id, i)).collect();
            while let Some((c, ctx)) = walk.unowned.pop() {
                if let Some(&i) = index.get(&c) {
                    walk.class(&self.prog.classes[i], ctx);
                }
            }
        }
        walk
    }

    /// Drops the records of the typings no body holds (an alternative tried and discarded, an
    /// inline match's scrutinee its reduction left behind), of every owned file or of `only`
    /// among them: what the walk from their roots does not reach.
    pub(super) fn compact_capture(&mut self, only: Option<&[FileId]>) {
        let Some(c) = self.prog.capture.as_deref() else { return };
        let top = 0u8;
        let top = &top as *const u8 as usize;
        let (seen, seen_pats, seen_locals) = {
            let walk = self.walk_owned(c, only, false);
            if super::capture::finish_trace() {
                eprintln!("capture finish: walk {} nodes deep, {} KB of stack below the finalisation's frame", walk.deepest, top.saturating_sub(walk.stack_low) / 1024);
                eprintln!("capture finish: walked footprint {} MB, resident {} MB; seen bits {} KB, far {}, patterns {} (capacity {}), locals {} (capacity {}), stack capacity {}", crate::alloc::footprint() >> 20, crate::alloc::resident_pages().rss >> 20, walk.seen.bits.len() * 8 / 1024, walk.seen.far.len(), walk.seen_pats.len(), walk.seen_pats.capacity(), walk.seen_locals.len(), walk.seen_locals.capacity(), walk.stack.capacity());
            }
            (walk.seen, walk.seen_pats, walk.seen_locals)
        };
        let stored = self.stored_inline_keys();
        let mut taken = self.prog.capture.take().unwrap();
        let c = &mut *taken;
        let files: Vec<FileId> = match only {
            Some(files) => files.to_vec(),
            None => c.made.keys().copied().collect(),
        };
        let mut dropped = 0u64;
        let mut gone: Vec<Key> = Vec::new();
        for f in files {
            let Some(keys) = c.made.remove(&f) else { continue };
            let mut kept = Vec::with_capacity(keys.len());
            for k in keys {
                let live = match (k, k.expr()) {
                    (_, Some(e)) => seen.has(e.0),
                    (Key::Pat(p), _) => seen_pats.contains_key(&p),
                    (Key::Local(s), _) => seen_locals.contains_key(&s),
                    _ => true,
                };
                if live {
                    kept.push(k);
                    continue;
                }
                // A record moved to another node (`Capture::moved`) is not there to drop.
                if c.has(k) {
                    dropped += !stored.holds(k) as u64;
                    c.drop_key(k);
                    gone.push(k);
                }
            }
            if !kept.is_empty() {
                c.made.insert(f, kept);
            }
        }
        c.sync_published(&gone);
        c.discarded += dropped;
        self.prog.capture = Some(taken);
    }

    /// The census of the capture over the owned sources.
    pub fn capture_census_of(&self) -> Option<Census> {
        let c = self.prog.capture.as_deref()?;
        let mut walk = self.walk_owned(c, None, true);
        let seen = &walk.seen;
        let mut orphans: Vec<(&str, TExprId)> = Vec::new();
        orphans.extend(c.targs.keys().filter(|e| !seen.has(e.0)).map(|&e| ("type arguments", e)));
        orphans.extend(c.forms.keys().filter(|e| !seen.has(e.0)).map(|&e| ("form", e)));
        orphans.extend(c.wraps.keys().filter(|e| !seen.has(e.0)).map(|&e| ("wrapper", e)));
        orphans.extend(c.inline_calls.keys().filter(|e| !seen.has(e.0)).map(|&e| ("inline call", e)));
        orphans.extend(c.builtin_calls.keys().filter(|e| !seen.has(e.0)).map(|&e| ("tuple member", e)));
        orphans.extend(c.evidence.keys().filter(|e| !seen.has(e.0)).map(|&e| ("evidence", e)));
        orphans.extend(c.block_classes.keys().filter(|e| !seen.has(e.0)).map(|&e| ("block's classes", e)));
        orphans.sort_by_key(|&(_, e)| e);
        let orphan_pats = c.pats.keys().filter(|p| !walk.seen_pats.contains_key(p)).count();
        let orphan_locals = c.locals.keys().filter(|s| !walk.seen_locals.contains_key(s)).count();
        let orphan_published = c.published_orphans();
        if orphan_published > 0 {
            walk.census.orphan_places.push(format!("published\t{} records\t-", orphan_published));
        }
        walk.census.orphans = (orphans.len() + orphan_pats + orphan_locals + orphan_published) as u64;
        walk.census.discarded = c.discarded;
        if c.copies_checked + c.copies_lost > 0 {
            let row = walk.census.rows.entry("copied record").or_default();
            row[Status::Captured as usize] += c.copies_checked;
            row[Status::Missing as usize] += c.copies_lost;
            walk.census.places.entry("copied record").or_default().extend(c.lost_at.iter().cloned());
        }
        for &(what, e) in orphans.iter().take(10) {
            let place = match self.prog.span_of(e) {
                Some((f, sp)) => {
                    let src = self.source(f);
                    format!("{}:{}", src.path, crate::source::locate(&src.text, sp.start as usize).0)
                }
                None => "-".to_string(),
            };
            let node = format!("{:?}", self.prog.expr(e));
            let node = node.split('(').next().unwrap_or("").to_string();
            walk.census.orphan_places.push(format!("{}\t{}\t{}", what, node, place));
        }
        let has_vars = |t: TypeId| t != NO_TYPE && self.types.has_vars(t);
        walk.census.unsolved = (c.targs.values().filter(|&&l| self.types.items(l).iter().any(|&t| has_vars(t))).count()
            + c.forms.values().filter(|f| {
                let mut v = false;
                super::capture::map_form(**f, &self.types, &mut |t| {
                    v |= has_vars(t);
                    t
                }, &|s| s);
                v
            }).count()) as u64;
        Some(walk.census)
    }

    /// Appends the census to `TEQ_CAPTURE_CENSUS`, when it names a file.
    pub fn capture_census(&self) {
        let Some(path) = std::env::var_os("TEQ_CAPTURE_CENSUS") else { return };
        let Some(census) = self.capture_census_of() else { return };
        let mut text = String::new();
        for (construct, n) in &census.rows {
            text.push_str(&format!("row\t{}\t{}\t{}\t{}\t{}\t{}\n", construct, n[0], n[1], n[2], n[3], n[4]));
        }
        for (construct, places) in &census.places {
            for p in places {
                text.push_str(&format!("missing\t{}\t{}\n", construct, p));
            }
        }
        text.push_str(&format!("orphans\t{}\ndiscarded\t{}\nunsolved\t{}\n", census.orphans, census.discarded, census.unsolved));
        for o in &census.orphan_places {
            text.push_str(&format!("orphan\t{}\n", o));
        }
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(text.as_bytes());
        }
        if let Some(path) = std::env::var_os("TEQ_CAPTURE_PARTS") {
            let lines: String = census.parts.iter().map(|l| format!("{}\n", l)).collect();
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = f.write_all(lines.as_bytes());
            }
        }
    }
}
