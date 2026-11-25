//! Code generation for expressions. An expression is emitted at the type its node leaves on the
//! stack, brought to the static type the typer recorded for it (a cast or an unboxing where a
//! generic result is used at a concrete type), and then to the type its consumer wants.

use crate::lir::{Adaptation, Capture, Comparing, ErasedType, Invoke, Kind, LiftedShape, LirId, MethodRef, Relation};
use super::classfile::*;
use super::free::{Analysis, Free};
use super::lifted::{Asker, LiftedBody, Placed};
use super::names::*;
use super::{Cx, Unit};
use crate::ast::{mods, ListRef};
use crate::classfile::ACC_STATIC;
use crate::intern::{FxMap, Name};
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::typer::loader::javaclass::JMember;
use crate::types::*;
use std::rc::Rc;

/// Where a virtual call or a field access on a qualifier points (`Gen::qualifier_site`): the
/// class the qualifier is evaluated at and the owner the instruction names, chosen per site
/// where `MethodRef` holds the declaration's.
pub struct Site {
    pub qualifier: ErasedType,
    pub owner: Rc<str>,
    pub interface: bool,
}

/// The instance a method runs on (`Gen::this_value`): an object's own, by its class's name, or
/// the one the method holds in its slot, with the type it is held at.
pub enum ThisValue {
    Module(Rc<str>),
    Slot(ErasedType),
}

/// How the value a node left is brought to the type wanted: one decision (`Gen::fit`), read
/// by the lowering.
pub enum Fit {
    /// A value class's value, its box made or opened (`Gen::lower_value_class`), a null tested
    /// for where the flag is set.
    ValueClass(ClassId, TList, bool),
    /// Through the type the typer recorded, where it knows more than the erased signature.
    Through(ErasedType),
    Direct,
}

/// What bringing a value class's value to a type takes (`Gen::vc_form`).
pub(super) enum VcForm {
    /// The value adapted as it is: dropped, a box kept, the underlying value where it is wanted.
    Adapt,
    /// The underlying value, of the class's erasure, boxed.
    Box(ErasedType),
    /// The box opened, after a null test where the flag is set.
    Open { null_safe: bool },
}

/// The way a member read `r.s` is generated: one decision (`Gen::field_route`), read by the
/// lowering.
pub enum FieldRoute {
    /// A member of a JavaScript type, which the JVM has none of.
    Js,
    /// A Java class's field.
    Java,
    /// An object selected through its owner: its instance.
    Object(ClassId),
    /// A given object through its owner, which is not initialised for it, as under scalac: a
    /// receiver that is no object's instance nor `this` is evaluated for its effect alone.
    GivenObject { receiver_evaluated: bool },
    /// A value class's parameter on a value of it: the value itself.
    Underlying(ClassId),
    /// A constructor's parameter read before the superclass's constructor ran: its local.
    Early,
    /// A field of this class's own instance (`Gen::own_field`).
    Own,
    /// A parameter of no accessor, read by its class's own code alone (`Cx::private_params`).
    PrivateParam(ClassId),
    /// The member's accessor.
    Accessor,
}

/// The way a read of `s`, a member with no receiver written, is lowered (`Gen::static_route`).
pub enum StaticRoute {
    /// A given object: its instance at the given's declared type.
    GivenObject,
    /// A Java class's static field.
    JavaField,
    /// An enum's value held in a static field of its companion: the companion, and the enum.
    EnumField(ClassId, ClassId),
    /// An enum's value held by its own class, its enum initialised first.
    EnumValue(ClassId),
    /// A top-level inline val: its constant.
    InlineVal,
    /// A static field of a class of the program.
    Field,
    /// The accessor on the member's object, or on its file's.
    Accessor,
}

/// The way a call `s(args)` with no receiver written is generated: one decision
/// (`Gen::static_call_route`), read by the lowering.
pub enum StaticCallRoute {
    /// The typer's conversion of a spread array to a sequence: the array, wrapped.
    Spread(TExprId),
    /// A static method of a Java class.
    JavaStatic,
    /// A member of a std object standing for a JDK class's statics.
    JdkStatic,
    /// A local function value applied through its name.
    LocalValue,
    /// A local definition: its lifted method, what it captures first.
    Lifted,
    /// A member of a file's object.
    FileModule,
}

/// How the arguments of a constructor are passed: one decision (`Gen::ctor_arguments_route`),
/// read through `Gen::new_route` and directly by the lowering.
pub enum CtorArguments {
    /// What the class captures first, an anonymous class's superclass's after.
    Captured,
    /// As the class file's constructor of a class this build does not write takes them: the
    /// type of the enclosing instance where it takes one, and its parameters' types
    /// (`Gen::unwritten_ctor_layout`).
    Unwritten(Option<ErasedType>, Vec<ErasedType>),
    /// The enclosing instance, at this type, before the declared ones, where the class file of
    /// a class this build does not write gives no one answer.
    UnwrittenOuter(ErasedType),
    /// At the declared parameters' types.
    Declared,
}

/// The way `new C(args)` is generated: one decision (`Gen::new_route`), read by the lowering.
pub enum NewRoute {
    /// A value class's value: its argument, at this type.
    Underlying(ErasedType, TExprId),
    /// A class of JavaScript, which the JVM has none of.
    Js,
    /// A new instance and its constructor's call.
    Instance(CtorArguments),
}

/// How the arguments of a secondary constructor are passed: one decision
/// (`Gen::via_arguments_route`), read by the lowering.
pub enum ViaArguments {
    /// As `CtorArguments::Unwritten`.
    Unwritten(Option<ErasedType>, Vec<ErasedType>),
    /// What a local class captures, the leading arguments of this count, before the declared
    /// ones.
    Captured(usize),
    /// The enclosing instance of a class nested in a class, at this type, before the declared
    /// ones.
    Outer(ErasedType),
    /// At the declared parameters' types.
    Declared,
}

/// The way `new C(args)` through a secondary constructor is generated: one decision
/// (`Gen::via_route`), read by the lowering.
pub enum ViaRoute {
    /// A constructor of no class.
    Unowned,
    /// A class of JavaScript, which the JVM has none of.
    Js,
    /// A new instance of the class and the constructor's call.
    Instance(ClassId, ViaArguments),
}

/// How the creation of an anonymous class passes its superclass's arguments, after what the
/// class captures: one decision (`Gen::anon_super_route`), read by the lowering.
pub(super) enum SuperArguments {
    /// As a creation of the superclass, a class that captures, passes its primary constructor's.
    Captured,
    /// As a creation through this secondary constructor of the superclass, a local class,
    /// passes them.
    Via(SymId),
    /// As `CtorArguments::Unwritten`.
    Unwritten(Option<ErasedType>, Vec<ErasedType>),
    /// At the parameters' types of the constructor selected.
    Declared,
}

/// One of the values a creation passes a lifted class for what it captures
/// (`Gen::passed_captures`).
pub(super) enum PassedCapture {
    /// The creation's argument for this parameter of the class's constructor.
    Argument(TExprId, SymId),
    /// What the scope holds for a local that a local def the class copies captures.
    Held(SymId),
}

/// How the argument that hands a lifted class or a local trait's lifted member a local is passed:
/// one decision (`Gen::capture_argument_route`), read by the lowering (`Gen::lower_capture_argument`).
pub(super) enum CaptureArgument {
    /// The argument's value at this type, where the class takes a value and the creation reads
    /// another local.
    Value(ErasedType),
    /// What the scope holds for the local: its value, its cell, a lazy val's holder.
    Held(SymId),
    /// A holder of the lazy local `lazy` made for the value this expression gives (a local
    /// object's own val, passed as `() => this`): made, filled and then passed.
    Holder { lazy: SymId, value: TExprId },
    /// The argument at its own type, the unit value for none.
    Own,
}

/// Where the default of a constructor's parameter comes from: one decision
/// (`Gen::ctor_default_route`), read by the lowering (`Gen::lower_ctor_default`).
pub(super) enum CtorDefault {
    /// A class of the JDK, which has no getters: the default's expression where the creation
    /// stands, nothing for a parameter that has none.
    InPlace(Option<TExprId>),
    /// The getter of a local class's local companion, on the receiver the typer selected it
    /// on, or on the companion's val, this local.
    LocalCompanion(SymId, MethodRef),
    /// The getter of the companion of a class nested in a class, an object of each enclosing
    /// instance: the companion read by this accessor on the creation's enclosing instance.
    InnerCompanion(MethodRef, MethodRef),
    /// A named local class's static getter, which takes what the class captures, parameters of
    /// this count, before the arguments.
    Captured(usize, MethodRef),
    /// The getter of the class's companion, on the companion.
    Companion(MethodRef),
    /// A jar class's getter as its class files declare it: its companion's, or the static
    /// forwarder of the class.
    Jar(MethodRef),
    /// The class's static getter.
    Static(MethodRef),
}

impl CtorDefault {
    /// The getter the route calls; none for a class of the JDK's default written in place.
    fn getter(&self) -> Option<&MethodRef> {
        match self {
            CtorDefault::InPlace(_) => None,
            CtorDefault::LocalCompanion(_, getter)
            | CtorDefault::InnerCompanion(_, getter)
            | CtorDefault::Captured(_, getter)
            | CtorDefault::Companion(getter)
            | CtorDefault::Jar(getter)
            | CtorDefault::Static(getter) => Some(getter),
        }
    }
}

/// An argument of a default's getter: the expression, the type it is written at (its
/// parameter's in the method's signature), and the type the getter's parameter takes.
pub(super) struct GetterArgument {
    pub expr: TExprId,
    pub ty: ErasedType,
    pub param: ErasedType,
}

/// The way a default argument is generated: one decision (`Gen::default_route`), read by the
/// lowering.
pub(super) enum DefaultRoute {
    /// A constructor's default, the receiver the typer selected the getter on where it selected
    /// one: where it comes from, the arguments before it (`GetterArgument`s, each at the type the
    /// route's getter takes it), and the parameter's type.
    Ctor(ClassId, Option<TExprId>, CtorDefault, Vec<GetterArgument>, ErasedType),
    /// The getter of a file's member on the file's object, over the arguments before the default
    /// (`GetterArgument`s), and the type the default is left as.
    FileMember(FileId, MethodRef, Vec<GetterArgument>, ErasedType),
    /// The getter of a method's default on the receiver the typer selected it on, over the
    /// arguments before the default: the receiver at the type it is read as, the getter, the
    /// class the call names where that is the receiver's (`Gen::qualifier_site`), with whether it
    /// is an interface, and the type the default is left as.
    Method { receiver: TExprId, receiver_type: ErasedType, getter: MethodRef, at: Option<(Rc<str>, bool)>, args: Vec<GetterArgument>, want: ErasedType },
    /// The getter of a default of a value class's method, called as `Gen::lower_extension_call`
    /// calls the method (dotty's `ExtensionMethods.extensionMethod`, `VCInlineMethods`): the
    /// companion's `m$default$N$extension` over the receiver's underlying value, at the type it
    /// is read as, and the arguments before the default, and the type the default is left as.
    Extension { receiver: TExprId, underlying: ErasedType, getter: MethodRef, args: Vec<GetterArgument>, want: ErasedType },
    /// The route where an operand of the getter is no path (`Gen::default_operands_are_paths`): a
    /// method's receiver, then the arguments before the default, each at its type, are evaluated in
    /// order into temporaries before the getter is called over them, so that one evaluated for its
    /// effect is evaluated once and in its place (a constructor's receiver is evaluated where the
    /// route has it, after the arguments).
    Held(Vec<(TExprId, ErasedType)>, Box<DefaultRoute>),
    /// The default `i` of a local def, its getter lifted with the def's captures (`Gen::lower_local_default`).
    LocalDef(SymId, usize),
    /// A default refused where it stands (`Kind::Refused`).
    Refused(crate::lir::Refusal),
}

/// The way a call `r.s(args)` is generated: one decision (`Gen::call_route`), read by the
/// lowering.
pub enum CallRoute {
    /// The typer's conversion of a spread array to a sequence: the array, wrapped.
    Spread(TExprId),
    /// A member of a JavaScript type, which the JVM has none of.
    Js,
    /// A value class's method, called as scalac calls it (`Gen::lower_extension_call`).
    Extension(ClassId, ExtensionRoute),
    /// An interpolator of scala-library that has no bytecode: the runtime's helper that stands for
    /// it (`jvm::interpolators`).
    Interpolator(SymId),
    /// An object selected through its owner: its instance, not a call.
    Object(ClassId),
    /// A static method of a Java class.
    JavaStatic,
    /// A member of a std object standing for a JDK class's statics.
    JdkStatic,
    /// `super[T].s(..)` of a member of a local trait, the proxies first (`Gen::member_proxies`).
    Proxied(ClassId),
    /// A local trait's lifted member, the proxies first (`Gen::member_proxies`).
    Lifted,
    /// `super.s(..)`.
    Super,
    /// The method on its receiver.
    Plain,
}

/// How a method of the value class it names is called: one decision (`Gen::extension_route`),
/// read by `Gen::call_route` and `Gen::lower_extension_call`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExtensionRoute {
    /// The companion's `m$extension`, where this build writes the companion.
    Companion,
    /// The class's own static `m$extension`, for a class of the program with no companion written.
    Static,
    /// The method of a jar's companion that stands for it (`Gen::jar_extension`).
    Jar,
}

/// The member a reference names, as dotty's refusal of an unwritable reference shows it
/// (`Gen::unwritable_reference`): a symbol, or a std template's method by its name.
#[derive(Clone, Copy)]
pub enum Referenced<'s> {
    Sym(SymId),
    Method(&'s str),
}

#[derive(Clone)]
pub struct Local {
    pub slot: u16,
    /// The declared type; a cell holds a value of this type as an `Object`.
    pub ty: ErasedType,
    pub cell: bool,
}

/// Where a scope keeps what it holds for a local (`Gen::held_in`), with the type it is held at.
pub(super) enum HeldIn {
    /// A local of the method.
    Local(ErasedType),
    /// A field of the instance: a local class's capture.
    Field(String, ErasedType),
}

/// How an anonymous class holds a local of the scope it stands in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Held {
    /// The value; a local lazy val's is its holder (`LazyVals.transformLocalDef`).
    Value,
    /// A `var`, shared with its scope through its cell.
    Cell,
}

/// What lifted code takes beyond the locals it names, as `LambdaLift` makes them its free variables:
/// the instance of each class whose code declares a local def the code copies or a local lazy val whose
/// methods it calls (`Gen::lazy_method`), where that class is not the code's own and its instance is no
/// module reached statically (`h.this`, the outer instance dotty reaches them through), and the key of
/// the non-local returns of the method declaring each lazy val whose initialiser returns
/// (`NonLocalReturns.nonLocalReturnKey`), which travels with the val's holder.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Extras {
    pub homes: Vec<ClassId>,
    pub keys: Vec<SymId>,
}

/// A static method made from a lambda, a local def or the initialiser of a local lazy val, as the
/// class file has it: what it takes and leaves (`shape`: `$this` where the body needs it, the key
/// of the def its non-local returns leave, the captures, the parameters), and the name the writer
/// gave it.
pub struct Lifted {
    pub name: String,
    pub shape: Rc<LiftedShape>,
    /// The shape's signature as a class file spells it.
    pub desc: String,
    /// Whether it takes `this`: the shape's `instance` is some.
    pub this: bool,
    /// The writer's, as `LiftedBody` has them: the local each capture is in the body.
    pub captures: Vec<Capture>,
    /// Where `this` is the instance of a class other than the one being written: a class's copy of a
    /// local def or lazy val declared in that class's code (`Gen::copy_home`), and what it lifts.
    pub home: Option<ClassId>,
    /// Where `this` is a module read statically, its class, taking no `this` (`MethodCx::this_module`):
    /// what a copy whose `this` is that module lifts.
    pub module: Option<Rc<str>>,
    /// Its place among the class's lifted methods (`lifted::Placed`), none for a jar default's thunk (teq's own).
    pub place: Option<u32>,
}

impl std::ops::Deref for Lifted {
    type Target = LiftedShape;

    fn deref(&self) -> &LiftedShape {
        &self.shape
    }
}

impl Lifted {
    pub(super) fn new(name: String, shape: Rc<LiftedShape>, captures: Vec<Capture>, home: Option<ClassId>, module: Option<Rc<str>>, place: Option<u32>) -> Lifted {
        let desc = method_desc(&shape.takes(), &shape.ret);
        Lifted { name, desc, this: shape.instance.is_some(), shape, captures, home, module, place }
    }
}

pub enum PendingBody {
    /// The parameters arrive as `Object`s and are unboxed into locals of their declared types.
    Lambda(Vec<SymId>, TExprId),
    Fun(FunId),
    /// The default of a local def's own parameter `n` (`TFun::own_default`).
    Default(FunId, usize),
    /// A local lazy val's accessor, `if x$lzy.initialized() then x$lzy.value() else x$lzyINIT()`,
    /// its holder the last parameter (`LazyVals.transformLocalDef`).
    LazyAccessor(SymId),
    /// A local lazy val's initialiser over its holder's monitor, the holder the last parameter.
    LazyInit(SymId, TExprId),
}

/// The getter of a default as scalac writes it (`Gen::default_getter_shape`).
pub struct GetterShape {
    /// The number in its name, `m$default$N`.
    pub index: usize,
    /// How many of the method's parameters, in the class file's order, it takes.
    pub before: usize,
    pub ret: ErasedType,
    pub by_name: bool,
}

/// An argument of a default's getter: the expression written where the getter takes it, at its type.
#[derive(Clone)]
pub struct Prior {
    pub expr: TExprId,
    pub ty: ErasedType,
}

impl DefaultRoute {
    /// The route, or the route over the arguments `priors` held in temporaries where its operands
    /// are not all paths.
    fn held_unless_paths(self, paths: bool, priors: &[Prior]) -> DefaultRoute {
        if paths {
            return self;
        }
        let held = priors.iter().map(|p| (p.expr, p.ty.clone()));
        DefaultRoute::Held(held.collect(), Box::new(self))
    }
}

pub struct Pending {
    pub lifted: Rc<Lifted>,
    pub body: PendingBody,
    /// Where the body's `return`s leave the def around it by a throw (`MethodCx::return_target`).
    pub return_target: ReturnTarget,
}

/// The def a closure's `return`s leave, at whose erased result they throw (`Gen::lower_non_local_return`).
#[derive(Clone, Default)]
pub enum ReturnTarget {
    Def(ErasedType),
    /// The closure stands in code no def holds (a parameter's default, a field's initialiser), where scalac refuses a
    /// `return`: teq's typer refuses it, or the method reading the closure that makes no key (`Gen::load_lazy_key`,
    /// `load_return_key`).
    NoDef,
    /// A closure the walk of the program's code did not meet (`Cx::return_def`).
    #[default]
    Unknown,
}

/// A def whose self calls in tail position are jumps to the start of its code, as the lowering decides it: the
/// writer binds the start (`MethodCx::tail_head`).
pub struct TailLoop {
    pub sym: SymId,
    pub params: Vec<SymId>,
    /// The calls of the def by itself under its `return`s that go round the loop
    /// (`Program::tail_jumps`).
    pub jumps: Vec<TExprId>,
}

pub struct MethodCx {
    pub locals: FxMap<SymId, Local>,
    /// The declared parameters in order, each with its slot.
    pub params: Vec<(u16, ErasedType)>,
    pub this_slot: Option<u16>,
    /// In an object's `<clinit>`, the temporary its head keeps the instance in: `this` while the method's tree is
    /// lowered, before the writer gives the temporary the slot `this_slot` then names (`Gen::hold_module_part`). The
    /// lowering asks of the two through `MethodCx::has_instance` and `MethodCx::instance_is_first_local` alone.
    pub this_kept: Option<crate::lir::TempId>,
    pub ret: ErasedType,
    pub tail: Option<TailLoop>,
    /// Where the code of a def that loops (`tail`) starts.
    pub tail_head: Option<Label>,
    /// The locals of the method that live in a cell (`free::cells`).
    pub cells: Vec<SymId>,
    /// Set while the arguments of the super constructor are evaluated: `this` is not there
    /// yet, and a field of it named there is the constructor parameter it will be set from.
    pub before_super: bool,
    /// The method is a lifted lambda, so a `return` in it would leave the def around it.
    pub lambda: bool,
    /// The method is a lifted lambda or a local lazy val's initialiser: a `return` in it throws
    /// `NonLocalReturnControl` with the key of the def it leaves (dotty's `NonLocalReturns`).
    pub closure: bool,
    /// The slot of that key: the def's, which it makes, or the closure's capture of it.
    pub return_key: Option<u16>,
    /// In a closure, the def its `return`s leave.
    pub return_target: ReturnTarget,
    /// In a value class's `m$extension`, `this` is the underlying value of this type.
    pub this_underlying: Option<ErasedType>,
    /// In a class's copy of a local lazy val's accessor or initialiser or of a local def that stands in a
    /// method of an object (or of a file's top level), `this` is that object's `MODULE$`
    /// (`Gen::copy_home_module`).
    pub this_module: Option<Rc<str>>,
    /// In a class's copy of a local def or lazy val declared in another class's code, `this` is that
    /// class's instance (`Lifted::home`).
    pub this_home: Option<ClassId>,
    /// The slots holding the keys that travel with local lazy vals' holders (`Extras::keys`).
    pub lazy_keys: FxMap<SymId, u16>,
    /// The slots holding the instances of other classes (`Extras::homes`) where no field holds them yet:
    /// a constructor's and a local trait's member's parameters.
    pub home_slots: Vec<(ClassId, u16)>,
    /// The `try` blocks with a finalizer and the `synchronized` blocks the code being generated
    /// stands in, innermost last: a `return` leaves through each one's finalizer or monitor exit
    /// (`Gen::write_try`, `Gen::monitor_block`).
    pub finalizers: Vec<FinallyFrame>,
    /// The copies of finalizers being written, outermost first (`Gen::write_finalizer`).
    pub finally_copies: Vec<FinallyCopy>,
    /// The exits of those frames a `return` has jumped to, as dotty's `shouldEmitCleanup` records
    /// that a cleanup block is wanted (BCodeBodyBuilder `genReturn`).
    pub exits_taken: Vec<Label>,
    /// The slot a `return` through those frames keeps the method's value in until the outermost
    /// cleanup returns it (dotty's `earlyReturnVar`), made by the outermost frame whose code returns
    /// (`Gen::early_return_for`).
    pub early_return: Option<u16>,
    /// The slots that hold the lowering's temporaries.
    pub temps: FxMap<crate::lir::TempId, u16>,
    /// The nodes of the null tests that open a value class's box (`Gen::lower_vc_open_null_safe`),
    /// whose `If` takes a return into its branches (`Gen::returned`).
    pub null_tests: Vec<LirId>,
    /// The temporaries whose slot a `Kind::Reserve` took and whose `Let` is still to come.
    pub reserved: FxMap<crate::lir::TempId, Reserved>,
    /// A temporary whose value is on the operand stack for its next read, which writes nothing
    /// (`Gen::kept_on_stack`).
    pub stacked: Option<crate::lir::TempId>,
    /// The ends of the lowering's labelled blocks being written (`Kind::Labeled`).
    pub labels: FxMap<crate::lir::LabelId, Label>,
    /// The `try`s whose handlers are being written, by the label their bodies leave by: the try's
    /// normal exit, which the value is carried to on the operand stack.
    pub try_exits: FxMap<crate::lir::LabelId, Label>,
    /// Where the method is defined, where the class writer knows it: the place of the refusals of its code
    /// (`Gen::end_method`), given with its context (`Gen::method_cx`).
    pub at: Option<(crate::source::FileId, crate::source::Span)>,
}

/// The state of a `try` that `Gen::open_try` began, for `Gen::close_try`.
pub struct TryOpen {
    outer: usize,
    stash: Vec<(u16, ErasedType, VT)>,
    kept: Option<Reserved>,
    made_early: bool,
    throwable_vt: VT,
    start: Label,
    end: Label,
    normal_exit: Label,
    cleanup: Label,
    protected: usize,
    from: u32,
}

/// A `try` with a finalizer or a `synchronized` block around the code being generated: a `return`
/// inside keeps its value in `MethodCx::early_return` and jumps to `cleanup`, the finalizer's or the
/// monitor exit's copy written for it, which returns or goes on to the enclosing frame's cleanup
/// (dotty's `registerCleanup` and `pendingCleanups`, BCodeSyncAndTry.scala 426-440).
#[derive(Clone, Copy)]
pub struct FinallyFrame {
    pub cleanup: Label,
}

/// One copy of a finalizer being written (`Gen::write_finalizer`): what its places are among those of every copy.
pub struct FinallyCopy {
    /// The `try` whose finalizer it is, the same in every copy of the finalizers around it (`Gen::finalizer_of`).
    pub of: u32,
    /// The closures the copy has met, by their entries in the class's table, each its place among its typed body's in
    /// the finalizer.
    pub lambdas: FxMap<Placed, u32>,
    /// How many places of each lambda's typed body the copy has met.
    pub places: FxMap<TExprId, u32>,
    /// How many `try` blocks with a finalizer the copy has written.
    pub tries: u32,
}

impl Default for MethodCx {
    fn default() -> Self {
        MethodCx {
            locals: FxMap::default(),
            params: Vec::new(),
            this_slot: None,
            this_kept: None,
            ret: ErasedType::V,
            tail: None,
            tail_head: None,
            cells: Vec::new(),
            before_super: false,
            lambda: false,
            closure: false,
            return_key: None,
            return_target: ReturnTarget::Unknown,
            this_underlying: None,
            this_module: None,
            this_home: None,
            lazy_keys: FxMap::default(),
            home_slots: Vec::new(),
            finalizers: Vec::new(),
            finally_copies: Vec::new(),
            exits_taken: Vec::new(),
            early_return: None,
            temps: FxMap::default(),
            null_tests: Vec::new(),
            reserved: FxMap::default(),
            stacked: None,
            labels: FxMap::default(),
            try_exits: FxMap::default(),
            at: None,
        }
    }
}

impl MethodCx {
    /// Whether the method runs on an instance `this` reads: one in a slot, or the one an object's `<clinit>` keeps
    /// (`this_kept`).
    pub fn has_instance(&self) -> bool {
        self.this_slot.is_some() || self.this_kept.is_some()
    }

    /// Whether the instance is the method's first local, as `this` is in an instance method. The one an object's
    /// `<clinit>` keeps is too: the `<clinit>` is static and takes no parameter, and its head, written first
    /// (`Gen::write_held`), declares no temporary before the one it keeps the instance in (`Gen::lower_module_head`).
    /// A temporary the head declared before that one would move it.
    pub fn instance_is_first_local(&self) -> bool {
        self.this_slot == Some(0) || self.this_kept.is_some()
    }
}

/// Where a class first names a super call's target: the kind of body, then the place in it
/// (`Gen::super_call_place`); the class's list takes its targets in this order.
pub type SuperPlace = (u8, u32, u32, u32);

/// A field of the class being written that its held members (the plan's phase) and its writes (`Gen::write_held` and
/// `Gen::finish_writer`, the write phase) both touch. While the two phases alternate, a checks build refuses each pair
/// writing the class from its whole plan (every hold first, then every write) would invert: a hold reading what an
/// earlier write of the class wrote, and a hold writing what an earlier write of the class read. A release build reads
/// and writes the field alone.
pub struct Phased<T> {
    value: T,
    field: PhasedField,
}

#[derive(Clone, Copy, Debug)]
pub enum PhasedField {
    SuperTargets,
    LazyHolders,
    CheckedMethods,
    LongNameRefused,
    ModuleTree,
    ClassHomes,
    ClassKeys,
}

thread_local! {
    /// The write phase's depth, and the fields the class's writes have written and read so far, by bit.
    static PHASE: std::cell::Cell<(u32, u32, u32)> = const { std::cell::Cell::new((0, 0, 0)) };
}

impl<T> Phased<T> {
    pub fn new(field: PhasedField, value: T) -> Phased<T> {
        Phased { value, field }
    }

    fn note(&self, write: bool) {
        if !cfg!(debug_assertions) {
            return;
        }
        let bit = 1u32 << self.field as u32;
        PHASE.with(|p| {
            let (depth, written, read) = p.get();
            if depth > 0 {
                p.set((depth, if write { written | bit } else { written }, if write { read } else { read | bit }));
            } else if !write && written & bit != 0 {
                panic!("a held member reads {:?}, which a write of its class wrote before it", self.field);
            } else if write && read & bit != 0 {
                panic!("a held member writes {:?}, which a write of its class read before it", self.field);
            }
        });
    }
}

impl<T> Phased<T> {
    /// The field as `Gen::emit_class`'s tail takes it, after every write of the class: the super calls' targets the
    /// writes recorded, which the class's interfaces take. Not checked: that tail is the class's end step, which reads
    /// what every write of the class gave.
    pub fn at_class_end(&mut self) -> &mut T {
        &mut self.value
    }
}

/// Whether a write of the class being written runs (`write_phase`), in checks builds; a release build answers false.
/// Once the class's walk has started, no write makes a lifted method's first ask or queues a body: each is plan work
/// made after the plan is whole.
pub fn writing() -> bool {
    cfg!(debug_assertions) && PHASE.with(|p| p.get().0 > 0)
}

impl<T> std::ops::Deref for Phased<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.note(false);
        &self.value
    }
}

impl<T> std::ops::DerefMut for Phased<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.note(true);
        &mut self.value
    }
}

/// The class's writes begin (`entering` true) or end; the class's checks start again (`Gen::begin_class`,
/// `Gen::reset_class`) where neither is given.
pub fn write_phase(entering: Option<bool>) {
    PHASE.with(|p| {
        let (depth, written, read) = p.get();
        p.set(match entering {
            Some(true) => (depth + 1, written, read),
            Some(false) => (depth - 1, written, read),
            None => (depth, 0, 0),
        });
    });
}

pub struct Gen<'a> {
    pub cx: &'a Cx<'a>,
    pub errors: Vec<String>,
    /// The program's references this target cannot write, refused at their place as dotty's
    /// erasure refuses them (`refuse`).
    pub refused: Vec<super::Refusal>,
    erased: Vec<Option<ErasedType>>,
    /// Whether the erasures are kept in `erased_overlay` alone: for a Gen that emits a few units
    /// (a session's retype), whose types are the store's last ids, the table by id would be made
    /// for every type before them.
    pub sparse: bool,
    /// The erasures of the types with an overlay's id (`arena::LOCAL_BASE` on), which the parallel
    /// typer's overlays keep past the merge: a table by the id would
    /// take gigabytes per class. A sparse Gen keeps every erasure here.
    erased_overlay: FxMap<TypeId, ErasedType>,
    /// Whether the types erased are a Java member's signature's, dotty's `TypeErasure` of
    /// `SourceLanguage.Java`: `Array[T]` of a type parameter is `T[]`, never `Object`, the
    /// parameter being `JavaDefined` (`isGenericArrayElement`). Its erasures are `erased_java`.
    java_erasure: bool,
    erased_java: FxMap<TypeId, ErasedType>,
    /// Whether the types erased are a signature's (a result, a value's or a parameter's type, a
    /// bound): `Nothing` and `Null` erase to the runtime's `Nothing$` and `Null$` through the whole
    /// erasure, a union's lub, an intersection's glb, a refinement's parent and an abstract member's
    /// bound included, as dotty's erasure keeps the bottom classes and its backend maps them
    /// (`CoreBTypesFromSymbols`); elsewhere they erase to `Object`. Its erasures are `erased_bottoms`.
    bottom_markers: bool,
    erased_bottoms: FxMap<TypeId, ErasedType>,
    /// The declared views met (`MethodSig::declared`), expanded (`declared_type`).
    declared: std::cell::RefCell<FxMap<TypeId, TypeId>>,
    class_rc: Vec<Option<Rc<str>>>,
    mrefs: FxMap<SymId, Rc<MethodRef>>,
    /// The function arities past 1 an erasure or a method's reference used when it was made,
    /// added again when the memo answers it, so that a unit's `function_arities` do not depend on
    /// the units the Gen emitted before it (a session keeps them per unit, `kept.rs`).
    erased_arities: FxMap<TypeId, u64>,
    mref_arities: FxMap<SymId, u64>,
    /// The fields of the traits read from class files, by trait (`class_file_trait_fields`).
    pub(super) trait_fields: FxMap<ClassId, Rc<super::classes::ClassFileTrait>>,
    pub cw: ClassWriter,
    /// The class being written as its definitions (`plan.rs`).
    pub plan: crate::lir::ClassDef,
    /// The class's lifted methods by their unit's names and places, beside `plan` (`lifted::LiftedTable`).
    pub(super) lifted_table: super::lifted::LiftedTable,
    /// The interfaces the class being written names in an `invokespecial`, each with the place
    /// of its first call (`super_call_place`): the JVM wants each among the class's direct
    /// superinterfaces, so the class's list takes them (dotty's `superCallTargets`). It is the
    /// writer's: emptied where a class's writer is made, read at the class's end.
    pub super_targets: Phased<Vec<(String, SuperPlace)>>,
    /// The end of the super call being written in the source, where its text is the class's
    /// file's: an `invokespecial` is written after its call's arguments.
    pub super_call_at: Option<u32>,
    /// The end and the start of the lifted body being written (a lambda, a by-name argument, a
    /// local def), none in a member's own code.
    pub lifted_body: Option<(u32, u32)>,
    /// Set while the primary constructor is written.
    pub ctor_code: bool,
    pub this_name: Rc<str>,
    pub this_class: Option<ClassId>,
    pub is_interface: bool,
    pub abstract_class: bool,
    /// The source file of the class being written, whose lines the line table counts.
    pub line_file: FileId,
    /// The call whose arguments are being written, with its last offset (`mark_line`).
    pub line_floor: Option<(TExprId, FileId, Span, u32)>,
    /// Inside an argument the typer made, whose line holds through it (`mark_line`).
    pub line_hold: bool,
    pub pending: Vec<Pending>,
    /// The bodies a hold asks for while a round of the class's lifted bodies is lowered (`Gen::drain_pending`): they
    /// belong to the next round, as the writer's deferral of them did, so that no round's set changes while it is popped.
    pub(super) pending_next: Vec<Pending>,
    /// Whether a round of `Gen::drain_pending` is lowering its bodies.
    pub(super) draining: bool,
    /// Checked by its key, as `lifted_table` is: a write that misses it panics (`Gen::lifted_fun`, `Gen::lazy_method`).
    pub lifted: FxMap<SymId, Rc<Lifted>>,
    /// The initialiser of each local lazy val the class's code reads (`x$lzyINITN`), beside its
    /// accessor in `lifted` (`LazyVals.transformLocalDef`).
    pub lifted_lazy_inits: FxMap<SymId, Rc<Lifted>>,
    /// The thread-safe lazy members of the class being written, in the order they were met
    /// (`Gen::lazy_holder`).
    pub lazy_holders: Phased<Vec<super::lazy::LazyHolder>>,
    pub lifted_defaults: FxMap<(SymId, usize), Rc<Lifted>>,
    pub an: Analysis,
    /// The lifted methods of the closures of the class's code, by their entries in the class's table
    /// (`Gen::lambda_method`).
    pub lambdas: FxMap<Placed, Rc<Lifted>>,
    /// The lifted methods of the lambdas of finalizers, by their typed body, the `try` whose finalizer holds them
    /// and their place among the body's in it: one method for every copy (`Gen::write_finalizer`).
    pub finally_lambdas: FxMap<(TExprId, u32, u32), Rc<Lifted>>,
    /// The `try` blocks with a finalizer the class's code has written, each by the one whose finalizer holds it and
    /// its place among that finalizer's (`Gen::finalizer_of`).
    pub finally_tries: FxMap<(u32, u32), u32>,
    /// How many `try` blocks with a finalizer the class's code has numbered (`Gen::finalizer_of`).
    pub tries_written: u32,
    /// The locals an anonymous class holds in fields: symbol, field name, type, and how.
    pub class_captures: Vec<(SymId, String, ErasedType, Held)>,
    /// The fields of the class being written holding its `Extras`: the other classes' instances and
    /// the keys of the lazy vals.
    pub class_homes: Phased<Vec<(ClassId, String)>>,
    pub class_keys: Phased<Vec<(SymId, String)>>,
    pub code: Code,
    /// The lowered nodes of the method being generated (`crate::lir`).
    pub lir: crate::lir::Body,
    /// The locals a lowering declared and the writer has not met yet, each with its type and
    /// whether it is kept in a cell: a later statement of the block is lowered before the
    /// declaration is written.
    pub(super) lower_declared: FxMap<SymId, (ErasedType, bool)>,
    /// The local defs the lowering running met, queued when it ends (`Gen::queue_local_defs`).
    pub(super) local_defs: Vec<SymId>,
    /// The templates read, by their text (`Gen::template`).
    pub(super) templates: FxMap<Box<str>, Rc<super::template::Template>>,
    /// The constructor tree of the object being written, between its two methods
    /// (`Gen::hold_module_part`).
    pub(super) module_tree: Phased<Option<super::classes::ModuleTree>>,
    /// The temporaries of the matches being lowered that hold a value class's primitive
    /// underlying value, with the class and its arguments: a type pattern tests, and a
    /// `MatchError` holds, its box.
    pub(super) vc_temps: FxMap<crate::lir::TempId, (ClassId, TList)>,
    pub m: MethodCx,
    pub context: String,
    /// The arities of the function interfaces the emitted code names, as a bit set.
    pub function_arities: u64,
    /// Classes a unit writes beside its own: the module class of a class whose static
    /// forwarders need its method table, a mirror class of static forwarders.
    pub extra_classes: Vec<(String, Vec<u8>)>,
    /// In link mode, the method table of the module class written last, and the module class
    /// whose static forwarders the class being written takes.
    pub module_table: Vec<crate::lir::MethodDef>,
    pub keep_table: bool,
    /// The bridges of the class being written that its inherited pairs made (`pair_bridges`), by name and
    /// descriptor: a module class's are left out of the table its static forwarders are made from, where
    /// an inherited method of that descriptor with a body takes a forwarder as scalac's has one.
    pub pair_bridged: Vec<(String, String)>,
    pub forward_from: Option<(Rc<str>, Vec<crate::lir::MethodDef>)>,
    /// The unit is written again because a jump of one of its methods did not reach in 16 bits
    /// (`Code::relax`).
    pub relax: bool,
    pub overflow: bool,
    /// A class of the unit being written passed the constant pool's bound (`Gen::class_too_large`):
    /// wide jumps would not shrink it.
    pub pool_too_large: bool,
    /// The class being written is a module's mirror (`Gen::mirror`), whose names scalac checks as
    /// a mirror's.
    pub writing_mirror: bool,
    /// The class being written has a name or a method's signature past a text constant's bytes,
    /// refused where dotty refuses it (`Gen::name_too_long`).
    pub long_name_refused: Phased<bool>,
    /// The name and descriptor of each method of the class that passed `Gen::check_method`, the one gate of a
    /// method of the class file: `Gen::finish_writer` writes no class that holds another.
    pub checked_methods: Phased<FxMap<(Rc<str>, String), ()>>,
    /// The static forwarders of a module's methods are being written (`Gen::static_forwarders`), which
    /// dotty adds to a class file apart from the methods it checks.
    pub writing_forwarders: bool,
    /// The first method of each class of the unit whose code passed what a jump of the pass reaches
    /// (`Code::finish`): its class, its name, its descriptor and its definition where the writer knows it.
    pub too_large_methods: Vec<(Rc<str>, String, String, Option<(crate::source::FileId, crate::source::Span)>)>,
    /// The `Object` a value class over a type parameter erases to in link mode, told apart by
    /// identity from the `Object` of a generic slot, where the value is boxed.
    vc_object: Rc<str>,
    /// The same for a class a value class conforms to and holds (`class V(val u: U) extends
    /// AnyVal with U`): the underlying value's type, told apart by identity from the class's own
    /// (a parameter declared `U`), where the value is boxed. By the class's binary name.
    vc_marks: FxMap<Rc<str>, Rc<str>>,
    /// The receiver of a value class's parameter selected (`missing[V].u`), which is unboxed
    /// through the box's accessor without the null test (`vc_field_read`).
    pub(super) vc_select: Option<TExprId>,
    /// The expansions of applied aliases (`alias_expansion`).
    alias_expansions: std::cell::RefCell<FxMap<(AliasId, TList), TypeId>>,
    /// The type parameters whose bounds are being erased, which a bound that names its own
    /// parameter again meets.
    erasing_params: Vec<TParamId>,
}

