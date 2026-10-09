//! Expression evaluation: the walk over `TExpr`, statements, patterns, `try`, the tail-call
//! loop and the dispatch of `@js` templates to builtins.

use super::value::*;
use super::*;
use crate::ast::{mods, ListRef};
use crate::symbols::ClassKind;
use crate::tir::*;
use crate::types::{Type, EMPTY_LIST};

impl<'a, 't> Interp<'a, 't> {
    #[inline]
    fn step(&mut self) -> R<()> {
        if self.steps == 0 {
            return Err(Control::Fail(if self.halted { Failure::Withheld } else { Failure::Budget }));
        }
        self.steps -= 1;
        Ok(())
    }

    /// The numeric kind the static type of `e` asks for, where the typer left a widening
    /// implicit: `0` where the value is taken as it comes.
    #[inline]
    fn coerce_kind(&mut self, e: TExprId) -> u8 {
        match self.coerce.get(e.idx()) {
            Some(&k) if k != K_UNKNOWN => k,
            _ => self.compute_coerce(e),
        }
    }

    /// The kind of `e`, read from its type once per node. Out of line, as `Value::coerce` is: what
    /// `eval` inlines on its way out shapes how every result is stored.
    #[cold]
    #[inline(never)]
    fn compute_coerce(&mut self, e: TExprId) -> u8 {
        let b = &self.typer.b;
        let kind = match self.typer.prog.expr_types.get(e.0) {
            // A body the loader typed records the base's types: read in the worker's view.
            Some(ty) if ty != NO_TYPE => match self.typer.types.get(self.typer.types.import(ty)) {
                Type::Class(c, args) if args == EMPTY_LIST => {
                    if c == b.int {
                        K_INT
                    } else if c == b.long {
                        K_LONG
                    } else if c == b.double {
                        K_DOUBLE
                    } else if c == b.float {
                        K_FLOAT
                    } else if c == b.byte {
                        K_BYTE
                    } else if c == b.short {
                        K_SHORT
                    } else if c == b.char {
                        K_CHAR
                    } else {
                        0
                    }
                }
                Type::Lit(l) => match self.typer.types.lit_val(l) {
                    LitVal::Int(_) => K_INT,
                    LitVal::Long(_) => K_LONG,
                    LitVal::Char(_) => K_CHAR,
                    _ => 0,
                },
                _ => 0,
            },
            _ => 0,
        };
        self.coerce[e.idx()] = kind;
        kind
    }

    /// Widens a value to the kind its static type has; a narrowing that would lose the value is
    /// left alone, since the type is then the coarser description.
    #[inline]
    fn adapt(v: Value, kind: u8) -> Value {
        let from = v.kind();
        if from == 0 || from == kind {
            return v;
        }
        match (from, kind) {
            (K_LONG | K_DOUBLE | K_FLOAT, K_INT | K_BYTE | K_SHORT | K_CHAR) | (K_DOUBLE, K_LONG) | (K_FLOAT, K_LONG) => v,
            _ => v.coerce(kind),
        }
    }

    pub(super) fn eval(&mut self, e: TExprId, fr: &Rc<Frame>, cx: &Ctx) -> R {
        self.step()?;
        let v = self.eval_node(e, fr, cx)?;
        let k = self.coerce_kind(e);
        Ok(if k != 0 { Self::adapt(v, k) } else { v })
    }

    /// An integer literal the typer narrowed to the `Byte`, `Short` or `Char` expected keeps
    /// that width, so that boxing it gives the narrow class (`(1: Byte): Any` is a `java.lang.Byte`).
    fn int_literal(&self, e: TExprId, v: i32) -> Value {
        let b = &self.typer.b;
        match self.prog().type_of(e) {
            Some(t) if t == b.t_byte => Value::Byte(v as i8),
            Some(t) if t == b.t_short => Value::Short(v as i16),
            Some(t) if t == b.t_char => Value::Char(v as u16),
            _ => Value::Int(v),
        }
    }

    fn eval_node(&mut self, e: TExprId, fr: &Rc<Frame>, cx: &Ctx) -> R {
        match self.prog().expr(e) {
            TExpr::Int(v) => Ok(self.int_literal(e, v)),
            TExpr::Long(v) => Ok(Value::Long(v)),
            TExpr::Double(v) => Ok(if self.prog().type_of(e) == Some(self.typer.b.t_float) { Value::Float(v as f32) } else { Value::Double(v) }),
            TExpr::Bool(v) => Ok(Value::Bool(v)),
            TExpr::Char(c) => Ok(Value::Char(c)),
            TExpr::Str(s) => Ok(Value::Str(self.string_lit(s))),
            TExpr::Unit => Ok(Value::Unit),
            TExpr::Null => Ok(Value::Null),
            TExpr::Local(s) => self.local(e, s, fr, cx),
            TExpr::This | TExpr::Super(_) => Ok(cx.this.clone()),
            TExpr::Static(s) => self.static_value(s),
            TExpr::Module(c) => self.module(c),
            TExpr::ClassOf(c) => Ok(self.class_value_of(c)),
            TExpr::Field(r, s) => {
                let recv = self.eval(r, fr, cx)?;
                if let Value::Obj(o) = &recv {
                    let (class, slot) = self.field_slots[e.idx()];
                    if class == o.class.0 {
                        if let Some(v) = o.fields.borrow().get(slot as usize) {
                            if !matches!(v, Value::Absent) {
                                return Ok(v.clone());
                            }
                        }
                    } else {
                        let key = self.field_key(s);
                        if let Some(slot) = self.slot_if_known(o.class, key) {
                            self.field_slots[e.idx()] = (o.class.0, slot);
                        }
                    }
                }
                self.get_field(recv, s)
            }
            TExpr::CallStatic(s, args) => {
                let info = self.syms().sym(s);
                let (owner, file) = (info.owner, info.file);
                if owner == Owner::Local {
                    let f = self.local(e, s, fr, cx)?;
                    let args = self.eval_args(Some(s), args, fr, cx)?;
                    return self.apply_value(f, args);
                }
                self.init_file_of(s, owner, file)?;
                let args = self.eval_args(Some(s), args, fr, cx)?;
                self.call_static_at(e, s, args)
            }
            TExpr::CallMethod(r, s, args) => {
                if let TExpr::Super(target) = self.prog().expr(r) {
                    let args = self.eval_args(Some(s), args, fr, cx)?;
                    return self.call_super(target, s, args, cx);
                }
                let recv = self.eval(r, fr, cx)?;
                let args = self.eval_args(Some(s), args, fr, cx)?;
                self.invoke_at(e, recv, s, args)
            }
            TExpr::CallClosure(f, args) => {
                let fv = self.eval(f, fr, cx)?;
                let args = self.eval_args(None, args, fr, cx)?;
                self.apply_value(fv, args)
            }
            TExpr::New(c, args) => {
                self.init_file_of_class(c)?;
                // A case class's companion is initialised before the arguments run, as scalac's
                // `apply` is called on it.
                self.touch_companion(c)?;
                if self.capture_count(c) > 0 {
                    let (env, vals) = self.anon_args(c, args, fr, cx)?;
                    return self.construct_new(c, vals, &env);
                }
                let vals = self.eval_ctor_args(c, args, fr, cx)?;
                self.construct_new(c, vals, fr)
            }
            TExpr::NewVia(s, args) => {
                let args = self.eval_args(Some(s), args, fr, cx)?;
                self.construct_via_new(s, args)
            }
            TExpr::Lambda(params, body) => {
                self.prof_alloc(|a| a.closures += 1);
                self.captured(fr, &cx.this);
                Ok(Value::Fun(Rc::new(Closure {
                    kind: ClosureKind::Lambda(params, body),
                    env: fr.clone(),
                    this: cx.this.clone(),
                    def_key: cx.def_key,
                    class: cx.class,
                    hash: Cell::new(0),
                })))
            }
            TExpr::If(c, t, els) => {
                if self.eval_bool(c, fr, cx)? {
                    self.eval(t, fr, cx)
                } else {
                    match els {
                        Some(x) => self.eval(x, fr, cx),
                        None => Ok(Value::Unit),
                    }
                }
            }
            TExpr::While(c, body) => {
                while self.eval_bool(c, fr, cx)? {
                    self.eval(body, fr, cx)?;
                }
                Ok(Value::Unit)
            }
            TExpr::Block(stmts, res) => {
                if stmts.is_empty() {
                    return self.eval(res, fr, cx);
                }
                if let Some((c, args, cases)) = self.tuple_match_shape(e, stmts, res) {
                    return self.eval_tuple_match(c, args, cases, fr, cx, false);
                }
                let inner = self.block_frame(stmts, fr, cx)?;
                let r = self.eval(res, &inner, cx);
                self.leave_block(inner, fr);
                r
            }
            TExpr::Assign(target, value) => {
                if let TExpr::Static(s) = self.prog().expr(target) {
                    let info = self.syms().sym(s);
                    let (owner, file) = (info.owner, info.file);
                    self.init_file_of(s, owner, file)?;
                }
                let v = self.eval(value, fr, cx)?;
                self.assign(target, v, fr, cx)?;
                Ok(Value::Unit)
            }
            TExpr::Match(scrut, cases) => {
                let v = self.eval(scrut, fr, cx)?;
                self.eval_match(v, cases, fr, cx, false)
            }
            TExpr::Prim(op, a, b) => self.eval_prim(op, a, b, fr, cx),
            TExpr::Unary(op, a) => {
                let v = self.eval(a, fr, cx)?;
                self.eval_unary(op, v)
            }
            TExpr::StrConcat(items) => self.eval_concat(items, fr, cx),
            TExpr::ToStr(inner, conv) => self.eval_to_str(inner, conv.kind(), fr, cx),
            TExpr::Js(s, args) => {
                if !self.typer.withheld_notes.is_empty() {
                    self.note_withheld_body(e);
                }
                self.template(s, args, fr, cx)
            }
            TExpr::TypeTest(inner, test) => {
                let v = self.eval(inner, fr, cx)?;
                Ok(Value::Bool(self.type_test(&v, test, fr, cx)?))
            }
            TExpr::SeqLit(items) => {
                let vals = self.eval_list(items, fr, cx)?;
                self.array_seq_of(Value::array(vals))
            }
            TExpr::ArrayLit(items) => {
                let vals = self.eval_list(items, fr, cx)?;
                Ok(Value::array(vals))
            }
            TExpr::Index(r, i) => {
                let v = self.eval(r, fr, cx)?;
                match v {
                    Value::Array(a) => Ok(a.borrow().get(i as usize).cloned().unwrap_or(Value::Null)),
                    Value::Match(m) => Ok(self.match_group(&m, i as usize)),
                    other => {
                        let shown = self.to_str(&other)?;
                        self.unsupported(format!("an index into {}", shown))
                    }
                }
            }
            TExpr::Return(v) => {
                let value = self.eval(v, fr, cx)?;
                Err(Control::Return(cx.def_key, value))
            }
            TExpr::Throw(inner, _) => {
                let v = self.eval(inner, fr, cx)?;
                match v {
                    Value::Null => self.throw_named("NullPointerException", "Cannot throw exception because the value is null"),
                    v => {
                        if self.trace {
                            let shown = match &v {
                                Value::Obj(o) => self.runtime_class_name(o.class).to_string(),
                                other => self.type_name(other).to_string(),
                            };
                            eprintln!("  throw {}", shown);
                        }
                        Err(Control::Throw(v))
                    }
                }
            }
            TExpr::Try(i) => self.eval_try(i, fr, cx, false),
            TExpr::Splice(_) => self.unsupported("a splice of an inline body typed at its definition runs at an expansion"),
            TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::JsSelect(..) | TExpr::ObjLit(_) | TExpr::Spread(_) => {
                self.unsupported("JavaScript interop is not executable by the interpreter")
            }
        }
    }

