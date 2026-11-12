//! Lazy vals on the JVM as dotty's `LazyVals` writes them (compiler/src/dotty/tools/dotc/transform/LazyVals.scala at
//! 3.8.4), over scala-library's runtime: a local's holder of the `LazyRef` family, read through an accessor and
//! computed by an initialiser under the holder's monitor (`transformLocalDef`, `transformStats`).

use super::classfile::*;
use super::gen::*;
use super::names::*;
use crate::ast::{mods, ListRef};
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::rc::Rc;

impl<'a> Gen<'a> {
    /// Whether `s` is a local lazy val (or a local object, a lazy val of its module): what
    /// `transformLazyVal` sends to `transformLocalDef`.
    pub fn is_lazy_local(&self, s: SymId) -> bool {
        let info = self.cx.input.syms.sym(s);
        info.owner == Owner::Local && info.mods & mods::LAZY != 0 && info.kind != SymKind::Def
    }

    /// The holder class of the local lazy val `s` and the type its `value` and `initialize` take:
    /// `Definitions.LazyHolder` (1551-1564) over the local's erased type, the eight primitives'
    /// classes, `LazyRef` for every other type (`Unit` among them).
    pub fn lazy_holder_class(&mut self, s: SymId) -> (Rc<str>, JType) {
        let t = self.sym_type(s);
        let class = match t {
            JType::I => "scala/runtime/LazyInt",
            JType::J => "scala/runtime/LazyLong",
            JType::Z => "scala/runtime/LazyBoolean",
            JType::F => "scala/runtime/LazyFloat",
            JType::D => "scala/runtime/LazyDouble",
            JType::B => "scala/runtime/LazyByte",
            JType::C => "scala/runtime/LazyChar",
            JType::S => "scala/runtime/LazyShort",
            _ => return (Rc::from(LAZY_REF), JType::object()),
        };
        (Rc::from(class), t)
    }

    pub fn lazy_holder_type(&mut self, s: SymId) -> JType {
        JType::L(self.lazy_holder_class(s).0)
    }

    /// `val x$lzyN = new LazyX()`, the local `s` standing for its holder.
    pub fn new_lazy_holder(&mut self, s: SymId) {
        let (class, _) = self.lazy_holder_class(s);
        self.new_object(&class);
        self.invoke(Invoke::Special, &class, false, "<init>", &[], &JType::V);
        let holder = JType::L(class);
        let slot = self.store_new(&holder);
        self.m.locals.insert(s, Local { slot, ty: holder, cell: false });
    }

    /// `transformStats` (LazyVals.scala 199-209): the holders of the lazy vals a statement list
    /// declares come first, so that a local def or a lazy val above a definition reads its holder.
    /// Every emitted copy of the list makes its own, as each of `emitFinalizer`'s copies of a
    /// `finally` does: a binding left by an earlier copy names that copy's slot.
    pub fn hoist_lazy_holders(&mut self, stmts: ListRef) {
        let prog = self.cx.input.prog;
        for s in &prog.stmts[stmts.range()] {
            if let TStmt::Val(sym, init) = *s {
                if self.is_lazy_local(sym) {
                    self.an.lazy_inits.insert(sym, init);
                    self.new_lazy_holder(sym);
                }
            }
        }
    }

    /// The class file holding the accessor and the initialiser of the local lazy val `s`, and whether it is an
    /// interface: the class whose code declares the val (`Cx::local_home`), wherever that code is written (a value
    /// class's methods in its companion, a constructor's defaults), or for a file's top-level code that file's module
    /// (`Cx::local_file`: the code's file, an inline body's expansion in it included). dotty's LambdaLift lifts both into that class, where a reader in another class calls them.
    fn lazy_owner(&mut self, s: SymId) -> (Rc<str>, bool) {
        let cx = self.cx;
        match (cx.local_home(s), cx.local_file(s)) {
            (Some(Some(h)), _) => (self.class_name(h), cx.is_interface(h)),
            (Some(None), Some(file)) => (Rc::from(cx.file_modules[file.0 as usize].as_str()), false),
            // A val whose declaring code the walk did not meet: the methods of the class reading it.
            _ => (self.this_name.clone(), self.is_interface),
        }
    }

