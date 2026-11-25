//! Lazy vals on the JVM as dotty's `LazyVals` writes them (compiler/src/dotty/tools/dotc/transform/LazyVals.scala at
//! 3.8.4), over scala-library's runtime: a local's holder of the `LazyRef` family, read through an accessor and
//! computed by an initialiser under the holder's monitor (`transformLocalDef`, `transformStats`).

use crate::lir::{Capture, Comparing, ErasedType, FieldFlags, Invoke, Kind, LiftedBody, LiftedShape, LirId, MethodDef, MethodFlags, MethodRef, Relation, Visibility};
use super::gen::*;
use super::lifted::{Asker, Placed};
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
    pub fn lazy_holder_class(&mut self, s: SymId) -> (Rc<str>, ErasedType) {
        let t = self.sym_type(s);
        let class = match t {
            ErasedType::I => "scala/runtime/LazyInt",
            ErasedType::J => "scala/runtime/LazyLong",
            ErasedType::Z => "scala/runtime/LazyBoolean",
            ErasedType::F => "scala/runtime/LazyFloat",
            ErasedType::D => "scala/runtime/LazyDouble",
            ErasedType::B => "scala/runtime/LazyByte",
            ErasedType::C => "scala/runtime/LazyChar",
            ErasedType::S => "scala/runtime/LazyShort",
            _ => return (Rc::from(LAZY_REF), ErasedType::object()),
        };
        (Rc::from(class), t)
    }

    pub fn lazy_holder_type(&mut self, s: SymId) -> ErasedType {
        ErasedType::L(self.lazy_holder_class(s).0)
    }

    /// The class file holding the accessor and the initialiser of the local lazy val `s`, and whether it is an
    /// interface: the class whose code declares the val (`Cx::local_home`), wherever that code is written (a value
    /// class's methods in its companion, a constructor's defaults), or for a file's top-level code that file's module
    /// (`Cx::local_file`: the code's file, an inline body's expansion in it included). dotty's LambdaLift lifts both into that class, where a reader in another class calls them.
    pub(super) fn lazy_owner(&mut self, s: SymId) -> (Rc<str>, bool) {
        let cx = self.cx;
        match (cx.local_home(s), cx.local_file(s)) {
            (Some(Some(h)), _) => (self.class_name(h), cx.is_interface(h)),
            (Some(None), Some(file)) => (Rc::from(cx.file_modules[file.0 as usize].as_str()), false),
            // A val whose declaring code the walk did not meet: the methods of the class reading it.
            _ => (self.this_name.clone(), self.is_interface),
        }
    }

    /// What the accessor and the initialiser of the local lazy val `s` take and leave, one shape for
    /// both (`LazyVals.transformLocalDef`, LazyVals.scala 181-194): the declaring class's instance where
    /// the initialiser reads `this`, the key of its method's returns where it returns, what it captures
    /// (a local lazy val as its holder, the key of its initialiser's returns beside it), then the holder;
    /// the val's value.
    pub(super) fn lazy_shape(&mut self, s: SymId) -> (Rc<LiftedShape>, Rc<LiftedBody>) {
        if let Some(found) = self.lifted_shapes.get(&s) {
            return found.clone();
        }
        let cx = self.cx;
        let free = self.an.of_lazy(cx, s);
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
                    captures.push(Capture { sym: f, cell: false, ty: ErasedType::object(), key: true });
                }
                continue;
            }
            let cell = self.held(f) == Held::Cell;
            let ty = if info.by_name { self.function_type(0) } else { self.sym_type(f) };
            captures.push(Capture { sym: f, cell, ty, key: false });
        }
        let instance = home.map(|h| self.class_type(h));
        let captured = captures.iter().map(|c| if c.cell { ErasedType::L(Rc::from(OBJECT_REF)) } else { c.ty.clone() }).collect();
        let holder = self.lazy_holder_type(s);
        let ret = self.sym_type(s);
        let shape = LiftedShape { instance, key: free.nonlocal, captured, params: vec![holder], ret };
        let made = (Rc::new(shape), Rc::new(LiftedBody { code: None, captures, home, module: None }));
        self.lifted_shapes.insert(s, made.clone());
        made
    }

    /// The accessor of the local lazy val `s` (`init` false), `def x(): T = if x$lzy.initialized() then
    /// x$lzy.value() else x$lzyINIT()` (LazyVals.scala 192-194), or its initialiser, `def x$lzyINITN(): T =
    /// x$lzy.synchronized { if x$lzy.initialized() then x$lzy.value() else x$lzy.initialize(rhs) }` (181-190), as
    /// its owner declares them (`lazy_owner`) and every reader calls them: static, of the shape `lazy_shape`
    /// gives; named as dotty's LambdaLift names the accessor `x` and the initialiser `x$lzyINITM` it lifts
    /// (`lifted::LiftWalk`).
    pub(super) fn lazy_method(&mut self, s: SymId, init: bool) -> Rc<Lifted> {
        let cached = if init { self.lifted_lazy_inits.get(&s) } else { self.lifted.get(&s) };
        if let Some(l) = cached {
            return l.clone();
        }
        let cx = self.cx;
        let (shape, body) = self.lazy_shape(s);
        let placed = cx.local_home(s).is_some();
        let (name, place) = self.lifted_entry(if init { Placed::LazyInit(s) } else { Placed::Def(s) }, Asker::LazyMethod, &shape);
        let lifted = Rc::new(Lifted::new(name, shape, body.captures.clone(), body.home, None, place));
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
                let return_target = self.lazy_return_target(s, &lifted);
                self.defer(Pending { lifted: lifted.clone(), body, return_target });
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
            self.defer(Pending { lifted: accessor, body: PendingBody::LazyAccessor(s), return_target: ReturnTarget::Unknown });
            let initialiser = self.lazy_method(s, true);
            let return_target = self.lazy_return_target(s, &initialiser);
            self.defer(Pending { lifted: initialiser, body: PendingBody::LazyInit(s, init), return_target });
        }
    }

    /// The def the `return`s of the method `lifted` of the local lazy val `s` leave: the def around the val, none
    /// where the walk met it in code no def holds.
    fn lazy_return_target(&mut self, s: SymId, lifted: &Lifted) -> ReturnTarget {
        let cx = self.cx;
        if !lifted.key {
            return ReturnTarget::Unknown;
        }
        match (cx.local_home(s), cx.return_def(s)) {
            (_, Some(def)) => ReturnTarget::Def(self.return_type(def)),
            (Some(_), None) => ReturnTarget::NoDef,
            (None, None) => ReturnTarget::Unknown,
        }
    }

    /// The tree of the accessor of `s`, its holder in `slot` (`PendingBody::LazyAccessor`): `if
    /// x$lzy.initialized() then x$lzy.value() else x$lzyINIT()`, each returned.
    pub fn lazy_accessor_tree(&mut self, s: SymId, slot: u16) -> LirId {
        let (class, value) = self.lazy_holder_class(s);
        let holder = ErasedType::L(class.clone());
        self.m.locals.insert(s, Local { slot, ty: holder.clone(), cell: false });
        let ret = self.m.ret.clone();
        let method = |name: &str, ret: ErasedType| {
            let desc = method_desc(&[], &ret);
            Rc::new(MethodRef { owner: class.clone(), owner_is_interface: false, name: name.to_string(), params: Vec::new(), ret, desc, kind: Invoke::Virtual })
        };
        let (initialized, read) = (method("initialized", ErasedType::Z), method("value", value.clone()));
        self.lowering(|g| {
            let tested = g.lir.push(Kind::Local(s), holder.clone());
            let cond = g.lir.call(&initialized, tested, &[]);
            let held = g.lir.push(Kind::Local(s), holder.clone());
            let held = g.lir.call(&read, held, &[]);
            let held = g.lower_adapt(held, &value, &ret);
            let then = g.lir.ret(held);
            let computed = g.lower_lazy_call(s, true);
            let otherwise = Some(g.lir.ret(computed));
            g.lir.push(Kind::If { cond, then, otherwise }, ErasedType::V)
        })
    }

    /// The tree of the initialiser of `s`, its holder in `slot`, the right-hand side `init`
    /// (`PendingBody::LazyInit`): `x$lzy.synchronized { if x$lzy.initialized() then x$lzy.value()
    /// else x$lzy.initialize(rhs) }`, returned.
    pub fn lazy_init_tree(&mut self, s: SymId, init: TExprId, slot: u16) -> LirId {
        let (class, value) = self.lazy_holder_class(s);
        let holder = ErasedType::L(class.clone());
        self.m.locals.insert(s, Local { slot, ty: holder.clone(), cell: false });
        let ret = self.m.ret.clone();
        let method = |name: &str, params: Vec<ErasedType>, ret: ErasedType| {
            let desc = method_desc(&params, &ret);
            Rc::new(MethodRef { owner: class.clone(), owner_is_interface: false, name: name.to_string(), params, ret, desc, kind: Invoke::Virtual })
        };
        let (initialized, read, initialize) = (method("initialized", Vec::new(), ErasedType::Z), method("value", Vec::new(), value.clone()), method("initialize", vec![value.clone()], value.clone()));
        self.lowering(|g| {
            let lock = g.lir.push(Kind::Local(s), holder.clone());
            let tested = g.lir.push(Kind::Local(s), holder.clone());
            let cond = g.lir.call(&initialized, tested, &[]);
            let held = g.lir.push(Kind::Local(s), holder.clone());
            let held = g.lir.call(&read, held, &[]);
            let then = g.lower_adapt(held, &value, &ret);
            let initialized_by = g.lir.push(Kind::Local(s), holder.clone());
            let computed = g.lower_expr(init, &ret);
            let computed = g.lower_adapt(computed, &ret, &value);
            let stored = g.lir.call(&initialize, initialized_by, &[computed]);
            let otherwise = Some(g.lower_adapt(stored, &value, &ret));
            let body = g.lir.push(Kind::If { cond, then, otherwise }, ret.clone());
            let synchronized = g.push_protected(Kind::Synchronized { lock, body, returns: false }, ret.clone());
            g.lir.ret(synchronized)
        })
    }
}

