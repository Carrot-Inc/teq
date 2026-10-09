//! The fast paths of the hot call and construction protocol: frames and argument vectors
//! taken from pools instead of the allocator, the member a call site resolved to remembered
//! per site and class, the facts of a pattern computed once, and the construction of a class
//! that only stores its parameters done without running a constructor.

use super::value::*;
use super::*;
use crate::ast::ListRef;
use crate::symbols::ClassKind;
use crate::tir::*;

const FRAME_POOL: usize = 256;
const VEC_POOL: usize = 64;
const NO_SITE: u32 = u32::MAX;

impl<'a, 't> Interp<'a, 't> {
    // ---- pools ----

    /// A closure is about to keep `env` and `this` (`registry.rs`).
    #[inline]
    pub(super) fn captured(&self, env: &Rc<Frame>, this: &Value) {
        if self.resident {
            super::registry::captured(env, this);
        }
    }

    /// A frame with the parent, from the pool when one is free. A pooled frame that a closure
    /// once captured is registered, and its new parent is younger than it: the parent joins
    /// the registry with its ancestors, as it would have when the frame was captured
    /// (`registry.rs`).
    #[inline]
    pub(super) fn take_frame(&mut self, parent: Option<Rc<Frame>>, cap: usize) -> Rc<Frame> {
        if let Some(mut fr) = self.frame_pool.pop() {
            if let Some(f) = Rc::get_mut(&mut fr) {
                if f.slot.get() != 0 {
                    if let Some(p) = parent.as_ref().filter(|p| p.slot.get() == 0) {
                        super::registry::frame(p);
                    }
                }
                f.parent = parent;
                f.vals.get_mut().reserve(cap);
                f.born.set(0);
                f.made.set(super::made_now());
                return fr;
            }
        }
        Frame::with(parent, Vec::with_capacity(cap))
    }

    /// Returns a frame to the pool when nothing else refers to it: its values are dropped now,
    /// as they would be with the frame, and the parent is let go.
    #[inline]
    pub(super) fn release_frame(&mut self, mut fr: Rc<Frame>) {
        if self.frame_pool.len() < FRAME_POOL {
            if let Some(f) = Rc::get_mut(&mut fr) {
                f.parent = None;
                f.vals.get_mut().clear();
                self.frame_pool.push(fr);
            }
        }
    }

    #[inline]
    pub(super) fn take_vec(&mut self, cap: usize) -> Vec<Value> {
        match self.vec_pool.pop() {
            Some(mut v) => {
                v.reserve(cap);
                v
            }
            None => Vec::with_capacity(cap.max(4)),
        }
    }

    #[inline]
    pub(super) fn recycle_vec(&mut self, mut v: Vec<Value>) {
        if self.vec_pool.len() < VEC_POOL && v.capacity() <= 64 {
            v.clear();
            self.vec_pool.push(v);
        }
    }

    /// The default `applyOrElse` gets from a `{ case ... }` literal, shared: it reads nothing
    /// of the scope it is made in.
    pub(super) fn miss_closure(&mut self) -> Value {
        if let Some(v) = &self.miss_closure {
            return v.clone();
        }
        let v = Value::Fun(Rc::new(Closure { kind: ClosureKind::MatchError, env: Frame::new(None), this: Value::Unit, def_key: 0, class: None, hash: Cell::new(0) }));
        self.miss_closure = Some(v.clone());
        v
    }

    /// The default `applyOrElse` gets from `collect` and `collectFirst`, which tell a miss from
    /// a result: `partialMiss`, shared.
    pub(super) fn partial_miss(&mut self) -> Value {
        if let Some(v) = &self.partial_miss {
            return v.clone();
        }
        let v = Value::Fun(Rc::new(Closure { kind: ClosureKind::Miss, env: Frame::new(None), this: Value::Unit, def_key: 0, class: None, hash: Cell::new(0) }));
        self.partial_miss = Some(v.clone());
        v
    }

    // ---- call sites ----