/// The arity of the largest tuple class, above which a tuple is a `TupleXXL`.
pub(super) const MAX_TUPLE_ARITY: usize = 22; // dotty's `Definitions.MaxTupleArity`

impl<'a> Gen<'a> {
    pub fn new(cx: &'a Cx<'a>) -> Gen<'a> {
        Gen {
            cx,
            errors: Vec::new(),
            refused: Vec::new(),
            erased: Vec::new(),
            sparse: false,
            erased_overlay: FxMap::default(),
            java_erasure: false,
            erased_java: FxMap::default(),
            bottom_markers: false,
            erased_bottoms: FxMap::default(),
            declared: Default::default(),
            class_rc: vec![None; cx.input.syms.classes.len()],
            mrefs: FxMap::default(),
            erased_arities: FxMap::default(),
            mref_arities: FxMap::default(),
            trait_fields: FxMap::default(),
            cw: ClassWriter::new(0, "x", OBJECT, &[]),
            plan: Default::default(),
            lifted_table: Default::default(),
            super_targets: Phased::new(PhasedField::SuperTargets, Vec::new()),
            super_call_at: None,
            lifted_body: None,
            ctor_code: false,
            this_name: Rc::from(""),
            this_class: None,
            is_interface: false,
            abstract_class: false,
            line_file: FileId(u32::MAX),
            line_floor: None,
            line_hold: false,
            pending: Vec::new(),
            pending_next: Vec::new(),
            draining: false,
            lifted: FxMap::default(),
            lifted_lazy_inits: FxMap::default(),
            lazy_holders: Phased::new(PhasedField::LazyHolders, Vec::new()),
            lifted_defaults: FxMap::default(),
            an: Analysis::default(),
            lambdas: FxMap::default(),
            finally_lambdas: FxMap::default(),
            finally_tries: FxMap::default(),
            tries_written: 0,
            class_captures: Vec::new(),
            class_homes: Phased::new(PhasedField::ClassHomes, Vec::new()),
            class_keys: Phased::new(PhasedField::ClassKeys, Vec::new()),
            code: Code::new(Vec::new()),
            lir: crate::lir::Body::default(),
            lower_declared: FxMap::default(),
            local_defs: Vec::new(),
            templates: FxMap::default(),
            module_tree: Phased::new(PhasedField::ModuleTree, None),
            vc_temps: FxMap::default(),
            m: MethodCx::default(),
            context: String::new(),
            function_arities: 0b11,
            extra_classes: Vec::new(),
            module_table: Vec::new(),
            keep_table: false,
            pair_bridged: Vec::new(),
            forward_from: None,
            relax: false,
            overflow: false,
            pool_too_large: false,
            writing_mirror: false,
            long_name_refused: Phased::new(PhasedField::LongNameRefused, false),
            checked_methods: Phased::new(PhasedField::CheckedMethods, FxMap::default()),
            writing_forwarders: false,
            too_large_methods: Vec::new(),
            vc_object: Rc::from(OBJECT),
            vc_marks: FxMap::default(),
            vc_select: None,
            alias_expansions: Default::default(),
            erasing_params: Vec::new(),
        }
    }

    pub fn unsupported(&mut self, what: &str) {
        self.errors.push(format!("{}: {}", self.context, what));
    }

    /// Refuses the program at `e` with the error `message`, a compile error at the expression's
    /// place; one of no place is the class's (`unsupported`).
    pub fn refuse(&mut self, e: TExprId, message: String) {
        match self.cx.input.prog.span_of(e) {
            Some((file, span)) => self.refused.push((file, span, message)),
            None => self.unsupported(&message),
        }
    }

    /// A class with a method too long for 16-bit jump offsets is emitted again, its jumps that do
    /// not reach widened (`Code::widen`).
    pub fn emit_unit(&mut self, unit: Unit) -> (String, Vec<u8>) {
        self.relax = false;
        self.pool_too_large = false;
        self.too_large_methods.clear();
        let (errors, refused) = (self.errors.len(), self.refused.len());
        let out = self.emit_unit_once(unit);
        if !self.overflow || self.pool_too_large {
            self.overflow = false;
            return out;
        }
        self.errors.truncate(errors);
        self.refused.truncate(refused);
        self.relax = true;
        self.overflow = false;
        self.too_large_methods.clear();
        let out = self.emit_unit_once(unit);
        // With wide jumps a method still past the bound has more than 65,535 bytes of code: scalac's error
        // (dotty's PostProcessor.sendToDisk reporting ASM's MethodTooLargeException), at the method.
        for (class, name, desc, at) in std::mem::take(&mut self.too_large_methods) {
            let message = format!("Error while emitting {}\nMethod too large: {}.{} {}", class, class, name, desc);
            match at.or_else(|| self.class_place(&class)) {
                Some((file, span)) => self.refused.push((file, span, message)),
                None => self.unsupported(&message),
            }
        }
        self.relax = false;
        self.overflow = false;
        out
    }

    fn emit_unit_once(&mut self, unit: Unit) -> (String, Vec<u8>) {
        self.extra_classes.clear();
        self.reset_class();
        match unit {
            Unit::Class(i) => self.emit_class_unit(i),
            Unit::File(f) => self.emit_file_unit(f),
        }
    }

    pub fn reset_class(&mut self) {
        write_phase(None);
        self.pending.clear();
        self.pending_next.clear();
        self.draining = false;
        self.local_defs.clear();
        self.pair_bridged.clear();
        self.lifted_table = Default::default();
        self.lifted.clear();
        self.lifted_lazy_inits.clear();
        self.lazy_holders.clear();
        self.lifted_defaults.clear();
        self.an = Analysis::default();
        self.lambdas.clear();
        self.finally_lambdas.clear();
        self.finally_tries.clear();
        self.class_captures.clear();
        self.class_homes.clear();
        self.class_keys.clear();
        *self.module_tree = None;
    }

    // ---- types ----

    pub fn class_name(&mut self, c: ClassId) -> Rc<str> {
        if let Some(n) = &self.class_rc[c.idx()] {
            return n.clone();
        }
        let n: Rc<str> = Rc::from(self.cx.class_names[c.idx()].as_str());
        self.class_rc[c.idx()] = Some(n.clone());
        n
    }

    pub fn class_type(&mut self, c: ClassId) -> ErasedType {
        ErasedType::L(self.class_name(c))
    }

    pub fn function_type(&mut self, arity: usize) -> ErasedType {
        self.function_arities |= 1u64 << arity.min(63);
        ErasedType::L(Rc::from(format!("scala/Function{}", arity)))
    }

    pub fn erase(&mut self, t: TypeId) -> ErasedType {
        if t == NO_TYPE {
            return ErasedType::object();
        }
        let t = self.cx.input.types.underlying(t);
        if t.0 >= crate::arena::LOCAL_BASE || self.sparse || self.java_erasure || self.bottom_markers {
            return self.erase_overlay(t);
        }
        if let Some(Some(j)) = self.erased.get(t.idx()) {
            let j = j.clone();
            if let Some(&used) = self.erased_arities.get(&t) {
                self.function_arities |= used;
            }
            return j;
        }
        let before = std::mem::take(&mut self.function_arities);
        let j = self.erase_uncached(t);
        self.keep_arities(before, |g, used| g.erased_arities.insert(t, used));
        if self.erased.len() <= t.idx() {
            self.erased.resize(t.idx() + 1, None);
        }
        self.erased[t.idx()] = Some(j.clone());
        j
    }

    fn erase_overlay(&mut self, t: TypeId) -> ErasedType {
        let memo = if self.java_erasure {
            &self.erased_java
        } else if self.bottom_markers {
            &self.erased_bottoms
        } else {
            &self.erased_overlay
        };
        if let Some(j) = memo.get(&t) {
            let j = j.clone();
            if let Some(&used) = self.erased_arities.get(&t) {
                self.function_arities |= used;
            }
            return j;
        }
        let before = std::mem::take(&mut self.function_arities);
        let j = self.erase_uncached(t);
        self.keep_arities(before, |g, used| g.erased_arities.insert(t, used));
        if self.java_erasure {
            self.erased_java.insert(t, j.clone());
        } else if self.bottom_markers {
            self.erased_bottoms.insert(t, j.clone());
        } else {
            self.erased_overlay.insert(t, j.clone());
        }
        j
    }

    /// `f` over the erasure of member `s`'s signature: Java's for a member that is Java's, its
    /// bottoms the runtime's markers (`bottom_markers`).
    fn in_language_of<T>(&mut self, s: SymId, f: impl FnOnce(&mut Self) -> T) -> T {
        let java = self.cx.java_defined(s);
        let before = std::mem::replace(&mut self.java_erasure, java);
        let out = self.in_signature(f);
        self.java_erasure = before;
        out
    }

    /// `f` over a signature's erasure (`bottom_markers`).
    fn in_signature<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let before = std::mem::replace(&mut self.bottom_markers, true);
        let out = f(self);
        self.bottom_markers = before;
        out
    }

    /// After a memo's entry was made with the arities taken from `before`: notes the ones past 1
    /// it used with the entry and puts `before` back beside them.
    fn keep_arities(&mut self, before: u64, note: impl FnOnce(&mut Self, u64) -> Option<u64>) {
        let used = self.function_arities & !0b11;
        self.function_arities |= before;
        if used != 0 {
            note(self, used);
        }
    }

    fn erase_uncached(&mut self, t: TypeId) -> ErasedType {
        fn context_function_arity(cx: &Cx, c: ClassId) -> Option<usize> {
            let info = cx.input.syms.class(c);
            let Owner::Package(p) = info.owner else { return None };
            if cx.input.interner.get(cx.input.syms.pkg(p).name) != "scala" {
                return None;
            }
            cx.input.interner.get(info.name).strip_prefix("ContextFunction")?.parse().ok()
        }
        let cx = self.cx;
        if self.bottom_markers {
            if let Some(j) = self.bottom_class(t) {
                return j;
            }
        }
        match cx.input.types.get(t) {
            Type::Class(c, args) => {
                let info = cx.input.syms.class(c);
                // A context function is a plain function of its arity at run time.
                if let Some(n) = context_function_arity(cx, c) {
                    return self.function_type(n);
                }
                if let Some(name) = special_erasure(cx, c) {
                    return ErasedType::L(Rc::from(name));
                }
                // A `*:` chain is the tuple class of its arity where it ends in one, `Product`
                // where it does not (`Int *: EmptyTuple`, `H *: T`), as scalac's `erasePair`.
                if scala_class(cx, c) == Some("*:") {
                    return match cx.input.types.cons_arity(t, c, &|k| tuple_arity(cx, k)) {
                        Some(n) if n <= 22 => ErasedType::L(Rc::from(format!("scala/Tuple{}", n))),
                        Some(_) => ErasedType::L(Rc::from(TUPLE_XXL)),
                        None => ErasedType::L(Rc::from(PRODUCT)),
                    };
                }
                match info.kind {
                    ClassKind::Builtin => match cx.input.interner.get(info.name) {
                        "Int" => ErasedType::I,
                        "Long" => ErasedType::J,
                        "Double" => ErasedType::D,
                        "Byte" => ErasedType::B,
                        "Short" => ErasedType::S,
                        "Float" => ErasedType::F,
                        "Boolean" => ErasedType::Z,
                        "Char" => ErasedType::C,
                        "Unit" => ErasedType::V,
                        "Array" => {
                            let elem = cx.input.types.items(args).first().copied();
                            self.array_type(elem)
                        }
                        _ => self.class_type(c),
                    },
                    // An opaque type erases to its underlying type over its arguments, as an alias
                    // does (`TypeErasure`): `opaque type D[A] = A` to the argument, `Arr[Int]` of
                    // `opaque type Arr[A] = Array[A]` to `[I`.
                    ClassKind::Opaque => match info.underlying {
                        Some(u) => match cx.input.types.get(u) {
                            Type::Param(p) => match info.tparams.iter().position(|&q| q == p).and_then(|i| cx.input.types.items(args).get(i).copied()) {
                                Some(arg) => self.erase(arg),
                                None => self.erase(u),
                            },
                            _ if !info.tparams.is_empty() && cx.input.types.items(args).len() == info.tparams.len() => {
                                let s: Subst = info.tparams.iter().copied().zip(cx.input.types.items(args).iter().copied()).collect();
                                self.erase(cx.input.types.subst(u, &s))
                            }
                            _ => self.erase(u),
                        },
                        None => ErasedType::object(),
                    },
                    _ if info.value_class => self.vc_erasure(c, args),
                    _ if info.js != JsKind::Scala => ErasedType::object(),
                    _ => self.class_type(c),
                }
            }
            Type::Param(p) => self.erase_param(p),
            Type::AppParam(p, args) => self.erase_applied_param(p, args),
            // An inference variable's instance erases as a body's type does, a bottom to `Object`:
            // teq's inference leaves `Nothing` where scalac's infers a real type
            // (`tests/cases/sam_function_value`, DIFFERENCES), and the marker would refuse the value.
            Type::Var(v) => match cx.input.insts.inst(v) {
                Some(inst) => {
                    let before = std::mem::replace(&mut self.bottom_markers, false);
                    let j = self.erase(inst);
                    self.bottom_markers = before;
                    j
                }
                None => ErasedType::object(),
            },
            Type::Union(a, b, _) => self.erase_union(a, b, false, &mut Vec::new()),
            Type::Inter(a, b) => {
                // A value class comes before every other part, as scalac's `ErasedValueType` does,
                // two of them ordered by their classes; the one chosen erases to its underlying type.
                match (self.value_class_of(a), self.value_class_of(b)) {
                    (Some(ca), Some(cb)) => {
                        let (na, nb) = (self.class_type(ca), self.class_type(cb));
                        let first = self.erased_glb_first(&na, &nb);
                        return self.erase(if first { a } else { b });
                    }
                    (Some(_), None) => return self.erase(a),
                    (None, Some(_)) => return self.erase(b),
                    (None, None) => {}
                }
                let (ea, eb) = (self.erase(a), self.erase(b));
                if self.erased_glb_first(&ea, &eb) { ea } else { eb }
            }
            Type::Lit(l) => match cx.input.types.lit_val(l) {
                LitVal::Int(_) => ErasedType::I,
                LitVal::Byte(_) => ErasedType::B,
                LitVal::Short(_) => ErasedType::S,
                LitVal::Long(_) => ErasedType::J,
                LitVal::Double(_) => ErasedType::D,
                LitVal::Float(_) => ErasedType::F,
                LitVal::Char(_) => ErasedType::C,
                LitVal::Bool(_) => ErasedType::Z,
                LitVal::Str(_) => ErasedType::L(Rc::from(STRING)),
                LitVal::Class(class) => return self.erase(class),
            },
            // As scalac erases them: a path to its type (a given object's to its class, the
            // module class its `TermRef` widens to), an abstract member to its upper bound, a
            // refinement to its parent.
            Type::This(c) => self.class_type(c),
            Type::Term(s) | Type::Select(_, s) if cx.given_objects.contains_key(&s) || cx.inner_given_objects.contains_key(&s) => {
                let k = cx.given_objects.get(&s).or_else(|| cx.inner_given_objects.get(&s)).copied().unwrap();
                self.class_type(k)
            }
            Type::Term(s) | Type::Select(_, s) => match cx.input.syms.sym(s).sig.as_ref().map(|sig| sig.ret) {
                Some(ret) => self.erase(ret),
                None => ErasedType::object(),
            },
            Type::Member(prefix, name) => match self.member_upper_bound(prefix, name) {
                Some(upper) => self.erase(upper),
                None => ErasedType::object(),
            },
            Type::AppMember(m, _) => self.erase(m),
            // A class through its prefix erases to the class (dotty's `TypeErasure` of a
            // class `TypeRef`).
            Type::Nested(_, class) => self.erase(class),
            Type::Decl(a) => match self.member_upper(a) {
                Some(upper) => self.erase(upper),
                None => ErasedType::object(),
            },
            Type::Lambda(_, body) | Type::Poly(_, body) => self.erase(body),
            Type::Refined(parent, _) | Type::Rec(parent) | Type::Annotated(parent) => self.erase(parent),
            // A match type the typer left unreduced erases to its bound.
            Type::Match(_, m) => {
                let bound = cx.input.types.match_info(m).bound;
                self.erase(bound)
            }
            // An applied alias erases as its expansion does (dotty's `checkedSuperType`,
            // `TypeErasure.scala` 768, 779), a match type it stands for to its bound over the
            // application's arguments.
            Type::Alias(a, args) => match self.alias_expansion(a, args) {
                Some(x) => self.erase(x),
                None => ErasedType::object(),
            },
            _ => ErasedType::object(),
        }
    }

    /// A declared view, expanded (`symbols::expand_declared`) once per view.
    fn declared_type(&self, t: TypeId) -> TypeId {
        let t = self.cx.input.types.underlying(t);
        if let Some(&x) = self.declared.borrow().get(&t) {
            return x;
        }
        let x = crate::symbols::expand_declared(self.cx.input.syms, self.cx.input.types, t);
        self.declared.borrow_mut().insert(t, x);
        x
    }

    /// The upper bound of type parameter `p` as the erasure reads it: as declared where the
    /// bound holds less of the declaration (`TParamInfo::declared_upper`).
    fn param_upper(&self, p: TParamId) -> TypeId {
        let info = self.cx.input.syms.tparam(p);
        if info.declared_upper != NO_TYPE { self.declared_type(info.declared_upper) } else { info.upper }
    }

    /// The upper bound of abstract type member `a` as the erasure reads it: as declared where the
    /// bound holds less of the declaration (an abstract member's `AliasInfo::declared_rhs`), as
    /// dotty's `checkedSuperType` follows the member's declared info.
    fn member_upper(&self, a: AliasId) -> Option<TypeId> {
        let info = &self.cx.input.syms.aliases[a.idx()];
        let (_, upper) = info.bounds?;
        Some(if info.declared_rhs != NO_TYPE { self.declared_type(info.declared_rhs) } else { upper })
    }

    /// A type parameter erases as its upper bound does, `Unit` as its box: scalac's
    /// `f[T <: String](t: T): T` is `(String)String`, `f[T <: Unit](t: T): T`
    /// `(BoxedUnit)BoxedUnit`. A bound that names the parameter again where it erases
    /// (`T <: Array[T]`, which scalac refuses) gives `Object` there.
    fn erase_param(&mut self, p: TParamId) -> ErasedType {
        if self.erasing_params.contains(&p) {
            return ErasedType::object();
        }
        self.erasing_params.push(p);
        let upper = self.param_upper(p);
        let j = self.in_signature(|g| g.storage(upper));
        self.erasing_params.pop();
        j
    }

    /// `F[A]` of a higher-kinded parameter erases as its bound applied to the arguments:
    /// `F[Int]` with `F[X] <: Array[X]` is an `int[]`.
    fn erase_applied_param(&mut self, p: TParamId, args: TList) -> ErasedType {
        if self.erasing_params.contains(&p) {
            return ErasedType::object();
        }
        self.erasing_params.push(p);
        let j = match self.applied_upper(p, args) {
            Applied::Is(t) => self.in_signature(|g| g.storage(t)),
            Applied::ArrayOf(e) => self.array_type(Some(e)),
        };
        self.erasing_params.pop();
        j
    }

    /// The upper bound of `F` applied to `args`, as far as the erasure reads it without making
    /// a type: the argument a bound `[X] =>> X` returns, the element of `[X] =>> Array[X]`, the
    /// bound's body otherwise, whose class the arguments do not change.
    fn applied_upper(&self, p: TParamId, args: TList) -> Applied {
        let types = self.cx.input.types;
        let upper = self.param_upper(p);
        let Type::Lambda(params, body) = types.get(upper) else { return Applied::Is(upper) };
        let arg_of = |t: TypeId| match types.get(t) {
            Type::Param(x) => types.items(params).iter().position(|&q| matches!(types.get(q), Type::Param(y) if y == x)).and_then(|i| types.items(args).get(i).copied()),
            _ => None,
        };
        match types.get(body) {
            Type::Param(_) => arg_of(body).map_or(Applied::Is(body), Applied::Is),
            Type::Class(c, bargs) if self.cx.input.syms.class(c).kind == ClassKind::Builtin && self.cx.input.interner.get(self.cx.input.syms.class(c).name) == "Array" => {
                match types.items(bargs).first().and_then(|&e| arg_of(e)) {
                    Some(e) => Applied::ArrayOf(e),
                    None => Applied::Is(body),
                }
            }
            _ => Applied::Is(body),
        }
    }

    /// The runtime's class of `t` where it is `Nothing` or `Null`, scala-library's `Nothing$` and
    /// `Null$`, which a scalac caller's descriptor names (dotty's `CoreBTypesFromSymbols`,
    /// `srNothingRef` and `srNullRef`): a signature's erasure (`bottom_markers`).
    fn bottom_class(&self, t: TypeId) -> Option<ErasedType> {
        let cx = self.cx;
        let which = match cx.input.types.get(t) {
            Type::Nothing => 0,
            Type::Class(c, _) if cx.input.syms.class(c).kind == ClassKind::Builtin => match cx.input.interner.get(cx.input.syms.class(c).name) {
                "Nothing" => 0,
                "Null" => 1,
                _ => return None,
            },
            _ => return None,
        };
        Some(ErasedType::L(Rc::from(super::runtime::ARRAY_MARKERS[which])))
    }

    /// The erasure of `A | B`, scalac's `erasedLub` of the parts' erasures (`Lub`): `Nothing | T`
    /// is `T`'s and `Null | C` is `C`'s for a reference `C`, two arrays of references the array of
    /// their elements' lub, two classes the last of their common base classes up to the first
    /// that no other common one derives from (`erased_lub`), `Object` where the parts erase to
    /// other kinds. `boxed` erases a value class as its class (an array's elements); `seen`
    /// holds the type parameters on the way to the union.
    fn erase_union(&mut self, a: TypeId, b: TypeId, boxed: bool, seen: &mut Vec<TParamId>) -> ErasedType {
        let (la, lb) = (self.lub_part(a, boxed, seen), self.lub_part(b, boxed, seen));
        let l = self.lub(la, lb);
        self.lub_type(&l)
    }

    /// A union's part as `erase_union` reads it.
    fn lub_part(&mut self, t: TypeId, boxed: bool, seen: &mut Vec<TParamId>) -> Lub {
        let cx = self.cx;
        match cx.input.types.get(t) {
            Type::Nothing => Lub::Nothing,
            Type::Class(c, args) => {
                let info = cx.input.syms.class(c);
                if info.kind == ClassKind::Builtin {
                    match cx.input.interner.get(info.name) {
                        "Nothing" => return Lub::Nothing,
                        "Null" => return Lub::Null,
                        "Array" => {
                            return match cx.input.types.items(args).first() {
                                Some(&e) if !self.generic_array_element(e) => Lub::Array(Box::new(self.lub_part(e, true, seen))),
                                _ => Lub::Erased(ErasedType::object()),
                            }
                        }
                        _ => return Lub::Erased(self.lub_erasure(t, boxed)),
                    }
                }
                // A class whose erasure is its own name is one whose bases the lub walks; a value
                // class keeps its class beside its underlying erasure.
                let e = self.lub_erasure(t, boxed);
                if e == self.class_type(c) {
                    Lub::Class(c)
                } else if info.value_class {
                    Lub::Value(c, e)
                } else {
                    Lub::Erased(e)
                }
            }
            Type::Param(p) | Type::AppParam(p, _) => {
                if seen.contains(&p) {
                    return Lub::Erased(ErasedType::object());
                }
                seen.push(p);
                let l = match cx.input.types.get(t) {
                    Type::AppParam(_, args) => match self.applied_upper(p, args) {
                        Applied::Is(u) => self.lub_part(u, boxed, seen),
                        Applied::ArrayOf(e) if !self.generic_array_element(e) => Lub::Array(Box::new(self.lub_part(e, true, seen))),
                        Applied::ArrayOf(_) => Lub::Erased(ErasedType::object()),
                    },
                    _ => self.lub_part(self.param_upper(p), boxed, seen),
                };
                seen.pop();
                l
            }
            Type::Union(a, b, _) => {
                let (la, lb) = (self.lub_part(a, boxed, seen), self.lub_part(b, boxed, seen));
                self.lub(la, lb)
            }
            // The side the erasure of the intersection takes: a value class's, else scalac's
            // `erasedGlb` of the two.
            Type::Inter(a, b) => {
                if !boxed {
                    if let Some(c) = self.value_class_of(t) {
                        return Lub::Value(c, self.erase(t));
                    }
                }
                let (la, lb) = (self.lub_part(a, boxed, seen), self.lub_part(b, boxed, seen));
                let (ja, jb) = (self.lub_type(&la), self.lub_type(&lb));
                if self.erased_glb_first(&ja, &jb) { la } else { lb }
            }
            Type::Var(v) => match cx.input.insts.inst(v) {
                Some(inst) => self.lub_part(inst, boxed, seen),
                None => Lub::Erased(ErasedType::object()),
            },
            Type::Term(s) | Type::Select(_, s) => match cx.input.syms.sym(s).sig.as_ref().map(|sig| sig.ret) {
                Some(ret) => self.lub_part(ret, boxed, seen),
                None => Lub::Erased(ErasedType::object()),
            },
            Type::Refined(parent, _) | Type::Rec(parent) | Type::Annotated(parent) => self.lub_part(parent, boxed, seen),
            Type::Member(..) | Type::Decl(_) => match self.abstract_upper(t) {
                Some(upper) => self.lub_part(upper, boxed, seen),
                None => Lub::Erased(ErasedType::object()),
            },
            _ => Lub::Erased(self.lub_erasure(t, boxed)),
        }
    }

    /// A part's own erasure, `Unit` as its box.
    fn lub_erasure(&mut self, t: TypeId, boxed: bool) -> ErasedType {
        match if boxed { self.erase_boxed_vc(t) } else { self.erase(t) } {
            ErasedType::V => ErasedType::L(Rc::from(BOXED_UNIT)),
            j => j,
        }
    }

    fn lub(&mut self, x: Lub, y: Lub) -> Lub {
        let reference = |l: &Lub| !matches!(l, Lub::Erased(j) if !j.is_ref()) && !matches!(l, Lub::Value(..));
        // `Null | C` is `C`'s where `C` derives from `Object` (`erasedLub`: `tp2.derivesFrom(defn.ObjectClass)`): a
        // value class kept as its class (an array's element, `boxed`) derives from `AnyVal` and a universal trait
        // from `Any` alone, and the lub of either with `Null` is what their base classes share, `Object`.
        let cx = self.cx;
        let of_object = |l: &Lub| {
            reference(l)
                && !matches!(l, Lub::Class(c) if {
                    let info = cx.input.syms.class(*c);
                    info.value_class || info.mods & crate::ast::mods::UNIVERSAL != 0
                })
        };
        match (&x, &y) {
            (Lub::Nothing, _) => return y,
            (_, Lub::Nothing) => return x,
            (Lub::Null, y) if of_object(y) => return y.clone(),
            (x, Lub::Null) if of_object(x) => return x.clone(),
            _ => {}
        }
        // Two parts that erase alike (`Show[Int] | D[Show[Int]]` of an opaque `D[A] = A`); never
        // two value classes, which scalac's lub takes to `Object` even where they are one class
        // (`V | V`, `V | W` over one `Int`).
        if matches!(x, Lub::Value(..)) || matches!(y, Lub::Value(..)) {
            return Lub::Erased(ErasedType::object());
        }
        if x == y || self.lub_type(&x) == self.lub_type(&y) {
            return x;
        }
        match (x, y) {
            (Lub::Array(ex), Lub::Array(ey)) if reference(&ex) && reference(&ey) => Lub::Array(Box::new(self.lub(*ex, *ey))),
            (x, y) => match (self.lub_bases(&x), self.lub_bases(&y)) {
                (Some(a), Some(b)) => self.erased_lub(&a, &b).map_or(Lub::Erased(ErasedType::object()), Lub::Class),
                _ => Lub::Erased(ErasedType::object()),
            },
        }
    }

    /// The base classes `erasedLub` walks of a part, in linearisation order: a class's, and the
    /// builtin `String`'s as the JDK's class file declares its interfaces (`Serializable`,
    /// `Comparable`, `CharSequence`, `Constable`, `ConstantDesc`, linearised the last first), those
    /// the program knows; a JDK class the std stands in for has the parents its stand-in declares.
    fn lub_bases(&self, l: &Lub) -> Option<Vec<ClassId>> {
        const STRING_BASES: [&str; 5] = ["java/lang/constant/ConstantDesc", "java/lang/constant/Constable", "java/lang/CharSequence", "java/lang/Comparable", "java/io/Serializable"];
        let cx = self.cx;
        match l {
            Lub::Class(c) => Some(cx.input.syms.class(*c).base_types.iter().map(|&(k, _)| k).collect()),
            Lub::Erased(s) if s.is_string() => Some(STRING_BASES.iter().filter_map(|name| cx.class_by_name.get(*name).copied()).collect()),
            _ => None,
        }
    }

    fn lub_type(&mut self, l: &Lub) -> ErasedType {
        match l {
            Lub::Class(c) => self.class_type(*c),
            Lub::Array(e) => {
                let e = match **e {
                    Lub::Nothing => self.array_marker(0),
                    Lub::Null => self.array_marker(1),
                    ref e => self.lub_type(e),
                };
                let mut d = String::from("[");
                e.desc(&mut d);
                ErasedType::L(Rc::from(d.as_str()))
            }
            Lub::Nothing if self.bottom_markers => self.array_marker(0),
            Lub::Null if self.bottom_markers => self.array_marker(1),
            Lub::Nothing | Lub::Null => ErasedType::object(),
            Lub::Value(_, j) | Lub::Erased(j) => j.clone(),
        }
    }

    /// scalac's `erasedLub` (`TypeErasure.erasedLub`) of two parts by their base classes: of the
    /// first's in linearisation order those the second derives from (is among its bases), the
    /// classes up to the first trait and that trait (its `takeUntil`); of those, the last that no
    /// other derives from (`TreeSeqMap | VectorMap` is the trait `StrictOptimizedMapOps`, `PA |
    /// PB` of two subclasses of `P0` is `P0`, `String | Both` `CharSequence` and `Both | String`
    /// `Serializable` for a `Both extends CharSequence with Serializable`).
    fn erased_lub(&self, a: &[ClassId], b: &[ClassId]) -> Option<ClassId> {
        let syms = self.cx.input.syms;
        let derives = |x: ClassId, y: ClassId| x == y || syms.class(x).base_types.iter().any(|&(k, _)| k == y);
        let mut candidates: Vec<ClassId> = Vec::new();
        for &k in a {
            if !b.contains(&k) {
                continue;
            }
            candidates.push(k);
            if syms.class(k).kind == ClassKind::Trait {
                break;
            }
        }
        candidates.iter().copied().filter(|&cand| candidates.iter().all(|&x| x == cand || !derives(x, cand))).last()
    }

    /// The upper bound of a type parameter or of an abstract type member.
    fn abstract_upper(&mut self, t: TypeId) -> Option<TypeId> {
        let cx = self.cx;
        match cx.input.types.get(t) {
            Type::Param(p) | Type::AppParam(p, _) => Some(self.param_upper(p)),
            Type::Member(prefix, name) => self.member_upper_bound(prefix, name),
            Type::Decl(a) => self.member_upper(a),
            _ => None,
        }
    }

    /// The kind of JVM array that holds every value of `t`, scalac's `arrayUpperBound`: the
    /// primitive's for a primitive type, `Object` for a reference type, none for a type whose
    /// values may be either (`Any`, `AnyVal`, `Matchable`, `Singleton`, `Int | String`).
    fn array_bound(&mut self, t: TypeId) -> Option<ErasedType> {
        let cx = self.cx;
        match cx.input.types.get(t) {
            Type::Any => None,
            Type::Nothing | Type::This(_) => Some(ErasedType::object()),
            Type::Class(c, args) => {
                let info = cx.input.syms.class(c);
                match info.kind {
                    ClassKind::Builtin => match cx.input.interner.get(info.name) {
                        "AnyVal" => None,
                        "Int" | "Long" | "Double" | "Byte" | "Short" | "Float" | "Boolean" | "Char" | "Unit" => Some(self.erase(t)),
                        _ => Some(ErasedType::object()),
                    },
                    ClassKind::Opaque => {
                        let u = info.underlying?;
                        match cx.input.types.get(u) {
                            Type::Param(p) => match info.tparams.iter().position(|&q| q == p).and_then(|i| cx.input.types.items(args).get(i).copied()) {
                                Some(arg) => self.array_bound(arg),
                                None => self.array_bound(u),
                            },
                            _ => self.array_bound(u),
                        }
                    }
                    _ if matches!(scala_class(cx, c), Some("Matchable" | "Singleton")) => None,
                    _ => Some(ErasedType::object()),
                }
            }
            Type::Lit(_) => Some(self.erase(t)),
            Type::Param(p) | Type::AppParam(p, _) => {
                if self.erasing_params.contains(&p) {
                    return None;
                }
                self.erasing_params.push(p);
                let b = match cx.input.types.get(t) {
                    Type::AppParam(_, args) => match self.applied_upper(p, args) {
                        Applied::Is(u) => self.array_bound(u),
                        Applied::ArrayOf(_) => Some(ErasedType::object()),
                    },
                    _ => self.array_bound(self.param_upper(p)),
                };
                self.erasing_params.pop();
                b
            }
            Type::Member(..) | Type::Decl(_) => {
                let upper = self.abstract_upper(t)?;
                self.array_bound(upper)
            }
            Type::Term(s) | Type::Select(_, s) => {
                let ret = cx.input.syms.sym(s).sig.as_ref()?.ret;
                self.array_bound(ret)
            }
            Type::Refined(parent, _) | Type::Rec(parent) | Type::Annotated(parent) | Type::Lambda(_, parent) | Type::Nested(_, parent) => self.array_bound(parent),
            Type::Alias(a, args) => {
                let x = self.alias_expansion(a, args)?;
                self.array_bound(x)
            }
            Type::Var(v) => {
                let inst = cx.input.insts.inst(v)?;
                self.array_bound(inst)
            }
            Type::Inter(a, b) => {
                let (ba, bb) = (self.array_bound(a), self.array_bound(b));
                if ba == bb { ba } else { ba.or(bb) }
            }
            Type::Union(a, b, _) => {
                let (ba, bb) = (self.array_bound(a), self.array_bound(b));
                if ba == bb { ba } else { None }
            }
            _ => None,
        }
    }

    /// Whether `Array[t]` erases to `Object`, scalac's `isGenericArrayElement`: `t` is abstract
    /// and its bound fits no one kind of JVM array, or an intersection of such types, or a
    /// union with one.
    fn generic_array_element(&mut self, t: TypeId) -> bool {
        let cx = self.cx;
        match cx.input.types.get(t) {
            Type::Param(_) => !self.java_erasure && self.array_bound(t).is_none(),
            Type::AppParam(..) | Type::Member(..) | Type::Decl(_) => self.array_bound(t).is_none(),
            Type::Inter(a, b) => self.generic_array_element(a) && self.generic_array_element(b),
            Type::Union(a, b, _) => self.generic_array_element(a) || self.generic_array_element(b),
            Type::Var(v) => cx.input.insts.inst(v).map_or(true, |inst| self.generic_array_element(inst)),
            Type::Alias(a, args) => self.alias_expansion(a, args).map_or(true, |x| self.generic_array_element(x)),
            _ => false,
        }
    }

    /// The erasure of a type where a value class is its class, not its underlying value
    /// (scalac's erasure with `semiEraseVCs` off): the elements of an array, the underlying
    /// value of a value class over a type parameter (`W[V]` holds a `V`).
    fn erase_boxed_vc(&mut self, t: TypeId) -> ErasedType {
        self.erase_boxed_vc_in(t, &mut Vec::new())
    }

    fn erase_boxed_vc_in(&mut self, t: TypeId, seen: &mut Vec<TParamId>) -> ErasedType {
        let cx = self.cx;
        let mut t = t;
        loop {
            match cx.input.types.get(t) {
                Type::Class(c, _) if cx.input.syms.class(c).value_class => return self.class_type(c),
                Type::Param(p) | Type::AppParam(p, _) if !seen.contains(&p) => {
                    seen.push(p);
                    t = self.param_upper(p);
                }
                Type::Var(v) => match cx.input.insts.inst(v) {
                    Some(inst) => t = inst,
                    None => return ErasedType::object(),
                },
                Type::Alias(a, args) => match self.alias_expansion(a, args) {
                    Some(x) => t = x,
                    None => return ErasedType::object(),
                },
                Type::Inter(a, b) => {
                    let at = seen.len();
                    let ea = self.erase_boxed_vc_in(a, seen);
                    seen.truncate(at);
                    let eb = self.erase_boxed_vc_in(b, seen);
                    return if self.erased_glb_first(&ea, &eb) { ea } else { eb };
                }
                Type::Union(a, b, _) => return self.erase_union(a, b, true, seen),
                _ => return self.storage(t),
            }
        }
    }

    /// The type alias `a` applied to `args` stands for, its right-hand side with the arguments
    /// for its parameters (`Types::subst`), so that an array's element, a value class's argument
    /// and a match type's bound are classified as the expansion (`TypeErasure.scala` 779, 901,
    /// 927-935). None for an abstract member and for an expansion that is the application again.
    pub(super) fn alias_expansion(&self, a: AliasId, args: TList) -> Option<TypeId> {
        if let Some(&x) = self.alias_expansions.borrow().get(&(a, args)) {
            return Some(x);
        }
        let types = self.cx.input.types;
        let info = &self.cx.input.syms.aliases[a.idx()];
        // An abstract type member has bounds, no right-hand side (`Type::Decl` erases it).
        if info.is_abstract() {
            return None;
        }
        // The right-hand side over exactly the alias's parameters is stored as a lambda over them.
        let body = match types.get(info.rhs) {
            Type::Lambda(ps, b)
                if types.items(ps).len() == info.tparams.len()
                    && types.items(ps).iter().zip(&info.tparams).all(|(&p, &q)| matches!(types.get(p), Type::Param(r) if r == q)) =>
            {
                b
            }
            _ => info.rhs,
        };
        let items = types.items(args);
        let s: Subst = info.tparams.iter().copied().zip(items.iter().copied()).collect();
        let mut x = types.subst(body, &s);
        // A curried alias (`type F[A] = [B] =>> B`, `F[String][Int]`) takes the rest of the arguments
        // in its lambdas, in order.
        let mut rest = items.get(info.tparams.len()..).unwrap_or(&[]);
        while !rest.is_empty() {
            let Type::Lambda(ps, b) = types.get(x) else { break };
            let ps = types.items(ps);
            let s: Subst = ps.iter().zip(rest).filter_map(|(&p, &a)| match types.get(p) {
                Type::Param(q) => Some((q, a)),
                _ => None,
            }).collect();
            x = types.subst(b, &s);
            rest = &rest[ps.len().min(rest.len())..];
        }
        if matches!(types.get(x), Type::Alias(b, bargs) if b == a && bargs == args) {
            return None;
        }
        self.alias_expansions.borrow_mut().insert((a, args), x);
        Some(x)
    }

    /// The erasure of the value class `c` applied to `args`, scalac's `eraseDerivedValueClass`:
    /// the underlying type as the arguments instantiate it (`W[String]` for `W[A](value: A)` is
    /// a `String`), a primitive boxed where the declared underlying type is a type parameter
    /// (`W[Int]` is an `Integer`), and an array's erasure as the class declares it.
    /// The result is marked as `vc_marked` says.
    pub(super) fn vc_erasure(&mut self, c: ClassId, args: TList) -> ErasedType {
        let j = self.vc_erasure_plain(c, args);
        self.vc_marked(c, j)
    }

    fn vc_erasure_plain(&mut self, c: ClassId, args: TList) -> ErasedType {
        let cx = self.cx;
        let syms = cx.input.syms;
        let info = syms.class(c);
        let declared = info.ctor.first().and_then(|cl| cl.params.first()).and_then(|p| syms.sym(p.sym).sig.as_ref().map(|sig| sig.ret));
        let arg_of = |p: TParamId| info.tparams.iter().position(|&q| q == p).and_then(|i| cx.input.types.items(args).get(i).copied());
        let arg = match declared.map(|d| cx.input.types.get(d)) {
            Some(Type::Param(p)) => arg_of(p),
            // `H[F[_], A](value: F[A])`: `H[List, Int]` holds a `List`, the class the argument names.
            Some(Type::AppParam(p, applied)) => {
                // `F[A]`'s arguments as the class's arguments instantiate them.
                let applied: Vec<TypeId> = cx.input.types.items(applied).iter().map(|&x| match cx.input.types.get(x) {
                    Type::Param(q) => arg_of(q).unwrap_or(x),
                    _ => x,
                }).collect();
                match arg_of(p).map(|a| cx.input.types.get(a)) {
                    Some(Type::Ctor(k)) => {
                        let info = cx.input.syms.class(k);
                        let array = info.kind == ClassKind::Builtin && cx.input.interner.get(info.name) == "Array";
                        return match special_erasure(cx, k) {
                            Some(name) => ErasedType::L(Rc::from(name)),
                            None if array => self.array_type(applied.first().copied()),
                            None if info.kind == ClassKind::Opaque || info.kind == ClassKind::Builtin || info.js != JsKind::Scala => self.vc_underlying(c),
                            None => self.class_type(k),
                        };
                    }
                    // `[X] =>> X` holds its argument; another body's erasure is its class's.
                    Some(Type::Lambda(params, body)) => match cx.input.types.get(body) {
                        Type::Param(x) => cx.input.types.items(params).iter().position(|&q| matches!(cx.input.types.get(q), Type::Param(y) if y == x)).and_then(|i| applied.get(i).copied()),
                        _ => Some(body),
                    },
                    _ => None,
                }
            }
            _ => None,
        };
        let Some(arg) = arg else { return self.vc_underlying(c) };
        match self.erase_boxed_vc(arg) {
            j if j.is_object() => ErasedType::L(self.vc_object.clone()),
            j if !j.is_ref() => ErasedType::L(Rc::from(j.box_class())),
            j => j,
        }
    }

    /// Whether the erasure of `A & B` is `A`'s, by scalac's order (`TypeErasure.erasedGlb`): an
    /// array, then a primitive, then a class before a trait, then a subclass before its
    /// superclass, then the first by its full name.
    fn erased_glb_first(&self, a: &ErasedType, b: &ErasedType) -> bool {
        if a == b {
            return true;
        }
        let array = |t: &ErasedType| match t {
            ErasedType::L(n) => n.strip_prefix('[').map(desc_type),
            _ => None,
        };
        match (array(a), array(b)) {
            (Some(ea), Some(eb)) => return self.erased_glb_first(&ea, &eb),
            (Some(_), None) => return true,
            (None, Some(_)) => return false,
            (None, None) => {}
        }
        let (ErasedType::L(x), ErasedType::L(y)) = (a, b) else {
            return match (a.is_ref(), b.is_ref()) {
                (false, true) => true,
                (true, false) => false,
                _ => scala_name_of(a) <= scala_name_of(b),
            };
        };
        let cx = self.cx;
        let class = |n: &str| if n == OBJECT { None } else { cx.class_named(n) };
        let (cx_, cy) = (class(x), class(y));
        let interface = |c: Option<ClassId>| c.map_or(false, |c| cx.is_interface(c));
        let (ix, iy) = (interface(cx_), interface(cy));
        if ix != iy {
            return !ix;
        }
        let derives = |sub: Option<ClassId>, sup: Option<ClassId>, sup_name: &str| {
            sup_name == OBJECT || matches!((sub, sup), (Some(s), Some(p)) if cx.input.syms.class(s).base_types.iter().any(|&(b, _)| b == p))
        };
        if derives(cx_, cy, y) {
            return true;
        }
        if derives(cy, cx_, x) {
            return false;
        }
        self.scala_full_name(cx_, x) <= self.scala_full_name(cy, y)
    }

    /// A class's full name as scalac's `fullName` spells it, its source names joined by dots, an
    /// object's (a module class's) with its `$`, which orders the parts of an intersection that
    /// nothing else orders.
    fn scala_full_name(&self, c: Option<ClassId>, binary: &str) -> String {
        let Some(c) = c else { return binary.replace('/', ".") };
        let syms = self.cx.input.syms;
        let interner = self.cx.input.interner;
        let part = |c: ClassId| {
            let info = syms.class(c);
            let name = interner.get(info.name);
            if info.kind == ClassKind::Object { format!("{}$", name) } else { name.to_string() }
        };
        let mut parts = vec![part(c)];
        let mut owner = syms.class(c).owner;
        while let Owner::Class(o) = owner {
            parts.push(part(o));
            owner = syms.class(o).owner;
        }
        let mut pkg = match owner {
            Owner::Package(p) => Some(p),
            _ => None,
        };
        while let Some(p) = pkg {
            let info = syms.pkg(p);
            if info.parent.is_none() {
                break;
            }
            parts.push(interner.get(info.name).to_string());
            pkg = info.parent;
        }
        parts.reverse();
        parts.join(".")
    }

    /// The declared upper bound of the type member `name` of the class the prefix is an
    /// instance of; the bound is in terms of that class, which erasure does not mind.
    fn member_upper_bound(&mut self, prefix: TypeId, name: Name) -> Option<TypeId> {
        let cx = self.cx;
        let mut t = prefix;
        let c = loop {
            match cx.input.types.get(t) {
                Type::This(c) | Type::Class(c, _) => break c,
                Type::Term(s) | Type::Select(_, s) => t = cx.input.syms.sym(s).sig.as_ref()?.ret,
                Type::Refined(p, _) | Type::Rec(p) | Type::Annotated(p) | Type::Nested(_, p) => t = p,
                Type::Var(v) => t = cx.input.insts.inst(v)?,
                Type::Inter(a, _) => t = a,
                _ => return None,
            }
        };
        for &(b, _) in &cx.input.syms.class(c).base_types {
            if let Some(&a) = cx.input.syms.class(b).type_aliases.get(&name) {
                return Some(self.member_upper(a).unwrap_or(cx.input.syms.aliases[a.idx()].rhs));
            }
        }
        None
    }

    // ---- JVM arrays ----

    /// `Array[T]` as scalac erases it: a JVM array of the element's erasure, a value class's
    /// elements boxed (`[LMeters;`), and `Object` where the element is a type parameter,
    /// `Any` or an abstract type, whose arrays may be of any kind.
    fn array_type(&mut self, elem: Option<TypeId>) -> ErasedType {
        let Some(elem) = elem else { return ErasedType::object() };
        let cx = self.cx;
        let mut t = elem;
        loop {
            match cx.input.types.get(t) {
                Type::Var(v) => match cx.input.insts.inst(v) {
                    Some(inst) => t = inst,
                    None => return ErasedType::object(),
                },
                Type::Nested(_, class) => t = class,
                // `Array[Id[Int]]` is an `int[]`, `Array[Id[V]]` a `V[]`.
                Type::Alias(a, args) => match self.alias_expansion(a, args) {
                    Some(x) => t = x,
                    None => return ErasedType::object(),
                },
                _ => break,
            }
        }
        // A singleton's array holds what the path's type does.
        match cx.input.types.get(t) {
            Type::Term(s) | Type::Select(_, s) => {
                if let Some(under) = cx.input.syms.sym(s).sig.as_ref().map(|sig| sig.ret) {
                    return self.array_type(Some(under));
                }
            }
            Type::This(c) => {
                let e = self.class_type(c);
                let mut d = String::from("[");
                e.desc(&mut d);
                return ErasedType::L(Rc::from(d.as_str()));
            }
            _ => {}
        }
        if self.generic_array_element(t) {
            return ErasedType::object();
        }
        let e = match cx.input.types.get(t) {
            Type::Any => ErasedType::object(),
            Type::Nothing => self.array_marker(0),
            // A bound that fits one kind of array is the element's: `Array[T <: String]` is a
            // `String[]`, `Array[T <: Int]` an `int[]`, `Array[T <: V]` a `V[]`.
            Type::Param(p) | Type::AppParam(p, _) => {
                if self.erasing_params.contains(&p) {
                    return ErasedType::object();
                }
                self.erasing_params.push(p);
                let a = match cx.input.types.get(t) {
                    Type::AppParam(_, args) => match self.applied_upper(p, args) {
                        Applied::Is(u) => self.array_type(Some(u)),
                        Applied::ArrayOf(e) => {
                            let inner = self.array_type(Some(e));
                            let mut d = String::from("[");
                            inner.desc(&mut d);
                            ErasedType::L(Rc::from(d.as_str()))
                        }
                    },
                    _ => self.array_type(Some(self.param_upper(p))),
                };
                self.erasing_params.pop();
                return a;
            }
            Type::Member(..) | Type::Decl(_) => match self.abstract_upper(t) {
                Some(upper) => return self.array_type(Some(upper)),
                None => return ErasedType::object(),
            },
            Type::Inter(..) | Type::Union(..) => self.erase_boxed_vc(t),
            Type::Class(c, _) if cx.input.syms.class(c).value_class => self.class_type(c),
            Type::Class(c, _) if cx.input.syms.class(c).kind == ClassKind::Builtin => match cx.input.interner.get(cx.input.syms.class(c).name) {
                "Null" => self.array_marker(1),
                "Nothing" => self.array_marker(0),
                _ => self.storage(t),
            },
            Type::Class(..) | Type::Lit(_) | Type::Nested(..) => self.storage(t),
            _ => return ErasedType::object(),
        };
        let mut d = String::from("[");
        e.desc(&mut d);
        ErasedType::L(Rc::from(d.as_str()))
    }

    /// The class scalac erases `Nothing` (0) or `Null` (1) to, scala-library's, which is also
    /// the component of their arrays.
    fn array_marker(&mut self, which: usize) -> ErasedType {
        ErasedType::L(Rc::from(super::runtime::ARRAY_MARKERS[which]))
    }

    /// The operation `name` on an array whose kind only the value knows (`erased_array_method`),
    /// its operands on the stack.
    fn erased_array_op(&mut self, name: &str) -> ErasedType {
        let m = erased_array_method(name);
        self.invoke_mref(&m);
        m.ret
    }

    /// `t` as a type test reads it (`Worker::test_for`): a variable's instance, an alias expanded, the parent of a
    /// refinement, of a recursive and of an annotated type, the class of a type under a prefix.
    pub fn tested_form(&mut self, t: TypeId) -> TypeId {
        let cx = self.cx;
        let mut t = cx.input.types.underlying(t);
        loop {
            match cx.input.types.get(t) {
                Type::Var(v) => match cx.input.insts.inst(v) {
                    Some(inst) => t = inst,
                    None => return t,
                },
                Type::Alias(a, args) => match self.alias_expansion(a, args) {
                    Some(x) => t = x,
                    None => return t,
                },
                Type::Refined(parent, _) | Type::Rec(parent) | Type::Annotated(parent) | Type::Nested(_, parent) => t = parent,
                _ => return t,
            }
        }
    }

    /// The element of the array type `t` (`tested_form`); None for any other type.
    pub fn array_elem_of(&mut self, t: TypeId) -> Option<TypeId> {
        let t = self.tested_form(t);
        let cx = self.cx;
        match cx.input.types.get(t) {
            Type::Class(c, args) if cx.input.syms.class(c).kind == ClassKind::Builtin && cx.input.interner.get(cx.input.syms.class(c).name) == "Array" => cx.input.types.items(args).first().copied(),
            _ => None,
        }
    }

    /// The test of a value against the array type `t`, as scalac's `TypeTestsCasts.transformTypeTest` makes it:
    /// the erased class of the array (`Ok`), or, where the element at the bottom of its array layers is generic
    /// (`MultiArrayOf(elem, ndims)` with `isGenericArrayElement(elem)`: the innermost array erases to no array
    /// class), the number of layers for `ScalaRunTime.isArray(x, ndims)` (`Err`). A type that is an array by its
    /// erasure alone (an opaque alias of one) is tested by that class; one that erases to no array class, as any
    /// array.
    pub fn array_test_of(&mut self, t: TypeId) -> Result<Rc<str>, i32> {
        // An opaque type that stands for itself has no erasure to read: any array, as before.
        if self.names_cyclic_opaque(t, &mut Vec::new()) {
            return Err(1);
        }
        let (mut layers, mut cur, mut innermost) = (0, t, None);
        while let Some(elem) = self.array_elem_of(cur) {
            layers += 1;
            innermost = Some(cur);
            cur = elem;
        }
        let erased_array = |g: &mut Self, t: TypeId| match g.erase(t) {
            ErasedType::L(n) if n.starts_with('[') => Some(n),
            _ => None,
        };
        match innermost {
            Some(inner) if erased_array(self, inner).is_none() => {
                // An element that is a refinement, a structural type or a polymorphic function type erases to its
                // parent's or its function's class for the test (`eraseArray` of scalac's `transformTypeTest`):
                // the layers around that class. The erasure of the array type itself (a signature's) is not this
                // leaf's and stays what it is.
                let elem = self.array_elem_of(inner).expect("an array type");
                match self.erased_element_form(elem) {
                    Some(one) => Ok(Rc::from(format!("{}{}", "[".repeat(layers as usize - 1), one))),
                    None => Err(layers),
                }
            }
            _ => erased_array(self, t).ok_or(1),
        }
    }

    /// Whether `t` names an opaque type that stands for itself (`opaque type Chain = Array[Chain]`: scalac refuses
    /// the definition, E140; the tree does not yet, and the erasure does not end on one). `open`: the opaque types
    /// whose definitions are being read, each as it is defined.
    fn names_cyclic_opaque(&self, t: TypeId, open: &mut Vec<ClassId>) -> bool {
        let cx = self.cx;
        if let Type::Class(c, _) = cx.input.types.get(t) {
            let info = cx.input.syms.class(c);
            if info.kind == ClassKind::Opaque {
                if open.contains(&c) {
                    return true;
                }
                if let Some(under) = info.underlying {
                    open.push(c);
                    let cyclic = self.names_cyclic_opaque(under, open);
                    open.pop();
                    if cyclic {
                        return true;
                    }
                }
            }
        }
        let mut parts = Vec::new();
        cx.input.types.parts(t, &mut parts);
        parts.into_iter().any(|p| self.names_cyclic_opaque(p, open))
    }

    /// The class of an array of `elem` for a type test where the element's form hides it from the array's own
    /// erasure (`array_type`): the array of a refinement's, a recursive or an annotated type's parent, of a
    /// structural type's `Object`, of a polymorphic function type's function class; None for any other element (a
    /// type parameter's is asked by its layers).
    fn erased_element_form(&mut self, elem: TypeId) -> Option<Rc<str>> {
        let cx = self.cx;
        let mut t = cx.input.types.underlying(elem);
        let mut stripped = false;
        loop {
            match cx.input.types.get(t) {
                Type::Refined(parent, _) | Type::Rec(parent) | Type::Annotated(parent) => {
                    t = parent;
                    stripped = true;
                }
                Type::Poly(_, body) => {
                    t = body;
                    stripped = true;
                }
                Type::Alias(a, args) => t = self.alias_expansion(a, args)?,
                _ => break,
            }
        }
        if !stripped || self.generic_array_element(t) {
            return None;
        }
        match self.array_type(Some(t)) {
            ErasedType::L(n) if n.starts_with('[') => Some(n),
            _ => None,
        }
    }

    pub fn array_element(t: &ErasedType) -> Option<ErasedType> {
        match t {
            ErasedType::L(n) if n.starts_with('[') => Some(parse_type(&n[1..]).0),
            _ => None,
        }
    }

    /// `array(i)` with the array and the index on the stack: the load instruction of a JVM
    /// array's kind, `ScalaRunTime.array_apply` on an erased one.
    pub fn array_load(&mut self, array: &ErasedType) -> ErasedType {
        let Some(elem) = Self::array_element(array) else {
            return self.erased_array_op("array_apply");
        };
        let opcode = match elem {
            ErasedType::I => op::IALOAD,
            ErasedType::J => op::LALOAD,
            ErasedType::F => op::FALOAD,
            ErasedType::D => op::DALOAD,
            ErasedType::Z | ErasedType::B => op::BALOAD,
            ErasedType::C => op::CALOAD,
            ErasedType::S => op::SALOAD,
            _ => op::AALOAD,
        };
        self.code.op(opcode);
        self.code.popn(2);
        let vt = self.vt(&elem);
        self.code.push(vt);
        elem
    }

    /// `array(i) = v` with the array, the index and the value (of type `value`) on the stack.
    pub fn array_store(&mut self, array: &ErasedType, value: &ErasedType) {
        let Some(elem) = Self::array_element(array) else {
            self.adapt(value, &ErasedType::object());
            self.erased_array_op("array_update");
            return;
        };
        self.adapt(value, &elem);
        let opcode = match elem {
            ErasedType::I => op::IASTORE,
            ErasedType::J => op::LASTORE,
            ErasedType::F => op::FASTORE,
            ErasedType::D => op::DASTORE,
            ErasedType::Z | ErasedType::B => op::BASTORE,
            ErasedType::C => op::CASTORE,
            ErasedType::S => op::SASTORE,
            _ => op::AASTORE,
        };
        self.code.op(opcode);
        self.code.popn(3);
    }

    pub fn array_length(&mut self, array: &ErasedType) {
        if Self::array_element(array).is_none() {
            self.erased_array_op("array_length");
            return;
        }
        self.code.op(op::ARRAYLENGTH);
        self.code.pop();
        self.code.push(VT::Int);
    }

    pub fn array_clone(&mut self, array: &ErasedType) -> ErasedType {
        if Self::array_element(array).is_none() {
            return self.erased_array_op("array_clone");
        }
        let ErasedType::L(name) = array else { return array.clone() };
        let object = ErasedType::object();
        self.invoke_desc(Invoke::Virtual, name, false, "clone", "()Ljava/lang/Object;", 0, &object);
        self.checkcast(name);
        array.clone()
    }

    /// A new array of type `array` whose length is on the stack.
    pub fn new_array(&mut self, array: &ErasedType) {
        let Some(elem) = Self::array_element(array) else {
            self.unsupported("a new array of an element type only a ClassTag knows");
            return;
        };
        let atype = match elem {
            ErasedType::Z => Some(4),
            ErasedType::C => Some(5),
            ErasedType::F => Some(6),
            ErasedType::D => Some(7),
            ErasedType::B => Some(8),
            ErasedType::S => Some(9),
            ErasedType::I => Some(10),
            ErasedType::J => Some(11),
            _ => None,
        };
        match (atype, &elem) {
            (Some(code), _) => self.code.op_u8(op::NEWARRAY, code),
            (None, ErasedType::L(n)) => {
                let index = self.cw.cp.class(n);
                self.code.op_u16(op::ANEWARRAY, index);
            }
            _ => {}
        }
        self.code.pop();
        let vt = self.vt(array);
        self.code.push(vt);
    }

    // ---- value classes in link mode ----

    /// The erasure of a value class in link mode, as scalac's: the erasure of its underlying
    /// type, the type of its one parameter.
    pub fn vc_underlying(&mut self, c: ClassId) -> ErasedType {
        let syms = self.cx.input.syms;
        let Some(field) = syms.class(c).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym) else { return ErasedType::object() };
        let Some(ret) = syms.sym(field).sig.as_ref().map(|sig| sig.ret) else { return ErasedType::object() };
        let t = self.storage(ret);
        self.vc_marked(c, t)
    }

    pub fn is_vc_object(&self, t: &ErasedType) -> bool {
        matches!(t, ErasedType::L(n) if Rc::ptr_eq(n, &self.vc_object))
    }

    /// Whether `t` is the marked type of a value class's underlying value (`vc_object`,
    /// `vc_marks`), which a reference of the same descriptor is not.
    pub fn is_vc_mark(&self, t: &ErasedType) -> bool {
        match t {
            ErasedType::L(n) => Rc::ptr_eq(n, &self.vc_object) || self.vc_marks.get(n).map_or(false, |m| Rc::ptr_eq(m, n)),
            _ => false,
        }
    }

    /// `u` without the mark of a value class's underlying value (`vc_marked`): the ordinary type of
    /// that descriptor, which a parameter declared with the type takes.
    pub(super) fn vc_unmarked(&self, u: &ErasedType) -> ErasedType {
        match u {
            ErasedType::L(n) if self.is_vc_mark(u) => ErasedType::L(Rc::from(&**n)),
            _ => u.clone(),
        }
    }

    /// The erasure `j` of the underlying value of value class `c`, marked where a reference of
    /// that descriptor could be the box: `Object`, or a class `c` conforms to. dotty keeps the
    /// two apart as `ErasedValueType` and the class (`Erasure.scala` 385-406).
    fn vc_marked(&mut self, c: ClassId, j: ErasedType) -> ErasedType {
        let ErasedType::L(n) = &j else { return j };
        if self.is_vc_mark(&j) {
            return j;
        }
        if j.is_object() {
            return ErasedType::L(self.vc_object.clone());
        }
        if n.starts_with('[') || !self.vc_conforms(c, n) {
            return j;
        }
        let n = n.clone();
        ErasedType::L(self.vc_marks.entry(n.clone()).or_insert_with(|| Rc::from(&*n)).clone())
    }

    /// The value class a type is an instance of.
    pub fn value_class_of(&self, t: TypeId) -> Option<ClassId> {
        self.value_class_type(t).map(|(c, _)| c)
    }

    /// The value class a type is an instance of and its type arguments; a type parameter
    /// bounded by one is an instance of it, which erases as the class does.
    pub(super) fn value_class_type(&self, t: TypeId) -> Option<(ClassId, TList)> {
        self.value_class_type_in(t, &mut Vec::new())
    }

    fn value_class_type_in(&self, t: TypeId, seen: &mut Vec<TParamId>) -> Option<(ClassId, TList)> {
        let cx = self.cx;
        let mut t = t;
        loop {
            match cx.input.types.get(t) {
                Type::Class(c, args) if cx.input.syms.class(c).value_class => return Some((c, args)),
                Type::Var(v) => t = cx.input.insts.inst(v)?,
                Type::Alias(a, args) => t = self.alias_expansion(a, args)?,
                Type::Param(p) | Type::AppParam(p, _) if !seen.contains(&p) => {
                    seen.push(p);
                    t = self.param_upper(p);
                }
                // A pattern's binder: the scrutinee's type and the tested one (`AnyVal & Meters`). Of
                // two value classes, the one the intersection erases to (`Gen::erase`).
                Type::Inter(a, b) => {
                    let at = seen.len();
                    let in_a = self.value_class_type_in(a, seen);
                    seen.truncate(at);
                    let in_b = self.value_class_type_in(b, seen);
                    seen.truncate(at);
                    return match (in_a, in_b) {
                        (Some((ca, aa)), Some((cb, ab))) => {
                            let named = |c: ClassId| ErasedType::L(Rc::from(cx.class_names[c.idx()].as_str()));
                            Some(if self.erased_glb_first(&named(ca), &named(cb)) { (ca, aa) } else { (cb, ab) })
                        }
                        (in_a, in_b) => in_a.or(in_b),
                    };
                }
                _ => return None,
            }
        }
    }

    /// The value class the value of `e` is an instance of: its recorded type's, or where the
    /// typer recorded a wider type over the node (an ascription or an upcast, `(v: Any)`,
    /// `v.asInstanceOf[Any]`, which keep no node of their own) the node's own, as dotty's
    /// `Typed` keeps its expression's type for `adaptToType` to box (`Erasure.scala` 594-604).
    pub(super) fn expr_value_class(&self, e: TExprId) -> Option<(ClassId, TList)> {
        if let Some(found) = self.recorded_type(e).and_then(|rt| self.value_class_type(rt)) {
            return Some(found);
        }
        let cx = self.cx;
        let declared = |s: SymId| {
            let info = cx.input.syms.sym(s);
            if info.by_name {
                return None;
            }
            info.sig.as_ref().and_then(|sig| self.value_class_type(sig.ret))
        };
        match cx.input.prog.expr(e) {
            TExpr::New(c, _) if cx.input.syms.class(c).value_class => Some((c, EMPTY_LIST)),
            // `this` in a value class's `m$extension`, the underlying value.
            TExpr::This if self.m.this_underlying.is_some() => self.this_class.filter(|&c| cx.input.syms.class(c).value_class).map(|c| (c, EMPTY_LIST)),
            TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) | TExpr::CallStatic(s, _) | TExpr::CallMethod(_, s, _) => declared(s),
            _ => None,
        }
    }

    /// Whether a value of erased type `t` is the underlying value of a value class that
    /// erases to `u`, rather than its box; the `Object` of the class's own erasure is one where
    /// its arguments make it another reference (`W[String]` is a `String`).
    fn holds_underlying(&self, t: &ErasedType, u: &ErasedType) -> bool {
        // A marked type is a value class's underlying value, whichever mark: the declaration's
        // `Object` of `V[A]` holds `V[U]`'s marked `U`, and the marked bound `U` of
        // `V[A <: U]` holds `V[R]`'s `R`.
        if self.is_vc_mark(u) {
            self.is_vc_mark(t)
        } else {
            t == u || !t.is_ref() || u.is_ref() && self.is_vc_mark(t)
        }
    }

    /// The constructor a value class `c`'s box is made by: its class file's of one parameter, or
    /// the one over the declared underlying value.
    pub(super) fn vc_box_ctor(&mut self, c: ClassId) -> Rc<MethodRef> {
        let name = self.class_name(c);
        let desc = self
            .cx
            .class_files
            .get(&c)
            .and_then(|cf| cf.methods.iter().find(|m| m.name == "<init>" && parse_method_desc(&m.descriptor).0.len() == 1))
            .map(|m| m.descriptor.clone())
            .unwrap_or_else(|| {
                let declared = self.vc_underlying(c);
                method_desc(std::slice::from_ref(&declared), &ErasedType::V)
            });
        let params = parse_method_desc(&desc).0;
        Rc::new(MethodRef { owner: name, owner_is_interface: false, name: "<init>".to_string(), params, ret: ErasedType::V, desc, kind: Invoke::Special })
    }

    /// The box of a value class `c` made over `value`, its underlying value of the type `u`, as
    /// dotty's `Boxing.box` makes it (`Erasure.scala` 272-275 at 3.8.4): `New(tycon, cast(tree,
    /// underlying))`, the value evaluated inside the instance's creation.
    pub(super) fn lower_vc_box(&mut self, c: ClassId, value: LirId, u: &ErasedType) -> LirId {
        let ctor = self.vc_box_ctor(c);
        let param = ctor.params[0].clone();
        let passed = self.lower_adapt(value, u, &param);
        // A class with an outer has the value held first, as master writes it: the value the typer
        // takes from the outer fails before the class is loaded (`Cx::vc_with_outer`).
        if self.cx.vc_with_outer(c) {
            let temp = self.lir.temp();
            let held = self.lir.keep(temp, passed);
            let read = self.lir.kept(temp, param);
            let made = self.lir.make(&ctor, &[read]);
            return self.lir.seq(&[held], made);
        }
        self.lir.make(&ctor, &[passed])
    }

    /// The underlying value, of the type `u`, of `value`, a box of the value class `c`, through the
    /// accessor of its field (`self()`, or the expanded `scala$collection$StringOps$$s()` of a
    /// private one); its zero, reported, where the class has no accessor.
    pub(super) fn lower_vc_unbox(&mut self, c: ClassId, value: LirId, u: &ErasedType) -> LirId {
        let Some(accessor) = self.vc_accessor(c, u) else {
            self.unsupported("the underlying value of a value class without an accessor");
            let none = self.lir.unit();
            return self.lower_adapt(none, &ErasedType::V, u);
        };
        let read = self.lir.call(&accessor, value, &[]);
        self.lower_adapt(read, &accessor.ret, u)
    }

    /// The accessor of a value class `c`'s underlying value: its class file's, else the one of
    /// its parameter's field, which answers `u`.
    fn vc_accessor(&mut self, c: ClassId, u: &ErasedType) -> Option<Rc<MethodRef>> {
        let name = self.class_name(c);
        let accessor = match self.cx.class_files.get(&c) {
            Some(cf) => {
                let field = cf.fields.iter().find(|f| f.access & ACC_STATIC == 0);
                field.and_then(|f| {
                    let desc = format!("(){}", f.descriptor);
                    let expanded = format!("$${}", f.name);
                    cf.methods.iter().find(|m| m.access & ACC_STATIC == 0 && m.descriptor == desc && (m.name == f.name || m.name.ends_with(&expanded))).map(|m| (m.name.clone(), desc))
                })
            }
            None => {
                let syms = self.cx.input.syms;
                let field = syms.class(c).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym);
                field.map(|f| (self.field_name(f), method_desc(&[], u)))
            }
        };
        let (accessor, desc) = accessor?;
        let (params, ret) = parse_method_desc(&desc);
        Some(Rc::new(MethodRef { owner: name, owner_is_interface: false, name: accessor, params, ret, desc, kind: Invoke::Virtual }))
    }

    /// `value`, of the type `from`, brought to the type the symbol `s` is declared with; where that
    /// is a value class, a generic value (a lambda's parameter, a pattern's binding) arrives as its
    /// box, which is opened after a null test (`Gen::lower_value_class`).
    pub(super) fn lower_to_sym(&mut self, value: LirId, from: &ErasedType, s: SymId) -> (LirId, ErasedType) {
        let declared = self.sym_type(s);
        let to = match self.sym_value_class(s) {
            Some((c, args)) => self.lower_value_class(value, from, c, args, true, &declared, None),
            None => self.lower_adapt(value, from, &declared),
        };
        (to, declared)
    }

    /// The value of `s`, stored as `from`, brought to `want`: a value class's underlying value
    /// is boxed where a reference is wanted (a case class's `productElement` of a field of one).
    pub(super) fn lower_adapt_from_sym(&mut self, value: LirId, s: SymId, from: &ErasedType, want: &ErasedType) -> LirId {
        match self.sym_value_class(s) {
            Some((class, args)) => self.lower_value_class(value, from, class, args, true, want, None),
            None => self.lower_adapt(value, from, want),
        }
    }

    /// The value class `s` holds the underlying value of, where it is one and not by name.
    pub(super) fn sym_value_class(&self, s: SymId) -> Option<(ClassId, TList)> {
        let info = self.cx.input.syms.sym(s);
        if info.by_name {
            return None;
        }
        info.sig.as_ref().and_then(|sig| self.value_class_type(sig.ret))
    }

    /// What bringing a value of the value class `c` applied to `args`, left as `t`, to `want` takes:
    /// a boxed one is opened, after a null test where `null_safe` and the value is not statically
    /// the box, as dotty's `Boxing.unbox` opens a tree that is no subtype of the class (`Erasure.scala`
    /// 292-317 at 3.8.4); the underlying value is boxed where a reference other than the class's
    /// erasure is wanted (a type parameter, `Any`, a trait the class extends).
    fn vc_form(&mut self, c: ClassId, args: TList, t: &ErasedType, want: &ErasedType, null_safe: bool) -> VcForm {
        // A value dropped is adapted to nothing (dotty adapts toward a consumer alone,
        // `Erasure.Boxing.adaptToType`): a cast to the class whose value is dropped tests nothing.
        if *want == ErasedType::V {
            return VcForm::Adapt;
        }
        let u = self.vc_erasure(c, args);
        // A reference wanted that the value class is not (a jar descriptor's `Encoder` for
        // circe's `Exported[ConfiguredEncoder[Mode]]`, scalac's erasure of the type the
        // parameter declares) is the underlying value's own.
        let underlying_wanted = u.is_ref() && matches!(want, ErasedType::L(n) if !n.starts_with('[') && !self.is_vc_mark(want) && !want.is_object() && !self.vc_conforms(c, n));
        let boxing = want.is_ref() && !self.holds_underlying(want, &u) && !underlying_wanted;
        match (self.holds_underlying(t, &u), boxing) {
            (true, true) => VcForm::Box(u),
            (true, false) => VcForm::Adapt,
            // A reference that holds the box goes to a destination that takes the box as it is, a
            // null and the reference's identity kept: dotty keeps a tree that conforms (`Erasure.scala`
            // 392-393) and unboxes only toward an `ErasedValueType` (399-400), where the null branch is
            // (292-317).
            (false, true) => VcForm::Adapt,
            (false, false) => VcForm::Open { null_safe: null_safe && *t != self.class_type(c) },
        }
    }

    /// `value`, of the value class `c` applied to `args` and left as `from`, brought to `want` in
    /// dotty's erasure's trees (`Gen::vc_form`): the box a `New` of the class over the underlying
    /// value (`Gen::lower_vc_box`), a box opened by its accessor (`Gen::lower_vc_unbox`), and with
    /// the null test, `Gen::lower_vc_open_null_safe`. `of` is the expression of the typed IR
    /// `value` was lowered from, where there is one.
    pub(super) fn lower_value_class(&mut self, value: LirId, from: &ErasedType, c: ClassId, args: TList, null_safe: bool, want: &ErasedType, of: Option<TExprId>) -> LirId {
        match self.vc_form(c, args, from, want, null_safe) {
            VcForm::Adapt => self.lower_adapt(value, from, want),
            VcForm::Box(u) => {
                let underlying = self.lower_adapt(value, from, &u);
                let made = self.lower_vc_box(c, underlying, &u);
                let boxed = self.class_type(c);
                self.lower_adapt(made, &boxed, want)
            }
            VcForm::Open { null_safe } => {
                let declared = self.vc_underlying(c);
                let opened = if null_safe {
                    self.lower_vc_open_null_safe(value, from, c, &declared, of)
                } else {
                    let boxed = self.class_type(c);
                    let cast = self.lower_adapt(value, from, &boxed);
                    self.lower_vc_unbox(c, cast, &declared)
                };
                self.lower_adapt(opened, &declared, want)
            }
        }
    }

    /// The underlying value, of the type `u`, of `value`, a reference of the type `from` that may be
    /// null, as dotty's `Boxing.unbox` gives it (`Erasure.scala` 302-314 at 3.8.4): a tree of type `Null`
    /// unboxed as it is (`adaptToType(null, underlying)`), else `evalOnce(tree) { t => If(t eq null,
    /// unboxed null, t.u) }`, the value read again where it is idempotent (`Gen::lower_reread`) and
    /// held in a temporary otherwise. A null unboxed is `BoxesRunTime`'s `unboxTo*` of it for a
    /// primitive, and for a reference the null itself, which reaches the test's join as the
    /// underlying type (a widening that writes nothing: the frame's type there).
    pub(super) fn lower_vc_open_null_safe(&mut self, value: LirId, from: &ErasedType, c: ClassId, u: &ErasedType, of: Option<TExprId>) -> LirId {
        let object = ErasedType::object();
        let null_unboxed = |g: &mut Self| {
            if u.is_ref() {
                let null = g.lir.push(Kind::Null, u.clone());
                return g.lir.push(Kind::Adapt { value: null, from: u.clone(), how: Adaptation::Widen }, u.clone());
            }
            let null = g.lir.push(Kind::Null, object.clone());
            g.lower_adapt(null, &object, u)
        };
        // The null, the literal or one an ascription holds (`Kind::Typed`, dotty's `Typed` over the literal): the
        // tree erasure adapts is of type `Null` either way.
        let null_typed = match &self.lir.node(value).kind {
            Kind::Null => true,
            Kind::Typed(inner) => matches!(self.lir.node(*inner).kind, Kind::Null),
            _ => false,
        };
        if null_typed {
            return null_unboxed(self);
        }
        let (held, tested, opened) = match self.lower_reread(value, of) {
            Some(again) => (None, value, again),
            None => {
                let temp = self.lir.temp();
                let held = self.lir.keep(temp, value);
                (Some(held), self.lir.kept(temp, from.clone()), self.lir.kept(temp, from.clone()))
            }
        };
        let null = self.lir.push(Kind::Null, object.clone());
        let cond = self.lir.compare(Relation::Eq, Comparing::Identity, tested, null);
        let then = null_unboxed(self);
        let boxed = self.class_type(c);
        let cast = self.lower_adapt(opened, from, &boxed);
        let otherwise = self.lower_vc_unbox(c, cast, u);
        let test = self.lir.push(Kind::If { cond, then, otherwise: Some(otherwise) }, u.clone());
        self.m.null_tests.push(test);
        let Some(held) = held else { return test };
        let block = self.lir.seq(&[held], test);
        self.m.null_tests.push(block);
        block
    }

    /// A second read of `value`, where dotty's `evalOnce` leaves its operand in place
    /// (`isIdempotentExpr`). A cast to the value class is the opening itself, not its operand
    /// (`a.asInstanceOf[V]` unboxes `a`): its operand is read again under it; any other read
    /// is `Gen::lower_read_again`'s.
    fn lower_reread(&mut self, value: LirId, of: Option<TExprId>) -> Option<LirId> {
        let node = self.lir.node(value);
        let (kind, ty) = (node.kind.clone(), node.ty.clone());
        let cast_of = of.and_then(|e| match self.cx.input.prog.expr(e) {
            TExpr::Cast(inner, ..) => Some(inner),
            _ => None,
        });
        match (kind, cast_of) {
            (Kind::Cast { value: inner, op }, Some(inner_of)) => {
                let again = self.lower_read_again(inner, Some(inner_of))?;
                Some(self.lir.push(Kind::Cast { value: again, op }, ty))
            }
            _ => self.lower_read_again(value, of),
        }
    }

    /// The one function on nodes for idempotence: a read idempotent by
    /// what it is (a parameter, a temporary, a local val, `this`, a literal) read again, else the
    /// expression `of` of the typed IR where the typer marks it idempotent
    /// (`Program::in_place_operands`), lowered again. A read the source wrote a cast around, which
    /// erasure dropped (`Program::is_cast`), is the typer's to answer: a written cast is impure.
    fn lower_read_again(&mut self, value: LirId, of: Option<TExprId>) -> Option<LirId> {
        let prog = self.cx.input.prog;
        let in_place = |e: TExprId| prog.operand_in_place(e);
        if of.is_some_and(|e| prog.is_cast(e)) {
            return of.filter(|&e| in_place(e)).map(|e| self.lower_node(e));
        }
        let node = self.lir.node(value);
        let (kind, ty) = (node.kind.clone(), node.ty.clone());
        let syms = self.cx.input.syms;
        match kind {
            Kind::Param(_) | Kind::Temp(_) | Kind::This | Kind::Null | Kind::Int(_) | Kind::Long(_) | Kind::Float(_) | Kind::Double(_) | Kind::Bool(_) | Kind::Char(_) | Kind::Str(_) | Kind::Text(_) => {
                Some(self.lir.push(kind, ty))
            }
            Kind::Local(s) if syms.sym(s).kind != SymKind::Var && !self.m.cells.contains(&s) => Some(self.lir.push(kind, ty)),
            // `scala.TupleN`'s own `_i` (a stable member) on a receiver read again, as a pattern definition's
            // variables read their tuple (`$1._i`, `Desugar.scala` 1650-1654 at 3.8.4): `exprPurity`'s `Select`, the
            // member's `refPurity` (a stable val, `PurePath`) under the qualifier's (`TreeInfo.scala` 597-599 and
            // 684-697 at 3.8.4), idempotent where the qualifier is (`isIdempotentExpr`, 649-650).
            Kind::Call { method, receiver, args, order: None, at: None } if args.len == 0 && is_tuple_accessor(&method) => match self.lower_read_again(receiver, None) {
                Some(again) => Some(self.lir.call(&method, again, &[])),
                None => of.filter(|&e| in_place(e)).map(|e| self.lower_node(e)),
            },
            _ => of.filter(|&e| in_place(e)).map(|e| self.lower_node(e)),
        }
    }

    /// Whether the value class `c` itself is a `name`: its own class or one of its parents.
    fn vc_conforms(&mut self, c: ClassId, name: &str) -> bool {
        if &*self.class_name(c) == name {
            return true;
        }
        let bases: Vec<ClassId> = self.cx.input.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        bases.into_iter().any(|b| &*self.class_name(b) == name)
    }

    /// The type of a local, a field or a parameter: `Unit` is a reference there.
    pub fn storage(&mut self, t: TypeId) -> ErasedType {
        match self.erase(t) {
            ErasedType::V => ErasedType::L(Rc::from(BOXED_UNIT)),
            j => j,
        }
    }

    /// The type a symbol's value has where it is stored.
    pub fn sym_type(&mut self, s: SymId) -> ErasedType {
        // A given object of an instance is read as scalac's inner module class.
        if let Some(&k) = self.cx.inner_given_objects.get(&s) {
            return self.class_type(k);
        }
        let info = self.cx.input.syms.sym(s);
        if info.by_name {
            return self.function_type(0);
        }
        match &info.sig {
            // The type as declared where the erasure reads more of it (`MethodSig::declared`).
            Some(sig) if sig.declared != NO_TYPE => {
                let declared = self.declared_type(sig.declared);
                self.in_signature(|g| g.storage(declared))
            }
            Some(sig) => self.in_signature(|g| g.storage(sig.ret)),
            None => ErasedType::object(),
        }
    }

    pub fn vt(&mut self, t: &ErasedType) -> VT {
        match t {
            ErasedType::V => VT::Top,
            ErasedType::Z | ErasedType::B | ErasedType::S | ErasedType::C | ErasedType::I => VT::Int,
            ErasedType::J => VT::Long,
            ErasedType::F => VT::Float,
            ErasedType::D => VT::Double,
            ErasedType::L(n) => VT::Object(self.cw.cp.class(n)),
        }
    }

    // ---- names and references ----

    pub fn method_name(&mut self, s: SymId) -> String {
        if let Some(target) = self.target_name(s) {
            return target;
        }
        self.source_method_name(s)
    }

    /// The name of method `s` in the class files as its source name gives it, `@targetName`
    /// apart: what the names of its default getters begin with (`$plus$default$1` of
    /// `@targetName("renamed") def +(x: Int = 1)`), as scalac names them.
    pub fn source_method_name(&mut self, s: SymId) -> String {
        let syms = self.cx.input.syms;
        let info = syms.sym(s);
        // A jar's member called already: the name its class file gives it, its `@targetName`
        // aside.
        if self.linked_owner(s).is_some() && self.target_name(s).is_none() {
            if let Some(m) = self.mrefs.get(&s) {
                return m.name.clone();
            }
        }
        let mut name = encode(self.cx.input.interner.get(info.name));
        // Extensions of one name overload by their erased parameters, as scalac's do; only those
        // that erase alike (which scalac refuses as a double definition) are named apart, `m$x1`.
        if info.is_extension {
            let siblings: &[SymId] = match info.owner {
                Owner::Class(c) => &syms.class(c).extensions,
                Owner::Package(p) => syms.package_object_entry(p, info.name).map_or(&[], |e| &e.extensions),
                Owner::Local => &[],
            };
            let mine = self.erased_param_types(s);
            let mut same: Vec<SymId> = siblings.iter().copied().filter(|&e| syms.sym(e).name == info.name && syms.sym(e).file == info.file).collect();
            same.retain(|&e| e == s || self.erased_param_types(e) == mine);
            same.sort_by(|&a, &b| crate::emit::layout::compare_syms(syms, self.cx.input.interner, a, b));
            let member_too = match info.owner {
                Owner::Class(c) => syms.class(c).members.get(&info.name).is_some_and(|&m| {
                    let single = [m];
                    let alts: Vec<SymId> = syms.alternatives(m).unwrap_or(&single).to_vec();
                    alts.iter().any(|&a| syms.sym(a).kind == SymKind::Def && self.erased_param_types(a) == mine)
                }),
                _ => false,
            };
            if let Some(rank) = same.iter().position(|&e| e == s) {
                if rank > 0 || member_too {
                    name.push_str(&format!("$x{}", rank + member_too as usize));
                }
            }
        }
        self.expanded_private_name(s, name)
    }

    /// scalac's expanded name `Class$$name` for a private member whose name a class above or
    /// below uses as well.
    fn expanded_private_name(&self, s: SymId, name: String) -> String {
        let syms = self.cx.input.syms;
        let info = syms.sym(s);
        match info.owner {
            Owner::Class(c)
                if info.mods & mods::PRIVATE != 0
                    && !info.scoped_private
                    && (syms.private_name_clashes(c, info.name) || (self.cx.is_interface(c) && matches!(info.kind, SymKind::Val | SymKind::Var))) =>
            {
                format!("{}$${}", self.cx.class_names[c.idx()].replace('/', "$"), name)
            }
            _ => name,
        }
    }

    /// `@targetName("name")`: the name of the method in the class file, as under scalac, its
    /// bridges' and forwarders' too.
    pub fn target_name(&self, s: SymId) -> Option<String> {
        if let Some(&n) = self.cx.input.target_names.and_then(|m| m.get(&s)).or_else(|| self.cx.input.product_target_names.and_then(|m| m.get(&s))) {
            return Some(encode(self.cx.input.interner.get(n)));
        }
        let &n = self.cx.input.source_target_names?.get(&s)?;
        Some(encode(self.cx.input.interner.get(n)))
    }

    /// The flattened parameters of a def: by-name ones are functions, repeated ones sequences.
    pub fn param_types(&mut self, s: SymId) -> Vec<ErasedType> {
        let cx = self.cx;
        if let Some((_, JMember::Method(_))) = cx.input.java_member(s) {
            return self.mref(s).params.clone();
        }
        if self.linked_owner(s).is_some() {
            return self.mref(s).params.clone();
        }
        self.erased_param_types(s)
    }

    /// A parameter's type as the signature spells it where that is a `*:` chain, which the
    /// body reads as the tuple class of its elements (`ParamSig::ty`). In the order the class
    /// file takes them (`jvm_param_order`).
    fn erased_param_types(&mut self, s: SymId) -> Vec<ErasedType> {
        self.in_language_of(s, |g| g.erased_params_of(s))
    }

    fn erased_params_of(&mut self, s: SymId) -> Vec<ErasedType> {
        let cx = self.cx;
        let Some(sig) = cx.input.syms.sym(s).info.sig.as_ref() else { return Vec::new() };
        let params: Vec<(SymId, Option<TypeId>)> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| (p.sym, (!p.by_name && !p.repeated).then_some(p.ty)))).collect();
        let types: Vec<ErasedType> = params
            .into_iter()
            .map(|(p, declared)| match declared {
                Some(_) if cx.input.syms.sym(p).sig.as_ref().is_some_and(|sig| sig.declared != NO_TYPE) => self.sym_type(p),
                Some(t) if self.is_cons_chain(t) => self.storage(t),
                _ => self.sym_type(p),
            })
            .collect();
        match self.jvm_param_order(s) {
            Some(order) => order.iter().map(|&k| types[k].clone()).collect(),
            None => types,
        }
    }

    /// The parameters of a right-associative extension method in the order its class file takes
    /// them, as scalac's `paramss` hold them: the method's first explicit clause before the
    /// receiver's (`$plus$colon(String, int)` of `extension (x: Int) def +:(s: String)`), each
    /// by its index in the signature's flattened order, which puts the receiver first. None for
    /// every other method, whose order is the signature's.
    pub fn jvm_param_order(&self, s: SymId) -> Option<Vec<usize>> {
        let cx = self.cx;
        let clauses = self.declared_clauses(s)?;
        let sig = cx.input.syms.sym(s).info.sig.as_ref()?;
        let starts: Vec<usize> = sig.clauses.iter().scan(0, |acc, c| {
            let at = *acc;
            *acc += c.params.len();
            Some(at)
        }).collect();
        let order: Vec<usize> = clauses.iter().flat_map(|&c| starts[c]..starts[c] + sig.clauses[c].params.len()).collect();
        (order.iter().enumerate().any(|(j, &k)| j != k)).then_some(order)
    }

    /// The clauses of a right-associative extension method as scalac's `paramss` hold them, by
    /// their indices in the signature: the method's first explicit clause before the receiver's.
    /// None for every other method.
    fn declared_clauses(&self, s: SymId) -> Option<Vec<usize>> {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        if !info.is_extension || !cx.input.interner.get(info.name).ends_with(':') {
            return None;
        }
        info.sig.as_ref()?.right_assoc_order(info.ext_clauses as usize)
    }

    /// Whether `t` is a `*:` chain or an array of one.
    fn is_cons_chain(&self, t: TypeId) -> bool {
        let types = self.cx.input.types;
        match types.get(t) {
            Type::Class(c, _) if scala_class(self.cx, c) == Some("*:") => true,
            Type::Class(c, args) if self.cx.input.syms.class(c).kind == ClassKind::Builtin && self.cx.input.interner.get(self.cx.input.syms.class(c).name) == "Array" => {
                types.items(args).first().map_or(false, |&e| self.is_cons_chain(e))
            }
            _ => false,
        }
    }

    pub fn return_type(&mut self, s: SymId) -> ErasedType {
        let cx = self.cx;
        if let Some((_, JMember::Method(_))) = cx.input.java_member(s) {
            let ret = self.mref(s).ret.clone();
            return self.java_value_type(ret);
        }
        if self.linked_owner(s).is_some() {
            return self.mref(s).ret.clone();
        }
        self.erased_return_type(s)
    }

    fn erased_return_type(&mut self, s: SymId) -> ErasedType {
        self.in_language_of(s, |g| g.erased_result_of(s))
    }

    fn erased_result_of(&mut self, s: SymId) -> ErasedType {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        let Some(sig) = info.sig.as_ref() else { return ErasedType::object() };
        let ret = sig.ret;
        // `x.type` of a parameter the signature spells as a `*:` chain is the chain's.
        let spelled = match cx.input.types.get(ret) {
            Type::Term(x) => sig.clauses.iter().flat_map(|c| c.params.iter()).find(|p| p.sym == x && !p.by_name && !p.repeated).map(|p| p.ty),
            _ => None,
        };
        if let Some(t) = spelled.filter(|&t| self.is_cons_chain(t)) {
            return self.storage(t);
        }
        if sig.declared != NO_TYPE {
            let declared = self.declared_type(sig.declared);
            return self.erase(declared);
        }
        self.erase(ret)
    }

    pub fn mref(&mut self, s: SymId) -> Rc<MethodRef> {
        if let Some(m) = self.mrefs.get(&s) {
            let m = m.clone();
            if let Some(&used) = self.mref_arities.get(&s) {
                self.function_arities |= used;
            }
            return m;
        }
        let before = std::mem::take(&mut self.function_arities);
        let m = self.make_mref(s);
        self.keep_arities(before, |g, used| g.mref_arities.insert(s, used));
        m
    }

    fn make_mref(&mut self, s: SymId) -> Rc<MethodRef> {
        let cx = self.cx;
        // A Java member is called as its class file declares it: the descriptor is exact.
        if let Some((cf, JMember::Method(i))) = cx.input.java_member(s) {
            let jm = &cf.methods[i as usize];
            let (params, ret) = parse_method_desc(&jm.descriptor);
            let kind = if jm.access & ACC_STATIC != 0 {
                Invoke::Static
            } else if cf.is_interface() {
                Invoke::Interface
            } else {
                Invoke::Virtual
            };
            let m = Rc::new(MethodRef {
                owner: Rc::from(cf.name.as_str()),
                owner_is_interface: cf.is_interface(),
                name: jm.name.clone(),
                params,
                ret,
                desc: jm.descriptor.clone(),
                kind,
            });
            self.mrefs.insert(s, m.clone());
            return m;
        }
        let info = cx.input.syms.sym(s);
        let params = self.erased_param_types(s);
        let is_value = !matches!(info.kind, SymKind::Def) && !(info.kind == SymKind::Given && !params.is_empty());
        let ret = if is_value { self.sym_type(s) } else { self.erased_return_type(s) };
        if let Some(m) = self.linked_mref(s, is_value, &params, &ret) {
            let m = Rc::new(m);
            self.mrefs.insert(s, m.clone());
            return m;
        }
        // `@jvmWide`: a def of the standard library whose result is not always what its type
        // says (`Map.map` reached through `Iterable` gives a `List` for a function without pairs).
        let wide = info.def.map_or(false, |d| cx.input.asts[info.file.0 as usize].def(d).annots.iter().any(|a| a.name == crate::names::JVM_WIDE));
        let ret = if wide && ret.is_ref() { ErasedType::object() } else { ret };
        let (owner, owner_is_interface, kind): (Rc<str>, bool, Invoke) = match info.owner {
            Owner::Class(c) if cx.is_interface(c) => (self.class_name(c), true, Invoke::Interface),
            // An interface's static is an `InterfaceMethodref` (`Function.identity`).
            Owner::Class(c) if cx.jdk_statics(c) => {
                let interface = cx.input.syms.class(c).companion.map_or(false, |t| cx.is_interface(t));
                (self.class_name(c), interface, Invoke::Static)
            }
            Owner::Class(c) => (self.class_name(c), false, Invoke::Virtual),
            _ => (Rc::from(cx.file_modules[info.file.0 as usize].as_str()), false, Invoke::Virtual),
        };
        let name = if is_value { self.field_name(s) } else { self.method_name(s) };
        // A given class's def answers its class, as scalac's (`Cx::given_classes`).
        let ret = match cx.given_classes.get(&s) {
            Some(&k) => self.class_type(k),
            None => ret,
        };
        let desc = method_desc(&params, &ret);
        let m = Rc::new(MethodRef { owner, owner_is_interface, name, params, ret, desc, kind });
        self.mrefs.insert(s, m.clone());
        m
    }

    /// In link mode, the class file of the jar class that declares `s`.
    fn linked_owner(&self, s: SymId) -> Option<&'a crate::classfile::ClassFile> {
        let cx: &'a Cx<'a> = self.cx;
        match cx.input.syms.sym(s).owner {
            Owner::Class(c) => cx.class_files.get(&c).map(|cf| &**cf),
            _ => None,
        }
    }

    /// A member of a jar class is called as its class file declares it, as a Java member is:
    /// the method of its name and arity whose parameters match teq's erasure best. Where the
    /// descriptor's types are not the erasure's, the call's arguments and result adapt to them.
    fn linked_mref(&mut self, s: SymId, is_value: bool, params: &[ErasedType], ret: &ErasedType) -> Option<MethodRef> {
        let cf = self.linked_owner(s)?;
        let cx = self.cx;
        let name = self.target_name(s).unwrap_or_else(|| encode(cx.input.interner.get(cx.input.syms.sym(s).name)).to_string());
        let arity = if is_value { 0 } else { params.len() };
        let own = method_desc(if is_value { &[] } else { params }, ret);
        // scalac mangles an accessor of an expanded name with the prefix inside the expansion:
        // TASTy's `inline$p$$x` is the class file's `p$$inline$x` (pureconfig's accessor of a
        // trait parameter).
        let mut name = name;
        let mut picked = pick_method(cf, &name, arity, &own);
        // teq's class file names an extension's overloads apart, `m`, `m$x1`, ..
        // (`method_name`): the one whose parameters are the call's.
        if cx.input.syms.sym(s).is_extension && picked.map_or(true, |m| m.descriptor != own) {
            let suffixed = cf.methods.iter().find(|m| m.descriptor == own && m.name.strip_prefix(name.as_str()).and_then(|r| r.strip_prefix("$x")).map_or(false, |k| !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit())));
            if let Some(m) = suffixed {
                name = m.name.clone();
                picked = Some(m);
            }
        }
        if picked.is_none() {
            if let Some(mangled) = name.strip_prefix("inline$").and_then(|r| r.rsplit_once("$$")).map(|(p, x)| format!("{}$$inline${}", p, x)) {
                picked = pick_method(cf, &mangled, arity, &own);
                if picked.is_some() {
                    name = mangled;
                }
            }
        }
        // A private member an inline body reaches goes through the `inline$m` accessor scalac
        // made for it, the class file's public method (magnolia's `returningNone`).
        let (m, name) = match picked {
            Some(m) if m.access & ACC_PRIVATE == 0 => (m, name),
            _ => {
                let accessor = format!("inline${}", name);
                match pick_method(cf, &accessor, arity, &own) {
                    Some(a) => (a, accessor),
                    None => (picked?, name),
                }
            }
        };
        let (desc_params, desc_ret) = parse_method_desc(&m.descriptor);
        // An `Object` the erasure marks as a value class's underlying value keeps the mark, also
        // where the descriptor names the instantiated underlying type's erasure (`Encoder` for
        // kantan's `Exported[CellEncoder[LocalDate]]`): a jar passes the value class as its
        // underlying value wherever its signature names the class.
        let retag = |d: ErasedType, o: Option<&ErasedType>| match o {
            Some(o) if *o == d || (self.is_vc_object(o) && d.is_ref()) => o.clone(),
            _ => d,
        };
        let params = desc_params.into_iter().enumerate().map(|(i, d)| retag(d, params.get(i))).collect();
        let ret = retag(desc_ret, Some(ret));
        Some(MethodRef {
            owner: Rc::from(cf.name.as_str()),
            owner_is_interface: cf.is_interface(),
            name,
            params,
            ret,
            desc: m.descriptor.clone(),
            kind: if cf.is_interface() { Invoke::Interface } else { Invoke::Virtual },
        })
    }

    /// The name of the field that holds a member and of its accessor. A trait's parameter is
    /// scalac's: a `val` its own name, a private one expanded (`T$$x`, as `Mixin` makes a
    /// trait's private members public); an anonymous using parameter's is the `x$N` the parser
    /// names it, scalac's.
    pub fn field_name(&mut self, s: SymId) -> String {
        let cx = self.cx;
        // A value's `@targetName` names its field as it names its getter.
        let base = match self.target_name(s) {
            Some(target) if cx.input.syms.sym(s).kind == SymKind::Val => target,
            _ => encode(cx.input.interner.get(cx.input.syms.sym(s).name)),
        };
        self.expanded_private_name(s, base)
    }

    // ---- instructions that keep the model in step ----

    pub fn iconst(&mut self, v: i32) {
        match v {
            -1..=5 => self.code.op((op::ICONST_0 as i32 + v) as u8),
            -128..=127 => self.code.op_u8(op::BIPUSH, v as u8),
            -32768..=32767 => self.code.op_u16(op::SIPUSH, v as u16),
            _ => {
                let i = self.cw.cp.int(v);
                self.ldc(i);
            }
        }
        self.code.push(VT::Int);
    }

    pub(super) fn ldc(&mut self, index: u16) {
        if index <= 255 {
            self.code.op_u8(op::LDC, index as u8);
        } else {
            self.code.op_u16(op::LDC_W, index);
        }
    }

    pub fn lconst(&mut self, v: i64) {
        if v == 0 || v == 1 {
            self.code.op(op::LCONST_0 + v as u8);
        } else {
            let i = self.cw.cp.long(v);
            self.code.op_u16(op::LDC2_W, i);
        }
        self.code.push(VT::Long);
    }

    pub fn dconst(&mut self, v: f64) {
        if v.to_bits() == 0f64.to_bits() || v == 1.0 {
            self.code.op(op::DCONST_0 + v as u8);
        } else {
            let i = self.cw.cp.double(v);
            self.code.op_u16(op::LDC2_W, i);
        }
        self.code.push(VT::Double);
    }

    pub fn fconst(&mut self, v: f32) {
        if v.to_bits() == 0f32.to_bits() || v == 1.0 || v == 2.0 {
            self.code.op(op::FCONST_0 + v as u8);
        } else {
            let i = self.cw.cp.float(v);
            self.ldc(i);
        }
        self.code.push(VT::Float);
    }

    pub fn sconst(&mut self, s: &str) {
        let i = self.cw.cp.string(s);
        self.ldc(i);
        let t = VT::Object(self.cw.cp.class(STRING));
        self.code.push(t);
    }

    pub fn load(&mut self, slot: u16, t: &ErasedType) {
        let (base, short) = match t {
            ErasedType::Z | ErasedType::B | ErasedType::S | ErasedType::C | ErasedType::I => (op::ILOAD, op::ILOAD_0),
            ErasedType::J => (op::LLOAD, op::LLOAD_0),
            ErasedType::F => (op::FLOAD, op::FLOAD_0),
            ErasedType::D => (op::DLOAD, op::DLOAD_0),
            ErasedType::L(_) => (op::ALOAD, op::ALOAD_0),
            ErasedType::V => return,
        };
        self.code.var_insn(base, short, slot);
        let vt = match self.code.locals.get(slot as usize) {
            Some(&v @ (VT::UninitThis | VT::Uninit(_))) => v,
            _ => self.vt(t),
        };
        self.code.push(vt);
    }

    pub fn store(&mut self, slot: u16, t: &ErasedType) {
        let (base, short) = match t {
            ErasedType::Z | ErasedType::B | ErasedType::S | ErasedType::C | ErasedType::I => (op::ISTORE, op::ISTORE_0),
            ErasedType::J => (op::LSTORE, op::LSTORE_0),
            ErasedType::F => (op::FSTORE, op::FSTORE_0),
            ErasedType::D => (op::DSTORE, op::DSTORE_0),
            ErasedType::L(_) => (op::ASTORE, op::ASTORE_0),
            ErasedType::V => return,
        };
        self.code.var_insn(base, short, slot);
        self.code.pop();
    }

    /// A fresh slot holding the value on the stack.
    pub fn store_new(&mut self, t: &ErasedType) -> u16 {
        let vt = self.vt(t);
        let slot = self.code.new_local(vt);
        self.store(slot, t);
        slot
    }

    pub fn pop_value(&mut self, t: &ErasedType) {
        match t {
            ErasedType::V => {}
            ErasedType::J | ErasedType::D => {
                self.code.op(op::POP2);
                self.code.pop();
            }
            _ => {
                self.code.op(op::POP);
                self.code.pop();
            }
        }
    }

    pub fn dup(&mut self) {
        let top = *self.code.stack.last().expect("dup of an empty stack");
        self.code.op(if matches!(top, VT::Long | VT::Double) { op::DUP2 } else { op::DUP });
        self.code.push(top);
    }

    /// Whether `c` is a Java-defined interface: one read from a class file, or one the std
    /// declares in a `java` package for the JDK's (`Cx::java_defined` of its members).
    /// The place of the super call being written, by which a class's targets are ordered: the
    /// order in which scalac's back end writes a class's bodies and so fills `superCallTargets`.
    /// First the members made for the class, a trait's super accessor among them
    /// (`ResolveSuper.transformTemplate` and `Mixin.transformTemplate` put them before the
    /// class's own), in the order written; then the primary constructor's code
    /// (`BCodeSkelBuilder.gen`: the constructor, then the body), the field initialisers and the
    /// body's statements; then the other members by their place in the source; last the
    /// lifted bodies, which `LambdaLift.transformTemplate` appends to the class, an inner one
    /// before the one around it: by the end of the body, the later start first. Inside one
    /// body the calls stand by their line and then by the call's end: a call in another's
    /// argument is written before it.
    fn super_call_place(&self) -> SuperPlace {
        let at = self.super_call_at.unwrap_or(u32::MAX);
        match (self.lifted_body, self.code.current_line()) {
            (Some((end, start)), _) => (3, end, u32::MAX - start, at),
            (None, None) => (0, 0, 0, 0),
            (None, Some(line)) => (if self.ctor_code { 1 } else { 2 }, u32::from(line), at, 0),
        }
    }

    pub(super) fn java_interface(&self, c: ClassId) -> bool {
        let cx = self.cx;
        let syms = cx.input.syms;
        let info = syms.class(c);
        if info.kind != ClassKind::Trait {
            return false;
        }
        if cx.input.java_class(c).is_some() {
            return true;
        }
        if !cx.input.sources.get(info.file.0 as usize).is_some_and(|f| f.is_std) {
            return false;
        }
        let mut owner = info.owner;
        while let Owner::Class(o) = owner {
            owner = syms.class(o).owner;
        }
        let Owner::Package(mut p) = owner else { return false };
        while let Some(parent) = syms.pkg(p).parent.filter(|&q| q != crate::symbols::ROOT_PKG) {
            p = parent;
        }
        syms.pkg(p).parent.is_some() && syms.pkg(p).name == crate::names::JAVA
    }

    pub fn invoke(&mut self, kind: Invoke, owner: &str, owner_is_interface: bool, name: &str, params: &[ErasedType], ret: &ErasedType) {
        let desc = method_desc(params, ret);
        self.invoke_desc(kind, owner, owner_is_interface, name, &desc, params.len(), ret);
    }

    pub fn invoke_desc(&mut self, kind: Invoke, owner: &str, owner_is_interface: bool, name: &str, desc: &str, n_params: usize, ret: &ErasedType) {
        // An `invokespecial` of an interface's method links only where the interface is a direct
        // superinterface of the class: every one written is recorded for the class's list
        // (`BCodeBodyBuilder.genCallMethod` adds to `superCallTargets`). A trait's call of its
        // own method names the trait itself, which is no interface of its own.
        if owner_is_interface && matches!(kind, Invoke::Special) && owner != &*self.this_name {
            let place = self.super_call_place();
            match self.super_targets.iter_mut().find(|(t, _)| t == owner) {
                Some((_, first)) => *first = (*first).min(place),
                None => self.super_targets.push((owner.to_string(), place)),
            }
        }
        let index = if owner_is_interface {
            self.cw.cp.interface_method(owner, name, desc)
        } else {
            self.cw.cp.method(owner, name, desc)
        };
        match kind {
            Invoke::Virtual => self.code.op_u16(op::INVOKEVIRTUAL, index),
            Invoke::Static => self.code.op_u16(op::INVOKESTATIC, index),
            Invoke::Special => self.code.op_u16(op::INVOKESPECIAL, index),
            Invoke::Interface => {
                self.code.op_u16(op::INVOKEINTERFACE, index);
                let words: usize = 1 + parse_method_desc(desc).0.iter().map(|p| if p.wide() { 2 } else { 1 }).sum::<usize>();
                self.code.bytes.push(words as u8);
                self.code.bytes.push(0);
            }
        }
        self.code.popn(n_params);
        if kind != Invoke::Static {
            let receiver = self.code.pop();
            if name == "<init>" {
                let initialised = VT::Object(self.cw.cp.class(owner));
                let this_type = VT::Object(self.cw.cp.class(&self.this_name.clone()));
                for t in self.code.stack.iter_mut().chain(self.code.locals.iter_mut()) {
                    if *t == receiver {
                        *t = if receiver == VT::UninitThis { this_type } else { initialised };
                    }
                }
            }
        }
        if *ret != ErasedType::V {
            let vt = self.vt(ret);
            self.code.push(vt);
        }
    }

    pub fn invoke_mref(&mut self, m: &MethodRef) {
        self.invoke_desc(m.kind, &m.owner, m.owner_is_interface, &m.name, &m.desc, m.params.len(), &m.ret);
    }

    pub fn new_object(&mut self, class: &str) {
        let offset = self.code.pc() as u16;
        let index = self.cw.cp.class(class);
        self.code.op_u16(op::NEW, index);
        self.code.push(VT::Uninit(offset));
        self.dup();
    }

    pub fn getstatic(&mut self, owner: &str, name: &str, t: &ErasedType) {
        let mut d = String::new();
        t.desc(&mut d);
        let f = self.cw.cp.field(owner, name, &d);
        self.code.op_u16(op::GETSTATIC, f);
        let vt = self.vt(t);
        self.code.push(vt);
    }

    pub fn putstatic(&mut self, owner: &str, name: &str, t: &ErasedType) {
        let mut d = String::new();
        t.desc(&mut d);
        let f = self.cw.cp.field(owner, name, &d);
        self.code.op_u16(op::PUTSTATIC, f);
        self.code.pop();
    }

    pub fn getfield(&mut self, owner: &str, name: &str, t: &ErasedType) {
        let mut d = String::new();
        t.desc(&mut d);
        let f = self.cw.cp.field(owner, name, &d);
        self.code.op_u16(op::GETFIELD, f);
        self.code.pop();
        let vt = self.vt(t);
        self.code.push(vt);
    }

    pub fn putfield(&mut self, owner: &str, name: &str, t: &ErasedType) {
        let mut d = String::new();
        t.desc(&mut d);
        let f = self.cw.cp.field(owner, name, &d);
        self.code.op_u16(op::PUTFIELD, f);
        self.code.popn(2);
    }

    pub fn checkcast(&mut self, class: &str) {
        let index = self.cw.cp.class(class);
        self.code.op_u16(op::CHECKCAST, index);
        self.code.pop();
        self.code.push(VT::Object(index));
    }

    pub fn instance_of(&mut self, class: &str) {
        let index = self.cw.cp.class(class);
        self.code.op_u16(op::INSTANCEOF, index);
        self.code.pop();
        self.code.push(VT::Int);
    }

    /// A conditional jump that pops `pops` operands.
    pub fn jump_if(&mut self, opcode: u8, pops: usize, target: Label) {
        self.code.reach();
        self.code.popn(pops);
        self.code.jump(opcode, target);
    }

    pub fn goto(&mut self, target: Label) {
        self.code.jump(op::GOTO, target);
    }

    pub fn return_value(&mut self, t: &ErasedType) {
        let opcode = match t {
            ErasedType::V => op::RETURN,
            ErasedType::Z | ErasedType::B | ErasedType::S | ErasedType::C | ErasedType::I => op::IRETURN,
            ErasedType::J => op::LRETURN,
            ErasedType::F => op::FRETURN,
            ErasedType::D => op::DRETURN,
            ErasedType::L(_) => op::ARETURN,
        };
        self.code.op(opcode);
        if *t != ErasedType::V {
            self.code.pop();
        }
        self.code.end_path();
    }

    pub fn unit_value(&mut self) {
        let t = ErasedType::L(Rc::from(BOXED_UNIT));
        self.getstatic(BOXED_UNIT, "UNIT", &t);
    }

    // ---- adaptation ----

    /// Whether the verifier takes a `from` for a `to` without a cast: an interface is not
    /// assignable to a class, whatever superclass its trait has.
    pub(super) fn is_subclass(&self, from: &str, to: &str) -> bool {
        let cx = self.cx;
        let by_name = if cx.input.open_world { &cx.all_by_name } else { &cx.class_by_name };
        let (Some(&f), Some(&t)) = (by_name.get(from), by_name.get(to)) else { return false };
        let syms = cx.input.syms;
        if syms.class(f).kind == ClassKind::Trait && syms.class(t).kind != ClassKind::Trait {
            return false;
        }
        // A given object of an instance implements its given's declared type, which a build over
        // its products, which may not see its class's parents, reads as this one does.
        if let Some(&g) = cx.inner_given_object_classes.get(&f) {
            let declared = syms.sym(g).sig.as_ref().and_then(|sig| match cx.input.types.get(sig.ret) {
                crate::types::Type::Class(d, _) => Some(d),
                crate::types::Type::Nested(_, class) => match cx.input.types.get(class) {
                    crate::types::Type::Class(d, _) => Some(d),
                    _ => None,
                },
                _ => None,
            });
            if declared.map_or(false, |d| d == t || syms.class(d).base_types.iter().any(|&(b, _)| b == t)) {
                return true;
            }
        }
        syms.class(f).base_types.iter().any(|&(b, _)| b == t)
    }

    /// What makes a value of `from` one of `to`, two types that differ.
    pub(super) fn adaptation(&self, from: &ErasedType, to: &ErasedType) -> Adaptation {
        match (from, to) {
            (_, ErasedType::V) => Adaptation::Drop,
            (ErasedType::V, ErasedType::L(_)) => Adaptation::Unit,
            (ErasedType::V, _) => Adaptation::Zero,
            (ErasedType::L(f), ErasedType::L(t)) if &**t == OBJECT || self.is_subclass(f, t) => Adaptation::Widen,
            (ErasedType::L(_), ErasedType::L(_)) => Adaptation::Narrow,
            // A Char is a String of one character where the typer takes one for the other.
            (ErasedType::L(f), ErasedType::C) if &**f == STRING => Adaptation::CharOfString,
            (ErasedType::C, ErasedType::L(t)) if &**t == STRING => Adaptation::StringOfChar,
            (ErasedType::L(_), _) => Adaptation::Unbox,
            (_, ErasedType::L(_)) => Adaptation::Box,
            _ => Adaptation::Convert,
        }
    }

    pub fn adapt(&mut self, from: &ErasedType, to: &ErasedType) {
        if from == to {
            return;
        }
        let how = self.adaptation(from, to);
        self.write_adaptation(how, from, to);
    }

    /// Writes the adaptation `how` (`Gen::adaptation`) of the value of `from` on the stack to `to`.
    pub(super) fn write_adaptation(&mut self, how: Adaptation, from: &ErasedType, to: &ErasedType) {
        match how {
            Adaptation::Drop => self.pop_value(from),
            Adaptation::Unit => {
                self.unit_value();
                let vt = self.vt(to);
                self.code.retype_top(vt);
            }
            Adaptation::Zero => match to {
                ErasedType::J => self.lconst(0),
                ErasedType::F => self.fconst(0.0),
                ErasedType::D => self.dconst(0.0),
                _ => self.iconst(0),
            },
            Adaptation::Widen => {
                let vt = self.vt(to);
                self.code.retype_top(vt);
            }
            Adaptation::Narrow => {
                let ErasedType::L(t) = to else { unreachable!("a cast is to a class") };
                self.checkcast(t);
            }
            Adaptation::CharOfString => {
                self.iconst(0);
                self.invoke_desc(Invoke::Virtual, STRING, false, "charAt", "(I)C", 1, &ErasedType::C);
            }
            Adaptation::StringOfChar => {
                self.invoke_desc(Invoke::Static, STRING, false, "valueOf", "(C)Ljava/lang/String;", 1, to);
            }
            Adaptation::Unbox => self.unbox(to),
            Adaptation::Box => {
                let ErasedType::L(t) = to else { unreachable!("a box is a reference") };
                let class = from.box_class();
                let mut desc = String::from("(");
                from.desc(&mut desc);
                desc.push_str(")L");
                desc.push_str(class);
                desc.push(';');
                let boxed = ErasedType::L(Rc::from(class));
                self.invoke_desc(Invoke::Static, class, false, "valueOf", &desc, 1, &boxed);
                if &**t != class {
                    let vt = self.vt(to);
                    self.code.retype_top(vt);
                }
            }
            Adaptation::Convert => {
                let (a, b) = (from, to);
                let insns = primitive_conversion(a, b);
                if !insns.is_empty() {
                    for i in insns {
                        self.code.op(i);
                    }
                    self.code.pop();
                    let vt = self.vt(b);
                    self.code.push(vt);
                }
            }
        }
    }

    /// The primitive `prim` out of the reference on the stack.
    pub(super) fn unbox(&mut self, prim: &ErasedType) {
        let (class, method, desc) = match prim {
            ErasedType::Z => ("java/lang/Boolean", "booleanValue", "()Z"),
            ErasedType::C => ("java/lang/Character", "charValue", "()C"),
            ErasedType::B => ("java/lang/Number", "byteValue", "()B"),
            ErasedType::S => ("java/lang/Number", "shortValue", "()S"),
            ErasedType::I => ("java/lang/Number", "intValue", "()I"),
            ErasedType::J => ("java/lang/Number", "longValue", "()J"),
            ErasedType::F => ("java/lang/Number", "floatValue", "()F"),
            _ => ("java/lang/Number", "doubleValue", "()D"),
        };
        // scalac's erasure unboxes through `BoxesRunTime.unboxToX(Object)` (Erasure.scala:
        // 319-324), one static call that gives the primitive's zero for `null` (a
        // `java.util.Map[K, Int]`'s missing key is 0) and refuses another kind's box; where
        // the class path lacks scala-library the same is spelled out: the value or a boxed
        // zero by `Objects.requireNonNullElse`, then `Number.xValue()`. No branch: a frame in
        // the middle of an expression would fix the operands under the value to the model's
        // wider types.
        if self.cx.boxes_runtime {
            let (m, d) = match prim {
                ErasedType::Z => ("unboxToBoolean", "(Ljava/lang/Object;)Z"),
                ErasedType::C => ("unboxToChar", "(Ljava/lang/Object;)C"),
                ErasedType::B => ("unboxToByte", "(Ljava/lang/Object;)B"),
                ErasedType::S => ("unboxToShort", "(Ljava/lang/Object;)S"),
                ErasedType::I => ("unboxToInt", "(Ljava/lang/Object;)I"),
                ErasedType::J => ("unboxToLong", "(Ljava/lang/Object;)J"),
                ErasedType::F => ("unboxToFloat", "(Ljava/lang/Object;)F"),
                _ => ("unboxToDouble", "(Ljava/lang/Object;)D"),
            };
            self.invoke_desc(Invoke::Static, "scala/runtime/BoxesRunTime", false, m, d, 1, prim);
            return;
        }
        let object = ErasedType::object();
        match prim {
            ErasedType::Z => self.getstatic("java/lang/Boolean", "FALSE", &ErasedType::L(Rc::from("java/lang/Boolean"))),
            ErasedType::C => {
                self.iconst(0);
                self.invoke_desc(Invoke::Static, "java/lang/Character", false, "valueOf", "(C)Ljava/lang/Character;", 1, &ErasedType::L(Rc::from("java/lang/Character")));
            }
            _ => {
                self.iconst(0);
                self.invoke_desc(Invoke::Static, "java/lang/Integer", false, "valueOf", "(I)Ljava/lang/Integer;", 1, &ErasedType::L(Rc::from("java/lang/Integer")));
            }
        }
        self.invoke_desc(Invoke::Static, "java/util/Objects", false, "requireNonNullElse", "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;", 2, &object);
        self.checkcast(class);
        self.invoke_desc(Invoke::Virtual, class, false, method, desc, 0, prim);
    }

    // ---- static types ----

    pub(super) fn recorded_type(&self, e: TExprId) -> Option<TypeId> {
        self.cx.input.prog.expr_types.get(e.0).filter(|&t| t != NO_TYPE)
    }

    pub fn recorded_erasure(&mut self, e: TExprId) -> ErasedType {
        self.recorded(e).unwrap_or_else(ErasedType::object)
    }

    pub(super) fn recorded(&mut self, e: TExprId) -> Option<ErasedType> {
        let t = self.cx.input.prog.expr_types.get(e.0)?;
        if t == NO_TYPE {
            return None;
        }
        Some(self.erase(t))
    }

    fn join(&self, a: ErasedType, b: ErasedType) -> ErasedType {
        if a == b {
            return a;
        }
        match (a.numeric_rank(), b.numeric_rank()) {
            (Some(x), Some(y)) => {
                if x >= y {
                    a
                } else {
                    b
                }
            }
            _ if a == ErasedType::V || b == ErasedType::V => ErasedType::V,
            _ => ErasedType::object(),
        }
    }

    /// The numeric type the typer gave a literal where that is narrower than its node (a `1`
    /// that is a `Byte`, a `1.5f` that is a `Float`).
    fn literal_type(&mut self, e: TExprId) -> Option<ErasedType> {
        self.recorded(e).filter(|t| matches!(t, ErasedType::B | ErasedType::S | ErasedType::F))
    }

    /// `shape`, the type a node's code leaves, or the wider numeric type the typer recorded on
    /// it: a widening between two kinds of box has a node of its own (`xs.head.toDouble` is
    /// `IntToDouble` over the selection, whose Int is unboxed as one), a literal typed narrower
    /// than its node (`val s: Short = 2`) is read through `literal_type`.
    fn widened(&mut self, e: TExprId, shape: ErasedType) -> ErasedType {
        match self.recorded(e) {
            Some(t) if shape.numeric_rank().is_some() && t.numeric_rank() > shape.numeric_rank() => t,
            _ => shape,
        }
    }

    /// The numeric type the typer recorded on a conditional, a match, a block or a `try` where
    /// it is wider than what its branches leave (`(if c then 1 else 2).toDouble`): a consumer
    /// that takes a reference boxes the widened value, not each branch's own.
    pub(super) fn widened_branches(&mut self, e: TExprId) -> Option<ErasedType> {
        let prog = self.cx.input.prog;
        if !matches!(prog.expr(e), TExpr::If(_, _, Some(_)) | TExpr::Match(..) | TExpr::Block(..) | TExpr::Try(_)) {
            return None;
        }
        let recorded = self.recorded(e)?;
        recorded.numeric_rank()?;
        let results: Vec<TExprId> = match prog.expr(e) {
            TExpr::If(_, t, Some(els)) => vec![t, els],
            TExpr::Match(_, cases) => prog.cases[cases.range()].iter().map(|c| c.body).collect(),
            TExpr::Block(_, res) => vec![res],
            TExpr::Try(i) => {
                let t = &prog.tries[i as usize];
                std::iter::once(t.body).chain(prog.cases[t.cases.range()].iter().map(|c| c.body)).collect()
            }
            _ => return None,
        };
        // A branch that throws leaves nothing to join.
        let mut branches: Option<ErasedType> = None;
        for r in results {
            if matches!(prog.expr(r), TExpr::Throw(..)) || self.recorded_type(r).map(|t| self.cx.input.types.canonical(t)) == Some(NOTHING) {
                continue;
            }
            let t = self.static_type(r);
            branches = Some(match branches {
                Some(b) => self.join(b, t),
                None => t,
            });
        }
        let branches = branches?;
        (branches.numeric_rank().is_some() && recorded.numeric_rank() > branches.numeric_rank()).then_some(recorded)
    }

    /// The type the expression has for a consumer without a wish of its own.
    pub fn static_type(&mut self, e: TExprId) -> ErasedType {
        let prog = self.cx.input.prog;
        match prog.expr(e) {
            TExpr::Int(_) => match self.literal_type(e) {
                Some(t) => t,
                None => self.widened(e, ErasedType::I),
            },
            TExpr::Long(_) => ErasedType::J,
            TExpr::Double(_) => self.literal_type(e).unwrap_or(ErasedType::D),
            TExpr::Bool(_) | TExpr::TypeTest(..) => ErasedType::Z,
            TExpr::Char(_) => ErasedType::C,
            TExpr::Str(_) | TExpr::StrConcat(_) | TExpr::ToStr(..) => ErasedType::L(Rc::from(STRING)),
            TExpr::Unit | TExpr::While(..) | TExpr::Assign(..) | TExpr::If(_, _, None) | TExpr::Return(_) => ErasedType::V,
            TExpr::Prim(op, ..) => self.widened(e, prim_type(op)),
            TExpr::Unary(op, _) => self.widened(e, unary_type(op)),
            TExpr::Block(_, res) => match self.recorded(e) {
                Some(t) => t,
                None => self.static_type(res),
            },
            TExpr::If(_, t, Some(els)) => match self.recorded(e) {
                Some(t) => t,
                None => {
                    let (a, b) = (self.static_type(t), self.static_type(els));
                    self.join(a, b)
                }
            },
            TExpr::Match(_, cases) => match self.recorded(e) {
                Some(t) => t,
                None => {
                    let mut out: Option<ErasedType> = None;
                    for c in &prog.cases[cases.range()] {
                        let t = self.static_type(c.body);
                        out = Some(match out {
                            Some(o) => self.join(o, t),
                            None => t,
                        });
                    }
                    out.unwrap_or(ErasedType::V)
                }
            },
            _ => match self.recorded(e) {
                Some(t) => t,
                None => self.natural_type(e),
            },
        }
    }

    /// What the code of a node leaves on the stack, for the nodes whose static type is not read
    /// off their shape.
    fn natural_type(&mut self, e: TExprId) -> ErasedType {
        let prog = self.cx.input.prog;
        match prog.expr(e) {
            TExpr::Local(s) => {
                let info = self.cx.input.syms.sym(s);
                if info.by_name { self.function_type(0) } else { self.sym_type(s) }
            }
            TExpr::Static(s) | TExpr::Field(_, s) => self.sym_type(s),
            TExpr::This | TExpr::Super(_) => match self.m.this_home.or(self.this_class) {
                Some(c) => self.class_type(c),
                None => ErasedType::object(),
            },
            TExpr::Module(c) | TExpr::New(c, _) => self.class_type(c),
            TExpr::ClassOf(..) => ErasedType::L(Rc::from("java/lang/Class")),
            TExpr::NewVia(s, _) => match self.cx.input.syms.sym(s).owner {
                Owner::Class(c) => self.class_type(c),
                _ => ErasedType::object(),
            },
            TExpr::CallStatic(s, _) | TExpr::CallMethod(_, s, _) => self.return_type(s),
            TExpr::Lambda(params, _) => self.function_type(params.len as usize),
            TExpr::SeqLit(_) => match self.cx.input.array_seq {
                Some(c) => self.class_type(c),
                None => ErasedType::object(),
            },
            TExpr::ArrayLit(_) => match self.recorded(e) {
                Some(ErasedType::L(n)) if n.starts_with('[') => ErasedType::L(n),
                _ => ErasedType::L(Rc::from(OBJECT_ARRAY)),
            },
            TExpr::Spread(inner) => self.static_type(inner),
            _ => ErasedType::object(),
        }
    }

    // ---- expressions ----

    /// The receiver (where another instance is called) and the arguments of the call `e` where it
    /// is a jump of the def that loops (`TailLoop::jumps`): one decision, read by the lowering and
    /// by the writing of a `return`.
    pub(super) fn jump_call(&self, e: TExprId) -> Option<(Option<TExprId>, ListRef)> {
        let tail = self.m.tail.as_ref().filter(|t| t.jumps.contains(&e))?;
        self.cx.input.prog.self_call(tail.sym, tail.params.len(), e)
    }

    pub(super) fn fit(&mut self, e: TExprId, t: &ErasedType, want: &ErasedType) -> Fit {
        if let Some((c, args)) = self.expr_value_class(e) {
            if *t != ErasedType::V {
                // A selection of the class's parameter (`Gen::lower_field_read`) opens the box with
                // no null test.
                return Fit::ValueClass(c, args, self.vc_select != Some(e));
            }
        }
        // The typer knows more than the erased signature: `xs.head` of a `List[Int]`. A
        // given object's read is of the given's declared type (`Gen::lower_given_object`), whatever
        // class the typer records for the module it is, so that no build casts it more than
        // another, whichever of a search and its name gave the read.
        let known = match self.cx.input.prog.expr(e) {
            TExpr::Int(_) | TExpr::Double(_) => Some(self.static_type(e)),
            TExpr::Long(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) => None,
            TExpr::Static(s) | TExpr::Field(_, s) if self.cx.given_objects.contains_key(&s) || self.cx.inner_given_objects.contains_key(&s) => None,
            _ => self.recorded(e),
        };
        match known {
            // Between two references the consumer's type decides; going through the
            // static type would unbox a generic result only to box it again. A value
            // that is dropped is not unboxed either: a Java method's null result stays.
            Some(s) if s != *t && s != ErasedType::V && *t != ErasedType::V && *want != ErasedType::V && !(t.is_ref() && want.is_ref() && (!s.is_ref() || want.is_object())) => Fit::Through(s),
            _ => Fit::Direct,
        }
    }

    /// Emits `e` at its static type.
    /// Records the source line of `e` for the code that follows, as scalac does: the line of
    /// the expression's point, which for a selection is its member's name, so that a chain of
    /// calls over several lines attributes each call to its own line. An expression of another
    /// file (an inlined body) keeps the line of the call around it; an argument the typer made
    /// (a given, a default) spans the call it is in and takes the line of the call's end, where
    /// scalac places it, through its whole tree: a given's copy keeps the spans of the site that
    /// searched first, which a worker may have made at another line.
    pub(super) fn mark_line(&mut self, e: TExprId) {
        let cx = self.cx;
        let prog = cx.input.prog;
        if self.line_hold {
            return;
        }
        let Some((file, span)) = prog.span_of(e) else { return };
        if file != self.line_file {
            return;
        }
        let point = match self.line_floor {
            Some((call, f, s, end)) if call != e && (f, s) == (file, span) => {
                self.line_hold = true;
                end
            }
            _ => {
                let point = match prog.expr(e) {
                    TExpr::CallMethod(recv, ..) | TExpr::Field(recv, _) => cx.point_after(file, span, prog.span_of(recv)),
                    TExpr::CallStatic(_, args) => match prog.expr_list(args).first() {
                        Some(&recv) => cx.point_after(file, span, prog.span_of(recv)),
                        None => span.start,
                    },
                    _ => span.start,
                };
                if matches!(prog.expr(e), TExpr::CallMethod(..) | TExpr::CallStatic(..) | TExpr::CallClosure(..) | TExpr::New(..) | TExpr::NewVia(..)) {
                    self.line_floor = Some((e, file, span, span.end.saturating_sub(1)));
                }
                point
            }
        };
        self.code.line(cx.line_of(file, point));
    }

    /// `try`/`catch`/`finally` through the exception table, as dotty's `genLoadTry` lays it out
    /// (BCodeSyncAndTry.scala 181-423). The body and each handler leave their value on the operand stack
    /// and jump to the normal exit; the finalizer is copied into a handler that takes every exception of
    /// the whole `try` up to it and throws it again, then, where a `return` inside took it, into the
    /// cleanup that returns (or goes on to the enclosing frame's), then onto the normal exit, over the
    /// value. Where a handler takes an exception, whose entry holds no operands, the operands under the
    /// `try` are stashed in locals and loaded again after it.
    pub(super) fn write_try(&mut self, want: &ErasedType, body: LirId, catch: Option<crate::lir::Catch>, finalizer: Option<LirId>, returns: bool) {
        // The body's recursion stands on this frame alone: what comes before it and after it is written by
        // functions of their own, which hold their locals only while they run.
        let open = self.open_try(want, catch.is_some(), finalizer, returns);
        self.write(body);
        self.close_try(want, open, catch, finalizer);
    }

    /// What `write_try` writes before the body: the stash, the kept value's slot, the early-return slot, the
    /// labels, and the protected range's start.
    #[inline(never)]
    fn open_try(&mut self, want: &ErasedType, caught: bool, finalizer: Option<LirId>, returns: bool) -> TryOpen {
        let outer = self.code.locals_mark();
        let stash = if caught { self.stash_stack() } else { Vec::new() };
        // The value is kept in a local across the normal exit's copy of a finalizer that holds a `try`
        // (`guardResult`, 219-225).
        // Its slot is taken here and holds nothing until the normal exit stores the value.
        let kept = match finalizer {
            Some(f) if *want != ErasedType::V && self.lir.holds_try(f) => Some(self.code.reserve_local(want.wide())),
            _ => None,
        };
        // A `return` inside the body or a case leaves through the finalizer, as the language runs
        // it on every exit: its value kept in the method's early-return slot, it jumps to the
        // finalizer's cleanup copy, which returns it (`genLoadTry` 3.B, 353-375; scalac's prints
        // `finally ran` before `fin: 1` for `try { return 1 } finally { println("finally ran") }`).
        let made_early = self.early_return_for(returns);
        // Named in the class's constants before the body's, as `genLoadTry` names it.
        let throwable_vt = self.vt(&ErasedType::L(Rc::from(THROWABLE)));
        let (start, end, normal_exit, cleanup) = (self.code.new_label(), self.code.new_label(), self.code.new_label(), self.code.new_label());
        if finalizer.is_some() {
            self.m.finalizers.push(FinallyFrame { cleanup });
        }
        // A handler's frame holds the locals of the whole protected range, not those a body made.
        let protected = self.code.locals_mark();
        self.code.bind(start);
        let from = self.code.pc();
        TryOpen { outer, stash, kept, made_early, throwable_vt, start, end, normal_exit, cleanup, protected, from }
    }

    /// What `write_try` writes after the body: the jump to the normal exit, the handlers, the finalizer's copies
    /// and the stash loaded again.
    #[inline(never)]
    fn close_try(&mut self, want: &ErasedType, open: TryOpen, catch: Option<crate::lir::Catch>, finalizer: Option<LirId>) {
        let TryOpen { outer, stash, kept, made_early, throwable_vt, start, end, normal_exit, cleanup, protected, from } = open;
        let ret = self.m.ret.clone();
        let throwable = ErasedType::L(Rc::from(THROWABLE));
        // A protected range has to hold an instruction.
        if self.code.pc() == from {
            self.code.op(op::NOP);
        }
        self.code.bind(end);
        self.leave_to_join(want, normal_exit);
        // Each handler entered with the exception of its class, each protecting the body, in the cases' order
        // (`genLoadTry` 2.a-2.c, BCodeSyncAndTry.scala 295-328).
        if let Some(catch) = catch {
            self.m.try_exits.insert(catch.done, normal_exit);
            for h in catch.handlers.iter() {
                let handler = self.code.new_label();
                self.code.locals_release(protected);
                let class = ErasedType::L(h.class.clone());
                let vt = self.vt(&class);
                self.code.bind_handler(handler, vt);
                let mark = self.code.locals_mark();
                match h.caught {
                    crate::lir::Caught::Dropped => {
                        self.code.op(op::POP);
                        self.code.pop();
                    }
                    crate::lir::Caught::Local(sym) => {
                        let slot = self.store_new(&class);
                        self.m.locals.insert(sym, Local { slot, ty: class.clone(), cell: false });
                    }
                    crate::lir::Caught::Temp(temp) => {
                        let slot = self.store_new(&class);
                        self.m.temps.insert(temp, slot);
                    }
                }
                self.write(h.body);
                self.code.locals_release(mark);
                let catch_type = self.cw.cp.class(&h.class);
                self.code.add_handler(Handler { start, end, handler, catch_type });
            }
        }
        if let Some(f) = finalizer {
            let t = self.finalizer_of();
            // A `return` inside the finalizer itself belongs to the enclosing frame; one in the body
            // or a case asked for the cleanup copy (`currentFinallyBlockNeedsCleanup`, 329-332).
            self.m.finalizers.pop();
            let needs_cleanup = self.m.exits_taken.contains(&cleanup);
            // The handler's range is the whole `try` up to it, the body's and the cases' jumps to
            // the normal exit inside (`protect(startTryBody, finalHandler, ...)`, 342-351).
            let any = self.code.new_label();
            self.code.add_handler(Handler { start, end: any, handler: any, catch_type: 0 });
            self.code.locals_release(protected);
            self.code.bind_handler(any, throwable_vt);
            // Its exception's slot is held until the `try` ends (dotty's `makeLocal`), so the copies
            // after it take the slots after it, and a finalizer's own vals the same slots in every
            // copy, as dotty's (`getOrMakeLocal`, BCodeBodyBuilder.scala 331-335).
            let thrown = self.store_new(&throwable);
            self.write_finalizer(f, t);
            self.load(thrown, &throwable);
            self.code.op(op::ATHROW);
            self.code.pop();
            self.code.end_path();
            if needs_cleanup {
                self.code.bind(cleanup);
                self.code.hold_local(thrown);
                self.write_finalizer(f, t);
                self.pending_cleanups(&ret);
            }
            self.code.bind(normal_exit);
            self.code.hold_local(thrown);
            if let Some(reserved) = kept {
                let vt = self.vt(want);
                self.code.type_local(reserved, vt);
                self.store(reserved.slot, want);
            }
            self.write_finalizer(f, t);
            if let Some(reserved) = kept {
                self.load(reserved.slot, want);
            }
        } else {
            self.code.bind(normal_exit);
        }
        self.unstash_stack(stash, kept.map(|r| r.slot), want);
        self.code.locals_release(outer);
        if made_early {
            self.m.early_return = None;
        }
    }

    /// After a cleanup copy (`pendingCleanups`, BCodeSyncAndTry.scala 426-440): on to the enclosing
    /// frame's cleanup, or the early-return slot returned.
    fn pending_cleanups(&mut self, ret: &ErasedType) {
        match self.m.finalizers.last().copied() {
            Some(outer) => {
                self.m.exits_taken.push(outer.cleanup);
                self.goto(outer.cleanup);
            }
            None => {
                if let Some(slot) = self.m.early_return {
                    self.load(slot, ret);
                }
                self.return_value(ret);
            }
        }
    }

    /// The early-return slot made before a frame whose code returns (`returns`), where the method
    /// returns a value and no enclosing frame made it: dotty makes `earlyReturnVar` at the first such
    /// return, but a frame of this writer's is its scope's, so the slot stands before the frame,
    /// given the zero of its type. Whether this frame made it, to drop it with its scope.
    fn early_return_for(&mut self, returns: bool) -> bool {
        let ret = self.m.ret.clone();
        // Out of a closure a `return` is thrown, through no frame.
        if !returns || self.m.closure || self.m.early_return.is_some() || ret == ErasedType::V {
            return false;
        }
        match &ret {
            ErasedType::L(_) => {
                self.code.op(op::ACONST_NULL);
                self.code.push(VT::Null);
            }
            _ => self.adapt(&ErasedType::V, &ret),
        }
        self.m.early_return = Some(self.store_new(&ret));
        true
    }

    /// The code `body` writes at the type `t` under the monitor of the object `recv` loads (a `Synchronized` node), as
    /// dotty's `genSynchronized` writes `recv.synchronized(body)` (BCodeSyncAndTry.scala 27-103): the receiver loaded
    /// once (`genLoadQualifier`) and kept in a local, `monitorenter`, the body protected by a catch-any handler that
    /// exits the monitor and rethrows (`protect`), its value kept in a local across the normal `monitorexit`. A
    /// `return` inside leaves through this block's frame (`registerCleanup`): its cleanup, written only where a return
    /// took it (`shouldEmitCleanup`), exits the monitor and goes on to the next enclosing frame's exit or returns
    /// (`pendingCleanups`). The value's type is the body's; the operands under the block stay where they are, a
    /// `return` inside dropping them as it leaves (`drop_stack`). `returns`: whether the body holds a `return` of the
    /// method (the node's `returns`), for which the early-return slot is made before the block.
    pub fn monitor_block(&mut self, recv: impl FnOnce(&mut Self), t: ErasedType, returns: bool, body: impl FnOnce(&mut Self, &ErasedType)) -> ErasedType {
        let object = ErasedType::object();
        recv(self);
        self.dup();
        let monitor = self.store_new(&object);
        self.code.op(op::MONITORENTER);
        self.code.pop();
        // `monitorResult`, and the frame's slots, made before the protected range, whose handler's frame
        // holds the locals made before it alone.
        let result = match &t {
            ErasedType::V => None,
            ErasedType::L(_) => {
                self.code.op(op::ACONST_NULL);
                self.code.push(VT::Null);
                Some(self.store_new(&t))
            }
            _ => {
                self.adapt(&ErasedType::V, &t);
                Some(self.store_new(&t))
            }
        };
        let ret = self.m.ret.clone();
        let made_early = self.early_return_for(returns);
        let (start, end, handler, cleanup, post) =
            (self.code.new_label(), self.code.new_label(), self.code.new_label(), self.code.new_label(), self.code.new_label());
        self.m.finalizers.push(FinallyFrame { cleanup });
        let protected = self.code.locals_mark();
        self.code.bind(start);
        let from = self.code.pc();
        body(self, &t);
        if let Some(slot) = result {
            self.store(slot, &t);
        }
        // `nopIfNeeded`: a protected range has to hold an instruction.
        if self.code.pc() == from {
            self.code.op(op::NOP);
        }
        self.code.bind(end);
        self.m.finalizers.pop();
        self.load(monitor, &object);
        self.code.op(op::MONITOREXIT);
        self.code.pop();
        self.goto(post);
        self.code.add_handler(Handler { start, end, handler, catch_type: 0 });
        self.code.locals_release(protected);
        let throwable = ErasedType::L(Rc::from(THROWABLE));
        let throwable_vt = self.vt(&throwable);
        self.code.bind_handler(handler, throwable_vt);
        self.load(monitor, &object);
        self.code.op(op::MONITOREXIT);
        self.code.pop();
        self.code.op(op::ATHROW);
        self.code.pop();
        self.code.end_path();
        if self.m.exits_taken.contains(&cleanup) {
            self.code.bind(cleanup);
            self.load(monitor, &object);
            self.code.op(op::MONITOREXIT);
            self.code.pop();
            self.pending_cleanups(&ret);
        }
        self.code.bind(post);
        if let Some(slot) = result {
            self.load(slot, &t);
        }
        if made_early {
            self.m.early_return = None;
        }
        t
    }

    /// Drops every operand on the stack (an uninitialised object's too): what a `return` leaving
    /// through a frame leaves dead under its value. The operands dropped, the bottom one first.
    pub(super) fn drop_stack(&mut self) -> Vec<VT> {
        let mut dropped = Vec::new();
        while let Some(&top) = self.code.stack.last() {
            match top {
                VT::Long | VT::Double => self.code.op(op::POP2),
                _ => self.code.op(op::POP),
            }
            self.code.pop();
            dropped.push(top);
        }
        dropped.reverse();
        dropped
    }

    /// A value at the type `want` carried on the stack to a join `exit` (a `try`'s normal exit, an `if`'s end, a
    /// labelled block's end), whose frame takes it at that type: the frame is recorded at the first jump, and the
    /// branch that jumps first may have left a narrower value (a `null`), which the other branches' are not.
    pub(super) fn leave_to_join(&mut self, want: &ErasedType, exit: Label) {
        if *want != ErasedType::V {
            let vt = self.vt(want);
            self.code.retype_top(vt);
        }
        self.goto(exit);
        if *want != ErasedType::V {
            self.code.pop();
        }
    }

    /// The operands on the stack, under a `try` whose handlers enter with none, moved into fresh locals: a local
    /// for each, the deepest's first, then stored from the top down (`genLoadTry` (0), BCodeSyncAndTry.scala
    /// 208-212, 233-240).
    fn stash_stack(&mut self) -> Vec<(u16, ErasedType, VT)> {
        let operands = self.code.stack.clone();
        let mut stash = Vec::new();
        for top in operands {
            let t = match top {
                VT::Int => ErasedType::I,
                VT::Long => ErasedType::J,
                VT::Float => ErasedType::F,
                VT::Double => ErasedType::D,
                // An object `new` made and its constructor has not run yet (an argument of a constructor, an
                // element of a list literal's array) is stashed as it is, as dotty's stack stash keeps it
                // (`acquireFullStack`): the verifier takes an uninitialised reference in a local.
                VT::Object(_) | VT::Null | VT::Uninit(_) => ErasedType::object(),
                _ => {
                    self.unsupported("not supported on the JVM yet: try as an argument of a constructor");
                    return Vec::new();
                }
            };
            stash.push((self.code.new_local(top), t, top));
        }
        for (slot, t, _) in stash.iter().rev() {
            self.store(*slot, t);
        }
        stash
    }

    /// The stashed operands loaded again under the `try`'s value, each at the type it had on the stack (a module's
    /// class, not `Object`: dotty's `makeTempLocal` of the operand's own type), each reference's local cleared after
    /// it; the value kept meanwhile in the local `kept` holds where there is one, else in a fresh one, cleared after
    /// it too (`genLoadTry` (5), BCodeSyncAndTry.scala 386-418).
    fn unstash_stack(&mut self, stash: Vec<(u16, ErasedType, VT)>, kept: Option<u16>, want: &ErasedType) {
        if stash.is_empty() {
            return;
        }
        let result = (*want != ErasedType::V).then(|| {
            kept.unwrap_or_else(|| {
                let vt = self.vt(want);
                self.code.new_local(vt)
            })
        });
        if let Some(slot) = result {
            self.store(slot, want);
        }
        for (slot, t, vt) in stash {
            self.load(slot, &t);
            if t.is_ref() {
                self.code.retype_top(vt);
                self.clear_local(slot, &t);
            }
        }
        if let Some(slot) = result {
            self.load(slot, want);
            if want.is_ref() {
                self.clear_local(slot, want);
            }
        }
    }

    fn clear_local(&mut self, slot: u16, t: &ErasedType) {
        self.code.op(op::ACONST_NULL);
        self.code.push(VT::Null);
        self.store(slot, t);
    }

    /// The parameter type under which the JDK renders a value of type `t`: `String.valueOf`,
    /// `StringBuilder.append` and `insert` have an overload per primitive, one for a `String`
    /// and one for any other reference.
    pub fn rendered_as(t: &ErasedType) -> ErasedType {
        match t {
            ErasedType::L(n) if &**n == STRING => t.clone(),
            ErasedType::L(_) | ErasedType::V => ErasedType::object(),
            ErasedType::B | ErasedType::S => ErasedType::I,
            p => p.clone(),
        }
    }

    pub fn string_value_of(&mut self, t: &ErasedType) {
        let param = match t {
            ErasedType::L(n) if &**n == STRING => return,
            ErasedType::L(_) => ErasedType::object(),
            ErasedType::V => {
                self.sconst("()");
                return;
            }
            ErasedType::B | ErasedType::S => ErasedType::I,
            p => p.clone(),
        };
        let string = ErasedType::L(Rc::from(STRING));
        self.invoke(Invoke::Static, STRING, false, "valueOf", &[param], &string);
    }

    /// Where `s` is a top-level inline val, its constant: it is erased with its getter
    /// (`isEffectivelyErased`, `Erasure.typedValDef`), a read being the constant of its type, which
    /// `Namer` keeps for an inline val and `InlineVals` requires, as dotty's typer inlines it (None
    /// for one with no constant type, which `InlineVals` refuses).
    pub(super) fn inline_val_constant(&self, s: SymId) -> Option<Option<LitId>> {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        if info.kind != SymKind::Val || info.mods & mods::INLINE == 0 || !matches!(info.owner, Owner::Package(_)) {
            return None;
        }
        Some(match info.sig.as_ref().map(|sig| cx.input.types.get(sig.ret)) {
            Some(Type::Lit(l)) => Some(l),
            _ => None,
        })
    }

    /// The statements of the blocks the head of a concatenation's chain brings (`Program::chain_head`), which run
    /// before the first operand, and the operands: a chain's own, those behind a block's result, or the head as one
    /// operand.
    pub(super) fn concat_route(&mut self, items: ListRef) -> (Vec<ListRef>, Vec<TExprId>) {
        let prog = self.cx.input.prog;
        let (mut statements, mut operands) = (Vec::new(), Vec::new());
        if let Some((&head, rest)) = prog.expr_list(items).split_first() {
            self.chain_head(head, &mut statements, &mut operands);
            operands.extend_from_slice(rest);
        }
        (statements, operands)
    }

    fn chain_head(&self, head: TExprId, statements: &mut Vec<ListRef>, operands: &mut Vec<TExprId>) {
        let prog = self.cx.input.prog;
        match prog.chain_head(head) {
            ChainHead::Operand => operands.push(head),
            ChainHead::Chain(items) => {
                if let Some((&inner, rest)) = prog.expr_list(items).split_first() {
                    self.chain_head(inner, statements, operands);
                    operands.extend_from_slice(rest);
                }
            }
            ChainHead::Block(stmts, res) => {
                statements.push(stmts);
                self.chain_head(res, statements, operands);
            }
        }
    }

    // ---- locals, fields, statics ----

    /// The instance the method runs on; None where the method has none.
    pub(super) fn this_value(&mut self) -> Option<ThisValue> {
        if let Some(module) = self.m.this_module.clone() {
            return Some(ThisValue::Module(module));
        }
        if !self.m.has_instance() {
            return None;
        }
        let t = match (self.m.this_home, self.m.this_underlying.clone(), self.this_class) {
            (Some(home), _, _) => self.class_type(home),
            (None, Some(underlying), _) => underlying,
            (None, None, Some(c)) => self.class_type(c),
            (None, None, None) => ErasedType::object(),
        };
        Some(ThisValue::Slot(t))
    }

    pub fn load_this(&mut self) -> ErasedType {
        let instance = self.lowering(|g| g.lower_instance());
        self.write(instance);
        self.lir.ty(instance).clone()
    }

    /// Where the scope keeps what it holds for a local: its value, its cell where a closure
    /// shares it, a lazy val's holder. None for a local out of scope, and for a local class's
    /// capture in code with no instance.
    pub(super) fn held_in(&mut self, s: SymId) -> Option<HeldIn> {
        let cell_type = || ErasedType::L(Rc::from(OBJECT_REF));
        if let Some((t, cell)) = self.declared_by_lowering(s) {
            return Some(HeldIn::Local(if *cell { cell_type() } else { t.clone() }));
        }
        if let Some(l) = self.m.locals.get(&s) {
            return Some(HeldIn::Local(if l.cell { cell_type() } else { l.ty.clone() }));
        }
        let (field, ty, held) = self.captured_field(s)?;
        self.this_value()?;
        Some(HeldIn::Field(field, match held {
            Held::Value => ty,
            Held::Cell => cell_type(),
        }))
    }

    /// The type and the cell of the local `s` as the running lowering declared it, before the writer
    /// stores it: none outside a lowering, where the scope is what the writer has stored.
    fn declared_by_lowering(&self, s: SymId) -> Option<&(ErasedType, bool)> {
        self.code.frozen.then(|| self.lower_declared.get(&s)).flatten()
    }

    pub(super) fn captured_field(&self, s: SymId) -> Option<(String, ErasedType, Held)> {
        self.class_captures.iter().find(|c| c.0 == s).map(|c| (c.1.clone(), c.2.clone(), c.3))
    }

    /// Whether the local lives in a cell where the current code sees it.
    pub fn is_cell(&self, s: SymId) -> bool {
        if let Some((_, cell)) = self.declared_by_lowering(s) {
            return *cell;
        }
        match self.m.locals.get(&s) {
            Some(l) => l.cell,
            None => self.captured_field(s).map_or(false, |c| c.2 == Held::Cell),
        }
    }

    /// The class the constant `classOf[t]` of the constant type `class_type`, `Class[t]` (`LitVal::Class`), is the
    /// value of, as `lower_class_of` names a class literal's (`class_kind`).
    pub(super) fn class_constant(&mut self, class_type: TypeId) -> ErasedType {
        let types = self.cx.input.types;
        match types.get(class_type) {
            Type::Class(_, args) if !types.items(args).is_empty() => self.class_literal_kind(types.items(args)[0]),
            _ => {
                self.unsupported("internal error: a class constant whose type is no `Class[T]`");
                ErasedType::object()
            }
        }
    }

    /// `CoreBTypes.toTypeKind` of a class constant's type `t` (`typeToTypeKind` 159-198): a class's by `class_kind`
    /// (`getClassBType`), `Nothing`'s `srNothingRef`, `Any`'s `Object`, an alias's its expansion's.
    fn class_literal_kind(&mut self, t: TypeId) -> ErasedType {
        let types = self.cx.input.types;
        match types.get(t) {
            Type::Class(c, args) => {
                let element = types.items(args).first().copied();
                self.class_kind(c, element)
            }
            Type::Nothing => self.array_marker(0),
            Type::Any => ErasedType::object(),
            Type::Alias(a, args) => match self.alias_expansion(a, args) {
                Some(x) => self.class_literal_kind(x),
                None => ErasedType::object(),
            },
            _ => {
                self.unsupported("internal error: a class constant of no class type");
                ErasedType::object()
            }
        }
    }

    /// `getClassBType` and `primitiveOrClassToBType` (CoreBTypes.scala) for the class `c` of a class literal:
    /// `Array`'s the array class of `element`'s erasure, `Nothing`'s and `Null`'s scala-library's `Nothing$` and
    /// `Null$` (`srNothingRef`, `srNullRef`), an opaque type's its underlying type's erasure (scalac's `ClassTag` of
    /// one over `Int` is `Int`'s), a primitive's or `Unit`'s its own, any other class's the class.
    pub(super) fn class_kind(&mut self, c: ClassId, element: Option<TypeId>) -> ErasedType {
        let info = self.cx.input.syms.class(c);
        if info.kind == ClassKind::Builtin {
            match self.cx.input.interner.get(info.name) {
                "Array" => {
                    return match element {
                        Some(elem) => self.array_type(Some(elem)),
                        None => ErasedType::L(Rc::from(OBJECT_ARRAY)),
                    };
                }
                "Nothing" => return self.array_marker(0),
                "Null" => return self.array_marker(1),
                _ => {}
            }
        }
        if info.kind == ClassKind::Opaque {
            if let Some(under) = info.underlying {
                return self.erase(under);
            }
        }
        if info.kind == ClassKind::Builtin {
            let primitive = match self.cx.input.interner.get(info.name) {
                "Int" => Some(ErasedType::I),
                "Long" => Some(ErasedType::J),
                "Double" => Some(ErasedType::D),
                "Float" => Some(ErasedType::F),
                "Boolean" => Some(ErasedType::Z),
                "Char" => Some(ErasedType::C),
                "Byte" => Some(ErasedType::B),
                "Short" => Some(ErasedType::S),
                "Unit" => Some(ErasedType::V),
                _ => None,
            };
            if let Some(t) = primitive {
                return t;
            }
        }
        ErasedType::L(self.class_name(c))
    }

    /// `genConstant`'s `ClazzTag` (BCodeBodyBuilder.scala 597-614) for the class of `kind`: a primitive's or `Unit`'s
    /// boxed class's `TYPE` (`boxedClassOfPrimitive`, `Unit`'s `java.lang.Void`), a reference or array class by `ldc`.
    pub(super) fn push_class(&mut self, kind: &ErasedType) -> ErasedType {
        let class = ErasedType::L(Rc::from("java/lang/Class"));
        match kind {
            ErasedType::L(name) => {
                let i = self.cw.cp.class(name);
                self.ldc(i);
                let t = VT::Object(self.cw.cp.class("java/lang/Class"));
                self.code.push(t);
            }
            ErasedType::V => self.getstatic("java/lang/Void", "TYPE", &class),
            primitive => self.getstatic(primitive.box_class(), "TYPE", &class),
        }
        class
    }

    /// The class of the object `c`'s instance; None for an object without a body that only its
    /// exports are reached through, which is not emitted, and for a Java class's statics, which
    /// have no object.
    pub(super) fn module_name(&mut self, c: ClassId) -> Option<Rc<str>> {
        if !self.cx.input.reach.classes[c.idx()] || self.cx.input.java_class(c).is_some() {
            return None;
        }
        Some(self.class_name(c))
    }

    /// The class of the object that holds a file's top-level definitions.
    pub(super) fn file_module_name(&self, file: crate::source::FileId) -> Rc<str> {
        Rc::from(self.cx.file_modules[file.0 as usize].as_str())
    }

    /// The instance of the object whose class is `name`.
    pub(super) fn get_module(&mut self, name: &Rc<str>) -> ErasedType {
        let t = ErasedType::L(name.clone());
        self.getstatic(name, "MODULE$", &t);
        t
    }

    pub(super) fn static_route(&mut self, s: SymId) -> StaticRoute {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        if cx.given_objects.contains_key(&s) {
            return StaticRoute::GivenObject;
        }
        if let Some((_, JMember::Field(_))) = cx.input.java_member(s) {
            return StaticRoute::JavaField;
        }
        if let SymKind::EnumValue(case) = info.kind {
            // A jar enum's value is a static field of its companion, as scalac writes it, and so
            // is a program enum's whose companion this build writes with the fields.
            if let Owner::Class(companion) = info.owner {
                if let Some(e) = cx.input.syms.class(companion).companion.filter(|&e| cx.linked_class(companion) || cx.has_enum_fields(e)) {
                    return StaticRoute::EnumField(companion, e);
                }
            }
            return StaticRoute::EnumValue(case);
        }
        if self.inline_val_constant(s).is_some() {
            return StaticRoute::InlineVal;
        }
        let m = self.mref(s);
        if m.kind == Invoke::Static && m.params.is_empty() && !matches!(info.kind, SymKind::Def) {
            return StaticRoute::Field;
        }
        StaticRoute::Accessor
    }

    /// The class a reference to `s` evaluates its receiver at: the site's, or the declaring one.
    pub(super) fn receiver_type(&mut self, s: SymId, site: &Option<Site>) -> ErasedType {
        match (site, self.cx.input.syms.sym(s).owner) {
            (Some(site), _) => site.qualifier.clone(),
            (None, Owner::Class(c)) => self.class_type(c),
            (None, _) => ErasedType::object(),
        }
    }

    /// Where a virtual call or a field access on the qualifier `r` of a member that `owner`
    /// declares points, as dotty's erasure and backend choose it: the qualifier's class, not the
    /// declaring one (`BCodeBodyBuilder.genApply` and `genLoad`'s `receiverClass`: "using
    /// sym.owner may lead to IllegalAccessError"), so that a public subclass reaches what a
    /// package-private superclass declares. The class is the qualifier after
    /// `Erasure.typedSelect`'s adaptation: its erased type (an erased value class's box, a
    /// member's or a local's declared erasure, so that a generic result is an `Object`, an
    /// ascription's the ascribed type, the outermost deciding, a block's its result's as
    /// `Typer.typedBlock` types it, the recorded type's for a conditional, a cast and the rest) where that derives from the owner and the JVM lets this class name it
    /// (`isJvmAccessible`: a Java class that is not public, from its own package alone); else the
    /// cast's target, the owner (`None`: the declaring class, the receiver cast to it as
    /// `Gen::lower_receiver` casts it) or, where the owner is not accessible, the qualifier's type before
    /// erasure (`originalQual`, scala3#8661). Where that is not accessible either, or is the
    /// erased type itself, the reference cannot be written: refused as dotty refuses it
    /// (Erasure.scala 746-758), `member` naming what is referenced. An array, a primitive or a
    /// bottom (`defn.isBottomClass`) is `None` too, its member `Object`'s.
    pub(super) fn qualifier_site(&mut self, r: TExprId, owner: &str, member: Referenced) -> Option<Site> {
        let cx = self.cx;
        // An ascription's value is of the ascribed type, the outermost one deciding: erasure
        // retains it (`Erasure.typedTyped`, `Program::ascribed_bits`), a generic result's `Object`
        // cast to it. Else a block's value is its result's (`Typer.typedBlock`).
        let mut result = r;
        while !cx.input.prog.is_ascribed(result) {
            let TExpr::Block(_, res) = cx.input.prog.expr(result) else { break };
            result = res;
        }
        let ascribed = cx.input.prog.is_ascribed(result) && self.expr_value_class(result).is_none();
        let erased = match self.recorded(result) {
            Some(t) if ascribed => t,
            _ => match self.expr_value_class(result) {
                Some((c, _)) => self.class_type(c),
                None => match cx.input.prog.expr(result) {
                    TExpr::Local(_) | TExpr::Static(_) | TExpr::Field(..) | TExpr::CallStatic(..) | TExpr::CallMethod(..) | TExpr::CallClosure(..) | TExpr::This | TExpr::Module(_) | TExpr::New(..) | TExpr::NewVia(..) => self.natural_type(result),
                    TExpr::Null | TExpr::Super(_) => return None,
                    _ => self.static_type(result),
                },
            },
        };
        let ErasedType::L(name) = erased else { return None };
        if name.starts_with('[') || super::runtime::ARRAY_MARKERS.contains(&&*name) {
            return None;
        }
        let accessible = self.jvm_accessible(&name);
        if accessible && &*name == owner {
            return None;
        }
        let target = if accessible && self.erased_derives(&name, owner) {
            name
        } else if self.jvm_accessible(owner) {
            return None;
        } else {
            match self.recorded(r) {
                Some(ErasedType::L(n)) if n != name && !n.starts_with('[') && self.erased_derives(&n, owner) && self.jvm_accessible(&n) => n,
                _ => {
                    let message = self.unwritable_reference(member, owner);
                    self.refuse(r, message);
                    return None;
                }
            }
        };
        let interface = cx.classes_named(&target).any(|c| cx.is_interface(c));
        Some(Site { qualifier: ErasedType::L(target.clone()), owner: target, interface })
    }

    /// dotty's message of a reference `typedSelect` cannot adapt: "Unable to emit reference to
    /// method m in class Base, class Base is not accessible in object Main" (the member's
    /// `showLocated`, its owner, the enclosing class).
    fn unwritable_reference(&self, member: Referenced, owner: &str) -> String {
        let cx = self.cx;
        let syms = cx.input.syms;
        let what = match member {
            Referenced::Method(name) => format!("method {}", name),
            Referenced::Sym(s) => {
                let kind = match cx.input.java_member(s) {
                    Some((cf, JMember::Field(i))) if cf.fields[i as usize].access & ACC_FINAL != 0 => "value",
                    Some((_, JMember::Field(_))) => "variable",
                    Some((_, JMember::Method(_))) => "method",
                    None => match syms.sym(s).kind {
                        SymKind::Def => "method",
                        SymKind::Var => "variable",
                        _ => "value",
                    },
                };
                format!("{} {}", kind, cx.input.interner.get(syms.sym(s).name))
            }
        };
        let shown = |c: ClassId| {
            let kind = match syms.class(c).kind {
                ClassKind::Trait => "trait",
                ClassKind::Object => "object",
                _ => "class",
            };
            format!("{} {}", kind, cx.input.interner.get(syms.class(c).name))
        };
        let simple = |n: &str| n.rsplit('/').next().unwrap_or(n).to_string();
        let declaring = cx.classes_named(owner).next().map_or_else(|| format!("class {}", simple(owner)), shown);
        let enclosing = self.this_class.map_or_else(|| format!("object {}", simple(&self.this_name).trim_end_matches('$')), shown);
        format!("Unable to emit reference to {} in {}, {} is not accessible in {}", what, declaring, declaring, enclosing)
    }

    /// Whether the erased class `from` derives from `to`, as `Erasure.typedSelect` tests an
    /// erased qualifier (`derivesFrom` after erasure, where a trait's parents are interfaces
    /// alone): `is_subclass`, and over every class the program names where that knows the
    /// reached ones alone (a JDK class, `ArrayList` for `Collection`).
    pub(super) fn erased_derives(&self, from: &str, to: &str) -> bool {
        if to == OBJECT || self.is_subclass(from, to) {
            return true;
        }
        let cx = self.cx;
        let syms = cx.input.syms;
        cx.classes_named(from).any(|f| {
            cx.classes_named(to).any(|t| !(syms.class(f).kind == ClassKind::Trait && syms.class(t).kind != ClassKind::Trait) && syms.class(f).base_types.iter().any(|&(b, _)| b == t))
        })
    }

    /// Whether the class being written can name the class `name` without the JVM refusing it, as
    /// dotty asks it (`Erasure.typedSelect.isJvmAccessible`): every class not defined in Java; a
    /// Java class where it is accessible from here before erasure (`SymDenotations.
    /// isAccessibleFrom` of its owner's `this`, within its `accessBoundary`), by the flags of its
    /// own `InnerClasses` entry where it is nested. A private nested class's boundary is its
    /// enclosing class, which no class teq writes is inside; a public one's, and a protected
    /// one's reached through its owner's `this`, is the root; a class of its package alone,
    /// top-level or nested, is accessible from that package, Java's packages flat (JVMS 5.4.4).
    fn jvm_accessible(&self, name: &str) -> bool {
        let cx = self.cx;
        let Some(java) = cx.classes_named(name).find_map(|c| cx.input.java_class(c)) else { return true };
        if java.cf.scala {
            return true;
        }
        fn package(n: &str) -> &str {
            n.rfind('/').map_or("", |at| &n[..at])
        }
        let access = match java.cf.own_inner_entry() {
            Some(entry) if entry.outer.is_some() => entry.access,
            _ => java.cf.access,
        };
        if access & ACC_PRIVATE != 0 {
            return false;
        }
        access & (ACC_PUBLIC | crate::classfile::ACC_PROTECTED) != 0 || package(name) == package(&self.this_name)
    }

    /// Where a virtual call of `s` through the reference `m` on the qualifier `r` points
    /// (`BCodeBodyBuilder.genCallMethod`): the qualifier's class (`qualifier_site`) for a member
    /// that is neither private nor static, `invokeinterface` where that class is an interface and
    /// `invokevirtual` where it is a class whatever declares the member; the declaring interface
    /// where the qualifier's interface inherits a member overriding one of `Object`'s, the
    /// workaround of JDK-8154587 (`isTraitMethodOverridingObjectMember`). dotty's `isScalaStatic`
    /// leaves out no member here: teq writes an `@static` member as its object's instance member,
    /// reached through the object, whose class is the declaring one (`qualifier_site`'s `None`).
    pub(super) fn invoke_site(&mut self, r: TExprId, s: SymId, m: &MethodRef) -> Option<Site> {
        if !self.virtual_member(s, m) {
            return None;
        }
        self.method_site(r, &m.owner, m.owner_is_interface, &m.name, &m.desc, Referenced::Sym(s))
    }

    /// `invoke_site` of a virtual call of the method `name` of descriptor `desc` that `owner`
    /// declares, a std template's (`assemble`) as a member's.
    pub(super) fn method_site(&mut self, r: TExprId, owner: &Rc<str>, owner_is_interface: bool, name: &str, desc: &str, member: Referenced) -> Option<Site> {
        let mut site = self.qualifier_site(r, owner, member)?;
        let overrides_object = &**owner != OBJECT
            && matches!(
                (name, desc),
                ("clone", "()Ljava/lang/Object;") | ("equals", "(Ljava/lang/Object;)Z") | ("finalize", "()V") | ("hashCode", "()I") | ("toString", "()Ljava/lang/String;")
            );
        if site.interface && overrides_object {
            site.owner = owner.clone();
            site.interface = owner_is_interface;
        }
        Some(site)
    }

    /// Whether `s`, called through `m`, is called in `genApply`'s virtual style: a class's or a
    /// trait's member that is neither private nor static (`InvokeStyle.Virtual`).
    fn virtual_member(&self, s: SymId, m: &MethodRef) -> bool {
        let info = self.cx.input.syms.sym(s);
        matches!(m.kind, Invoke::Virtual | Invoke::Interface) && matches!(info.owner, Owner::Class(_)) && info.mods & mods::PRIVATE == 0
    }

    /// Where a call of `Object`'s member `name` on the qualifier `r` points (`qualifier_site`,
    /// its `None` `Object` itself) and the class the qualifier is evaluated at.
    pub(super) fn object_member_site(&mut self, r: TExprId, name: &str) -> (Option<Site>, ErasedType) {
        let site = self.qualifier_site(r, OBJECT, Referenced::Method(name));
        let qualifier = site.as_ref().map_or_else(ErasedType::object, |s| s.qualifier.clone());
        (site, qualifier)
    }

    /// Whether `r.s` reads the field of this class's own instance: a val that a subclass may override is read
    /// through its accessor, a qualified private one's (`private[p]`, no `Private` flag in dotty) among them.
    pub(super) fn own_field(&self, r: TExprId, s: SymId) -> bool {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        matches!(cx.input.prog.expr(r), TExpr::This)
            && matches!(info.owner, Owner::Class(c) if Some(c) == self.this_class
                && !cx.is_interface(c)
                && (info.mods & mods::PRIVATE != 0 && !info.scoped_private || cx.input.syms.class(c).subclasses.is_empty() && (!cx.input.open_world || cx.input.syms.class(c).mods & mods::FINAL != 0)))
            && !cx.is_lazy_member(s)
            && info.kind != SymKind::Def
            && self.m.instance_is_first_local()
            && self.this_class.map_or(false, |c| *cx.class_names[c.idx()] == *self.this_name)
    }

    pub(super) fn field_route(&self, r: TExprId, s: SymId) -> FieldRoute {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        if cx.input.syms.js_member(s) {
            return FieldRoute::Js;
        }
        if let Some((_, JMember::Field(_))) = cx.input.java_member(s) {
            return FieldRoute::Java;
        }
        if let SymKind::Object(c) = info.kind {
            return FieldRoute::Object(c);
        }
        if cx.given_objects.contains_key(&s) {
            let receiver_evaluated = !matches!(cx.input.prog.expr(r), TExpr::Module(_) | TExpr::This);
            return FieldRoute::GivenObject { receiver_evaluated };
        }
        if let Some(c) = self.vc_field(r, s) {
            return FieldRoute::Underlying(c);
        }
        if self.m.before_super && matches!(cx.input.prog.expr(r), TExpr::This) && self.m.locals.contains_key(&s) {
            return FieldRoute::Early;
        }
        if self.own_field(r, s) {
            return FieldRoute::Own;
        }
        if let (true, Owner::Class(c)) = (cx.private_params.contains_key(&s), info.owner) {
            return FieldRoute::PrivateParam(c);
        }
        FieldRoute::Accessor
    }

    /// The implementation a program class was given for the deferred given `s`
    /// (`TClass::deferred_givens`) where the receiver `r` is an instance of that class or of a
    /// subclass: read on the class, as a class read from its module's products is, whose pickle
    /// holds the implementation as its member, so that a whole build and a split one agree.
    pub(super) fn deferred_implementation(&self, r: TExprId, s: SymId) -> Option<SymId> {
        let cx = self.cx;
        if cx.input.syms.sym(s).mods & mods::DEFERRED == 0 {
            return None;
        }
        let mut c = match cx.input.types.get(self.recorded_type(r)?) {
            Type::This(c) | Type::Class(c, _) => c,
            Type::Nested(_, class) => match cx.input.types.get(class) {
                Type::Class(c, _) => c,
                _ => return None,
            },
            _ => return None,
        };
        loop {
            let i = cx.tclass_of[c.idx()];
            if i != u32::MAX {
                if let Some(&(_, implementation)) = cx.input.prog.classes[i as usize].deferred_givens.iter().find(|&&(d, _)| d == s) {
                    return Some(implementation);
                }
            }
            c = cx.input.syms.class(c).superclass?;
        }
    }

    /// In link mode, the parameter of a value class read on a value of it is the value itself:
    /// the class, where `r.s` is such a read.
    fn vc_field(&self, r: TExprId, s: SymId) -> Option<ClassId> {
        let cx = self.cx;
        let Owner::Class(c) = cx.input.syms.sym(s).owner else { return None };
        if !cx.input.syms.class(c).value_class {
            return None;
        }
        let field = cx.input.syms.class(c).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym)?;
        if field != s && cx.input.syms.class(c).members.get(&cx.input.syms.sym(s).name) != Some(&field) {
            return None;
        }
        if matches!(cx.input.prog.expr(r), TExpr::This) && Some(c) == self.this_class && self.m.this_underlying.is_none() {
            return None;
        }
        Some(c)
    }

    /// The setter of the member `m` refers to: `m_$eq`, which takes a value of the member's type.
    pub(super) fn setter_ref(&self, m: &MethodRef) -> MethodRef {
        let params = vec![m.ret.clone()];
        let desc = method_desc(&params, &ErasedType::V);
        MethodRef { owner: m.owner.clone(), owner_is_interface: m.owner_is_interface, name: format!("{}_$eq", m.name), params, ret: ErasedType::V, desc, kind: m.kind }
    }

    // ---- calls ----

    /// Parameter types in the class file's order (`jvm_param_order`) put back in the signature's.
    pub(super) fn signature_order(&self, order: &[usize], params: &[ErasedType]) -> Vec<ErasedType> {
        let mut written = vec![ErasedType::object(); params.len()];
        for (j, &k) in order.iter().enumerate() {
            written[k] = params[j].clone();
        }
        written
    }

    /// Where the default `i` of a constructor of `c` comes from, the secondary `via` where named, for
    /// a getter that takes the first `taken` of the parameters; `received` where the typer selected
    /// the getter on a receiver.
    pub(super) fn ctor_default_route(&mut self, c: ClassId, via: Option<SymId>, received: bool, i: usize, taken: usize, types: &[ErasedType]) -> CtorDefault {
        let cx = self.cx;
        if cx.jvm_class(c).is_some() {
            let default = match (via, cx.tclass_of.get(c.idx()).copied().filter(|&i| i != u32::MAX)) {
                (Some(s), _) => cx.input.prog.funs.iter().find(|f| f.sym == s).and_then(|f| f.defaults.get(i).copied().flatten()),
                (None, Some(k)) => cx.input.prog.classes[k as usize].ctor_defaults[self.capture_count(c)..].get(i).copied().flatten(),
                (None, None) => None,
            };
            return CtorDefault::InPlace(default);
        }
        // A local class's getters are its local companion's, the receiver the typer selects them on.
        if cx.ctor_defaults_on_local_companion(c) {
            let m = cx.input.syms.class(c).companion.expect("a local companion");
            let module = self.class_name(m);
            let v = cx.input.syms.class(m).local_module.expect("a local module");
            return CtorDefault::LocalCompanion(v, self.companion_getter(module, c, via, i, types));
        }
        // A class nested in a class whose companion, an object of each enclosing instance reached
        // by its getter, holds the getters (scalac's layout): another module's or a jar's, and the
        // program's (`ctor_defaults_on_inner_companion`), and a written companion's that forwards to
        // the class's (`inner_written_companion_members`), on the creation's enclosing instance or,
        // where a parent's call passes none, on the one the class captures.
        let on_companion = cx.ctor_defaults_on_inner_companion(c);
        if received && (!cx.emits(c) || via.is_none() && cx.has_inner_written_case_companion(c)) || on_companion {
            if let Owner::Class(o) = cx.input.syms.class(c).owner {
                let trait_owner = cx.input.syms.class(o).kind == ClassKind::Trait;
                if cx.input.syms.class(o).kind == ClassKind::Class || trait_owner {
                    let module: Rc<str> = Rc::from(cx.companion_name(c));
                    let ret = ErasedType::L(module.clone());
                    let desc = method_desc(&[], &ret);
                    // A trait's companion is reached through the interface's accessor, which the
                    // classes mixing it in implement.
                    let kind = if trait_owner { Invoke::Interface } else { Invoke::Virtual };
                    let name = cx.input.interner.get(cx.input.syms.class(c).name).to_string();
                    let accessor = MethodRef { owner: self.class_name(o), owner_is_interface: trait_owner, name, params: Vec::new(), ret, desc, kind };
                    return CtorDefault::InnerCompanion(accessor, self.companion_getter(module, c, via, i, types));
                }
            }
        }
        let owner = self.class_name(c);
        let before = (0..taken).map(|k| types.get(k).cloned().unwrap_or_else(ErasedType::object));
        let n = self.capture_count(c);
        if via.is_none() && n > 0 && cx.input.syms.class(c).kind != ClassKind::Anon {
            let mut params = self.anon_ctor_params(c);
            let captures = params.len();
            params.extend(before);
            let ret = self.default_getter_ret(self.ctor_clause_shape(c, via, i).1, |_| types[i].clone());
            let desc = method_desc(&params, &ret);
            let getter = MethodRef { owner, owner_is_interface: false, name: format!("$lessinit$greater$default${}", n + i + 1), params, ret, desc, kind: Invoke::Static };
            return CtorDefault::Captured(captures, getter);
        }
        let name = format!("$lessinit$greater$default${}", i + 1);
        // A class whose companion holds the getters (scalac's layout): each takes the parameters
        // of the clauses before the default's, a by-name parameter's returning the value.
        if cx.ctor_defaults_on_companion(c) {
            let module: Rc<str> = Rc::from(cx.companion_name(c));
            return CtorDefault::Companion(self.companion_getter(module, c, via, i, types));
        }
        if let Some(getter) = self.jar_ctor_getter(c, &name) {
            return CtorDefault::Jar(getter);
        }
        let params: Vec<ErasedType> = before.collect();
        let ret = self.default_getter_ret(self.ctor_clause_shape(c, via, i).1, |_| types[i].clone());
        let desc = method_desc(&params, &ret);
        CtorDefault::Static(MethodRef { owner, owner_is_interface: self.cx.is_interface(c), name, params, ret, desc, kind: Invoke::Static })
    }

    /// The type a default's getter returns where the typer gives it no type of its own: for a by-name parameter the
    /// value, the erasure of the parameter's result type (`by_name`), as scalac's getter returns it and the caller's
    /// thunk around the getter's call returns it; for any other the parameter's type (`param`). One rule for the
    /// getters of a method (`Gen::default_getter_shape`) and of a constructor (`Gen::ctor_default_route`, the getters
    /// `Gen::ctor_default_getters` and the companions' writers write), and for the type a constructor's default is
    /// left as (`Gen::default_route`).
    pub(super) fn default_getter_ret(&mut self, by_name: Option<TypeId>, param: impl FnOnce(&mut Self) -> ErasedType) -> ErasedType {
        match by_name {
            Some(t) => self.erase(t),
            None => param(self),
        }
    }

    /// Where the clause of the declared parameter `i` of `c`'s primary constructor, or of its secondary constructor
    /// `via`, starts, and its result type where the parameter is by-name (`clause_shape`).
    pub(super) fn ctor_clause_shape(&self, c: ClassId, via: Option<SymId>, i: usize) -> (usize, Option<TypeId>) {
        let cx = self.cx;
        match via {
            Some(s) => cx.input.syms.sym(s).sig.as_ref().map_or((i, None), |sig| clause_shape(&sig.clauses, i)),
            None => clause_shape(&cx.input.syms.class(c).ctor, i),
        }
    }

    /// The getter of default `i` on the companion `module` of `c`, as scalac's companion has
    /// it: it takes the parameters of the clauses before the default's and returns the
    /// parameter's type, a by-name parameter's value.
    fn companion_getter(&mut self, module: Rc<str>, c: ClassId, via: Option<SymId>, i: usize, types: &[ErasedType]) -> MethodRef {
        let (before, by_name) = self.ctor_clause_shape(c, via, i);
        let ret = self.default_getter_ret(by_name, |_| types[i].clone());
        let params = types[..before].to_vec();
        let desc = method_desc(&params, &ret);
        MethodRef { owner: module, owner_is_interface: false, name: format!("$lessinit$greater$default${}", i + 1), params, ret, desc, kind: Invoke::Virtual }
    }

    /// A jar class's constructor default as scalac's class files have it: the companion's getter,
    /// taking the arguments of the clauses before the default's only, called on the companion as
    /// scalac calls it, or the static forwarder of the class where the companion's class file is
    /// not read.
    fn jar_ctor_getter(&mut self, c: ClassId, name: &str) -> Option<MethodRef> {
        let cx = self.cx;
        let companion = cx.input.syms.class(c).companion;
        let forwarder = cx.class_files.get(&c).and_then(|cf| cf.methods.iter().find(|m| m.name == name && m.access & ACC_STATIC != 0)).map(|m| m.descriptor.clone());
        let on_companion = || {
            let k = companion?;
            let cf = cx.class_files.get(&k)?;
            cf.methods.iter().find(|m| m.name == name && m.access & ACC_STATIC == 0).map(|m| (Rc::from(cf.name.as_str()), m.descriptor.clone()))
        };
        let (kind, owner, desc) = match (on_companion(), forwarder) {
            (Some((module, desc)), _) => (Invoke::Virtual, module, desc),
            (None, Some(desc)) => (Invoke::Static, self.class_name(c), desc),
            (None, None) => return None,
        };
        let (params, ret) = parse_method_desc(&desc);
        Some(MethodRef { owner, owner_is_interface: false, name: name.to_string(), params, ret, desc, kind })
    }

    /// Why the getter of a constructor's default, where no argument precedes it, takes what the
    /// method being written does not hold: a local companion or a capture out of scope, a capture
    /// of a kind no local holds, an inner class's enclosing instance; None where the lowering has a
    /// node for all of it.
    pub(super) fn ctor_default_refusal(&mut self, c: ClassId, route: &CtorDefault, recv: Option<TExprId>) -> Option<crate::lir::Refusal> {
        let name = |g: &Self, s: SymId| -> Rc<str> { Rc::from(g.cx.input.interner.get(g.cx.input.syms.sym(s).name)) };
        match *route {
            CtorDefault::InPlace(_) | CtorDefault::Companion(_) | CtorDefault::Jar(_) | CtorDefault::Static(_) => None,
            CtorDefault::LocalCompanion(v, _) => (recv.is_none() && self.held_in(v).is_none()).then(|| crate::lir::Refusal::LocalNotInScope(name(self, v))),
            CtorDefault::InnerCompanion(..) if recv.is_some() => None,
            CtorDefault::InnerCompanion(..) => match self.class_captured(c).first().copied() {
                Some(s) => self.held_in(s).is_none().then(|| crate::lir::Refusal::LocalNotInScope(name(self, s))),
                None => {
                    let outer = match self.cx.input.syms.class(c).owner {
                        Owner::Class(o) => o,
                        _ => c,
                    };
                    Some(crate::lir::Refusal::InstanceNotInScope(Rc::from(self.cx.input.interner.get(self.cx.input.syms.class(outer).name))))
                }
            },
            CtorDefault::Captured(..) => self.class_captures_after(c, recv.is_some() as usize).into_iter().find_map(|s| {
                if !self.is_capturable(s) {
                    Some(crate::lir::Refusal::DefaultCaptureOfThisKind)
                } else {
                    self.held_in(s).is_none().then(|| crate::lir::Refusal::LocalNotInScope(name(self, s)))
                }
            }),
        }
    }

    pub(super) fn static_call_route(&mut self, s: SymId, args: ListRef) -> StaticCallRoute {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        if let Some(array) = self.spread_conversion(s, args) {
            return StaticCallRoute::Spread(array);
        }
        if let Some((_, JMember::Method(_))) = cx.input.java_member(s) {
            return StaticCallRoute::JavaStatic;
        }
        if matches!(info.owner, Owner::Class(c) if cx.jdk_statics(c)) && self.mref(s).kind == Invoke::Static {
            return StaticCallRoute::JdkStatic;
        }
        match (info.owner == Owner::Local, info.kind != SymKind::Def) {
            (true, true) => StaticCallRoute::LocalValue,
            (true, false) => StaticCallRoute::Lifted,
            (false, _) => StaticCallRoute::FileModule,
        }
    }

    pub(super) fn default_route(&mut self, of: DefaultOf, at: u32, args: ListRef) -> DefaultRoute {
        let cx = self.cx;
        let items = cx.input.prog.expr_list(args);
        let index = (at & !DEFAULT_RECEIVER) as usize;
        let recv = (at & DEFAULT_RECEIVER != 0).then(|| items[0]);
        if let DefaultOf::Method(s) = of {
            let info = cx.input.syms.sym(s);
            if recv.is_none() && info.owner == Owner::Local && info.name != crate::names::INIT {
                return DefaultRoute::LocalDef(s, index);
            }
            if info.name != crate::names::INIT {
                let priors = items[recv.is_some() as usize..].to_vec();
                return self.method_default_route(s, index, recv, &priors);
            }
        }
        let (c, via, types) = match of {
            DefaultOf::Ctor(c) => (c, None, self.ctor_param_types(c)),
            DefaultOf::Method(s) if cx.input.syms.sym(s).name == crate::names::INIT => {
                // A constructor of no class is refused.
                let Some(c) = self.ctor_class(s) else { return DefaultRoute::Refused(crate::lir::Refusal::DefaultOfConstructorOfNoClass) };
                (c, Some(s), self.param_types(s))
            }
            DefaultOf::Method(_) => unreachable!("a method's default: `method_default_route` above"),
        };
        let priors = self.written_priors(&items[recv.is_some() as usize..], &types);
        let route = self.ctor_default_route(c, via, recv.is_some(), index, priors.len(), &types);
        if let Some(refusal) = self.ctor_default_refusal(c, &route, recv) {
            return DefaultRoute::Refused(refusal);
        }
        // Each argument at the type the route's getter takes it: a companion's getter and a jar's
        // adapt it to their parameters (`Gen::lower_getter_arguments`, `CtorDefault::Jar`), a captured class's
        // and a static one take it as written; the class files of the JDK take none.
        let written = |p: &Prior| (p.expr, p.ty.clone());
        let args: Vec<GetterArgument> = match &route {
            CtorDefault::InPlace(_) => Vec::new(),
            CtorDefault::LocalCompanion(_, getter) | CtorDefault::InnerCompanion(_, getter) | CtorDefault::Companion(getter) | CtorDefault::Jar(getter) => {
                let params = if matches!(route, CtorDefault::Jar(_)) { getter.params.clone() } else { parse_method_desc(&getter.desc).0 };
                priors.iter().zip(params).map(|(p, param)| {
                    let (expr, ty) = written(p);
                    GetterArgument { expr, ty, param }
                }).collect()
            }
            CtorDefault::Captured(..) | CtorDefault::Static(_) => priors.iter().map(|p| {
                let (expr, ty) = written(p);
                GetterArgument { expr, ty: ty.clone(), param: ty }
            }).collect(),
        };
        // A by-name parameter's default is left as its getter returns it (`Gen::default_getter_ret`).
        let want = match route.getter() {
            Some(getter) if self.ctor_clause_shape(c, via, index).1.is_some() => getter.ret.clone(),
            _ => types[index].clone(),
        };
        DefaultRoute::Ctor(c, recv, route, args, want).held_unless_paths(self.default_operands_are_paths(None, &items[recv.is_some() as usize..]), &priors)
    }

    /// `default_route` for the default `i` of the method `s`, `recv` the receiver the typer
    /// selected its getter on, `priors` the arguments before it.
    fn method_default_route(&mut self, s: SymId, i: usize, recv: Option<TExprId>, priors: &[TExprId]) -> DefaultRoute {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        if let Some(route) = recv.and_then(|r| self.value_class_default_route(r, s, i, priors)) {
            return route;
        }
        let m = self.mref(s);
        let types = match self.jvm_param_order(s).filter(|o| o.len() == m.params.len()) {
            Some(order) => self.signature_order(&order, &m.params),
            None => m.params.clone(),
        };
        let shape = self.default_getter_shape(s, i);
        let want = self.default_getter_want(s, i, &shape);
        let paths = self.default_operands_are_paths(recv, priors);
        let written = self.written_priors(priors, &types);
        let priors = written.clone();
        let taken = self.getter_args(s, shape.before, &priors);
        let before: Vec<ErasedType> = taken.iter().map(|p| p.ty.clone()).collect();
        let (name, desc) = self.default_getter_ref(s, i, &before);
        let (params, ret) = parse_method_desc(&desc);
        // A by-name parameter's getter passed as a function made where it is called.
        let thunk = self.function_type(0);
        if want == thunk && ret != thunk {
            return DefaultRoute::Refused(crate::lir::Refusal::ByNameDefaultWithoutSignature);
        }
        let args: Vec<GetterArgument> = taken
            .iter()
            .zip(&params)
            .map(|(p, param)| GetterArgument { expr: p.expr, ty: p.ty.clone(), param: param.clone() })
            .collect();
        let Some(r) = recv else {
            let getter = MethodRef { owner: m.owner.clone(), owner_is_interface: m.owner_is_interface, name, params, ret, desc, kind: m.kind };
            return DefaultRoute::FileMember(info.file, getter, args, want).held_unless_paths(paths, &written);
        };
        match cx.input.prog.expr(r) {
            // A getter selected on `super` inside a trait: the accessor the classes mixing the trait
            // in implement (`SuperAccessors`), on the trait's `this`.
            TExpr::Super(SuperTarget::Mixin(t)) => {
                let getter = self.super_getter_accessor_ref(t, s, i);
                let want = if shape.by_name { shape.ret.clone() } else { m.params[i].clone() };
                if want == thunk && getter.ret != thunk {
                    return DefaultRoute::Refused(crate::lir::Refusal::ByNameDefaultWithoutSignature);
                }
                let args: Vec<GetterArgument> = taken
                    .iter()
                    .zip(&getter.params)
                    .map(|(p, param)| GetterArgument { expr: p.expr, ty: p.ty.clone(), param: param.clone() })
                    .collect();
                let receiver_type = self.receiver_type(s, &None);
                return DefaultRoute::Method { receiver: r, receiver_type, getter, at: None, args, want }.held_unless_paths(paths, &written);
            }
            // A getter selected on `super` is the superclass's or the class's, as
            // `Applications.findDefaultGetter` selects it on the receiver: `invokespecial`.
            TExpr::Super(target) => {
                let owner = match target {
                    SuperTarget::Chain => self.this_class.and_then(|c| cx.input.syms.class(c).superclass),
                    SuperTarget::Class(c) | SuperTarget::Mixin(c) => Some(c),
                };
                let receiver_type = self.receiver_type(s, &None);
                let getter = match owner {
                    Some(c) => MethodRef { owner: self.class_name(c), owner_is_interface: cx.input.syms.class(c).kind == ClassKind::Trait, name, params, ret, desc, kind: Invoke::Special },
                    None => MethodRef { owner: m.owner.clone(), owner_is_interface: m.owner_is_interface, name, params, ret, desc, kind: m.kind },
                };
                return DefaultRoute::Method { receiver: r, receiver_type, getter, at: None, args, want }.held_unless_paths(paths, &written);
            }
            _ => {}
        }
        // Another receiver's getter is named where the receiver points, as the method is (`invoke_site`).
        let site = if self.virtual_member(s, &m) { self.qualifier_site(r, &m.owner, Referenced::Sym(s)) } else { None };
        let (receiver_type, at) = match site {
            Some(site) => (ErasedType::L(site.owner.clone()), Some((site.owner, site.interface))),
            None => (self.receiver_type(s, &None), None),
        };
        let getter = MethodRef { owner: m.owner.clone(), owner_is_interface: m.owner_is_interface, name, params, ret, desc, kind: m.kind };
        DefaultRoute::Method { receiver: r, receiver_type, getter, at, args, want }.held_unless_paths(paths, &written)
    }

    /// The arguments of a getter's application as they are written, each at its parameter's type.
    fn written_priors(&self, priors: &[TExprId], types: &[ErasedType]) -> Vec<Prior> {
        priors.iter().enumerate().map(|(k, &expr)| Prior { expr, ty: types.get(k).cloned().unwrap_or_else(ErasedType::object) }).collect()
    }

    /// Whether a getter's receiver (where it has one) and its arguments are paths, whose writing
    /// where the getter takes them tells nothing apart: the typer's plan passes only those
    /// (`Worker::default_application`, its lifter leaving a path in place, `Worker::no_lift`, and
    /// lifting anything else to a val), but an inline parameter's default is expanded into a
    /// later default's arguments where it stands (`Worker::own_getter_expansion`), so the back end
    /// tests them itself, by the typer's rule of a path (`Worker::expr_purity` at `PURE_PATH`): a
    /// literal, `this`, `super`, an object, a reference to a val or a parameter that is no `var`,
    /// lazy, by-name or inline one, a stable member's selection on a path, and a by-name
    /// argument's thunk, which its making does not run.
    fn default_operands_are_paths(&self, recv: Option<TExprId>, priors: &[TExprId]) -> bool {
        recv.is_none_or(|r| self.default_operand_is_path(r)) && priors.iter().all(|&a| self.default_operand_is_path(a))
    }

    pub(super) fn default_operand_is_path(&self, e: TExprId) -> bool {
        let cx = self.cx;
        let stable = |s: SymId| {
            let info = cx.input.syms.sym(s);
            let fixed = info.mods & (crate::ast::mods::MUTABLE | crate::ast::mods::LAZY | crate::ast::mods::INLINE) == 0 && !info.by_name;
            fixed && matches!(info.kind, SymKind::Val | SymKind::Param | SymKind::Object(_) | SymKind::EnumValue(_))
        };
        let stable_def = |s: SymId| {
            let info = cx.input.syms.sym(s);
            info.kind == SymKind::Def && info.stable
        };
        match cx.input.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null => true,
            TExpr::This | TExpr::Super(_) | TExpr::Module(_) | TExpr::Lambda(..) => true,
            TExpr::Local(s) | TExpr::Static(s) => stable(s),
            TExpr::Field(q, s) => stable(s) && self.default_operand_is_path(q),
            TExpr::CallStatic(s, l) => l.len == 0 && stable_def(s),
            TExpr::CallMethod(q, s, l) => l.len == 0 && stable_def(s) && self.default_operand_is_path(q),
            _ => false,
        }
    }

    /// The type the default `i` of method `s` is left as: the parameter's, a by-name one's value.
    fn default_getter_want(&mut self, s: SymId, i: usize, shape: &GetterShape) -> ErasedType {
        if shape.by_name {
            return shape.ret.clone();
        }
        let m = self.mref(s);
        match self.jvm_param_order(s) {
            Some(order) => order.iter().position(|&k| k == i).map_or_else(|| m.params[i].clone(), |j| m.params[j].clone()),
            None => m.params[i].clone(),
        }
    }

    /// The route of a default a value class's method declares itself, its getter on the receiver's
    /// underlying value as `Gen::lower_extension_call` calls the method: the companion's
    /// `m$default$N$extension` where this build writes it, a library companion's as its class file
    /// declares it (refused where it has none); None for any other method, for a default the method
    /// inherits from a universal trait's member (that member's getter, which the box inherits, on
    /// the boxed receiver), and for a value class of the program whose companion this build does
    /// not write (its `m$default$N` on the box).
    fn value_class_default_route(&mut self, r: TExprId, s: SymId, i: usize, priors: &[TExprId]) -> Option<DefaultRoute> {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        let Owner::Class(c) = info.owner else { return None };
        if !cx.input.syms.class(c).value_class || info.kind != SymKind::Def {
            return None;
        }
        let inherited = info.info.sig.as_ref().and_then(|sig| sig.clauses.iter().flat_map(|cl| cl.params.iter()).nth(i)).is_some_and(|p| !p.has_default);
        if inherited {
            return None;
        }
        let underlying = self.vc_underlying(c);
        let paths = self.default_operands_are_paths(Some(r), priors);
        let (getter, types, want) = if cx.vc_on_companion(c) {
            let rest = self.erased_param_types(s);
            let shape = self.default_getter_shape(s, i);
            let mut params = vec![underlying.clone()];
            params.extend(rest[..shape.before].iter().cloned());
            let ret = shape.ret.clone();
            let desc = method_desc(&params, &ret);
            let name = format!("{}$default${}$extension", self.source_method_name(s), shape.index);
            let want = if shape.by_name { ret.clone() } else { rest[i].clone() };
            let getter = MethodRef { owner: Rc::from(cx.companion_name(c)), owner_is_interface: false, name, params, ret, desc, kind: Invoke::Virtual };
            (getter, rest, want)
        } else {
            let companion = cx.vc_companions.get(&c)?;
            let name = format!("{}$default${}$extension", encode(cx.input.interner.get(cx.input.syms.sym(s).name)), i + 1);
            // The getter takes the underlying value first: one that takes nothing is no getter of it.
            let Some((gm, (params, ret))) = companion.methods.iter().filter(|m| m.name == name).map(|m| (m, parse_method_desc(&m.descriptor))).find(|(_, (params, _))| !params.is_empty()) else {
                return Some(DefaultRoute::Refused(crate::lir::Refusal::ValueClassDefaultWithoutGetter));
            };
            let types = params[1..].to_vec();
            let getter = MethodRef { owner: Rc::from(companion.name.as_str()), owner_is_interface: false, name, params, ret: ret.clone(), desc: gm.descriptor.clone(), kind: Invoke::Virtual };
            (getter, types, ret)
        };
        let written = self.written_priors(priors, &types);
        let args = written
            .iter()
            .zip(&getter.params[1..])
            .map(|(p, param)| GetterArgument { expr: p.expr, ty: p.ty.clone(), param: param.clone() })
            .collect();
        Some(DefaultRoute::Extension { receiver: r, underlying, getter, args, want }.held_unless_paths(paths, &written))
    }

    /// The name and descriptor of the getter of default `i` of method `s`: a jar's as its class
    /// file declares it, the program's as `emit_fun` writes it; `before` are the types of the
    /// parameters before the default, which a jar member without its getter takes.
    pub fn default_getter_ref(&mut self, s: SymId, i: usize, before: &[ErasedType]) -> (String, String) {
        let m = self.mref(s);
        let shape = self.default_getter_shape(s, i);
        let name = format!("{}$default${}", self.source_method_name(s), shape.index);
        let jar = self.linked_owner(s).and_then(|cf| cf.methods.iter().find(|g| g.name == name && g.access & ACC_STATIC == 0)).map(|g| g.descriptor.clone());
        let desc = match jar {
            Some(desc) => desc,
            // A jar member without its getter: teq's erasure of it, as before.
            None if self.linked_owner(s).is_some() => method_desc(before, &m.params[i]),
            None => method_desc(&m.params[..shape.before], &shape.ret),
        };
        (name, desc)
    }

    /// The getter of the default of parameter `i` (flattened, in the signature's order) of program
    /// method `s`, as scalac writes it: the number in its name (`m$default$N`), how many
    /// parameters it takes, those of the clauses before the default's as the class file orders
    /// them (`m.params[..before]`), its result, the parameter's type, a by-name parameter's value
    /// rather than its thunk, and whether the parameter is by-name. A right-associative
    /// extension's are numbered and ordered as its declaration has them (`declared_clauses`):
    /// `$plus$colon$default$1()String` of `extension (n: Int) def +:(s: String = "x")`.
    pub fn default_getter_shape(&mut self, s: SymId, i: usize) -> GetterShape {
        let m = self.mref(s);
        let fallback = GetterShape { index: i + 1, before: i, ret: m.params[i].clone(), by_name: false };
        let Some(sig) = self.cx.input.syms.sym(s).info.sig.as_ref() else { return fallback };
        let mut start = 0;
        let mut found = None;
        for (ci, cl) in sig.clauses.iter().enumerate() {
            if i < start + cl.params.len() {
                found = Some((ci, i - start));
                break;
            }
            start += cl.params.len();
        }
        let Some((ci, k)) = found else { return fallback };
        let p = sig.clauses[ci].params[k].clone();
        let clauses = self.declared_clauses(s).unwrap_or_else(|| (0..sig.clauses.len()).collect());
        let pos = clauses.iter().position(|&c| c == ci).unwrap_or(ci);
        let before: usize = clauses[..pos].iter().map(|&c| sig.clauses[c].params.len()).sum();
        // The getter's type as the typer completed it (`Namer.inferredResultType`): the default's
        // own where the parameter's type names the method's type parameters (`a$default$1[A]:
        // String`), as scalac's class file has it; the parameter's otherwise.
        let own = self.cx.input.getters.and_then(|t| crate::typer::getters::getter_type(t, self.cx.input.syms, crate::tir::DefaultOf::Method(s), i)).filter(|&t| !self.cx.input.types.same(t, p.ty));
        let ret = match own {
            Some(t) => self.erase(t),
            None => self.default_getter_ret(p.by_name.then_some(p.ty), |_| m.params[before + k].clone()),
        };
        GetterShape { index: before + k + 1, before, ret, by_name: p.by_name }
    }

    /// The arguments of a default's getter among `temps`, the arguments before the default in the
    /// signature's order: those of the clauses before the default's as the class file orders them.
    pub fn getter_args(&self, s: SymId, before: usize, temps: &[Prior]) -> Vec<Prior> {
        match self.jvm_param_order(s) {
            Some(order) => order[..before.min(order.len())].iter().filter_map(|&k| temps.get(k).cloned()).collect(),
            None => temps.to_vec(),
        }
    }

    /// The type of a Java member's value as the program sees it, of its declared type `t`: what
    /// the member leaves, a JVM array included.
    fn java_value_type(&self, t: ErasedType) -> ErasedType {
        t
    }

    /// The erasure of the elements of a varargs literal's sequence type; a value class's
    /// instances are its boxes there, whatever the class holds.
    pub(super) fn seq_literal_element(&mut self, e: TExprId) -> ErasedType {
        let types = self.cx.input.types;
        match self.recorded_type(e).map(|t| types.get(t)) {
            Some(Type::Class(_, args)) => match types.items(args).first() {
                Some(&elem) if self.value_class_of(elem).is_some() => ErasedType::object(),
                Some(&elem) => self.erase(elem),
                None => ErasedType::object(),
            },
            _ => ErasedType::object(),
        }
    }

    /// The array an argument `arr*` spliced into a Java array parameter stands for: the typer
    /// passes it through scala-library's conversion to a sequence (`copyArrayToImmutableIndexedSeq`,
    /// `wrapRefArray`, ..), whose argument it marks (`Program::spread_bits`); such a conversion
    /// written in the program is copied, as scalac copies it. The array goes as it is where the
    /// JVM takes it for the parameter: any array of references for an array of references, the
    /// typer having found its elements the parameter's (a `checkcast` where the class is not
    /// known to be), and a primitive array for the same.
    pub(super) fn spliced_array(&mut self, a: TExprId, want: &str) -> Option<TExprId> {
        let cx = self.cx;
        let (s, args) = match cx.input.prog.expr(a) {
            TExpr::CallStatic(s, args) | TExpr::CallMethod(_, s, args) => (s, args),
            _ => return None,
        };
        let array = self.spread_conversion(s, args)?;
        let ErasedType::L(class) = self.erase(self.recorded_type(array)?) else { return None };
        let references = |d: &str| d.starts_with("[L") || d.starts_with("[[");
        let fits = &*class == want || (references(want) && references(&class));
        fits.then_some(array)
    }

    /// The array `arr*` of the typer's conversion `s(args)` to the sequence a varargs parameter
    /// takes (`copyArrayToImmutableIndexedSeq`, `wrapRefArray`, ..), where the typer marked it
    /// spread (`Program::spread_bits`); a conversion the program writes has none.
    pub(super) fn spread_conversion(&mut self, s: SymId, args: ListRef) -> Option<TExprId> {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        let name = cx.input.interner.get(info.name);
        let conversion = name == "copyArrayToImmutableIndexedSeq" || name == "genericWrapArray" || (name.starts_with("wrap") && name.ends_with("Array"));
        let Owner::Class(owner) = info.owner else { return None };
        if !conversion || !matches!(&*self.class_name(owner), "scala/Predef$" | "scala/LowPriorityImplicits" | "scala/LowPriorityImplicits2") {
            return None;
        }
        let &[array] = cx.input.prog.expr_list(args) else { return None };
        cx.input.prog.is_spread(array).then_some(array)
    }

    pub(super) fn call_route(&mut self, r: TExprId, s: SymId, args: ListRef) -> CallRoute {
        let cx = self.cx;
        if let Some(array) = self.spread_conversion(s, args) {
            return CallRoute::Spread(array);
        }
        if cx.input.syms.js_member(s) {
            return CallRoute::Js;
        }
        if let Some((c, route)) = self.extension_route(s) {
            return CallRoute::Extension(c, route);
        }
        if let Some(&(_, helper)) = cx.input.reach.interpolators.iter().find(|&&(m, _)| m == s) {
            return CallRoute::Interpolator(helper);
        }
        if let SymKind::Object(c) = cx.input.syms.sym(s).kind {
            return CallRoute::Object(c);
        }
        if self.mref(s).kind == Invoke::Static {
            if cx.input.java_member(s).is_some() {
                return CallRoute::JavaStatic;
            }
            if matches!(cx.input.syms.sym(s).owner, Owner::Class(c) if cx.jdk_statics(c)) {
                return CallRoute::JdkStatic;
            }
        }
        let receiver = cx.input.prog.expr(r);
        if let TExpr::Super(SuperTarget::Class(t)) = receiver {
            if self.takes_proxies(s) {
                return CallRoute::Proxied(t);
            }
        }
        if self.lifted_call(s, args) {
            if !matches!(receiver, TExpr::Super(SuperTarget::Mixin(_))) {
                return CallRoute::Lifted;
            }
        }
        match receiver {
            TExpr::Super(_) => CallRoute::Super,
            _ => CallRoute::Plain,
        }
    }

    /// Whether a call of `s` with `args` passes proxies first: `s` a local trait's member that
    /// takes them (a lifted member, `TFun::lifted`, or a lazy val's initialiser read by the
    /// block) called with them. A pure test: which proxies they are is the capture analysis's,
    /// asked where the call is lowered (`Gen::lower_proxied_call`).
    fn lifted_call(&self, s: SymId, args: ListRef) -> bool {
        self.proxied_fun(s).is_some_and(|fun| fun.params.len() == args.len as usize)
    }

    /// How `s`, a method of a value class, is called, with the class; None for any other member
    /// and for a jar's value class whose companion has no method standing for it.
    fn extension_route(&mut self, s: SymId) -> Option<(ClassId, ExtensionRoute)> {
        let cx = self.cx;
        let Owner::Class(c) = cx.input.syms.sym(s).owner else { return None };
        if !cx.input.syms.class(c).value_class || cx.input.syms.sym(s).kind != SymKind::Def {
            return None;
        }
        // A value class whose companion this build writes is called through it, as scalac and a
        // build over the class's products call it.
        if cx.vc_on_companion(c) {
            return Some((c, ExtensionRoute::Companion));
        }
        if cx.tclass_of[c.idx()] != u32::MAX && !cx.input.link.jar_classes.contains_key(&c) {
            return Some((c, ExtensionRoute::Static));
        }
        self.jar_extension(c, s).map(|_| (c, ExtensionRoute::Jar))
    }

    /// The method of a jar's companion that stands for the method `s` of the value class `c`,
    /// with the companion, the signature teq erases `s` to behind the underlying value, and the
    /// method's name.
    #[allow(clippy::type_complexity)]
    fn jar_extension(&mut self, c: ClassId, s: SymId) -> Option<(&'a crate::classfile::ClassFile, &'a crate::classfile::Method, Vec<ErasedType>, ErasedType, String)> {
        let cx = self.cx;
        let companion: &'a crate::classfile::ClassFile = cx.vc_companions.get(&c)?;
        let mut own = vec![self.vc_underlying(c)];
        own.extend(self.erased_param_types(s));
        let ret = self.erased_return_type(s);
        let name = format!("{}$extension", self.target_name(s).unwrap_or_else(|| encode(cx.input.interner.get(cx.input.syms.sym(s).name))));
        let m = pick_method(companion, &name, own.len(), &method_desc(&own, &ret))?;
        Some((companion, m, own, ret, name))
    }

    /// The method a call of `s`, a method of the value class `c`, invokes by `route`, over the
    /// underlying value and the parameters: the companion's `m$extension`, the class's own static
    /// one, or the method of a jar's companion that stands for it (`Gen::jar_extension`).
    pub(super) fn extension_method(&mut self, s: SymId, c: ClassId, route: ExtensionRoute) -> MethodRef {
        let cx = self.cx;
        if route == ExtensionRoute::Jar {
            let (companion, m, _, _, name) = self.jar_extension(c, s).expect("a jar companion's method, by the route");
            let (params, ret) = parse_method_desc(&m.descriptor);
            return MethodRef { owner: Rc::from(companion.name.as_str()), owner_is_interface: false, name, params, ret, desc: m.descriptor.clone(), kind: Invoke::Virtual };
        }
        let mut params = vec![self.vc_underlying(c)];
        params.extend(self.erased_param_types(s));
        let ret = self.erased_return_type(s);
        let name = format!("{}$extension", self.method_name(s));
        let (owner, kind) = match route {
            ExtensionRoute::Companion => (Rc::from(cx.companion_name(c)), Invoke::Virtual),
            _ => (self.class_name(c), Invoke::Static),
        };
        let desc = method_desc(&params, &ret);
        MethodRef { owner, owner_is_interface: false, name, params, ret, desc, kind }
    }

    /// The class the constructor `s` is of; None for a constructor of no class.
    pub(super) fn ctor_class(&self, s: SymId) -> Option<ClassId> {
        match self.cx.input.syms.sym(s).owner {
            Owner::Class(c) => Some(c),
            _ => None,
        }
    }

    pub(super) fn via_route(&mut self, s: SymId, args: ListRef) -> ViaRoute {
        let cx = self.cx;
        let Some(c) = self.ctor_class(s) else { return ViaRoute::Unowned };
        if cx.input.syms.class(c).js != JsKind::Scala {
            return ViaRoute::Js;
        }
        ViaRoute::Instance(c, self.via_arguments_route(c, s, args))
    }

    pub(super) fn via_arguments_route(&mut self, c: ClassId, s: SymId, args: ListRef) -> ViaArguments {
        // A class this build does not write: the secondary as its class file declares it.
        if let Some((outer, declared)) = self.unwritten_ctor_layout(c, Some(s)).filter(|(_, declared)| args.len as usize == declared.len() + 1) {
            return ViaArguments::Unwritten(outer, declared);
        }
        // What a local class captures leads every constructor's parameters (dotty's `LambdaLift.addFreeParams`),
        // passed as a creation through the primary passes them (`lower_new_arguments`).
        if self.via_asks_captures(c, s) {
            return ViaArguments::Captured(self.via_own_captures(c, s).len().min(args.len as usize));
        }
        let declared = self.param_types(s).len();
        match self.via_outer(c, args.len as usize, declared) {
            Some(outer) => ViaArguments::Outer(outer),
            None => ViaArguments::Declared,
        }
    }

    /// Whether the secondary constructor `s` of `c` takes what the class captures, which the
    /// analysis of captures decides.
    pub(super) fn via_asks_captures(&mut self, c: ClassId, s: SymId) -> bool {
        self.via_own_captures(c, s).contains(&true)
    }

    /// For each capture of `c` its second constructor `s` has a parameter for, whether the parameter is
    /// that capture itself (the class's own symbol: the typer's `settle_local_class` gives every constructor
    /// of a local class its captures) and not one of its own (the enclosing instance of a class nested in a
    /// class).
    pub fn via_own_captures(&mut self, c: ClassId, s: SymId) -> Vec<bool> {
        let cx = self.cx;
        let Some(&idx) = cx.tclass_of.get(c.idx()).filter(|&&i| i != u32::MAX) else { return Vec::new() };
        let tc = &cx.input.prog.classes[idx as usize];
        let n = self.capture_count(c).min(tc.ctor_params.len());
        let Some(fun) = tc.ctors.iter().map(|&f| &cx.input.prog.funs[f.idx()]).find(|f| f.sym == s) else { return Vec::new() };
        tc.ctor_params[..n].iter().zip(fun.params.iter()).map(|(a, b)| a == b).collect()
    }

    /// The type of the enclosing instance a secondary constructor of `c` takes first: a class
    /// nested in a class, called with one argument more than the secondary declares.
    fn via_outer(&mut self, c: ClassId, n_args: usize, n_params: usize) -> Option<ErasedType> {
        let cx = self.cx;
        let info = cx.input.syms.class(c);
        match info.owner {
            Owner::Class(o) if matches!(info.kind, ClassKind::Class | ClassKind::GivenImpl) && !matches!(cx.input.syms.class(o).kind, ClassKind::Object | ClassKind::Builtin) && n_args == n_params + 1 => Some(self.class_type(o)),
            _ => None,
        }
    }

    /// The companion object an instance creation of `c` initialises first, where it has one
    /// with an instance.
    pub(super) fn touched_module(&mut self, c: ClassId) -> Option<Rc<str>> {
        let cx = self.cx;
        let companion = crate::emit::touched_companion(cx.input.syms, &cx.layout.has_body, c)?;
        self.module_name(companion)
    }

    pub(super) fn new_route(&mut self, e: TExprId, c: ClassId, args: ListRef) -> NewRoute {
        let cx = self.cx;
        // A value class erases to its underlying value in link mode.
        if cx.input.syms.class(c).value_class {
            if let Some(&a) = cx.input.prog.expr_list(args).first() {
                let u = match self.recorded_type(e).and_then(|rt| self.value_class_type(rt)) {
                    Some((k, targs)) if k == c => self.vc_erasure(c, targs),
                    _ => self.vc_underlying(c),
                };
                return NewRoute::Underlying(u, a);
            }
        }
        if cx.input.syms.class(c).js != JsKind::Scala {
            return NewRoute::Js;
        }
        NewRoute::Instance(self.ctor_arguments_route(c, args))
    }

    /// Whether a creation of `c` passes what the class captures, which the analysis of captures
    /// decides: a class that captures, or an anonymous one.
    pub(super) fn asks_captures(&self, c: ClassId) -> bool {
        self.capture_count(c) > 0 || self.cx.input.syms.class(c).kind == ClassKind::Anon
    }

    pub(super) fn ctor_arguments_route(&mut self, c: ClassId, args: ListRef) -> CtorArguments {
        if self.asks_captures(c) {
            return CtorArguments::Captured;
        }
        if let Some((outer, declared)) = self.unwritten_ctor_layout(c, None).filter(|(_, declared)| args.len as usize == declared.len() + 1) {
            return CtorArguments::Unwritten(outer, declared);
        }
        match self.outer_of_unwritten(c, args) {
            Some(outer) => CtorArguments::UnwrittenOuter(outer),
            None => CtorArguments::Declared,
        }
    }

    pub(super) fn anon_super_route(&mut self, sup: ClassId, via: Option<SymId>, args: ListRef) -> SuperArguments {
        match via {
            None if self.capture_count(sup) > 0 => return SuperArguments::Captured,
            Some(v) if self.via_own_captures(sup, v).contains(&true) => return SuperArguments::Via(v),
            _ => {}
        }
        // A superclass this build does not write: as its class file's constructor takes them,
        // an instance it does not take left out (`anon_super_params`).
        match self.unwritten_ctor_layout(sup, via).filter(|(_, declared)| args.len as usize == declared.len() + 1) {
            Some((outer, declared)) => SuperArguments::Unwritten(outer, declared),
            None => SuperArguments::Declared,
        }
    }

    /// The superclass of an anonymous class whose creation evaluates its superclass's call
    /// (`tir::parent_call_at_creation`: its first parent is a class), the secondary constructor
    /// its arguments go to, and the parameter types of that constructor; nothing for any other
    /// class, whose constructor makes the call.
    pub fn anon_super_params(&mut self, c: ClassId) -> Option<(ClassId, Option<SymId>, Vec<ErasedType>)> {
        let cx = self.cx;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX || !crate::tir::parent_call_at_creation(cx.input.syms, cx.input.types, c) {
            return None;
        }
        let tc = &cx.input.prog.classes[idx as usize];
        let n_args = tc.parent_args?.len as usize;
        let sup = cx.input.syms.class(c).superclass?;
        let via = tc.parent_via;
        // A superclass this build does not write: the selected constructor as its class file
        // declares it, the enclosing instance first where it takes one (`lower_unwritten_ctor_arguments`).
        if let Some((outer, declared)) = self.unwritten_ctor_layout(sup, via).filter(|(_, declared)| n_args == declared.len() + 1) {
            return Some((sup, via, outer.into_iter().chain(declared).collect()));
        }
        let types = match via {
            // A local superclass's second constructor takes its class's captures first (`lower_via_arguments`).
            Some(v) if self.via_own_captures(sup, v).contains(&true) => {
                let mut types = self.anon_ctor_params(sup);
                types.extend(self.param_types(v));
                types
            }
            // A secondary constructor of a class nested in a class takes the enclosing instance
            // first, as every one of its constructors does (`lower_via_arguments`).
            Some(v) => {
                let params = self.param_types(v);
                self.via_outer(sup, n_args, params.len()).into_iter().chain(params).collect()
            }
            None if self.capture_count(sup) > 0 => {
                let mut types = self.anon_ctor_params(sup);
                types.extend(self.ctor_param_types(sup));
                types
            }
            None => self.ctor_param_types(sup),
        };
        Some((sup, via, types))
    }

    /// The type of the enclosing instance a `new` of `c` passes first where this build does not
    /// write `c` and its class file's constructors do not say (`unwritten_ctor_layout`): a class,
    /// a given's class or an enum's class case nested in a class or trait (a class's enum's
    /// companion), made with one argument more than its constructor declares.
    fn outer_of_unwritten(&mut self, c: ClassId, args: ListRef) -> Option<ErasedType> {
        let cx = self.cx;
        if cx.tclass_of[c.idx()] != u32::MAX {
            return None;
        }
        let info = cx.input.syms.class(c);
        let Owner::Class(o) = info.owner else { return None };
        if !matches!(info.kind, ClassKind::Class | ClassKind::GivenImpl | ClassKind::EnumCase) || matches!(cx.input.syms.class(o).kind, ClassKind::Object | ClassKind::Builtin) {
            return None;
        }
        let declared: usize = info.ctor.iter().map(|cl| cl.params.len()).sum();
        (args.len as usize == declared + 1).then(|| self.class_type(o))
    }

    /// The constructor of `c` the typer selected, its primary or the secondary `via`, as the class
    /// file of a class this build does not write declares it (another module's, a jar's): the
    /// type of the enclosing instance where the constructor takes one first (scalac's
    /// `ExplicitOuter` gives it to a class that needs its outer; a class private to its owner
    /// that reads none has no such parameter), and its parameters' types. The alternative is
    /// the one whose parameters are the selected symbol's, alone or after the owner's instance,
    /// never one of the same arity; nothing where two of the class file's constructors fit, or
    /// none does, and for a class of this build or one without a class file (lean products).
    pub(crate) fn unwritten_ctor_layout(&mut self, c: ClassId, via: Option<SymId>) -> Option<(Option<ErasedType>, Vec<ErasedType>)> {
        let cx = self.cx;
        if cx.tclass_of[c.idx()] != u32::MAX {
            return None;
        }
        let cf = cx.class_files.get(&c)?;
        let own: Vec<ErasedType> = match via {
            Some(v) => self.param_types(v),
            None => {
                let params: Vec<SymId> = cx.input.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
                params.into_iter().map(|p| self.sym_type(p)).collect()
            }
        };
        let owner = match cx.input.syms.class(c).owner {
            Owner::Class(o) => Some(self.class_type(o)),
            _ => None,
        };
        let mut found: Option<(Option<ErasedType>, Vec<ErasedType>)> = None;
        for m in cf.methods.iter().filter(|m| m.name == "<init>" && m.access & (ACC_STATIC | ACC_BRIDGE) == 0) {
            let (params, _) = parse_method_desc(&m.descriptor);
            let layout = if params == own {
                (None, params)
            } else if params.len() == own.len() + 1 && Some(&params[0]) == owner.as_ref() && params[1..] == own[..] {
                (Some(params[0].clone()), params[1..].to_vec())
            } else {
                continue;
            };
            if found.is_some() {
                return None;
            }
            found = Some(layout);
        }
        found
    }

    pub(super) fn capture_argument_route(&mut self, a: TExprId, p: SymId) -> CaptureArgument {
        let cx = self.cx;
        match cx.input.prog.expr(a) {
            // A value the class takes where the creation reads another local (the outer
            // instance a local object's lazy val holds) is that local's value.
            TExpr::Local(s) if s != p && self.held(p) == Held::Value && !self.is_local_def(p) && !self.cx.input.syms.sym(p).by_name => CaptureArgument::Value(self.anon_capture_type(p)),
            // A local lazy val is passed as its holder (`LazyVals.transformLocalDef`).
            TExpr::Local(s) if self.is_capturable(s) => CaptureArgument::Held(s),
            // Where the class is made inside a local object, the typer passes the object's own val as
            // `() => this` (`enclosing_object_selfs`): the class takes a holder of that value.
            TExpr::Lambda(params, body) if params.len == 0 && self.is_lazy_local(p) => CaptureArgument::Holder { lazy: p, value: body },
            _ => CaptureArgument::Own,
        }
    }

    /// What a creation of the lifted class `c` passes for its captures, in order: the leading
    /// `n` of its arguments, then what the local defs the class copies capture.
    pub(super) fn passed_captures(&mut self, c: ClassId, args: ListRef, n: usize) -> Vec<PassedCapture> {
        let cx = self.cx;
        let items = cx.input.prog.expr_list(args)[..n].to_vec();
        let passed = cx.input.prog.classes[cx.tclass_of[c.idx()] as usize].ctor_params[..n].iter().filter(|&&s| !self.is_local_def(s)).count();
        let through_defs = self.class_captured(c).split_off(passed);
        let params: Vec<SymId> = cx.input.prog.classes[cx.tclass_of[c.idx()] as usize].ctor_params[..n].to_vec();
        let mut out = Vec::with_capacity(n + through_defs.len());
        for (a, p) in items.into_iter().zip(params) {
            // A local def is not passed: the class gets a copy of the lifted method.
            if matches!(cx.input.prog.expr(a), TExpr::Local(s) if self.is_local_def(s)) {
                continue;
            }
            out.push(PassedCapture::Argument(a, p));
        }
        out.extend(through_defs.into_iter().map(PassedCapture::Held));
        out
    }

    /// What a named local class captures from the capture at `from` on, a local def left out:
    /// the class has a copy of it.
    pub(super) fn class_captures_after(&mut self, c: ClassId, from: usize) -> Vec<SymId> {
        self.class_captured(c).into_iter().skip(from).filter(|&s| !self.is_local_def(s)).collect()
    }

    pub(super) fn is_capturable(&self, s: SymId) -> bool {
        self.cx.input.syms.sym(s).owner == Owner::Local
    }

    pub fn ctor_param_types(&mut self, c: ClassId) -> Vec<ErasedType> {
        let cx = self.cx;
        let params: Vec<SymId> = cx.input.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
        let own: Vec<ErasedType> = params.into_iter().map(|p| self.sym_type(p)).collect();
        // A jar class's constructor is called as its class file declares it.
        let Some(cf) = cx.class_files.get(&c) else { return own };
        match pick_method(cf, "<init>", own.len(), &method_desc(&own, &ErasedType::V)) {
            Some(m) => parse_method_desc(&m.descriptor).0.into_iter().zip(&own).map(|(d, o)| if d == *o { o.clone() } else { d }).collect(),
            None => own,
        }
    }

    /// How many leading constructor arguments of `c` are captured locals: all of an anonymous
    /// class's, the leading ones of a named local class.
    pub fn capture_count(&self, c: ClassId) -> usize {
        let cx = self.cx;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX {
            return 0;
        }
        let tc = &cx.input.prog.classes[idx as usize];
        match cx.input.syms.class(c).kind {
            ClassKind::Anon => tc.ctor_params.len(),
            // A local trait's captures are what its `$init$` takes, held by the classes mixing it in.
            ClassKind::Trait => 0,
            _ => tc.captures,
        }
    }

    /// The constructor of a lifted class takes what its body captured first, a `var` as its cell, then
    /// its `Extras` (`class_extras`).
    pub fn anon_ctor_params(&mut self, c: ClassId) -> Vec<ErasedType> {
        let cx = self.cx;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX {
            return Vec::new();
        }
        let mut out: Vec<ErasedType> = self.class_captured(c).into_iter().map(|s| self.anon_capture_type(s)).collect();
        let extras = self.class_extras(c);
        out.extend(self.extras_types(&extras));
        out
    }

    /// The locals a lifted class holds in fields: what its body names, a local def left out
    /// since the class gets a copy of it, and in its place what that def captures.
    pub fn class_captured(&mut self, c: ClassId) -> Vec<SymId> {
        let cx = self.cx;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX {
            return Vec::new();
        }
        let n = self.capture_count(c);
        let named = &cx.input.prog.classes[idx as usize].ctor_params[..n];
        let mut out: Vec<SymId> = named.iter().copied().filter(|&s| !self.is_local_def(s)).collect();
        let defs: Vec<SymId> = named.iter().copied().filter(|&s| self.is_local_def(s)).collect();
        // A local lazy val is held as its holder, and the class's copy of its accessor takes what the
        // initialiser captures, as a local def's copy does (`LambdaLift`'s free variables of both).
        let lazies: Vec<SymId> = named.iter().copied().filter(|&s| self.is_lazy_local(s)).collect();
        for s in defs {
            let free = self.an.of_copy(cx, s);
            for &f in &free.syms {
                if !out.contains(&f) && !self.is_local_def(f) {
                    out.push(f);
                }
            }
        }
        for s in lazies {
            let free = self.an.of_lazy(cx, s);
            for &f in &free.syms {
                if !out.contains(&f) && !self.is_local_def(f) {
                    out.push(f);
                }
            }
        }
        out
    }

    /// Where the class being written is not the one whose code declares the local def or lazy val `s` and
    /// that one is an object reached statically (or the file's top level), the module its copy of `s`
    /// takes for `this`: dotty's `LambdaLift` lifts `s` into that class, whose instance is its `MODULE$`.
    pub(super) fn copy_home_module(&mut self, s: SymId) -> Option<Rc<str>> {
        let cx = self.cx;
        match cx.local_home(s)? {
            Some(h) if Some(h) != self.this_class && self.static_home(h) => Some(self.class_name(h)),
            None if self.this_class.is_some() => {
                let file = cx.local_file(s).unwrap_or(cx.input.syms.sym(s).file);
                Some(Rc::from(cx.file_modules[file.0 as usize].as_str()))
            }
            _ => None,
        }
    }

    /// Whether the instance of `h` is its `MODULE$`, read where it is wanted.
    pub(super) fn static_home(&self, h: ClassId) -> bool {
        self.cx.input.syms.class(h).kind == ClassKind::Object && self.cx.statically_placed(h)
    }

    /// The class whose instance a copy of the local def or lazy val `s` takes for `this` in the class being
    /// written, where that is another class's whose instance is no `MODULE$` (`Lifted::home`).
    pub(super) fn copy_home(&self, s: SymId) -> Option<ClassId> {
        let h = self.cx.local_home(s)??;
        (Some(h) != self.this_class && !self.static_home(h)).then_some(h)
    }

    /// `free`, the captures of the local def or lazy val `s`, without `this` where a module stands for it
    /// (`copy_home_module`).
    pub(super) fn copy_free(&mut self, s: SymId, free: Rc<Free>) -> Free {
        let mut free = (*free).clone();
        if free.this && self.copy_home_module(s).is_some() {
            free.this = false;
        }
        free
    }

    /// Whether the initialiser of the local lazy val `s` returns from its method, so that the key of
    /// those returns travels with its holder.
    pub(super) fn lazy_keyed(&mut self, s: SymId) -> bool {
        let cx = self.cx;
        self.is_lazy_local(s) && self.an.of_lazy(cx, s).nonlocal
    }

    /// The `Extras` of code holding the locals `declared` (a lifted class's captures, a local trait's
    /// proxies) in the class `own`: the instances that its copies of local defs and lazy vals read, and
    /// the keys of the lazy vals it holds or its copies read.
    pub fn extras_of(&mut self, declared: &[SymId], own: Option<ClassId>) -> Extras {
        let cx = self.cx;
        let mut out = Extras::default();
        let mut lazies: Vec<SymId> = Vec::new();
        for &s in declared {
            if self.is_local_def(s) {
                let free = self.an.of_copy(cx, s);
                if free.this {
                    self.add_home(&mut out, s, own);
                }
                lazies.extend(free.syms.iter().copied().filter(|&f| self.is_lazy_local(f)));
            } else if self.is_lazy_local(s) {
                lazies.push(s);
                let free = self.an.of_lazy(cx, s);
                lazies.extend(free.syms.iter().copied().filter(|&f| self.is_lazy_local(f)));
            }
        }
        for x in lazies {
            let free = self.an.of_lazy(cx, x);
            if free.this {
                self.add_home(&mut out, x, own);
            }
            if free.nonlocal && !out.keys.contains(&x) {
                out.keys.push(x);
            }
        }
        out
    }

    fn add_home(&self, out: &mut Extras, s: SymId, own: Option<ClassId>) {
        if let Some(Some(h)) = self.cx.local_home(s) {
            if Some(h) != own && !self.static_home(h) && !out.homes.contains(&h) {
                out.homes.push(h);
            }
        }
    }

    /// The `Extras` of the lifted class `c`, held in fields after its captures.
    pub fn class_extras(&mut self, c: ClassId) -> Extras {
        let cx = self.cx;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX {
            return Extras::default();
        }
        let n = self.capture_count(c);
        let declared = cx.input.prog.classes[idx as usize].ctor_params[..n].to_vec();
        self.extras_of(&declared, Some(c))
    }

    /// The types the `Extras` `e` are passed as.
    pub fn extras_types(&mut self, e: &Extras) -> Vec<ErasedType> {
        let mut out: Vec<ErasedType> = e.homes.iter().map(|&h| self.class_type(h)).collect();
        out.extend(e.keys.iter().map(|_| ErasedType::object()));
        out
    }

    /// Binds the `Extras` `e` that a method takes from the slot `first` on.
    pub fn bind_extras(&mut self, e: &Extras, first: u16) {
        let mut slot = first;
        for &h in &e.homes {
            self.m.home_slots.push((h, slot));
            slot += 1;
        }
        for &x in &e.keys {
            self.m.lazy_keys.insert(x, slot);
            slot += 1;
        }
    }

    /// The field of a lifted class holding the instance of `h`, the `i`th of its `Extras::homes` (numbered as the
    /// captures' fields are, two homes of one name apart).
    pub fn home_field(&self, h: ClassId, i: usize) -> String {
        format!("{}$this{}", encode(self.cx.input.interner.get(self.cx.input.syms.class(h).name)), i)
    }

    /// The field holding the key that travels with the holder of the local lazy val `x`, the `i`th of the
    /// class's `Extras::keys`.
    pub fn key_field(&self, x: SymId, i: usize) -> String {
        format!("{}$lzyKey{}", encode(self.cx.input.interner.get(self.cx.input.syms.sym(x).name)), i)
    }

    /// Pushes the key travelling with the holder of the local lazy val `x`: a parameter or a field of the
    /// code holding the holder, or in the method declaring the val, that method's own key.
    pub fn load_lazy_key(&mut self, x: SymId) {
        let object = ErasedType::object();
        if let Some(&slot) = self.m.lazy_keys.get(&x) {
            self.load(slot, &object);
            return;
        }
        if self.m.this_home.is_none() && self.m.this_module.is_none() {
            if let Some(field) = self.class_keys.iter().find(|(s, _)| *s == x).map(|(_, f)| f.clone()) {
                let owner = self.this_name.clone();
                self.load_this();
                self.getfield(&owner, &field, &object);
                return;
            }
        }
        match self.m.return_key {
            Some(slot) => self.load(slot, &object),
            None => {
                self.unsupported("a non-local return of a lazy val whose method makes no key");
                self.code.op(op::ACONST_NULL);
                self.code.push(VT::Null);
            }
        }
    }

    pub fn is_local_def(&self, s: SymId) -> bool {
        let info = self.cx.input.syms.sym(s);
        info.kind == SymKind::Def && info.owner == Owner::Local
    }

    pub fn held(&self, s: SymId) -> Held {
        let info = self.cx.input.syms.sym(s);
        if info.owner != Owner::Local {
            Held::Value
        } else if info.kind == SymKind::Var {
            Held::Cell
        } else {
            Held::Value
        }
    }

    /// The values a local trait's member or initialiser takes first (`TFun::captures`, a trait's
    /// `TClass::captures`), held as a lifted class holds its captures (`anon_capture_type`): a
    /// local def by what it captures, which the trait's copy of it reads (`class_captured`).
    pub fn proxy_syms(&mut self, proxies: &[SymId]) -> Vec<SymId> {
        let cx = self.cx;
        let mut out: Vec<SymId> = Vec::with_capacity(proxies.len());
        for &p in proxies {
            if !self.is_local_def(p) {
                if !out.contains(&p) {
                    out.push(p);
                }
                // A local lazy val by its holder and what its initialiser captures (`class_captured`).
                if self.is_lazy_local(p) {
                    let free = self.an.of_lazy(cx, p);
                    for &f in &free.syms {
                        if !out.contains(&f) && !self.is_local_def(f) {
                            out.push(f);
                        }
                    }
                }
                continue;
            }
            let free = self.an.of_copy(cx, p);
            for &f in &free.syms {
                if !out.contains(&f) && !self.is_local_def(f) {
                    out.push(f);
                }
            }
        }
        out
    }

    /// The definition of `s` where it is a local trait's member that takes proxies first.
    pub(super) fn proxied_fun(&self, s: SymId) -> Option<&'a crate::tir::TFun> {
        let cx = self.cx;
        let f = *cx.fun_of_sym.get(s.idx())?;
        if f == u32::MAX {
            return None;
        }
        let fun = &cx.input.prog.funs[f as usize];
        (fun.captures > 0).then_some(fun)
    }

    /// Whether the member `s` of a local trait takes proxies first (`member_proxies`).
    fn takes_proxies(&self, s: SymId) -> bool {
        self.proxied_fun(s).is_some()
    }

    /// The proxies the member `s` of a local trait takes first, when it takes some.
    pub fn member_proxies(&mut self, s: SymId) -> Option<Vec<SymId>> {
        let k = self.proxied_fun(s)?.captures as usize;
        let declared = self.proxied_fun(s)?.params[..k].to_vec();
        Some(self.proxy_syms(&declared))
    }

    /// The `Extras` the member `s` of a local trait takes after its proxies (`extras_of` its declared ones).
    pub fn member_extras(&mut self, s: SymId) -> Extras {
        let cx = self.cx;
        let Some(&f) = cx.fun_of_sym.get(s.idx()) else { return Extras::default() };
        if f == u32::MAX {
            return Extras::default();
        }
        let fun = &cx.input.prog.funs[f as usize];
        let declared = fun.params[..fun.captures as usize].to_vec();
        self.extras_of(&declared, None)
    }

    /// The method `m` taking the proxies first, then their `Extras`.
    pub fn proxied_mref(&mut self, m: &MethodRef, proxies: &[SymId], extras: &Extras) -> MethodRef {
        let mut params: Vec<ErasedType> = proxies.iter().map(|&p| self.anon_capture_type(p)).collect();
        params.extend(self.extras_types(extras));
        params.extend(m.params.iter().cloned());
        let desc = method_desc(&params, &m.ret);
        MethodRef { params, desc, ..m.clone() }
    }

    /// The proxies a method takes first, bound as the locals they stand for: a `var` read
    /// through its cell, a lazy val through its thunk.
    pub fn bind_proxies(&mut self, proxies: &[SymId]) {
        for &p in proxies {
            let held = self.held(p);
            let declared = self.sym_type(p);
            let ty = self.anon_capture_type(p);
            if let Some(local) = self.m.locals.get_mut(&p) {
                match held {
                    Held::Cell => {
                        local.cell = true;
                        local.ty = declared;
                    }
                    _ => local.ty = ty,
                }
            }
        }
    }

    pub fn anon_capture_type(&mut self, s: SymId) -> ErasedType {
        let info = self.cx.input.syms.sym(s);
        if self.held(s) == Held::Cell {
            ErasedType::L(Rc::from(OBJECT_REF))
        } else if self.is_lazy_local(s) {
            self.lazy_holder_type(s)
        } else if info.by_name {
            self.function_type(0)
        } else {
            self.sym_type(s)
        }
    }

    // ---- lambdas and lifted definitions ----

    /// Queues a lifted body. The queue's order is the order of the class's methods, so nothing
    /// is queued while a lowering runs (docs/LIR.md, "What a slice may not do").
    pub(super) fn defer(&mut self, body: Pending) {
        assert!(!self.code.frozen, "a lifted body is queued while a lowering runs");
        if writing() {
            panic!("a write of its class queues the lifted body of {}", body.lifted.name);
        }
        self.pending.push(body);
    }

    /// What the lifted method of the code whose captures are `free` takes and binds: of a lambda (`code`, its
    /// parameters and body), or the copy of the local def or lazy val `copy_of`, its `this` the instance of the
    /// class whose code declares it where that is another class's (`Lifted::home`), else the code's own.
    pub(super) fn lifted_shape(&mut self, free: &Free, params: Vec<ErasedType>, ret: ErasedType, copy_of: Option<SymId>, code: Option<(Vec<SymId>, TExprId)>) -> (Rc<LiftedShape>, Rc<LiftedBody>) {
        let home = copy_of.and_then(|s| self.copy_home(s));
        let mut captures = Vec::new();
        for &s in &free.syms {
            let info = self.cx.input.syms.sym(s);
            if info.kind == SymKind::Def {
                continue;
            }
            // A local lazy val travels as its holder (`LazyVals.transformLocalDef`), by value, the
            // key of its initialiser's returns beside it.
            if self.is_lazy_local(s) {
                let ty = self.lazy_holder_type(s);
                captures.push(Capture { sym: s, cell: false, ty, key: false });
                if self.lazy_keyed(s) {
                    captures.push(Capture { sym: s, cell: false, ty: ErasedType::object(), key: true });
                }
                continue;
            }
            let cell = self.is_cell(s);
            let ty = if info.by_name { self.function_type(0) } else { self.sym_type(s) };
            captures.push(Capture { sym: s, cell, ty, key: false });
        }
        let home = if free.this { home.or(self.m.this_home) } else { None };
        // In code whose `this` is a module read statically, so is the lifted code's.
        let module = if free.this && home.is_none() { self.m.this_module.clone() } else { None };
        let instance = (free.this && module.is_none()).then(|| match home.or(self.this_class) {
            Some(c) => self.class_type(c),
            None => ErasedType::object(),
        });
        let captured = captures.iter().map(|c| if c.cell { ErasedType::L(Rc::from(OBJECT_REF)) } else { c.ty.clone() }).collect();
        let shape = LiftedShape { instance, key: free.nonlocal, captured, params, ret };
        (Rc::new(shape), Rc::new(LiftedBody { code, captures, home, module }))
    }

    /// The key of the non-local returns of the def being written, or the one its closure was given.
    pub(super) fn load_return_key(&mut self) {
        match self.m.return_key {
            Some(slot) => self.load(slot, &ErasedType::object()),
            None => {
                self.unsupported("a non-local return of a def that makes no key");
                self.code.op(op::ACONST_NULL);
                self.code.push(VT::Null);
            }
        }
    }

    /// The method of the closure whose lifted method is `target`, lowered against `shape`: the class's table's entry of
    /// that name, made where the closure was lowered (`Gen::closure_entry`), its body queued where the class's holds first
    /// meet the closure, before any of its code is written (`ask_lifted`): one method however often the node is
    /// written. A lambda at a place of a finalizer is the method of that place in every copy, which the writer may lower
    /// again (`Gen::write_finalizer`). None where the table has no closure of that name, or two methods of it: the class
    /// is refused.
    pub(super) fn lambda_method(&mut self, target: &MethodRef, shape: &Rc<LiftedShape>) -> Option<Rc<Lifted>> {
        let placed = match self.lifted_table.placed(&target.name, &target.desc) {
            Some(Some(placed @ Placed::Lambda(..))) => placed,
            // Refused where the second method of the name and descriptor was named (`Gen::lifted_entry`).
            Some(None) => return None,
            _ => {
                self.unsupported(&format!("internal error: the closure's lifted method {} has no entry in the class", target.name));
                return None;
            }
        };
        let Some((body, (params, code))) = self.lifted_table.made(placed).and_then(|(_, body)| body.code.clone().map(|code| (body, code))) else {
            self.unsupported(&format!("internal error: the closure's lifted method {} has no body in the class", target.name));
            return None;
        };
        let copy = self.m.finally_copies.last_mut().map(|copy| {
            let places = &mut copy.places;
            let at = *copy.lambdas.entry(placed).or_insert_with(|| {
                let n = places.entry(code).or_insert(0);
                *n += 1;
                *n - 1
            });
            (code, copy.of, at)
        });
        let met = self.lambdas.get(&placed).or_else(|| copy.and_then(|k| self.finally_lambdas.get(&k)));
        if let Some(l) = met.cloned() {
            if (&*l.shape, &l.captures, l.home, &l.module) != (&**shape, &body.captures, body.home, &body.module) {
                self.unsupported(&format!("internal error: the lambda {} is written again with other captures", l.name));
            }
            self.lambdas.insert(placed, l.clone());
            return Some(l);
        }
        if writing() {
            panic!("a write of its class makes the first ask of a lambda's method");
        }
        let place = self.lifted_table.place(placed);
        let lifted = Rc::new(Lifted::new(target.name.clone(), shape.clone(), body.captures.clone(), body.home, body.module.clone(), place));
        self.lambdas.insert(placed, lifted.clone());
        if let Some(k) = copy {
            self.finally_lambdas.insert(k, lifted.clone());
        }
        let return_target = if shape.key { self.return_target() } else { ReturnTarget::Unknown };
        self.defer(Pending { lifted: lifted.clone(), body: PendingBody::Lambda(params, code), return_target });
        Some(lifted)
    }

    /// The def a `return` written here leaves: the method's own, or in a closure the one its key came with.
    pub(super) fn return_target(&self) -> ReturnTarget {
        if self.m.closure { self.m.return_target.clone() } else { ReturnTarget::Def(self.m.ret.clone()) }
    }

    /// The number of the `try` whose finalizer is about to be written, the same in every copy of the finalizers
    /// around it: by the one whose copy is being written and its place among that finalizer's `try` blocks.
    fn finalizer_of(&mut self) -> u32 {
        let Some(copy) = self.m.finally_copies.last_mut() else {
            self.tries_written += 1;
            return self.tries_written;
        };
        copy.tries += 1;
        let written = &mut self.tries_written;
        *self.finally_tries.entry((copy.of, copy.tries)).or_insert_with(|| {
            *written += 1;
            *written
        })
    }

    /// One copy of the finalizer of the `try` numbered `of` (`Gen::finalizer_of`), `f`: a lambda at each of its places
    /// the method of that place in the copy written first (`Gen::lambda_method`).
    fn write_finalizer(&mut self, f: LirId, of: u32) {
        self.m.finally_copies.push(FinallyCopy { of, lambdas: FxMap::default(), places: FxMap::default(), tries: 0 });
        self.write(f);
        self.m.finally_copies.pop();
    }

    /// `invokedynamic` through `LambdaMetafactory`: the captures on the stack become an instance
    /// of the function interface whose `apply` is the lifted method.
    /// The type of `this` a lifted method takes: the class's whose code it is, the box of a value
    /// class whose body its companion holds (`extension_bodies`) too.
    pub fn lifted_this_type(&mut self) -> ErasedType {
        match self.m.this_home.or(self.this_class) {
            Some(c) => self.class_type(c),
            None => ErasedType::L(self.this_name.clone()),
        }
    }

    /// The closure whose `method` is the lifted method `target` of the shape `shape`, over the captures on the stack;
    /// `home` the class whose instance the method's code runs on where that is not the class being written.
    pub(super) fn lambda_call_site(&mut self, target: &MethodRef, shape: &LiftedShape, home: Option<ClassId>, interface: &ErasedType, method: &MethodRef) {
        let mut captured: Vec<ErasedType> = Vec::new();
        if shape.instance.is_some() {
            let this = match home {
                Some(h) => self.class_type(h),
                None => self.lifted_this_type(),
            };
            captured.push(this);
        }
        if shape.key {
            captured.push(ErasedType::object());
        }
        captured.extend(shape.captured.iter().cloned());
        self.call_site(target, interface, &method.name, &method.desc, &captured);
    }

    /// The call site of the interface's `method`, whose erased signature is `sam`.
    fn call_site(&mut self, target: &MethodRef, interface: &ErasedType, method: &str, sam: &str, captured: &[ErasedType]) {
        let cp = &mut self.cw.cp;
        let impl_ref = if target.owner_is_interface {
            cp.interface_method(&target.owner, &target.name, &target.desc)
        } else {
            cp.method(&target.owner, &target.name, &target.desc)
        };
        let impl_handle = cp.method_handle(6, impl_ref);
        let sam_type = cp.method_type(sam);
        let factory = cp.method(
            "java/lang/invoke/LambdaMetafactory",
            "metafactory",
            "(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/invoke/MethodType;Ljava/lang/invoke/MethodHandle;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;",
        );
        let factory_handle = cp.method_handle(6, factory);
        let bootstrap = self.cw.bootstrap_method(factory_handle, &[sam_type, impl_handle, sam_type]);
        let site_desc = method_desc(captured, interface);
        let site = self.cw.cp.invoke_dynamic(bootstrap, method, &site_desc);
        self.code.op_u16(op::INVOKEDYNAMIC, site);
        self.code.bytes.extend_from_slice(&[0, 0]);
        self.code.popn(captured.len());
        let vt = self.vt(interface);
        self.code.push(vt);
    }

    /// What the lifted method of the local def `s` takes and binds, decided where the def is first met.
    pub(super) fn local_fun_shape(&mut self, s: SymId) -> (Rc<LiftedShape>, Rc<LiftedBody>) {
        if let Some(found) = self.lifted_table.made(Placed::Def(s)) {
            return found;
        }
        let cx = self.cx;
        let free = self.an.of_fun(cx, s);
        let free = self.copy_free(s, free);
        let params = self.param_types(s);
        let ret = self.return_type(s);
        let made = self.lifted_shape(&free, params, ret, Some(s), None);
        self.lifted_table.keep_made(Placed::Def(s), made.clone());
        made
    }

    /// Queues the methods of the local defs a lowering met (`Gen::lowering`, once it ends), in the context of the
    /// method that code is written in, so that the class writing a local def's declaration writes its method
    /// whether or not a call of the class reaches it (`LambdaLift.liftDef` lifts every local def of the class).
    pub(super) fn queue_local_defs(&mut self) {
        for s in std::mem::take(&mut self.local_defs) {
            self.lifted_fun(s);
        }
    }

    /// The lifted method of the local def `s`, named and its body queued where the class first asks for it.
    pub fn lifted_fun(&mut self, s: SymId) -> Rc<Lifted> {
        if let Some(l) = self.lifted.get(&s) {
            return l.clone();
        }
        if writing() {
            panic!("a write of its class makes the first ask of a local def's method");
        }
        let cx = self.cx;
        let (shape, body) = self.local_fun_shape(s);
        let (name, place) = self.lifted_entry(Placed::Def(s), Asker::LiftedFun, &shape);
        let lifted = Rc::new(Lifted::new(name, shape, body.captures.clone(), body.home, body.module.clone(), place));
        self.lifted.insert(s, lifted.clone());
        // A local inline method has its getters alone (`Desugar.addDefaultGetters`), which
        // take their names from it.
        let f = cx.fun_of_sym[s.idx()];
        if f != u32::MAX && cx.input.prog.funs[f as usize].body.is_some() {
            self.defer(Pending { lifted: lifted.clone(), body: PendingBody::Fun(FunId(f)), return_target: ReturnTarget::Unknown });
        }
        lifted
    }

    /// Whether parameter `i` of the def `s`, flattened, is by-name.
    pub(super) fn local_param_by_name(&self, s: SymId, i: usize) -> bool {
        self.cx.input.syms.sym(s).sig.as_ref().is_some_and(|sig| clause_shape(&sig.clauses, i).1.is_some())
    }

    /// What the getter of the default `i` of the local def `s` takes and binds: its default's own captures
    /// (`free::Analysis::of_default`), then the parameters a member's getter of the same shape takes (`Gen::default_getter_shape`: those of the clauses before
    /// the default's, in the class file's order); it returns what that getter returns, a by-name parameter's value
    /// (`Desugar.addDefaultGetters`).
    pub(super) fn local_default_shape(&mut self, s: SymId, i: usize) -> (Rc<LiftedShape>, Rc<LiftedBody>) {
        if let Some(found) = self.lifted_table.made(Placed::Default(s, i)) {
            return found;
        }
        let cx = self.cx;
        let free = Rc::new(self.an.of_default(cx, s, i));
        let free = self.copy_free(s, free);
        let (def, _) = self.local_fun_shape(s);
        let shape = self.default_getter_shape(s, i);
        let params: Vec<ErasedType> = def.params[..shape.before].to_vec();
        let made = self.lifted_shape(&free, params, shape.ret, Some(s), None);
        self.lifted_table.keep_made(Placed::Default(s, i), made.clone());
        made
    }

    /// The getter of the default `i` of the local def `s`, named by its unit and placed by the walk, and its body to
    /// queue where the getter is first called (none where it was made before).
    pub(super) fn local_default_method(&mut self, s: SymId, i: usize) -> (Rc<Lifted>, Option<Pending>) {
        if let Some(l) = self.lifted_defaults.get(&(s, i)) {
            return (l.clone(), None);
        }
        if writing() {
            panic!("a write of its class makes the first ask of a local default's getter");
        }
        let f = self.cx.fun_of_sym[s.idx()];
        let (shape, body) = self.local_default_shape(s, i);
        let (name, place) = self.lifted_entry(Placed::Default(s, i), Asker::LocalDefaultMethod, &shape);
        let l = Rc::new(Lifted::new(name, shape, body.captures.clone(), body.home, body.module.clone(), place));
        self.lifted_defaults.insert((s, i), l.clone());
        (l.clone(), Some(Pending { lifted: l, body: PendingBody::Default(FunId(f), i), return_target: ReturnTarget::Unknown }))
    }

    // ---- blocks ----

    /// The erased elements of the tuple type `ct`, a value class as no primitive (the
    /// specialisation does not take it, whatever it erases to).
    pub(super) fn tuple_definition_elements(&mut self, ct: TypeId) -> Vec<ErasedType> {
        let cx = self.cx;
        match cx.input.types.get(cx.input.types.underlying(ct)) {
            Type::Class(_, args) => cx.input.types.items(args).to_vec().into_iter().map(|a| if self.value_class_of(a).is_some() { ErasedType::object() } else { self.erase(a) }).collect(),
            _ => Vec::new(),
        }
    }

    /// dotty's `SpecializeTuples` accessor of the element `i` of a tuple of the elements `elems`:
    /// `_1$mcI$sp` where every element of a `Tuple1` or `Tuple2` is a primitive its specialisation
    /// takes (Int, Long, Double; for `Tuple2` Char and Boolean too), with its type, else `_i` of
    /// `Object`.
    pub(super) fn tuple_accessor(elems: &[ErasedType], i: usize) -> (String, ErasedType) {
        let code = |t: &ErasedType| match (t, elems.len()) {
            (ErasedType::I, _) => Some("I"),
            (ErasedType::J, _) => Some("J"),
            (ErasedType::D, _) => Some("D"),
            (ErasedType::C, 2) => Some("C"),
            (ErasedType::Z, 2) => Some("Z"),
            _ => None,
        };
        if elems.len() <= 2 && elems.iter().all(|t| code(t).is_some()) {
            (format!("_{}$mc{}$sp", i + 1, code(&elems[i]).unwrap()), elems[i].clone())
        } else {
            (format!("_{}", i + 1), ErasedType::object())
        }
    }

    /// The elements of a tuple pattern past 22 (`case (a1, .., an)` on scalac's `TupleXXL`: the typer's
    /// `xxl_elements_pattern`, a test of the class over `TupleXXL.unapplySeq`'s `Some` of a sequence of exactly `n`)
    /// where each is a wildcard, a variable or a typed one: a `MatchableTuple` `Gen::lower_xxl_definition` lowers.
    pub(super) fn xxl_definition_items(&mut self, pat: TPatId) -> Option<Vec<TPatId>> {
        let prog = self.cx.input.prog;
        let TPat::Test(_, _, u) = prog.pats[pat.idx()] else { return None };
        let TPat::Unapply(_, call, some) = prog.pats[u.idx()] else { return None };
        let TExpr::CallMethod(r, _, _) = prog.expr(call) else { return None };
        let TExpr::Module(m) = prog.expr(r) else { return None };
        if !self.class_name(m).starts_with("scala/runtime/TupleXXL") {
            return None;
        }
        let TPat::Class(_, _, _, subs) = prog.pats[some.idx()] else { return None };
        let [seq] = prog.pat_lists[subs.range()] else { return None };
        let TPat::Seq(items, None) = prog.pats[seq.idx()] else { return None };
        let items = prog.pat_lists[items.range()].to_vec();
        items.iter().all(|&p| matches!(prog.pats[p.idx()], TPat::Wildcard | TPat::Bind(_, None)) || typed_element(prog, p).is_some()).then_some(items)
    }

    /// The variables of a pattern definition of the general match, in `getVariables`' order.
    pub(super) fn definition_variables(&self, pat: TPatId) -> Vec<SymId> {
        let mut vars = Vec::new();
        pattern_variables(self.cx.input.prog, pat, &mut vars);
        vars
    }

    // ---- primitive operations ----

    /// The runtime's helper `m` applied to its arguments, which are evaluated: its object goes
    /// under them.
    pub(super) fn invoke_helper(&mut self, m: &MethodRef) {
        let mark = self.code.locals_mark();
        let mut temps = Vec::new();
        for t in m.params.iter().rev() {
            temps.push((self.store_new(t), t.clone()));
        }
        let owner_type = ErasedType::L(m.owner.clone());
        self.getstatic(&m.owner, "MODULE$", &owner_type);
        for (slot, t) in temps.iter().rev() {
            self.load(*slot, t);
        }
        self.code.locals_release(mark);
        self.invoke_mref(m);
    }

}