/// A thread-safe lazy member of the class being written (`LazyVals.transformMemberDefThreadSafeNew`): its getter's
/// name and type, its `@volatile` `Object` container, the container's static `VarHandle` and its initialiser.
#[derive(Clone)]
pub struct LazyHolder {
    pub getter: String,
    pub ty: ErasedType,
    pub container: String,
    pub handle: String,
    pub init: String,
}

impl<'a> Gen<'a> {
    /// The holder of the lazy member `getter` of the class being written, made at its first asking
    /// with its names (§8) and its handle declared before the class's other fields (`addInFront`),
    /// so that the later one comes first (`varHandleInfo.defs = h :: defs`).
    pub fn lazy_holder(&mut self, getter: &str, ty: &ErasedType) -> LazyHolder {
        if let Some(h) = self.lazy_holders.iter().find(|h| h.getter == getter) {
            return h.clone();
        }
        let (container, init) = self.lazy_member_names(getter);
        let handle = format!("{}$lzyHandle", container);
        self.field_first(FieldFlags { is_static: true, is_final: true, ..FieldFlags::PRIVATE }, &handle, &ErasedType::L(Rc::from(VAR_HANDLE)));
        let h = LazyHolder { getter: getter.to_string(), ty: ty.clone(), container, handle, init };
        self.lazy_holders.push(h.clone());
        h
    }

    /// A lazy member the class holds beside its fields' (a made companion, a Scala 2 trait's private
    /// lazy val or object): its container, its getter `getter` of the given access, its
    /// initialiser computing `init`.
    pub fn thread_safe_member(&mut self, getter: &str, ty: &ErasedType, init: super::classes::HolderInit, flags: MethodFlags) {
        let h = self.lazy_holder(getter, ty);
        self.field(FieldFlags { volatile: true, ..FieldFlags::PRIVATE }, &h.container, &ErasedType::object());
        let desc = method_desc(&[], ty);
        self.hold_member(flags, getter, &desc, &[], ty.clone(), &[], super::classes::MethodSite::default(), "lazy member's getter", |g| g.lower_thread_safe_getter(&h));
        self.thread_safe_init(&h, &init);
    }

    /// The `VarHandle` lookups of the class's lazy members, first in its `<clinit>`
    /// (`MoveStatics.rebuild` of the handles `addInFront` put first): `MethodHandles.lookup()
    /// .findVarHandle(classOf[C], "x$lzyN", classOf[Object])`, in the handles' order.
    pub(super) fn lower_var_handle_lookups(&mut self) -> Vec<LirId> {
        let owner = self.this_name.clone();
        let lookup_class: Rc<str> = Rc::from("java/lang/invoke/MethodHandles$Lookup");
        let class = ErasedType::L(Rc::from("java/lang/Class"));
        let string = ErasedType::L(Rc::from(STRING));
        let handle_type = ErasedType::L(Rc::from(VAR_HANDLE));
        let lookup = method_ref(Invoke::Static, &Rc::from("java/lang/invoke/MethodHandles"), false, "lookup", &[], &ErasedType::L(lookup_class.clone()));
        let find = method_ref(Invoke::Virtual, &lookup_class, false, "findVarHandle", &[class.clone(), string.clone(), class], &handle_type);
        let mut stores = Vec::new();
        for h in self.lazy_holders.clone().iter().rev() {
            let lookups = self.lir.static_call(&lookup, &[]);
            let of = self.lir.class_of(ErasedType::L(owner.clone()));
            let container = self.lir.text(&Rc::from(h.container.as_str()), string.clone());
            let object = self.lir.class_of(ErasedType::object());
            let handle = self.lir.call(&find, lookups, &[of, container, object]);
            stores.push(self.lir.store_static(&owner, &h.handle, handle));
        }
        stores
    }

    /// What a `<clinit>` is: static, and private where the class has lazy members' handles, as
    /// `MoveStatics` makes the static constructor of the `@static` handles. No handle is
    /// registered while a `<clinit>` is written (`lazy_holder`): what it is holds from its start.
    pub fn clinit_flags(&self) -> MethodFlags {
        let visibility = if self.lazy_holders.is_empty() { Visibility::Package } else { Visibility::Private };
        MethodFlags { visibility, is_static: true, ..MethodFlags::PUBLIC }
    }

