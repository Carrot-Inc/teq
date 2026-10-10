use super::names::{is_js_identifier, js_string};
use super::share::{HoleKind, HoleNode};
use super::scope::Root;
use super::{Ctx, Emitter, FunKind, Held};
use super::layout::is_eager_top_val;
use crate::ast::{mods, ListRef};
use crate::source::FileId;
use crate::symbols::*;
use crate::tir::*;
use crate::types::{ClassId, SymId};
use std::fmt::Write;
use std::rc::Rc;
use std::sync::atomic::Ordering;

impl<'a> Emitter<'a> {
    /// True when the expression can be written as a JS expression without statements.
    pub fn is_simple(&mut self, e: TExprId) -> bool {
        if self.is_opaque(e) {
            return true;
        }
        match self.simple_cache[e.idx()].load(Ordering::Relaxed) {
            1 => return true,
            2 => return false,
            _ => {}
        }
        let prog = self.prog;
        let list_simple = |this: &mut Self, l: ListRef| {
            prog.expr_list(l).iter().all(|&a| this.is_simple(a))
        };
        let simple = match prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_)
            | TExpr::Str(_) | TExpr::Unit | TExpr::Local(_) | TExpr::This | TExpr::Super(_) | TExpr::Static(_)
            | TExpr::Module(_) | TExpr::ClassOf(_) | TExpr::Lambda(..) | TExpr::JsImport(_) | TExpr::JsGlobal(..) => true,
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::TypeTest(r, _) | TExpr::Cast(r, ..)
            | TExpr::Index(r, _) | TExpr::Spread(r) | TExpr::JsSelect(r, _) => self.is_simple(r),
            TExpr::New(c, _) if self.closure_anons.contains_key(&c) => true,
            TExpr::CallStatic(_, args) | TExpr::New(_, args) | TExpr::NewVia(_, args) | TExpr::StrConcat(args)
            | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) | TExpr::ObjLit(args) => {
                list_simple(self, args)
            }
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.is_simple(r) && list_simple(self, args)
            }
            TExpr::Prim(_, a, b) => self.is_simple(a) && self.is_simple(b),
            TExpr::If(c, t, Some(els)) => self.is_simple(c) && self.is_simple(t) && self.is_simple(els),
            TExpr::Block(stmts, res) => {
                let stmts = &prog.stmts[stmts.range()];
                stmts.iter().all(|s| matches!(*s, TStmt::Expr(x) if self.is_simple(x))) && self.is_simple(res)
            }
            TExpr::Null => true,
            TExpr::If(_, _, None) | TExpr::While(..) | TExpr::Assign(..) | TExpr::Match(..) | TExpr::Return(_)
            | TExpr::Throw(..) | TExpr::Try(_) => false,
            TExpr::Splice(_) => unreachable!("a stored inline body is emitted nowhere"),
        };
        self.simple_cache[e.idx()].store(if simple { 1 } else { 2 }, Ordering::Relaxed);
        simple
    }

    pub(super) fn emit_args(&mut self, args: ListRef) {
        self.out.push('(');
        self.emit_more_args(args, "");
    }

    /// The arguments of a call of the def or method `s`, without the `undefined` an omitted
    /// argument stands for behind the last one written: a default of the JS function covers it.
    pub(super) fn emit_call_args(&mut self, s: SymId, args: ListRef) {
        let params: Vec<bool> = match &self.syms.sym(s).sig {
            Some(sig) => sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.has_default)).collect(),
            None => Vec::new(),
        };
        self.emit_args_with_defaults(args, &params);
    }

    pub(super) fn emit_ctor_args(&mut self, c: ClassId, args: ListRef) {
        let params: Vec<bool> = self.syms.class(c).ctor.iter().flat_map(|c| c.params.iter().map(|p| p.has_default)).collect();
        self.emit_args_with_defaults(args, &params);
    }

    fn emit_args_with_defaults(&mut self, args: ListRef, has_default: &[bool]) {
        let items = self.prog.expr_list(args);
        let mut keep = items.len();
        while keep > 0 {
            match self.peek(items[keep - 1]) {
                TExpr::Spread(inner) if self.seq_literal_items(inner).map_or(false, |l| l.is_empty()) => keep -= 1,
                TExpr::Unit if has_default.get(keep - 1) == Some(&true) => keep -= 1,
                _ => break,
            }
        }
        self.out.push('(');
        self.emit_more_args(ListRef { start: args.start, len: keep as u32 }, "");
    }

    /// The arguments after an opening parenthesis that may hold one already, and the closing one.
    pub(super) fn emit_more_args(&mut self, args: ListRef, mut separator: &'static str) {
        let prog = self.prog;
        for &a in prog.expr_list(args) {
            // Literal varargs of a spread parameter are passed as plain arguments.
            let inlined = match self.peek(a) {
                TExpr::Spread(inner) => self.seq_literal_items(inner),
                _ => None,
            };
            for &item in inlined.unwrap_or(std::slice::from_ref(&a)) {
                self.out.push_str(separator);
                separator = ", ";
                self.emit_value(item);
            }
        }
        self.out.push(')');
    }

    fn emit_closure_call(&mut self, f: TExprId, args: ListRef) {
        // A default import is read off the module namespace, which must not become `this`.
        if matches!(self.peek(f), TExpr::JsImport(i) if self.is_default_import(i)) {
            self.out.push_str("(0, ");
            self.emit_expr(f);
            self.out.push(')');
            self.emit_args(args);
            return;
        }
        let plain = matches!(
            self.peek(f),
            TExpr::Local(_)
                | TExpr::CallClosure(..)
                | TExpr::CallStatic(..)
                | TExpr::JsImport(_)
                | TExpr::JsGlobal(_, false)
                | TExpr::JsSelect(..)
                | TExpr::Module(_)
        );
        if !plain {
            self.out.push('(');
        }
        self.emit_expr(f);
        if !plain {
            self.out.push(')');
        }
        self.emit_args(args);
    }

    /// The `apply` of a function class (`scala.Function1`), or its abstract redeclaration in a
    /// trait below one, which every value of the trait answers as the function it is. An
    /// overloaded `apply` of a trait is a member of its own.
    fn is_function_apply(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        if info.name != crate::names::APPLY {
            return false;
        }
        let Owner::Class(k) = info.owner else { return false };
        if self.is_function_class(k) {
            return true;
        }
        let owner = self.syms.class(k);
        let params: usize = info.sig.as_ref().map_or(0, |sig| sig.clauses.iter().map(|c| c.params.len()).sum());
        owner.kind == ClassKind::Trait
            && !info.alternative
            && self.reach.is_abstract(self.syms, s)
            && owner.base_types.iter().skip(1).any(|&(b, _)| self.is_function_class(b) && self.syms.class(b).tparams.len() == params + 1)
    }

    /// An arrow function only stands on its own as a whole argument or element; next to an
    /// operator or a member access of the template it needs parentheses.
    fn arrow_needs_parens(&self, arg: TExprId, before: &str, after: &str) -> bool {
        if !matches!(self.peek(arg), TExpr::Lambda(..)) && !self.is_closure(arg) {
            return false;
        }
        !Self::stands_alone(before, after)
    }

    /// Whether a template's placeholder is a whole argument or element, between what comes
    /// `before` and `after` it.
    fn stands_alone(before: &str, after: &str) -> bool {
        let opens = matches!(before.trim_end().as_bytes().last(), None | Some(b'(' | b',' | b'['));
        let closes = matches!(after.trim_start().as_bytes().first(), None | Some(b')' | b',' | b']'));
        opens && closes
    }

    /// Whether the expression, written where it stands on its own, is a conditional, an
    /// assignment, an arrow function or a comma expression, which the condition of a
    /// conditional cannot be without parentheses.
    fn is_conditional_level(&mut self, e: TExprId) -> bool {
        if self.is_opaque(self.through_same(e)) || self.is_statement_node(e) {
            return false;
        }
        match self.peek(e) {
            TExpr::If(_, _, Some(_)) | TExpr::JsGlobal(_, true) => true,
            TExpr::Field(r, s) if self.syms.js_member(s) => self.global_scope_member(r, s).is_some(),
            TExpr::Js(t, _) => matches!(template_shape(&self.prog.strings[t.idx()]), Shape::Compound { conditional: true }),
            _ => false,
        }
    }

    /// The constructor arguments of an anonymous class are the bindings its body captures, passed
    /// as they are: a lazy val or a local def as the function that stands for it, a `var` as a
    /// cell that reads and writes the variable itself.
    fn emit_capture_args(&mut self, c: ClassId, args: ListRef) {
        self.out.push('(');
        let captures = match self.local_captures.get(&c) {
            Some(&n) => n as usize,
            None => (args.len - self.anon_parent_args.get(&c).copied().unwrap_or(0)) as usize,
        };
        for (i, &a) in self.prog.expr_list(args).iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
            }
            let TExpr::Local(s) = self.peek(a) else {
                self.emit_expr(a);
                continue;
            };
            if i >= captures {
                self.emit_expr(a);
                continue;
            }
            let captured_here = self.captures.contains(&s);
            let name = self.local_ref(s);
            if self.syms.sym(s).kind != SymKind::Var {
                self.out.push_str(&name);
            } else if captured_here {
                self.out.push_str(name.strip_suffix(".v").unwrap_or(&name));
            } else {
                let _ = write!(self.out, "$ref(() => {0}, ($v) => {{ {0} = $v; }})", name);
            }
        }
        self.out.push(')');
    }

    /// `(a, b) => f(a, b)` is `f` itself. Besides saving a call this keeps the identity of an
    /// eta-expanded def stable, which JS libraries rely on (React tells components apart by it).
    /// A def whose call runs its file's initialiser first is `f` where the file is initialised
    /// or where `f` checks the initialiser itself (`Reach::forwarded`), else the lambda with the
    /// check in its call.
    pub(super) fn forwarded_function(&self, params: ListRef, body: TExprId) -> Option<SymId> {
        let target = self.prog.forwarded_target(params, body)?;
        let TExpr::CallStatic(_, args) = self.prog.expr(body) else { return None };
        if self.hole_of(body).is_some() || self.prog.expr_list(args).iter().any(|&a| self.hole_of(a).is_some()) {
            return None;
        }
        let checked = self.needs_init(target, None).is_none() || self.reach.forwarded.get(target.idx()).copied().unwrap_or(false);
        checked.then_some(target)
    }

    /// The initialiser to run in front of `e`, a call or a read of a top-level definition whose
    /// access runs its file's initialiser first (`Emitter::needs_init`); an eager val's accessor
    /// runs it itself.
    pub(super) fn init_before(&self, e: TExprId) -> Option<FileId> {
        if self.is_opaque(e) {
            return None;
        }
        let e = self.through_same(e);
        if self.is_opaque(e) {
            return None;
        }
        match self.prog.expr(e) {
            TExpr::CallStatic(s, _) => self.needs_init(s, Some(e)),
            TExpr::Static(s) if !is_eager_top_val(self.syms, s, self.const_vals) => self.needs_init(s, Some(e)),
            TExpr::New(c, _) if self.reach.trigger_classes.get(c.idx()).copied().unwrap_or(false) && Some(e) != self.checked => {
                let file = self.syms.class(c).file;
                (self.init_file != Some(file)).then_some(file)
            }
            _ => None,
        }
    }

    /// `(init(), ` in front of an access that runs its file's initialiser first, whose `)`
    /// follows it.
    fn open_init(&mut self, e: TExprId) -> bool {
        let Some(init) = self.init_before(e).and_then(|file| self.file_init(file)) else { return false };
        let _ = write!(self.out, "({}(), ", init);
        true
    }

    /// `init();` as a statement in front of the statement holding `e`, an access that runs its
    /// file's initialiser first; the access is then written without its check.
    fn init_stmt(&mut self, e: TExprId) {
        if let Some(file) = self.init_before(e) {
            self.write_init_stmt(file);
            self.checked = Some(e);
        }
    }

    fn write_init_stmt(&mut self, file: FileId) {
        if let Some(init) = self.file_init(file) {
            self.line();
            let _ = write!(self.out, "{}();", init);
            self.mark_stmt();
        }
    }

    /// A value whose evaluation runs nothing: what an assignment may evaluate before the check
    /// of the assigned variable's file, as its setter checks.
    fn runs_nothing(&self, e: TExprId) -> bool {
        match self.peek(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null | TExpr::This => true,
            TExpr::Local(s) => self.syms.sym(s).kind != SymKind::Var && self.syms.sym(s).mods & mods::LAZY == 0,
            _ => false,
        }
    }

    pub(super) fn seq_literal_items(&self, e: TExprId) -> Option<&'a [TExprId]> {
        match self.peek(e) {
            TExpr::SeqLit(items) => Some(self.prog.expr_list(items)),
            _ => None,
        }
    }

    /// Expands `...$n` of an intrinsic template; literal varargs need no runtime sequence.
    fn emit_template_spread(&mut self, arg: Option<TExprId>, rest: &str) {
        match arg.and_then(|a| self.seq_literal_items(a)) {
            Some([]) if rest.trim_start().starts_with(',') => self.out.push_str("...[]"),
            Some(items) => {
                for (i, &item) in items.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.emit_expr(item);
                }
            }
            None => {
                self.out.push_str("...");
                match arg {
                    Some(a) => self.emit_expr(a),
                    None => self.out.push_str("[]"),
                }
            }
        }
    }

    /// Nodes that only exist as JS statements; in value position they get wrapped in an IIFE. A
    /// block of expression statements is a comma expression.
    fn is_statement_node(&mut self, e: TExprId) -> bool {
        if self.is_opaque(e) {
            return false;
        }
        let e = self.through_same(e);
        if self.is_opaque(e) {
            return false;
        }
        match self.prog.expr(e) {
            TExpr::Block(stmts, res) => {
                let stmts = &self.prog.stmts[stmts.range()];
                !stmts.iter().all(|s| matches!(*s, TStmt::Expr(x) if self.is_simple(x))) || self.is_statement_node(res)
            }
            TExpr::If(_, _, None) | TExpr::While(..) | TExpr::Assign(..) | TExpr::Match(..) | TExpr::Return(_)
            | TExpr::Try(_) => true,
            _ => false,
        }
    }

    /// The expression as an operand: what needs parentheses next to an operator has them.
    pub fn emit_expr(&mut self, e: TExprId) {
        self.emit_expr_as(e, false);
    }

    /// The expression where it stands on its own: returned, assigned, passed, an element, the
    /// condition of an `if`, or the body of an arrow function.
    pub fn emit_value(&mut self, e: TExprId) {
        self.emit_expr_as(e, true);
    }

    fn emit_expr_as(&mut self, e: TExprId, bare: bool) {
        if let Some((n, kind)) = self.hole_of(e) {
            let _ = write!(self.out, "$k{}", n);
            if kind == HoleKind::Thunk {
                self.out.push_str("()");
            }
            return;
        }
        if let Some(site) = self.site_of(e) {
            self.emit_site_call(site);
            return;
        }
        if self.is_statement_node(e) {
            self.out.push_str("(() => ");
            self.fn_depth += 1;
            self.enter_scope(Root::Expr(e));
            self.emit_body(e);
            self.leave_scope();
            self.fn_depth -= 1;
            self.out.push_str(")()");
            return;
        }
        let prog = self.prog;
        let (open, close) = if bare { ("", "") } else { ("(", ")") };
        match prog.expr(e) {
            TExpr::Int(v) => {
                if v < 0 {
                    let _ = write!(self.out, "{}{}{}", open, v, close);
                } else {
                    let _ = write!(self.out, "{}", v);
                }
            }
            TExpr::Long(v) => {
                if v < 0 {
                    let _ = write!(self.out, "{}{}n{}", open, v, close);
                } else {
                    let _ = write!(self.out, "{}n", v);
                }
            }
            TExpr::Double(v) => {
                if let Some(g) = double_global(v) {
                    self.note_text(g);
                }
                if v.is_nan() {
                    self.out.push_str("NaN");
                } else if v.is_infinite() {
                    let _ = write!(self.out, "{}", if v > 0.0 { "Infinity" } else if bare { "-Infinity" } else { "(-Infinity)" });
                } else if v < 0.0 || (v == 0.0 && v.is_sign_negative()) {
                    let _ = write!(self.out, "{}{:?}{}", open, v, close);
                } else {
                    let _ = write!(self.out, "{:?}", v);
                }
            }
            TExpr::Bool(v) => self.out.push_str(if v { "true" } else { "false" }),
            TExpr::Char(c) => {
                let s = match c {
                    0xD800..=0xDFFF => crate::text::lone_surrogate(c as u32).to_string(),
                    _ => String::from_utf16_lossy(&[c]),
                };
                js_string(&s, &mut self.out);
            }
            TExpr::Str(s) => js_string(&prog.strings[s.idx()], &mut self.out),
            TExpr::Unit => self.out.push_str("undefined"),
            TExpr::Local(s) => {
                let n = self.local_ref(s);
                self.out.push_str(&n);
                if self.syms.sym(s).mods & mods::LAZY != 0 {
                    self.out.push_str("()");
                }
            }
            TExpr::This | TExpr::Super(_) => self.out.push_str(self.this_name),
            TExpr::Static(s) => {
                let init = self.open_init(e);
                self.emit_static(s);
                if init {
                    self.out.push(')');
                }
            }
            TExpr::Module(c) => match self.syms.class(c).js_binding {
                Some(b) => {
                    let binding = self.js_binding(b);
                    self.out.push_str(&binding);
                }
                None => self.emit_accessor_call(c),
            },
            TExpr::ClassOf(c) => self.emit_class_of(c),
            TExpr::Field(r, s) if self.syms.js_member(s) => match self.global_scope_member(r, s) {
                // An undeclared global would throw; a facade reads it as undefined.
                Some(name) => {
                    let _ = write!(self.out, "{1}typeof {0} === \"undefined\" ? undefined : {0}{2}", name, open, close);
                }
                None => {
                    self.emit_receiver(r);
                    let p = self.js_prop(s);
                    self.out.push_str(&p);
                }
            },
            TExpr::Field(r, s) if matches!(self.peek(r), TExpr::This) && self.ctor_locals.contains(&s) => {
                let n = self.ctor_param(s);
                self.out.push_str(&n);
            }
            TExpr::Field(r, s) => {
                self.emit_receiver(r);
                self.out.push('.');
                let n = self.sym_name(s);
                self.out.push_str(&n);
                if self.syms.sym(s).needs_accessor {
                    self.out.push_str("()");
                }
            }
            TExpr::CallStatic(s, args) => {
                let init = self.open_init(e);
                let n = self.local_ref(s);
                self.out.push_str(&n);
                self.emit_call_args(s, args);
                if init {
                    self.out.push(')');
                }
            }
            TExpr::CallMethod(r, s, args) if matches!(self.peek(r), TExpr::Super(_)) => {
                let TExpr::Super(target) = prog.expr(r) else { unreachable!() };
                let n = self.sym_name(s);
                match target {
                    // JavaScript's `super` is the prototype behind the class that holds the method.
                    SuperTarget::Chain if self.this_name == "this" => {
                        let _ = write!(self.out, "super.{}", n);
                        self.emit_args(args);
                    }
                    SuperTarget::Chain => {
                        let _ = write!(self.out, "super.{}.call({}", n, self.this_name);
                        self.emit_more_args(args, ", ");
                    }
                    SuperTarget::Class(c) => {
                        let class = self.class_name(c);
                        let _ = write!(self.out, "{}.prototype.{}.call({}", class, n, self.this_name);
                        self.emit_more_args(args, ", ");
                    }
                    SuperTarget::Mixin(tr) => {
                        let accessor = self.super_accessor_name(tr, s);
                        let _ = write!(self.out, "{}.{}", self.this_name, accessor);
                        self.emit_args(args);
                    }
                }
            }
            TExpr::CallMethod(r, s, args) if self.syms.js_member(s) => {
                match self.global_scope_member(r, s) {
                    Some(name) => self.out.push_str(&name),
                    None => {
                        self.emit_receiver(r);
                        let p = self.js_prop(s);
                        self.out.push_str(&p);
                    }
                }
                self.emit_args(args);
            }
            // A function value is a JS function, whether a lambda, a lowered anonymous class
            // or an instance of a callable class: the `apply` of its function type is a call.
            TExpr::CallMethod(f, s, args) if self.is_function_apply(s) => self.emit_closure_call(f, args),
            TExpr::CallMethod(r, s, args) => {
                self.emit_receiver(r);
                self.out.push('.');
                let n = self.sym_name(s);
                self.out.push_str(&n);
                self.emit_call_args(s, args);
            }
            TExpr::CallClosure(f, args) => self.emit_closure_call(f, args),
            TExpr::New(c, args) if self.closure_anons.contains_key(&c) => {
                let idx = self.closure_anons[&c];
                self.emit_closure(e, idx, args);
            }
            TExpr::New(c, _) if self.syms.class(c).kind == ClassKind::Builtin => self.out.push_str("({})"),
            TExpr::NewVia(s, args) => {
                let factory = self.ctor_factory(s);
                self.out.push_str(&factory);
                self.emit_args(args);
            }
            TExpr::New(c, args) => {
                let init = self.open_init(e);
                let touched = self.companion_touch(c);
                let touch = touched.map(|companion| self.open_touch(companion));
                let n = self.class_name(c);
                self.out.push_str("new ");
                self.out.push_str(&n);
                if self.syms.class(c).kind == ClassKind::Anon || self.local_captures.contains_key(&c) {
                    self.emit_capture_args(c, args);
                } else {
                    self.emit_ctor_args(c, args);
                }
                if let Some(touch) = touch {
                    self.close_touch(touch);
                }
                if init {
                    self.out.push(')');
                }
            }
            TExpr::Lambda(params, body) => {
                if let Some(target) = self.forwarded_function(params, body) {
                    let n = self.local_ref(target);
                    self.out.push_str(&n);
                    return;
                }
                if let (true, Some((n, HoleKind::Thunk))) = (params.is_empty(), self.hole_of(body)) {
                    let _ = write!(self.out, "$k{}", n);
                    return;
                }
                let ps = prog.sym_list(params).to_vec();
                self.enter_scope(Root::Lambda(e));
                let mark = self.emit_params(&ps, &[]);
                self.out.push_str(" => ");
                self.fn_depth += 1;
                if self.is_simple(body) && !matches!(self.peek(body), TExpr::Unit) {
                    // A template or a comma expression could open with a brace.
                    let needs_parens = matches!(self.peek(body), TExpr::Js(..) | TExpr::Block(..));
                    if needs_parens {
                        self.out.push('(');
                    }
                    self.enter_cond();
                    self.emit_value(body);
                    self.leave_cond();
                    if needs_parens {
                        self.out.push(')');
                    }
                } else {
                    self.emit_body(body);
                }
                self.fn_depth -= 1;
                self.leave_params(mark);
                self.leave_scope();
            }
            TExpr::If(c, t, Some(els)) => {
                self.out.push_str(open);
                // `a ? b : c ? d : e` is the conditional of `c` in the second branch.
                let wrap = self.is_conditional_level(c);
                if wrap {
                    self.out.push('(');
                }
                self.emit_value(c);
                if wrap {
                    self.out.push(')');
                }
                self.out.push_str(" ? ");
                self.enter_cond();
                self.emit_value(t);
                self.out.push_str(" : ");
                self.emit_value(els);
                self.leave_cond();
                self.out.push_str(close);
            }
            TExpr::Block(stmts, res) => {
                let effects: Vec<TExprId> = prog.stmts[stmts.range()]
                    .iter()
                    .filter_map(|s| match *s {
                        TStmt::Expr(x) if !self.is_idle(x) => Some(x),
                        _ => None,
                    })
                    .collect();
                let touched = match effects.as_slice() {
                    [x] => self.touched_object(*x),
                    _ => None,
                };
                if effects.is_empty() {
                    self.emit_expr(res);
                } else if let Some(c) = touched {
                    let touch = self.open_touch(c);
                    self.emit_expr(res);
                    self.close_touch(touch);
                } else {
                    self.out.push('(');
                    for x in effects {
                        self.emit_comma_operand(x);
                        self.out.push_str(", ");
                    }
                    self.emit_comma_operand(res);
                    self.out.push(')');
                }
            }
            TExpr::Prim(op, a, b) => self.emit_prim(op, a, b, bare),
            TExpr::Unary(op, a) => self.emit_unary(op, a, bare),
            TExpr::StrConcat(items) => {
                let items = prog.expr_list(items);
                self.out.push_str(open);
                let starts_with_string = items.first().map_or(false, |&i| {
                    matches!(self.peek(i), TExpr::Str(_) | TExpr::Char(_))
                });
                if !starts_with_string {
                    self.out.push_str("\"\" + ");
                }
                for (i, &item) in items.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(" + ");
                    }
                    match self.peek(item) {
                        TExpr::ToStr(inner, conv) if conv.kind() == StrKind::Plain => self.emit_operand(inner, " + ", " + "),
                        _ => self.emit_operand(item, " + ", " + "),
                    }
                }
                self.out.push_str(close);
            }
            TExpr::ToStr(inner, conv) => {
                let f = to_str_conv_text(prog, inner, conv);
                self.note_text(f);
                self.out.push_str(f);
                self.out.push('(');
                self.emit_value(inner);
                self.out.push(')');
            }
            TExpr::Js(template, args) => {
                let t = &prog.strings[template.idx()];
                let args = prog.expr_list(args);
                let wrap = !bare && matches!(template_shape(t), Shape::Compound { .. });
                if wrap {
                    self.out.push('(');
                }
                let bytes = t.as_bytes();
                let mut i = 0;
                let mut start = 0;
                // The first argument is always evaluated; where the others stand may not be.
                let mut first = true;
                while i < bytes.len() {
                    if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
                        let idx = (bytes[i + 1] - b'0') as usize;
                        let cond = !(first && idx == 0);
                        first = false;
                        if cond {
                            self.enter_cond();
                        }
                        if t[start..i].ends_with("...") {
                            self.out.push_str(&t[start..i - 3]);
                            self.emit_template_spread(args.get(idx).copied(), &t[i + 2..]);
                        } else {
                            self.out.push_str(&t[start..i]);
                            match args.get(idx) {
                                Some(&a) if self.arrow_needs_parens(a, &t[..i], &t[i + 2..]) => {
                                    self.out.push('(');
                                    self.emit_expr(a);
                                    self.out.push(')');
                                }
                                // A template that is a whole argument or element stands on its own.
                                Some(&a) if matches!(self.peek(a), TExpr::Js(..)) && Self::stands_alone(&t[..i], &t[i + 2..]) => self.emit_value(a),
                                Some(&a) => self.emit_expr(a),
                                None => self.out.push_str("undefined"),
                            }
                        }
                        if cond {
                            self.leave_cond();
                        }
                        i += 2;
                        start = i;
                    } else {
                        i += 1;
                    }
                }
                self.out.push_str(&t[start..]);
                if wrap {
                    self.out.push(')');
                }
            }
            TExpr::TypeTest(inner, _) if self.test_hole(HoleNode::TestExpr(e)).is_some() => {
                let n = self.test_hole(HoleNode::TestExpr(e)).unwrap();
                let _ = write!(self.out, "$k{}(", n);
                self.emit_value(inner);
                self.out.push(')');
            }
            TExpr::TypeTest(inner, test) => {
                if let TExpr::Local(s) = self.peek(inner) {
                    let n = self.local_ref(s);
                    self.emit_test_as(test, &n, bare);
                } else {
                    self.out.push_str("(($v) => ");
                    self.emit_test_as(test, "$v", true);
                    self.out.push_str(")(");
                    self.emit_value(inner);
                    self.out.push(')');
                }
            }
            TExpr::Cast(inner, op, _) => self.emit_cast(inner, op, bare),
            TExpr::SeqLit(items) => {
                match self.array_seq {
                    Some(c) => {
                        let n = self.class_name(c);
                        let _ = write!(self.out, "new {}(", n);
                    }
                    None => self.out.push('('),
                }
                self.out.push('[');
                for (i, &a) in prog.expr_list(items).iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.emit_value(a);
                }
                self.out.push_str("])");
            }
            TExpr::ArrayLit(items) => {
                self.out.push('[');
                for (i, &a) in prog.expr_list(items).iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.emit_value(a);
                }
                self.out.push(']');
            }
            TExpr::Index(r, i) => {
                self.emit_receiver(r);
                let _ = write!(self.out, "[{}]", i);
            }
            TExpr::JsImport(i) => {
                let binding = self.js_import(i);
                self.out.push_str(&binding);
            }
            TExpr::JsGlobal(name, guarded) => {
                let global = Self::js_global_ref(self.interner.get(name));
                if guarded {
                    let _ = write!(self.out, "{1}typeof {0} === \"undefined\" ? undefined : {0}{2}", global, open, close);
                } else {
                    self.out.push_str(&global);
                }
            }
            // A member of the global scope is the bare identifier.
            TExpr::JsSelect(r, name) if self.is_global_scope(r) => {
                let global = Self::js_global_ref(self.interner.get(name));
                self.out.push_str(&global);
            }
            TExpr::JsSelect(r, name) => {
                self.emit_receiver(r);
                let p = Self::js_prop_access(self.interner.get(name));
                self.out.push_str(&p);
            }
            TExpr::ObjLit(items) => {
                self.out.push_str("({");
                for (i, pair) in prog.expr_list(items).chunks(2).enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    match self.peek(pair[0]) {
                        TExpr::Str(s) if is_js_identifier(&prog.strings[s.idx()]) => {
                            self.out.push_str(&prog.strings[s.idx()]);
                        }
                        _ if self.hole_of(pair[0]).is_some() => {
                            self.out.push('[');
                            self.emit_expr(pair[0]);
                            self.out.push(']');
                        }
                        _ => self.emit_expr(pair[0]),
                    }
                    self.out.push_str(": ");
                    self.emit_value(pair[1]);
                }
                self.out.push_str("})");
            }
            TExpr::Spread(inner) => {
                self.out.push_str("...");
                self.emit_expr(inner);
            }
            TExpr::Null => self.out.push_str("null"),
            TExpr::Throw(inner, unwrap) => {
                self.note_text(THROW);
                self.out.push_str(THROW);
                self.out.push('(');
                self.emit_thrown(inner, unwrap);
                self.out.push(')');
            }
            TExpr::If(_, _, None) | TExpr::While(..) | TExpr::Assign(..) | TExpr::Match(..) | TExpr::Return(_)
            | TExpr::Try(_) => {
                unreachable!("non-simple expression")
            }
            TExpr::Splice(_) => unreachable!("a stored inline body is emitted nowhere"),
        }
    }

    /// What `throw e` throws: the raw value of a `JavaScriptException`, or `e` itself.
    fn emit_thrown(&mut self, e: TExprId, unwrap: bool) {
        if unwrap {
            self.note_text(UNWRAP_JS);
            self.out.push_str(UNWRAP_JS);
            self.out.push('(');
            self.emit_expr(e);
            self.out.push(')');
        } else {
            self.emit_expr(e);
        }
    }

    fn emit_receiver(&mut self, r: TExprId) {
        if let TExpr::Js(t, _) = self.peek(r) {
            if template_shape(&self.prog.strings[t.idx()]) != Shape::Atomic && self.hole_of(r).is_none() && self.site_of(r).is_none() {
                self.out.push('(');
                self.emit_value(r);
                self.out.push(')');
                return;
            }
        }
        let needs_parens = matches!(
            self.peek(r),
            TExpr::Int(_) | TExpr::Double(_) | TExpr::Lambda(..) | TExpr::Long(_)
        ) || self.is_closure(r)
            || !self.is_simple(r);
        if needs_parens {
            self.out.push('(');
        }
        self.emit_expr(r);
        if needs_parens {
            self.out.push(')');
        }
    }

    fn emit_static(&mut self, s: SymId) {
        let info = self.syms.sym(s);
        if let SymKind::EnumValue(case) = info.kind {
            self.emit_enum_value_ref(s, case);
            return;
        }
        let n = self.sym_name(s);
        if info.owner != Owner::Local {
            self.note_name(&n);
        }
        self.out.push_str(&n);
        if !self.const_vals[s.idx()] {
            self.out.push_str("()");
        }
    }

    pub(super) fn int_literal_nonzero(&self, e: TExprId) -> bool {
        matches!(self.peek(e), TExpr::Int(v) if v != 0 && v != -1)
    }

    /// `(a mid b)`, without the parentheses where `bare`. No comparison tells a negative zero
    /// from zero, so an `Int` remainder by a constant stands there without its `| 0`.
    fn emit_comparison(&mut self, a: TExprId, mid: &str, b: TExprId, bare: bool) {
        if !bare {
            self.out.push('(');
        }
        self.emit_compared(a, "(", mid);
        self.out.push_str(mid);
        self.emit_compared(b, mid, ")");
        if !bare {
            self.out.push(')');
        }
    }

    fn emit_compared(&mut self, e: TExprId, before: &str, after: &str) {
        let under = self.through_same(e);
        match self.peek(e) {
            TExpr::Prim(PrimOp::IntRem, x, d) if self.int_literal_nonzero(d) && !self.is_opaque(under) => {
                self.emit_binary("(", x, " % ", d, ")", false)
            }
            _ => self.emit_operand(e, before, after),
        }
    }

    /// `pre a mid b post`; without the outer parentheses of `pre` and `post` where `bare`.
    fn emit_binary(&mut self, pre: &str, a: TExprId, mid: &str, b: TExprId, post: &str, bare: bool) {
        let strip = bare && pre.starts_with('(') && post.ends_with(')');
        self.out.push_str(if strip { &pre[1..] } else { pre });
        self.emit_operand(a, pre, mid);
        self.out.push_str(mid);
        self.emit_operand(b, mid, post);
        self.out.push_str(if strip { &post[..post.len() - 1] } else { post });
    }

    /// An operand of a comma expression, where a template of any shape stands as it is.
    fn emit_comma_operand(&mut self, e: TExprId) {
        if matches!(self.peek(e), TExpr::Js(..)) {
            self.emit_value(e);
        } else {
            self.emit_expr(e);
        }
    }

    /// An operand between the texts `before` and `after`: an arrow function next to an operator
    /// takes it into its body without parentheses.
    fn emit_operand(&mut self, e: TExprId, before: &str, after: &str) {
        let arrow = (matches!(self.peek(e), TExpr::Lambda(..)) || self.is_closure(e)) && !self.is_opaque(self.through_same(e));
        if arrow && !Self::stands_alone(before, after) {
            self.out.push('(');
            self.emit_expr(e);
            self.out.push(')');
        } else {
            self.emit_expr(e);
        }
    }

    fn emit_prim(&mut self, op: PrimOp, a: TExprId, b: TExprId, bare: bool) {
        use PrimOp::*;
        if matches!(op, RefEq | RefNe | Eq | Ne) {
            self.hold_compared(a);
            self.hold_compared(b);
        }
        let (pre, mid, post) = prim_text(op, self.int_literal_nonzero(b));
        self.note_text(pre);
        match op {
            Lt | Le | Gt | Ge | RefEq | RefNe => self.emit_comparison(a, mid, b, bare),
            BoolAnd | BoolOr => {
                if !bare {
                    self.out.push('(');
                }
                self.emit_expr(a);
                self.out.push_str(mid);
                self.enter_cond();
                self.emit_expr(b);
                self.leave_cond();
                if !bare {
                    self.out.push(')');
                }
            }
            _ => self.emit_binary(pre, a, mid, b, post, bare),
        }
    }

    fn emit_unary(&mut self, op: UnOp, a: TExprId, bare: bool) {
        use UnOp::*;
        // A widening to the same JavaScript number: the operand as it stands.
        if op.same_number() {
            return self.emit_expr_as(a, bare);
        }
        // `!` makes a Boolean of the number that `&` and `|` give on its own.
        if let (BoolNot, TExpr::Prim(p @ (PrimOp::BoolStrictAnd | PrimOp::BoolStrictOr), x, y), None) = (op, self.peek(a), self.site_of(a)) {
            let mid = if p == PrimOp::BoolStrictAnd { " & " } else { " | " };
            return self.emit_binary("(!(", x, mid, y, "))", bare);
        }
        let (mut pre, mut post) = unary_text(op);
        self.note_text(pre);
        if bare && pre.starts_with('(') && post.ends_with(')') && !matches!(op, CharToInt) {
            pre = &pre[1..];
            post = &post[..post.len() - 1];
        }
        self.out.push_str(pre);
        if matches!(op, CharToInt | CharToLong) {
            self.emit_receiver(a);
        } else {
            self.emit_expr(a);
        }
        self.out.push_str(post);
    }

    /// `x.asInstanceOf[T]` as dotty's erasure leaves it: the runtime's helper that tests the value
    /// and gives it back (`$as` of a class, `$asA` of a trait by its number, `$asS` of a `String`,
    /// `$asT` of what another test written at the cast tells), the unboxing (`$uI` and kin), the failure of a cast to
    /// `Nothing`; the value itself for a cast left undecided, which no output holds.
    fn emit_cast(&mut self, inner: TExprId, op: CastOp, bare: bool) {
        let prog = self.prog;
        let test = match op {
            CastOp::Written => return self.emit_expr_as(inner, bare),
            CastOp::Nothing => return self.emit_helper_call("$asNothing", inner),
            CastOp::Unbox(test, _) => return self.emit_helper_call(unbox_helper(prog.tests[test.idx()]), inner),
            CastOp::Check(test, _) => test,
        };
        match prog.tests[test.idx()] {
            TypeTest::Class(c) => {
                self.hold(c, Held::Tested);
                self.note_text("$as");
                self.out.push_str("$as(");
                self.emit_value(inner);
                let n = self.class_name(c);
                let _ = write!(self.out, ", {})", n);
            }
            TypeTest::Str => self.emit_helper_call("$asS", inner),
            TypeTest::Trait(c) => {
                self.note_text("$asA");
                self.out.push_str("$asA(");
                self.emit_value(inner);
                let _ = write!(self.out, ", {}, ", self.class_number(c));
                // Named as a class's `$classOf` names it: qualified where the output carries the
                // qualified names, else simple.
                let name = if self.prog.uses_get_class() || self.reach.uses_class_of { self.qualified_name(c) } else { self.interner.get(self.syms.class(c).name).to_string() };
                js_string(&name, &mut self.out);
                self.out.push(')');
            }
            _ => {
                self.note_text("$asT");
                let name = self.cast_target_name(test);
                if let TExpr::Local(s) = self.peek(inner) {
                    let n = self.local_ref(s);
                    let _ = write!(self.out, "$asT({}, ", n);
                    self.emit_test_as(test, &n, true);
                } else {
                    self.out.push_str("(($v) => $asT($v, ");
                    self.emit_test_as(test, "$v", true);
                }
                self.out.push_str(", ");
                js_string(&name, &mut self.out);
                self.out.push(')');
                if !matches!(self.peek(inner), TExpr::Local(_)) {
                    self.out.push_str(")(");
                    self.emit_value(inner);
                    self.out.push(')');
                }
            }
        }
    }

    fn emit_helper_call(&mut self, helper: &str, arg: TExprId) {
        self.note_text(helper);
        self.out.push_str(helper);
        self.out.push('(');
        self.emit_value(arg);
        self.out.push(')');
    }

    /// The class a failed cast names as its target, as the JVM names it, for a test other than a
    /// class's or a trait's.
    fn cast_target_name(&self, test: TestId) -> String {
        let prog = self.prog;
        match prog.tests[test.idx()] {
            TypeTest::Class(c) | TypeTest::Trait(c) => self.qualified_name(c),
            TypeTest::Or(_, b) => self.cast_target_name(b),
            TypeTest::Function(n) if n > 22 => "scala.runtime.FunctionXXL".to_string(),
            TypeTest::Function(n) => format!("scala.Function{}", n),
            t => t.boxed_class_name().to_string(),
        }
    }

    fn emit_test(&mut self, test: TestId, x: &str) {
        self.emit_test_as(test, x, false);
    }

    pub(super) fn emit_test_as(&mut self, test: TestId, x: &str, bare: bool) {
        let prog = self.prog;
        let (open, close) = if bare { ("", "") } else { ("(", ")") };
        let t = prog.tests[test.idx()];
        if let Some((text, compound)) = test_text(t) {
            self.note_text(text);
            let n = match t {
                TypeTest::Trait(c) => self.class_number(c),
                TypeTest::Function(n) => n as u32,
                _ => 0,
            };
            if compound {
                self.out.push_str(open);
            }
            let b = text.as_bytes();
            let mut start = 0;
            for i in 0..b.len() {
                if b[i] == b'$' && matches!(b.get(i + 1), Some(b'0' | b'1')) {
                    self.out.push_str(&text[start..i]);
                    if b[i + 1] == b'0' {
                        self.out.push_str(x);
                    } else {
                        let _ = write!(self.out, "{}", n);
                    }
                    start = i + 2;
                }
            }
            self.out.push_str(&text[start..]);
            if compound {
                self.out.push_str(close);
            }
            return;
        }
        match t {
            TypeTest::Class(c) => {
                self.hold(c, Held::Tested);
                let n = self.class_name(c);
                let _ = write!(self.out, "{}{} instanceof {}{}", open, x, n, close);
            }
            TypeTest::Value(v) => {
                self.hold_compared(v);
                let _ = write!(self.out, "{}{} === ", open, x);
                self.emit_expr(v);
                self.out.push_str(close);
            }
            TypeTest::Or(a, b) => {
                self.out.push_str(open);
                self.emit_test(a, x);
                self.out.push_str(" || ");
                self.emit_test(b, x);
                self.out.push_str(close);
            }
            TypeTest::And(a, b) => {
                self.out.push_str(open);
                self.emit_test(a, x);
                self.out.push_str(" && ");
                self.emit_test(b, x);
                self.out.push_str(close);
            }
            _ => unreachable!("a test written from its text"),
        }
    }

    // ---- statements ----

    fn finish(&mut self, e: TExprId, ctx: &Ctx) {
        if matches!(ctx, Ctx::Return) {
            if let Some((recv, args)) = self.tail_call(e) {
                self.emit_tail_jump(recv, args);
                return;
            }
        }
        if !matches!(ctx, Ctx::Discard) || !self.is_idle(e) {
            self.init_stmt(e);
        }
        self.line();
        match ctx {
            Ctx::Return if matches!(self.peek(e), TExpr::Unit) => self.out.push_str("return"),
            Ctx::Return => {
                self.out.push_str("return ");
                self.emit_value(e);
            }
            Ctx::Discard => {
                if self.is_idle(e) {
                    self.out.truncate(self.out.trim_end().len());
                    let len = self.out.len();
                    if let Some(f) = self.frames.last_mut() {
                        f.mark = f.mark.map(|(at, indent)| (at.min(len), indent));
                    }
                    return;
                }
                if let Some(c) = self.touched_object(e) {
                    self.out.truncate(self.out.trim_end().len());
                    self.emit_accessor_stmt(c);
                    return;
                }
                let starts_ambiguous = matches!(self.peek(e), TExpr::Lambda(..) | TExpr::Js(..)) || self.is_closure(e);
                if starts_ambiguous {
                    self.out.push('(');
                }
                self.emit_value(e);
                if starts_ambiguous {
                    self.out.push(')');
                }
            }
            Ctx::Assign(target) => {
                self.out.push_str(target);
                self.out.push_str(" = ");
                self.emit_value(e);
            }
        }
        self.out.push(';');
        self.checked = None;
    }

    /// An object read for its body to run, in statement position.
    fn touched_object(&self, e: TExprId) -> Option<ClassId> {
        match self.peek(e) {
            TExpr::Module(c) if self.syms.class(c).js_binding.is_none() => Some(c),
            _ => None,
        }
    }

    /// A statement that does nothing: `()`, or a reference to an object whose constructor runs
    /// no body, which the typer leaves in front of a member reached through the object's exports.
    pub(super) fn is_idle(&self, e: TExprId) -> bool {
        match self.peek(e) {
            TExpr::Unit => true,
            TExpr::Module(c) => !self.has_body[c.idx()],
            _ => false,
        }
    }

    /// The self call that `e` is, inside a def emitted as a loop.
    fn tail_call(&self, e: TExprId) -> Option<(Option<TExprId>, ListRef)> {
        let tail = self.tail.as_ref()?;
        let fun = &self.prog.funs[tail.fun.idx()];
        self.prog.self_call(fun.sym, fun.params.len(), e)
    }

    /// Whether a tail position of `e` holds such a call, which keeps an `if` in statement form.
    fn jumps(&self, e: TExprId) -> bool {
        let Some(tail) = &self.tail else { return false };
        let fun = &self.prog.funs[tail.fun.idx()];
        self.prog.tail_self_calls(fun.sym, fun.params.len(), e).0 > 0
    }

    /// Assigns the parameters of the loop and continues it. The arguments read the copies of the
    /// current iteration; a left-out default argument reads the parameters already assigned, as
    /// a default of the JS function does.
    fn emit_tail_jump(&mut self, recv: Option<TExprId>, args: ListRef) {
        let tail = self.tail.as_ref().unwrap();
        let params = tail.params.clone();
        let receiver = tail.receiver.clone();
        let fun = &self.prog.funs[tail.fun.idx()];
        let syms = fun.params.clone();
        let defaults = fun.defaults.clone();
        for (i, &a) in self.prog.expr_list(args).iter().enumerate() {
            self.line();
            let _ = write!(self.out, "{} = ", params[i]);
            match defaults[i] {
                Some(d) if matches!(self.prog.expr(a), TExpr::Unit) => {
                    let plain: Vec<Option<Rc<str>>> = syms.iter().map(|&p| self.sym_names[p.idx()].take()).collect();
                    for (&p, name) in syms.iter().zip(&params) {
                        self.sym_names[p.idx()] = Some(name.clone());
                    }
                    self.emit_value(d);
                    for (&p, name) in syms.iter().zip(plain) {
                        self.sym_names[p.idx()] = name;
                    }
                }
                _ => self.emit_value(a),
            }
            self.out.push(';');
        }
        if let (Some(receiver), Some(r)) = (receiver, recv) {
            self.line();
            let _ = write!(self.out, "{} = ", receiver);
            self.emit_expr(r);
            self.out.push(';');
        }
        self.line();
        self.out.push_str("continue;");
    }

    pub fn emit_stmt(&mut self, e: TExprId, ctx: &Ctx) {
        // The target of an assignment is written inside the blocks of the value assigned.
        let Ctx::Assign(target) = ctx else { return self.emit_stmt_braced(e, ctx) };
        self.assign_targets.push(Rc::from(super::scope::identifier_of(target)));
        self.emit_stmt_braced(e, ctx);
        self.assign_targets.pop();
    }

    fn emit_stmt_braced(&mut self, e: TExprId, ctx: &Ctx) {
        if self.is_opaque(e) {
            return self.finish(e, ctx);
        }
        let e = self.through_same(e);
        if self.is_opaque(e) {
            return self.finish(e, ctx);
        }
        if let TExpr::Block(stmts, _) = self.prog.expr(e) {
            if !stmts.is_empty() {
                self.line();
                self.open("{");
                self.enter_scope(Root::Expr(e));
                self.emit_stmt_unbraced(e, ctx);
                self.leave_scope();
                self.close("}");
                return;
            }
        }
        self.emit_stmt_unbraced(e, ctx);
    }

    pub(super) fn emit_block_stmts(&mut self, stmts: ListRef) {
        let items = &self.prog.stmts[stmts.range()];
        self.register_decls(items);
        // Local functions first, so that forward references work as in Scala.
        for s in items {
            if let TStmt::Fun(f) = s {
                self.mark_stmt();
                self.emit_function(*f, FunKind::Local);
            }
        }
        for s in items {
            self.mark_stmt();
            match *s {
                TStmt::Fun(_) => {}
                TStmt::Expr(x) => self.emit_stmt(x, &Ctx::Discard),
                TStmt::Val(sym, init) => self.emit_local_val(sym, init),
                TStmt::Pat(pat, init) => self.emit_pattern_val(pat, init),
            }
        }
    }

    /// Emits the statements of `e` directly into the current JS block.
    pub fn emit_stmt_unbraced(&mut self, e: TExprId, ctx: &Ctx) {
        if self.is_opaque(e) {
            return self.finish(e, ctx);
        }
        let e = self.through_same(e);
        if self.is_opaque(e) {
            return self.finish(e, ctx);
        }
        let prog = self.prog;
        match prog.expr(e) {
            TExpr::Block(stmts, res) => {
                self.emit_block_stmts(stmts);
                self.mark_stmt();
                self.emit_stmt(res, ctx);
            }
            TExpr::If(c, t, els)
                if !self.is_simple(e) || !matches!(ctx, Ctx::Return) || els.is_none() || self.jumps(e) =>
            {
                self.line();
                self.out.push_str("if (");
                self.emit_value(c);
                self.out.push_str(") ");
                self.enter_cond();
                self.open("{");
                self.enter_scope(Root::Expr(t));
                self.emit_stmt_unbraced(t, ctx);
                self.leave_scope();
                self.close("}");
                if let Some(els) = els {
                    self.out.push_str(" else ");
                    self.open("{");
                    self.enter_scope(Root::Expr(els));
                    self.emit_stmt_unbraced(els, ctx);
                    self.leave_scope();
                    self.close("}");
                }
                self.leave_cond();
            }
            TExpr::While(c, body) => {
                self.line();
                self.out.push_str("while (");
                self.emit_value(c);
                self.out.push_str(") ");
                self.enter_cond();
                self.open("{");
                self.enter_scope(Root::Expr(body));
                self.emit_stmt_unbraced(body, &Ctx::Discard);
                self.leave_scope();
                self.close("}");
                self.leave_cond();
            }
            TExpr::Assign(target, value) => {
                let mut lhs = String::new();
                std::mem::swap(&mut lhs, &mut self.out);
                self.swapped += 1;
                self.emit_assign_target(target);
                self.swapped -= 1;
                std::mem::swap(&mut lhs, &mut self.out);
                if let Some(setter) = lhs.strip_suffix("$set") {
                    if let (TExpr::Static(s), false) = (self.prog.expr(target), self.runs_nothing(value)) {
                        if let Some(file) = self.needs_init(s, None) {
                            self.write_init_stmt(file);
                        }
                    }
                    self.line();
                    let _ = write!(self.out, "{}$set(", setter);
                    self.emit_value(value);
                    self.out.push_str(");");
                } else if self.is_simple(value) {
                    self.line();
                    self.out.push_str(&lhs);
                    self.out.push_str(" = ");
                    self.emit_value(value);
                    self.out.push(';');
                } else {
                    self.emit_stmt(value, &Ctx::Assign(lhs));
                }
            }
            TExpr::Match(scrut, cases) => self.emit_match(scrut, cases, ctx),
            TExpr::Return(v) if self.fn_depth == 0 => self.emit_stmt_unbraced(v, &Ctx::Return),
            TExpr::Return(v) => {
                let key = match &self.nonlocal_return {
                    Some(k) => k.clone(),
                    None => {
                        let k: Rc<str> = Rc::from(self.fresh_in("nlr", self.def_scope));
                        self.nonlocal_return = Some(k.clone());
                        k
                    }
                };
                self.line();
                let _ = write!(self.out, "throw {{$nlr: {}, v: ", key);
                self.emit_expr(v);
                self.out.push_str("};");
            }
            TExpr::Throw(inner, unwrap) => {
                self.line();
                self.out.push_str("throw ");
                self.emit_thrown(inner, unwrap);
                self.out.push(';');
            }
            TExpr::Try(i) => self.emit_try(i, ctx),
            // `a || b` is `if a then true else b`, so a tail call in `b` jumps.
            TExpr::Prim(op @ (PrimOp::BoolAnd | PrimOp::BoolOr), a, b)
                if matches!(ctx, Ctx::Return) && self.jumps(b) =>
            {
                let is_or = op == PrimOp::BoolOr;
                self.line();
                self.out.push_str(if is_or { "if (" } else { "if (!(" });
                self.emit_value(a);
                self.out.push_str(if is_or { ") return true;" } else { ")) return false;" });
                self.enter_cond();
                self.emit_stmt_unbraced(b, ctx);
                self.leave_cond();
            }
            _ => self.finish(e, ctx),
        }
    }

    fn emit_assign_target(&mut self, target: TExprId) {
        match self.prog.expr(target) {
            TExpr::Static(s) => {
                let n = self.sym_name(s);
                if self.syms.sym(s).owner != Owner::Local {
                    self.note_name(&format!("{}set", n));
                }
                self.out.push_str(&n);
                self.out.push_str("set");
            }
            TExpr::Field(r, s) if self.syms.js_member(s) => match self.global_scope_member(r, s) {
                Some(name) => self.out.push_str(&name),
                None => {
                    self.emit_receiver(r);
                    let p = self.js_prop(s);
                    self.out.push_str(&p);
                }
            },
            TExpr::Field(r, s) => {
                self.emit_receiver(r);
                self.out.push('.');
                let n = self.field_name(s);
                self.out.push_str(&n);
            }
            _ => self.emit_expr(target),
        }
    }

    fn emit_local_val(&mut self, sym: SymId, init: TExprId) {
        let name = self.sym_name(sym);
        if self.syms.sym(sym).mods & mods::LAZY != 0 {
            self.line();
            let _ = write!(self.out, "let {0}$v, {0}$d = false;", name);
            self.line();
            let _ = write!(self.out, "const {0} = () => ", name);
            self.open("{");
            self.enter_scope(Root::Expr(init));
            self.line();
            let _ = write!(self.out, "if (!{0}$d) ", name);
            self.open("{");
            self.enter_scope(Root::Expr(init));
            self.enter_cond();
            self.emit_stmt(init, &Ctx::Assign(format!("{}$v", name)));
            self.leave_cond();
            self.line();
            let _ = write!(self.out, "{}$d = true;", name);
            self.leave_scope();
            self.leave_scope();
            self.close("}");
            self.line();
            let _ = write!(self.out, "return {}$v;", name);
            self.close("};");
            return;
        }
        let mutable = self.syms.sym(sym).kind == SymKind::Var;
        if self.is_simple(init) {
            self.line();
            let _ = write!(self.out, "{} {} = ", if mutable { "let" } else { "const" }, name);
            self.emit_value(init);
            self.out.push(';');
        } else {
            self.line();
            let _ = write!(self.out, "let {};", name);
            self.emit_stmt(init, &Ctx::Assign(name.to_string()));
        }
    }

    /// `val (a, b) = (x, y)` binds `a` to `x` and `b` to `y` without making the tuple; any other
    /// pattern is tested and taken apart on the value.
    fn emit_pattern_val(&mut self, pat: TPatId, init: TExprId) {
        let prog = self.prog;
        match (prog.pats[pat.idx()], self.peek(init)) {
            (TPat::Bind(sym, None), _) => return self.emit_local_val(sym, init),
            (TPat::Wildcard, _) => {
                if !self.is_pure_value(init) {
                    self.emit_stmt(init, &Ctx::Discard);
                }
                return;
            }
            (TPat::Test(test, _, inner), _) if matches!(prog.tests[test.idx()], TypeTest::Always) && self.test_hole(HoleNode::TestPat(pat)).is_none() => {
                return self.emit_pattern_val(inner, init);
            }
            (TPat::Class(c, _, _, subs), TExpr::New(made, args)) if made == c && self.is_tuple_class(c) && subs.len == args.len => {
                let subs = prog.pat_lists[subs.range()].to_vec();
                let args = prog.expr_list(args).to_vec();
                for (sub, arg) in subs.into_iter().zip(args) {
                    self.emit_pattern_val(sub, arg);
                }
                return;
            }
            _ => {}
        }
        let tmp = self.fresh("d");
        self.line();
        let _ = write!(self.out, "let {};", tmp);
        if self.is_simple(init) {
            self.line();
            let _ = write!(self.out, "{} = ", tmp);
            self.emit_value(init);
            self.out.push(';');
        } else {
            self.emit_stmt(init, &Ctx::Assign(tmp.clone()));
        }
        let mut steps = Vec::new();
        self.pattern_steps(pat, &tmp, &mut steps);
        for step in steps {
            self.line();
            match step {
                Step::Test(cond, _) => {
                    let _ = write!(self.out, "if (!({})) $matchError({});", cond, tmp);
                }
                Step::Bind(decl) => self.out.push_str(&decl),
            }
        }
    }

    /// A value whose evaluation has no effect and no cost worth a statement of its own.
    fn is_pure_value(&self, e: TExprId) -> bool {
        matches!(
            self.peek(e),
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_)
                | TExpr::Unit | TExpr::Local(_) | TExpr::This | TExpr::Lambda(..)
        ) || self.is_closure(e)
    }

    fn is_tuple_class(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        matches!(info.owner, Owner::Package(p) if self.interner.get(self.syms.pkg(p).name) == "scala")
            && self.is_tuple_name(self.interner.get(info.name))
    }

    fn expr_to_string(&mut self, e: TExprId) -> String {
        let mut buf = String::new();
        std::mem::swap(&mut buf, &mut self.out);
        self.swapped += 1;
        self.emit_expr(e);
        self.swapped -= 1;
        std::mem::swap(&mut buf, &mut self.out);
        buf
    }

    fn test_to_string(&mut self, t: TestId, x: &str) -> String {
        let mut buf = String::new();
        std::mem::swap(&mut buf, &mut self.out);
        self.swapped += 1;
        self.emit_test(t, x);
        self.swapped -= 1;
        std::mem::swap(&mut buf, &mut self.out);
        buf
    }

    /// Flattens a pattern into a sequence of tests and bindings against `scrut`.
    fn pattern_steps(&mut self, pat: TPatId, scrut: &str, out: &mut Vec<Step>) {
        let prog = self.prog;
        match prog.pats[pat.idx()] {
            TPat::Wildcard => {}
            TPat::Bind(sym, inner) => {
                let n = self.sym_name(sym);
                out.push(Step::Bind(format!("const {} = {};", n, scrut)));
                if let Some(i) = inner {
                    self.pattern_steps(i, scrut, out);
                }
            }
            TPat::Test(_, _, inner) if self.test_hole(HoleNode::TestPat(pat)).is_some() => {
                let n = self.test_hole(HoleNode::TestPat(pat)).unwrap();
                out.push(Step::Test(format!("$k{}({})", n, scrut), !scrut.starts_with('$')));
                self.pattern_steps(inner, scrut, out);
            }
            TPat::Test(test, _, inner) => {
                let tested_by_inner = matches!((prog.tests[test.idx()], prog.pats[inner.idx()]), (TypeTest::Class(a), TPat::Class(b, ..)) if a == b);
                if !matches!(prog.tests[test.idx()], TypeTest::Always) && !tested_by_inner {
                    let on_scrutinee = !scrut.starts_with('$') && !matches!(prog.tests[test.idx()], TypeTest::Value(_));
                    out.push(Step::Test(self.test_to_string(test, scrut), on_scrutinee));
                }
                self.pattern_steps(inner, scrut, out);
            }
            TPat::Equals(e, strict) => {
                self.hold_compared(e);
                let v = self.expr_to_string(e);
                out.push(Step::Test(
                    if strict { format!("{} === {}", scrut, v) } else { format!("$eq({}, {})", v, scrut) },
                    false,
                ));
            }
            TPat::Class(c, _, fields, subs) => {
                self.hold(c, Held::Tested);
                let n = self.class_name(c);
                out.push(Step::Test(format!("{} instanceof {}", scrut, n), !scrut.starts_with('$')));
                let fields = prog.sym_list(fields).to_vec();
                let subs = prog.pat_lists[subs.range()].to_vec();
                for (f, s) in fields.iter().zip(subs) {
                    let access = format!("{}.{}", scrut, self.field_name(*f));
                    match prog.pats[s.idx()] {
                        TPat::Wildcard => {}
                        TPat::Bind(_, None) | TPat::Equals(..) => self.pattern_steps(s, &access, out),
                        _ => {
                            let tmp = self.fresh("p");
                            out.push(Step::Bind(format!("const {} = {};", tmp, access)));
                            self.pattern_steps(s, &tmp, out);
                        }
                    }
                }
            }
            TPat::Alt(alts) => {
                let alts = prog.pat_lists[alts.range()].to_vec();
                let conds: Vec<String> = alts.iter().map(|&a| self.pattern_cond(a, scrut)).collect();
                out.push(Step::Test(conds.join(" || "), false));
            }
            TPat::Unapply(sym, call, inner) => {
                let n = self.sym_name(sym);
                out.push(Step::Bind(format!("const {} = {};", n, scrut)));
                let tmp = self.fresh("u");
                let value = self.expr_to_string(call);
                out.push(Step::Bind(format!("const {} = {};", tmp, value)));
                self.pattern_steps(inner, &tmp, out);
            }
            TPat::Seq(items, rest) => {
                let items = prog.pat_lists[items.range()].to_vec();
                let tmp = self.fresh("q");
                let wrap = match (rest, self.array_seq) {
                    (Some(_), Some(c)) => format!(", {}", self.class_name(c)),
                    _ => String::new(),
                };
                let n = items.len();
                out.push(Step::Bind(format!("const {} = $seqPat({}, {}, {}{});", tmp, scrut, n, rest.is_some(), wrap)));
                out.push(Step::Test(format!("{} !== null", tmp), false));
                for (i, s) in items.iter().chain(rest.iter()).enumerate() {
                    let access = format!("{}[{}]", tmp, i);
                    match prog.pats[s.idx()] {
                        TPat::Wildcard => {}
                        TPat::Bind(_, None) | TPat::Equals(..) => self.pattern_steps(*s, &access, out),
                        _ => {
                            let elem = self.fresh("p");
                            out.push(Step::Bind(format!("const {} = {};", elem, access)));
                            self.pattern_steps(*s, &elem, out);
                        }
                    }
                }
            }
        }
    }

    fn pattern_binds(&mut self, pat: TPatId, name: &str) -> bool {
        let prog = self.prog;
        match prog.pats[pat.idx()] {
            TPat::Bind(sym, inner) => {
                &*self.sym_name(sym) == name || inner.map_or(false, |i| self.pattern_binds(i, name))
            }
            TPat::Test(_, _, inner) | TPat::Unapply(_, _, inner) => self.pattern_binds(inner, name),
            TPat::Class(_, _, _, subs) | TPat::Alt(subs) => {
                prog.pat_lists[subs.range()].iter().any(|&s| self.pattern_binds(s, name))
            }
            TPat::Seq(items, rest) => {
                prog.pat_lists[items.range()].iter().chain(rest.iter()).any(|&s| self.pattern_binds(s, name))
            }
            TPat::Wildcard | TPat::Equals(..) => false,
        }
    }

    /// A single boolean expression for a pattern without bindings.
    fn pattern_cond(&mut self, pat: TPatId, scrut: &str) -> String {
        let mut steps = Vec::new();
        self.pattern_steps(pat, scrut, &mut steps);
        let mut conds: Vec<String> = Vec::new();
        let mut substitutions: Vec<(String, String)> = Vec::new();
        for s in steps {
            match s {
                Step::Test(mut c, _) => {
                    for (tmp, access) in &substitutions {
                        c = c.replace(tmp.as_str(), access.as_str());
                    }
                    conds.push(format!("({})", c));
                }
                Step::Bind(decl) => {
                    // `const $pN = access;` temporaries are inlined into the condition.
                    if let Some(rest) = decl.strip_prefix("const ") {
                        if let Some((name, value)) = rest.trim_end_matches(';').split_once(" = ") {
                            let mut value = value.to_string();
                            for (tmp, access) in &substitutions {
                                value = value.replace(tmp.as_str(), access.as_str());
                            }
                            substitutions.push((name.to_string(), value));
                        }
                    }
                }
            }
        }
        if conds.is_empty() {
            "true".to_string()
        } else {
            format!("({})", conds.join(" && "))
        }
    }

    /// True when emitting `e` in return position is guaranteed to end in a `return`, a throw or a
    /// jump of the tail loop.
    pub(super) fn always_returns(&self, e: TExprId) -> bool {
        match self.prog.expr(self.through_same(e)) {
            TExpr::Block(_, res) => self.always_returns(res),
            TExpr::If(_, t, Some(els)) => self.always_returns(t) && self.always_returns(els),
            TExpr::If(_, _, None) | TExpr::While(..) | TExpr::Assign(..) => false,
            TExpr::Try(i) => {
                let t = &self.prog.tries[i as usize];
                self.always_returns(t.body) && self.prog.cases[t.cases.range()].iter().all(|c| self.always_returns(c.body))
            }
            _ => true,
        }
    }

    /// `try`/`catch`/`finally` as the JavaScript statement: the cases match on what was caught,
    /// and a value no case takes is thrown again, as in Scala.
    fn emit_try(&mut self, i: u32, ctx: &Ctx) {
        let prog = self.prog;
        let t = &prog.tries[i as usize];
        self.line();
        // A `try` that handles nothing is its body, which JavaScript's `try` cannot be alone.
        if t.cases.is_empty() && t.finalizer.is_none() {
            self.open("{");
            self.enter_scope(Root::Expr(t.body));
            self.emit_stmt_unbraced(t.body, ctx);
            self.leave_scope();
            self.close("}");
            return;
        }
        self.open("try {");
        self.enter_scope(Root::Expr(t.body));
        self.emit_stmt_unbraced(t.body, ctx);
        self.leave_scope();
        self.close("}");
        if !t.cases.is_empty() {
            self.enter_scope(Root::Cases(t.cases.start, t.cases.len));
            let raw = self.fresh("e");
            let _ = write!(self.out, " catch ({}) ", raw);
            self.open("{");
            let scrut = if t.wraps {
                let wrapped = self.fresh("e");
                self.line();
                let _ = write!(self.out, "const {} = $wrapJs({});", wrapped, raw);
                wrapped
            } else {
                raw.clone()
            };
            let miss = format!("throw {};", raw);
            self.emit_cases(&scrut, t.cases, ctx, &miss);
            self.leave_scope();
            self.close("}");
        }
        if let Some(f) = t.finalizer {
            self.out.push_str(" finally ");
            self.open("{");
            self.enter_scope(Root::Expr(f));
            self.emit_stmt_unbraced(f, &Ctx::Discard);
            self.leave_scope();
            self.close("}");
        }
    }

    fn emit_match(&mut self, scrut: TExprId, cases: ListRef, ctx: &Ctx) {
        let prog = self.prog;
        let clauses = &prog.cases[cases.range()];
        let named = match self.peek(scrut) {
            TExpr::Local(s) => Some(self.local_ref(s)),
            TExpr::This => Some(self.this_name.to_string()),
            _ => None,
        };
        // `x match { case x: T => .. }` declares another `x`, which would hide the scrutinee.
        let returns = matches!(ctx, Ctx::Return);
        let binds = |this: &mut Self, n: &str| {
            let blocks = this.case_blocks(n, cases);
            (0..clauses.len()).any(|k| {
                let root = this.case_root(blocks[k], cases.start + k as u32, cases, !returns);
                if let Some(r) = root {
                    this.enter_pending(r);
                }
                let binds = this.pattern_binds(clauses[k].pat, n);
                if root.is_some() {
                    this.leave_pending();
                }
                binds
            })
        };
        let scrut_name = match named.filter(|n| !binds(self, n)) {
            Some(name) => name,
            None => {
                let tmp = self.fresh("s");
                if self.is_simple(scrut) {
                    self.line();
                    let _ = write!(self.out, "const {} = ", tmp);
                    self.emit_value(scrut);
                    self.out.push(';');
                } else {
                    self.line();
                    let _ = write!(self.out, "let {};", tmp);
                    self.emit_stmt(scrut, &Ctx::Assign(tmp.clone()));
                }
                tmp
            }
        };
        let miss = format!("$matchError({});", scrut_name);
        self.emit_cases(&scrut_name, cases, ctx, &miss);
    }

    /// The cases against the value in `scrut_name` as a sequence of `if`s, each ending in a
    /// return or a break out of the labelled block that stands in for the match in statement
    /// position, then the statement `miss` when none of them takes the value; a last case that
    /// always applies is written flat, without a test or the `miss` that would follow it.
    fn emit_cases(&mut self, scrut_name: &str, list: ListRef, ctx: &Ctx, miss: &str) {
        let prog = self.prog;
        let cases = &prog.cases[list.range()];
        let returns = matches!(ctx, Ctx::Return);
        let label = if returns { String::new() } else { self.fresh_label("m") };
        if !returns {
            self.line();
            let _ = write!(self.out, "{}: ", label);
            self.open("{");
            self.enter_scope(Root::Cases(list.start, list.len));
        }
        self.enter_cond();
        let mut exhausted = false;
        let blocks = self.case_blocks(scrut_name, list);
        let mut steps: Vec<Vec<Step>> = Vec::with_capacity(cases.len());
        for (k, case) in cases.iter().enumerate() {
            let root = self.case_root(blocks[k], list.start + k as u32, list, false);
            if let Some(r) = root {
                self.enter_pending(r);
            }
            let mut own = Vec::new();
            self.pattern_steps(case.pat, scrut_name, &mut own);
            if root.is_some() {
                self.leave_pending();
            }
            steps.push(own);
        }
        let shared_test = |steps: &[Step]| match steps.first() {
            Some(Step::Test(c, true)) => Some(c.clone()),
            _ => None,
        };
        let mut i = 0;
        while i < cases.len() {
            // Consecutive cases opening with the same type test on the scrutinee share it.
            let mut j = i + 1;
            if let Some(test) = shared_test(&steps[i]) {
                while j < cases.len() && shared_test(&steps[j]).as_deref() == Some(test.as_str()) {
                    j += 1;
                }
                if j > i + 1 {
                    debug_assert!((i..j).all(|k| blocks[k] == CaseBlock::Shared(list.start + i as u32, (j - i) as u32)));
                    self.line();
                    let _ = write!(self.out, "if ({}) ", test);
                    self.open("{");
                    self.enter_scope(Root::Cases(list.start + i as u32, (j - i) as u32));
                    for k in i..j {
                        let own = std::mem::take(&mut steps[k]).split_off(1);
                        self.emit_case(&cases[k], list.start + k as u32, own, false, true, returns, &label, ctx);
                    }
                    self.leave_scope();
                    self.close("}");
                    i = j;
                    continue;
                }
            }
            let own = std::mem::take(&mut steps[i]);
            let flat = i == cases.len() - 1 && cases[i].guard.is_none() && !own.iter().any(|s| matches!(s, Step::Test(..)));
            debug_assert!(blocks[i] == if flat { CaseBlock::Enclosing } else { CaseBlock::Own });
            self.emit_case(&cases[i], list.start + i as u32, own, !flat, !flat, returns, &label, ctx);
            exhausted = flat;
            i += 1;
        }
        if !exhausted {
            self.line();
            self.out.push_str(miss);
        }
        self.leave_cond();
        if !returns {
            self.leave_scope();
            self.close("}");
        }
    }

    /// Where the bindings of each case are declared, decided before the cases are taken apart
    /// as `emit_cases` decides it after: a case of several that open with the same type test of
    /// the scrutinee in the block they share, a last case that tests nothing in the block the
    /// cases are written in, any other in a block of its own.
    fn case_blocks(&mut self, scrut: &str, list: ListRef) -> Vec<CaseBlock> {
        let cases = &self.prog.cases[list.range()];
        let firsts: Vec<Option<String>> = cases.iter().map(|c| self.scrutinee_test(c.pat, scrut)).collect();
        let mut blocks = vec![CaseBlock::Own; cases.len()];
        let mut i = 0;
        while i < cases.len() {
            let mut j = i + 1;
            if let Some(test) = &firsts[i] {
                while j < cases.len() && firsts[j].as_deref() == Some(test.as_str()) {
                    j += 1;
                }
            }
            if j > i + 1 {
                for b in &mut blocks[i..j] {
                    *b = CaseBlock::Shared(list.start + i as u32, (j - i) as u32);
                }
                i = j;
                continue;
            }
            if i == cases.len() - 1 && cases[i].guard.is_none() && !self.pattern_tests(cases[i].pat) {
                blocks[i] = CaseBlock::Enclosing;
            }
            i += 1;
        }
        blocks
    }

    /// The scope a case's bindings are declared in, where a block holds them; `label` says
    /// that the labelled block of the cases is still to be opened.
    fn case_root(&self, block: CaseBlock, at: u32, list: ListRef, label: bool) -> Option<Root> {
        match block {
            CaseBlock::Own => Some(Root::Case(at)),
            CaseBlock::Shared(start, len) => Some(Root::Cases(start, len)),
            CaseBlock::Enclosing if label => Some(Root::Cases(list.start, list.len)),
            CaseBlock::Enclosing => None,
        }
    }

    /// The type test of the scrutinee that `pattern_steps` starts the case with, which cases
    /// next to each other share.
    fn scrutinee_test(&mut self, pat: TPatId, scrut: &str) -> Option<String> {
        let prog = self.prog;
        let on_scrutinee = !scrut.starts_with('$');
        match prog.pats[pat.idx()] {
            TPat::Test(_, _, _) if self.test_hole(HoleNode::TestPat(pat)).is_some() => {
                let n = self.test_hole(HoleNode::TestPat(pat)).unwrap();
                on_scrutinee.then(|| format!("$k{}({})", n, scrut))
            }
            TPat::Test(test, _, inner) => {
                let tested_by_inner = matches!((prog.tests[test.idx()], prog.pats[inner.idx()]), (TypeTest::Class(a), TPat::Class(b, ..)) if a == b);
                if !matches!(prog.tests[test.idx()], TypeTest::Always) && !tested_by_inner {
                    let value = matches!(prog.tests[test.idx()], TypeTest::Value(_));
                    (on_scrutinee && !value).then(|| self.test_to_string(test, scrut))
                } else {
                    self.scrutinee_test(inner, scrut)
                }
            }
            TPat::Class(c, ..) => on_scrutinee.then(|| format!("{} instanceof {}", scrut, self.class_name(c))),
            _ => None,
        }
    }

    /// Whether `pattern_steps` gives the pattern a test.
    fn pattern_tests(&self, pat: TPatId) -> bool {
        let prog = self.prog;
        match prog.pats[pat.idx()] {
            TPat::Wildcard => false,
            TPat::Bind(_, inner) => inner.map_or(false, |i| self.pattern_tests(i)),
            TPat::Test(..) if self.test_hole(HoleNode::TestPat(pat)).is_some() => true,
            TPat::Test(test, _, inner) => {
                let tested_by_inner = matches!((prog.tests[test.idx()], prog.pats[inner.idx()]), (TypeTest::Class(a), TPat::Class(b, ..)) if a == b);
                (!matches!(prog.tests[test.idx()], TypeTest::Always) && !tested_by_inner) || self.pattern_tests(inner)
            }
            TPat::Equals(..) | TPat::Class(..) | TPat::Alt(_) | TPat::Seq(..) => true,
            TPat::Unapply(_, _, inner) => self.pattern_tests(inner),
        }
    }

    /// One case: its tests as one condition, its bindings, its guard and its body. Bindings and
    /// body get a block of their own where no test opened one and `block` asks for it; the body
    /// ends in a return or in a break out of the labelled block where cases follow (`exits`).
    #[allow(clippy::too_many_arguments)]
    fn emit_case(&mut self, case: &TCase, at: u32, steps: Vec<Step>, block: bool, exits: bool, returns: bool, label: &str, ctx: &Ctx) {
        let mut depth = 0;
        let mut pending: Vec<String> = Vec::new();
        let flush = |this: &mut Self, pending: &mut Vec<String>, depth: &mut usize| {
            if !pending.is_empty() {
                this.line();
                let _ = write!(this.out, "if ({}) ", pending.join(" && "));
                this.open("{");
                this.enter_scope(Root::Case(at));
                *depth += 1;
                pending.clear();
            }
        };
        for step in steps {
            match step {
                Step::Test(c, _) => pending.push(c),
                Step::Bind(decl) => {
                    flush(self, &mut pending, &mut depth);
                    if depth == 0 && block {
                        self.line();
                        self.open("{");
                        self.enter_scope(Root::Case(at));
                        depth += 1;
                    }
                    self.line();
                    self.out.push_str(&decl);
                }
            }
        }
        flush(self, &mut pending, &mut depth);
        if let Some(g) = case.guard {
            self.line();
            self.out.push_str("if (");
            self.emit_value(g);
            self.out.push_str(") ");
            self.open("{");
            self.enter_scope(Root::Case(at));
            depth += 1;
        }
        if depth == 0 && block {
            self.line();
            self.open("{");
            self.enter_scope(Root::Case(at));
            depth += 1;
        }
        self.emit_stmt_unbraced(case.body, ctx);
        if !returns {
            if exits {
                self.line();
                let _ = write!(self.out, "break {};", label);
            }
        } else if exits && !self.always_returns(case.body) {
            // A body made of statements (assignment, loop, one-armed if) emits no return and
            // would otherwise fall through into the following cases.
            self.line();
            self.out.push_str("return;");
        }
        for _ in 0..depth {
            self.leave_scope();
            self.close("}");
        }
    }
}