    /// A method call at expression `e` on an object: the member its class resolves `s` to,
    /// remembered per site for the class last seen there.
    pub(super) fn invoke_at(&mut self, e: TExprId, recv: Value, s: SymId, args: Vec<Value>) -> R {
        let Value::Obj(o) = &recv else { return self.invoke(recv, s, args) };
        let class = o.class;
        let i = e.idx();
        let slot = self.call_sites[i];
        if slot != NO_SITE {
            let (c, name, m) = &self.site_entries[slot as usize];
            if *c == class {
                let (name, m) = (*name, m.clone());
                return self.invoke_member(recv, m, name, args);
            }
        }
        let (m, name) = self.object_member(class, s);
        if slot != NO_SITE {
            self.site_entries[slot as usize] = (class, name, m.clone());
        } else {
            self.site_entries.push((class, name, m.clone()));
            self.call_sites[i] = (self.site_entries.len() - 1) as u32;
        }
        self.invoke_member(recv, m, name, args)
    }

    /// A call of a member of an object through its module, at expression `e`.
    pub(super) fn call_static_at(&mut self, e: TExprId, s: SymId, args: Vec<Value>) -> R {
        if let Owner::Class(c) = self.syms().sym(s).owner {
            let module = self.module(c)?;
            return self.invoke_at(e, module, s, args);
        }
        self.call_static(s, args)
    }

    /// Whether a macro's run reached function `f` for the first time, to record its file.
    #[inline]
    pub(super) fn note_macro_fun(&mut self, f: FunId) {
        let i = f.idx();
        if !self.macro_seen[i] {
            self.macro_seen[i] = true;
            let sym = self.prog().funs[i].sym;
            let file = self.syms().sym(sym).file;
            self.typer.macro_files.insert(file, ());
        }
    }

    /// Whether a macro's run read the typed body of class `c` for the first time (its
    /// initialisers, its constructor), to record its file.
    #[inline]
    pub(super) fn note_macro_class(&mut self, c: ClassId) {
        let i = c.idx();
        if !self.macro_classes[i] {
            *self.macro_classes.at(i) = true;
            let file = self.syms().class(c).file;
            self.typer.macro_files.insert(file, ());
        }
    }

    // ---- patterns ----

    /// Whether the pattern binds anything, computed once per pattern.
    #[inline]
    pub(super) fn pat_binds_cached(&mut self, pat: TPatId) -> bool {
        let i = pat.idx();
        match self.pat_facts[i] {
            1 => false,
            2 => true,
            _ => {
                let b = self.pat_binds(pat);
                self.pat_facts[i] = if b { 2 } else { 1 };
                b
            }
        }
    }

    /// The field `f` of the object matched by the sub-pattern at `key` (its index in the
    /// pattern lists), read from the slot the last object of the same class had it in.
    pub(super) fn pat_field(&mut self, key: usize, v: &Value, o: &Rc<Object>, f: SymId) -> R {
        let (class, slot) = self.pat_slots[key];
        if class == o.class.0 {
            if let Some(x) = o.fields.borrow().get(slot as usize) {
                if !matches!(x, Value::Absent) {
                    return Ok(x.clone());
                }
            }
        } else {
            let k = self.field_key(f);
            if let Some(slot) = self.slot_if_known(o.class, k) {
                self.pat_slots[key] = (o.class.0, slot);
            }
        }
        self.get_field(v.clone(), f)
    }