    /// Whether the erased type `tp` is a class `LazyValControlState` derives from (2.5).
    fn control_state_super(&self, tp: &ErasedType) -> bool {
        matches!(tp, ErasedType::L(n) if self.cx.control_state_bases.iter().any(|b| **b == **n))
    }

    /// The getter of a `@threadUnsafe` member `getter` of the type `ty` over its container and flag
    /// (`mkThreadUnsafeDef`, LazyVals.scala 230-239): `if !flag then { container = rhs; flag = true }; container`,
    /// the right-hand side the member's own or a trait's static (`HolderInit::TraitStatic`), none where it has none
    /// (reported by the caller), held in the context `method` (`Gen::hold_tree`).
    pub fn thread_unsafe_getter(&mut self, method: MethodDef, getter: &str, ty: &ErasedType, init: Option<super::classes::HolderInit>) {
        use super::classes::HolderInit;
        let (container, flag) = self.thread_unsafe_names(getter);
        let owner = self.this_name.clone();
        let this = ErasedType::L(owner.clone());
        self.hold_tree(method, "@threadUnsafe lazy val's getter", false, |g| {
            let mut stmts = Vec::new();
            if let Some(init) = init {
                let receiver = g.lir.this(this.clone());
                let set = g.lir.field(receiver, &owner, &flag, ErasedType::Z);
                let unset = g.lir.push(Kind::Not(set), ErasedType::Z);
                let value = match init {
                    HolderInit::Expr(e) => g.lower_expr(e, ty),
                    HolderInit::TraitStatic(base) => {
                        let forwarder = method_ref(Invoke::Static, &base, true, &format!("{}$", getter), &[ErasedType::L(base.clone())], ty);
                        let instance = g.lir.this(this.clone());
                        g.lir.static_call(&forwarder, &[instance])
                    }
                    HolderInit::Module { .. } => unreachable!("a `@threadUnsafe` member's own or its trait's right-hand side"),
                };
                let receiver = g.lir.this(this.clone());
                let stored = g.lir.store(receiver, &owner, &container, value);
                let receiver = g.lir.this(this.clone());
                let done = g.lir.bool(true);
                let flagged = g.lir.store(receiver, &owner, &flag, done);
                let unit = g.lir.unit();
                let compute = g.lir.seq(&[stored, flagged], unit);
                stmts.push(g.lir.push(Kind::If { cond: unset, then: compute, otherwise: None }, ErasedType::V));
            }
            let receiver = g.lir.this(this.clone());
            let held = g.lir.field(receiver, &owner, &container, ty.clone());
            let answer = g.lir.ret(held);
            g.lir.seq(&stmts, answer)
        });
    }

    /// The getter of a lazy member over its holder (`mkThreadSafeDef` 374-389): `val r = container; if <r holds a
    /// value> then r.asInstanceOf[T] else if r eq NullValue then null else x$lzyINITN()`, returned. A value is
    /// `r.isInstanceOf[T]`, or for a type a control state's class derives from `r != null &&
    /// !r.isInstanceOf[LazyValControlState]` (`immediateValueCondition`, 367-371).
    pub(super) fn lower_thread_safe_getter(&mut self, h: &LazyHolder) -> LirId {
        let owner = self.this_name.clone();
        let this = ErasedType::L(owner.clone());
        let object = ErasedType::object();
        let tp = h.ty.clone();
        let tested: Rc<str> = match &tp {
            ErasedType::L(n) => n.clone(),
            prim => Rc::from(prim.box_class()),
        };
        let r = self.lir.temp();
        let receiver = self.lir.this(this.clone());
        let container = self.lir.field(receiver, &owner, &h.container, object.clone());
        let held = self.lir.push(Kind::Let { temp: r, init: container }, ErasedType::V);
        let value = self.lir.push(Kind::Temp(r), object.clone());
        let cond = if self.control_state_super(&tp) {
            let null = self.lir.push(Kind::Null, object.clone());
            let present = self.lir.push(Kind::Compare { relation: Relation::Ne, how: Comparing::Identity, left: value, right: null }, ErasedType::Z);
            let value = self.lir.push(Kind::Temp(r), object.clone());
            let control = self.lir.push(Kind::Is { value, class: Rc::from(LAZY_CONTROL) }, ErasedType::Z);
            let settled = self.lir.push(Kind::Not(control), ErasedType::Z);
            self.lir.push(Kind::And(present, settled), ErasedType::Z)
        } else {
            self.lir.push(Kind::Is { value, class: tested }, ErasedType::Z)
        };
        let value = self.lir.push(Kind::Temp(r), object.clone());
        let then = self.lower_adapt(value, &object, &tp);
        let value = self.lir.push(Kind::Temp(r), object.clone());
        let marker = self.lir.module(&Rc::from(NULL_VALUE));
        let is_null_value = self.lir.push(Kind::Compare { relation: Relation::Eq, how: Comparing::Identity, left: value, right: marker }, ErasedType::Z);
        let null = self.lir.push(Kind::Null, object.clone());
        let null = self.lower_adapt(null, &object, &tp);
        let desc = method_desc(&[], &object);
        let init = Rc::new(MethodRef { owner: owner.clone(), owner_is_interface: false, name: h.init.clone(), params: Vec::new(), ret: object.clone(), desc, kind: Invoke::Special });
        let receiver = self.lir.this(this);
        let computed = self.lir.call(&init, receiver, &[]);
        let computed = self.lower_adapt(computed, &object, &tp);
        let otherwise = Some(self.lir.push(Kind::If { cond: is_null_value, then: null, otherwise: Some(computed) }, tp.clone()));
        let answer = self.lir.push(Kind::If { cond, then, otherwise }, tp);
        let answer = self.lir.ret(answer);
        self.lir.seq(&[held], answer)
    }

    /// `x$lzyINITN`, the initialiser as dotty's `mkThreadSafeDef` builds it (LazyVals.scala 353-469): a loop with no
    /// condition over the container's state (`initMainLoop`, 466); where it is null, `null` to `Evaluating` by a
    /// compare-and-set, the right-hand side computed in a `try` whose `finally` publishes the value (or `NullValue`
    /// for null), handing it to the waiters' `Waiting` latch where one was installed, and the value returned;
    /// `Evaluating` met is replaced by a `Waiting`, `Waiting` waited on, `NullValue` answered as null, a value
    /// answered (`ifNotUninitialized`, 445-463).
    pub fn thread_safe_init(&mut self, h: &LazyHolder, rhs: &super::classes::HolderInit) {
        use super::classes::HolderInit;
        let object = ErasedType::object();
        let roots: Vec<TExprId> = match rhs {
            HolderInit::Expr(e) => vec![*e],
            _ => Vec::new(),
        };
        // Two lazy members of one name in one class would be a fault of the names (§8): never one dropped in silence.
        let init_desc = method_desc(&[], &object);
        if self.plan.has_method(&h.init, &init_desc) {
            self.unsupported(&format!("internal error: a second lazy initialiser {} in the class", h.init));
            return;
        }
        let flags = MethodFlags { visibility: Visibility::Private, ..MethodFlags::PUBLIC };
        self.hold_member(flags, &h.init, "()Ljava/lang/Object;", &[], object, &roots, super::classes::MethodSite::default(), "lazy val's initialiser", |g| g.lower_thread_safe_init(h, rhs));
    }