/// Where the bindings of a case are declared (`Emitter::case_blocks`).
#[derive(Clone, Copy, PartialEq, Debug)]
enum CaseBlock {
    Own,
    /// In the block of the cases that share their first test: the first case and how many.
    Shared(u32, u32),
    /// In the block the cases are written in.
    Enclosing,
}

pub enum Step {
    /// A condition, and whether it is a type test on the scrutinee, which consecutive cases share.
    Test(String, bool),
    Bind(String),
}

/// What a template is at its top level, outside its brackets and literals.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Shape {
    /// A primary, member, call or `new` expression: an operand and a receiver as it stands.
    Atomic,
    /// A prefix operator in front of one (`-128`, `void $0.f()`): an operand, not a receiver.
    Unary,
    /// A binary operator, a conditional, an assignment, an arrow or a comma: parentheses as an
    /// operand; `conditional` for the kinds that also need them as a conditional's condition.
    Compound { conditional: bool },
}

fn template_shape(t: &str) -> Shape {
    let b = t.trim().as_bytes();
    let mut i = 0;
    let mut unary = false;
    loop {
        let rest = &b[i..];
        if let Some(n) = [&b"void "[..], b"typeof ", b"delete ", b"await "].iter().find(|p| rest.starts_with(p)).map(|p| p.len()) {
            i += n;
        } else if matches!(rest.first(), Some(b'-' | b'+' | b'!' | b'~')) && !rest.starts_with(b"--") && !rest.starts_with(b"++") {
            i += 1;
        } else {
            break;
        }
        unary = true;
    }
    if b[i..].starts_with(b"new ") {
        i += 4;
    }
    if b.get(i) == Some(&b'/') {
        // A regular expression literal: up to the `/` that closes it, outside a class.
        i += 1;
        let mut class = false;
        while i < b.len() && (class || b[i] != b'/') {
            match b[i] {
                b'\\' => i += 1,
                b'[' => class = true,
                b']' => class = false,
                _ => {}
            }
            i += 1;
        }
        i += 1;
    }
    let mut depth = 0usize;
    let mut compound = false;
    let mut conditional = false;
    while i < b.len() {
        let c = b[i];
        match c {
            b'"' | b'\'' | b'`' => {
                i += 1;
                while i < b.len() && b[i] != c {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            _ if depth > 0 => {}
            b'?' if b.get(i + 1) == Some(&b'.') => i += 1,
            b'?' if b.get(i + 1) == Some(&b'?') => {
                compound = true;
                i += 1;
            }
            b'?' | b',' => {
                compound = true;
                conditional = true;
            }
            b'=' => {
                let before = if i > 0 { b[i - 1] } else { 0 };
                let after = b.get(i + 1).copied().unwrap_or(0);
                compound = true;
                if after == b'>' || (after != b'=' && !matches!(before, b'=' | b'!' | b'<' | b'>')) {
                    conditional = true;
                }
                if after == b'=' || after == b'>' {
                    i += 1;
                }
            }
            _ if c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c == b'.' => {}
            _ => compound = true,
        }
        i += 1;
    }
    match (compound, unary) {
        (true, _) => Shape::Compound { conditional },
        (false, true) => Shape::Unary,
        (false, false) => Shape::Atomic,
    }
}

// The texts the emitter writes around the operands of its operations, which the emission writes
// and the walk of a scope reads the names of (`scope.rs`): a local named like a global one of
// them uses is named apart where the scope writes it, and only there.

/// Per binary operation, in the order of `PrimOp`: the text before the left operand, between
/// the two and after the right one.
const PRIM_TEXT: [(PrimOp, &str, &str, &str); 45] = {
    use PrimOp::*;
    [
        (IntAdd, "(", " + ", " | 0)"),
        (IntSub, "(", " - ", " | 0)"),
        (IntMul, "Math.imul(", ", ", ")"),
        (IntDiv, "$idiv(", ", ", ")"),
        (IntRem, "$irem(", ", ", ")"),
        (IntAnd, "(", " & ", ")"),
        (IntOr, "(", " | ", ")"),
        (IntXor, "(", " ^ ", ")"),
        (IntShl, "(", " << ", ")"),
        (IntShr, "(", " >> ", ")"),
        (IntUshr, "(", " >>> ", " | 0)"),
        (LongAdd, "BigInt.asIntN(64, ", " + ", ")"),
        (LongSub, "BigInt.asIntN(64, ", " - ", ")"),
        (LongMul, "BigInt.asIntN(64, ", " * ", ")"),
        (LongDiv, "$ldiv(", ", ", ")"),
        (LongRem, "$lrem(", ", ", ")"),
        (LongAnd, "(", " & ", ")"),
        (LongOr, "(", " | ", ")"),
        (LongXor, "(", " ^ ", ")"),
        (LongShl, "BigInt.asIntN(64, ", " << (", " & 63n))"),
        (LongShr, "(", " >> (", " & 63n))"),
        (LongUshr, "$lushr(", ", ", ")"),
        (DoubleAdd, "(", " + ", ")"),
        (DoubleSub, "(", " - ", ")"),
        (DoubleMul, "(", " * ", ")"),
        (DoubleDiv, "(", " / ", ")"),
        (DoubleRem, "(", " % ", ")"),
        (FloatAdd, "Math.fround(", " + ", ")"),
        (FloatSub, "Math.fround(", " - ", ")"),
        (FloatMul, "Math.fround(", " * ", ")"),
        (FloatDiv, "Math.fround(", " / ", ")"),
        (FloatRem, "Math.fround(", " % ", ")"),
        (Lt, "(", " < ", ")"),
        (Le, "(", " <= ", ")"),
        (Gt, "(", " > ", ")"),
        (Ge, "(", " >= ", ")"),
        (RefEq, "(", " === ", ")"),
        (RefNe, "(", " !== ", ")"),
        (Eq, "$eq(", ", ", ")"),
        (Ne, "!$eq(", ", ", ")"),
        (BoolAnd, "(", " && ", ")"),
        (BoolOr, "(", " || ", ")"),
        (BoolXor, "(", " !== ", ")"),
        // JavaScript's `&` and `|` give a number.
        (BoolStrictAnd, "!!(", " & ", ")"),
        (BoolStrictOr, "!!(", " | ", ")"),
    ]
};

/// An `Int` division and remainder by a constant other than 0 and -1; `| 0` makes the negative
/// zero of `-5 % 5` the Int zero it is.
const INT_DIV_BY_CONSTANT: (&str, &str, &str) = ("(", " / ", " | 0)");
const INT_REM_BY_CONSTANT: (&str, &str, &str) = ("(", " % ", " | 0)");

pub(super) fn prim_text(op: PrimOp, by_constant: bool) -> (&'static str, &'static str, &'static str) {
    match op {
        PrimOp::IntDiv if by_constant => INT_DIV_BY_CONSTANT,
        PrimOp::IntRem if by_constant => INT_REM_BY_CONSTANT,
        _ => {
            let (at, pre, mid, post) = PRIM_TEXT[op as usize];
            debug_assert_eq!(at, op);
            (pre, mid, post)
        }
    }
}

/// Per unary operation, in the order of `UnOp`: the text before the operand and after it.
const UNARY_TEXT: [(UnOp, &str, &str); 27] = {
    use UnOp::*;
    [
        (IntNeg, "(-", " | 0)"),
        (LongNeg, "BigInt.asIntN(64, -", ")"),
        (DoubleNeg, "(-", ")"),
        (FloatNeg, "(-", ")"),
        (BoolNot, "(!", ")"),
        (IntNot, "(~", ")"),
        (LongNot, "(~", ")"),
        (IntToLong, "BigInt(", ")"),
        (LongToDouble, "Number(", ")"),
        (LongToInt, "Number(BigInt.asIntN(32, ", "))"),
        (DoubleToInt, "$d2i(", ")"),
        (DoubleToLong, "$d2l(", ")"),
        (CharToInt, "", ".charCodeAt(0)"),
        (CharToLong, "BigInt(", ".charCodeAt(0))"),
        (IntToChar, "String.fromCharCode(", " & 65535)"),
        (IntToByte, "(", " << 24 >> 24)"),
        (IntToShort, "(", " << 16 >> 16)"),
        (IntToFloat, "Math.fround(", ")"),
        (LongToFloat, "Math.fround(Number(", "))"),
        (FloatToInt, "$d2i(", ")"),
        (FloatToLong, "$d2l(", ")"),
        (FloatToDouble, "(", ")"),
        (DoubleToFloat, "Math.fround(", ")"),
        (IntToDouble, "", ""),
        (ByteToShort, "", ""),
        (ByteToInt, "", ""),
        (ShortToInt, "", ""),
    ]
};

pub(super) fn unary_text(op: UnOp) -> (&'static str, &'static str) {
    let (at, pre, post) = UNARY_TEXT[op as usize];
    debug_assert_eq!(at, op);
    (pre, post)
}

/// The tests written from a text, `$0` the tested value and `$1` a number of the test: the text,
/// and whether it takes parentheses as an operand.
pub(super) fn test_text(t: TypeTest) -> Option<(&'static str, bool)> {
    Some(match t {
        TypeTest::Always => ("true", false),
        TypeTest::Trait(_) => ("$isA($0, $1)", false),
        TypeTest::Number => ("typeof $0 === \"number\"", true),
        TypeTest::Int => ("$isInt($0)", false),
        TypeTest::Array => ("Array.isArray($0)", false),
        TypeTest::Long => ("typeof $0 === \"bigint\"", true),
        TypeTest::Byte => ("$isByte($0)", false),
        TypeTest::Short => ("$isShort($0)", false),
        TypeTest::Float => ("$isFloat($0)", false),
        TypeTest::Str | TypeTest::Char => ("typeof $0 === \"string\"", true),
        TypeTest::Bool => ("typeof $0 === \"boolean\"", true),
        TypeTest::Unit => ("$0 === undefined", true),
        // Past 22 parameters every function is one erased class, `FunctionXXL`.
        TypeTest::Function(n) if n > 22 => ("typeof $0 === \"function\" && $0.length > 22", true),
        TypeTest::Function(_) => ("typeof $0 === \"function\" && $0.length === $1", true),
        TypeTest::Null => ("$0 === null", true),
        TypeTest::AnyRef => ("$isRef($0)", false),
        TypeTest::AnyVal => ("($0 !== null)", false),
        TypeTest::Class(_) | TypeTest::Value(_) | TypeTest::Or(..) | TypeTest::And(..) => return None,
    })
}

/// The runtime's unboxing of a cast to the primitive the test is of (`$uI` for `Int`).
pub(super) fn unbox_helper(t: TypeTest) -> &'static str {
    match t {
        TypeTest::Long => "$uJ",
        TypeTest::Number => "$uD",
        TypeTest::Float => "$uF",
        TypeTest::Byte => "$uB",
        TypeTest::Short => "$uS",
        TypeTest::Str | TypeTest::Char => "$uC",
        TypeTest::Bool => "$uZ",
        _ => "$uI",
    }
}

/// The function a value is turned into a string with, before a concatenation.
/// The function a `ToStr` is written as: the program's `toString` of what may be `null` (a
/// string's, a reference's) is `$toStr`, which fails on `null` as a member selected from it does;
/// of a string by construction (a literal, a concatenation, a conversion), the string itself.
pub(super) fn to_str_conv_text(prog: &crate::tir::Program, inner: TExprId, conv: crate::tir::StrConv) -> &'static str {
    match conv.kind() {
        StrKind::Str if !conv.is_rendering() && prog.is_non_null_string(inner) => "",
        StrKind::Str | StrKind::Generic if !conv.is_rendering() => TO_STR_CALL,
        kind => to_str_text(kind),
    }
}