    fn eval_bool(&mut self, e: TExprId, fr: &Rc<Frame>, cx: &Ctx) -> R<bool> {
        match self.eval(e, fr, cx)? {
            Value::Bool(b) => Ok(b),
            other => {
                let shown = self.to_str(&other)?;
                self.unsupported(format!("a condition that is no Boolean: {}", shown))
            }
        }
    }

    #[inline(never)]
    fn eval_to_str(&mut self, inner: TExprId, kind: StrKind, fr: &Rc<Frame>, cx: &Ctx) -> R {
        let v = self.eval(inner, fr, cx)?;
        self.prof_alloc(|a| a.strings += 1);
        if let (StrKind::Str, Value::Str(_)) = (kind, &v) {
            return Ok(v);
        }
        let mut s = String::new();
        self.render(kind, &v, &mut s, &[])?;
        Ok(Value::string(s))
    }

    /// Out of `eval_node`, with the chain's loop inlined: a call per chain is 0.2% of the
    /// instructions of the macro benchmark (`macro-cls`).
    #[inline(never)]
    fn eval_concat(&mut self, items: ListRef, fr: &Rc<Frame>, cx: &Ctx) -> R {
        self.prof_alloc(|a| a.strings += 1);
        let mut out = String::new();
        let mut late = Vec::new();
        self.concat(items, fr, cx, &mut out, &mut late)?;
        if !late.is_empty() {
            out = self.render_late(&out, late)?;
        }
        Ok(Value::string(out))
    }

    /// A chain of `+` as scalac's JVM backend runs it: every operand evaluated, then every
    /// operand rendered. The text goes to `out` as the operands come; an object waits in `late`
    /// with the place of its text, since only its rendering can run the program's code or read
    /// what a later operand changes. Whether the chain goes on in its head is the IR's to say
    /// (`Program::chain_head`).
    #[inline(always)]
    fn concat(&mut self, items: ListRef, fr: &Rc<Frame>, cx: &Ctx, out: &mut String, late: &mut Vec<(usize, Value)>) -> R<()> {
        for i in 0..items.len as usize {
            let item = self.prog().expr_lists[items.start as usize + i];
            match self.prog().expr(item) {
                TExpr::ToStr(inner, conv) if conv.is_rendering() => {
                    self.step()?;
                    let v = self.eval(inner, fr, cx)?;
                    if v.renders_by_value() {
                        self.render(conv.kind(), &v, out, late)?;
                    } else {
                        late.push((out.len(), v));
                    }
                }
                TExpr::StrConcat(_) | TExpr::Block(..) | TExpr::If(..) if i == 0 && self.concat_head(item, fr, cx, out, late)? => {}
                _ => match self.eval(item, fr, cx)? {
                    Value::Str(s) => push_text(out, late, &s),
                    v => self.render(StrKind::Generic, &v, out, late)?,
                },
            }
        }
        Ok(())
    }

    /// Whether `head` was a chain, which has been run as the start of the chain it heads.
    #[inline(never)]
    fn concat_head(&mut self, head: TExprId, fr: &Rc<Frame>, cx: &Ctx, out: &mut String, late: &mut Vec<(usize, Value)>) -> R<bool> {
        match self.prog().chain_head(head) {
            ChainHead::Operand => Ok(false),
            ChainHead::Chain(items) => {
                self.step()?;
                self.concat(items, fr, cx, out, late)?;
                Ok(true)
            }
            ChainHead::Block(stmts, res) => {
                self.step()?;
                let inner = self.exec_stmts(stmts, fr, cx)?;
                let r = self.concat_head(res, &inner, cx, out, late);
                self.leave_block(inner, fr);
                r
            }
        }
    }

    fn render(&mut self, kind: StrKind, v: &Value, out: &mut String, late: &[(usize, Value)]) -> R<()> {
        match (kind, v) {
            (_, Value::Str(s)) => push_text(out, late, s),
            (_, Value::Char(c)) => push_text(out, late, crate::text::unit_char(*c).encode_utf8(&mut [0; 4])),
            (StrKind::Double, Value::Float(f)) => out.push_str(&java_float(*f)),
            (StrKind::Double, v) if v.is_number() => out.push_str(&java_double(v.as_f64().unwrap())),
            (StrKind::Long, v) if v.is_number() => out.push_str(&v.as_i64().unwrap().to_string()),
            _ => push_text(out, late, &self.to_str(v)?),
        }
        Ok(())
    }

    /// `text` with the operands in `late` rendered at their places, a pair cut by such a place
    /// joined again.
    fn render_late(&mut self, text: &str, late: Vec<(usize, Value)>) -> R<String> {
        let mut out = String::with_capacity(text.len());
        let mut done = 0;
        for (at, v) in late {
            crate::text::push_str(&mut out, &text[done..at]);
            done = at;
            let s = self.to_str(&v)?;
            crate::text::push_str(&mut out, &s);
        }
        crate::text::push_str(&mut out, &text[done..]);
        Ok(out)
    }

    pub(super) fn eval_list(&mut self, items: ListRef, fr: &Rc<Frame>, cx: &Ctx) -> R<Vec<Value>> {
        let mut out = self.take_vec(items.len as usize);
        for i in 0..items.len as usize {
            let item = self.prog().expr_lists[items.start as usize + i];
            out.push(self.eval(item, fr, cx)?);
        }
        Ok(out)
    }

    /// The arguments of a call of `sym`: an omitted one, which the typer writes as `()` where
    /// the parameter has a default, arrives as `Absent` for the callee to fill in.
    pub(super) fn eval_args(&mut self, sym: Option<SymId>, args: ListRef, fr: &Rc<Frame>, cx: &Ctx) -> R<Vec<Value>> {
        let mut out = self.take_vec(args.len as usize);
        for i in 0..args.len as usize {
            let item = self.prog().expr_lists[args.start as usize + i];
            if let (TExpr::Unit, Some(s)) = (self.prog().expr(item), sym) {
                if self.param_has_default(s, i) {
                    out.push(Value::Absent);
                    continue;
                }
            }
            out.push(self.eval(item, fr, cx)?);
        }
        Ok(out)
    }

    fn eval_ctor_args(&mut self, c: ClassId, args: ListRef, fr: &Rc<Frame>, cx: &Ctx) -> R<Vec<Value>> {
        let mut out = self.take_vec(args.len as usize);
        for i in 0..args.len as usize {
            let item = self.prog().expr_lists[args.start as usize + i];
            if let TExpr::Unit = self.prog().expr(item) {
                if self.ctor_param_has_default(c, i) {
                    out.push(Value::Absent);
                    continue;
                }
            }
            out.push(self.eval(item, fr, cx)?);
        }
        Ok(out)
    }