/// `ScalaRunTime`'s method that does the operation `name` (`array_apply`, `array_update`,
/// `array_length`, `array_clone`) on an array whose kind only the value knows, as dotty's erasure
/// calls it on a generic array.
pub(super) fn erased_array_method(name: &str) -> MethodRef {
    let object = ErasedType::object();
    let (params, ret) = match name {
        "array_apply" => (vec![object.clone(), ErasedType::I], object),
        "array_update" => (vec![object.clone(), ErasedType::I, object], ErasedType::V),
        "array_length" => (vec![object], ErasedType::I),
        "array_clone" => (vec![object.clone()], object),
        _ => unreachable!("no array operation {}", name),
    };
    let desc = method_desc(&params, &ret);
    MethodRef { owner: Rc::from(SCALA_RUNTIME), owner_is_interface: false, name: name.to_string(), params, ret, desc, kind: Invoke::Static }
}

/// The instructions that make a primitive `b` of a primitive `a`: between the stack classes
/// first (an int, a long, a float, a double), then narrowed to the small integer type wanted.
pub(super) fn primitive_conversion(a: &ErasedType, b: &ErasedType) -> Vec<u8> {
    let mut insns: Vec<u8> = Vec::with_capacity(2);
    match (a.stack_type(), b.stack_type()) {
        (ErasedType::I, ErasedType::J) => insns.push(op::I2L),
        (ErasedType::I, ErasedType::F) => insns.push(op::I2F),
        (ErasedType::I, ErasedType::D) => insns.push(op::I2D),
        (ErasedType::J, ErasedType::I) => insns.push(op::L2I),
        (ErasedType::J, ErasedType::F) => insns.push(op::L2F),
        (ErasedType::J, ErasedType::D) => insns.push(op::L2D),
        (ErasedType::F, ErasedType::I) => insns.push(op::F2I),
        (ErasedType::F, ErasedType::J) => insns.push(op::F2L),
        (ErasedType::F, ErasedType::D) => insns.push(op::F2D),
        (ErasedType::D, ErasedType::I) => insns.push(op::D2I),
        (ErasedType::D, ErasedType::J) => insns.push(op::D2L),
        (ErasedType::D, ErasedType::F) => insns.push(op::D2F),
        _ => {}
    }
    let narrowed_already = matches!((a, b), (ErasedType::B, ErasedType::S) | (ErasedType::B | ErasedType::S | ErasedType::Z, ErasedType::I));
    if !narrowed_already {
        match b {
            ErasedType::B => insns.push(op::I2B),
            ErasedType::S => insns.push(op::I2S),
            ErasedType::C => insns.push(op::I2C),
            _ => {}
        }
    }
    insns
}