pub(super) const TO_STR_CALL: &str = "$toStr";

pub(super) fn to_str_text(kind: StrKind) -> &'static str {
    match kind {
        StrKind::Str => "",
        StrKind::Plain | StrKind::Long => "String",
        StrKind::Double => "$dstr",
        StrKind::Generic => "$str",
    }
}

/// The global a `Double` literal is written as, where it is one.
pub(super) fn double_global(v: f64) -> Option<&'static str> {
    if v.is_nan() {
        Some("NaN")
    } else if v.is_infinite() {
        Some("Infinity")
    } else {
        None
    }
}

pub(super) const THROW: &str = "$throw";
pub(super) const UNWRAP_JS: &str = "$unwrapJs";

/// Every text of the tables above, for the stems of what the output can render.
pub(super) fn op_texts() -> impl Iterator<Item = &'static str> {
    let prim = PRIM_TEXT.iter().flat_map(|&(_, pre, _, post)| [pre, post]);
    let unary = UNARY_TEXT.iter().flat_map(|&(_, pre, post)| [pre, post]);
    let tests = [
        TypeTest::Always, TypeTest::Trait(ClassId(0)), TypeTest::Number, TypeTest::Int, TypeTest::Array, TypeTest::Long, TypeTest::Byte,
        TypeTest::Short, TypeTest::Float, TypeTest::Str, TypeTest::Bool, TypeTest::Unit, TypeTest::Function(0), TypeTest::Null,
        TypeTest::AnyRef, TypeTest::AnyVal,
    ]
    .into_iter()
    .filter_map(|t| test_text(t).map(|(text, _)| text));
    let strs = [StrKind::Plain, StrKind::Double, StrKind::Generic].into_iter().map(to_str_text);
    let casts = ["$as", "$asA", "$asS", "$asT", "$asNothing", "$uI", "$uJ", "$uD", "$uF", "$uB", "$uS", "$uC", "$uZ"];
    prim.chain(unary).chain(tests).chain(strs).chain([TO_STR_CALL, INT_DIV_BY_CONSTANT.0, INT_REM_BY_CONSTANT.0, "NaN", "Infinity", THROW, UNWRAP_JS]).chain(casts)
}