    /// Whether the `i`th argument a `new` passes may be omitted; an anonymous class passes its
    /// superclass's arguments on.
    fn ctor_param_has_default(&self, c: ClassId, i: usize) -> bool {
        let info = self.syms().class(c);
        let c = match (info.kind, info.superclass) {
            (ClassKind::Anon, Some(sup)) => sup,
            _ => c,
        };
        self.syms().class(c).ctor.iter().flat_map(|cl| cl.params.iter()).nth(i).map_or(false, |p| p.has_default)
    }

    /// The arguments that create an anonymous class: its captures, then what goes on to its
    /// superclass. A captured local stays in the scope the class is created in, which its
    /// members read from (a lazy val is not forced by the capture); a capture that is no local,
    /// the enclosing `this`, is bound in a scope of its own.
    fn anon_args(&mut self, c: ClassId, args: ListRef, fr: &Rc<Frame>, cx: &Ctx) -> R<(Rc<Frame>, Vec<Value>)> {
        let n = self.capture_count(c);
        let captures = self.tclass_field(c, |tc| tc.ctor_params[..n].to_vec()).unwrap_or_default();
        let mut vals = Vec::with_capacity(args.len as usize);
        let mut bound = Vec::new();
        for i in 0..args.len as usize {
            let item = self.prog().expr_lists[args.start as usize + i];
            if i < captures.len() {
                let lazy = self.syms().sym(captures[i]).mods & mods::LAZY != 0;
                match self.prog().expr(item) {
                    // The captured local itself is read from the environment; another local
                    // (`o` of `new o.Inner`) is the value passed.
                    TExpr::Local(s) if s == captures[i] => vals.push(Value::Absent),
                    // A thunk for a captured lazy val (the `this` of a local object passed for
                    // its own val) is read as the lazy val is.
                    TExpr::Lambda(params, body) if lazy && params.len == 0 => {
                        self.captured(fr, &cx.this);
                        let v = Value::Fun(Rc::new(Closure {
                            kind: ClosureKind::Lazy(body),
                            env: fr.clone(),
                            this: cx.this.clone(),
                            def_key: cx.def_key,
                            class: cx.class,
                            hash: Cell::new(0),
                        }));
                        bound.push((captures[i], v.clone()));
                        vals.push(v);
                    }
                    _ => {
                        let v = self.eval(item, fr, cx)?;
                        bound.push((captures[i], v.clone()));
                        vals.push(v);
                    }
                }
            } else if matches!(self.prog().expr(item), TExpr::Unit) && self.ctor_param_has_default(c, i - captures.len()) {
                vals.push(Value::Absent);
            } else {
                vals.push(self.eval(item, fr, cx)?);
            }
        }
        let env = if bound.is_empty() { fr.clone() } else { Frame::with(Some(fr.clone()), bound) };
        Ok((env, vals))
    }

    fn param_has_default(&self, s: SymId, i: usize) -> bool {
        match &self.syms().sym(s).sig {
            Some(sig) => sig.clauses.iter().flat_map(|c| c.params.iter()).nth(i).map_or(false, |p| p.has_default),
            None => false,
        }
    }

    // ---- locals ----

    fn local(&mut self, e: TExprId, s: SymId, fr: &Rc<Frame>, cx: &Ctx) -> R {
        let v = match self.lookup(e, s, fr) {
            Some(v) => v,
            None => {
                // A constructor parameter read in the body of its class, or a capture a
                // named class was constructed with.
                if let Value::Obj(_) = &cx.this {
                    let key = match self.field_key_of_param(s) {
                        Some(key) => Some(key),
                        None if self.syms().sym(s).owner == Owner::Local => Some(self.field_key(s)),
                        None => None,
                    };
                    if let Some(key) = key {
                        let v = self.read_slot(&cx.this, key);
                        if !matches!(v, Value::Absent) {
                            return Ok(v);
                        }
                    }
                }
                let name = self.name(self.syms().sym(s).name).to_string();
                return self.unsupported(format!("the local {} is not in scope", name));
            }
        };
        if let Value::Fun(c) = &v {
            if let ClosureKind::Lazy(init) = &c.kind {
                let init = *init;
                return self.init_lazy_local(e, s, init, c.clone(), fr);
            }
        }
        Ok(v)
    }

    /// A lazy local's first read: its initialiser runs and the local takes the value. The
    /// initialiser is the rest of its frame's making, in the frame's epoch (`super::watch`).
    #[cold]
    #[inline(never)]
    fn init_lazy_local(&mut self, e: TExprId, s: SymId, init: TExprId, c: Rc<Closure>, fr: &Rc<Frame>) -> R {
        let cx2 = Ctx { this: c.this.clone(), def_key: c.def_key, class: c.class, tail: None };
        let env = c.env.clone();
        let born = env.born.get();
        let outer = super::resume_epoch(born);
        let context = super::enter_init(|| format!("the lazy local {}", self.name(self.syms().sym(s).name)));
        let value = self.eval(init, &env, &cx2);
        drop(context);
        let value = value.and_then(|value| super::publish(|| self.set_local(e, s, value.clone(), fr)).map(|_| value));
        if let Ok(v) = &value {
            super::stamp(v, born);
        }
        super::leave_epoch(outer);
        value
    }