/// The stack type of a comparison or operation between operands of the given ranks.
pub(super) fn rank_type(rank: u8) -> ErasedType {
    use crate::typer::prims::*;
    match rank {
        R_LONG => ErasedType::J,
        R_FLOAT => ErasedType::F,
        R_DOUBLE => ErasedType::D,
        _ => ErasedType::I,
    }
}

fn prim_type(op: PrimOp) -> ErasedType {
    use PrimOp::*;
    match op {
        IntAdd | IntSub | IntMul | IntDiv | IntRem | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr => ErasedType::I,
        LongAdd | LongSub | LongMul | LongDiv | LongRem | LongAnd | LongOr | LongXor | LongShl | LongShr | LongUshr => ErasedType::J,
        DoubleAdd | DoubleSub | DoubleMul | DoubleDiv | DoubleRem => ErasedType::D,
        FloatAdd | FloatSub | FloatMul | FloatDiv | FloatRem => ErasedType::F,
        _ => ErasedType::Z,
    }
}

fn unary_type(op: UnOp) -> ErasedType {
    use UnOp::*;
    match op {
        IntNeg | IntNot | LongToInt | DoubleToInt | CharToInt | FloatToInt | ByteToInt | ShortToInt => ErasedType::I,
        LongNeg | LongNot | IntToLong | DoubleToLong | CharToLong | FloatToLong => ErasedType::J,
        DoubleNeg | LongToDouble | FloatToDouble | IntToDouble => ErasedType::D,
        FloatNeg | IntToFloat | LongToFloat | DoubleToFloat => ErasedType::F,
        BoolNot => ErasedType::Z,
        IntToChar => ErasedType::C,
        ByteToShort => ErasedType::S,
        IntToByte => ErasedType::B,
        IntToShort => ErasedType::S,
    }
}