    /// The accessor of the local lazy val `s` (`init` false), `def x(): T = if x$lzy.initialized() then
    /// x$lzy.value() else x$lzyINIT()` (LazyVals.scala 192-194), or its initialiser, `def x$lzyINITN(): T =
    /// x$lzy.synchronized { if x$lzy.initialized() then x$lzy.value() else x$lzy.initialize(rhs) }` (181-190), as
    /// its owner declares them (`lazy_owner`) and every reader calls them: static, taking the declaring class's
    /// instance where the initialiser reads `this`, the key of its method's returns where it returns, what it
    /// captures, then the holder; named `x$lzyN$1` and `x$lzyINITM$1` by the file's numbers (§8), dotty's
    /// LazyLocalInitName freshened once by LambdaLift.
    fn lazy_method(&mut self, s: SymId, init: bool) -> Rc<Lifted> {
        let cached = if init { self.lifted_lazy_inits.get(&s) } else { self.lifted.get(&s) };
        if let Some(l) = cached {
            return l.clone();
        }
        let cx = self.cx;
        let free = self.an.of_lazy(cx, s);
        let placed = cx.local_home(s).is_some();
        let home = match cx.local_home(s) {
            Some(Some(h)) if free.this => Some(h),
            None if free.this => self.this_class,
            _ => None,
        };
        let mut captures = Vec::new();
        for &f in &free.syms {
            let info = cx.input.syms.sym(f);
            if info.kind == SymKind::Def {
                continue;
            }
            // A local lazy val travels as its holder, the key of its initialiser's returns beside it.
            if self.is_lazy_local(f) {
                let ty = self.lazy_holder_type(f);
                captures.push(Capture { sym: f, cell: false, ty, key: false });
                if self.lazy_keyed(f) {
                    captures.push(Capture { sym: f, cell: false, ty: JType::object(), key: true });
                }
                continue;
            }
            let cell = self.held(f) == Held::Cell;
            let ty = if info.by_name { self.function_type(0) } else { self.sym_type(f) };
            captures.push(Capture { sym: f, cell, ty, key: false });
        }
        let mut all: Vec<JType> = Vec::new();
        if let Some(h) = home {
            all.push(self.class_type(h));
        }
        if free.nonlocal {
            all.push(JType::object());
        }
        for c in &captures {
            all.push(if c.cell { JType::L(Rc::from(OBJECT_REF)) } else { c.ty.clone() });
        }
        let holder = self.lazy_holder_type(s);
        all.push(holder.clone());
        let ret = self.sym_type(s);
        let desc = method_desc(&all, &ret);
        let (n, m) = self.lazy_local_numbers(s);
        let base = encode(cx.input.interner.get(cx.input.syms.sym(s).name));
        let name = if init { format!("{}$lzyINIT{}$1", base, m) } else { format!("{}$lzy{}$1", base, n) };
        let lifted = Rc::new(Lifted { name, desc, this: home.is_some(), key: free.nonlocal, captures, params: vec![holder], ret, home, module: None });
        if init {
            self.lifted_lazy_inits.insert(s, lifted.clone());
        } else {
            self.lifted.insert(s, lifted.clone());
        }
        // Unplaced (`lazy_owner`), the class reading it writes the method.
        if !placed {
            let body = match cx.lazy_local_init(s).or_else(|| self.an.lazy_inits.get(&s).copied()) {
                Some(e) if init => Some(PendingBody::LazyInit(s, e)),
                _ if init => None,
                _ => Some(PendingBody::LazyAccessor(s)),
            };
            if let Some(body) = body {
                self.pending.push(Pending { lifted: lifted.clone(), body });
            }
        }
        lifted
    }

    /// The accessors and initialisers of the local lazy vals of the code of `home` (or, with `None`, of the file
    /// `file`'s top level): written in that class's file, once, whatever class reads them (`Cx::owned_lazies`).
    pub fn owned_lazy_methods(&mut self, home: Option<ClassId>, file: crate::source::FileId) {
        let cx = self.cx;
        for &s in cx.owned_lazies(home, file) {
            let Some(init) = cx.lazy_local_init(s) else { continue };
            let accessor = self.lazy_method(s, false);
            self.pending.push(Pending { lifted: accessor, body: PendingBody::LazyAccessor(s) });
            let initialiser = self.lazy_method(s, true);
            self.pending.push(Pending { lifted: initialiser, body: PendingBody::LazyInit(s, init) });
        }
    }

    /// A read of the local lazy val `s`: its accessor over its holder.
    pub fn lazy_read(&mut self, s: SymId) -> JType {
        let lifted = self.lazy_method(s, false);
        self.call_lazy_lifted(s, &lifted);
        lifted.ret.clone()
    }

    /// A call of the accessor or the initialiser of `s` in its owner (`lazy_owner`): the declaring class's
    /// instance, the key that travels with the holder of `s` (`load_lazy_key`), the captures, then the holder.
    fn call_lazy_lifted(&mut self, s: SymId, lifted: &Lifted) {
        self.load_captures_keyed(lifted, Some(s));
        self.load_capture(s);
        let (owner, interface) = self.lazy_owner(s);
        self.invoke_desc(Invoke::Static, &owner, interface, &lifted.name, &lifted.desc, lifted.arity(), &lifted.ret);
    }

    /// `x$lzy.initialized()`, `x$lzy.value()` and `x$lzy.initialize(v)` of the holder in `slot`.
    fn holder_call(&mut self, class: &str, slot: u16, method: &str, value: &JType) {
        let holder = JType::L(Rc::from(class));
        match method {
            "initialized" => {
                self.load(slot, &holder);
                self.invoke(Invoke::Virtual, class, false, "initialized", &[], &JType::Z);
            }
            "value" => {
                self.load(slot, &holder);
                self.invoke(Invoke::Virtual, class, false, "value", &[], value);
            }
            _ => self.invoke(Invoke::Virtual, class, false, "initialize", &[value.clone()], value),
        }
    }

    /// The body of the accessor of `s`, its holder in `slot` (`PendingBody::LazyAccessor`).
    pub fn lazy_accessor_body(&mut self, s: SymId, slot: u16) {
        let (class, value) = self.lazy_holder_class(s);
        let holder = JType::L(class.clone());
        self.m.locals.insert(s, Local { slot, ty: holder, cell: false });
        let ret = self.m.ret.clone();
        let compute = self.code.new_label();
        self.holder_call(&class, slot, "initialized", &value);
        self.jump_if(op::IFEQ, 1, compute);
        self.holder_call(&class, slot, "value", &value);
        self.adapt(&value, &ret);
        self.return_value(&ret);
        self.code.bind(compute);
        let init = self.lazy_method(s, true);
        self.call_lazy_lifted(s, &init);
        self.return_value(&ret);
    }

    /// The body of the initialiser of `s`, its holder in `slot`, the right-hand side `init`
    /// (`PendingBody::LazyInit`): the holder's monitor held as `synchronized` holds it.
    pub fn lazy_init_body(&mut self, s: SymId, init: TExprId, slot: u16) {
        let (class, value) = self.lazy_holder_class(s);
        let holder = JType::L(class.clone());
        self.m.locals.insert(s, Local { slot, ty: holder.clone(), cell: false });
        let ret = self.m.ret.clone();
        let t = self.monitor_block(
            |g| g.load(slot, &holder),
            ret.clone(),
            |g, t| {
                let compute = g.code.new_label();
                let done = g.code.new_label();
                g.holder_call(&class, slot, "initialized", &value);
                g.jump_if(op::IFEQ, 1, compute);
                g.holder_call(&class, slot, "value", &value);
                g.adapt(&value, t);
                g.goto(done);
                g.code.bind(compute);
                g.load(slot, &holder);
                g.expr(init, t);
                g.adapt(t, &value);
                g.holder_call(&class, slot, "initialize", &value);
                g.adapt(&value, t);
                g.code.bind(done);
            },
        );
        self.adapt(&t, &ret);
        self.return_value(&ret);
    }
}