    /// The value of the local `s`, found where the last evaluation of the same node found it, or
    /// by a scan of the frames.
    pub(super) fn lookup(&mut self, e: TExprId, s: SymId, fr: &Rc<Frame>) -> Option<Value> {
        let hint = self.local_slots[e.idx()];
        if hint != 0 {
            let (depth, slot) = ((hint >> 16) as usize - 1, (hint & 0xffff) as usize);
            let mut f = fr;
            let mut ok = true;
            for _ in 0..depth {
                match &f.parent {
                    Some(p) => f = p,
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                let vals = f.vals.borrow();
                if let Some((sym, v)) = vals.get(slot) {
                    if *sym == s {
                        return Some(v.clone());
                    }
                }
            }
        }
        let mut f = fr;
        let mut depth = 0usize;
        loop {
            {
                let vals = f.vals.borrow();
                for (i, (sym, v)) in vals.iter().enumerate().rev() {
                    if *sym == s {
                        if depth < 0x7fff && i < 0xffff {
                            self.local_slots[e.idx()] = (((depth + 1) as u32) << 16) | i as u32;
                        }
                        return Some(v.clone());
                    }
                }
            }
            match &f.parent {
                Some(p) => {
                    f = p;
                    depth += 1;
                }
                None => return None,
            }
        }
    }

    fn set_local(&mut self, e: TExprId, s: SymId, v: Value, fr: &Rc<Frame>) -> R<()> {
        let hint = self.local_slots[e.idx()];
        if hint != 0 {
            let (depth, slot) = ((hint >> 16) as usize - 1, (hint & 0xffff) as usize);
            let mut f = Some(fr);
            for _ in 0..depth {
                f = f.and_then(|f| f.parent.as_ref());
            }
            if let Some(f) = f {
                let mut vals = f.vals.borrow_mut();
                if let Some((sym, place)) = vals.get_mut(slot) {
                    if *sym == s {
                        super::written(f.born.get(), f.made.get(), || Some(super::Node::Frame(f.clone())), |other| format!("the local {} of a frame{}{}", self.name(self.syms().sym(s).name), if other { " another run opened" } else { "" }, super::made_by(f.born.get())));
                        *place = v;
                        return Ok(());
                    }
                }
            }
        }
        let mut f = fr;
        let mut depth = 0usize;
        loop {
            {
                let mut vals = f.vals.borrow_mut();
                if let Some(slot) = vals.iter().rposition(|(sym, _)| *sym == s) {
                    super::written(f.born.get(), f.made.get(), || Some(super::Node::Frame(f.clone())), |other| format!("the local {} of a frame{}{}", self.name(self.syms().sym(s).name), if other { " another run opened" } else { "" }, super::made_by(f.born.get())));
                    vals[slot].1 = v;
                    if depth < 0x7fff && slot < 0xffff {
                        self.local_slots[e.idx()] = (((depth + 1) as u32) << 16) | slot as u32;
                    }
                    return Ok(());
                }
            }
            match &f.parent {
                Some(p) => {
                    f = p;
                    depth += 1;
                }
                None => break,
            }
        }
        let name = self.name(self.syms().sym(s).name).to_string();
        self.unsupported(format!("the local {} is not in scope for an assignment", name))
    }

    // ---- blocks ----

    /// Runs the statements of a block in a frame of their own when they declare something.
    fn block_frame(&mut self, stmts: ListRef, fr: &Rc<Frame>, cx: &Ctx) -> R<Rc<Frame>> {
        let range = stmts.range();
        let declares = self.prog().stmts[range.clone()].iter().any(|s| !matches!(s, TStmt::Expr(_)));
        let inner = if declares { self.take_frame(Some(fr.clone()), 2) } else { fr.clone() };
        if declares {
            self.prof_alloc(|a| a.frames += 1);
            // Local functions first, so that forward references work as in Scala.
            for i in range.clone() {
                if let TStmt::Fun(f) = self.prog().stmts[i] {
                    let sym = self.prog().funs[f.idx()].sym;
                    self.captured(&inner, &cx.this);
                    inner.bind(
                        sym,
                        Value::Fun(Rc::new(Closure {
                            kind: ClosureKind::LocalDef(f),
                            env: inner.clone(),
                            this: cx.this.clone(),
                            def_key: cx.def_key,
                            class: cx.class,
                            hash: Cell::new(0),
                        })),
                    );
                }
            }
        }
        for i in range {
            match self.prog().stmts[i] {
                TStmt::Expr(x) => {
                    self.eval(x, &inner, cx)?;
                }
                TStmt::Fun(_) => {}
                TStmt::Val(sym, init) => {
                    let v = if self.syms().sym(sym).mods & mods::LAZY != 0 {
                        self.captured(&inner, &cx.this);
                        Value::Fun(Rc::new(Closure {
                            kind: ClosureKind::Lazy(init),
                            env: inner.clone(),
                            this: cx.this.clone(),
                            def_key: cx.def_key,
                            class: cx.class,
                            hash: Cell::new(0),
                        }))
                    } else {
                        self.eval(init, &inner, cx)?
                    };
                    inner.bind(sym, v);
                }
                TStmt::Pat(pat, init) => {
                    let v = self.eval(init, &inner, cx)?;
                    if !self.match_pat(pat, &v, &inner, cx)? {
                        return self.match_error(v);
                    }
                }
            }
        }
        Ok(inner)
    }

    /// A frame that holds local defs refers to itself through them; when nothing else kept it,
    /// the cycle is cut here.
    fn leave_block(&mut self, inner: Rc<Frame>, outer: &Rc<Frame>) {
        if Rc::ptr_eq(&inner, outer) {
            return;
        }
        if Rc::strong_count(&inner) == 1 {
            self.release_frame(inner);
            return;
        }
        let self_refs = inner
            .vals
            .borrow()
            .iter()
            .filter(|(_, v)| matches!(v, Value::Fun(c) if matches!(c.kind, ClosureKind::LocalDef(_) | ClosureKind::Lazy(_)) && Rc::ptr_eq(&c.env, &inner)))
            .count();
        if self_refs > 0 && Rc::strong_count(&inner) == self_refs + 1 {
            inner.vals.borrow_mut().clear();
        }
    }

    pub(super) fn exec_stmts(&mut self, stmts: ListRef, fr: &Rc<Frame>, cx: &Ctx) -> R<Rc<Frame>> {
        if stmts.is_empty() {
            return Ok(fr.clone());
        }
        self.block_frame(stmts, fr, cx)
    }

    // ---- assignment ----

    fn assign(&mut self, target: TExprId, v: Value, fr: &Rc<Frame>, cx: &Ctx) -> R<()> {
        match self.prog().expr(target) {
            TExpr::Local(s) => {
                if self.set_local(target, s, v.clone(), fr).is_ok() {
                    return Ok(());
                }
                if let Value::Obj(_) = &cx.this {
                    if let Some(key) = self.field_key_of_param(s) {
                        self.write_slot(&cx.this, key, v);
                        return Ok(());
                    }
                }
                let name = self.name(self.syms().sym(s).name).to_string();
                self.unsupported(format!("the local {} is not in scope for an assignment", name))
            }
            TExpr::Field(r, s) => {
                let recv = self.eval(r, fr, cx)?;
                self.set_field(recv, s, v)
            }
            TExpr::Static(s) => self.set_static(s, v),
            _ => self.unsupported("an assignment to something that is no variable"),
        }
    }

    // ---- calls ----

    /// A top-level def, or a member reached statically: an object's method through its module.
    /// A top-level def's file is initialised before it runs where the caller has not done so
    /// before the arguments (an export's forwarder).
    pub(super) fn call_static(&mut self, s: SymId, args: Vec<Value>) -> R {
        let info = self.syms().sym(s);
        let (owner, file) = (info.owner, info.file);
        match owner {
            Owner::Class(c) => {
                let module = self.module(c)?;
                self.invoke(module, s, args)
            }
            _ => {
                self.init_file_of(s, owner, file)?;
                // The JVM's std writes some of the builtins' methods in Scala over its own
                // runtime; a macro runs them as the builtins, as on the other targets.
                if self.typer.jvm {
                    let name = self.qualified_name(s);
                    if let Some(b) = self.builtins.get(&*name).cloned() {
                        return b(self, &args);
                    }
                }
                let Some(f) = self.fun_of(s) else {
                    let name = self.qualified_name(s);
                    return self.unsupported(format!("{} has no body to run", name));
                };
                self.call_fun(f, Value::Unit, args, None, None)
            }
        }
    }

    /// The function of a def, typed now when the standard library deferred it.
    pub(super) fn fun_of(&mut self, s: SymId) -> Option<FunId> {
        if let Some(&f) = self.typer.fun_of_sym.get(&s) {
            crate::typer::bundle::entered_at(&self.typer.prog.funs, f.0, crate::typer::bundle::Entry::Fun);
            return Some(f);
        }
        let body = self.typer.deferred_body(s);
        self.note_withheld();
        match body {
            Some(crate::typer::check::DeferredBody::Fun(f)) => Some(f),
            _ => self.typer.fun_of_sym.get(&s).copied(),
        }
    }

    fn tail_info(&mut self, f: FunId) -> Option<(SymId, u32)> {
        let fun = &self.prog().funs[f.idx()];
        let (sym, arity) = (fun.sym, fun.params.len() as u32);
        match self.tail_of[f.idx()] {
            1 => return None,
            2 => return Some((sym, arity)),
            _ => {}
        }
        let loops = match fun.body {
            Some(body) if self.syms().is_effectively_final(sym) => self.prog().tail_self_calls(sym, arity as usize, body).0 > 0,
            _ => false,
        };
        self.tail_of[f.idx()] = if loops { 2 } else { 1 };
        loops.then_some((sym, arity))
    }

    /// Calls the function `f` with `this` and the arguments; `parent` is the scope a local def or
    /// a member of an anonymous class reads its captures from, `class` the class of a method.
    pub(super) fn call_fun(&mut self, f: FunId, this: Value, args: Vec<Value>, parent: Option<Rc<Frame>>, class: Option<ClassId>) -> R {
        if !self.typer.poisoned_withheld.is_empty() && self.typer.poisoned_withheld.contains_key(&f) {
            // A member of an anonymous class whose body its product withholds, kept quiet until
            // called, as the reach reports it once it marks the member; every run that calls it
            // stops, a later macro expansion's too.
            let diags = self.typer.poisoned.remove(&f).unwrap_or_default();
            self.typer.diags.items.extend(diags);
            return self.withheld_reached();
        }
        self.halt_on_withheld()?;
        let Some(body) = self.prog().funs[f.idx()].body else {
            let sym = self.prog().funs[f.idx()].sym;
            let name = self.qualified_name(sym);
            return self.unsupported(format!("{} is abstract", name));
        };
        if self.macro_ctx.is_some() {
            self.note_macro_fun(f);
        }
        if let Some(native) = self.native_of(f) {
            let prof = self.prof_fun(f);
            let r = native(self, &this, &args);
            self.prof_exit(prof);
            match r {
                Ok(Some(v)) => {
                    self.recycle_vec(args);
                    return Ok(v);
                }
                Ok(None) => {}
                Err(e) => return Err(e),
            }
        }
        self.enter()?;
        if self.trace && self.depth < self.trace_depth {
            let sym = self.prog().funs[f.idx()].sym;
            let name = self.qualified_name(sym);
            eprintln!("{}call {}", "  ".repeat(self.depth as usize), name);
        }
        let key = self.fresh_key();
        let tail = self.tail_info(f);
        let mut cx = Ctx { this, def_key: key, class, tail };
        let mut args = args;
        let prof = self.prof_fun(f);
        let result = loop {
            let frame = match self.bind_params(f, args, parent.clone(), &cx) {
                Ok(fr) => fr,
                Err(e) => break Err(e),
            };
            let r = self.eval_tail(body, &frame, &cx);
            self.release_frame(frame);
            match r {
                Err(Control::Return(k, v)) if k == key => break Ok(v),
                Err(Control::Tail(next, recv)) => {
                    args = next;
                    if let Some(t) = recv {
                        cx.this = t;
                    }
                }
                other => break other,
            }
        };
        self.prof_exit(prof);
        self.depth -= 1;
        result
    }

    fn bind_params(&mut self, f: FunId, mut args: Vec<Value>, parent: Option<Rc<Frame>>, cx: &Ctx) -> R<Rc<Frame>> {
        let n = self.prog().funs[f.idx()].params.len();
        self.prof_alloc(|a| a.frames += 1);
        let frame = self.take_frame(parent, n);
        let mut given = args.drain(..);
        for i in 0..n {
            let p = self.prog().funs[f.idx()].params[i];
            let mut v = given.next().unwrap_or(Value::Absent);
            if let Value::Absent = v {
                v = match self.prog().funs[f.idx()].defaults[i] {
                    Some(d) => self.eval(d, &frame, cx)?,
                    None => Value::Unit,
                };
            }
            frame.bind(p, v);
        }
        drop(given);
        self.recycle_vec(args);
        Ok(frame)
    }

    /// After the typer ran for the run: a body withheld from its products that it met stops
    /// the run at its next step, the budget spent (`step`), so nothing the program does after
    /// it runs.
    #[inline]
    pub(super) fn note_withheld(&mut self) {
        if self.typer.withheld_met != self.withheld_seen {
            self.withheld_seen = self.typer.withheld_met;
            self.halted = true;
            self.steps = 0;
        }
    }

    /// The diagnostic of the withheld body `e` stands for, reported again where an attempt that
    /// typed the body gave it up: the run stops there (`withheld_reached`) and says why.
    #[cold]
    #[inline(never)]
    fn note_withheld_body(&mut self, e: TExprId) {
        let Some(d) = self.typer.withheld_notes.get(&e) else { return };
        if !self.typer.diags.items.iter().any(|x| x.file == d.file && x.span == d.span && x.msg == d.msg) {
            let d = d.clone();
            self.typer.diags.items.push(d);
        }
    }

    /// Stops the run where it reaches a withheld body (`typer::WITHHELD_TEMPLATE`).
    pub(super) fn withheld_reached(&mut self) -> R {
        self.halted = true;
        self.steps = 0;
        Err(Control::Fail(Failure::Withheld))
    }

    /// Stops the run there, where a call, a builtin or a construction would run code without a
    /// step first.
    #[inline]
    pub(super) fn halt_on_withheld(&mut self) -> R<()> {
        self.note_withheld();
        if self.halted {
            return Err(Control::Fail(Failure::Withheld));
        }
        Ok(())
    }

    pub(super) fn enter(&mut self) -> R<()> {
        if self.depth >= self.max_depth {
            // The exception's constructor needs a few levels of its own.
            self.max_depth += 64;
            let e = self.throw_named::<Value>("StackOverflowError", "");
            self.max_depth -= 64;
            return match e {
                Err(Control::Throw(v)) => Err(Control::Throw(v)),
                _ => Err(Control::Fail(Failure::Depth)),
            };
        }
        self.depth += 1;
        Ok(())
    }

    /// Calls a function value: a lambda, a local def, a partial function literal or an object
    /// with an `apply` method.
    pub(super) fn apply_value(&mut self, f: Value, args: Vec<Value>) -> R {
        match f {
            Value::Fun(c) => self.call_closure(&c, args),
            Value::Obj(_) => {
                let apply = self.member_by_name(&f, crate::names::APPLY);
                self.invoke_member(f, apply, crate::names::APPLY, args)
            }
            Value::Null => self.throw_named("NullPointerException", "Cannot invoke a function because the value is null"),
            other => {
                let shown = self.to_str(&other)?;
                self.unsupported(format!("a call of {}, which is no function", shown))
            }
        }
    }

    pub(super) fn call_closure(&mut self, c: &Rc<Closure>, args: Vec<Value>) -> R {
        match &c.kind {
            ClosureKind::Lambda(params, body) => {
                self.enter()?;
                let n = params.len as usize;
                self.prof_alloc(|a| a.frames += 1);
                let frame = self.take_frame(Some(c.env.clone()), n);
                let mut args = args;
                let mut given = args.drain(..);
                for i in 0..n {
                    let p = self.prog().sym_lists[params.start as usize + i];
                    let v = match given.next() {
                        Some(Value::Absent) | None => Value::Unit,
                        Some(v) => v,
                    };
                    frame.bind(p, v);
                }
                drop(given);
                self.recycle_vec(args);
                let cx = Ctx { this: c.this.clone(), def_key: c.def_key, class: c.class, tail: None };
                let prof = self.prof_lambda(*body);
                let r = self.eval(*body, &frame, &cx);
                self.prof_exit(prof);
                self.release_frame(frame);
                self.depth -= 1;
                r
            }
            ClosureKind::LocalDef(f) => self.call_fun(*f, c.this.clone(), args, Some(c.env.clone()), c.class),
            ClosureKind::Partial(apply_or_else, _) => {
                let mut args = args;
                let x = args.drain(..).next().unwrap_or(Value::Unit);
                self.recycle_vec(args);
                let miss = self.miss_closure();
                let mut pair = self.take_vec(2);
                pair.push(x);
                pair.push(miss);
                self.apply_value(apply_or_else.clone(), pair)
            }
            ClosureKind::Lazy(init) => {
                let cx = Ctx { this: c.this.clone(), def_key: c.def_key, class: c.class, tail: None };
                let env = c.env.clone();
                self.eval(*init, &env, &cx)
            }
            ClosureKind::Miss => Ok(Value::Absent),
            ClosureKind::MatchError => {
                let v = args.into_iter().next().unwrap_or(Value::Unit);
                self.match_error(v)
            }
            ClosureKind::Builtin(b) => {
                let b = b.clone();
                b(self, &args)
            }
        }
    }

    /// The member `super.s` inside trait `tr` names for an instance of `class`: bound in the
    /// class, or in the superclass that was the first to mix the trait in.
    fn mixin_super_target(&mut self, class: ClassId, tr: ClassId, s: SymId) -> Option<Option<SymId>> {
        let mut at = Some(class);
        while let Some(k) = at {
            if let Some(a) = self.super_accessor(k, tr, s) {
                return Some(a);
            }
            at = self.syms().class(k).superclass;
        }
        None
    }

    fn call_super(&mut self, target: SuperTarget, s: SymId, args: Vec<Value>, cx: &Ctx) -> R {
        let name = self.syms().dispatch_name(s);
        let this = cx.this.clone();
        let member = match target {
            SuperTarget::Chain => {
                let Some(c) = cx.class else { return self.unsupported("a super call outside a class") };
                match self.syms().class(c).superclass {
                    Some(sup) => self.member(sup, name),
                    None => self.any_member(name),
                }
            }
            SuperTarget::Class(c) => {
                let own = self.own_member(c, name);
                match own {
                    Member::Missing => self.member(c, name),
                    m => m,
                }
            }
            SuperTarget::Mixin(tr) => {
                let Value::Obj(o) = &this else { return self.unsupported("a mixin super call on a value") };
                let class = o.class;
                let mut found = self.mixin_super_target(class, tr, s);
                // A class whose record has no binding for the trait (another worker's class, or
                // one checked before the trait's body was typed, as scala-library's
                // `CachedReverse` is while a macro runs) is bound by the same rule from the class
                // records, which are not changed for it.
                if found.is_none() {
                    found = self.typer.mixin_super_target_of(class, tr, s);
                }
                match found {
                    Some(Some(t)) => {
                        let Owner::Class(owner) = self.syms().sym(t).owner else { return self.unsupported("a super accessor to a non-member") };
                        let n = self.syms().dispatch_name(t);
                        match self.own_member(owner, n) {
                            Member::Missing => self.member(owner, n),
                            m => m,
                        }
                    }
                    _ => self.any_member(name),
                }
            }
        };
        self.invoke_member(this, member, name, args)
    }

    fn any_member(&self, name: Name) -> Member {
        match name {
            crate::names::TO_STRING => Member::AnyToString,
            crate::names::EQUALS => Member::AnyEquals,
            crate::names::HASH_CODE => Member::AnyHash,
            _ => Member::Missing,
        }
    }

    // ---- tail positions ----

    /// Evaluates `e` where a self call of the def would be a jump of its loop.
    fn eval_tail(&mut self, e: TExprId, fr: &Rc<Frame>, cx: &Ctx) -> R {
        let Some((sym, arity)) = cx.tail else { return self.eval(e, fr, cx) };
        match self.prog().expr(e) {
            TExpr::Block(stmts, res) => {
                self.step()?;
                if stmts.is_empty() {
                    return self.eval_tail(res, fr, cx);
                }
                if let Some((c, args, cases)) = self.tuple_match_shape(e, stmts, res) {
                    return self.eval_tuple_match(c, args, cases, fr, cx, true);
                }
                let inner = self.block_frame(stmts, fr, cx)?;
                let r = self.eval_tail(res, &inner, cx);
                self.leave_block(inner, fr);
                r
            }
            TExpr::If(c, t, els) => {
                self.step()?;
                if self.eval_bool(c, fr, cx)? {
                    self.eval_tail(t, fr, cx)
                } else {
                    match els {
                        Some(x) => self.eval_tail(x, fr, cx),
                        None => Ok(Value::Unit),
                    }
                }
            }
            TExpr::Match(scrut, cases) => {
                self.step()?;
                let v = self.eval(scrut, fr, cx)?;
                self.eval_match(v, cases, fr, cx, true)
            }
            TExpr::Try(i) if self.prog().tries[i as usize].finalizer.is_none() => {
                self.step()?;
                self.eval_try(i, fr, cx, true)
            }
            TExpr::Prim(op @ (PrimOp::BoolAnd | PrimOp::BoolOr), a, b) => {
                self.step()?;
                let x = self.eval_bool(a, fr, cx)?;
                if (op == PrimOp::BoolAnd) == x {
                    self.eval_tail(b, fr, cx)
                } else {
                    Ok(Value::Bool(x))
                }
            }
            TExpr::Return(v) => {
                self.step()?;
                self.eval_tail(v, fr, cx)
            }
            TExpr::CallStatic(s, _) if s == sym => {
                if let Some((None, args)) = self.prog().self_call(sym, arity as usize, e) {
                    self.step()?;
                    let vals = self.eval_args(Some(s), args, fr, cx)?;
                    return Err(Control::Tail(vals, None));
                }
                self.eval(e, fr, cx)
            }
            TExpr::CallMethod(_, s, _) if s == sym => {
                if let Some((recv, args)) = self.prog().self_call(sym, arity as usize, e) {
                    self.step()?;
                    let recv = match recv {
                        Some(r) => Some(self.eval(r, fr, cx)?),
                        None => None,
                    };
                    let vals = self.eval_args(Some(s), args, fr, cx)?;
                    return Err(Control::Tail(vals, recv));
                }
                self.eval(e, fr, cx)
            }
            _ => self.eval(e, fr, cx),
        }
    }

    // ---- match ----

    fn eval_match(&mut self, v: Value, cases: ListRef, fr: &Rc<Frame>, cx: &Ctx, tail: bool) -> R {
        for i in cases.range() {
            let case = self.prog().cases[i];
            if self.quick_reject(case.pat, &v, fr) {
                continue;
            }
            let binds = self.pat_binds_cached(case.pat);
            if binds {
                self.prof_alloc(|a| a.frames += 1);
            }
            let inner = if binds { self.take_frame(Some(fr.clone()), 2) } else { fr.clone() };
            if !self.match_pat(case.pat, &v, &inner, cx)? {
                if binds {
                    self.release_frame(inner);
                }
                continue;
            }
            if let Some(g) = case.guard {
                if !self.eval_bool(g, &inner, cx)? {
                    if binds {
                        self.release_frame(inner);
                    }
                    continue;
                }
            }
            let r = if tail { self.eval_tail(case.body, &inner, cx) } else { self.eval(case.body, &inner, cx) };
            if binds {
                self.release_frame(inner);
            }
            return r;
        }
        self.match_error(v)
    }

    /// The match of a case lambda over the tuple of its parameters, matched component by
    /// component against the values themselves: the tuple is built only for the `MatchError`
    /// when no case takes them.
    fn eval_tuple_match(&mut self, c: ClassId, args: ListRef, cases: ListRef, fr: &Rc<Frame>, cx: &Ctx, tail: bool) -> R {
        let n = args.len as usize;
        let mut vals = self.take_vec(n);
        for i in 0..n {
            let a = self.prog().expr_lists[args.start as usize + i];
            vals.push(self.eval(a, fr, cx)?);
        }
        'cases: for i in cases.range() {
            let case = self.prog().cases[i];
            let TPat::Class(_, _, _, subs) = self.prog().pats[case.pat.idx()] else { break };
            // The literal components reject a case before anything is destructured.
            for pass in 0..2 {
                for k in 0..n {
                    let sub = self.prog().pat_lists[subs.start as usize + k];
                    let literal = matches!(self.prog().pats[sub.idx()], TPat::Equals(..));
                    if literal == (pass == 0) && self.quick_reject(sub, &vals[k], fr) {
                        continue 'cases;
                    }
                }
            }
            let binds = self.pat_binds_cached(case.pat);
            if binds {
                self.prof_alloc(|a| a.frames += 1);
            }
            let inner = if binds { self.take_frame(Some(fr.clone()), 2) } else { fr.clone() };
            for k in 0..n {
                let sub = self.prog().pat_lists[subs.start as usize + k];
                match self.prog().pats[sub.idx()] {
                    TPat::Wildcard => {}
                    // The last component's binder takes the value itself: no later sub-pattern
                    // or guard can fail and hand the components to the next case.
                    TPat::Bind(sym, None) if k == n - 1 && case.guard.is_none() => {
                        let v = std::mem::replace(&mut vals[k], Value::Unit);
                        inner.bind(sym, v);
                    }
                    _ => {
                        if !self.match_pat(sub, &vals[k], &inner, cx)? {
                            if binds {
                                self.release_frame(inner);
                            }
                            continue 'cases;
                        }
                    }
                }
            }
            if let Some(g) = case.guard {
                if !self.eval_bool(g, &inner, cx)? {
                    if binds {
                        self.release_frame(inner);
                    }
                    continue 'cases;
                }
            }
            self.recycle_vec(vals);
            let r = if tail { self.eval_tail(case.body, &inner, cx) } else { self.eval(case.body, &inner, cx) };
            if binds {
                self.release_frame(inner);
            }
            return r;
        }
        let v = self.construct_new(c, vals, fr)?;
        self.match_error(v)
    }