/// The method of a class file that a call of `name` with `arity` parameters and teq's erased
/// descriptor `own` resolves to: the exact descriptor, else the one with the most parameters
/// alike, the first of them on a tie; never a static method or a bridge.
/// The bound of a higher-kinded parameter applied to its arguments (`Gen::applied_upper`): a type
/// that erases as the application does, or the array of an argument.
enum Applied {
    Is(TypeId),
    ArrayOf(TypeId),
}

/// What a part of a union erases to, as scalac's `erasedLub` reads it: a class whose bases it
/// walks, a value class, an array of such a part, the bottom types, or an erasure of another kind.
#[derive(Clone, PartialEq)]
enum Lub {
    Class(ClassId),
    /// A value class and its erasure, its underlying value's.
    Value(ClassId, ErasedType),
    Array(Box<Lub>),
    Nothing,
    Null,
    Erased(ErasedType),
}

/// scalac's `specialErasure`: no tuple class at run time extends the tuple supertypes, which
/// erase to what every tuple is, and `Singleton` is a reference of any class.
fn special_erasure(cx: &Cx, c: ClassId) -> Option<&'static str> {
    match scala_class(cx, c)? {
        "Tuple" | "NonEmptyTuple" => Some(PRODUCT),
        "Singleton" => Some(OBJECT),
        _ => None,
    }
}