#[cfg(test)]
mod tests {
    use super::{template_shape, Shape};

    #[test]
    fn tables_follow_the_operations() {
        for (i, &(op, ..)) in super::PRIM_TEXT.iter().enumerate() {
            assert_eq!(op as usize, i, "{:?}", op);
        }
        for (i, &(op, ..)) in super::UNARY_TEXT.iter().enumerate() {
            assert_eq!(op as usize, i, "{:?}", op);
        }
    }

    #[test]
    fn shapes_of_templates() {
        let atomic = ["$0.length", "$charAt($0, $1)", "new Array($0).fill($1)", "/[\\p{L}]/u.test($1)", "$0?.x", "($0 + $1)", "\"a\"", "$0[$1]"];
        for t in atomic {
            assert_eq!(template_shape(t), Shape::Atomic, "{}", t);
        }
        for t in ["-128", "-Infinity", "void $0.splice($1, 1)", "typeof $0", "!$0.isEmpty()"] {
            assert_eq!(template_shape(t), Shape::Unary, "{}", t);
        }
        for t in ["$parseInt($0) << 16 >> 16", "$0 === $1", "$0 ?? $1", "$0 >= 1", "typeof $0 === \"string\""] {
            assert_eq!(template_shape(t), Shape::Compound { conditional: false }, "{}", t);
        }
        for t in ["$0 ? 'a' : 'bb'", "$0.length = $1", "$0 += 1", "(x) => x", "$0, $1", "$1 === null || $1.indexOf(\"%\") < 0 ? $1 : $1.replace(/x/g, \"\")"] {
            assert_eq!(template_shape(t), Shape::Compound { conditional: true }, "{}", t);
        }
    }
}