    fn lower_thread_safe_init(&mut self, h: &LazyHolder, rhs: &super::classes::HolderInit) -> LirId {
        use super::classes::HolderInit;
        let owner = self.this_name.clone();
        let this = ErasedType::L(owner.clone());
        let object = ErasedType::object();
        let z = ErasedType::Z;
        let evaluating: Rc<str> = Rc::from(LAZY_EVALUATING);
        let waiting: Rc<str> = Rc::from(LAZY_WAITING);
        let waiting_ty = ErasedType::L(waiting.clone());
        let null_value: Rc<str> = Rc::from(NULL_VALUE);
        let control: Rc<str> = Rc::from(LAZY_CONTROL);
        // `CAS(this, expected, new)` over the container (`casFlag`, 365-369)
        let container_name: Rc<str> = Rc::from(h.container.as_str());
        let cas_of = |g: &mut Self, expected: LirId, new: LirId| {
            let object = g.lir.this(ErasedType::L(owner.clone()));
            g.lir.push(Kind::LazyCas { object, owner: owner.clone(), field: container_name.clone(), expected, new }, ErasedType::Z)
        };
        let container = |g: &mut Self| {
            let receiver = g.lir.this(ErasedType::L(owner.clone()));
            g.lir.field(receiver, &owner, &h.container, ErasedType::object())
        };
        let eq = |g: &mut Self, left: LirId, right: LirId| g.lir.push(Kind::Compare { relation: Relation::Eq, how: Comparing::Identity, left, right }, ErasedType::Z);
        let method = |owner: &Rc<str>, name: &str, kind: Invoke| Rc::new(MethodRef { owner: owner.clone(), owner_is_interface: false, name: name.into(), params: Vec::new(), ret: ErasedType::V, desc: "()V".into(), kind });
        // `if CAS(_x, null, Evaluating) then { var result = null; var resultNullable = null; try ...; return
        // resultNullable }` (`initialize`, 393-441)
        let (result, nullable) = (self.lir.temp(), self.lir.temp());
        let initialize = {
            let null = self.lir.push(Kind::Null, object.clone());
            let ev = self.lir.module(&evaluating);
            let cond = cas_of(self, null, ev);
            let null = self.lir.push(Kind::Null, object.clone());
            let declare_result = self.lir.push(Kind::Let { temp: result, init: null }, ErasedType::V);
            let null = self.lir.push(Kind::Null, object.clone());
            let declare_nullable = self.lir.push(Kind::Let { temp: nullable, init: null }, ErasedType::V);
            let value = match rhs {
                HolderInit::Expr(e) => self.lower_expr(*e, &h.ty),
                HolderInit::TraitStatic(base) => {
                    let static_ = Rc::new(MethodRef { owner: base.clone(), owner_is_interface: true, name: format!("{}$", h.getter), params: vec![ErasedType::L(base.clone())], ret: h.ty.clone(), desc: method_desc(&[ErasedType::L(base.clone())], &h.ty), kind: Invoke::Static });
                    let receiver = self.lir.this(this.clone());
                    self.lir.static_call(&static_, &[receiver])
                }
                HolderInit::Module { class, outer } => {
                    let ctor = Rc::new(MethodRef { owner: class.clone(), owner_is_interface: false, name: "<init>".into(), params: vec![ErasedType::L(outer.clone())], ret: ErasedType::V, desc: method_desc(&[ErasedType::L(outer.clone())], &ErasedType::V), kind: Invoke::Special });
                    let receiver = self.lir.this(this.clone());
                    self.lir.make(&ctor, &[receiver])
                }
            };
            // `resultNullable = rhs`, boxed (412)
            let boxed = self.lower_adapt(value, &h.ty.clone(), &object);
            let assign_nullable = self.lir.push(Kind::SetTemp { temp: nullable, value: boxed }, ErasedType::V);
            // `if resultNullable eq null then result = NullValue else result = resultNullable` (413-417)
            let read = self.lir.push(Kind::Temp(nullable), object.clone());
            let null = self.lir.push(Kind::Null, object.clone());
            let is_null = eq(self, read, null);
            let marker = self.lir.module(&null_value);
            let to_marker = self.lir.push(Kind::SetTemp { temp: result, value: marker }, ErasedType::V);
            let read = self.lir.push(Kind::Temp(nullable), object.clone());
            let to_value = self.lir.push(Kind::SetTemp { temp: result, value: read }, ErasedType::V);
            let publish_value = self.lir.push(Kind::If { cond: is_null, then: to_marker, otherwise: Some(to_value) }, ErasedType::V);
            let unit = self.lir.unit();
            let body = self.lir.seq(&[assign_nullable, publish_value], unit);
            // `finally if !CAS(_x, Evaluating, result) then { val lock = _x.asInstanceOf[Waiting]; CAS(_x, lock,
            // result); lock.countDown() }` (`fin`, `lockRel`, 396-406), with no `else` (dotty's `unitLiteral`)
            let ev = self.lir.module(&evaluating);
            let read = self.lir.push(Kind::Temp(result), object.clone());
            let published = cas_of(self, ev, read);
            let failed = self.lir.push(Kind::Not(published), z.clone());
            let lock = self.lir.temp();
            let held = container(self);
            let held = self.lir.push(Kind::Cast { value: held, op: crate::lir::Cast::Checked }, waiting_ty.clone());
            let declare_lock = self.lir.push(Kind::Let { temp: lock, init: held }, ErasedType::V);
            let read_lock = self.lir.push(Kind::Temp(lock), waiting_ty.clone());
            let read = self.lir.push(Kind::Temp(result), object.clone());
            let handed = cas_of(self, read_lock, read);
            let handed = self.lower_adapt(handed, &z, &ErasedType::V);
            let read_lock = self.lir.push(Kind::Temp(lock), waiting_ty.clone());
            let release = self.lir.call(&method(&waiting, "countDown", Invoke::Virtual), read_lock, &[]);
            let release_lock = self.lir.seq(&[declare_lock, handed], release);
            let finalizer = self.lir.push(Kind::If { cond: failed, then: release_lock, otherwise: None }, ErasedType::V);
            let evaluate = self.push_protected(Kind::Try { body, catch: None, finalizer: Some(finalizer), returns: false }, ErasedType::V);
            let read = self.lir.push(Kind::Temp(nullable), object.clone());
            let answer = self.lir.ret(read);
            let then = self.lir.seq(&[declare_result, declare_nullable, evaluate], answer);
            self.lir.push(Kind::If { cond, then, otherwise: None }, ErasedType::V)
        };
        // A control state: `Evaluating` gets a latch, `Waiting` is waited on, `NullValue` answers null; a value is
        // answered (`ifNotUninitialized`, 445-463).
        let current = self.lir.temp();
        let not_uninitialized = {
            let read = self.lir.push(Kind::Temp(current), object.clone());
            let is_control = self.lir.push(Kind::Is { value: read, class: control.clone() }, z.clone());
            let read = self.lir.push(Kind::Temp(current), object.clone());
            let ev = self.lir.module(&evaluating);
            let is_evaluating = eq(self, read, ev);
            let latch = self.lir.make(&method(&waiting, "<init>", Invoke::Special), &[]);
            let read = self.lir.push(Kind::Temp(current), object.clone());
            let install = cas_of(self, read, latch);
            let install = self.lower_adapt(install, &z, &ErasedType::V);
            let read = self.lir.push(Kind::Temp(current), object.clone());
            let is_waiting = self.lir.push(Kind::Is { value: read, class: waiting.clone() }, z.clone());
            let read = self.lir.push(Kind::Temp(current), object.clone());
            let latch = self.lir.push(Kind::Cast { value: read, op: crate::lir::Cast::Checked }, waiting_ty.clone());
            let wait = self.lir.call(&method(&waiting, "await", Invoke::Virtual), latch, &[]);
            let null = self.lir.push(Kind::Null, object.clone());
            let answer_null = self.lir.ret(null);
            let not_evaluating = self.lir.push(Kind::If { cond: is_waiting, then: wait, otherwise: Some(answer_null) }, ErasedType::V);
            let state = self.lir.push(Kind::If { cond: is_evaluating, then: install, otherwise: Some(not_evaluating) }, ErasedType::V);
            let read = self.lir.push(Kind::Temp(current), object.clone());
            let answer = self.lir.ret(read);
            self.lir.push(Kind::If { cond: is_control, then: state, otherwise: Some(answer) }, ErasedType::V)
        };
        // `while <EmptyTree> do { val current = _x; if current eq null then initialize else ifNotUninitialized }`
        // (`initBody`, `initMainLoop`, 465-466)
        let held = container(self);
        let declare_current = self.lir.push(Kind::Let { temp: current, init: held }, ErasedType::V);
        let read = self.lir.push(Kind::Temp(current), object.clone());
        let null = self.lir.push(Kind::Null, object);
        let uninitialized = eq(self, read, null);
        let step = self.lir.push(Kind::If { cond: uninitialized, then: initialize, otherwise: Some(not_uninitialized) }, ErasedType::V);
        let body = self.lir.seq(&[declare_current], step);
        self.lir.push(Kind::While { cond: None, body }, ErasedType::V)
    }
}