/// A typed element of a tuple pattern, `x: T` or `_: T` (dotty's `isVarPattern` or `Typed(Ident, tpt)`; `x @ (_: T)` is
/// neither): its test, the tested type and the variable.
pub(super) fn typed_element(prog: &Program, p: TPatId) -> Option<(TestId, TypeId, Option<SymId>)> {
    let wild = |w: TPatId| matches!(prog.pats[w.idx()], TPat::Wildcard);
    match prog.pats[p.idx()] {
        TPat::Test(test, t, inner) if wild(inner) => Some((test, t, None)),
        TPat::Test(test, t, inner) => match prog.pats[inner.idx()] {
            TPat::Bind(sym, None) => Some((test, t, Some(sym))),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `method` is scala-library's own case accessor `_i` of `scala.TupleN` (`Gen::tuple_accessor` names it, a
/// specialised `TupleN$mcXY$sp` one included): a stable member of a final class, which no class overrides. Another
/// class's `_i` (a case class's, a `Product`'s) is not.
fn is_tuple_accessor(method: &MethodRef) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let Some(class) = method.owner.strip_prefix("scala/Tuple") else { return false };
    let arity = class.split("$mc").next().unwrap_or("");
    let name = method.name.split("$mc").next().unwrap_or("");
    let specialised = class.len() > arity.len();
    digits(arity) && (!specialised || class.ends_with("$sp")) && method.params.is_empty() && name.strip_prefix('_').is_some_and(digits)
}

/// The variable of an element of a tuple pattern `Gen::lower_tuple_definition` lowers.
pub(super) fn tuple_variable(prog: &Program, p: TPatId) -> Option<SymId> {
    match prog.pats[p.idx()] {
        TPat::Bind(sym, None) => Some(sym),
        _ => typed_element(prog, p).and_then(|(_, _, sym)| sym),
    }
}

/// The class `SpecializeTuples` makes of a pair of the elements' types, each I, J, D, C or Z (`Tuple2$mcII$sp`);
/// none for another tuple, which `TupleN$.apply` makes.
pub(super) fn specialised_tuple(elems: &[ErasedType]) -> Option<String> {
    let code = |t: &ErasedType| match t {
        ErasedType::I => Some('I'),
        ErasedType::J => Some('J'),
        ErasedType::D => Some('D'),
        ErasedType::C => Some('C'),
        ErasedType::Z => Some('Z'),
        _ => None,
    };
    let codes: String = if elems.len() == 2 { elems.iter().map(code).collect::<Option<String>>()? } else { return None };
    Some(format!("scala/Tuple2$mc{codes}$sp"))
}

/// The variables a pattern binds, in the order they are written (dotty's `getVariables`).
fn pattern_variables(prog: &Program, p: TPatId, out: &mut Vec<SymId>) {
    match prog.pats[p.idx()] {
        TPat::Wildcard | TPat::Equals(..) => {}
        TPat::Bind(sym, inner) => {
            out.push(sym);
            if let Some(i) = inner {
                pattern_variables(prog, i, out);
            }
        }
        TPat::Test(_, _, inner) | TPat::Unapply(_, _, inner) => pattern_variables(prog, inner, out),
        TPat::Class(_, _, _, subs) | TPat::Alt(subs) => {
            for &q in &prog.pat_lists[subs.range()] {
                pattern_variables(prog, q, out);
            }
        }
        TPat::Seq(items, rest) => {
            for &q in &prog.pat_lists[items.range()] {
                pattern_variables(prog, q, out);
            }
            if let Some(r) = rest {
                pattern_variables(prog, r, out);
            }
        }
    }
}

/// The elements of a tuple class, `Tuple2`'s two.
fn tuple_arity(cx: &Cx, c: ClassId) -> Option<usize> {
    scala_class(cx, c)?.strip_prefix("Tuple")?.parse().ok()
}

/// The name of a top-level class of the package `scala`, which scalac's erasure treats by name.
fn scala_class<'c>(cx: &'c Cx, c: ClassId) -> Option<&'c str> {
    let info = cx.input.syms.class(c);
    let Owner::Package(p) = info.owner else { return None };
    let pkg = cx.input.syms.pkg(p);
    let top_level = pkg.parent.map_or(false, |r| cx.input.syms.pkg(r).parent.is_none());
    if info.kind == ClassKind::Object || !top_level || cx.input.interner.get(pkg.name) != "scala" {
        return None;
    }
    Some(cx.input.interner.get(info.name))
}