/// A thread-safe lazy member of the class being written (`LazyVals.transformMemberDefThreadSafeNew`): its getter's
/// name and type, its `@volatile` `Object` container, the container's static `VarHandle` and its initialiser.
#[derive(Clone)]
pub struct LazyHolder {
    pub getter: String,
    pub ty: JType,
    pub container: String,
    pub handle: String,
    pub init: String,
}

impl<'a> Gen<'a> {
    /// The holder of the lazy member `getter` of the class being written, made at its first asking
    /// with its names (§8) and its handle declared before the class's other fields (`addInFront`),
    /// so that the later one comes first (`varHandleInfo.defs = h :: defs`).
    pub fn lazy_holder(&mut self, getter: &str, ty: &JType) -> LazyHolder {
        if let Some(h) = self.lazy_holders.iter().find(|h| h.getter == getter) {
            return h.clone();
        }
        let (container, init) = self.lazy_member_names(getter);
        let handle = format!("{}$lzyHandle", container);
        let mut d = String::new();
        JType::L(Rc::from(VAR_HANDLE)).desc(&mut d);
        self.cw.field_first(ACC_PRIVATE | ACC_STATIC | ACC_FINAL, &handle, &d);
        let h = LazyHolder { getter: getter.to_string(), ty: ty.clone(), container, handle, init };
        self.lazy_holders.push(h.clone());
        h
    }

    /// A lazy member the class holds beside its fields' (a made companion, a Scala 2 trait's private
    /// lazy val or object): its container, its getter `getter` of the given access, its
    /// initialiser computing `init`.
    pub fn thread_safe_member(&mut self, getter: &str, ty: &JType, init: super::classes::HolderInit, access: u16) {
        let h = self.lazy_holder(getter, ty);
        self.cw.field(ACC_PRIVATE | ACC_VOLATILE, &h.container, "Ljava/lang/Object;");
        let desc = method_desc(&[], ty);
        self.begin_method(false, false, &[], ty.clone(), &[]);
        self.thread_safe_getter(&h);
        self.return_value(ty);
        self.end_method(access, getter, &desc);
        self.thread_safe_init(&h, &init);
    }

    /// The `VarHandle` lookups of the class's lazy members, first in its `<clinit>`
    /// (`MoveStatics.rebuild` of the handles `addInFront` put first): `MethodHandles.lookup()
    /// .findVarHandle(classOf[C], "x$lzyN", classOf[Object])`, in the handles' order.
    pub fn var_handle_lookups(&mut self) {
        let owner = self.this_name.clone();
        let lookup = JType::L(Rc::from("java/lang/invoke/MethodHandles$Lookup"));
        let class = JType::L(Rc::from("java/lang/Class"));
        let handle_type = JType::L(Rc::from(VAR_HANDLE));
        for h in self.lazy_holders.clone().iter().rev() {
            self.invoke(Invoke::Static, "java/lang/invoke/MethodHandles", false, "lookup", &[], &lookup);
            let class_index = self.cw.cp.class(&owner);
            self.ldc(class_index);
            self.code.push(VT::Object(self.cw.cp.class("java/lang/Class")));
            self.sconst(&h.container);
            let object_index = self.cw.cp.class(OBJECT);
            self.ldc(object_index);
            self.code.push(VT::Object(self.cw.cp.class("java/lang/Class")));
            self.invoke(Invoke::Virtual, "java/lang/invoke/MethodHandles$Lookup", false, "findVarHandle", &[class.clone(), JType::L(Rc::from(STRING)), class.clone()], &handle_type);
            self.putstatic(&owner, &h.handle, &handle_type);
        }
    }

    /// The access of a `<clinit>`: `private static` where the class has lazy members' handles,
    /// as `MoveStatics` makes the static constructor of the `@static` handles.
    pub fn clinit_access(&self) -> u16 {
        if self.lazy_holders.is_empty() {
            ACC_STATIC
        } else {
            ACC_PRIVATE | ACC_STATIC
        }
    }

    /// Whether the erased type `tp` is a class `LazyValControlState` derives from (2.5).
    fn control_state_super(&self, tp: &JType) -> bool {
        matches!(tp, JType::L(n) if self.cx.control_state_bases.iter().any(|b| **b == **n))
    }

    /// The getter's body over the holder (`mkThreadSafeDef` 374-389), its value left on the stack
    /// at the member's type: the container read, returned where it holds a value
    /// (`immediateValueCondition`), null where it holds `NullValue`, else the initialiser's.
    pub fn thread_safe_getter(&mut self, h: &LazyHolder) {
        let owner = self.this_name.clone();
        let this = JType::L(owner.clone());
        let object = JType::object();
        let tp = h.ty.clone();
        self.load(0, &this);
        self.getfield(&owner, &h.container, &object);
        let r = self.store_new(&object);
        let slow = self.code.new_label();
        let call = self.code.new_label();
        let done = self.code.new_label();
        if self.control_state_super(&tp) {
            self.load(r, &object);
            self.jump_if(op::IFNULL, 1, slow);
            self.load(r, &object);
            self.instance_of(LAZY_CONTROL);
            self.jump_if(op::IFNE, 1, slow);
        } else {
            self.load(r, &object);
            let tested: String = match &tp {
                JType::L(n) => n.to_string(),
                prim => prim.box_class().to_string(),
            };
            self.instance_of(&tested);
            self.jump_if(op::IFEQ, 1, slow);
        }
        self.load(r, &object);
        self.adapt(&object, &tp);
        self.goto(done);
        self.code.bind(slow);
        self.load(r, &object);
        let null_value = JType::L(Rc::from(NULL_VALUE));
        self.getstatic(NULL_VALUE, "MODULE$", &null_value);
        self.jump_if(op::IF_ACMPNE, 2, call);
        self.code.op(op::ACONST_NULL);
        self.code.push(VT::Null);
        self.adapt(&object, &tp);
        self.goto(done);
        self.code.bind(call);
        self.load(0, &this);
        self.invoke(Invoke::Special, &owner, false, &h.init, &[], &object);
        self.adapt(&object, &tp);
        self.code.bind(done);
    }