/// The numbers of the lazy names of a source file (`LazyLocalName`, `LazyLocalInitName`, `LazyBitMapName`, counted per
/// compilation unit by `FreshNameCreator`, NameKinds.scala 220-222, util/FreshNameCreator.scala 24-28): per member its
/// container's and its initialiser's (a `@threadUnsafe` member's: its flag's), per local lazy val its initialiser's.
#[derive(Default)]
pub struct LazyNames {
    members: crate::intern::FxMap<(Option<ClassId>, String), (u32, u32)>,
    /// A local lazy val's holder's number and its initialiser's.
    locals: crate::intern::FxMap<SymId, (u32, u32)>,
}

/// The walk of a file in the order dotty's `LazyVals` meets its lazy definitions (`Gen::walk_lazy_names`).
#[derive(Default)]
struct NameWalk {
    unit: UnitWalk,
    counts: crate::intern::FxMap<(String, NameCount), u32>,
    out: LazyNames,
}

/// What the walks of a unit's code share (`UnitVisitor`): the classes walked, the file's local classes by place, the
/// named local classes an inline expansion made, and where the definition being walked starts.
#[derive(Default)]
pub(super) struct UnitWalk {
    pub(super) walked: crate::intern::FxMap<ClassId, ()>,
    /// The file's local classes by the start of their spans.
    locals: Vec<(u32, ClassId)>,
    /// The named local classes an inline expansion in the file made, whose spans are their template's: each walked
    /// where the walk first meets its construction.
    expanded: crate::intern::FxMap<ClassId, ()>,
    /// The start of the definition being walked: its own local classes are the ones after it.
    pub(super) scope: u32,
}