fn pick_method<'c>(cf: &'c crate::classfile::ClassFile, name: &str, arity: usize, own: &str) -> Option<&'c crate::classfile::Method> {
    let (own_params, own_ret) = parse_method_desc(own);
    let mut best: Option<(&crate::classfile::Method, usize)> = None;
    for m in cf.methods.iter().filter(|m| m.name == name && m.access & (ACC_STATIC | ACC_BRIDGE) == 0) {
        if m.descriptor == own {
            return Some(m);
        }
        let (params, ret) = parse_method_desc(&m.descriptor);
        if params.len() != arity {
            continue;
        }
        let score = 2 * params.iter().zip(&own_params).filter(|(a, b)| a == b).count() + (ret == own_ret) as usize;
        if best.map_or(true, |(_, b)| score > b) {
            best = Some((m, score));
        }
    }
    best.map(|(m, _)| m)
}

/// A primitive's class in scalac's names, which orders two primitives in an intersection.
fn scala_name_of(t: &ErasedType) -> &'static str {
    match t {
        ErasedType::Z => "scala.Boolean",
        ErasedType::B => "scala.Byte",
        ErasedType::C => "scala.Char",
        ErasedType::S => "scala.Short",
        ErasedType::I => "scala.Int",
        ErasedType::J => "scala.Long",
        ErasedType::F => "scala.Float",
        ErasedType::D => "scala.Double",
        _ => "scala.Unit",
    }
}

/// The type an element descriptor of an array names.
fn desc_type(d: &str) -> ErasedType {
    match d.as_bytes().first() {
        Some(b'Z') => ErasedType::Z,
        Some(b'B') => ErasedType::B,
        Some(b'C') => ErasedType::C,
        Some(b'S') => ErasedType::S,
        Some(b'I') => ErasedType::I,
        Some(b'J') => ErasedType::J,
        Some(b'F') => ErasedType::F,
        Some(b'D') => ErasedType::D,
        Some(b'L') => ErasedType::L(Rc::from(d[1..].trim_end_matches(';'))),
        _ => ErasedType::L(Rc::from(d)),
    }
}

/// Where the clause of flattened parameter `i` starts among `clauses`, and its type where the
/// parameter is by-name: what a constructor's default getter takes and returns.
pub(super) fn clause_shape(clauses: &[ClauseSig], i: usize) -> (usize, Option<TypeId>) {
    let mut start = 0;
    for cl in clauses {
        if i < start + cl.params.len() {
            let p = &cl.params[i - start];
            return (start, p.by_name.then_some(p.ty));
        }
        start += cl.params.len();
    }
    (i, None)
}