    /// `x$lzyINITN`, the initialiser's state machine over the container (`mkThreadSafeDef`
    /// 391-457): `null` to `Evaluating` by a compare-and-set, the right-hand side computed, the
    /// value (or `NullValue` for null) published in a `finally`, which hands it to the waiters'
    /// `Waiting` latch where one was installed and resets the state where the computation threw;
    /// `Evaluating` met is replaced by a `Waiting` and waited on, `Waiting` waited on, `NullValue`
    /// answered as null, a value answered.
    pub fn thread_safe_init(&mut self, h: &LazyHolder, rhs: &super::classes::HolderInit) {
        use super::classes::HolderInit;
        let owner = self.this_name.clone();
        let this = JType::L(owner.clone());
        let object = JType::object();
        let handle_type = JType::L(Rc::from(VAR_HANDLE));
        let evaluating = JType::L(Rc::from(LAZY_EVALUATING));
        let waiting: Rc<str> = Rc::from(LAZY_WAITING);
        let roots: Vec<TExprId> = match rhs {
            HolderInit::Expr(e) => vec![*e],
            _ => Vec::new(),
        };
        // Two lazy members of one name in one class would be a fault of the names (§8): never one dropped in silence.
        let init_desc = method_desc(&[], &object);
        if self.cw.has_method(&h.init, &init_desc) {
            self.unsupported(&format!("internal error: a second lazy initialiser {} in the class", h.init));
            return;
        }
        self.begin_method(false, false, &[], object.clone(), &roots);
        let lp = self.code.new_label();
        self.code.bind(lp);
        self.load(0, &this);
        self.getfield(&owner, &h.container, &object);
        let cur = self.store_new(&object);
        let not_null = self.code.new_label();
        self.load(cur, &object);
        self.jump_if(op::IFNONNULL, 1, not_null);
        // `if CAS(this, null, Evaluating)`, else round again.
        self.getstatic(&owner, &h.handle, &handle_type);
        self.load(0, &this);
        self.code.op(op::ACONST_NULL);
        self.code.push(VT::Null);
        self.getstatic(LAZY_EVALUATING, "MODULE$", &evaluating);
        self.var_handle_cas();
        self.jump_if(op::IFEQ, 1, lp);
        self.code.op(op::ACONST_NULL);
        self.code.push(VT::Null);
        let result = self.store_new(&object);
        self.code.op(op::ACONST_NULL);
        self.code.push(VT::Null);
        let nullable = self.store_new(&object);
        let (start, end, handler) = (self.code.new_label(), self.code.new_label(), self.code.new_label());
        let protected = self.code.locals_mark();
        self.code.bind(start);
        let from = self.code.pc();
        // `resultNullable = rhs`, boxed (412).
        match rhs {
            HolderInit::Expr(e) => self.expr(*e, &h.ty),
            HolderInit::TraitStatic(base) => {
                self.load(0, &this);
                self.invoke(Invoke::Static, base, true, &format!("{}$", h.getter), &[JType::L(base.clone())], &h.ty);
            }
            HolderInit::Module { class, outer } => {
                self.new_object(class);
                self.load(0, &this);
                self.invoke(Invoke::Special, class, false, "<init>", &[JType::L(outer.clone())], &JType::V);
            }
        }
        let ty = h.ty.clone();
        self.adapt(&ty, &object);
        let vt = self.vt(&object);
        self.code.retype_top(vt);
        self.store(nullable, &object);
        // `if resultNullable eq null then result = NullValue else result = resultNullable`.
        let present = self.code.new_label();
        let set = self.code.new_label();
        self.load(nullable, &object);
        self.jump_if(op::IFNONNULL, 1, present);
        let null_value = JType::L(Rc::from(NULL_VALUE));
        self.getstatic(NULL_VALUE, "MODULE$", &null_value);
        self.store(result, &object);
        self.goto(set);
        self.code.bind(present);
        self.load(nullable, &object);
        self.store(result, &object);
        self.code.bind(set);
        if self.code.pc() == from {
            self.code.op(op::NOP);
        }
        self.code.bind(end);
        self.thread_safe_publish(h, result);
        self.load(nullable, &object);
        self.return_value(&object);
        self.code.add_handler(Handler { start, end, handler, catch_type: 0 });
        self.code.locals_release(protected);
        let throwable = JType::L(Rc::from(THROWABLE));
        let tvt = self.vt(&throwable);
        self.code.bind_handler(handler, tvt);
        let thrown = self.store_new(&throwable);
        self.thread_safe_publish(h, result);
        self.load(thrown, &throwable);
        self.code.op(op::ATHROW);
        self.code.pop();
        self.code.end_path();
        // A control state: `Evaluating` gets a latch to wait on, `Waiting` is waited on,
        // `NullValue` answers null; a value is answered.
        self.code.bind(not_null);
        let answer = self.code.new_label();
        let not_evaluating = self.code.new_label();
        let answer_null = self.code.new_label();
        self.load(cur, &object);
        self.instance_of(LAZY_CONTROL);
        self.jump_if(op::IFEQ, 1, answer);
        self.load(cur, &object);
        self.getstatic(LAZY_EVALUATING, "MODULE$", &evaluating);
        self.jump_if(op::IF_ACMPNE, 2, not_evaluating);
        self.getstatic(&owner, &h.handle, &handle_type);
        self.load(0, &this);
        self.load(cur, &object);
        self.new_object(&waiting);
        self.invoke(Invoke::Special, &waiting, false, "<init>", &[], &JType::V);
        self.var_handle_cas();
        self.pop_value(&JType::Z);
        self.goto(lp);
        self.code.bind(not_evaluating);
        self.load(cur, &object);
        self.instance_of(&waiting);
        self.jump_if(op::IFEQ, 1, answer_null);
        self.load(cur, &object);
        self.checkcast(&waiting);
        self.invoke(Invoke::Virtual, &waiting, false, "await", &[], &JType::V);
        self.goto(lp);
        self.code.bind(answer_null);
        self.code.op(op::ACONST_NULL);
        self.code.push(VT::Null);
        self.return_value(&object);
        self.code.bind(answer);
        self.load(cur, &object);
        self.return_value(&object);
        self.end_method(ACC_PRIVATE, &h.init, "()Ljava/lang/Object;");
    }