/// A walk of a unit's code in the order a phase of dotty's meets its definitions: the lazy names' (`NameWalk`,
/// `LazyVals` numbering a definition after the trees inside it) and the lifted methods' (`lifted::LiftWalk`,
/// `LambdaLift` naming a definition before them). The code's traversal is one (`Gen::walk_expr`); a template's is each
/// walk's own (`class`), the two orders of a template being the one difference.
pub(super) trait UnitVisitor<'a> {
    /// Whether an anonymous class is walked at its creation alone, before its arguments, where
    /// `{ class $anon ..; new $anon(..) }` holds it; else after them, or at its place where a statement after it
    /// comes first.
    const ANON_FIRST: bool;
    fn unit(&mut self) -> &mut UnitWalk;
    /// The template of the class `c`, in this walk's order.
    fn class(&mut self, g: &mut Gen<'a>, c: ClassId);
    /// The creation of the anonymous class `c` with `args`, its captures then its superclass's arguments: whether
    /// this walk took it whole.
    fn anonymous(&mut self, _g: &mut Gen<'a>, _c: ClassId, _args: ListRef) -> bool {
        false
    }
    /// A block, before its statements.
    fn block(&mut self, _g: &mut Gen<'a>, _e: TExprId) {}
    fn block_end(&mut self) {}
    /// A lambda over `params` whose body is `body`: whether the body is walked.
    fn lambda(&mut self, _g: &mut Gen<'a>, _params: ListRef, _body: TExprId) -> bool {
        true
    }
    fn lambda_end(&mut self) {}
    /// A local def: whether it is walked.
    fn local_fun(&mut self, _g: &mut Gen<'a>, _f: FunId) -> bool {
        true
    }
    fn local_fun_end(&mut self) {}
    /// The getter of the default of the def `f`'s own parameter `i` (`TFun::own_defaults`), a local def's where
    /// `local`.
    fn default_getter(&mut self, _g: &mut Gen<'a>, _f: FunId, _i: usize, _local: bool) {}
    fn default_getter_end(&mut self) {}
    /// The right-hand side of the local val `sym` (of a pattern definition, `None`): whether it is walked.
    fn val(&mut self, _g: &mut Gen<'a>, _sym: Option<SymId>) -> bool {
        true
    }
    fn val_end(&mut self, _g: &mut Gen<'a>, _sym: Option<SymId>) {}
    /// Each expression the walk meets, before what it holds.
    fn expr(&mut self, _g: &mut Gen<'a>, _e: TExprId) {}
    /// Each pattern the walk meets, before the expressions it holds.
    fn pattern(&mut self, _g: &mut Gen<'a>, _p: TPatId) {}
}

/// A kind of lazy name, each counted apart per name: `x$lzyN`, `x$lzyINITN`, `xbitmap$N`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum NameCount {
    Container,
    Initialiser,
    Flag,
}

/// `StdNames.str.sanitize` (StdNames.scala 44-45): `<` and `>` replaced by `$`.
pub(super) fn sanitize(raw: &str) -> String {
    raw.replace(['<', '>'], "$")
}

impl NameWalk {
    /// `FreshNameCreator.newName` (24-28): the counter of `sanitize(prefix) + separator`, the prefix the
    /// definition's name as written (`raw`), apart from its mangling in the emitted name.
    fn fresh(&mut self, raw: &str, kind: NameCount) -> u32 {
        let n = self.counts.entry((sanitize(raw), kind)).or_insert(0);
        *n += 1;
        *n
    }

    /// A thread-safe member: its container's number, then its initialiser's (`transformMemberDefThreadSafeNew`
    /// 476, `mkThreadSafeDef` 359), kept by its getter's emitted name; a `@threadUnsafe` one: its container's, then
    /// its flag's (`transformMemberDefThreadUnsafe` 268, 276).
    fn member(&mut self, owner: Option<ClassId>, getter: &str, raw: &str, thread_unsafe: bool) {
        let n = self.fresh(raw, NameCount::Container);
        let m = self.fresh(raw, if thread_unsafe { NameCount::Flag } else { NameCount::Initialiser });
        self.out.members.entry((owner, getter.to_string())).or_insert((n, m));
    }

    /// A made companion of a class: `transformSyntheticModule` names its eager holder alone (140); teq's
    /// thread-safe holder takes the container's number for its initialiser too.
    fn made(&mut self, owner: Option<ClassId>, getter: &str, raw: &str) {
        let n = self.fresh(raw, NameCount::Container);
        self.out.members.entry((owner, getter.to_string())).or_insert((n, n));
    }

    /// A local lazy val: its holder's number, then its initialiser's (`transformLocalDef` 168, 181).
    fn local(&mut self, s: SymId, raw: &str) {
        let n = self.fresh(raw, NameCount::Container);
        let m = self.fresh(raw, NameCount::Initialiser);
        self.out.locals.insert(s, (n, m));
    }
}

impl<'a> UnitVisitor<'a> for NameWalk {
    const ANON_FIRST: bool = false;

    fn unit(&mut self) -> &mut UnitWalk {
        &mut self.unit
    }

    fn class(&mut self, g: &mut Gen<'a>, c: ClassId) {
        g.walk_class_names(self, c);
    }

    fn val_end(&mut self, g: &mut Gen<'a>, sym: Option<SymId>) {
        if let Some(s) = sym.filter(|&s| g.is_lazy_local(s)) {
            self.local(s, g.cx.input.interner.get(g.cx.input.syms.sym(s).name));
        }
    }
}

impl<'a> Gen<'a> {
    /// The lazy names of the file `file`, walked once whichever class asks first.
    pub(super) fn file_lazy_names(&mut self, file: crate::source::FileId) -> &'a LazyNames {
        let cx = self.cx;
        match cx.lazy_names.get(file.0 as usize) {
            Some(cell) => cell.get_or_init(|| self.walk_lazy_names(file)),
            None => &EMPTY_NAMES,
        }
    }

    /// The numbers of a lazy member's container and initialiser (a `@threadUnsafe` one's flag), by the file's walk: of
    /// the class `owner`'s member whose getter is `getter` (the file's module's for `None`).
    pub(super) fn lazy_member_numbers_in(&mut self, file: crate::source::FileId, owner: Option<ClassId>, getter: &str) -> Option<(u32, u32)> {
        self.file_lazy_names(file).members.get(&(owner, getter.to_string())).copied()
    }

    /// The names of a lazy member's container and initialiser in the class being written, `x$lzyN` and
    /// `x$lzyINITM`, by the file's walk.
    pub(super) fn lazy_member_names(&mut self, getter: &str) -> (String, String) {
        let (n, m) = self.lazy_member_numbers(getter);
        (format!("{}$lzy{}", getter, n), format!("{}$lzyINIT{}", getter, m))
    }

    /// The names of a `@threadUnsafe` member's container and flag in the class being written, `x$lzyN` and
    /// `xbitmap$M`, by the file's walk.
    pub(super) fn thread_unsafe_names(&mut self, getter: &str) -> (String, String) {
        let (n, m) = self.lazy_member_numbers(getter);
        (format!("{}$lzy{}", getter, n), format!("{}bitmap${}", getter, m))
    }

    fn lazy_member_numbers(&mut self, getter: &str) -> (u32, u32) {
        let owner = self.this_class;
        let file = match owner {
            Some(c) => self.cx.input.syms.class(c).module_file(),
            None => self.line_file,
        };
        let Some(numbers) = self.lazy_member_numbers_in(file, owner, getter) else {
            self.unsupported(&format!("internal error: the lazy member {} has no number in its unit", getter));
            return (0, 0);
        };
        numbers
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
    pub(super) fn lazy_local_numbers(&mut self, s: SymId) -> Option<(u32, u32)> {
        let file = self.cx.local_file(s).unwrap_or(self.cx.input.syms.sym(s).file);
        self.file_lazy_names(file).locals.get(&s).copied()
    }

    /// The classes of the file `file` that stand at the top level, by place, a made companion left out, and its
    /// opaque types' companions; its local classes and the named local classes its inline expansions made noted in
    /// `w` (`walk_lazy_names`, `walk_lifted_names`).
    pub(super) fn unit_classes(&mut self, file: crate::source::FileId, w: &mut UnitWalk) -> (Vec<(u32, ClassId)>, Vec<ClassId>) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let mut tops: Vec<(u32, ClassId)> = Vec::new();
        let opaque = cx.opaque_companions.get(&file.0).cloned().unwrap_or_default();
        for &c in cx.unit_classes(file) {
            let info = syms.class(c);
            let made = info.made_at.is_some() || info.placed_at.is_some();
            match info.owner {
                Owner::Package(_) if !opaque.contains(&c) && !made => tops.push((info.span.start, c)),
                // An expansion's copy is met at its first construction, an anonymous one at its `New`.
                Owner::Local if made => {
                    if info.kind != ClassKind::Anon {
                        w.expanded.insert(c, ());
                    }
                }
                // A class a dropped typing made (`ClassInfo::dropped`) is no class of the file's walk: the one the
                // kept typing made at its place takes the numbers.
                Owner::Local if syms.in_dropped_local(c) => {}
                Owner::Local => w.locals.push((info.span.start, c)),
                _ => {}
            }
        }
        tops.sort();
        w.locals.sort();
        (tops, opaque)
    }

    /// The package object of the file `file`: its vals, defs and opaque types' companions by place (kinds 0, 1, 2
    /// with a symbol's, a function's and a class's id), and the vals' initialisers.
    pub(super) fn unit_package_items(&mut self, file: crate::source::FileId, opaque: &[ClassId]) -> (Vec<(u32, u8, u32)>, crate::intern::FxMap<SymId, TExprId>) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let mut items: Vec<(u32, u8, u32)> = Vec::new();
        let group = cx.file_group[file.0 as usize];
        if group != u32::MAX {
            for &(s, _) in cx.layout.file_groups[group as usize].1.iter() {
                items.push((syms.sym(s).span.start, 0, s.0));
            }
        }
        for &f in &cx.file_funs[file.0 as usize] {
            items.push((syms.sym(cx.input.prog.funs[f.idx()].sym).span.start, 1, f.0));
        }
        for &k in opaque {
            items.push((syms.class(k).span.start, 2, k.0));
        }
        items.sort();
        let vals = if group == u32::MAX { Default::default() } else { cx.layout.file_groups[group as usize].1.iter().copied().collect() };
        (items, vals)
    }

    /// The unit's classes the walk did not meet (a quote's, an expansion's that nothing constructs), after the rest
    /// by their binary names: every definition the unit writes has its numbers, the same on every run.
    /// A class a dropped typing made, and whatever is nested in one however deep (`Symbols::in_dropped_local`:
    /// an object of a dropped local class is owned by it and carries no mark of its own), comes last of all:
    /// it has the binary name of the class the kept typing made at its place, which takes the numbers as
    /// where nothing was typed and dropped.
    pub(super) fn unit_rest(&mut self, file: crate::source::FileId, w: &UnitWalk, opaque: &[ClassId]) -> Vec<ClassId> {
        let cx = self.cx;
        let syms = cx.input.syms;
        let mut rest: Vec<(bool, Rc<str>, ClassId)> = Vec::new();
        for &c in cx.unit_classes(file) {
            if !w.walked.contains_key(&c) && !opaque.contains(&c) {
                rest.push((syms.in_dropped_local(c), self.class_name(c), c));
            }
        }
        rest.sort();
        rest.into_iter().map(|(_, _, c)| c).collect()
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
        let (tops, opaque) = self.unit_classes(file, &mut w.unit);
        for (_, c) in tops {
            self.walk_class_names(&mut w, c);
        }
        // The package object: the file's vals and defs in their order, its opaque types' companions.
        let (items, vals) = self.unit_package_items(file, &opaque);
        for (_, kind, id) in items {
            match kind {
                0 => {
                    let s = SymId(id);
                    if let Some(&e) = vals.get(&s) {
                        w.unit.scope = syms.sym(s).span.start;
                        self.walk_expr(&mut w, e);
                    }
                    let info = syms.sym(s);
                    let lazy = info.mods & mods::LAZY != 0 || (info.kind == SymKind::Given && !self.is_given_def(s));
                    if lazy && !cx.given_objects.contains_key(&s) && !cx.given_classes.contains_key(&s) {
                        let name = self.field_name(s);
                        w.member(None, &name, cx.input.interner.get(info.name), self.is_thread_unsafe(s));
                    }
                }
                1 => self.walk_fun(&mut w, FunId(id), false),
                _ => self.walk_class_names(&mut w, ClassId(id)),
            }
        }
        for c in self.unit_rest(file, &w.unit, &opaque) {
            self.walk_class_names(&mut w, c);
        }
        w.out
    }

    fn walk_class_names(&mut self, w: &mut NameWalk, c: ClassId) {
        if w.unit.walked.insert(c, ()).is_some() {
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
        let outer_scope = std::mem::replace(&mut w.unit.scope, info.span.start);
        // The superclass call: the temporaries of its named arguments in their written order, then the arguments
        // (dotty's `{ val b$1 = ..; val a$1 = ..; super(a$1, b$1) }`).
        self.walk_stmts(w, tc.parent_prelude);
        if let Some(args) = tc.parent_args {
            for &a in prog.expr_list(args) {
                self.walk_expr(w, a);
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
        let nested = self.nested_classes(tc);
        for (i, &k) in nested.iter().enumerate() {
            items.push((syms.class(k).span.start, 2, i));
        }
        items.sort();
        let fields_lazy: crate::intern::FxMap<SymId, ()> = if holds { self.lazy_field_syms(tc) } else { Default::default() };
        for (_, kind, i) in items {
            match kind {
                0 => match tc.init[i] {
                    TInit::Field(s, e) => {
                        self.walk_expr(w, e);
                        if fields_lazy.contains_key(&s) {
                            let name = self.field_name(s);
                            w.member(Some(c), &name, cx.input.interner.get(syms.sym(s).name), self.is_thread_unsafe(s));
                        }
                        if let SymKind::Object(k) = syms.sym(s).kind {
                            self.walk_class_names(w, k);
                        }
                    }
                    TInit::Stmt(e) => self.walk_expr(w, e),
                    // A trait's parameters: the temporaries of its named arguments, then the arguments.
                    TInit::Parent(_, call) => {
                        self.walk_stmts(w, call.prelude);
                        for &a in prog.expr_list(call.args) {
                            self.walk_expr(w, a);
                        }
                    }
                },
                1 => self.walk_fun(w, tc.methods[i], false),
                3 => self.walk_fun(w, tc.ctors[i], false),
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
            self.walk_expr(w, d);
        }
        // Its local classes the walk has not met yet, at their places.
        self.walk_local_classes_before(w, info.span.end);
        if holds {
            for (name, raw, thread_unsafe) in self.lazy_forwarder_names(tc) {
                w.member(Some(c), &name, &raw, thread_unsafe);
            }
        }
        w.unit.scope = outer_scope;
    }

    /// The member classes of the class of `tc`: its nested classes, and the objects nested in an object reached
    /// statically, which are no field and no entry of `nested` (a class's), whose module val and class stand at
    /// their place in the template all the same (MegaPhase meets its ValDef there). In their order of ids.
    pub(super) fn nested_classes(&self, tc: &TClass) -> Vec<ClassId> {
        let cx = self.cx;
        let syms = cx.input.syms;
        let c = tc.id;
        let mut nested: Vec<ClassId> = syms.class(c).nested.values().copied().collect();
        let fields: Vec<SymId> = tc.init.iter().filter_map(|i| if let TInit::Field(s, _) = *i { Some(s) } else { None }).collect();
        for s in cx.members_in_order(c) {
            if let SymKind::Object(k) = syms.sym(s).kind {
                if syms.sym(s).owner == Owner::Class(c) && !fields.contains(&s) && !nested.contains(&k) && cx.tclass_of.get(k.idx()).is_some_and(|&i| i != u32::MAX) {
                    nested.push(k);
                }
            }
        }
        nested.sort();
        nested
    }

    /// The def `f`'s body, then its default getters (`addDefaultGetters`), a by-name parameter's the value it returns,
    /// then its local classes the walk has not met; a local def's where `local`.
    pub(super) fn walk_fun<V: UnitVisitor<'a>>(&mut self, v: &mut V, f: FunId, local: bool) {
        let prog = self.cx.input.prog;
        let fun = &prog.funs[f.idx()];
        if local && !v.local_fun(self, f) {
            return;
        }
        let span = self.cx.input.syms.sym(fun.sym).span;
        let outer_scope = std::mem::replace(&mut v.unit().scope, span.start);
        if let Some(b) = fun.body {
            self.walk_expr(v, b);
        }
        if local {
            v.local_fun_end();
        }
        for (i, d) in fun.own_defaults() {
            v.default_getter(self, f, i, local);
            // The getter of a by-name parameter's default returns the value: the typer's thunk is no lambda.
            let d = match prog.expr(d) {
                TExpr::Lambda(ps, body) if ps.len == 0 && self.local_param_by_name(fun.sym, i) => body,
                _ => d,
            };
            self.walk_expr(v, d);
            v.default_getter_end();
        }
        // Its local classes the walk has not met yet (one never constructed, after the last statement).
        let end = fun.body.and_then(|b| self.expr_span(b)).map_or(span.end, |sp| sp.end.max(span.end));
        self.walk_local_classes_before(v, end);
        v.unit().scope = outer_scope;
    }

    pub(super) fn expr_span(&self, e: TExprId) -> Option<crate::source::Span> {
        self.cx.input.prog.expr_spans.get(e.0).map(|(_, sp)| sp)
    }

    /// The local classes of the definition being walked declared before `at` and not walked yet, each walked at
    /// its declaration in their order, constructed or not (MegaPhase meets the local `TypeDef` there).
    pub(super) fn walk_local_classes_before<V: UnitVisitor<'a>>(&mut self, v: &mut V, at: u32) {
        let syms = self.cx.input.syms;
        let w = v.unit();
        let due: Vec<ClassId> = w
            .locals
            .iter()
            .filter(|&&(p, k)| p >= w.scope && p < at && !w.walked.contains_key(&k) && !(V::ANON_FIRST && syms.class(k).kind == ClassKind::Anon))
            .map(|&(_, k)| k)
            .collect();
        for k in due {
            v.class(self, k);
        }
    }

    /// The statements `stmts` in their order, each after the local classes declared before it.
    pub(super) fn walk_stmts<V: UnitVisitor<'a>>(&mut self, v: &mut V, stmts: ListRef) {
        for s in self.cx.input.prog.stmts[stmts.range()].to_vec() {
            self.walk_stmt(v, s);
        }
    }

    pub(super) fn walk_stmt<V: UnitVisitor<'a>>(&mut self, v: &mut V, s: TStmt) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let syms = cx.input.syms;
        match s {
            TStmt::Expr(x) if prog.is_removed(x) => {}
            TStmt::Expr(x) => {
                if let Some(sp) = self.expr_span(x) {
                    self.walk_local_classes_before(v, sp.start);
                }
                self.walk_expr(v, x);
            }
            TStmt::Val(sym, init) => {
                self.walk_local_classes_before(v, syms.sym(sym).span.start);
                if v.val(self, Some(sym)) {
                    self.walk_expr(v, init);
                    v.val_end(self, Some(sym));
                }
            }
            TStmt::Fun(f) => {
                self.walk_local_classes_before(v, syms.sym(prog.funs[f.idx()].sym).span.start);
                self.walk_fun(v, f, true);
            }
            TStmt::Pat(p, init, _) => {
                if let Some(sp) = self.expr_span(init) {
                    self.walk_local_classes_before(v, sp.start);
                }
                if v.val(self, None) {
                    self.walk_expr(v, init);
                    v.val_end(self, None);
                }
                self.walk_pat(v, p);
            }
        }
    }

    /// The expressions a pattern holds (`pat_exprs`), in its order: a case's pattern is met before its guard and
    /// its body, a pattern definition's after its right-hand side.
    fn walk_pat<V: UnitVisitor<'a>>(&mut self, v: &mut V, p: TPatId) {
        v.pattern(self, p);
        let mut held = Vec::new();
        super::pat_exprs(self.cx.input.prog, p, &mut held);
        for e in held {
            self.walk_expr(v, e);
        }
    }

    pub(super) fn walk_expr<V: UnitVisitor<'a>>(&mut self, v: &mut V, e: TExprId) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let syms = cx.input.syms;
        v.expr(self, e);
        match prog.expr(e) {
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::TypeTest(r, _, _) | TExpr::Cast(r, ..) | TExpr::Index(r, _)
            | TExpr::Spread(r) | TExpr::JsSelect(r, _) | TExpr::Return(r) | TExpr::Throw(r, _) => self.walk_expr(v, r),
            TExpr::CallStatic(_, args) | TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args)
            | TExpr::ArrayLit(args) | TExpr::ObjLit(args) | TExpr::Default(_, _, args) => {
                for &a in prog.expr_list(args) {
                    self.walk_expr(v, a);
                }
            }
            // A `New` is its type tree alone: a named local class is met at its declaration
            // (`walk_local_classes_before`); an anonymous class, whose `TypeDef` stands where its expression does
            // (`new C { .. }` is `{ class $anon ..; new $anon }`), here. An inline expansion's copy of a named local
            // class, whose `TypeDef` the expansion's block holds where no span of the unit tells, before its first
            // construction.
            TExpr::New(c, args) => {
                if v.unit().expanded.contains_key(&c) {
                    v.class(self, c);
                }
                let anon = syms.class(c).owner == Owner::Local && syms.class(c).kind == ClassKind::Anon;
                if anon && v.anonymous(self, c, args) {
                    return;
                }
                if anon && V::ANON_FIRST {
                    v.class(self, c);
                }
                for &a in prog.expr_list(args) {
                    self.walk_expr(v, a);
                }
                if anon && !V::ANON_FIRST {
                    v.class(self, c);
                }
            }
            TExpr::NewVia(ctor, args) => {
                if let Owner::Class(c) = syms.sym(ctor).owner {
                    if v.unit().expanded.contains_key(&c) {
                        v.class(self, c);
                    }
                }
                for &a in prog.expr_list(args) {
                    self.walk_expr(v, a);
                }
            }
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.walk_expr(v, r);
                for &a in prog.expr_list(args) {
                    self.walk_expr(v, a);
                }
            }
            TExpr::Lambda(params, body) => {
                if v.lambda(self, params, body) {
                    self.walk_expr(v, body);
                    v.lambda_end();
                }
            }
            TExpr::If(c, t, els) => {
                self.walk_expr(v, c);
                self.walk_expr(v, t);
                if let Some(x) = els {
                    self.walk_expr(v, x);
                }
            }
            TExpr::While(a, b) | TExpr::Assign(a, b) | TExpr::Prim(_, a, b) => {
                self.walk_expr(v, a);
                self.walk_expr(v, b);
            }
            TExpr::Block(stmts, res) => {
                v.block(self, e);
                self.walk_stmts(v, stmts);
                if let Some(sp) = self.expr_span(res) {
                    self.walk_local_classes_before(v, sp.start);
                }
                self.walk_expr(v, res);
                if let Some(sp) = self.expr_span(e) {
                    self.walk_local_classes_before(v, sp.end);
                }
                v.block_end();
            }
            TExpr::Match(scrut, cases) => {
                self.walk_expr(v, scrut);
                for c in prog.cases[cases.range()].to_vec() {
                    self.walk_pat(v, c.pat);
                    if let Some(g) = c.guard {
                        self.walk_expr(v, g);
                    }
                    self.walk_expr(v, c.body);
                }
            }
            TExpr::Try(i) => {
                let t = prog.tries[i as usize].clone();
                self.walk_expr(v, t.body);
                for c in prog.cases[t.cases.range()].to_vec() {
                    self.walk_pat(v, c.pat);
                    if let Some(g) = c.guard {
                        self.walk_expr(v, g);
                    }
                    self.walk_expr(v, c.body);
                }
                if let Some(f) = t.finalizer {
                    self.walk_expr(v, f);
                }
            }
            _ => {}
        }
    }
}

static EMPTY_NAMES: std::sync::LazyLock<LazyNames> = std::sync::LazyLock::new(LazyNames::default);