    /// The value a pattern compares with when it costs nothing to know: a literal, or a local
    /// that holds a primitive or a string (a local read forces no lazy val and an object's
    /// `equals` runs no code of the program then).
    fn plain_compared(&mut self, e: TExprId, fr: &Rc<Frame>) -> Option<Value> {
        match self.prog().expr(e) {
            TExpr::Int(x) => Some(Value::Int(x)),
            TExpr::Char(x) => Some(Value::Char(x)),
            TExpr::Bool(x) => Some(Value::Bool(x)),
            TExpr::Str(x) => Some(Value::Str(self.string_lit(x))),
            TExpr::Local(s) => match self.lookup(e, s, fr) {
                Some(v) if !matches!(v, Value::Obj(_) | Value::Fun(_) | Value::Absent) => Some(v),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether a pattern cannot match, told from its literal sub-patterns alone: a value or
    /// the fields the sub-patterns test, read from their slots, are compared without running
    /// any code, before the case destructures and binds. `false` says nothing.
    pub(super) fn quick_reject(&mut self, pat: TPatId, v: &Value, fr: &Rc<Frame>) -> bool {
        match self.prog().pats[pat.idx()] {
            TPat::Bind(_, Some(inner)) => self.quick_reject(inner, v, fr),
            TPat::Equals(e, strict) => {
                if matches!(v, Value::Obj(_) | Value::Absent) {
                    return false;
                }
                let Some(lit) = self.plain_compared(e, fr) else { return false };
                if strict { !lit.same(v) } else { !self.equal(&lit, v).unwrap_or(true) }
            }
            TPat::Class(c, _, fields, subs) => {
                let Value::Obj(o) = v else { return true };
                if !self.is_subclass(o.class, c) {
                    return true;
                }
                for i in 0..fields.len as usize {
                    let sub = self.prog().pat_lists[subs.start as usize + i];
                    let lit = match self.prog().pats[sub.idx()] {
                        TPat::Equals(e, _) => match self.plain_compared(e, fr) {
                            Some(lit) => lit,
                            None => continue,
                        },
                        TPat::Class(..) => {
                            let f = self.prog().sym_lists[fields.start as usize + i];
                            let key = subs.start as usize + i;
                            let Some(&(class, slot)) = self.pat_slots.get(key) else { continue };
                            if class != o.class.0 {
                                continue;
                            }
                            let field = o.fields.borrow().get(slot as usize).cloned();
                            let _ = f;
                            match field {
                                Some(field) if !matches!(field, Value::Absent) => {
                                    if self.quick_reject(sub, &field, fr) {
                                        return true;
                                    }
                                }
                                _ => {}
                            }
                            continue;
                        }
                        _ => continue,
                    };
                    let key = subs.start as usize + i;
                    let Some(&(class, slot)) = self.pat_slots.get(key) else { continue };
                    if class != o.class.0 {
                        continue;
                    }
                    let field = o.fields.borrow().get(slot as usize).cloned();
                    let Some(field) = field else { continue };
                    if matches!(field, Value::Obj(_) | Value::Absent) {
                        continue;
                    }
                    if !self.equal(&lit, &field).unwrap_or(true) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// A block of the shape the typer gives a case lambda over several parameters, `val t =
    /// new TupleN(c0, ..); t match { case (p0, ..) => .. }`, where every case is a tuple pattern
    /// of that arity and nothing else names the tuple: its class, the components and the
    /// cases, decided once per block.
    pub(super) fn tuple_match_shape(&mut self, e: TExprId, stmts: ListRef, res: TExprId) -> Option<(ClassId, ListRef, ListRef)> {
        let i = e.idx();
        match self.block_facts[i] {
            1 => None,
            2 => self.tuple_match_parts(stmts, res).map(|(_, _, c, args, cases)| (c, args, cases)),
            _ => {
                let shape = self.tuple_match_parts(stmts, res).filter(|&(t, scrut, c, args, cases)| self.tuple_match_fits(t, scrut, c, args, cases, res));
                self.block_facts[i] = if shape.is_some() { 2 } else { 1 };
                shape.map(|(_, _, c, args, cases)| (c, args, cases))
            }
        }
    }

    /// The parts of a block of the shape, read off the IR: the tuple's local, the scrutinee,
    /// the tuple class, the components and the cases.
    fn tuple_match_parts(&self, stmts: ListRef, res: TExprId) -> Option<(SymId, TExprId, ClassId, ListRef, ListRef)> {
        if stmts.len != 1 {
            return None;
        }
        let TStmt::Val(t, init) = self.prog().stmts[stmts.start as usize] else { return None };
        let TExpr::New(c, args) = self.prog().expr(init) else { return None };
        let TExpr::Match(scrut, cases) = self.prog().expr(res) else { return None };
        if !matches!(self.prog().expr(scrut), TExpr::Local(s) if s == t) {
            return None;
        }
        Some((t, scrut, c, args, cases))
    }

    /// Whether the parts allow matching the components on their own: a tuple class of the
    /// components' arity, every case a tuple pattern of that arity, and no other mention of
    /// the tuple in the cases.
    fn tuple_match_fits(&self, t: SymId, scrut: TExprId, c: ClassId, args: ListRef, cases: ListRef, res: TExprId) -> bool {
        let n = args.len as usize;
        if n < 2 || self.typer.b.tuples.get(n).copied().flatten() != Some(c) {
            return false;
        }
        for i in cases.range() {
            let pat = self.prog().cases[i].pat;
            match self.prog().pats[pat.idx()] {
                TPat::Class(k, _, fields, subs) if k == c && subs.len as usize == n && fields.len as usize == n => {}
                _ => return false,
            }
        }
        !self.prog().descendants(res).any(|x| x != scrut && matches!(self.prog().expr(x), TExpr::Local(s) if s == t))
    }

    // ---- construction ----

    /// The slots of the constructor parameters of a class whose construction only stores them:
    /// no body, no parent to construct, no captures, no defaults in play. `None` for any other.
    pub(super) fn ctor_plan(&mut self, c: ClassId) -> Option<Rc<[u32]>> {
        if let Some(plan) = &self.ctor_plans[c.idx()] {
            return plan.clone();
        }
        let plan = self.plain_ctor(c);
        self.ctor_plans[c.idx()] = Some(plan.clone());
        plan
    }

    fn plain_ctor(&mut self, c: ClassId) -> Option<Rc<[u32]>> {
        let info = self.syms().class(c);
        if info.kind != ClassKind::Class || info.js_binding.is_some() {
            return None;
        }
        if let Some(sup) = info.superclass {
            if !self.trivial_ctor(sup, 0) {
                return None;
            }
        }
        let ti = self.tclass_index(c)?;
        let params = {
            let tc = &self.prog().classes[ti];
            if tc.parent_via.is_some() || !tc.parent_prelude.is_empty() || tc.parent_args.map_or(false, |l| !l.is_empty()) || tc.captures != 0 || !tc.init.is_empty() {
                return None;
            }
            tc.ctor_params.clone()
        };
        let mut slots = Vec::with_capacity(params.len());
        for p in params {
            let key = self.field_key(p);
            slots.push(self.slot(c, key));
        }
        Some(Rc::from(slots))
    }

    /// Whether constructing `c` as a superclass does nothing: no parameters, no body, no
    /// parent arguments, and the same of its own superclass.
    fn trivial_ctor(&mut self, c: ClassId, depth: u32) -> bool {
        if depth > 32 {
            return false;
        }
        let info = self.syms().class(c);
        if info.kind != ClassKind::Class || info.js_binding.is_some() {
            return false;
        }
        let superclass = info.superclass;
        let Some(ti) = self.tclass_index(c) else { return false };
        let tc = &self.prog().classes[ti];
        if tc.parent_via.is_some() || !tc.parent_prelude.is_empty() || tc.parent_args.map_or(false, |l| !l.is_empty()) || tc.captures != 0 || !tc.init.is_empty() || !tc.ctor_params.is_empty() {
            return false;
        }
        match superclass {
            Some(sup) => self.trivial_ctor(sup, depth + 1),
            None => true,
        }
    }

    /// The arguments of a plain construction, an omitted one replaced by its default when that
    /// is a literal; `Err` hands them back for the full path.
    pub(super) fn plain_args(&mut self, c: ClassId, slots: &[u32], args: Vec<Value>) -> Result<Vec<Value>, Vec<Value>> {
        if args.len() != slots.len() {
            return Err(args);
        }
        if !args.iter().any(|a| matches!(a, Value::Absent)) {
            return Ok(args);
        }
        let Some(ti) = self.tclass_index(c) else { return Err(args) };
        let mut filled = args.clone();
        for (i, a) in filled.iter_mut().enumerate() {
            if !matches!(a, Value::Absent) {
                continue;
            }
            let Some(d) = self.prog().classes[ti].ctor_defaults.get(i).copied().flatten() else { return Err(args) };
            *a = match self.prog().expr(d) {
                TExpr::Int(v) => Value::Int(v),
                TExpr::Long(v) => Value::Long(v),
                TExpr::Double(v) => Value::Double(v),
                TExpr::Bool(v) => Value::Bool(v),
                TExpr::Char(v) => Value::Char(v),
                TExpr::Str(s) => Value::Str(self.string_lit(s)),
                TExpr::Unit => Value::Unit,
                TExpr::Null => Value::Null,
                _ => return Err(args),
            };
        }
        Ok(filled)
    }

    /// An instance of a class with a plain constructor: the arguments stored in their slots.
    pub(super) fn construct_plain(&mut self, c: ClassId, slots: &[u32], args: Vec<Value>) -> Value {
        let n = slots.iter().copied().max().map_or(0, |m| m as usize + 1);
        let mut fields = Vec::with_capacity(n);
        fields.resize(n, Value::Absent);
        let mut args = args;
        for (&slot, v) in slots.iter().zip(args.drain(..)) {
            fields[slot as usize] = v;
        }
        self.recycle_vec(args);
        Value::Obj(Rc::new(Object { class: c, fields: RefCell::new(fields), env: None, name: None, ordinal: std::cell::Cell::new(0), hash: std::cell::Cell::new(0), slot: std::cell::Cell::new(0), born: std::cell::Cell::new(0), made: std::cell::Cell::new(super::made_now()) }))
    }
}