    /// The initialiser's `finally` (424-436): `if !CAS(this, Evaluating, result) then { val lock =
    /// x$lzy.asInstanceOf[Waiting]; CAS(this, lock, result); lock.countDown() }`.
    fn thread_safe_publish(&mut self, h: &LazyHolder, result: u16) {
        let owner = self.this_name.clone();
        let this = JType::L(owner.clone());
        let object = JType::object();
        let handle_type = JType::L(Rc::from(VAR_HANDLE));
        let evaluating = JType::L(Rc::from(LAZY_EVALUATING));
        let waiting = JType::L(Rc::from(LAZY_WAITING));
        let done = self.code.new_label();
        self.getstatic(&owner, &h.handle, &handle_type);
        self.load(0, &this);
        self.getstatic(LAZY_EVALUATING, "MODULE$", &evaluating);
        self.load(result, &object);
        self.var_handle_cas();
        self.jump_if(op::IFNE, 1, done);
        self.load(0, &this);
        self.getfield(&owner, &h.container, &object);
        self.checkcast(LAZY_WAITING);
        let lock = self.store_new(&waiting);
        self.getstatic(&owner, &h.handle, &handle_type);
        self.load(0, &this);
        self.load(lock, &waiting);
        self.load(result, &object);
        self.var_handle_cas();
        self.pop_value(&JType::Z);
        self.load(lock, &waiting);
        self.invoke(Invoke::Virtual, LAZY_WAITING, false, "countDown", &[], &JType::V);
        self.code.bind(done);
    }
}

/// The numbers of the lazy names of a source file (`LazyLocalName`, `LazyLocalInitName`, counted per compilation unit
/// by `FreshNameCreator`, NameKinds.scala 220-222, util/FreshNameCreator.scala 24-28): per member its container's and
/// initialiser's, per local lazy val its initialiser's.
#[derive(Default)]
pub struct LazyNames {
    members: crate::intern::FxMap<(Option<ClassId>, String), (u32, u32)>,
    /// A local lazy val's holder's number and its initialiser's.
    locals: crate::intern::FxMap<SymId, (u32, u32)>,
}

/// The walk of a file in the order dotty's `LazyVals` meets its lazy definitions (`Gen::walk_lazy_names`).
#[derive(Default)]
struct NameWalk {
    counts: crate::intern::FxMap<(String, bool), u32>,
    out: LazyNames,
    walked: crate::intern::FxMap<ClassId, ()>,
    /// The file's local classes by the start of their spans.
    locals: Vec<(u32, ClassId)>,
    /// The named local classes an inline expansion in the file made, whose spans are their template's: each walked
    /// where the walk first meets its construction.
    expanded: crate::intern::FxMap<ClassId, ()>,
    /// The start of the definition being walked: its own local classes are the ones after it.
    scope: u32,
}

/// `StdNames.str.sanitize` (StdNames.scala 44-45): `<` and `>` replaced by `$`.
fn sanitize(raw: &str) -> String {
    raw.replace(['<', '>'], "$")
}

impl NameWalk {
    /// `FreshNameCreator.newName` (24-28): the counter of `sanitize(prefix) + separator`, the prefix the
    /// definition's name as written (`raw`), apart from its mangling in the emitted name.
    fn fresh(&mut self, raw: &str, init: bool) -> u32 {
        let n = self.counts.entry((sanitize(raw), init)).or_insert(0);
        *n += 1;
        *n
    }

    /// A thread-safe member: its container's number, then its initialiser's (`transformMemberDefThreadSafeNew`
    /// 476, `mkThreadSafeDef` 359), kept by its getter's emitted name.
    fn member(&mut self, owner: Option<ClassId>, getter: &str, raw: &str) {
        let n = self.fresh(raw, false);
        let m = self.fresh(raw, true);
        self.out.members.entry((owner, getter.to_string())).or_insert((n, m));
    }

    /// A made companion of a class: `transformSyntheticModule` names its eager holder alone (140); teq's
    /// thread-safe holder takes the container's number for its initialiser too.
    fn made(&mut self, owner: Option<ClassId>, getter: &str, raw: &str) {
        let n = self.fresh(raw, false);
        self.out.members.entry((owner, getter.to_string())).or_insert((n, n));
    }

    /// A local lazy val: its holder's number, then its initialiser's (`transformLocalDef` 168, 181).
    fn local(&mut self, s: SymId, raw: &str) {
        let n = self.fresh(raw, false);
        let m = self.fresh(raw, true);
        self.out.locals.insert(s, (n, m));
    }
}