    pub(super) fn pat_binds(&self, pat: TPatId) -> bool {
        let prog = self.prog();
        match prog.pats[pat.idx()] {
            TPat::Wildcard | TPat::Equals(..) => false,
            TPat::Bind(..) | TPat::Unapply(..) => true,
            TPat::Test(_, _, inner) => self.pat_binds(inner),
            TPat::Class(_, _, _, subs) | TPat::Alt(subs) => prog.pat_lists[subs.range()].iter().any(|&s| self.pat_binds(s)),
            TPat::Seq(items, rest) => prog.pat_lists[items.range()].iter().chain(rest.iter()).any(|&s| self.pat_binds(s)),
        }
    }

    pub(super) fn match_pat(&mut self, pat: TPatId, v: &Value, fr: &Rc<Frame>, cx: &Ctx) -> R<bool> {
        match self.prog().pats[pat.idx()] {
            TPat::Wildcard => Ok(true),
            TPat::Bind(sym, inner) => {
                fr.bind(sym, v.clone());
                match inner {
                    Some(i) => self.match_pat(i, v, fr, cx),
                    None => Ok(true),
                }
            }
            TPat::Test(test, _, inner) => {
                if !self.type_test(v, test, fr, cx)? {
                    return Ok(false);
                }
                self.match_pat(inner, v, fr, cx)
            }
            TPat::Equals(e, strict) => {
                let other = match self.prog().expr(e) {
                    TExpr::Int(i) => Value::Int(i),
                    TExpr::Char(c) => Value::Char(c),
                    TExpr::Bool(b) => Value::Bool(b),
                    TExpr::Str(s) => Value::Str(self.string_lit(s)),
                    _ => self.eval(e, fr, cx)?,
                };
                if strict { Ok(other.same(v)) } else { self.equal(&other, v) }
            }
            TPat::Class(c, _, fields, subs) => {
                let Value::Obj(o) = v else { return Ok(false) };
                if !self.is_subclass(o.class, c) {
                    return Ok(false);
                }
                for i in 0..fields.len as usize {
                    let f = self.prog().sym_lists[fields.start as usize + i];
                    let sub = self.prog().pat_lists[subs.start as usize + i];
                    match self.prog().pats[sub.idx()] {
                        TPat::Wildcard => {}
                        TPat::Bind(sym, None) => {
                            let field = self.pat_field(subs.start as usize + i, v, o, f)?;
                            fr.bind(sym, field);
                        }
                        _ => {
                            let field = self.pat_field(subs.start as usize + i, v, o, f)?;
                            if !self.match_pat(sub, &field, fr, cx)? {
                                return Ok(false);
                            }
                        }
                    }
                }
                Ok(true)
            }
            TPat::Alt(alts) => {
                for i in alts.range() {
                    let a = self.prog().pat_lists[i];
                    if self.match_pat(a, v, fr, cx)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            TPat::Unapply(sym, call, inner) => {
                fr.bind(sym, v.clone());
                let result = self.eval(call, fr, cx)?;
                self.match_pat(inner, &result, fr, cx)
            }
            TPat::Seq(items, rest) => {
                let n = items.len as usize;
                let Some(parts) = self.seq_pattern(v, n, rest.is_some())? else { return Ok(false) };
                for i in 0..n {
                    let sub = self.prog().pat_lists[items.start as usize + i];
                    if !self.match_pat(sub, &parts[i], fr, cx)? {
                        return Ok(false);
                    }
                }
                if let Some(r) = rest {
                    if !self.match_pat(r, &parts[n], fr, cx)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }

    // ---- try ----

    fn eval_try(&mut self, i: u32, fr: &Rc<Frame>, cx: &Ctx, tail: bool) -> R {
        let t = &self.prog().tries[i as usize];
        let (body, cases, finalizer, wraps) = (t.body, t.cases, t.finalizer, t.wraps);
        let result = match self.eval(body, fr, cx) {
            Err(Control::Throw(raw)) if !cases.is_empty() => {
                let caught = if wraps { self.wrap_thrown(raw.clone())? } else { raw.clone() };
                let mut handled = None;
                for k in cases.range() {
                    let case = self.prog().cases[k];
                    let inner = Frame::new(Some(fr.clone()));
                    if !self.match_pat(case.pat, &caught, &inner, cx)? {
                        continue;
                    }
                    if let Some(g) = case.guard {
                        if !self.eval_bool(g, &inner, cx)? {
                            continue;
                        }
                    }
                    handled = Some(if tail { self.eval_tail(case.body, &inner, cx) } else { self.eval(case.body, &inner, cx) });
                    break;
                }
                handled.unwrap_or(Err(Control::Throw(raw)))
            }
            other => other,
        };
        match finalizer {
            Some(f) if !matches!(result, Err(Control::Fail(_))) => {
                self.eval(f, fr, cx)?;
                result
            }
            _ => result,
        }
    }

    /// What a `catch` sees of a thrown value that is no `Throwable`: it wrapped in a
    /// `JavaScriptException`, when the program has that class.
    fn wrap_thrown(&mut self, v: Value) -> R {
        if let Value::Obj(o) = &v {
            if let Some(t) = self.prog().throwable {
                if self.is_subclass(o.class, t) {
                    return Ok(v);
                }
            }
        }
        match self.prog().js_exception {
            Some(c) => self.construct_new(c, vec![v], &Frame::new(None)),
            None => Ok(v),
        }
    }

    // ---- primitives ----

    fn eval_prim(&mut self, op: PrimOp, a: TExprId, b: TExprId, fr: &Rc<Frame>, cx: &Ctx) -> R {
        use PrimOp::*;
        if matches!(op, BoolAnd | BoolOr) {
            let x = self.eval_bool(a, fr, cx)?;
            if (op == BoolAnd) == x {
                return Ok(Value::Bool(self.eval_bool(b, fr, cx)?));
            }
            return Ok(Value::Bool(x));
        }
        let x = self.eval(a, fr, cx)?;
        let y = self.eval(b, fr, cx)?;
        self.prim(op, x, y)
    }

    pub(super) fn prim(&mut self, op: PrimOp, x: Value, y: Value) -> R {
        use PrimOp::*;
        let int = |v: &Value| v.as_i32();
        let long = |v: &Value| v.as_i64();
        let dbl = |v: &Value| v.as_f64();
        let bad = |it: &mut Self, x: &Value, y: &Value| -> R {
            let (sx, sy) = (it.to_str(x)?, it.to_str(y)?);
            it.unsupported(format!("{:?} on {} and {}", op, sx, sy))
        };
        Ok(match op {
            IntAdd | IntSub | IntMul | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr => {
                let (Some(a), Some(b)) = (int(&x), int(&y)) else { return bad(self, &x, &y) };
                Value::Int(match op {
                    IntAdd => a.wrapping_add(b),
                    IntSub => a.wrapping_sub(b),
                    IntMul => a.wrapping_mul(b),
                    IntAnd => a & b,
                    IntOr => a | b,
                    IntXor => a ^ b,
                    IntShl => a.wrapping_shl(b as u32),
                    IntShr => a.wrapping_shr(b as u32),
                    _ => ((a as u32).wrapping_shr(b as u32)) as i32,
                })
            }
            IntDiv | IntRem => {
                let (Some(a), Some(b)) = (int(&x), int(&y)) else { return bad(self, &x, &y) };
                if b == 0 {
                    return self.throw_named("ArithmeticException", "/ by zero");
                }
                Value::Int(if op == IntDiv { a.wrapping_div(b) } else { a.wrapping_rem(b) })
            }
            LongAdd | LongSub | LongMul | LongAnd | LongOr | LongXor | LongShl | LongShr | LongUshr => {
                let (Some(a), Some(b)) = (long(&x), long(&y)) else { return bad(self, &x, &y) };
                Value::Long(match op {
                    LongAdd => a.wrapping_add(b),
                    LongSub => a.wrapping_sub(b),
                    LongMul => a.wrapping_mul(b),
                    LongAnd => a & b,
                    LongOr => a | b,
                    LongXor => a ^ b,
                    LongShl => a.wrapping_shl(b as u32),
                    LongShr => a.wrapping_shr(b as u32),
                    _ => ((a as u64).wrapping_shr(b as u32)) as i64,
                })
            }
            LongDiv | LongRem => {
                let (Some(a), Some(b)) = (long(&x), long(&y)) else { return bad(self, &x, &y) };
                if b == 0 {
                    return self.throw_named("ArithmeticException", "/ by zero");
                }
                Value::Long(if op == LongDiv { a.wrapping_div(b) } else { a.wrapping_rem(b) })
            }
            DoubleAdd | DoubleSub | DoubleMul | DoubleDiv | DoubleRem => {
                let (Some(a), Some(b)) = (dbl(&x), dbl(&y)) else { return bad(self, &x, &y) };
                Value::Double(match op {
                    DoubleAdd => a + b,
                    DoubleSub => a - b,
                    DoubleMul => a * b,
                    DoubleDiv => a / b,
                    _ => a % b,
                })
            }
            FloatAdd | FloatSub | FloatMul | FloatDiv | FloatRem => {
                let (Some(a), Some(b)) = (dbl(&x), dbl(&y)) else { return bad(self, &x, &y) };
                let (a, b) = (a as f32, b as f32);
                Value::Float(match op {
                    FloatAdd => a + b,
                    FloatSub => a - b,
                    FloatMul => a * b,
                    FloatDiv => a / b,
                    _ => a % b,
                })
            }
            Lt | Le | Gt | Ge => {
                let r = match (&x, &y) {
                    (Value::Str(a), Value::Str(b)) => {
                        let c = compare_strings(a, b);
                        match op {
                            Lt => c < 0,
                            Le => c <= 0,
                            Gt => c > 0,
                            _ => c >= 0,
                        }
                    }
                    _ if x.is_fractional() || y.is_fractional() => {
                        let (Some(a), Some(b)) = (dbl(&x), dbl(&y)) else { return bad(self, &x, &y) };
                        match op {
                            Lt => a < b,
                            Le => a <= b,
                            Gt => a > b,
                            _ => a >= b,
                        }
                    }
                    _ => {
                        let (Some(a), Some(b)) = (long(&x), long(&y)) else { return bad(self, &x, &y) };
                        match op {
                            Lt => a < b,
                            Le => a <= b,
                            Gt => a > b,
                            _ => a >= b,
                        }
                    }
                };
                Value::Bool(r)
            }
            RefEq => Value::Bool(x.same(&y)),
            RefNe => Value::Bool(!x.same(&y)),
            Eq => Value::Bool(self.equal(&x, &y)?),
            Ne => Value::Bool(!self.equal(&x, &y)?),
            BoolXor => Value::Bool(!x.same(&y)),
            BoolStrictAnd | BoolStrictOr => {
                let (Some(a), Some(b)) = (x.as_bool(), y.as_bool()) else { return bad(self, &x, &y) };
                Value::Bool(if op == BoolStrictAnd { a && b } else { a || b })
            }
            BoolAnd | BoolOr => unreachable!(),
        })
    }

    fn eval_unary(&mut self, op: UnOp, v: Value) -> R {
        use UnOp::*;
        let bad = |it: &mut Self, v: &Value| -> R {
            let s = it.to_str(v)?;
            it.unsupported(format!("{:?} on {}", op, s))
        };
        Ok(match op {
            IntNeg => match v.as_i32() {
                Some(i) => Value::Int(i.wrapping_neg()),
                None => return bad(self, &v),
            },
            LongNeg => match v.as_i64() {
                Some(l) => Value::Long(l.wrapping_neg()),
                None => return bad(self, &v),
            },
            DoubleNeg => match v.as_f64() {
                Some(d) => Value::Double(-d),
                None => return bad(self, &v),
            },
            FloatNeg => match v.as_f64() {
                Some(d) => Value::Float(-(d as f32)),
                None => return bad(self, &v),
            },
            BoolNot => match v {
                Value::Bool(b) => Value::Bool(!b),
                _ => return bad(self, &v),
            },
            IntNot => match v.as_i32() {
                Some(i) => Value::Int(!i),
                None => return bad(self, &v),
            },
            LongNot => match v.as_i64() {
                Some(l) => Value::Long(!l),
                None => return bad(self, &v),
            },
            IntToLong | CharToLong | FloatToLong | DoubleToLong => match v.as_i64() {
                Some(l) => Value::Long(l),
                None => return bad(self, &v),
            },
            LongToDouble | FloatToDouble => match v.as_f64() {
                Some(d) => Value::Double(d),
                None => return bad(self, &v),
            },
            LongToInt | DoubleToInt | CharToInt | FloatToInt => match v.as_i32() {
                Some(i) => Value::Int(i),
                None => return bad(self, &v),
            },
            IntToChar => match v.as_i32() {
                Some(i) => Value::Char(i as u16),
                None => return bad(self, &v),
            },
            IntToByte => match v.as_i32() {
                Some(i) => Value::Byte(i as i8),
                None => return bad(self, &v),
            },
            IntToShort => match v.as_i32() {
                Some(i) => Value::Short(i as i16),
                None => return bad(self, &v),
            },
            IntToFloat | LongToFloat | DoubleToFloat => match v.as_f64() {
                Some(d) => Value::Float(d as f32),
                None => return bad(self, &v),
            },
            IntToDouble => match v.as_f64() {
                Some(d) => Value::Double(d),
                None => return bad(self, &v),
            },
            ByteToShort => match v.as_i32() {
                Some(i) => Value::Short(i as i16),
                None => return bad(self, &v),
            },
            ByteToInt | ShortToInt => match v.as_i32() {
                Some(i) => Value::Int(i),
                None => return bad(self, &v),
            },
        })
    }

    // ---- type tests ----

    pub(super) fn type_test(&mut self, v: &Value, test: TestId, fr: &Rc<Frame>, cx: &Ctx) -> R<bool> {
        Ok(match self.prog().tests[test.idx()] {
            TypeTest::Always => true,
            TypeTest::Class(c) | TypeTest::Trait(c) => self.is_instance(v, c),
            TypeTest::Number => matches!(v, Value::Double(_)),
            TypeTest::Int => matches!(v, Value::Int(_)),
            TypeTest::Array => matches!(v, Value::Array(_)),
            TypeTest::Long => matches!(v, Value::Long(_)),
            TypeTest::Byte => matches!(v, Value::Byte(_)),
            TypeTest::Short => matches!(v, Value::Short(_)),
            TypeTest::Float => matches!(v, Value::Float(_)),
            TypeTest::Str => matches!(v, Value::Str(_)),
            TypeTest::Char => matches!(v, Value::Char(_)),
            TypeTest::Bool => matches!(v, Value::Bool(_)),
            TypeTest::Unit => matches!(v, Value::Unit),
            TypeTest::Function(n) => self.function_arity(v) == Some(n as usize),
            TypeTest::Null => matches!(v, Value::Null),
            TypeTest::AnyRef => v.is_ref() && !matches!(v, Value::Null),
            TypeTest::AnyVal => !matches!(v, Value::Null),
            TypeTest::Value(e) => {
                let lit = self.eval(e, fr, cx)?;
                lit.same(v)
            }
            TypeTest::Or(a, b) => self.type_test(v, a, fr, cx)? || self.type_test(v, b, fr, cx)?,
            TypeTest::And(a, b) => self.type_test(v, a, fr, cx)? && self.type_test(v, b, fr, cx)?,
        })
    }

    /// Whether the value is an instance of the class or trait `c`.
    pub(super) fn is_instance(&mut self, v: &Value, c: ClassId) -> bool {
        match v {
            Value::Obj(o) => self.is_subclass(o.class, c) || self.clause_is_instance(v, c),
            Value::Fun(f) => {
                let info = self.syms().class(c);
                let name = self.name(info.name);
                if self.prog().partial_function == Some(c) {
                    return matches!(f.kind, ClosureKind::Partial(..));
                }
                if let Some(rest) = name.strip_prefix("Function") {
                    if let Ok(n) = rest.parse::<usize>() {
                        return self.function_arity(v) == Some(n);
                    }
                }
                false
            }
            Value::Str(_) => {
                let info = self.syms().class(c);
                matches!(self.name(info.name), "String" | "CharSequence" | "Comparable" | "Serializable" | "Object")
            }
            Value::Int(_) | Value::Long(_) | Value::Double(_) | Value::Float(_) | Value::Short(_) | Value::Byte(_) => {
                let info = self.syms().class(c);
                matches!(self.name(info.name), "Number" | "Comparable" | "Serializable" | "Object")
            }
            Value::Bool(_) | Value::Char(_) => {
                let info = self.syms().class(c);
                matches!(self.name(info.name), "Comparable" | "Serializable" | "Object")
            }
            Value::Class(_) => self.name(self.syms().class(c).name) == "Class",
            Value::Tree(t) => {
                let t = *t;
                self.tree_is_instance(t, c)
            }
            Value::Type(t) => {
                let t = *t;
                self.type_repr_is_instance(t, c)
            }
            Value::Sym(_) => self.reflect_class_named(c, "Symbol"),
            Value::Pos(..) => self.reflect_class_named(c, "Position"),
            Value::Src(_) => self.reflect_class_named(c, "SourceFile"),
            _ => false,
        }
    }

    pub(super) fn is_subclass(&self, k: ClassId, c: ClassId) -> bool {
        k == c || self.syms().class(k).base_types.iter().any(|&(b, _)| b == c)
    }

    fn function_arity(&mut self, v: &Value) -> Option<usize> {
        match v {
            Value::Fun(c) => Some(match &c.kind {
                ClosureKind::Lambda(params, _) => params.len as usize,
                ClosureKind::LocalDef(f) => self.prog().funs[f.idx()].params.len(),
                ClosureKind::Partial(..) | ClosureKind::Miss | ClosureKind::MatchError => 1,
                ClosureKind::Lazy(_) => 0,
                ClosureKind::Builtin(_) => 1,
            }),
            Value::Obj(o) => {
                let info = self.syms().class(o.class);
                for &(b, _) in info.base_types.iter().skip(1) {
                    let bi = self.syms().class(b);
                    if matches!(bi.owner, Owner::Package(p) if self.name(self.syms().pkg(p).name) == "scala") {
                        if let Some(rest) = self.name(bi.name).strip_prefix("Function") {
                            if let Ok(n) = rest.parse::<usize>() {
                                return Some(n);
                            }
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    // ---- templates ----

    fn template(&mut self, s: StrRef, args: ListRef, fr: &Rc<Frame>, cx: &Ctx) -> R {
        let resolved = self.resolve_template(s);
        self.halt_on_withheld()?;
        match resolved {
            Template::Builtin(b) => {
                // A quote renames the locals of its body before its holes are evaluated, so
                // that a quoted reference to one of them inside a hole names the same run.
                let quote = self.prog().strings[s.idx()] == "$quote";
                if self.trace && self.depth < self.trace_depth {
                    let name = match self.prog().template_syms.get(&s).copied() {
                        Some(sym) => self.qualified_name(sym).to_string(),
                        None => self.prog().strings[s.idx()].to_string(),
                    };
                    eprintln!("{}tmpl {}", "  ".repeat(self.depth as usize), name);
                }
                if quote {
                    self.enter_quote(args)?;
                }
                let result = match self.eval_list(args, fr, cx) {
                    Ok(vals) => {
                        let prof = self.prof_template(s);
                        let r = b(self, &vals);
                        self.prof_exit(prof);
                        self.recycle_vec(vals);
                        r
                    }
                    Err(e) => Err(e),
                };
                if quote {
                    self.quote_renames.pop();
                }
                result
            }
            Template::Body(sym, member) => {
                let mut vals = self.eval_args(Some(sym), args, fr, cx)?;
                if member {
                    if vals.is_empty() {
                        return self.unsupported("a member template without a receiver");
                    }
                    let recv = vals.remove(0);
                    self.invoke(recv, sym, vals)
                } else {
                    self.call_static(sym, vals)
                }
            }
            Template::AbstractBuiltin(b, sym) => {
                let mut vals = self.eval_list(args, fr, cx)?;
                if matches!(vals.first(), Some(Value::Obj(_))) {
                    let recv = vals.remove(0);
                    self.invoke(recv, sym, vals)
                } else {
                    b(self, &vals)
                }
            }
            Template::Missing(name) => self.unsupported(format!("no builtin for {}", name)),
        }
    }

    /// A class member an object receiver answers through dispatch: one without a body, or any
    /// member of an abstract class, whose objects are its subclasses'.
    fn is_dispatched_class_member(&mut self, sym: SymId) -> bool {
        let info = self.syms().sym(sym);
        let in_class = matches!(info.owner, Owner::Class(c) if self.syms().class(c).kind != ClassKind::Trait) && !info.is_extension;
        let abstract_owner = matches!(info.owner, Owner::Class(c) if self.syms().class(c).mods & mods::ABSTRACT != 0);
        in_class && (abstract_owner || self.typer.declared_without_body(sym))
    }

    fn resolve_template(&mut self, s: StrRef) -> Template {
        if let Some(t) = &self.template_cache[s.idx()] {
            crate::typer::bundle::cached_at(&self.typer.prog.strings, s.0);
            return t.clone();
        }
        let t = match self.prog().template_syms.get(&s).copied() {
            Some(sym) => {
                crate::typer::bundle::entered_at(&self.typer.prog.strings, s.0, crate::typer::bundle::Entry::Template);
                let name = self.qualified_name(sym);
                let abstract_member = self.is_dispatched_class_member(sym);
                match self.builtins.get(&*name) {
                    Some(b) if abstract_member => Template::AbstractBuiltin(b.clone(), sym),
                    Some(b) => Template::Builtin(b.clone()),
                    None => {
                        let info = self.syms().sym(sym);
                        let (owner, member) = (info.owner, matches!(info.owner, Owner::Class(_)) && !info.is_extension);
                        // A body of the std is typed when first asked for.
                        let has_body = match owner {
                            Owner::Class(c) if self.syms().class(c).kind == ClassKind::Trait && member => true,
                            _ => self.fun_of(sym).is_some() || self.typer.deferred_body(sym).is_some(),
                        };
                        // An abstract member (`Buffer.limit()` under a `@jvm` template) runs
                        // the receiver's implementation.
                        if has_body || (member && abstract_member) {
                            Template::Body(sym, member)
                        } else {
                            Template::Missing(Rc::from(format!("{} (a @js template without a Scala body)", name)))
                        }
                    }
                }
            }
            None => {
                let text = self.prog().strings[s.idx()].clone();
                match self.builtins.get(&text) {
                    Some(b) => Template::Builtin(b.clone()),
                    None => Template::Missing(Rc::from(format!("the template {}", text))),
                }
            }
        };
        self.template_cache[s.idx()] = Some(t.clone());
        t
    }
}

/// `text::push_str` into a chain's text, except at the place of an operand still to render, which
/// comes between the two.
#[inline]
fn push_text(out: &mut String, late: &[(usize, Value)], part: &str) {
    if late.last().is_some_and(|&(at, _)| at == out.len()) {
        out.push_str(part);
    } else {
        crate::text::push_str(out, part);
    }
}