impl<'a> Gen<'a> {
    /// The lazy names of the file `file`, walked once whichever class asks first.
    fn file_lazy_names(&mut self, file: crate::source::FileId) -> &'a LazyNames {
        let cx = self.cx;
        match cx.lazy_names.get(file.0 as usize) {
            Some(cell) => cell.get_or_init(|| self.walk_lazy_names(file)),
            None => &EMPTY_NAMES,
        }
    }

    /// The names of a lazy member's container and initialiser in the class being written, `x$lzyN` and
    /// `x$lzyINITM`, by the file's walk.
    pub(super) fn lazy_member_names(&mut self, getter: &str) -> (String, String) {
        let owner = self.this_class;
        let file = match owner {
            Some(c) => self.cx.input.syms.class(c).module_file(),
            None => self.line_file,
        };
        let Some((n, m)) = self.file_lazy_names(file).members.get(&(owner, getter.to_string())).copied() else {
            self.unsupported(&format!("internal error: the lazy member {} has no number in its unit", getter));
            return (format!("{}$lzy", getter), format!("{}$lzyINIT", getter));
        };
        (format!("{}$lzy{}", getter, n), format!("{}$lzyINIT{}", getter, m))
    }

    /// Whether the class `owner` (the file `file`'s module for `None`; for a class, `file` the unit holding it,
    /// `ClassInfo::module_file`) holds a lazy member, as the names walk numbers them (§8): its `<clinit>` looks up
    /// their handles first, and is written after them.
    pub(super) fn holds_lazy_members(&mut self, owner: Option<ClassId>, file: crate::source::FileId) -> bool {
        self.file_lazy_names(file).members.keys().any(|(o, _)| *o == owner)
    }

    /// The numbers of the local lazy val `s`'s holder and initialiser, `x$lzyN` and `x$lzyINITM`, by the walk of
    /// the file of the code declaring it (`Cx::local_file`): an inline method's expansion is numbered in the unit
    /// that holds it, as dotty freshens it there, each expansion its own.
    pub(super) fn lazy_local_numbers(&mut self, s: SymId) -> (u32, u32) {
        let file = self.cx.local_file(s).unwrap_or(self.cx.input.syms.sym(s).file);
        let Some(numbers) = self.file_lazy_names(file).locals.get(&s).copied() else {
            let name = self.cx.input.interner.get(self.cx.input.syms.sym(s).name);
            self.unsupported(&format!("internal error: the local lazy val {} has no number in its unit", name));
            return (0, 0);
        };
        numbers
    }

    /// The walk (§8): per package clause the classes, traits and objects that stand at the top level in their
    /// order, each class followed by its made companion, then the file's package object
    /// (`Desugar.packageDef` 1948-1968); in a template the parents' arguments, then the statements in order
    /// (MegaPhase.scala 371-377), a definition after the trees inside it, a method's default getters after it
    /// (`addDefaultGetters`), an object's module val before its class, a class's made companion after it
    /// (`Desugar.classDef` 1144); then the class's forwarders of its traits' lazy members, after the whole body
    /// (`Mixin.transformTemplate` 274-299).
    fn walk_lazy_names(&mut self, file: crate::source::FileId) -> LazyNames {
        let cx = self.cx;
        let syms = cx.input.syms;
        let mut w = NameWalk::default();
        let mut tops: Vec<(u32, ClassId)> = Vec::new();
        let opaque = cx.opaque_companions.get(&file.0).cloned().unwrap_or_default();
        for (i, info) in syms.classes.iter().enumerate() {
            let c = ClassId(i as u32);
            if info.module_file() != file || cx.tclass_of[i] == u32::MAX {
                continue;
            }
            let made = info.made_at.is_some() || info.placed_at.is_some();
            match info.owner {
                Owner::Package(_) if !opaque.contains(&c) && !made => tops.push((info.span.start, c)),
                // An expansion's copy is met at its first construction, an anonymous one at its `New`.
                Owner::Local if made => {
                    if info.kind != ClassKind::Anon {
                        w.expanded.insert(c, ());
                    }
                }
                Owner::Local => w.locals.push((info.span.start, c)),
                _ => {}
            }
        }
        tops.sort();
        w.locals.sort();
        for (_, c) in tops {
            self.walk_class_names(&mut w, c);
        }
        // The package object: the file's vals and defs in their order, its opaque types' companions.
        let mut items: Vec<(u32, u8, u32)> = Vec::new();
        let group = cx.file_group[file.0 as usize];
        if group != u32::MAX {
            for &(s, e) in cx.layout.file_groups[group as usize].1.iter() {
                items.push((syms.sym(s).span.start, 0, s.0));
                let _ = e;
            }
        }
        for &f in &cx.file_funs[file.0 as usize] {
            items.push((syms.sym(cx.input.prog.funs[f.idx()].sym).span.start, 1, f.0));
        }
        for &k in &opaque {
            items.push((syms.class(k).span.start, 2, k.0));
        }
        items.sort();
        let vals: crate::intern::FxMap<SymId, TExprId> = if group == u32::MAX { Default::default() } else { cx.layout.file_groups[group as usize].1.iter().copied().collect() };
        for (_, kind, id) in items {
            match kind {
                0 => {
                    let s = SymId(id);
                    if let Some(&e) = vals.get(&s) {
                        w.scope = syms.sym(s).span.start;
                        self.walk_expr_names(&mut w, e);
                    }
                    let info = syms.sym(s);
                    let lazy = info.mods & mods::LAZY != 0 || (info.kind == SymKind::Given && !self.is_given_def(s));
                    if lazy && !cx.given_objects.contains_key(&s) && !cx.given_classes.contains_key(&s) {
                        let name = self.field_name(s);
                        w.member(None, &name, cx.input.interner.get(info.name));
                    }
                }
                1 => self.walk_fun_names(&mut w, FunId(id)),
                _ => self.walk_class_names(&mut w, ClassId(id)),
            }
        }
        // The unit's classes the walk did not meet (a quote's, an expansion's that nothing constructs), after the rest
        // by their binary names: every definition the unit writes has its numbers, the same on every run.
        let mut rest: Vec<(Rc<str>, ClassId)> = Vec::new();
        for (i, info) in syms.classes.iter().enumerate() {
            let c = ClassId(i as u32);
            if info.module_file() == file && cx.tclass_of[i] != u32::MAX && !w.walked.contains_key(&c) && !opaque.contains(&c) {
                rest.push((self.class_name(c), c));
            }
        }
        rest.sort();
        for (_, c) in rest {
            self.walk_class_names(&mut w, c);
        }
        w.out
    }

    fn walk_class_names(&mut self, w: &mut NameWalk, c: ClassId) {
        if w.walked.insert(c, ()).is_some() {
            return;
        }
        let cx = self.cx;
        let syms = cx.input.syms;
        let prog = cx.input.prog;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX {
            return;
        }
        let tc = &prog.classes[idx as usize];
        let info = syms.class(c);
        let is_trait = info.kind == ClassKind::Trait;
        let holds = !is_trait;
        let outer_scope = std::mem::replace(&mut w.scope, info.span.start);
        // The superclass call: the temporaries of its named arguments in their written order, then the arguments
        // (dotty's `{ val b$1 = ..; val a$1 = ..; super(a$1, b$1) }`).
        self.walk_stmts_names(w, tc.parent_prelude);
        if let Some(args) = tc.parent_args {
            for &a in prog.expr_list(args) {
                self.walk_expr_names(w, a);
            }
        }
        // The body's members by their place: fields and statements, methods, member classes.
        let mut items: Vec<(u32, u8, usize)> = Vec::new();
        for (i, init) in tc.init.iter().enumerate() {
            match *init {
                TInit::Field(s, _) => items.push((syms.sym(s).span.start, 0, i)),
                TInit::Stmt(e) => items.push((cx.input.prog.expr_spans.get(e.0).map_or(0, |(_, sp)| sp.start), 0, i)),
                TInit::Parent(..) => items.push((0, 0, i)),
            }
        }
        for (i, &f) in tc.methods.iter().enumerate() {
            items.push((syms.sym(prog.funs[f.idx()].sym).span.start, 1, i));
        }
        // A secondary constructor is a `DefDef` of the body at its place: its self call's arguments, then its body.
        for (i, &f) in tc.ctors.iter().enumerate() {
            items.push((syms.sym(prog.funs[f.idx()].sym).span.start, 3, i));
        }
        let mut nested: Vec<ClassId> = info.nested.values().copied().collect();
        // An object nested in an object reached statically is no field and no entry of `nested` (a class's): its
        // module val and class stand at its place in the template all the same (MegaPhase meets its ValDef there).
        let fields: Vec<SymId> = tc.init.iter().filter_map(|i| if let TInit::Field(s, _) = *i { Some(s) } else { None }).collect();
        for s in cx.members_in_order(c) {
            if let SymKind::Object(k) = syms.sym(s).kind {
                if syms.sym(s).owner == Owner::Class(c) && !fields.contains(&s) && !nested.contains(&k) && cx.tclass_of.get(k.idx()).is_some_and(|&i| i != u32::MAX) {
                    nested.push(k);
                }
            }
        }
        nested.sort();
        for (i, &k) in nested.iter().enumerate() {
            items.push((syms.class(k).span.start, 2, i));
        }
        items.sort();
        let fields_lazy: crate::intern::FxMap<SymId, ()> = if holds { self.lazy_field_syms(tc) } else { Default::default() };
        for (_, kind, i) in items {
            match kind {
                0 => match tc.init[i] {
                    TInit::Field(s, e) => {
                        self.walk_expr_names(w, e);
                        if fields_lazy.contains_key(&s) {
                            let name = self.field_name(s);
                            w.member(Some(c), &name, cx.input.interner.get(syms.sym(s).name));
                        }
                        if let SymKind::Object(k) = syms.sym(s).kind {
                            self.walk_class_names(w, k);
                        }
                    }
                    TInit::Stmt(e) => self.walk_expr_names(w, e),
                    // A trait's parameters: the temporaries of its named arguments, then the arguments.
                    TInit::Parent(_, call) => {
                        self.walk_stmts_names(w, call.prelude);
                        for &a in prog.expr_list(call.args) {
                            self.walk_expr_names(w, a);
                        }
                    }
                },
                1 => self.walk_fun_names(w, tc.methods[i]),
                3 => self.walk_fun_names(w, tc.ctors[i]),
                _ => {
                    let k = nested[i];
                    self.walk_class_names(w, k);
                    if holds && (cx.has_inner_case_companion(k) || cx.has_instance_case_companion(k)) {
                        let raw = cx.input.interner.get(syms.class(k).name);
                        w.made(Some(c), &encode(raw), raw);
                    }
                    if let Some(m) = syms.class(k).companion.filter(|&m| syms.class(m).def.is_none()) {
                        self.walk_class_names(w, m);
                    }
                }
            }
        }
        for &d in tc.ctor_defaults.iter().flatten() {
            self.walk_expr_names(w, d);
        }
        // Its local classes the walk has not met yet, at their places.
        self.walk_local_classes_before(w, info.span.end);
        if holds {
            for (name, raw) in self.lazy_forwarder_names(tc) {
                w.member(Some(c), &name, &raw);
            }
        }
        w.scope = outer_scope;
    }

    fn walk_fun_names(&mut self, w: &mut NameWalk, f: FunId) {
        let prog = self.cx.input.prog;
        let fun = &prog.funs[f.idx()];
        let span = self.cx.input.syms.sym(fun.sym).span;
        let outer_scope = std::mem::replace(&mut w.scope, span.start);
        if let Some(b) = fun.body {
            self.walk_expr_names(w, b);
        }
        for &d in fun.defaults.iter().flatten() {
            self.walk_expr_names(w, d);
        }
        // Its local classes the walk has not met yet (one never constructed, after the last statement).
        let end = fun.body.and_then(|b| self.expr_span(b)).map_or(span.end, |sp| sp.end.max(span.end));
        self.walk_local_classes_before(w, end);
        w.scope = outer_scope;
    }

    fn expr_span(&self, e: TExprId) -> Option<crate::source::Span> {
        self.cx.input.prog.expr_spans.get(e.0).map(|(_, sp)| sp)
    }

    /// The local classes of the definition being walked declared before `at` and not walked yet, each walked at
    /// its declaration in their order, constructed or not (MegaPhase meets the local `TypeDef` there).
    fn walk_local_classes_before(&mut self, w: &mut NameWalk, at: u32) {
        let due: Vec<ClassId> = w.locals.iter().filter(|&&(p, k)| p >= w.scope && p < at && !w.walked.contains_key(&k)).map(|&(_, k)| k).collect();
        for k in due {
            self.walk_class_names(w, k);
        }
    }

    /// The statements `stmts` in their order, each after the local classes declared before it.
    fn walk_stmts_names(&mut self, w: &mut NameWalk, stmts: ListRef) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let syms = cx.input.syms;
        for s in prog.stmts[stmts.range()].to_vec() {
            match s {
                TStmt::Expr(x) => {
                    if let Some(sp) = self.expr_span(x) {
                        self.walk_local_classes_before(w, sp.start);
                    }
                    self.walk_expr_names(w, x);
                }
                TStmt::Val(sym, init) => {
                    self.walk_local_classes_before(w, syms.sym(sym).span.start);
                    self.walk_expr_names(w, init);
                    if self.is_lazy_local(sym) {
                        w.local(sym, cx.input.interner.get(syms.sym(sym).name));
                    }
                }
                TStmt::Fun(f) => {
                    self.walk_local_classes_before(w, syms.sym(prog.funs[f.idx()].sym).span.start);
                    self.walk_fun_names(w, f);
                }
                TStmt::Pat(p, init) => {
                    if let Some(sp) = self.expr_span(init) {
                        self.walk_local_classes_before(w, sp.start);
                    }
                    self.walk_expr_names(w, init);
                    self.walk_pat_names(w, p);
                }
            }
        }
    }

    /// The expressions a pattern holds (`pat_exprs`), in its order: a case's pattern is met before its guard and
    /// its body, a pattern definition's after its right-hand side.
    fn walk_pat_names(&mut self, w: &mut NameWalk, p: TPatId) {
        let mut held = Vec::new();
        super::pat_exprs(self.cx.input.prog, p, &mut held);
        for e in held {
            self.walk_expr_names(w, e);
        }
    }

    fn walk_expr_names(&mut self, w: &mut NameWalk, e: TExprId) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let syms = cx.input.syms;
        match prog.expr(e) {
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::TypeTest(r, _) | TExpr::Cast(r, ..) | TExpr::Index(r, _)
            | TExpr::Spread(r) | TExpr::JsSelect(r, _) | TExpr::Return(r) | TExpr::Throw(r, _) => self.walk_expr_names(w, r),
            TExpr::CallStatic(_, args) | TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args)
            | TExpr::ArrayLit(args) | TExpr::ObjLit(args) | TExpr::Default(_, _, args) => {
                for &a in prog.expr_list(args) {
                    self.walk_expr_names(w, a);
                }
            }
            // A `New` is its type tree alone: a named local class is met at its declaration
            // (`walk_local_classes_before`); an anonymous class, whose `TypeDef` stands where its expression does
            // (`new C { .. }` is `{ class $anon ..; new $anon }`), here. An inline expansion's copy of a named local
            // class, whose `TypeDef` the expansion's block holds where no span of the unit tells, before its first
            // construction.
            TExpr::New(c, args) => {
                if w.expanded.contains_key(&c) {
                    self.walk_class_names(w, c);
                }
                for &a in prog.expr_list(args) {
                    self.walk_expr_names(w, a);
                }
                if syms.class(c).owner == Owner::Local && syms.class(c).kind == ClassKind::Anon {
                    self.walk_class_names(w, c);
                }
            }
            TExpr::NewVia(ctor, args) => {
                if let Owner::Class(c) = syms.sym(ctor).owner {
                    if w.expanded.contains_key(&c) {
                        self.walk_class_names(w, c);
                    }
                }
                for &a in prog.expr_list(args) {
                    self.walk_expr_names(w, a);
                }
            }
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.walk_expr_names(w, r);
                for &a in prog.expr_list(args) {
                    self.walk_expr_names(w, a);
                }
            }
            TExpr::Lambda(_, body) => self.walk_expr_names(w, body),
            TExpr::If(c, t, els) => {
                self.walk_expr_names(w, c);
                self.walk_expr_names(w, t);
                if let Some(x) = els {
                    self.walk_expr_names(w, x);
                }
            }
            TExpr::While(a, b) | TExpr::Assign(a, b) | TExpr::Prim(_, a, b) => {
                self.walk_expr_names(w, a);
                self.walk_expr_names(w, b);
            }
            TExpr::Block(stmts, res) => {
                self.walk_stmts_names(w, stmts);
                if let Some(sp) = self.expr_span(res) {
                    self.walk_local_classes_before(w, sp.start);
                }
                self.walk_expr_names(w, res);
                if let Some(sp) = self.expr_span(e) {
                    self.walk_local_classes_before(w, sp.end);
                }
            }
            TExpr::Match(scrut, cases) => {
                self.walk_expr_names(w, scrut);
                for c in prog.cases[cases.range()].to_vec() {
                    self.walk_pat_names(w, c.pat);
                    if let Some(g) = c.guard {
                        self.walk_expr_names(w, g);
                    }
                    self.walk_expr_names(w, c.body);
                }
            }
            TExpr::Try(i) => {
                let t = prog.tries[i as usize].clone();
                self.walk_expr_names(w, t.body);
                for c in prog.cases[t.cases.range()].to_vec() {
                    self.walk_pat_names(w, c.pat);
                    if let Some(g) = c.guard {
                        self.walk_expr_names(w, g);
                    }
                    self.walk_expr_names(w, c.body);
                }
                if let Some(f) = t.finalizer {
                    self.walk_expr_names(w, f);
                }
            }
            _ => {}
        }
    }
}

static EMPTY_NAMES: std::sync::LazyLock<LazyNames> = std::sync::LazyLock::new(LazyNames::default);
