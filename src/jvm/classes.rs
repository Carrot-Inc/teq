//! Classes, traits, objects, enums and the module class of a file's top-level definitions.

use crate::lir::{ClassFlags, Comparing, ErasedType, FieldFlags, Invoke, Kind, LirId, MethodFlags, MethodRef, Refusal, Relation, Unlowered, Visibility};
use super::classfile::*;
use super::free;
use super::gen::*;
use super::names::*;
use super::plan::{begun_method_def, method_access, MODULE_FIELD};
use super::runtime::LAUNCHER;
use crate::ast::mods;
use crate::intern::{FxMap, Name};
use crate::names as n;
use crate::source::FileId;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::rc::Rc;

/// What a trait read from a class file gives a class mixing it in (`class_file_trait_fields`):
/// the fields its TASTy's members name, and what the class file alone declares
/// (`class_file_storage`): the abstract `T$_setter_$x_$eq` setters, with their descriptors, of
/// the vals the members do not name, and a Scala 2 trait's private vars, private lazy vals and
/// objects, which the transcoded pickle leaves out or has no static initialiser for.
pub(super) struct ClassFileTrait {
    fields: Vec<FieldDef>,
    unmatched: Vec<(String, String)>,
    /// The names of its constants, final vals whose getter the trait defines, of no field.
    constants: Vec<String>,
    /// A private var's expanded name (`s2$T$$count`) with its abstract getter's type.
    vars: Vec<(String, ErasedType)>,
    /// A member held in a lazy holder: its getter's name and type, and how it is computed.
    holders: Vec<(String, ErasedType, HolderInit)>,
}


/// What a lazy member's initialiser computes (`Gen::thread_safe_init`): the right-hand side
/// dotty's tree holds, the forwarder's per `Mixin` for a trait's member.
#[derive(Clone)]
pub(super) enum HolderInit {
    /// The trait's static `x$(T)`, which runs the lazy val's or object's initialiser.
    TraitStatic(Rc<str>),
    Expr(TExprId),
    /// A Scala 2 trait's object, or the companion of a case class nested in the class: its
    /// module class made with the instance as its outer reference, `new T$cell$(this)` (dotty's
    /// `Mixin`, 286-295).
    Module { class: Rc<str>, outer: Rc<str> },
}

/// A field of a class with the symbol it stores.
#[derive(Clone)]
struct FieldDef {
    sym: SymId,
    name: String,
    ty: ErasedType,
    /// A lazy val or a given: the field holds the boxed value once it is computed.
    lazy: bool,
    /// For a lazy member of a trait: the trait whose static initialiser computes the value.
    lazy_owner: Option<ClassId>,
    init: Option<TExprId>,
    mutable: bool,
    /// A `final val` of a constant type (`tir::constant_val`): no field, a getter answering the
    /// constant, the initialiser run for its effects where it is no literal.
    constant: Option<ConstVal>,
}

/// Where the parts of a constructor's body go as they are lowered: into the one tree of the
/// body, or written each in its turn, where a part has no nodes yet and `Gen`'s own code
/// writes it.
pub(super) enum Parts {
    /// The statements, and the fields the class declares where the class file meets them.
    Tree(Vec<LirId>, Vec<Declared>),
    Written,
}

/// A field a class declares before the statement at `at` of an object's constructor tree, where
/// the class file meets it: a field enters the constant pool as it is declared, so its place among
/// the statements is the class file's order (`Gen::write_module_part`).
pub(super) struct Declared {
    pub(super) at: usize,
    pub(super) flags: FieldFlags,
    pub(super) name: String,
    pub(super) ty: ErasedType,
}

/// How a primary constructor passes the arguments of its superclass's constructor, after the
/// name and ordinal a value of a Java enum passes up: one decision (`Gen::super_call_route`).
enum SuperCall {
    /// The parameters an anonymous class takes for them, of these types.
    Own(Vec<ErasedType>),
    /// The written arguments at the declared parameters' types (none where the superclass
    /// takes none).
    Declared(Vec<ErasedType>, crate::ast::ListRef),
    /// As the class file of a superclass this build does not write takes them.
    Unwritten(ClassId, crate::ast::ListRef),
    /// Through a secondary constructor of the superclass (a Java superclass's selected
    /// constructor among them).
    Via(ClassId, SymId, crate::ast::ListRef),
    /// What the superclass, a class nested in a class or local, captures first.
    Captured(ClassId, crate::ast::ListRef),
    /// Every parameter's default, none written. Not lowered yet.
    Defaults(ClassId, Vec<ErasedType>),
    /// An enum case without the arguments of its enum.
    Refused,
}

/// How a secondary constructor's call of another constructor of its class passes the
/// arguments: one decision (`Gen::self_call_route`).
#[derive(Clone, Copy)]
enum SelfCall {
    /// To the primary constructor, at its declared parameters' types (`CtorArguments::Declared`).
    Declared(ClassId, crate::ast::ListRef),
    /// To the primary constructor otherwise: what the class captures first.
    Primary(ClassId, crate::ast::ListRef),
    /// To another secondary constructor.
    Via(ClassId, SymId, crate::ast::ListRef),
}

/// Where a value class's synthesized member finds the underlying value (`Gen::vc_member_tree`):
/// a temporary the body declares first, or a parameter of the method at a position.
#[derive(Clone, Copy)]
pub(super) enum Underlying {
    Kept(crate::lir::TempId),
    Param(usize),
}

impl Underlying {
    fn node(self, g: &mut Gen<'_>, u: &ErasedType) -> LirId {
        match self {
            Underlying::Kept(held) => g.lir.kept(held, u.clone()),
            Underlying::Param(i) => g.param(i),
        }
    }
}

/// Whether a build prints, for each method it writes, why parts of it were left to the old
/// generator (`TEQ_LIR_STATS` set): one line `LIRGAPS<TAB><class>.<name><desc><TAB><reasons of
/// its nodes><TAB><reasons of its code outside nodes><TAB><instructions outside nodes>`, the
/// reasons the names of `lir::Unlowered`, `-` for none.
fn lir_stats() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("TEQ_LIR_STATS").is_some())
}

/// Whether a value class's synthesized member is a tree (`Gen::vc_member_tree`). Not
/// `productElement` and `productElementName`, whose index is tested by `ifne` (an `Int` against
/// zero), nor `equals` over a reference, which holds the other's value in a slot while this one's
/// is adapted under it.
pub(super) fn vc_member_lowers(member: &str, u: &ErasedType) -> bool {
    !matches!(member, "productElement" | "productElementName") && !(member == "equals" && u.is_ref())
}

/// An object's constructor tree, kept from the first of the class file's two methods written
/// from it to the second (`Gen::write_module`), with the locals its lowering declared.
pub(super) struct ModuleTree {
    lir: crate::lir::Body,
    declared: FxMap<SymId, (ErasedType, bool)>,
    root: LirId,
    fields: Vec<Declared>,
}

/// A primary constructor as it is decided before its body is lowered.
struct PrimaryCtor<'t> {
    tc: &'t TClass,
    fields: &'t [FieldDef],
    super_name: &'t str,
    fields_first: bool,
    is_value_case: bool,
    is_object: bool,
    is_anon: bool,
    /// It passes its name and ordinal up to `java.lang.Enum`.
    java_enum: bool,
    n_captures: usize,
    /// The position among the parameters of a value of an enum's name, its ordinal after it.
    name_param: usize,
    /// The position of the first parameter an anonymous class passes its superclass.
    first_super_arg: usize,
    route: SuperCall,
}

impl PrimaryCtor<'_> {
    /// The parameters a value of a Java enum passes up first: its name and ordinal.
    fn leading_params(&self) -> Vec<usize> {
        if self.java_enum { vec![self.name_param, self.name_param + 1] } else { Vec::new() }
    }
}

impl<'a> Gen<'a> {
    fn is_lazy_member(&self, s: SymId) -> bool {
        self.cx.is_lazy_member(s)
    }

    // ---- methods ----

    /// Starts a method of the class being written, by its flags, its name and its signature as
    /// a class file spells it: `this` in slot 0 unless static, then the parameters. The types
    /// the code holds its parameters at are not always the signature's: a method that implements
    /// a class file's holds a value class's underlying value as the erasure marks it
    /// (`Gen::linked_mref`).
    ///
    /// The method joins the class's plan here, unless the class holds one of its name and
    /// signature already: the first one begun is the class's. A later one's code is written all
    /// the same and left out at its end (`end_method`).
    pub fn begin_method(&mut self, flags: MethodFlags, name: &str, desc: &str, this_uninit: bool, params: &[(Option<SymId>, ErasedType)], ret: ErasedType, roots: &[TExprId]) {
        let method = begun_method_def(flags, name, desc, params, &ret);
        self.repeated = if self.plan.has_method(name, desc) {
            Some(method)
        } else {
            self.plan.methods.push(method);
            None
        };
        let mut locals: Vec<VT> = Vec::new();
        let mut m = MethodCx::default();
        if !flags.is_static {
            let this = self.this_name.clone();
            locals.push(if this_uninit { VT::UninitThis } else { VT::Object(self.cw.cp.class(&this)) });
            m.this_slot = Some(0);
        }
        for (sym, ty) in params {
            let slot = locals.len() as u16;
            let vt = self.vt(ty);
            locals.push(vt);
            if ty.wide() {
                locals.push(VT::Top2);
            }
            if let Some(s) = sym {
                m.locals.insert(*s, Local { slot, ty: ty.clone(), cell: false });
            }
            m.params.push((slot, ty.clone()));
        }
        m.ret = ret;
        let syms: Vec<SymId> = params.iter().filter_map(|p| p.0).collect();
        m.cells = free::cells(self.cx, &mut self.an, &syms, roots);
        self.code = Code::new(locals);
        self.code.wide = self.wide;
        self.lir.clear();
        self.lower_declared.clear();
        self.m = m;
    }

    /// Ends the method begun: its code goes to the class file under the plan's header. A
    /// method whose code could not be written is none of the class's, and leaves the plan. The
    /// code of a method the class held already (`begin_method`) is dropped, after it was written
    /// for all that writing it leaves in the class: the constants and the bootstrap methods its
    /// instructions name, the names it takes and the lifted bodies it asks for (`fresh_name`,
    /// `defer`, the memos of what is lifted), the interfaces its `super` calls go to
    /// (`super_targets`), the function arities it names and what it reports. It asks for no
    /// second writing of the class with wide jumps (`overflow`).
    pub fn end_method(&mut self) {
        let code = std::mem::replace(&mut self.code, Code::new(Vec::new()));
        let repeated = self.repeated.take();
        if code.underflow || code.misreserved {
            let m = repeated.or_else(|| self.plan.methods.pop()).expect("a method was begun");
            let line = code.last_line().map_or(String::new(), |l| format!(" at line {}", l));
            let fault = if code.underflow { "the operand stack model underflowed" } else { "internal error: a value stored in slots reserved for another width" };
            self.unsupported(&format!("{} while writing {}{}{}", fault, m.name, m.desc, line));
            return;
        }
        if repeated.is_some() {
            return;
        }
        let m = self.plan.methods.last().expect("a method was begun");
        if lir_stats() {
            let named = |why: &mut Vec<Unlowered>| {
                why.sort_unstable();
                why.dedup();
                let names: Vec<String> = why.iter().map(|why| format!("{:?}", why)).collect();
                if names.is_empty() { "-".to_string() } else { names.join(",") }
            };
            let (nodes, outside) = (named(&mut self.m.unlowered), named(&mut self.m.outside));
            self.gaps_lines.push(format!("LIRGAPS\t{}.{}{}\t{}\t{}\t{}", self.this_name, m.name, m.desc, nodes, outside, code.outside));
        }
        self.cw.method(method_access(m.flags), &m.name, &m.desc, Some(code));
        self.overflow |= self.cw.overflow;
    }

    /// The lifted methods that the methods emitted so far asked for, and the ones those ask for.
    pub fn drain_pending(&mut self) {
        while let Some(p) = self.pending.pop() {
            self.lifted_body = Some(self.lifted_span(&p));
            self.emit_lifted(p);
            self.lifted_body = None;
        }
    }

    /// The end and the start of a lifted body's text, by which the super calls of lifted bodies
    /// are ordered; the last place for one without text of its own.
    fn lifted_span(&self, p: &Pending) -> (u32, u32) {
        let prog = self.cx.input.prog;
        let body = match &p.body {
            PendingBody::Lambda(_, body) | PendingBody::LazyInit(_, body) => Some(*body),
            PendingBody::Fun(f) => prog.funs[f.idx()].body,
            PendingBody::Default(f, i) => prog.funs[f.idx()].defaults[*i],
            PendingBody::LazyAccessor(_) | PendingBody::JarGetter(_) => None,
        };
        body.and_then(|e| prog.span_of(e)).map_or((u32::MAX, 0), |(_, span)| (span.end, span.start))
    }

    fn emit_lifted(&mut self, p: Pending) {
        let cx = self.cx;
        let l = p.lifted.clone();
        if let PendingBody::JarGetter(g) = &p.body {
            self.jar_getter_thunk(&l, g);
            return;
        }
        // A local lazy val's accessor or initialiser named as another's: never one dropped in silence.
        if matches!(p.body, PendingBody::LazyAccessor(_) | PendingBody::LazyInit(..)) && self.plan.has_method(&l.name, &l.desc) {
            self.unsupported(&format!("internal error: a second local lazy val's method {}{} in the class", l.name, l.desc));
            return;
        }
        let mut params: Vec<(Option<SymId>, ErasedType)> = Vec::new();
        if l.this {
            // A copy of code declared in another class's code takes that class's instance.
            let this = match l.home.or(self.this_class) {
                Some(c) => self.class_type(c),
                None => ErasedType::L(self.this_name.clone()),
            };
            params.push((None, this));
        }
        let key_slot = params.len() as u16;
        if l.key {
            params.push((None, ErasedType::object()));
        }
        // The keys that travel with lazy vals' holders, by their slots.
        let mut keys: Vec<(SymId, u16)> = Vec::new();
        for c in &l.captures {
            if c.key {
                keys.push((c.sym, params.iter().map(|(_, t)| if t.wide() { 2 } else { 1 }).sum::<u16>()));
                params.push((None, c.ty.clone()));
                continue;
            }
            params.push((Some(c.sym), if c.cell { ErasedType::L(Rc::from(OBJECT_REF)) } else { c.ty.clone() }));
        }
        let first_param = params.len();
        let (param_syms, body, roots): (Vec<SymId>, Option<TExprId>, Vec<TExprId>) = match &p.body {
            PendingBody::Lambda(syms, body) => (syms.clone(), Some(*body), vec![*body]),
            PendingBody::Fun(f) => {
                let fun = &cx.input.prog.funs[f.idx()];
                (fun.params.clone(), fun.body, fun.body.into_iter().collect())
            }
            PendingBody::Default(f, i) => {
                let fun = &cx.input.prog.funs[f.idx()];
                let before = self.clause_start(fun.sym, *i);
                (fun.params[..before].to_vec(), fun.defaults[*i], fun.defaults[*i].into_iter().collect())
            }
            PendingBody::LazyInit(_, init) => (Vec::new(), Some(*init), vec![*init]),
            PendingBody::LazyAccessor(_) => (Vec::new(), None, Vec::new()),
            PendingBody::JarGetter(_) => unreachable!(),
        };
        let is_lambda = matches!(p.body, PendingBody::Lambda(..));
        for (i, t) in l.params.iter().enumerate() {
            // The parameters of a lambda arrive boxed and get locals of their own below.
            let sym = if is_lambda { None } else { param_syms.get(i).copied() };
            params.push((sym, t.clone()));
        }
        self.begin_method(MethodFlags { is_static: true, synthetic: true, ..MethodFlags::PUBLIC }, &l.name, &l.desc, false, &params, l.ret.clone(), &roots);
        self.m.lambda = is_lambda;
        self.m.closure = is_lambda || matches!(p.body, PendingBody::LazyInit(..));
        if l.key {
            self.m.return_key = Some(key_slot);
            // A lazy val's accessor and initialiser take the key of its holder.
            if let PendingBody::LazyAccessor(s) | PendingBody::LazyInit(s, _) = p.body {
                self.m.lazy_keys.insert(s, key_slot);
            }
        }
        if l.this {
            self.m.this_slot = Some(0);
            self.m.this_home = l.home;
        }
        for (s, slot) in keys {
            self.m.lazy_keys.insert(s, slot);
        }
        // A class's copy of a local def of an object reached statically reads its `MODULE$` for `this`,
        // and so does what such code lifts.
        if let PendingBody::Fun(f) | PendingBody::Default(f, _) = p.body {
            self.m.this_module = self.copy_home_module(cx.input.prog.funs[f.idx()].sym);
        }
        if l.module.is_some() {
            self.m.this_module = l.module.clone();
        }
        for c in l.captures.iter().filter(|c| !c.key) {
            if let Some(local) = self.m.locals.get_mut(&c.sym) {
                local.ty = c.ty.clone();
                local.cell = c.cell;
            }
        }
        if is_lambda {
            let object = ErasedType::object();
            let mut slot = params[..first_param].iter().map(|(_, t)| if t.wide() { 2 } else { 1 }).sum::<u16>();
            for &s in &param_syms {
                let declared = self.sym_type(s);
                if declared.is_object() && !self.is_vc_object(&declared) {
                    self.m.locals.insert(s, Local { slot, ty: declared, cell: false });
                } else {
                    self.outside(Unlowered::LiftedParameterPrologue);
                    let declared = self.write_slot_to_sym(slot, &object, s);
                    let cell = self.m.cells.contains(&s);
                    self.declare_local(s, declared, cell);
                }
                slot += 1;
            }
        }
        match (&p.body, body) {
            // A local lazy val's accessor and initialiser take its holder last (`LazyVals.transformLocalDef`).
            (PendingBody::LazyAccessor(sym), _) => {
                let slot = params[..params.len() - 1].iter().map(|(_, t)| if t.wide() { 2 } else { 1 }).sum::<u16>();
                self.lazy_accessor_body(*sym, slot);
            }
            (PendingBody::LazyInit(sym, init), _) => {
                let slot = params[..params.len() - 1].iter().map(|(_, t)| if t.wide() { 2 } else { 1 }).sum::<u16>();
                self.lazy_init_body(*sym, *init, slot);
            }
            (PendingBody::Fun(f), Some(body)) => {
                let fun = &cx.input.prog.funs[f.idx()];
                self.method_body(fun.sym, &fun.params, body);
            }
            // A by-name parameter's getter returns the value (`Desugar.addDefaultGetters`).
            (PendingBody::Default(f, i), Some(body)) if self.local_param_by_name(cx.input.prog.funs[f.idx()].sym, *i) => {
                let ret = l.ret.clone();
                self.body(|g| {
                    let value = g.lower_default_value(body, true, &ret);
                    g.lir.ret(value)
                });
            }
            (_, Some(body)) => self.tail(body),
            (_, None) => {
                self.outside(Unlowered::LocalDefWithoutBody);
                self.unsupported("a local def without a body");
                let ret = l.ret.clone();
                self.adapt(&ErasedType::V, &ret);
                self.return_value(&ret);
            }
        }
        self.end_method();
    }

    fn jar_getter_thunk(&mut self, l: &Lifted, g: &JarGetter) {
        let (captured, _) = parse_method_desc(&l.desc);
        let params: Vec<(Option<SymId>, ErasedType)> = captured.iter().map(|t| (None, t.clone())).collect();
        self.begin_method(MethodFlags { is_static: true, synthetic: true, ..MethodFlags::PUBLIC }, &l.name, &l.desc, false, &params, l.ret.clone(), &[]);
        self.outside(Unlowered::JarGetterThunk);
        let mut slot = 0;
        for t in &captured {
            self.load(slot, t);
            slot += if t.wide() { 2 } else { 1 };
        }
        let n_params = captured.len() - (g.kind != Invoke::Static) as usize;
        self.invoke_desc(g.kind, &g.owner, g.owner_is_interface, &g.name, &g.desc, n_params, &g.ret);
        self.adapt(&g.ret, &l.ret);
        self.return_value(&l.ret);
        self.end_method();
    }

    /// The body in return position, as a loop when the def calls itself in tail position and
    /// nothing can override it.
    pub fn method_body(&mut self, sym: SymId, params: &[SymId], body: TExprId) {
        let cx = self.cx;
        if self.an.returns_through_closure(cx, body) {
            return self.keyed_body(sym, params, body);
        }
        let (calls, _) = cx.input.prog.tail_self_calls(sym, params.len(), body);
        let jumps = cx.input.prog.tail_jumps(cx.input.syms, sym, params.len(), body).calls;
        if (calls > 0 || !jumps.is_empty()) && cx.input.syms.is_effectively_final(sym) && params.iter().all(|p| self.m.locals.get(p).map_or(false, |l| !l.cell)) {
            let head = self.code.new_label();
            self.code.bind(head);
            self.m.tail = Some(TailLoop { sym, params: params.to_vec(), head, jumps });
        }
        self.tail(body);
        self.m.tail = None;
    }

    /// A def's body that a closure it makes returns from, as dotty's
    /// `NonLocalReturns.nonLocalReturnTry` writes it: a fresh key of each run, which the closures
    /// capture and their `return`s throw in a `NonLocalReturnControl`, and around the body the
    /// handler that returns the value of one carrying this run's key and throws any other on.
    fn keyed_body(&mut self, sym: SymId, params: &[SymId], body: TExprId) {
        self.outside(Unlowered::NonLocalReturnKey);
        let object = ErasedType::object();
        let control = ErasedType::L(Rc::from(NON_LOCAL_RETURN));
        self.new_object(OBJECT);
        self.invoke(Invoke::Special, OBJECT, false, "<init>", &[], &ErasedType::V);
        let key = self.store_new(&object);
        let outer_key = self.m.return_key.replace(key);
        let (start, end, handler) = (self.code.new_label(), self.code.new_label(), self.code.new_label());
        // A handler's frame holds the locals of the whole protected range.
        let protected = self.code.locals_mark();
        self.code.bind(start);
        let from = self.code.pc();
        let cx = self.cx;
        let (calls, _) = cx.input.prog.tail_self_calls(sym, params.len(), body);
        let jumps = cx.input.prog.tail_jumps(cx.input.syms, sym, params.len(), body).calls;
        let tail = (calls > 0 || !jumps.is_empty()) && cx.input.syms.is_effectively_final(sym) && params.iter().all(|p| self.m.locals.get(p).map_or(false, |l| !l.cell));
        if tail {
            let head = self.code.new_label();
            self.code.bind(head);
            self.m.tail = Some(TailLoop { sym, params: params.to_vec(), head, jumps });
        }
        self.tail(body);
        self.m.tail = None;
        if self.code.pc() == from {
            self.code.op(op::NOP);
        }
        self.code.bind(end);
        let index = self.cw.cp.class(NON_LOCAL_RETURN);
        self.code.add_handler(Handler { start, end, handler, catch_type: index });
        self.code.locals_release(protected);
        let vt = self.vt(&control);
        self.code.bind_handler(handler, vt);
        let caught = self.store_new(&control);
        let rethrow = self.code.new_label();
        self.load(caught, &control);
        self.invoke(Invoke::Virtual, NON_LOCAL_RETURN, false, "key", &[], &object);
        self.load(key, &object);
        self.jump_if(op::IF_ACMPNE, 2, rethrow);
        self.load(caught, &control);
        self.invoke(Invoke::Virtual, NON_LOCAL_RETURN, false, "value", &[], &object);
        let ret = self.m.ret.clone();
        self.adapt(&object, &ret);
        self.return_value(&ret);
        self.code.bind(rethrow);
        self.load(caught, &control);
        self.code.op(op::ATHROW);
        self.code.pop();
        self.code.end_path();
        self.m.return_key = outer_key;
    }

    fn emit_fun(&mut self, f: FunId) {
        let cx = self.cx;
        let fun = &cx.input.prog.funs[f.idx()];
        if fun.captures > 0 {
            return self.emit_proxied_fun(f);
        }
        let info = cx.input.syms.sym(fun.sym);
        let m = self.mref(fun.sym);
        let name = cx.input.interner.get(info.name);
        self.context = format!("{}.{}", self.this_name, name);
        let ordered: Vec<SymId> = match self.jvm_param_order(fun.sym) {
            Some(order) if order.len() == fun.params.len() => order.iter().map(|&k| fun.params[k]).collect(),
            _ => fun.params.clone(),
        };
        let params: Vec<(Option<SymId>, ErasedType)> = ordered.iter().zip(&m.params).map(|(&s, t)| (Some(s), t.clone())).collect();
        if params.len() != m.params.len() {
            self.unsupported("a def whose parameters differ from its signature");
            return;
        }
        match fun.body {
            Some(body) if self.is_value_class_method() => {
                self.value_class_method(f, &m, &params, body);
                self.emit_bridges(fun.sym, &m);
            }
            Some(body) => {
                // An outer accessor is scalac's, final and synthetic (`Worker::outer_accessor`).
                let outer_accessor = info.def.is_none() && info.mods & mods::FINAL != 0 && name.ends_with("$$$outer");
                // A final method outside a trait is final in its class file (dotty's `javaFlags`).
                let fin = info.mods & mods::FINAL != 0 && !self.is_interface;
                let flags = MethodFlags { is_final: outer_accessor || fin, synthetic: outer_accessor, ..MethodFlags::PUBLIC };
                self.begin_method(flags, &m.name, &m.desc, false, &params, m.ret.clone(), &[body]);
                self.code.line(cx.line_of(info.file, info.span.start));
                self.method_body(fun.sym, &fun.params, body);
                self.end_method();
                self.emit_bridges(fun.sym, &m);
                if self.is_interface && (info.mods & mods::PRIVATE == 0 || info.scoped_private) {
                    self.trait_static_forwarder(&m.name, &m.params, &m.ret);
                }
            }
            // An inline method is not output; its ordinary getters below are.
            None if info.mods & mods::INLINE != 0 => {}
            None => {
                if self.is_interface || self.abstract_class {
                    if !self.plan.has_method(&m.name, &m.desc) {
                        self.abstract_method(MethodFlags::ABSTRACT, &m.name, &m.desc);
                    }
                }
            }
        }
        // scalac's getters: each takes the parameters of the clauses before its own, and a
        // by-name parameter's returns the value, which the caller wraps.
        for (i, d) in fun.defaults.iter().enumerate() {
            let Some(d) = *d else { continue };
            let shape = self.default_getter_shape(fun.sym, i);
            let (before, ret) = (shape.before, shape.ret.clone());
            let getter = format!("{}$default${}", self.source_method_name(fun.sym), shape.index);
            if let Some(c) = self.this_class.filter(|&c| cx.vc_on_companion(c)) {
                self.box_forwarder(c, &getter, &format!("{}$extension", getter), &params[..before], &ret);
                continue;
            }
            let desc = method_desc(&m.params[..before], &ret);
            // A forwarder's getters are forwarders of the member's, final as it is
            // (`Namer.foreachDefaultGetterOf`); a method's own take its access alone
            // (`Desugar.defaultGetters`).
            let fin = !self.is_interface && cx.input.syms.is_forwarder(fun.sym);
            self.begin_method(MethodFlags { is_final: fin, ..MethodFlags::PUBLIC }, &getter, &desc, false, &params[..before], ret.clone(), &[d]);
            self.body(|g| {
                let value = g.lower_default_value(d, shape.by_name, &ret);
                g.lir.ret(value)
            });
            self.end_method();
            if self.is_interface {
                self.trait_static_forwarder(&getter, &m.params[..before], &ret);
            }
        }
    }

    /// A member of a local trait that takes the values of its block first (`TFun::captures`), as
    /// dotty's `LambdaLift` leaves it: the default method over the proxies and the parameters,
    /// with its static `m$`, which the forwarders of the classes mixing the trait in call; the
    /// member as declared stays abstract, the slot every other call goes through.
    fn emit_proxied_fun(&mut self, f: FunId) {
        let cx = self.cx;
        let fun = &cx.input.prog.funs[f.idx()];
        let info = cx.input.syms.sym(fun.sym);
        let m = self.mref(fun.sym);
        let name = cx.input.interner.get(info.name);
        self.context = format!("{}.{}", self.this_name, name);
        let k = fun.captures as usize;
        let declared = &fun.params[k..];
        if declared.len() != m.params.len() {
            self.unsupported("a def whose parameters differ from its signature");
            return;
        }
        // A local trait's lifted member has its lifted method alone, as dotty's trait has it.
        if !fun.lifted && !self.plan.has_method(&m.name, &m.desc) {
            self.abstract_method(MethodFlags::ABSTRACT, &m.name, &m.desc);
        }
        // A default's getter reads the block's values too: the forwarders' implement it.
        for (i, d) in fun.defaults[k..].iter().enumerate() {
            if d.is_none() {
                continue;
            }
            let shape = self.default_getter_shape(fun.sym, i);
            let getter = format!("{}$default${}", self.source_method_name(fun.sym), shape.index);
            let desc = method_desc(&m.params[..shape.before], &shape.ret);
            if !self.plan.has_method(&getter, &desc) {
                self.abstract_method(MethodFlags::ABSTRACT, &getter, &desc);
            }
        }
        let Some(body) = fun.body else { return };
        let proxies = self.proxy_syms(&fun.params[..k]);
        let extras = self.extras_of(&fun.params[..k], None);
        let pm = self.proxied_mref(&m, &proxies, &extras);
        let mut params: Vec<(Option<SymId>, ErasedType)> = proxies.iter().zip(&pm.params).map(|(&s, t)| (Some(s), t.clone())).collect();
        let extras_slot = 1 + params.iter().map(|(_, t)| if t.wide() { 2 } else { 1 }).sum::<u16>();
        params.extend(pm.params[proxies.len()..pm.params.len() - m.params.len()].iter().map(|t| (None, t.clone())));
        params.extend(declared.iter().zip(&m.params).map(|(&s, t)| (Some(s), t.clone())));
        self.begin_method(MethodFlags::PUBLIC, &pm.name, &pm.desc, false, &params, pm.ret.clone(), &[body]);
        self.bind_proxies(&proxies);
        self.bind_extras(&extras, extras_slot);
        self.code.line(cx.line_of(info.file, info.span.start));
        self.method_body(fun.sym, &fun.params, body);
        self.end_method();
        self.trait_static_forwarder(&pm.name, &pm.params, &pm.ret);
    }

    /// scalac's static forwarder `m$(T, args)` of a trait's concrete method `m`, which calls the
    /// default method on its first argument: a class scalac compiles mixing the trait in
    /// implements `m` by calling it. Every concrete method of the trait has one, a protected
    /// or final one, an extension, a default getter, a lazy or constant val's getter; a private
    /// one has none.
    pub fn trait_static_forwarder(&mut self, name: &str, params: &[ErasedType], ret: &ErasedType) {
        let owner = self.this_name.clone();
        let this = ErasedType::L(owner.clone());
        let mut types = vec![this];
        types.extend(params.iter().cloned());
        let typed: Vec<(Option<SymId>, ErasedType)> = types.iter().map(|t| (None, t.clone())).collect();
        self.begin_method(MethodFlags { is_static: true, ..MethodFlags::PUBLIC }, &format!("{}$", name), &method_desc(&types, ret), false, &typed, ret.clone(), &[]);
        let default = method_ref(Invoke::Special, &owner, true, name, params, ret);
        self.body(|g| {
            let receiver = g.param(0);
            let args: Vec<LirId> = (1..types.len()).map(|i| g.param(i)).collect();
            let result = g.lir.call(&default, receiver, &args);
            g.lir.ret(result)
        });
        self.end_method();
    }

    /// dotty's `isConstExprFinalVal` of the val `s` initialised with `init` (`tir::constant_val`):
    /// no field holds it, in its class or a class that mixes its trait in. A top-level val's owner is
    /// its file's module class, a Scala object (`Desugar.packageDef`), where dotty asks the same
    /// (`file_constant`).
    pub(super) fn constant(&self, s: SymId, init: TExprId) -> Option<ConstVal> {
        let input = &self.cx.input;
        if matches!(input.syms.sym(s).owner, Owner::Package(_)) {
            return self.file_constant(s, init);
        }
        constant_val(input.prog, input.syms, input.types, input.asts, s, init)
    }

    /// dotty's `isConstExprFinalVal` (SymUtils.scala 303-306) of the top-level val `s`: final and
    /// of a `ConstantType` as its `info.resultType` is, the type the typer recorded for it
    /// (`MethodSig::ret`) where none is written. Its initialiser, where it is no literal, runs in
    /// `<clinit>` for its effects, the getter answering the constant. A type written that is no
    /// literal type makes none: teq's types drop the reference to an alias, which dotty's
    /// `info.resultType` keeps (`type A = 42; final val x: A` keeps its field), until they keep it
    /// (queued with `tir::constant_val`, which this goes for then).
    fn file_constant(&self, s: SymId, init: TExprId) -> Option<ConstVal> {
        let input = &self.cx.input;
        let info = input.syms.sym(s);
        if info.kind != SymKind::Val || info.mods & mods::FINAL == 0 || info.mods & (mods::LAZY | mods::GIVEN | mods::INLINE) != 0 {
            return None;
        }
        let file = info.file.0 as usize;
        if file < input.asts.len() {
            let ast = &input.asts[file];
            if let Some(crate::ast::DefKind::Val { ty: Some(t), .. }) = info.def.and_then(|d| ast.defs.get(d.idx())).map(|def| &def.kind) {
                if !matches!(ast.ty(*t), crate::ast::TyExpr::Lit(_)) {
                    return None;
                }
            }
        }
        let Some(Type::Lit(l)) = info.sig.as_ref().map(|sig| input.types.get(sig.ret)) else { return None };
        let literal = matches!(input.prog.expr(init), TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_));
        Some(if literal { ConstVal::Literal } else { ConstVal::Declared(l) })
    }

    /// The constant a constant val's getter answers: its literal initialiser, or the constant of
    /// the type it has (`Memoize` leaves the getter, the backend writes its `ConstantType`).
    fn lower_constant_value(&mut self, constant: Option<ConstVal>, init: TExprId, ty: &ErasedType) -> LirId {
        let Some(ConstVal::Declared(l)) = constant else { return self.lower_expr(init, ty) };
        let value = self.lower_literal(l);
        let pushed = self.lir.ty(value).clone();
        self.lower_adapt(value, &pushed, ty)
    }

    /// The value of the constant type `l` written (`lower_literal`), and its type.
    pub(super) fn literal_value(&mut self, l: LitId) -> ErasedType {
        let value = self.lowering(|g| g.lower_literal(l));
        self.write(value);
        self.lir.ty(value).clone()
    }

    /// `BCodeBodyBuilder.genConstant` (570-617) of the constant type `l`: its value, of its JVM
    /// type, every tag the types hold (a `null` and `()` are no constant types here).
    pub(super) fn lower_literal(&mut self, l: LitId) -> LirId {
        let (kind, ty) = match self.cx.input.types.lit_val(l) {
            LitVal::Class(t) => (Kind::ClassOf(self.class_constant(t)), ErasedType::L(Rc::from("java/lang/Class"))),
            LitVal::Int(v) => (Kind::Int(v), ErasedType::I),
            LitVal::Byte(v) => (Kind::Int(v as i32), ErasedType::B),
            LitVal::Short(v) => (Kind::Int(v as i32), ErasedType::S),
            LitVal::Long(v) => (Kind::Long(v), ErasedType::J),
            LitVal::Double(bits) => (Kind::Double(f64::from_bits(bits)), ErasedType::D),
            LitVal::Float(bits) => (Kind::Float(f32::from_bits(bits)), ErasedType::F),
            LitVal::Char(c) => (Kind::Char(c), ErasedType::C),
            LitVal::Bool(b) => (Kind::Bool(b), ErasedType::Z),
            LitVal::Str(n) => (Kind::Text(Rc::from(self.cx.input.interner.get(n))), ErasedType::L(Rc::from(STRING))),
        };
        self.lir.push(kind, ty)
    }

    /// The value of default `d` as its getter returns it: a by-name parameter's default is typed
    /// as the argument passed for it, a lambda without parameters, whose body is the value.
    pub(super) fn lower_default_value(&mut self, d: TExprId, by_name: bool, ret: &ErasedType) -> LirId {
        let prog = self.cx.input.prog;
        match prog.expr(d) {
            TExpr::Lambda(ps, body) if by_name && ps.len == 0 => self.lower_expr(body, ret),
            _ if by_name => {
                let thunk = self.function_type(0);
                let function = self.lower_expr(d, &thunk);
                let ErasedType::L(owner) = &thunk else { unreachable!() };
                let object = ErasedType::object();
                let apply = method_ref(Invoke::Interface, owner, true, "apply", &[], &object);
                let value = self.lir.call(&apply, function, &[]);
                self.lower_adapt(value, &object, ret)
            }
            _ => self.lower_expr(d, ret),
        }
    }

    fn is_value_class_method(&self) -> bool {
        let cx = self.cx;
        self.this_class.map_or(false, |c| cx.input.syms.class(c).value_class)
    }

    /// In link mode, a method of a value class is scalac's pair: the static `m$extension`,
    /// which takes the underlying value first and holds the body, and the method of the box,
    /// which forwards to it.
    fn value_class_method(&mut self, f: FunId, m: &MethodRef, params: &[(Option<SymId>, ErasedType)], body: TExprId) {
        let cx = self.cx;
        let Some(c) = self.this_class else { return };
        let fun = &cx.input.prog.funs[f.idx()];
        let info = cx.input.syms.sym(fun.sym);
        let u = self.vc_underlying(c);
        // The body is the companion's `m$extension` (`extension_bodies`): the box's method
        // calls it on the underlying value.
        if cx.vc_on_companion(c) {
            self.box_forwarder(c, &m.name, &format!("{}$extension", m.name), params, &m.ret);
            return;
        }
        let mut ext_params = vec![(None, u.clone())];
        ext_params.extend(params.iter().cloned());
        let ext_types: Vec<ErasedType> = ext_params.iter().map(|p| p.1.clone()).collect();
        let ext_desc = method_desc(&ext_types, &m.ret);
        let ext_name = format!("{}$extension", m.name);
        self.begin_method(MethodFlags { is_static: true, ..MethodFlags::PUBLIC }, &ext_name, &ext_desc, false, &ext_params, m.ret.clone(), &[body]);
        self.m.this_slot = Some(0);
        self.m.this_underlying = Some(u.clone());
        self.code.line(cx.line_of(info.file, info.span.start));
        self.method_body(fun.sym, &fun.params, body);
        self.end_method();
        self.begin_method(MethodFlags::PUBLIC, &m.name, &m.desc, false, params, m.ret.clone(), &[]);
        self.outside(Unlowered::ValueClassForwarder);
        let this = self.class_type(c);
        self.load(0, &this);
        self.vc_unbox(c, &u);
        let mut slot = 1;
        for (_, t) in params {
            self.load(slot, t);
            slot += if t.wide() { 2 } else { 1 };
        }
        let owner = self.this_name.clone();
        self.invoke_desc(Invoke::Static, &owner, false, &ext_name, &ext_desc, ext_types.len(), &m.ret);
        self.return_value(&m.ret);
        self.end_method();
    }

    /// A method of the box of value class `c`, `name(params)`, calling the companion's
    /// `target(u, params)` on the underlying value.
    fn box_forwarder(&mut self, c: ClassId, name: &str, target: &str, params: &[(Option<SymId>, ErasedType)], ret: &ErasedType) {
        let u = self.vc_underlying(c);
        let module: Rc<str> = Rc::from(self.cx.companion_name(c));
        let typed: Vec<(Option<SymId>, ErasedType)> = params.iter().map(|(_, t)| (None, t.clone())).collect();
        let own: Vec<ErasedType> = params.iter().map(|(_, t)| t.clone()).collect();
        self.begin_method(MethodFlags::PUBLIC, name, &method_desc(&own, ret), false, &typed, ret.clone(), &[]);
        let mut types = vec![u.clone()];
        types.extend(own.iter().cloned());
        let extension = method_ref(Invoke::Virtual, &module, false, target, &types, ret);
        let this = self.class_type(c);
        self.body(|g| {
            let receiver = g.lir.module(&module);
            let instance = g.lir.this(this.clone());
            let mut args = vec![g.lower_value_class(instance, &this, c, EMPTY_LIST, false, &u, None)];
            for i in 0..own.len() {
                args.push(g.param(i));
            }
            let result = g.lir.call(&extension, receiver, &args);
            g.lir.ret(result)
        });
        self.end_method();
    }

    /// A member overrides the members of the ancestors that share its name; where the erased
    /// signature differs (`compare(Int, Int)` for `compare(T, T)`) a bridge forwards to it.
    fn emit_bridges(&mut self, sym: SymId, m: &MethodRef) {
        self.emit_bridges_as(sym, self.cx.input.syms.sym(sym).name, m);
    }

    /// The bridges of `m`, which `sym` defines under the member name `name`: a var's setter is
    /// the member `x_=`.
    fn emit_bridges_as(&mut self, sym: SymId, name: Name, m: &MethodRef) {
        let cx = self.cx;
        let Some(c) = self.this_class else { return };
        let syms = cx.input.syms;
        let info = syms.sym(sym);
        let bases: Vec<ClassId> = syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        for b in bases {
            let base = syms.class(b);
            let inherited: Vec<SymId> = if info.is_extension {
                base.extensions.iter().copied().filter(|&e| syms.sym(e).name == name).collect()
            } else {
                match base.members.get(&name) {
                    Some(&e) => match syms.alternatives(e) {
                        Some(alts) => alts.iter().copied().filter(|&a| syms.sym(a).owner == Owner::Class(b)).collect(),
                        None => vec![e],
                    },
                    None => Vec::new(),
                }
            };
            for inherited in inherited {
            if matches!(syms.sym(inherited).kind, SymKind::Object(_)) {
                continue;
            }
            // Under the pairs' dump each decision is written (tests/pairs-twin.sh holds them to the record).
            // A var's setter is walked by the var under the setter's name: the dump names the setter, the member the
            // record pairs.
            let walked = if name == info.name { sym } else { syms.class(c).members.get(&name).copied().filter(|&x| syms.sym(x).owner == Owner::Class(c)).unwrap_or(sym) };
            let dump = |this: &mut Self, decision: &str| {
                if crate::tir::pairs_dump().is_some() {
                    let bm = this.mref(inherited);
                    let differs = if bm.desc == m.desc { "same-descriptor" } else { "descriptors-differ" };
                    // whether the class's table holds the member (a member the back end makes by rule, a case
                    // companion's `apply`, is in none: the record, which reads the table, has no pair of it)
                    let held = if syms.class(c).member_order.contains(&walked) { "table" } else { "rule" };
                    crate::tir::pairs_dump_write(&format!("@{}\tjvm-walk\t{}\t{}\t{decision}\t{differs}\t{held}\n", c.0, walked.0, inherited.0));
                }
            };
            // An alternative of an overloaded name is bridged when it takes the parameters
            // of the inherited one as the class sees them: the types agree after erasure of
            // the class's type arguments, which the bridge's own parameters show.
            if syms.sym(inherited).alternative || syms.sym(sym).alternative {
                let overrides = cx.input.prog.overrides.get(&sym).map_or(false, |ps| ps.contains(&inherited));
                if !overrides {
                    dump(self, "not-overriding");
                    continue;
                }
            }
            let bm = self.mref(inherited);
            if bm.name != m.name || bm.desc == m.desc || bm.params.len() != m.params.len() || self.plan.has_method(&bm.name, &bm.desc) {
                let decision = if bm.name != m.name { "other-name" } else if bm.desc == m.desc { "same" } else if bm.params.len() != m.params.len() { "arity" } else { "declared" };
                dump(self, decision);
                continue;
            }
            if !self.params_may_override(inherited, &bm.params, &m.params) {
                dump(self, "declined");
                continue;
            }
            dump(self, "bridge");
            self.bridge(sym, &bm, m);
            }
        }
    }

    /// The bridge `bm` forwarding to `m`, the member `sym`'s method: each argument and the
    /// result adapted between the two erasures, through the value class where `sym` declares
    /// one (`A <: V`), which the bridge's reference holds as its box.
    fn bridge(&mut self, sym: SymId, bm: &MethodRef, m: &MethodRef) {
        let (declared, declared_ret): (Vec<TypeId>, TypeId) = match self.cx.input.syms.sym(sym).sig.as_ref() {
            Some(sig) => (sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).collect(), sig.ret),
            None => (Vec::new(), NO_TYPE),
        };
        let params: Vec<(Option<SymId>, ErasedType)> = bm.params.iter().map(|t| (None, t.clone())).collect();
        self.begin_method(MethodFlags { synthetic: true, bridge: true, ..MethodFlags::PUBLIC }, &bm.name, &bm.desc, false, &params, bm.ret.clone(), &[]);
        let kind = if self.is_interface { Invoke::Interface } else { Invoke::Virtual };
        let target = Rc::new(MethodRef { owner: self.this_name.clone(), owner_is_interface: self.is_interface, kind, ..m.clone() });
        self.body(|g| {
            let Some(ThisValue::Slot(this_type)) = g.this_value() else { unreachable!("a bridge runs on an instance") };
            let receiver = g.lir.this(this_type);
            let mut args = Vec::with_capacity(m.params.len());
            for (i, (from, to)) in bm.params.iter().zip(&m.params).enumerate() {
                let given = g.param(i);
                args.push(match declared.get(i).and_then(|&t| g.value_class_type(t)) {
                    Some((c, vargs)) => g.lower_value_class(given, from, c, vargs, true, to, None),
                    None => g.lower_adapt(given, from, to),
                });
            }
            let result = g.lir.call(&target, receiver, &args);
            let result = match g.value_class_type(declared_ret) {
                Some((c, vargs)) if m.ret != ErasedType::V => g.lower_value_class(result, &m.ret, c, vargs, true, &bm.ret, None),
                _ => g.lower_adapt(result, &m.ret, &bm.ret),
            };
            g.lir.ret(result)
        });
        self.end_method();
    }

    /// Whether a member with the parameters `own` can override `inherited`, whose erased
    /// parameters are `erased`: a parameter may erase differently only where the inherited one
    /// is abstract (a type parameter, an abstract type member), which the override fixes; a
    /// class type there makes the member an overload (`max(FiniteDuration)` beside
    /// `max(Duration)`), which scalac does not bridge.
    fn params_may_override(&self, inherited: SymId, erased: &[ErasedType], own: &[ErasedType]) -> bool {
        let cx = self.cx;
        let Some(sig) = cx.input.syms.sym(inherited).info.sig.as_ref() else { return true };
        let declared: Vec<TypeId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).collect();
        if declared.len() != erased.len() {
            return true;
        }
        erased.iter().zip(own).zip(&declared).all(|((e, o), &t)| {
            e == o || !matches!(cx.input.types.get(t), Type::Class(..) | Type::Union(..) | Type::Inter(..) | Type::Lit(_) | Type::Refined(..) | Type::Rec(_))
        })
    }

    // ---- classes ----

    /// What kind of class a class is, as dotty's `BCodeUtils.javaFlags` of its symbol has it: a
    /// trait an abstract interface; abstract where declared so (an enum is);
    /// final where the symbol is `Final` and not abstract: written so, a module class
    /// (`ModuleClassCreationFlags`), a given object's among them, an anonymous class
    /// (`Typer.typedNew`), an enum's case (`DesugarEnums`, a value's class being anonymous) or a
    /// value class (`Namer`); and an enum's class and its class cases are enums (`Enum`).
    fn class_flags(&self, c: ClassId) -> ClassFlags {
        let cx = self.cx;
        let info = cx.input.syms.class(c);
        if info.kind == ClassKind::Trait {
            return ClassFlags { interface: true, is_abstract: true, ..ClassFlags::default() };
        }
        let class_case = info.kind == ClassKind::EnumCase && info.singleton.is_none();
        if info.kind == ClassKind::Enum || info.kind == ClassKind::Class && info.mods & mods::ABSTRACT != 0 {
            return ClassFlags { is_abstract: true, is_enum: info.kind == ClassKind::Enum, ..ClassFlags::default() };
        }
        let module = matches!(info.kind, ClassKind::Object) || info.inner_object.is_some() || info.kind == ClassKind::GivenImpl && !cx.given_class_classes.contains_key(&c);
        // A given class is not (`Desugar.classDef`: the def answering it is).
        let given_class = cx.given_class_classes.contains_key(&c);
        let fin = !given_class && (info.mods & mods::FINAL != 0 || module || matches!(info.kind, ClassKind::Anon | ClassKind::EnumCase) || info.value_class);
        ClassFlags { is_final: fin, is_enum: class_case, ..ClassFlags::default() }
    }

    pub fn emit_class(&mut self, idx: usize) -> (String, Vec<u8>) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let syms = cx.input.syms;
        let tc = &prog.classes[idx];
        let c = tc.id;
        let info = syms.class(c);
        let name = self.class_name(c);
        self.context = name.to_string();
        let is_trait = info.kind == ClassKind::Trait;
        let is_object = info.kind == ClassKind::Object;
        let given_module = cx.given_object_classes.contains_key(&c);
        let is_enum = info.kind == ClassKind::Enum;
        let is_anon = info.kind == ClassKind::Anon;
        let enum_class = cx.enum_of_case(c);
        let is_value_case = info.kind == ClassKind::EnumCase && info.singleton.is_some();
        let is_case = cx.is_case_class(c);
        let is_case_object = cx.is_case_object(c);
        let superclass = info.superclass.filter(|_| !is_trait);
        let is_abstract = info.kind == ClassKind::Class && info.mods & mods::ABSTRACT != 0;
        let super_name: Rc<str> = match enum_class.or(superclass) {
            Some(e) => self.class_name(e),
            None => Rc::from(OBJECT),
        };
        let mut interfaces: Vec<String> = Vec::new();
        let mut listed: Vec<ClassId> = enum_class.or(superclass).into_iter().collect();
        let mut desugared: Vec<ClassId> = Vec::new();
        // In the order of the parents clause, the reverse of the linearisation's (dotty's
        // `directlyInheritedTraits`, the traits before the superclass), those no other of them
        // brings, as `BCodeSkelBuilder.initJClass` lists them. The ones it leaves out, the traits
        // another brings or the superclass does, are no interfaces of the class unless one of its
        // super calls names them: those join the list at the class's end (`super_targets`). A
        // library class the std's stands for (`link_library_class`) goes by the same name.
        let before_super = info.base_types.iter().position(|&(b, _)| Some(b) == enum_class.or(superclass)).unwrap_or(info.base_types.len());
        let mut traits: Vec<(ClassId, bool)> = Vec::new();
        for (i, &(b, _)) in info.base_types.iter().enumerate().skip(1).rev() {
            let bname = &cx.class_names[b.idx()];
            if syms.class(b).kind == ClassKind::Trait && syms.class(b).js == JsKind::Scala && **bname != *name && !traits.iter().any(|&(t, _)| cx.class_names[t.idx()] == *bname) {
                if is_desugared_parent(syms, cx.input.interner, b) {
                    desugared.push(b);
                    continue;
                }
                traits.push((b, i < before_super));
            }
        }
        let brought = |t: ClassId| traits.iter().any(|&(u, direct)| direct && u != t && syms.class(u).base_types.iter().skip(1).any(|&(x, _)| x == t));
        // The direct traits in their order, each with whether another of them brings it.
        let direct: Vec<(String, bool)> = traits.iter().filter(|&&(_, direct)| direct).map(|&(t, _)| (cx.class_names[t.idx()].clone(), brought(t))).collect();
        interfaces.extend(direct.iter().filter(|(_, brought)| !brought).map(|(n, _)| n.clone()));
        let direct_listed = interfaces.len();
        listed.extend(traits.iter().map(|&(b, _)| b));
        // The parents `Desugar.classDef` gives (`Product` and `java.io.Serializable`, an enum's
        // `Enum`) as scalac's backend lists them: after the others, less one another parent or the
        // superclass brings (`directlyInheritedTraits` less the redundant, `CoreBTypesFromSymbols`):
        // `Product` alone below `RuntimeException`, `Enum` alone for an enum.
        for (i, &d) in desugared.iter().enumerate() {
            let brought = listed.iter().chain(desugared.iter().enumerate().filter(|&(j, _)| j != i).map(|(_, k)| k)).any(|&k| syms.class(k).base_types.iter().skip(1).any(|&(x, _)| x == d));
            if !brought {
                interfaces.push(cx.class_names[d.idx()].to_string());
            }
        }
        // A case object is its own mirror, as scalac's: `Mirror.Singleton`.
        if is_case_object && !interfaces.iter().any(|i| i == super::callable::MIRROR_SINGLETON) {
            interfaces.push(super::callable::MIRROR_SINGLETON.to_string());
        }
        let case_companion = info.companion.filter(|&k| is_object && cx.has_case_companion(k));
        if case_companion.map_or(false, |k| cx.has_product_mirror(k)) && !interfaces.iter().any(|i| i == super::callable::MIRROR_PRODUCT) {
            interfaces.push(super::callable::MIRROR_PRODUCT.to_string());
        }
        // The companion of a class's enum, an object of each enclosing instance, is its mirror too.
        // The companion of a class's enum or of a local enum, an object of each enclosing instance or
        // of each run of the block: its lookups are its members (`enumLookupMethods`).
        let instance_enum_companion = (info.inner_object.is_some() || info.local_module.is_some()) && info.companion.is_some_and(|e| syms.class(e).kind == ClassKind::Enum);
        if (is_object && info.companion.map_or(false, |e| cx.has_enum_fields(e)) || instance_enum_companion) && !interfaces.iter().any(|i| i == super::callable::MIRROR_SUM) {
            interfaces.push(super::callable::MIRROR_SUM.to_string());
        }
        // What the class's parents give; a module class's `Serializable` comes after it, and after
        // the targets of its super calls (`initJClass` appends it last).
        let declared = interfaces.len();
        if is_object || info.inner_object.is_some() || cx.given_object_classes.contains_key(&c) || cx.inner_given_object_classes.contains_key(&c) {
            let listed = super::callable::module_interfaces(&interfaces, info.owner == Owner::Local);
            interfaces.extend(listed);
        }
        let flags = self.class_flags(c);
        let path = &cx.input.sources[info.file.0 as usize].path;
        self.begin_class(&name, flags, &super_name, &interfaces[..declared], &interfaces[declared..], Some(path.rsplit(std::path::is_separator).next().unwrap_or(path)));
        self.super_targets.clear();
        self.lifted_body = None;
        self.ctor_code = false;

        self.this_name = name.clone();
        self.this_class = Some(c);
        self.is_interface = is_trait;
        self.abstract_class = is_enum || is_abstract;
        self.line_file = info.file;

        let n_captures = if is_anon { tc.ctor_params.len() } else { tc.captures };
        if n_captures > 0 {
            // A given class's or object's enclosing instance is scalac's field `$outer`.
            let given_class = cx.given_class_classes.contains_key(&c) || cx.inner_given_object_classes.contains_key(&c);
            for (i, s) in self.class_captured(c).into_iter().enumerate() {
                let base = encode(cx.input.interner.get(syms.sym(s).name));
                let sinfo = syms.sym(s);
                // A local lazy val is held as its holder (`LazyVals.transformLocalDef`).
                let ty = if sinfo.by_name { self.function_type(0) } else if self.is_lazy_local(s) { self.lazy_holder_type(s) } else { self.sym_type(s) };
                let held = self.held(s);
                let field = if given_class && i == 0 { "$outer".to_string() } else { format!("{}$c{}", base, i) };
                self.class_captures.push((s, field, ty, held));
            }
            // The instances and keys its copies of local defs and lazy vals read (`Gen::class_extras`).
            let extras = self.class_extras(c);
            for (i, &h) in extras.homes.iter().enumerate() {
                let field = self.home_field(h, i);
                self.class_homes.push((h, field));
            }
            for (i, &x) in extras.keys.iter().enumerate() {
                let field = self.key_field(x, i);
                self.class_keys.push((x, field));
            }
        }

        let fields = if is_trait { Vec::new() } else { self.class_fields(tc, n_captures) };
        // The class's thread-safe lazy members, registered as they are written (`Gen::lazy_holder`),
        // their handles declared first and looked up in the `<clinit>` written last.
        self.lazy_holders.clear();
        // An object's `<clinit>` is written last, once the handles it looks up first are known; a class with
        // no lazy member (the names walk numbers none) writes it first, as it always has.
        let mut module_clinit_wanted = false;
        let clinit_first = !self.holds_lazy_members(Some(c), info.module_file());
        // The holders an early `<clinit>` looked up (those of the fields written before it).
        let mut early_lookups: Option<usize> = None;
        for f in fields.iter().filter(|f| f.constant.is_none()) {
            if f.lazy && self.is_thread_unsafe(f.sym) {
                // `@threadUnsafe`: the container of the member's type and its flag, no handle
                // (`transformMemberDefThreadUnsafe`, `Gen::thread_unsafe_getter`).
                let (container, flag) = self.thread_unsafe_names(&f.name);
                self.field(FieldFlags::PRIVATE, &container, &f.ty);
                self.field(FieldFlags::PRIVATE, &flag, &ErasedType::Z);
                continue;
            }
            if f.lazy {
                // `private @volatile var x$lzyN: AnyRef` (`transformMemberDefThreadSafeNew`).
                let h = self.lazy_holder(&f.name, &f.ty);
                self.field(FieldFlags { volatile: true, ..FieldFlags::PRIVATE }, &h.container, &ErasedType::object());
                continue;
            }
            let volatile = self.is_volatile(f.sym);
            let is_final = !volatile && cx.private_params.contains_key(&f.sym);
            self.field(FieldFlags { is_final, volatile, ..FieldFlags::PRIVATE }, &f.name, &f.ty);
        }
        for (s, field, _, _) in self.class_captures.clone() {
            let ty = self.anon_capture_type(s);
            let outer = field == "$outer";
            self.field(FieldFlags { is_final: outer, synthetic: outer, ..FieldFlags::PRIVATE }, &field, &ty);
        }
        // An instance of another class is read by the copies of the classes made here too.
        for (h, field) in self.class_homes.clone() {
            let ty = self.class_type(h);
            self.field(FieldFlags { visibility: Visibility::Package, is_final: true, synthetic: true, ..FieldFlags::PRIVATE }, &field, &ty);
        }
        for (_, field) in self.class_keys.clone() {
            self.field(FieldFlags::PRIVATE, &field, &ErasedType::object());
        }
        // Decided before an object's `<clinit>` is written, which writes its constructor's body.
        let primary = (!is_trait).then(|| self.primary_ctor(tc, &fields, &super_name, enum_class.or(superclass), superclass.is_some(), is_value_case, is_object || given_module, is_anon));
        if is_object {
            // An object nested in this one is a module val of it, a term member that is no method,
            // so a static final field of its module class (dotty's `addClassFields`), which nothing
            // assigns: the nested object is read through its own `MODULE$`.
            // Its flags are the module term's (`javaFieldFlags`): private for a private object.
            let syms = cx.input.syms;
            let mut nested: Vec<(SymId, ClassId)> = Vec::new();
            for &m in &syms.class(c).member_order {
                if let SymKind::Object(k) = syms.sym(m).kind {
                    if syms.sym(m).owner == Owner::Class(c) {
                        nested.push((m, k));
                    }
                }
            }
            let mut module_fields: Vec<(FieldFlags, String, ErasedType)> = Vec::new();
            for (m, k) in nested {
                // The companion the typer made for a case class is among those below.
                if cx.input.reach.classes[k.idx()] && !cx.made_case_companion(k) {
                    module_fields.push((module_field_flags(syms, m), encode(cx.input.interner.get(syms.class(k).name)), ErasedType::L(self.class_name(k))));
                }
            }
            // And a companion the backend makes where the program has none (`emit_class_unit`),
            // public as a synthesized companion is.
            let mut made: Vec<(String, Rc<str>)> = Vec::new();
            for &k in syms.class(c).nested.values() {
                if syms.class(k).companion.map_or(true, |m| cx.made_case_companion(m)) && (cx.has_case_companion(k) || cx.has_value_companion(k) || cx.has_default_companion(k)) {
                    made.push((encode(cx.input.interner.get(syms.class(k).name)), Rc::from(format!("{}$", self.class_name(k)))));
                }
            }
            // By their names, then by their types as descriptors spell them.
            made.sort_by(|a, b| (&a.0, format!("L{};", a.1)).cmp(&(&b.0, format!("L{};", b.1))));
            module_fields.extend(made.into_iter().map(|(field, class)| (MODULE_FIELD, field, ErasedType::L(class))));
            for (flags, field, ty) in module_fields {
                self.field(flags, &field, &ty);
            }
            self.field(MODULE_FIELD, "MODULE$", &ErasedType::L(name.clone()));
            if clinit_first {
                self.module_clinit(&primary.as_ref().expect("an object's constructor").1);
                early_lookups = Some(self.lazy_holders.len());
            } else {
                module_clinit_wanted = true;
            }
        }
        // A given object of a package or of objects is scalac's module class, an object's
        // protocol (`BCodeSkelBuilder`, 181-263): `<clinit>` makes the instance, whose constructor
        // runs the superclass's alone, stores it in `MODULE$`, then runs the body, which may read
        // the given through `MODULE$`.
        if given_module {
            self.field(MODULE_FIELD, "MODULE$", &ErasedType::L(name.clone()));
            if clinit_first {
                self.module_clinit(&primary.as_ref().expect("an object's constructor").1);
                early_lookups = Some(self.lazy_holders.len());
            } else {
                module_clinit_wanted = true;
            }
        }
        self.given_object_fields(&tc.init);
        let instance_enum = cx.enum_of_case(c).or(is_enum.then_some(c)).is_some_and(|e| cx.instance_enum(e));
        if is_value_case {
            self.field(FieldFlags::PRIVATE, "$name", &ErasedType::L(Rc::from(STRING)));
            self.field(FieldFlags::PRIVATE, "$ordinal", &ErasedType::I);
            if !instance_enum {
                self.field(FieldFlags { visibility: Visibility::Public, is_static: true, ..FieldFlags::PRIVATE }, "$instance", &ErasedType::L(name.clone()));
            }
        }
        if is_enum && !instance_enum {
            // The values are made together, in declaration order, when the enum is first
            // touched: a read of one initialises its class, and with it this superclass. Those of
            // an enum read through its companion's fields the companion makes (`enum_value_parts`).
            // Its `<clinit>` is written last where it has lazy members (`enum_clinit`).
            if clinit_first {
                self.enum_clinit(c);
                early_lookups = Some(self.lazy_holders.len());
            }
            // What the companion calls so that the values exist before its own body runs.
            self.begin_method(MethodFlags { is_static: true, ..MethodFlags::PUBLIC }, "$touch", "()V", false, &[], ErasedType::V, &[]);
            self.body(|g| {
                let unit = g.lir.unit();
                g.lir.ret(unit)
            });
            self.end_method();
        }

        if is_trait {
            self.trait_members(tc);
            self.super_accessor_declarations(c);
            if !cx.ctor_defaults_on_companion(c) {
                let clauses = cx.input.syms.class(c).ctor.clone();
                self.ctor_default_getters(Some(c), Some(crate::tir::DefaultOf::Ctor(c)), &tc.ctor_params, &tc.ctor_defaults, &clauses);
            }
        } else {
            self.ctor_code = true;
            let (params, k) = primary.as_ref().expect("a class's constructor");
            self.constructor(params, k);
            self.ctor_code = false;
            for &f in &tc.ctors {
                if cx.input.reach.funs[f.idx()] {
                    self.secondary_ctor(f);
                }
            }
            for f in fields.iter().filter(|f| !cx.private_params.contains_key(&f.sym)) {
                self.accessors(f);
            }
            self.instance_case_companion_fields(c);
            self.trait_setters(c, &fields);
            self.class_file_storage(c);
        }
        let mut methods = tc.methods.clone();
        methods.sort_by(|&a, &b| crate::emit::layout::compare_syms(syms, cx.input.interner, prog.funs[a.idx()].sym, prog.funs[b.idx()].sym));
        for m in methods {
            if cx.input.reach.funs[m.idx()] {
                self.emit_fun(m);
            }
        }
        // A sealed class's companion's `ordinal` (`SyntheticMembers.addMirrorSupport`), and the
        // mirror members of a local class's companion, a local object of its block; the other
        // companions of a case class and of an enum have theirs by rule (`companion_members`,
        // `enum_companion_members`).
        // And of the companion of a case class of a block's object that the rule leaves, the class
        // taking more than its enclosing instance (`has_inner_written_case_companion`).
        let local = info.local_module.is_some() || (info.inner_object.is_some() && info.companion.is_some_and(|k| cx.in_a_block(k) && !cx.has_inner_written_case_companion(k)));
        if info.companion.is_some_and(|k| syms.class(k).kind != ClassKind::Enum && (local || syms.class(k).mods & mods::CASE == 0)) {
            // Published whatever this build reaches: a consumer's `Mirror.SumOf` calls it
            // (`SyntheticMembers.addMirrorSupport` enters it in every such companion).
            for &m in tc.mirror_members() {
                self.emit_fun(m);
            }
        }
        if syms.class(c).has_exports {
            let mut aliases: Vec<crate::types::AliasId> = syms.class(c).type_aliases.values().copied().filter(|&a| syms.aliases[a.idx()].owner == Owner::Class(c)).collect();
            aliases.sort();
            self.companion_forwarders(&aliases);
        }
        self.given_class_defs(Owner::Class(c), None);
        if is_trait || is_enum || is_abstract {
            self.abstract_members(c);
        }
        if !is_trait {
            self.super_accessors(tc);
            self.implemented_nested_objects(tc);
            self.emit_inherited_bridges(tc);
            self.trait_constant_bridges(tc);
            // A case value class's product members are its companion's (`value_class_members`).
            if !(info.value_class && cx.vc_on_companion(c)) {
                self.product_members(c, is_case || is_case_object, is_value_case, is_object || info.local_module.is_some() || info.inner_object.is_some());
            }
            if is_case_object {
                self.singleton_mirror_members();
            }
            if cx.has_copy(c) {
                self.copy_members(c);
            }
            if let Some(k) = case_companion {
                self.companion_members(k, Some(c));
            }
            if let Some(k) = info.companion.filter(|&k| {
                is_object && !cx.has_case_companion(k) && cx.ctor_defaults_on_companion(k) || info.local_module.is_some() && cx.ctor_defaults_on_local_companion(k)
            }) {
                self.ctor_default_getters_of(k);
            }
            if let Some(v) = info.companion.filter(|&v| is_object && cx.has_value_companion(v)) {
                self.extension_copies(v);
            }
            if let Some(e) = info.companion.filter(|&e| (is_object || instance_enum_companion) && syms.class(e).kind == ClassKind::Enum) {
                self.enum_companion_members(e, c);
            }
            if info.value_class && (!is_case || cx.vc_on_companion(c)) {
                self.value_class_members(c);
            }
            self.object_method_forwarders(c);
            if !is_enum {
                self.mixin_members(c);
                self.superclass_forwarders(c);
                self.superclass_bridges(c);
            }
            // After every member the class declares itself (a case object's `fromProduct`), which a
            // pair's bridge would otherwise stand for; before the link mode's guesses (`jar_bridges`).
            self.pair_bridges(tc);
            self.jar_bridges(c, tc.inherited_pairs());
            self.jar_trait_mixins(c);
            self.inner_case_companion_getters(c);
            if let Some(k) = info.companion.filter(|_| info.inner_object.is_some()) {
                self.inner_written_companion_members(k, c);
            }
        }
        // The `<clinit>`, its lazy members' `VarHandle` lookups first (`addInFront`, `MoveStatics`).
        if module_clinit_wanted {
            self.module_clinit(&primary.as_ref().expect("an object's constructor").1);
        } else if is_enum && !instance_enum && !clinit_first {
            self.enum_clinit(c);
        } else if !self.lazy_holders.is_empty() && !self.plan.has_method("<clinit>", "()V") {
            self.begin_method(self.clinit_flags(), "<clinit>", "()V", false, &[], ErasedType::V, &[]);
            self.body(|g| {
                let lookups = g.lower_var_handle_lookups();
                let unit = g.lir.unit();
                let done = g.lir.seq(&lookups, unit);
                g.lir.ret(done)
            });
            self.end_method();
        }
        if early_lookups.is_some_and(|n| self.lazy_holders.len() > n) {
            self.unsupported("a lazy member the names walk did not number (its handle has no lookup)");
        }
        // The methods of its code's local lazy vals, whichever class reads them.
        self.owned_lazy_methods(Some(c), info.file);
        self.drain_pending();
        // The targets of the class's super calls are interfaces of it (`initJClass`): a direct
        // trait another brings keeps its place among the direct ones, any other target comes
        // after the parents' list, in the order scalac's back end writes the class's bodies and
        // so fills its set (`super_call_place`). A call copied into a class that is no instance
        // of the interface (a value class's method in its companion) names no interface of the
        // class.
        if !self.super_targets.is_empty() {
            let mut recorded = std::mem::take(&mut self.super_targets);
            // The super accessors' targets stand trait by trait, the first mixed in first
            // (`ResolveSuper.transformTemplate` takes the class's mixins in that order), and
            // in a trait's own order inside it; the accessors themselves are written in the
            // linearisation's order.
            let mut accessors: Vec<(usize, usize, &str)> = Vec::new();
            for (i, a) in tc.super_accessors.iter().enumerate() {
                let Some(Owner::Class(o)) = a.target.map(|t| cx.input.syms.sym(t).owner) else { continue };
                if self.java_interface(o) {
                    let mixed = info.base_types.iter().position(|&(b, _)| b == a.of_trait).unwrap_or(0);
                    accessors.push((usize::MAX - mixed, i, &cx.class_names[o.idx()]));
                }
            }
            accessors.sort();
            for (target, place) in recorded.iter_mut() {
                if place.0 == 0 {
                    if let Some(n) = accessors.iter().position(|&(_, _, t)| t == target.as_str()) {
                        place.1 = n as u32 + 1;
                    }
                }
            }
            recorded.sort_by_key(|&(_, place)| place);
            let targets: Vec<String> = recorded.into_iter().map(|(t, _)| t).filter(|t| info.base_types.iter().skip(1).any(|&(b, _)| cx.class_names[b.idx()] == *t)).collect();
            let mut list: Vec<String> = direct.iter().filter(|(n, brought)| !brought || targets.contains(n)).map(|(n, _)| n.clone()).collect();
            list.extend(interfaces[direct_listed..declared].iter().cloned());
            for t in targets {
                if !list.contains(&t) && !interfaces[declared..].contains(&t) {
                    list.push(t);
                }
            }
            list.extend(interfaces[declared..].iter().cloned());
            self.cw.set_interfaces(&list);
        }
        self.finish_class(&name)
    }

    /// The `<clinit>` of an enum's class: its lazy members' handles, then the values, made
    /// together in declaration order when the enum is first touched (a read of one initialises its
    /// class, and with it this superclass); those of an enum read through its companion's fields
    /// the companion makes (`enum_value_parts`).
    fn enum_clinit(&mut self, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let companion_makes = cx.has_enum_fields(c);
        self.begin_method(self.clinit_flags(), "<clinit>", "()V", false, &[], ErasedType::V, &[]);
        self.body(|g| {
            let mut stmts = g.lower_var_handle_lookups();
            for &case in &syms.class(c).children {
                let cinfo = syms.class(case);
                if companion_makes || cinfo.singleton.is_none() || !cx.input.reach.classes[case.idx()] {
                    continue;
                }
                let case_name = g.class_name(case);
                let string = ErasedType::L(Rc::from(STRING));
                let init = method_ref(Invoke::Special, &case_name, false, "<init>", &[string.clone(), ErasedType::I], &ErasedType::V);
                let name = g.lir.text(&Rc::from(cx.input.interner.get(cinfo.name)), string);
                let ordinal = g.lir.int(cinfo.ordinal as i32);
                let made = g.lir.make(&init, &[name, ordinal]);
                stmts.push(g.lir.store_static(&case_name, "$instance", made));
            }
            let unit = g.lir.unit();
            let done = g.lir.seq(&stmts, unit);
            g.lir.ret(done)
        });
        self.end_method();
    }

    /// The static fields scalac declares in the owner of given objects, `Show$.given_Show_Int`
    /// of the given's module class, which nothing stores or reads: every read goes to the given's
    /// `MODULE$`.
    fn given_object_fields(&mut self, inits: &[TInit]) {
        let cx = self.cx;
        for init in inits {
            let TInit::Field(s, _) = *init else { continue };
            let Some(&k) = cx.given_objects.get(&s) else { continue };
            let name = self.field_name(s);
            let class = self.class_name(k);
            self.field(MODULE_FIELD, &name, &ErasedType::L(class));
        }
    }

    /// The start of a module class's `<clinit>`, as scalac's: the instance is made and stored in
    /// `MODULE$` before anything of the body runs, so the body may name the object, and the
    /// constructor has run the superclass's alone. The instance stays in a local that stands
    /// for `this` in the body, which the caller writes next.
    pub fn begin_module_clinit(&mut self, flags: MethodFlags, name: &Rc<str>, roots: &[TExprId]) {
        self.begin_method(flags, "<clinit>", "()V", false, &[], ErasedType::V, roots);
        self.outside(Unlowered::ModuleClinit);
        // The handles of the module class's lazy members come first (`MoveStatics.rebuild`'s
        // static constructor, which `genPlainClass` puts before the instance's making).
        self.var_handle_lookups();
        let this = ErasedType::L(name.clone());
        self.new_object(name);
        self.invoke(Invoke::Special, name, false, "<init>", &[], &ErasedType::V);
        self.dup();
        self.putstatic(name, "MODULE$", &this);
        let slot = self.store_new(&this);
        self.m.this_slot = Some(slot);
    }

    /// `<clinit>` of an object: the instance, the values of the enum it is the companion of,
    /// then the body of its constructor's tree (the traits it adds, its fields and its
    /// statements, in order), where the class file runs it (`Gen::write_module_part`).
    fn module_clinit(&mut self, k: &PrimaryCtor) {
        let tc = k.tc;
        let name = self.this_name.clone();
        let this = ErasedType::L(name.clone());
        let roots = self.primary_roots(k);
        self.begin_module_clinit(self.clinit_flags(), &name, &roots);
        if let Err(why) = self.constructor_lowers(k) {
            self.outside(why);
            let mut parts = Parts::Written;
            self.object_enum_parts(&mut parts, tc);
            self.class_body(&mut parts, tc, k.fields, &this);
            self.return_value(&ErasedType::V);
        } else {
            self.write_module(false, |g| g.primary_tree(k));
        }
        self.end_method();
    }

    /// What an object that is an enum's companion runs before its body (`Gen::enum_companion_parts`):
    /// the enum's `$touch` where the enum makes its values, the values it holds.
    fn object_enum_parts(&mut self, parts: &mut Parts, tc: &TClass) {
        let cx = self.cx;
        let name = self.this_name.clone();
        let companion = cx.input.syms.class(tc.id).companion;
        let values = companion.map_or(Vec::new(), |e| self.enum_value_fields(e));
        let touch = companion.filter(|&e| cx.input.syms.class(e).kind == ClassKind::Enum && cx.input.reach.classes[e.idx()] && !cx.has_enum_fields(e)).map(|e| self.class_name(e));
        self.enum_companion_parts(parts, &name, touch, &values);
    }

    /// The body of a class as its constructor runs it after the superclass's: the initialisers
    /// of the traits it adds, its fields and its statements, in the order they are written.
    fn class_body(&mut self, parts: &mut Parts, tc: &TClass, fields: &[FieldDef], this: &ErasedType) {
        let owner = self.this_name.clone();
        // The `$init$` of the traits the typer schedules no initialiser of run in their place
        // among the traits', before the class's own body.
        let order: Vec<ClassId> = self.cx.input.syms.class(tc.id).base_types.iter().skip(1).rev().map(|&(b, _)| b).collect();
        let place = |b: ClassId| order.iter().position(|&x| x == b).unwrap_or(usize::MAX);
        let mut scala2 = self.unscheduled_trait_inits(tc).into_iter().peekable();
        for init in &tc.init {
            let before = match *init {
                TInit::Parent(b, _) => place(b),
                TInit::Field(s, _) if matches!(self.cx.input.syms.sym(s).owner, Owner::Class(t) if t != tc.id) => 0,
                _ => usize::MAX,
            };
            while let Some(b) = scala2.next_if(|&b| place(b) < before) {
                self.part(parts, |g| g.lower_trait_init(b, this));
            }
            match *init {
                TInit::Field(s, e) => {
                    let Some(f) = fields.iter().find(|f| f.sym == s) else { continue };
                    if f.lazy {
                        continue;
                    }
                    // `Constructors`: a constant's initialiser that is no literal runs for its
                    // effects alone.
                    if let Some(constant) = f.constant {
                        if let ConstVal::Declared(_) = constant {
                            self.part(parts, |g| g.lower_expr(e, &ErasedType::V));
                        }
                        continue;
                    }
                    self.part(parts, |g| {
                        let instance = g.lir.this(this.clone());
                        let value = g.lower_expr(e, &f.ty);
                        g.lir.store(instance, &owner, &f.name, value)
                    });
                }
                TInit::Stmt(e) => self.part(parts, |g| g.lower_expr(e, &ErasedType::V)),
                TInit::Parent(b, call) => {
                    // The values of the block a local trait's body reads lead its arguments, and
                    // go to its `$init$` (`TClass::captures`).
                    let proxies = self.trait_init_proxies(b);
                    let k = call.captures;
                    let given: Vec<TExprId> = self.cx.input.prog.expr_list(call.args).iter().take(k as usize).copied().collect();
                    let call = ParentCall { args: crate::ast::ListRef { start: call.args.start + k, len: call.args.len - k }, captures: 0, ..call };
                    if !call.args.is_empty() && !self.trait_arguments(parts, b, call, fields, this) {
                        return;
                    }
                    if let Some((_, proxies, declared)) = proxies {
                        // The values as the code holds them: written in place (`Gen::class_body_lowers`).
                        assert!(matches!(parts, Parts::Written), "a local trait's proxies in a tree");
                        let base = self.class_name(b);
                        let mut types = vec![ErasedType::L(base.clone())];
                        types.extend(proxies.iter().map(|&p| self.anon_capture_type(p)));
                        let extras = self.extras_of(&declared, None);
                        types.extend(self.extras_types(&extras));
                        let slot = self.m.this_slot.unwrap_or(0);
                        self.load(slot, this);
                        self.load_proxies(&proxies, &declared, &given);
                        self.invoke(Invoke::Static, &base, true, "$init$", &types, &ErasedType::V);
                    } else if self.trait_has_init(b) {
                        self.part(parts, |g| g.lower_trait_init(b, this));
                    }
                }
            }
        }
        for b in scala2 {
            self.part(parts, |g| g.lower_trait_init(b, this));
        }
    }

    /// Whether every part of a class's body has nodes (`Gen::class_body`), or why not: no local
    /// trait's initialiser takes the values of the block (`Gen::load_proxies` reads the `Extras`).
    fn class_body_lowers(&mut self, tc: &TClass) -> Result<(), Unlowered> {
        tc.init.iter().try_for_each(|init| match *init {
            TInit::Parent(b, _) if self.trait_init_proxies(b).is_some() => Err(Unlowered::CtorTraitProxies),
            TInit::Parent(..) | TInit::Field(..) | TInit::Stmt(_) => Ok(()),
        })
    }

    /// The node `lower` makes, put in the parts of the body.
    fn part(&mut self, parts: &mut Parts, lower: impl FnOnce(&mut Self) -> LirId) {
        let node = self.lowering(lower);
        self.add(parts, &[node]);
    }

    fn add(&mut self, parts: &mut Parts, nodes: &[LirId]) {
        match parts {
            Parts::Tree(all, _) => all.extend_from_slice(nodes),
            Parts::Written => {
                for &node in nodes {
                    self.write(node);
                }
            }
        }
    }

    /// The tree of a body of no value whose statements are the parts.
    pub(super) fn returning(&mut self, parts: Parts) -> (LirId, Vec<Declared>) {
        let Parts::Tree(nodes, fields) = parts else { unreachable!("the parts of a tree") };
        let unit = self.lir.unit();
        let done = self.lir.seq(&nodes, unit);
        (self.lir.ret(done), fields)
    }

    /// The field declared before the next part: now where the parts are written in turn.
    fn declare(&mut self, parts: &mut Parts, flags: FieldFlags, name: &str, ty: &ErasedType) {
        match parts {
            Parts::Tree(nodes, fields) => fields.push(Declared { at: nodes.len(), flags, name: name.to_string(), ty: ty.clone() }),
            Parts::Written => self.field(flags, name, ty),
        }
    }

    /// What the companion object of an enum runs before its own body: the enum's `$touch` where
    /// another holds the values (`touch`), then the values it holds (`Gen::enum_value_parts`).
    pub(super) fn enum_companion_parts(&mut self, parts: &mut Parts, module: &Rc<str>, touch: Option<Rc<str>>, values: &[(String, SymId)]) {
        if let Some(enum_name) = touch {
            let touched = method_ref(Invoke::Static, &enum_name, false, "$touch", &[], &ErasedType::V);
            self.part(parts, |g| g.lir.static_call(&touched, &[]));
        }
        self.enum_value_parts(parts, module, values);
    }

    /// The values of an enum its companion `module` holds in its static fields, each declared as
    /// it is stored, in their order: those of an enum read through
    /// these fields (`has_enum_fields`) made here, each stored in its field as soon as it is made,
    /// as scalac's companion makes them (a value's constructor may read an earlier value through its
    /// field, `case B extends E(E.A)`), each its case's instance too (`$instance`); another enum's
    /// read from their cases' instances, once the enum made them all (the caller's `$touch`). An
    /// enum whose every case is a value has scalac's `$values` too, the array `values` copies.
    fn enum_value_parts(&mut self, parts: &mut Parts, module: &Rc<str>, values: &[(String, SymId)]) {
        let Some(&(_, first)) = values.first() else { return };
        let Owner::Class(o) = self.cx.input.syms.sym(first).owner else { return };
        let Some(e) = self.cx.input.syms.class(o).companion else { return };
        let enum_type = self.class_type(e);
        let desc = self.desc_of(&enum_type);
        let makes = self.cx.has_enum_fields(e);
        let string = ErasedType::L(Rc::from(STRING));
        for (name, v) in values {
            self.declare(parts, FieldFlags { enum_value: true, ..MODULE_FIELD }, name, &enum_type);
            let SymKind::EnumValue(case) = self.cx.input.syms.sym(*v).kind else { continue };
            let case_name = self.class_name(case);
            let t = ErasedType::L(case_name.clone());
            if makes {
                let cinfo = self.cx.input.syms.class(case);
                let label: Rc<str> = Rc::from(self.cx.input.interner.get(cinfo.name));
                let ordinal = cinfo.ordinal as i32;
                let ctor = method_ref(Invoke::Special, &case_name, false, "<init>", &[string.clone(), ErasedType::I], &ErasedType::V);
                self.part(parts, |g| {
                    let text = g.lir.text(&label, string.clone());
                    let at = g.lir.int(ordinal);
                    let made = g.lir.make(&ctor, &[text, at]);
                    let held = g.lir.temp();
                    let kept = g.lir.keep(held, made);
                    let instance = g.lir.kept(held, t.clone());
                    let own = g.lir.store_static(&case_name, "$instance", instance);
                    let value = g.lir.kept(held, t.clone());
                    let value = g.lower_adapt(value, &t, &enum_type);
                    let stored = g.lir.store_static(module, name, value);
                    g.lir.seq(&[kept, own], stored)
                });
            } else {
                self.part(parts, |g| {
                    let instance = g.lir.static_field(&case_name, "$instance", t.clone());
                    let value = g.lower_adapt(instance, &t, &enum_type);
                    g.lir.store_static(module, name, value)
                });
            }
        }
        if values.len() == self.cx.input.syms.class(e).children.len() {
            let array = ErasedType::L(Rc::from(format!("[{}", desc)));
            self.declare(parts, FieldFlags { is_static: true, is_final: true, ..FieldFlags::PRIVATE }, "$values", &array);
            self.part(parts, |g| {
                let mut items = Vec::with_capacity(values.len());
                for (name, _) in values {
                    items.push(g.lir.static_field(module, name, enum_type.clone()));
                }
                let items = g.lir.list(&items);
                let made = g.lir.push(Kind::ArrayOf { elem: enum_type.clone(), items }, array.clone());
                g.lir.store_static(module, "$values", made)
            });
        }
    }

    /// The statements a constructor's or a trait's arguments hold before them (the values of
    /// named and default arguments): an expression's and a val's, nothing else. Each val is a
    /// temporary the typer hoists ahead of the call (`Typer::hoist`, `elaborate_clause`), never lazy.
    fn prelude(&mut self, parts: &mut Parts, stmts: &[TStmt]) {
        for &s in stmts {
            match s {
                TStmt::Expr(_) | TStmt::Val(..) => {
                    let nodes = self.lowering(|g| {
                        let mut nodes = Vec::new();
                        g.lower_statement(s, &mut nodes);
                        nodes
                    });
                    self.add(parts, &nodes);
                }
                TStmt::Fun(_) | TStmt::Pat(..) => {}
            }
        }
    }

    /// Whether a val is lazy, which `Gen::local_val` gives a holder.
    pub(super) fn lazy_val(&self, s: SymId) -> bool {
        self.cx.input.syms.sym(s).mods & mods::LAZY != 0
    }

    /// How many of the arguments a class passes to the local trait `b` are the values of the
    /// block its body reads, and those values as `$init$` takes them.
    fn trait_init_proxies(&mut self, b: ClassId) -> Option<(usize, Vec<SymId>, Vec<SymId>)> {
        let cx = self.cx;
        let idx = cx.tclass_of[b.idx()];
        if idx == u32::MAX {
            return None;
        }
        let tc = &cx.input.prog.classes[idx as usize];
        if tc.captures == 0 {
            return None;
        }
        let k = tc.captures;
        let declared = tc.ctor_params[..k].to_vec();
        let proxies = self.proxy_syms(&declared);
        Some((k, proxies, declared))
    }

    /// The trait `b`'s initialiser run on the instance, `this` its type.
    fn lower_trait_init(&mut self, b: ClassId, this: &ErasedType) -> LirId {
        let base = self.class_name(b);
        let init = method_ref(Invoke::Static, &base, true, "$init$", &[ErasedType::L(base.clone())], &ErasedType::V);
        let instance = self.lir.this(this.clone());
        self.lir.static_call(&init, &[instance])
    }

    /// The traits a class mixes in anew that have an `$init$` the typer schedules no call of,
    /// least derived first, as dotty's `Mixin` calls the `$init$` of every such trait but a
    /// `NoInits` one (`superCallOpt`): the typer's parent initialisers count no lazy val (a
    /// trait of lazy vals alone has one), and a Scala 2 trait's transcoded pickle keeps no
    /// private member and no statement. A trait of neither the build nor a class file has none
    /// known here.
    fn unscheduled_trait_inits(&self, tc: &TClass) -> Vec<ClassId> {
        let cx = self.cx;
        let syms = cx.input.syms;
        let inherited = self.traits_implemented_above(tc.id);
        let mut out = Vec::new();
        for &(b, _) in syms.class(tc.id).base_types.iter().skip(1).rev() {
            if syms.class(b).kind != ClassKind::Trait || inherited.contains(&b) {
                continue;
            }
            if tc.init.iter().any(|i| matches!(*i, TInit::Parent(t, _) if t == b)) {
                continue;
            }
            if (cx.tclass_of[b.idx()] != u32::MAX || cx.class_files.contains_key(&b)) && self.trait_has_init(b) {
                out.push(b);
            }
        }
        out
    }

    /// Whether `@volatile` marks the val or var a field holds, which scalac's field carries as
    /// `ACC_VOLATILE` (`Mixin` copies a trait's var's annotations to the class's member,
    /// `Memoize` to its field): a source's by the annotation's class (`Worker::jvm_volatile`), a
    /// jar's or a product's by its pickle (`Loaded::volatile`).
    fn is_volatile(&self, s: SymId) -> bool {
        let input = &self.cx.input;
        input.volatile.map_or(false, |v| v.get(&s).is_some()) || input.source_volatile.map_or(false, |v| v.contains_key(&s))
    }

    /// Whether `@threadUnsafe` marks the lazy member `s` of the sources (`Worker::jvm_thread_unsafe`): dotty's
    /// `sym.isThreadUnsafe`, which `LazyVals.transformLazyVal` sends to `transformMemberDefThreadUnsafe`. A class
    /// holding a trait's marked member asks of the trait's symbol, whose annotations `MixinOps.mkForwarderSym`
    /// copies to the class's.
    pub(super) fn is_thread_unsafe(&self, s: SymId) -> bool {
        self.cx.input.source_thread_unsafe.map_or(false, |m| m.contains_key(&s))
    }

    /// What a class passes to the parameters of a trait it mixes in anew, before the trait's
    /// `$init$` runs, as dotty's `Mixin` writes it (`transformConstructor`, `traitInits`): the
    /// statements of named or default arguments first, then each argument stored into the
    /// field of its parameter, or evaluated and dropped where the class or a trait further down
    /// overrides the parameter's accessor (`Mixin` 301-303, so `class C extends T(side()) {
    /// override val x = 9 }` runs `side()` and T's body reads `x` as 0).
    fn trait_arguments(&mut self, parts: &mut Parts, b: ClassId, call: ParentCall, fields: &[FieldDef], this: &ErasedType) -> bool {
        let cx = self.cx;
        let prog = cx.input.prog;
        let owner = self.this_name.clone();
        let params = self.trait_params(b);
        let args = prog.expr_list(call.args).to_vec();
        if params.len() != args.len() {
            self.part(parts, |g| g.lower_refused(Refusal::TraitArgumentsMismatch));
            return false;
        }
        self.prelude(parts, &prog.stmts[call.prelude.range()]);
        for (p, arg) in params.into_iter().zip(args) {
            match fields.iter().find(|f| f.sym == p) {
                Some(f) => self.part(parts, |g| {
                    let instance = g.lir.this(this.clone());
                    let value = g.lower_expr(arg, &f.ty);
                    g.lir.store(instance, &owner, &f.name, value)
                }),
                None => self.part(parts, |g| g.lower_expr(arg, &ErasedType::V)),
            }
        }
        true
    }

    /// The parameters of a trait's constructor, every clause's in order.
    fn trait_params(&self, b: ClassId) -> Vec<SymId> {
        self.cx.input.syms.class(b).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect()
    }

    /// Whether a trait has the static `$init$` the classes mixing it in call: a trait of the
    /// build as `trait_members` writes one, a trait read from a class file as the file declares
    /// it (a trait of parameters and defs alone has none, as scalac's `NoInitsTrait`).
    fn trait_has_init(&self, b: ClassId) -> bool {
        let cx = self.cx;
        let idx = cx.tclass_of[b.idx()];
        if idx != u32::MAX {
            return self.source_trait_has_init(&cx.input.prog.classes[idx as usize]);
        }
        match cx.class_files.get(&b) {
            Some(file) => file.methods.iter().any(|m| m.name == "$init$" && m.access & ACC_STATIC != 0),
            None => true,
        }
    }

    /// scalac gives every trait with a concrete val, lazy val or object, or a statement, an
    /// `$init$`, which the classes mixing it in call.
    fn source_trait_has_init(&self, tc: &TClass) -> bool {
        self.cx.input.syms.class(tc.id).has_statements || tc.init.iter().any(|i| !matches!(i, TInit::Parent(..)))
    }

    /// The fields of a class: its constructor parameters, the vals of its body, the evidence of
    /// the traits it fills in, and the lazy members of its traits.
    fn class_fields(&mut self, tc: &TClass, captures: usize) -> Vec<FieldDef> {
        let cx = self.cx;
        let syms = cx.input.syms;
        let mut out: Vec<FieldDef> = Vec::new();
        for &p in &tc.ctor_params[captures..] {
            let ty = self.sym_type(p);
            let name = self.field_name(p);
            out.push(FieldDef { sym: p, name, ty, lazy: false, lazy_owner: None, init: None, mutable: syms.sym(p).kind == SymKind::Var, constant: None });
        }
        for init in &tc.init {
            let TInit::Field(s, e) = *init else { continue };
            // A given object of an object is its own module class, of no field here, and a given
            // class without parameter clauses (`given f[A]: F[A] with`) is scalac's def making one.
            if out.iter().any(|f| f.sym == s) || cx.given_objects.contains_key(&s) || cx.given_classes.contains_key(&s) {
                continue;
            }
            // The val of a made case companion the rule writes, its field and getter scalac's
            // (`instance_case_companion_fields`, `inner_case_companion_getters`).
            if crate::tir::is_made_case_companion_val(syms, s) {
                let made = syms.class(tc.id).nested.get(&syms.sym(s).name).and_then(|&k| syms.class(k).companion);
                if made.is_some_and(|m| cx.made_case_companion(m)) {
                    continue;
                }
            }
            let ty = self.sym_type(s);
            let name = self.field_name(s);
            let lazy = self.is_lazy_member(s);
            let constant = self.constant(s, e);
            out.push(FieldDef { sym: s, name, ty, lazy, lazy_owner: None, init: Some(e), mutable: syms.sym(s).kind == SymKind::Var, constant });
        }
        let inherited = self.traits_implemented_above(tc.id);
        let bases: Vec<ClassId> = syms.class(tc.id).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        // The members and names held already: a trait's val of a name the class or a trait
        // further down holds is theirs.
        let mut held_syms: FxMap<SymId, ()> = out.iter().map(|f| (f.sym, ())).collect();
        let mut held_names: FxMap<String, ()> = out.iter().map(|f| (f.name.clone(), ())).collect();
        for b in bases {
            let idx = cx.tclass_of[b.idx()];
            if syms.class(b).kind != ClassKind::Trait || inherited.contains(&b) {
                continue;
            }
            // The trait's parameters first, as `traitInits` writes them: a field each but where
            // the class or a trait further down overrides the accessor (`isInImplementingClass`),
            // the evidence of a trait that takes nothing else being the class's own already.
            for p in self.trait_params(b) {
                let name = self.field_name(p);
                if held_syms.contains_key(&p) || held_names.contains_key(&name) {
                    continue;
                }
                held_syms.insert(p, ());
                held_names.insert(name.clone(), ());
                let ty = self.sym_type(p);
                out.push(FieldDef { sym: p, name, ty, lazy: false, lazy_owner: None, init: None, mutable: syms.sym(p).kind == SymKind::Var, constant: None });
            }
            if idx == u32::MAX {
                let read = self.class_file_trait_fields(b);
                for f in read.fields.iter() {
                    if !held_syms.contains_key(&f.sym) && !held_names.contains_key(&f.name) {
                        held_syms.insert(f.sym, ());
                        held_names.insert(f.name.clone(), ());
                        out.push(f.clone());
                    }
                }
                for name in &read.constants {
                    held_names.entry(name.clone()).or_insert(());
                }
                continue;
            }
            for init in &cx.input.prog.classes[idx as usize].init {
                let TInit::Field(s, e) = *init else { continue };
                // A constant is the trait's own getter, of no field, which overrides a val of
                // its name further up as a val would (a trait parameter's: `Mixin` 301-303).
                if self.constant(s, e).is_some() {
                    let name = self.field_name(s);
                    held_names.entry(name).or_insert(());
                    continue;
                }
                if self.is_given_def(s) || held_syms.contains_key(&s) {
                    continue;
                }
                let name = self.field_name(s);
                if held_names.contains_key(&name) {
                    continue;
                }
                held_syms.insert(s, ());
                held_names.insert(name.clone(), ());
                let ty = self.sym_type(s);
                if self.is_lazy_member(s) {
                    out.push(FieldDef { sym: s, name, ty, lazy: true, lazy_owner: Some(b), init: Some(e), mutable: false , constant: None });
                } else {
                    out.push(FieldDef { sym: s, name, ty, lazy: false, lazy_owner: None, init: None, mutable: syms.sym(s).kind == SymKind::Var , constant: None });
                }
            }
        }
        out
    }

    /// A class's primary constructor: its parameters, and what is decided of its body before
    /// any of it is lowered. `enum_class` is the superclass or the enum the class is a value of.
    fn primary_ctor<'t>(
        &mut self,
        tc: &'t TClass,
        fields: &'t [FieldDef],
        super_name: &'t str,
        enum_class: Option<ClassId>,
        fields_first: bool,
        is_value_case: bool,
        is_object: bool,
        is_anon: bool,
    ) -> (Vec<(Option<SymId>, ErasedType)>, PrimaryCtor<'t>) {
        let cx = self.cx;
        let mut params: Vec<(Option<SymId>, ErasedType)> = Vec::new();
        // An enum over `java.lang.Enum` passes the name and ordinal of its values up to it.
        let java_enum = cx.is_java_enum(tc.id) || (is_value_case && cx.enum_of_case(tc.id).map_or(false, |e| cx.is_java_enum(e)));
        let n_captures = if is_anon { tc.ctor_params.len() } else { tc.captures };
        for t in self.anon_ctor_params(tc.id) {
            params.push((None, t));
        }
        // The name and ordinal follow what the class captures: a value of an enum of a block or a
        // class takes its companion's instance first (`enum_value_new`).
        let name_param = params.len();
        if is_value_case || java_enum {
            params.push((None, ErasedType::L(Rc::from(STRING))));
            params.push((None, ErasedType::I));
        }
        let first_super_arg = params.len();
        // An anonymous class over a class receives the arguments of the superclass's
        // constructor, evaluated where the instance is created, after its captures.
        let anon_super = if is_anon { self.anon_super_params(tc.id) } else { None };

        for t in anon_super.iter().flat_map(|(_, _, types)| types.iter()) {
            params.push((None, t.clone()));
        }
        for &p in &tc.ctor_params[n_captures..] {
            let t = self.sym_type(p);
            params.push((Some(p), t));
        }
        let route = self.super_call_route(tc, enum_class, anon_super.map(|(_, _, types)| types), is_anon);
        (params, PrimaryCtor { tc, fields, super_name, fields_first, is_value_case, is_object, is_anon, java_enum, n_captures, name_param, first_super_arg, route })
    }

    fn constructor(&mut self, params: &[(Option<SymId>, ErasedType)], k: &PrimaryCtor) {
        let cx = self.cx;
        let tc = k.tc;
        let roots = self.primary_roots(k);
        let types: Vec<ErasedType> = params.iter().map(|p| p.1.clone()).collect();
        // A static module is made by its own `<clinit>` only, as scalac's.
        let private = k.is_object && cx.statically_placed(tc.id) && cx.input.syms.class(tc.id).local_module.is_none();
        let visibility = if private { Visibility::Private } else { Visibility::Public };
        self.begin_method(MethodFlags { visibility, ..MethodFlags::PUBLIC }, "<init>", &method_desc(&types, &ErasedType::V), true, params, ErasedType::V, &roots);
        // The constructor reads what the class captures from its parameters, which the
        // arguments of the superclass's constructor need before the fields are set.
        let mut slot = 1u16;
        for (s, _, ty, held) in self.class_captures.clone() {
            let stored = self.anon_capture_type(s);
            match held {
                Held::Value => {
                    self.m.locals.insert(s, Local { slot, ty, cell: false });
                }
                Held::Cell => {
                    let ty = self.sym_type(s);
                    self.m.locals.insert(s, Local { slot, ty, cell: true });
                }
            }
            slot += if stored.wide() { 2 } else { 1 };
        }
        let extras = Extras { homes: self.class_homes.iter().map(|h| h.0).collect(), keys: self.class_keys.iter().map(|k| k.0).collect() };
        self.bind_extras(&extras, slot);
        if let Err(why) = self.constructor_lowers(k) {
            self.outside(why);
            self.constructor_parts(&mut Parts::Written, k);
            self.return_value(&ErasedType::V);
        } else if k.is_object {
            self.write_module(true, |g| g.primary_tree(k));
        } else {
            self.body(|g| {
                let (root, fields) = g.primary_tree(k);
                assert!(fields.is_empty(), "a class declares no field in its constructor's tree");
                root
            });
        }
        self.end_method();

        if !cx.ctor_defaults_on_companion(tc.id) && !cx.ctor_defaults_on_inner_companion(tc.id) && !cx.ctor_defaults_on_local_companion(tc.id) {
            let clauses = cx.input.syms.class(tc.id).ctor.clone();
            self.ctor_default_getters(Some(tc.id), Some(crate::tir::DefaultOf::Ctor(tc.id)), &tc.ctor_params, &tc.ctor_defaults, &clauses);
        }
    }

    /// What a primary constructor's locals are found shared in (`free::cells`): its body's
    /// initialisers and statements, an object's too, and the superclass's arguments.
    fn primary_roots(&self, k: &PrimaryCtor) -> Vec<TExprId> {
        let prog = self.cx.input.prog;
        let mut roots: Vec<TExprId> = Vec::new();
        for init in &k.tc.init {
            init_roots(prog, init, &mut roots);
        }
        if let Some(args) = k.tc.parent_args.filter(|_| !k.is_anon) {
            roots.extend_from_slice(prog.expr_list(args));
        }
        roots
    }

    /// Whether every part of a primary constructor has nodes, an object's body among them, so
    /// that the constructor is one tree; or why not.
    fn constructor_lowers(&mut self, k: &PrimaryCtor) -> Result<(), Unlowered> {
        self.super_call_lowers(k)?;
        self.class_body_lowers(k.tc)
    }

    /// Whether the lowering has nodes for the superclass's call by its route, or why not: every
    /// route but every parameter's default and the refused one.
    fn super_call_lowers(&self, k: &PrimaryCtor) -> Result<(), Unlowered> {
        match k.route {
            SuperCall::Defaults(..) => Err(Unlowered::CtorSuperDefaults),
            SuperCall::Own(_) | SuperCall::Declared(..) | SuperCall::Unwritten(..) | SuperCall::Via(..) | SuperCall::Captured(..) | SuperCall::Refused => Ok(()),
        }
    }

    /// A primary constructor's one tree (`Gen::constructor_lowers`).
    fn primary_tree(&mut self, k: &PrimaryCtor) -> (LirId, Vec<Declared>) {
        let mut parts = Parts::Tree(Vec::new(), Vec::new());
        self.constructor_parts(&mut parts, k);
        self.returning(parts)
    }

    /// One of the two methods of the class file an object's constructor tree is written in
    /// (`Gen::write_module_part`), its `<init>` or its `<clinit>`. The first of them written makes
    /// the tree with `make` and keeps it, with the locals its lowering declared, for the other.
    pub(super) fn write_module(&mut self, init: bool, make: impl FnOnce(&mut Self) -> (LirId, Vec<Declared>)) {
        let (root, fields, made) = match self.module_tree.take() {
            Some(kept) => {
                self.lir = kept.lir;
                self.lower_declared = kept.declared;
                (kept.root, kept.fields, false)
            }
            None => {
                let (root, fields) = self.lowering(make);
                (root, fields, true)
            }
        };
        self.write_module_part(root, init, &fields);
        if made {
            let lir = std::mem::take(&mut self.lir);
            let declared = std::mem::take(&mut self.lower_declared);
            self.module_tree = Some(ModuleTree { lir, declared, root, fields });
        }
    }

    /// How a primary constructor passes the arguments of its superclass's constructor
    /// (`SuperCall`), `superclass` the superclass or the enum it is a value of.
    fn super_call_route(&mut self, tc: &TClass, superclass: Option<ClassId>, anon_super: Option<Vec<ErasedType>>, is_anon: bool) -> SuperCall {
        let cx = self.cx;
        let Some(e) = superclass else { return SuperCall::Declared(Vec::new(), crate::ast::ListRef::EMPTY) };
        if let Some(types) = anon_super {
            return SuperCall::Own(types);
        }
        if !is_anon {
            // A superclass this build does not write is called as its class file declares the
            // selected constructor, the enclosing instance first where it takes one, as a creation
            // of it is (`ExplicitOuter.OuterOps.args` for `New` and `Super` alike).
            if let Some(args) = tc.parent_args.filter(|args| self.unwritten_ctor_layout(e, tc.parent_via).is_some_and(|(_, declared)| args.len as usize == declared.len() + 1)) {
                return SuperCall::Unwritten(e, args);
            }
            // A parent call that goes to a secondary constructor of the superclass, the
            // enclosing instance of an inner superclass first as a secondary `new` passes it
            // (`ExplicitOuter.OuterOps.args` for `New`, `Super` and `This` alike).
            if let Some(via) = tc.parent_via {
                return SuperCall::Via(e, via, tc.parent_args.unwrap_or(crate::ast::ListRef::EMPTY));
            }
            // A superclass nested in a class or local takes what it captures first.
            if let Some(args) = tc.parent_args.filter(|_| self.capture_count(e) > 0) {
                return SuperCall::Captured(e, args);
            }
        }
        let types = self.ctor_param_types(e);
        match tc.parent_args {
            Some(args) => SuperCall::Declared(types, args),
            None if types.is_empty() => SuperCall::Declared(types, crate::ast::ListRef::EMPTY),
            // Every parameter of the superclass has a default: each its getter's.
            None if cx.input.syms.class(e).ctor.iter().all(|cl| cl.params.iter().all(|p| p.has_default)) => SuperCall::Defaults(e, types),
            None => SuperCall::Refused,
        }
    }

    /// The parts of a primary constructor's body in order. A refused call of the superclass's
    /// constructor is the last: nothing after it is lowered or reported.
    fn constructor_parts(&mut self, parts: &mut Parts, k: &PrimaryCtor) {
        let prog = self.cx.input.prog;
        let tc = k.tc;
        let this = ErasedType::L(self.this_name.clone());
        let owner = self.this_name.clone();
        if k.is_value_case {
            // Before the enum's constructor runs, whose body may print the value.
            for (i, field) in [(k.name_param, "$name"), (k.name_param + 1, "$ordinal")] {
                self.part(parts, |g| {
                    let instance = g.lir.this(this.clone());
                    let value = g.param(i);
                    g.lir.store(instance, &owner, field, value)
                });
            }
        }
        // Scala sets the fields a class takes as parameters before the constructor of its
        // superclass runs, whose body may read them through an overridden member.
        if k.fields_first && !k.is_anon {
            self.param_fields(parts, tc, k.fields);
        }
        self.prelude(parts, &prog.stmts[tc.parent_prelude.range()]);
        self.super_call(parts, k);
        if let SuperCall::Refused = k.route {
            return;
        }
        if k.n_captures > 0 {
            // What the class captures, then the instances and the keys its copies of local defs
            // and lazy vals read, from the parameters that lead.
            let mut held: Vec<String> = self.class_captures.iter().map(|c| c.1.clone()).collect();
            held.extend(self.class_homes.iter().map(|h| h.1.clone()));
            held.extend(self.class_keys.iter().map(|x| x.1.clone()));
            for (i, field) in held.iter().enumerate() {
                self.part(parts, |g| {
                    let instance = g.lir.this(this.clone());
                    let value = g.param(i);
                    g.lir.store(instance, &owner, field, value)
                });
            }
        }
        if !k.is_anon && !k.fields_first {
            self.param_fields(parts, tc, k.fields);
        }
        // An object's body is its constructor's, which the class file runs in the `<clinit>`
        // (`Gen::write_module_part`); a constructor written in place leaves it to the `<clinit>`
        // written in place (`Gen::module_clinit`).
        if !(k.is_object && matches!(parts, Parts::Written)) {
            if k.is_object {
                self.object_enum_parts(parts, tc);
            }
            self.class_body(parts, tc, k.fields, &this);
        }
    }

    /// The fields of the class's own constructor parameters set from them.
    fn param_fields(&mut self, parts: &mut Parts, tc: &TClass, fields: &[FieldDef]) {
        let this = ErasedType::L(self.this_name.clone());
        let owner = self.this_name.clone();
        for &p in &tc.ctor_params {
            let Some(f) = fields.iter().find(|f| f.sym == p) else { continue };
            self.part(parts, |g| {
                let instance = g.lir.this(this.clone());
                let value = g.lower_held(p).expect("a constructor's parameter in its scope");
                g.lir.store(instance, &owner, &f.name, value)
            });
        }
    }

    /// The superclass's constructor run on the instance: the name and ordinal of a value of a
    /// Java enum first, then the arguments by the route.
    fn super_call(&mut self, parts: &mut Parts, k: &PrimaryCtor) {
        if self.super_call_lowers(k).is_ok() {
            self.part(parts, |g| g.lower_superclass_ctor(k));
            return;
        }
        // Every parameter's default, not lowered yet, written in place (`Gen::constructor`).
        assert!(matches!(parts, Parts::Written), "a route not lowered in a tree");
        let leading = k.leading_params();
        let mut params: Vec<ErasedType> = leading.iter().map(|&i| self.m.params[i].1.clone()).collect();
        let this = ErasedType::L(self.this_name.clone());
        self.load(0, &this);
        for &i in &leading {
            self.part(parts, |g| g.param(i));
        }
        let SuperCall::Defaults(e, types) = &k.route else { unreachable!("a route taken above") };
        self.m.before_super = true;
        self.all_ctor_defaults(*e, types);
        self.m.before_super = false;
        params.extend(types.iter().cloned());
        self.invoke(Invoke::Special, k.super_name, false, "<init>", &params, &ErasedType::V);
    }

    /// The superclass's constructor run on the instance, by a route that has nodes (`Gen::super_call_lowers`):
    /// the name and ordinal of a value of a Java enum, then an anonymous class's own parameters, the written
    /// arguments at the declared parameters' types, or the arguments as a creation of the superclass passes
    /// them through its selected constructor, what it captures first, or as its class file takes them
    /// (`Gen::lower_via_arguments`, `Gen::lower_new_arguments`, `Gen::lower_unwritten_ctor_arguments`).
    fn lower_superclass_ctor(&mut self, k: &PrimaryCtor) -> LirId {
        if let SuperCall::Refused = k.route {
            return self.lower_refused(Refusal::EnumCaseWithoutArguments);
        }
        let leading = k.leading_params();
        let mut params: Vec<ErasedType> = leading.iter().map(|&i| self.m.params[i].1.clone()).collect();
        let mut args: Vec<LirId> = leading.iter().map(|&i| self.param(i)).collect();
        self.m.before_super = true;
        let (nodes, types) = match &k.route {
            SuperCall::Own(types) => ((0..types.len()).map(|i| self.param(k.first_super_arg + i)).collect(), types.clone()),
            SuperCall::Declared(types, written) => {
                let (list, _) = self.lower_arguments(None, *written, types);
                (self.lir.lists[list.start as usize..(list.start + list.len) as usize].to_vec(), types.clone())
            }
            &SuperCall::Via(e, s, written) => {
                let route = self.via_arguments_route(e, s, written);
                self.lower_via_arguments(e, s, written, route)
            }
            &SuperCall::Captured(e, written) => {
                let route = self.ctor_arguments_route(e, written);
                self.lower_new_arguments(e, written, route)
            }
            &SuperCall::Unwritten(e, written) => {
                let via = k.tc.parent_via;
                let (outer, declared) = self.unwritten_ctor_layout(e, via).expect("its class file's layout, by the route");
                self.lower_unwritten_ctor_arguments(via, written, outer, declared)
            }
            SuperCall::Defaults(..) => unreachable!("not lowered (`Gen::super_call_lowers`)"),
            SuperCall::Refused => unreachable!("refused above"),
        };
        self.m.before_super = false;
        args.extend(nodes);
        params.extend(types);
        let ctor = method_ref(Invoke::Special, &Rc::from(k.super_name), false, "<init>", &params, &ErasedType::V);
        self.lir.init(&ctor, &args)
    }

    /// `def this(params) = { this(args); stmts }`: an `<init>` overload, as scalac's, that calls
    /// the constructor its self call goes to on the uninitialised `this` and then runs the
    /// statements.
    fn secondary_ctor(&mut self, f: FunId) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let fun = &prog.funs[f.idx()];
        let Some(body) = fun.body else { return };
        let TExpr::Block(stmts, after) = prog.expr(body) else { return };
        let Some((&TStmt::Expr(call), prelude)) = prog.stmts[stmts.range()].split_last() else { return };
        let Some(route) = self.self_call_route(call) else { return };
        // What a local class captures leads every constructor's parameters (dotty's `LambdaLift.addFreeParams`),
        // as the primary's and the default getters' take it: a `var` as its cell, a lazy val as its holder, a
        // local def's own captures in its place. A parameter that is not the capture itself (the enclosing
        // instance of a class nested in a class) is the constructor's own, as before.
        let own = self.this_class.map_or(Vec::new(), |c| self.via_own_captures(c, fun.sym));
        let mut params: Vec<(Option<SymId>, ErasedType)> = Vec::new();
        let mut held: Option<(Vec<(SymId, SymId)>, Extras, u16)> = None;
        let mut declared_from = 0;
        if let (true, Some(c)) = (own.contains(&true), self.this_class) {
            let captured: Vec<SymId> = cx.input.prog.classes[cx.tclass_of[c.idx()] as usize].ctor_params[..own.len()].to_vec();
            // Each parameter with the capture it stands for; then what the class's copies of its captured
            // local defs capture (`class_captured`).
            let mut slots: Vec<(SymId, SymId)> = captured.iter().enumerate().filter(|&(_, &cap)| !self.is_local_def(cap)).map(|(j, &cap)| (fun.params[j], cap)).collect();
            let through = self.class_captured(c).split_off(slots.len());
            slots.extend(through.into_iter().map(|s| (s, s)));
            let extras = self.class_extras(c);
            params.extend(slots.iter().map(|&(p, cap)| (Some(p), self.anon_capture_type(cap))));
            let extras_slot = 1 + params.iter().map(|(_, t)| if t.wide() { 2 } else { 1 }).sum::<u16>();
            params.extend(self.extras_types(&extras).into_iter().map(|t| (None, t)));
            held = Some((slots, extras, extras_slot));
            declared_from = own.len().min(fun.params.len());
        }
        params.extend(fun.params[declared_from..].iter().map(|&s| (Some(s), self.sym_type(s))));
        self.context = format!("{}.<init>", self.this_name);
        let taken: Vec<ErasedType> = params.iter().map(|p| p.1.clone()).collect();
        self.begin_method(MethodFlags::PUBLIC, "<init>", &method_desc(&taken, &ErasedType::V), true, &params, ErasedType::V, &[body]);
        if let Some((slots, extras, extras_slot)) = held {
            self.bind_extras(&extras, extras_slot);
            for &(p, cap) in &slots {
                if self.held(cap) == Held::Cell {
                    let ty = self.sym_type(cap);
                    if let Some(local) = self.m.locals.get_mut(&p) {
                        local.ty = ty;
                        local.cell = true;
                    }
                }
            }
        }
        self.body(|g| {
            let mut parts = Parts::Tree(Vec::new(), Vec::new());
            g.secondary_parts(&mut parts, prelude, route, after);
            g.returning(parts).0
        });
        self.end_method();
        if !self.this_class.map_or(false, |c| cx.ctor_defaults_on_companion(c) || cx.ctor_defaults_on_inner_companion(c) || cx.ctor_defaults_on_local_companion(c)) {
            let clauses: Vec<ClauseSig> = cx.input.syms.sym(fun.sym).sig.as_ref().map_or(Vec::new(), |sig| sig.clauses.clone());
            self.ctor_default_getters(None, Some(crate::tir::DefaultOf::Method(fun.sym)), &fun.params, &fun.defaults, &clauses);
        }
    }

    /// How a secondary constructor's call of another constructor of its class passes the
    /// arguments (`SelfCall`); nothing for a call of no class's constructor, whose method is left
    /// out.
    fn self_call_route(&mut self, call: TExprId) -> Option<SelfCall> {
        let cx = self.cx;
        match cx.input.prog.expr(call) {
            // The primary constructor, its captures (the enclosing instance of a class nested
            // in a class, the secondary's own outer parameter) first.
            TExpr::New(c, args) => match self.ctor_arguments_route(c, args) {
                CtorArguments::Declared => Some(SelfCall::Declared(c, args)),
                _ => Some(SelfCall::Primary(c, args)),
            },
            TExpr::NewVia(s, args) => match cx.input.syms.sym(s).owner {
                Owner::Class(c) => Some(SelfCall::Via(c, s, args)),
                _ => None,
            },
            _ => None,
        }
    }

    /// The parts of a secondary constructor's body in order: the statements before its call,
    /// the call, then what follows it.
    fn secondary_parts(&mut self, parts: &mut Parts, prelude: &[TStmt], route: SelfCall, after: TExprId) {
        self.prelude(parts, prelude);
        self.part(parts, |g| g.lower_self_call(route));
        self.part(parts, |g| g.lower_expr(after, &ErasedType::V));
    }

    /// A secondary constructor's call of another constructor of its class on the instance: the
    /// written arguments at the primary's declared parameters' types, or the arguments as a
    /// creation passes them, what the class captures first (`Gen::lower_new_arguments`), through a
    /// secondary (`Gen::lower_via_arguments`).
    fn lower_self_call(&mut self, route: SelfCall) -> LirId {
        let owner = self.this_name.clone();
        self.m.before_super = true;
        let (args, types) = match route {
            SelfCall::Declared(c, args) => {
                let types = self.ctor_param_types(c);
                let (list, _) = self.lower_arguments(None, args, &types);
                (self.lir.lists[list.start as usize..(list.start + list.len) as usize].to_vec(), types)
            }
            SelfCall::Primary(c, args) => {
                let route = self.ctor_arguments_route(c, args);
                self.lower_new_arguments(c, args, route)
            }
            SelfCall::Via(c, s, args) => {
                let route = self.via_arguments_route(c, s, args);
                self.lower_via_arguments(c, s, args, route)
            }
        };
        self.m.before_super = false;
        let ctor = method_ref(Invoke::Special, &owner, false, "<init>", &types, &ErasedType::V);
        self.lir.init(&ctor, &args)
    }

    /// The defaults of a constructor of a class whose companion is not static (an inner or a
    /// local class), as static getters of the class: each takes what the class captures, then
    /// the parameters of the clauses before the default's own, as a companion's getter does.
    /// The static getters of a class's constructor defaults (`class`'s, a local class taking
    /// what it captures first, as its constructor does: a `var` as its cell).
    fn ctor_default_getters(&mut self, class: Option<ClassId>, of: Option<crate::tir::DefaultOf>, params: &[SymId], defaults: &[Option<TExprId>], clauses: &[ClauseSig]) {
        let declared: usize = clauses.iter().map(|c| c.params.len()).sum();
        let captures = params.len().saturating_sub(declared);
        let (captured, extras): (Vec<SymId>, Extras) = match class {
            Some(c) if captures > 0 && self.cx.input.syms.class(c).kind != ClassKind::Anon => (self.class_captured(c), self.class_extras(c)),
            _ => (params[..captures].to_vec(), Extras::default()),
        };
        let extras_types = self.extras_types(&extras);
        for (i, d) in defaults.iter().enumerate() {
            let Some(d) = *d else { continue };
            if i < captures {
                continue;
            }
            let upto = captures + super::gen::clause_shape(clauses, i - captures).0;
            let mut before: Vec<(Option<SymId>, ErasedType)> = captured.iter().map(|&p| (Some(p), self.anon_capture_type(p))).collect();
            let extras_slot = before.iter().map(|(_, t)| if t.wide() { 2 } else { 1 }).sum::<u16>();
            before.extend(extras_types.iter().map(|t| (None, t.clone())));
            before.extend(params[captures..upto.min(i)].iter().map(|&p| (Some(p), self.sym_type(p))));
            // The getter's type as the typer completed it (`Namer.inferredResultType`), the
            // parameter's where nothing refers to the class's type parameters.
            let declared_ty = clauses.iter().flat_map(|c| c.params.iter()).nth(i - captures).map(|p| p.ty);
            let own = of.and_then(|of| self.cx.input.getters.and_then(|t| crate::typer::getters::getter_type(t, self.cx.input.syms, of, i - captures))).filter(|&t| Some(t) != declared_ty);
            let ret = match own {
                Some(t) => self.erase(t),
                None => self.sym_type(params[i]),
            };
            let by_name = super::gen::clause_shape(clauses, i - captures).1.is_some();
            let types: Vec<ErasedType> = before.iter().map(|p| p.1.clone()).collect();
            self.begin_method(MethodFlags { is_static: true, ..MethodFlags::PUBLIC }, &format!("$lessinit$greater$default${}", i + 1), &method_desc(&types, &ret), false, &before, ret.clone(), &[d]);
            self.bind_extras(&extras, extras_slot);
            for &s in &captured {
                if self.held(s) == Held::Cell {
                    let ty = self.sym_type(s);
                    if let Some(local) = self.m.locals.get_mut(&s) {
                        local.ty = ty;
                        local.cell = true;
                    }
                }
            }
            self.body(|g| {
                let value = g.lower_default_value(d, by_name, &ret);
                g.lir.ret(value)
            });
            self.end_method();
        }
    }

    /// The class's own lazy members that `transformMemberDefThreadSafe` writes (`Gen::walk_lazy_names`).
    pub(super) fn lazy_field_syms(&mut self, tc: &TClass) -> FxMap<SymId, ()> {
        let captures = if self.cx.input.syms.class(tc.id).kind == ClassKind::Anon { tc.ctor_params.len() } else { tc.captures };
        self.class_fields(tc, captures).iter().filter(|f| f.lazy && f.lazy_owner.is_none()).map(|f| (f.sym, ())).collect()
    }

    /// The getters of the forwarders `Mixin` gives the class for its traits' lazy members, in the order the class
    /// holds them (`class_fields`' trait part, then `class_file_storage`'s holders), each with its name as written
    /// (a class file's holder by its name there decoded, as dotty's unpickler reads it) and whether `@threadUnsafe`
    /// marks it (`Gen::is_thread_unsafe`: a member of the build's sources alone).
    pub(super) fn lazy_forwarder_names(&mut self, tc: &TClass) -> Vec<(String, String, bool)> {
        let c = tc.id;
        let captures = if self.cx.input.syms.class(c).kind == ClassKind::Anon { tc.ctor_params.len() } else { tc.captures };
        let fields = self.class_fields(tc, captures);
        let interner = self.cx.input.interner;
        let mut out: Vec<(String, String, bool)> = fields.iter().filter(|f| f.lazy && f.lazy_owner.is_some()).map(|f| (f.name.clone(), interner.get(self.cx.input.syms.sym(f.sym).name).to_string(), self.is_thread_unsafe(f.sym))).collect();
        let cx = self.cx;
        let syms = cx.input.syms;
        let inherited = self.traits_implemented_above(c);
        let bases: Vec<ClassId> = syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        for b in bases {
            if syms.class(b).kind != ClassKind::Trait || inherited.contains(&b) || cx.tclass_of[b.idx()] != u32::MAX {
                continue;
            }
            let read = self.class_file_trait_fields(b);
            for (name, _, _) in &read.holders {
                if !fields.iter().any(|f| &f.name == name) && !out.iter().any(|(n, _, _)| n == name) {
                    out.push((name.clone(), super::names::decode(name), false));
                }
            }
        }
        out
    }

    fn accessors(&mut self, f: &FieldDef) {
        let owner = self.this_name.clone();
        let getter_desc = method_desc(&[], &f.ty);
        // A `@threadUnsafe` member's getter computes its right-hand side itself (`Gen::thread_unsafe_getter`).
        let thread_unsafe = f.lazy && self.is_thread_unsafe(f.sym);
        // A trait's final val, object and given are final in the class that holds them, as
        // scalac's `Mixin` copies the member's flags (`mkForwarderSym`); a using parameter's is
        // not, as the parameter is no given definition.
        let info = self.cx.input.syms.sym(f.sym);
        let from_trait = matches!(info.owner, Owner::Class(t) if self.cx.input.syms.class(t).kind == ClassKind::Trait);
        let param = matches!(info.owner, Owner::Class(t) if self.trait_params(t).contains(&f.sym));
        let fin = info.mods & mods::FINAL != 0 || (!param && (info.mods & mods::GIVEN != 0 || info.kind == SymKind::Given));
        // So is the implementation of a deferred given, as dotty's `implementDeferredGivens` makes it,
        // and the getter of a class's given object or object, scalac's module (`ModuleValCreationFlags`
        // hold `Final`), or of a lazy given alias or a `final lazy val`, whose symbol LazyVals keeps.
        let module = self.cx.inner_given_objects.contains_key(&f.sym) || matches!(info.kind, SymKind::Object(_));
        let lazy_final = f.lazy && fin && !from_trait;
        // A constant's getter is final, as the val is (`Memoize`).
        let is_final = (from_trait && fin) || module || lazy_final || self.implements_deferred(f.sym) || f.constant.is_some();
        self.begin_method(MethodFlags { is_final, ..MethodFlags::PUBLIC }, &f.name, &getter_desc, false, &[], f.ty.clone(), &f.init.filter(|_| thread_unsafe && f.lazy_owner.is_none()).into_iter().collect::<Vec<_>>());
        let mut holder = None;
        if thread_unsafe {
            let init = match (f.lazy_owner, f.init) {
                (Some(t), _) => Some(HolderInit::TraitStatic(self.class_name(t))),
                (None, Some(init)) => Some(HolderInit::Expr(init)),
                (None, None) => None,
            };
            if init.is_none() {
                self.unsupported("a lazy member without an initialiser");
            }
            self.thread_unsafe_getter(&f.name, &f.ty, init);
        } else if f.lazy {
            // `transformMemberDefThreadSafe`: the getter over the holder, the initialiser after it.
            let init = match (f.lazy_owner, f.init) {
                (Some(t), _) => Some(HolderInit::TraitStatic(self.class_name(t))),
                (None, Some(init)) => Some(HolderInit::Expr(init)),
                (None, None) => None,
            };
            let h = self.lazy_holder(&f.name, &f.ty);
            if init.is_none() {
                self.unsupported("a lazy member without an initialiser");
            }
            self.thread_safe_getter(&h);
            holder = init.map(|init| (h, init));
        } else if let (Some(constant), Some(init)) = (f.constant, f.init) {
            self.body(|g| {
                let value = g.lower_constant_value(Some(constant), init, &f.ty);
                g.lir.ret(value)
            });
        } else {
            self.body(|g| g.getter_body(&f.name, &f.ty));
        }
        self.end_method();
        let getter = MethodRef {
            owner: owner.clone(),
            owner_is_interface: false,
            name: f.name.clone(),
            params: Vec::new(),
            ret: f.ty.clone(),
            desc: getter_desc,
            kind: Invoke::Virtual,
        };
        // The bridges are made against the member's signature: a trait's read from a class file
        // is completed by the typer (`complete_library_trait_fields`), and without one there is
        // no erasure to bridge from.
        let typed = self.cx.input.syms.sym(f.sym).sig.is_some();
        if typed {
            self.emit_bridges(f.sym, &getter);
        }
        if let Some((h, init)) = holder {
            self.thread_safe_init(&h, &init);
        }
        if f.mutable {
            let setter = MethodRef {
                owner: owner.clone(),
                owner_is_interface: false,
                name: format!("{}_$eq", f.name),
                params: vec![f.ty.clone()],
                ret: ErasedType::V,
                desc: method_desc(&[f.ty.clone()], &ErasedType::V),
                kind: Invoke::Virtual,
            };
            self.begin_method(MethodFlags::PUBLIC, &setter.name, &setter.desc, false, &[(None, f.ty.clone())], ErasedType::V, &[]);
            self.body(|g| {
                let given = g.param(0);
                g.setter_body(&f.name, given)
            });
            self.end_method();
            let setter_name = setter_member_name(self.cx.input.interner, self.cx.input.syms.sym(f.sym).name);
            if let Some(name) = setter_name.filter(|_| typed) {
                self.emit_bridges_as(f.sym, name, &setter);
            }
        }
    }

    /// Whether `s` is the implementation the class being written was given for a deferred given
    /// (`TClass::deferred_givens`).
    fn implements_deferred(&self, s: SymId) -> bool {
        let cx = self.cx;
        let Some(c) = self.this_class else { return false };
        let i = cx.tclass_of[c.idx()];
        i != u32::MAX && cx.input.prog.classes[i as usize].deferred_givens.iter().any(|&(_, d)| d == s)
    }

    /// A trait is an interface, as under scalac. A val or var with an initialiser is an
    /// abstract getter and setter (`T$_setter_$x_$eq` for a val, `x_$eq` for a var) that the
    /// implementing classes define over a field of their own, and its initialiser runs in the
    /// static `$init$` with the body's statements, in their order. A lazy member is a default
    /// method computing the value, reached from the implementing class's lazy accessor through
    /// the static `x$`. Evidence is an abstract accessor.
    fn trait_members(&mut self, tc: &TClass) {
        let cx = self.cx;
        let this = ErasedType::L(self.this_name.clone());
        let owner = self.this_name.clone();
        // The values of the block the body reads come first: parameters of `$init$` alone.
        for &p in &tc.ctor_params[tc.captures..] {
            let ty = self.sym_type(p);
            let name = self.field_name(p);
            self.abstract_method(MethodFlags::ABSTRACT, &name, &method_desc(&[], &ty));
            if cx.input.syms.sym(p).kind == SymKind::Var {
                self.abstract_method(MethodFlags::ABSTRACT, &format!("{}_$eq", name), &method_desc(&[ty], &ErasedType::V));
            }
        }
        // The field an initialiser stores into, if any, and the expression, in body order.
        let mut runs: Vec<(Option<SymId>, TExprId)> = Vec::new();
        for init in &tc.init {
            match *init {
                // A given class without parameter clauses (`given f[A]: F[A] with`) is scalac's
                // def making one (`given_class_defs`).
                TInit::Field(s, _) if cx.given_classes.contains_key(&s) => {}
                TInit::Field(s, e) if self.is_lazy_member(s) => {
                    let ty = self.sym_type(s);
                    let name = self.field_name(s);
                    let desc = method_desc(&[], &ty);
                    self.begin_method(MethodFlags::PUBLIC, &name, &desc, false, &[], ty.clone(), &[e]);
                    self.body(|g| {
                        let value = g.lower_expr(e, &ty);
                        g.lir.ret(value)
                    });
                    self.end_method();
                    self.begin_method(MethodFlags { is_static: true, ..MethodFlags::PUBLIC }, &format!("{}$", name), &method_desc(&[this.clone()], &ty), false, &[(None, this.clone())], ty.clone(), &[]);
                    let computed = method_ref(Invoke::Special, &owner, true, &name, &[], &ty);
                    self.body(|g| {
                        let instance = g.param(0);
                        let value = g.lir.call(&computed, instance, &[]);
                        g.lir.ret(value)
                    });
                    self.end_method();
                }
                TInit::Field(s, e) if self.constant(s, e).is_some() => {
                    let constant = self.constant(s, e);
                    let ty = self.sym_type(s);
                    let name = self.field_name(s);
                    self.begin_method(MethodFlags::PUBLIC, &name, &method_desc(&[], &ty), false, &[], ty.clone(), &[e]);
                    self.body(|g| {
                        let value = g.lower_constant_value(constant, e, &ty);
                        g.lir.ret(value)
                    });
                    self.end_method();
                    self.trait_static_forwarder(&name, &[], &ty);
                    // `Constructors`: an initialiser that is no literal runs in `$init$`.
                    if let Some(ConstVal::Declared(_)) = constant {
                        runs.push((None, e));
                    }
                }
                TInit::Field(s, e) => {
                    let ty = self.sym_type(s);
                    let name = self.field_name(s);
                    let desc = method_desc(&[], &ty);
                    if !self.plan.has_method(&name, &desc) {
                        self.abstract_method(MethodFlags::ABSTRACT, &name, &desc);
                    }
                    let setter = self.trait_setter_name(s);
                    let setter_desc = method_desc(&[ty], &ErasedType::V);
                    if !self.plan.has_method(&setter, &setter_desc) {
                        self.abstract_method(MethodFlags::ABSTRACT, &setter, &setter_desc);
                    }
                    runs.push((Some(s), e));
                }
                TInit::Stmt(e) => runs.push((None, e)),
                TInit::Parent(..) => {}
            }
        }
        if self.source_trait_has_init(tc) {
            let roots: Vec<TExprId> = runs.iter().map(|r| r.1).collect();
            let proxies = self.proxy_syms(&tc.ctor_params[..tc.captures]);
            let mut params = vec![(None, this.clone())];
            for &p in &proxies {
                let t = self.anon_capture_type(p);
                params.push((Some(p), t));
            }
            let extras_slot = params.iter().map(|(_, t)| if t.wide() { 2 } else { 1 }).sum::<u16>();
            let extras = self.extras_of(&tc.ctor_params[..tc.captures], None);
            for t in self.extras_types(&extras) {
                params.push((None, t));
            }
            let types: Vec<ErasedType> = params.iter().map(|p| p.1.clone()).collect();
            self.begin_method(MethodFlags { is_static: true, ..MethodFlags::PUBLIC }, "$init$", &method_desc(&types, &ErasedType::V), false, &params, ErasedType::V, &roots);
            self.bind_proxies(&proxies);
            self.bind_extras(&extras, extras_slot);
            self.m.this_slot = Some(0);
            self.body(|g| {
                let mut stmts = Vec::with_capacity(runs.len());
                for (field, e) in runs {
                    stmts.push(match field {
                        Some(s) => {
                            let ty = g.sym_type(s);
                            let setter = g.trait_setter_name(s);
                            let store = method_ref(Invoke::Interface, &owner, true, &setter, std::slice::from_ref(&ty), &ErasedType::V);
                            let instance = g.param(0);
                            let value = g.lower_expr(e, &ty);
                            g.lir.call(&store, instance, &[value])
                        }
                        None => g.lower_expr(e, &ErasedType::V),
                    });
                }
                let unit = g.lir.unit();
                let done = g.lir.seq(&stmts, unit);
                g.lir.ret(done)
            });
            self.end_method();
        }
    }

    /// The traits whose vals the superclass (an enum case's enum) implements already, as the
    /// typer's `parent_inits` leaves their `$init$` to it: a class implements the vals of the
    /// traits it mixes in anew, as scalac does, so that a class file on the class path keeps its
    /// final accessors (scalatest's `AnyFunSuite` over `AnyFunSuiteLike`'s `engine`).
    fn traits_implemented_above(&self, c: ClassId) -> Vec<ClassId> {
        let syms = self.cx.input.syms;
        let info = syms.class(c);
        let above = match (info.kind, info.owner) {
            (ClassKind::EnumCase, Owner::Class(companion)) => syms.class(companion).companion,
            _ => info.superclass,
        };
        above.map_or(Vec::new(), |a| syms.class(a).base_types.iter().skip(1).map(|&(b, _)| b).collect())
    }

    /// The setter through which a trait's `$init$` stores a val or var of the trait:
    /// scalac's `p$T$_setter_$x_$eq` for a val, the var's own `x_$eq`.
    fn trait_setter_name(&mut self, s: SymId) -> String {
        let field = self.field_name(s);
        let info = self.cx.input.syms.sym(s);
        match info.owner {
            Owner::Class(t) if info.kind != SymKind::Var => format!("{}$_setter_${}_$eq", self.class_name(t).replace('/', "$"), field),
            _ => format!("{}_$eq", field),
        }
    }

    /// The setters of the vals of the traits a class mixes in, each storing into the class's
    /// field of the val's name where that field holds the trait's val or a val of the class's
    /// own that overrides it, and doing nothing where a trait further down holds it, or the
    /// class's member is a lazy val or a constructor parameter. A trait read from a class file
    /// declares the setter's descriptor.
    fn trait_setters(&mut self, c: ClassId, fields: &[FieldDef]) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let inherited = self.traits_implemented_above(c);
        let params: Vec<SymId> = syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
        let bases: Vec<ClassId> = syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        // A trait's place in the linearisation: the later, the earlier the class mixes it in.
        let position = |t: ClassId| bases.iter().position(|&b| b == t).unwrap_or(0);
        let mut by_name: FxMap<&str, usize> = FxMap::default();
        for (i, f) in fields.iter().enumerate() {
            by_name.entry(f.name.as_str()).or_insert(i);
        }
        for &b in &bases {
            let idx = cx.tclass_of[b.idx()];
            if syms.class(b).kind != ClassKind::Trait || inherited.contains(&b) {
                continue;
            }
            let traits_fields: Vec<(SymId, Option<ErasedType>)> = if idx == u32::MAX {
                self.class_file_trait_fields(b).fields.iter().map(|f| (f.sym, Some(f.ty.clone()))).collect()
            } else {
                cx.input.prog.classes[idx as usize].init.iter().filter_map(|init| match *init {
                    TInit::Field(s, e) if self.constant(s, e).is_none() => Some((s, None)),
                    _ => None,
                }).collect()
            };
            for (s, declared) in traits_fields {
                if self.is_lazy_member(s) || syms.sym(s).kind == SymKind::Var {
                    continue;
                }
                let ty = declared.unwrap_or_else(|| self.sym_type(s));
                let setter = self.trait_setter_name(s);
                let desc = method_desc(&[ty.clone()], &ErasedType::V);
                if self.plan.has_method(&setter, &desc) {
                    continue;
                }
                // scalac's `Memoize` (156-181) stores into the class's field of the member's name
                // where one exists when `Mixin` writes the setter: a val of the class's body, or
                // the field of this trait's val or of a trait before it in the order the class
                // mixes them in (`transformTemplate`, 345-354), so that the trait's `$init$` reads
                // its value; nothing where the class's member is a lazy val or a constructor
                // parameter (163-168), or a trait after this one holds the field.
                let name = self.field_name(s);
                let held = by_name.get(name.as_str()).map(|&i| &fields[i]).filter(|f| !f.lazy && f.constant.is_none() && !params.contains(&f.sym)).filter(|f| match syms.sym(f.sym).owner {
                    Owner::Class(k) if k == c => true,
                    Owner::Class(t) => position(t) >= position(b),
                    _ => false,
                });
                self.begin_method(MethodFlags::PUBLIC, &setter, &desc, false, &[(None, ty.clone())], ErasedType::V, &[]);
                self.body(|g| match held {
                    Some(f) => {
                        let given = g.param(0);
                        let value = g.lower_adapt(given, &ty, &f.ty);
                        g.setter_body(&f.name, value)
                    }
                    None => {
                        let unit = g.lir.unit();
                        g.lir.ret(unit)
                    }
                });
                self.end_method();
            }
        }
    }

    /// The fields a trait read from a class file gives a class mixing it in, another module's
    /// trait or a jar's, as scalac's `Mixin` makes them (`traitInits` and `setters`), in the
    /// class file's order: a val's (the abstract `T$_setter_$x_$eq`), a var's (the abstract
    /// `x_$eq`), a lazy val's or a given's (its static `x$`, which computes the value), each with
    /// the member the field holds. A private member's names are expanded, `T$$x` (`scala.App`'s
    /// `_args`, `scala$App$$_args_$eq`). The class lays them out where it lays out a source
    /// trait's, so that a build over the trait's products writes the class the whole build
    /// writes. They depend on the trait alone, and the emitter thread keeps them.
    fn class_file_trait_fields(&mut self, b: ClassId) -> Rc<ClassFileTrait> {
        if let Some(fields) = self.trait_fields.get(&b) {
            return fields.clone();
        }
        let fields = Rc::new(self.read_class_file_trait_fields(b));
        self.trait_fields.insert(b, fields.clone());
        fields
    }

    fn read_class_file_trait_fields(&mut self, b: ClassId) -> ClassFileTrait {
        let cx = self.cx;
        let syms = cx.input.syms;
        let Some(file) = cx.class_files.get(&b).cloned() else { return ClassFileTrait { fields: Vec::new(), unmatched: Vec::new(), constants: Vec::new(), vars: Vec::new(), holders: Vec::new() } };
        let mut members: FxMap<String, SymId> = FxMap::default();
        for &s in &syms.class(b).member_order {
            let info = syms.sym(s);
            let field = info.owner == Owner::Class(b)
                && match info.kind {
                    SymKind::Val => true,
                    // An abstract var's setter is the abstract `x_$eq` too, of no field.
                    SymKind::Var => info.mods & mods::ABSTRACT == 0,
                    // A product's given (a jar's is a val) of no parameters; one with parameters
                    // is scalac's def (`is_given_def`), and one without a signature is neither.
                    SymKind::Given => info.sig.is_some() && !self.is_given_def(s),
                    _ => false,
                };
            if field {
                let name = self.field_name(s);
                members.entry(name).or_insert(s);
            }
        }
        let setter_prefix = format!("{}$_setter_$", self.class_name(b).replace('/', "$"));
        let mut out = Vec::new();
        let mut unmatched = Vec::new();
        for m in &file.methods {
            let (name, lazy, mutable) = if let Some(x) = m.name.strip_prefix(&setter_prefix).and_then(|x| x.strip_suffix("_$eq")) {
                if m.access & ACC_ABSTRACT != 0 && !members.get(x).map_or(false, |&s| matches!(syms.sym(s).kind, SymKind::Val | SymKind::Given) && !self.is_lazy_member(s)) {
                    unmatched.push((m.name.clone(), m.descriptor.clone()));
                    continue;
                }
                (x, false, false)
            } else if m.access & ACC_STATIC != 0 {
                let Some(x) = m.name.strip_suffix('$') else { continue };
                (x, true, false)
            } else if m.access & ACC_ABSTRACT != 0 {
                let Some(x) = m.name.strip_suffix("_$eq") else { continue };
                (x, false, true)
            } else {
                continue;
            };
            let Some(&s) = members.get(name) else { continue };
            if mutable != (syms.sym(s).kind == SymKind::Var) || lazy != self.is_lazy_member(s) {
                continue;
            }
            let (params, ret) = parse_method_desc(&m.descriptor);
            // A lazy member's static `x$(T)` takes the trait alone, a setter its value.
            let ty = match params.as_slice() {
                [_] if lazy => ret,
                [p] => p.clone(),
                _ => continue,
            };
            out.push(FieldDef { sym: s, name: name.to_string(), ty, lazy, lazy_owner: lazy.then_some(b), init: None, mutable , constant: None });
        }
        // A val whose getter the trait defines, of no setter, is a constant (`constant`).
        let constants = members
            .iter()
            .filter(|&(_, &s)| syms.sym(s).kind == SymKind::Val && !self.is_lazy_member(s) && !out.iter().any(|f| f.sym == s))
            .filter(|&(name, _)| file.methods.iter().any(|m| m.name == *name && m.access & (ACC_ABSTRACT | ACC_STATIC) == 0 && m.descriptor.starts_with("()")))
            .map(|(name, _)| name.clone())
            .collect();
        let (vars, holders) = self.class_file_only_members(b, &file, &members);
        ClassFileTrait { fields: out, unmatched, constants, vars, holders }
    }

    /// What a Scala 2 trait's class file declares that its transcoded pickle does not give the
    /// class a field for, each as scalac 3's `Mixin` implements it for a `Scala2x` trait
    /// (`traitInits`, 286-295, its private members' expanded names): a private var, whose
    /// abstract getter and setter `T$$x` and `T$$x_$eq` the trait's code calls; a private lazy
    /// val (not a private def: the pickle's `LAZY` flag), whose default getter `T$$x` computes it
    /// anew and whose static `T$$x$` the class's holder calls once; an object, whose abstract
    /// getter returns its module class `T$x$`, made once with the instance as its outer reference
    /// (the static `x$` a Scala 3 trait has instead is `class_file_trait_fields`'). A trait of a
    /// Scala 2 jar by its pickle, a nested one's its owner's (`Cx::scala2_pickles`).
    fn class_file_only_members(&mut self, b: ClassId, file: &crate::classfile::ClassFile, members: &FxMap<String, SymId>) -> (Vec<(String, ErasedType)>, Vec<(String, ErasedType, HolderInit)>) {
        let mut vars = Vec::new();
        let mut holders = Vec::new();
        let Some(raw) = self.cx.scala2_pickles.get(&b).cloned() else { return (vars, holders) };
        let binary = self.class_name(b);
        let expanded = format!("{}$$", binary.replace('/', "$"));
        // A private lazy val's default getter and static `T$$x$` are a private def's too: the
        // pickle's `LAZY` flag tells them apart, on the members of the trait's symbol, the class
        // symbol whose owners' names make its binary name (`Pickle::class_named`).
        let simple = binary.rsplit('/').next().unwrap_or(&binary).to_string();
        let lazies: Vec<String> = crate::scala2::pickle::Pickle::parse(crate::scala2::pickle::decode_signature((*raw).clone())).ok().map_or(Vec::new(), |p| {
            use crate::scala2::pickle as pk;
            let Some(t) = p.class_named(&simple) else { return Vec::new() };
            (0..p.len() as u32)
                .filter_map(|i| p.local_sym(i))
                .filter(|x| x.flags & pk::flags::LAZY != 0 && x.owner == t)
                .map(|x| format!("{}{}", expanded, p.name(x.name).trim_end()))
                .collect()
        });
        let declares = |name: &str, desc: &str, abstract_: bool| {
            file.methods.iter().any(|m| m.name == name && m.descriptor == desc && (m.access & ACC_ABSTRACT != 0) == abstract_ && m.access & ACC_STATIC == 0)
        };
        let this_desc = format!("(L{};)", binary);
        for m in &file.methods {
            let (params, ret) = parse_method_desc(&m.descriptor);
            if m.access & ACC_STATIC != 0 {
                let Some(x) = m.name.strip_suffix('$') else { continue };
                if !x.starts_with(&expanded) || members.contains_key(x) || !m.descriptor.starts_with(&this_desc) || ret == ErasedType::V || !lazies.iter().any(|l| l == x) {
                    continue;
                }
                if declares(x, &method_desc(&[], &ret), false) {
                    holders.push((x.to_string(), ret, HolderInit::TraitStatic(binary.clone())));
                }
            } else if m.access & ACC_ABSTRACT != 0 {
                if let Some(x) = m.name.strip_suffix("_$eq") {
                    let [ty] = params.as_slice() else { continue };
                    if x.starts_with(&expanded) && !members.contains_key(x) && ret == ErasedType::V && declares(x, &method_desc(&[], ty), true) {
                        vars.push((x.to_string(), ty.clone()));
                    }
                    continue;
                }
                let ErasedType::L(class) = &ret else { continue };
                let simple = m.name.strip_prefix(&expanded).unwrap_or(&m.name);
                let module = format!("{}${}$", binary, simple);
                let static_init = file.methods.iter().any(|s| s.access & ACC_STATIC != 0 && s.name == format!("{}$", m.name));
                if params.is_empty() && **class == *module && !static_init {
                    holders.push((m.name.clone(), ret.clone(), HolderInit::Module { class: class.clone(), outer: binary.clone() }));
                }
            }
        }
        (vars, holders)
    }

    /// A given with type parameters or a parameter clause: scalac's def (`Parsers.givenDef`,
    /// 4580-4588; `Desugar` 1093 for a given class), computed on every call where a
    /// parameterless one is a lazy val, so a trait's gives the class mixing it in no field.
    pub(super) fn is_given_def(&self, s: SymId) -> bool {
        let info = self.cx.input.syms.sym(s);
        info.kind == SymKind::Given && info.sig.as_ref().map_or(false, |sig| !sig.tparams.is_empty() || !sig.clauses.is_empty())
    }

    /// What a class mixing in a trait read from a class file holds from the class file alone,
    /// after `trait_setters` (`ClassFileTrait`): the setter of a val the TASTy's members do not
    /// name, a Scala 2 trait's private val, which the transcoder leaves out
    /// (`scala2::transcode`), or a val's `@targetName`: a field of the descriptor's type, the
    /// getter where the class has none, and the setter storing into the field, nothing where the
    /// class's own getter overrides the val; a Scala 2 trait's private var, a field with its
    /// getter and setter; its private lazy val and its object, each a lazy holder. The trait's
    /// `$init$` stores their initial values (`unscheduled_trait_inits`).
    fn class_file_storage(&mut self, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let inherited = self.traits_implemented_above(c);
        let bases: Vec<ClassId> = syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        for b in bases {
            if syms.class(b).kind != ClassKind::Trait || inherited.contains(&b) || cx.tclass_of[b.idx()] != u32::MAX {
                continue;
            }
            let read = self.class_file_trait_fields(b);
            let setter_prefix = format!("{}$_setter_$", self.class_name(b).replace('/', "$"));
            for (setter, desc) in &read.unmatched {
                let Some(field) = setter.strip_prefix(&setter_prefix).and_then(|x| x.strip_suffix("_$eq")) else { continue };
                let (params, ret) = parse_method_desc(desc);
                if params.len() != 1 || ret != ErasedType::V || self.plan.has_method(setter, desc) {
                    continue;
                }
                let ty = params[0].clone();
                let overridden = self.plan.has_method(field, &method_desc(&[], &ty));
                if !overridden {
                    self.plain_field(field, &ty);
                }
                self.begin_method(MethodFlags::PUBLIC, setter, desc, false, &[(None, ty.clone())], ErasedType::V, &[]);
                self.body(|g| {
                    if overridden {
                        let unit = g.lir.unit();
                        g.lir.ret(unit)
                    } else {
                        let given = g.param(0);
                        g.setter_body(field, given)
                    }
                });
                self.end_method();
            }
            for (field, ty) in &read.vars {
                let setter_desc = method_desc(&[ty.clone()], &ErasedType::V);
                if self.plan.has_method(field, &method_desc(&[], ty)) || self.plan.has_method(&format!("{}_$eq", field), &setter_desc) {
                    continue;
                }
                self.plain_field(field, ty);
                self.begin_method(MethodFlags::PUBLIC, &format!("{}_$eq", field), &setter_desc, false, &[(None, ty.clone())], ErasedType::V, &[]);
                self.body(|g| {
                    let given = g.param(0);
                    g.setter_body(field, given)
                });
                self.end_method();
            }
            // Each a thread-safe lazy val of the class, as `Mixin`'s forwarder of the member is one.
            for (name, ty, init) in &read.holders {
                let getter_desc = method_desc(&[], ty);
                if self.plan.has_method(name, &getter_desc) {
                    continue;
                }
                self.thread_safe_member(name, ty, init.clone(), MethodFlags::PUBLIC);
            }
        }
    }

    /// A private field of the class being written and its public getter.
    fn plain_field(&mut self, field: &str, ty: &ErasedType) {
        self.field(FieldFlags::PRIVATE, field, ty);
        self.begin_method(MethodFlags::PUBLIC, field, &method_desc(&[], ty), false, &[], ty.clone(), &[]);
        self.body(|g| g.getter_body(field, ty));
        self.end_method();
    }

    /// A getter's body: the class's field returned.
    fn getter_body(&mut self, field: &str, ty: &ErasedType) -> LirId {
        let owner = self.this_name.clone();
        let instance = self.lir.this(ErasedType::L(owner.clone()));
        let value = self.lir.field(instance, &owner, field, ty.clone());
        self.lir.ret(value)
    }

    /// A setter's body: the class's field set to `value`, nothing returned.
    fn setter_body(&mut self, field: &str, value: LirId) -> LirId {
        let owner = self.this_name.clone();
        let instance = self.lir.this(ErasedType::L(owner.clone()));
        let stored = self.lir.store(instance, &owner, field, value);
        let unit = self.lir.unit();
        let done = self.lir.seq(&[stored], unit);
        self.lir.ret(done)
    }

    /// The members a trait, an abstract class or an enum declares without defining them, those
    /// the output needs (`Reach::declared`): a call through the type resolves against the
    /// declaration, and link and product output keep them for later callers.
    fn abstract_members(&mut self, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let info = syms.class(c);
        let members: Vec<SymId> = info.member_order.iter().chain(info.extensions.iter()).copied().collect();
        for s in members {
            let sinfo = syms.sym(s);
            // A concrete var's setter is the var's `x_$eq`, written with its field or, in a trait,
            // with its declarations.
            if sinfo.intrinsic.is_some() || matches!(sinfo.kind, SymKind::Object(_) | SymKind::EnumValue(_) | SymKind::Param) || crate::typer::setters::is_concrete_setter(syms, s) {
                continue;
            }
            let f = cx.fun_of_sym[s.idx()];
            if f != u32::MAX && cx.input.prog.funs[f as usize].body.is_some() {
                continue;
            }
            if sinfo.sig.is_none() || !cx.input.reach.declared.get(s.idx()).copied().unwrap_or(false) {
                continue;
            }
            let m = self.mref(s);
            if !self.plan.has_method(&m.name, &m.desc) {
                self.abstract_method(MethodFlags::ABSTRACT, &m.name, &m.desc);
            }
        }
    }

    /// The bridges of the constants of the traits the class is the first to mix in. A trait's
    /// constant is the trait's own default getter, of no field of the class (`class_fields`), and
    /// the members it overrides under another erased signature are bridged in the class, as the
    /// class's own val's are: dotty's `Bridges` makes no exception for a constant.
    fn trait_constant_bridges(&mut self, tc: &TClass) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let inherited = self.traits_implemented_above(tc.id);
        let bases: Vec<ClassId> = syms.class(tc.id).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        for b in bases {
            let idx = cx.tclass_of[b.idx()];
            if syms.class(b).kind != ClassKind::Trait || inherited.contains(&b) || idx == u32::MAX {
                continue;
            }
            for init in &cx.input.prog.classes[idx as usize].init {
                let TInit::Field(s, e) = *init else { continue };
                // a constant val's getter: an inline val's is left to its own lowering
                if syms.sym(s).mods & mods::INLINE != 0 || self.constant(s, e).is_none() || syms.sym(s).sig.is_none() {
                    continue;
                }
                let ret = self.sym_type(s);
                let getter = MethodRef {
                    owner: self.this_name.clone(),
                    owner_is_interface: false,
                    name: self.field_name(s),
                    params: Vec::new(),
                    desc: method_desc(&[], &ret),
                    ret,
                    kind: Invoke::Virtual,
                };
                self.emit_bridges(s, &getter);
            }
        }
    }

    /// Inherited alternatives of an overloaded name that implement an inherited declaration
    /// with another erased signature (`TClass::bridges`).
    fn emit_inherited_bridges(&mut self, tc: &TClass) {
        let mut inherited: Option<Vec<(String, String)>> = None;
        for &(declared, implementation) in &tc.bridges {
            let bm = self.mref(declared);
            let m = self.mref(implementation);
            if bm.desc == m.desc || bm.params.len() != m.params.len() || self.plan.has_method(&bm.name, &bm.desc) {
                continue;
            }
            // A bridge an inherited pair made (`pair_bridges`' rule): not over a method with a body of that
            // descriptor the class inherits.
            if tc.inherited_pairs().iter().any(|p| p.overriding == implementation && p.overridden == declared) {
                if inherited.is_none() {
                    inherited = Some(self.inherited_methods(tc.id, true));
                }
                if inherited.as_ref().is_some_and(|ms| ms.iter().any(|(n, d)| *n == *bm.name && *d == *bm.desc)) {
                    continue;
                }
            }
            self.bridge(implementation, &bm, &m);
        }
    }

    /// The bridges of the class's inherited pairs (`TClass::inherited_pairs`, dotty's `Bridges.add`
    /// over its `BridgesCursor`): a trait's method implementing another trait's declaration whose
    /// erasure differs is answered under the declaration's descriptor by a call of the method.
    /// Where the superclass has the pair (`bridged`) the bridge is its; where the method's trait
    /// derives from the declaration's owner the pair is that trait's own, which a trait this build
    /// makes bridges as a default method (`emit_bridges_as`) and a trait read from class files
    /// bridges where its class file declares the descriptor with a body (teq's products do;
    /// scalac's traits declare no bridge, its classes do).
    fn pair_bridges(&mut self, tc: &TClass) {
        let cx = self.cx;
        let syms = cx.input.syms;
        // In the order the link mode's guesses meet them (`jar_bridges`): by the declaring base's place in the
        // linearisation, then the method's place in its class file (or among its members, a class of this build).
        let bases = &syms.class(tc.id).base_types;
        let mut pairs: Vec<(usize, usize, crate::tir::ClassPair)> = Vec::new();
        for &p in tc.inherited_pairs() {
            let Owner::Class(dk) = syms.sym(p.overridden).owner else { continue };
            let at = bases.iter().position(|&(b, _)| b == dk).unwrap_or(usize::MAX);
            let place = match cx.class_files.get(&dk) {
                Some(f) => {
                    let r = self.mref(p.overridden);
                    f.methods.iter().position(|x| x.name == r.name && x.descriptor == r.desc).unwrap_or(usize::MAX)
                }
                None => syms.class(dk).member_order.iter().position(|&m| m == p.overridden).unwrap_or(usize::MAX),
            };
            pairs.push((at, place, p));
        }
        pairs.sort_by_key(|&(at, place, _)| (at, place));
        let mut inherited: Option<Vec<(String, String)>> = None;
        for (_, _, p) in pairs {
            let (o, d) = (p.overriding, p.overridden);
            if p.bridged || syms.sym(o).kind != SymKind::Def || syms.sym(d).kind != SymKind::Def || syms.sym(o).is_extension {
                continue;
            }
            let bm = self.mref(d);
            let m = self.mref(o);
            if bm.name != m.name || bm.desc == m.desc || bm.params.len() != m.params.len() || self.plan.has_method(&bm.name, &bm.desc) {
                continue;
            }
            // Not over a method with a body of that descriptor the class inherits (a trait's own `f(Object)`
            // beside its `f(Int)`): the class answers the descriptor by it.
            if inherited.is_none() {
                inherited = Some(self.inherited_methods(tc.id, true));
            }
            if inherited.as_ref().is_some_and(|ms| ms.iter().any(|(n, dsc)| *n == *bm.name && *dsc == *bm.desc)) {
                continue;
            }
            if let (Owner::Class(k), Owner::Class(dk)) = (syms.sym(o).owner, syms.sym(d).owner) {
                let own_pair = syms.class(k).base_types.iter().any(|&(b, _)| b == dk);
                let bridged_there = match cx.class_files.get(&k) {
                    Some(f) => f.methods.iter().any(|x| x.name == bm.name && x.descriptor == bm.desc && x.access & ACC_ABSTRACT == 0),
                    None => cx.tclass_of[k.idx()] != u32::MAX,
                };
                if own_pair && bridged_there {
                    continue;
                }
            }
            self.pair_bridged.push((bm.name.to_string(), bm.desc.to_string()));
            self.bridge(o, &bm, &m);
        }
    }

    /// In link mode, the bridges scalac's mixin phase puts in a class for the jar traits it
    /// extends: a jar method left abstract under a descriptor nothing the class inherits
    /// implements, where a method of its name and arity does, forwards to that method
    /// (`PartialOrdering.tryCompare` returning `Option` beside `Ordering`'s default returning
    /// `Some`, `IterableOnceOps.filter` returning `Object` beside `Iterator`'s).
    fn jar_bridges(&mut self, c: ClassId, pairs: &[crate::tir::ClassPair]) {
        let cx = self.cx;
        if self.is_interface || self.abstract_class {
            return;
        }
        let syms = cx.input.syms;
        let files: Vec<&'a crate::classfile::ClassFile> = syms.class(c).base_types.iter().skip(1).filter_map(|&(b, _)| cx.class_files.get(&b).map(|f| &**f)).collect();
        if files.is_empty() {
            return;
        }
        let usable = |m: &crate::classfile::Method| m.access & (ACC_STATIC | ACC_PRIVATE) == 0 && !m.name.starts_with('<');
        let mut implemented: Vec<(String, String)> = Vec::new();
        for f in &files {
            for m in f.methods.iter().filter(|m| usable(m) && m.access & ACC_ABSTRACT == 0) {
                implemented.push((m.name.clone(), m.descriptor.clone()));
            }
        }
        for &(b, _) in syms.class(c).base_types.iter() {
            if cx.class_files.contains_key(&b) || cx.tclass_of[b.idx()] == u32::MAX {
                continue;
            }
            let own: Vec<SymId> = cx.members_in_order(b).into_iter().filter(|&m| syms.sym(m).owner == Owner::Class(b) && syms.sym(m).kind == SymKind::Def).collect();
            for m in own {
                let r = self.mref(m);
                implemented.push((r.name.clone(), r.desc.clone()));
            }
        }
        // The class's inherited pairs whose declaration is a jar's: the method that implements it is the pair's, not
        // a guess by name and arity (`f(Object)` of `T[Int]` by `f(Int)`, never `f(String)` beside it).
        let mut answered: Vec<(String, String, String)> = Vec::new();
        for p in pairs {
            if syms.sym(p.overriding).kind == SymKind::Def && syms.sym(p.overridden).kind == SymKind::Def {
                let (r, o) = (self.mref(p.overridden), self.mref(p.overriding));
                // A target of the declaration's own descriptor is no bridge's (an inline def a library's class
                // file keeps no body of): the guess stands, as on master.
                if o.name == r.name && o.params.len() == r.params.len() && o.desc != r.desc {
                    answered.push((r.name.to_string(), r.desc.to_string(), o.desc.to_string()));
                }
            }
        }
        let mut abstract_ones: Vec<(String, String)> = Vec::new();
        for f in &files {
            for m in f.methods.iter().filter(|m| usable(m) && m.access & ACC_ABSTRACT != 0) {
                let key = (m.name.clone(), m.descriptor.clone());
                if !abstract_ones.contains(&key) && !implemented.contains(&key) {
                    abstract_ones.push(key);
                }
            }
        }
        for (name, desc) in abstract_ones {
            if self.plan.has_method(&name, &desc) {
                continue;
            }
            let (params, _) = parse_method_desc(&desc);
            if let Some((_, _, target_desc)) = answered.iter().find(|(n, d, _)| *n == name && *d == desc) {
                let target_desc = target_desc.clone();
                self.self_bridge(&name, &desc, &target_desc);
                continue;
            }
            let target = implemented
                .iter()
                .filter(|(n, d)| *n == name && parse_method_desc(d).0.len() == params.len())
                .max_by_key(|(_, d)| parse_method_desc(d).0.iter().zip(&params).filter(|(a, b)| a == b).count())
                .cloned();
            let Some((_, target_desc)) = target else { continue };
            self.self_bridge(&name, &desc, &target_desc);
        }
    }

    /// The members of a parent trait that nested objects of the class implement: each an
    /// accessor answering the object's module (`object t extends T` for `val t: T`).
    fn implemented_nested_objects(&mut self, tc: &TClass) {
        for (declared, nested) in crate::emit::implemented_nested_objects(self.cx.input.syms, tc.id) {
            let m = self.mref(declared);
            self.begin_method(MethodFlags::PUBLIC, &m.name, &m.desc, false, &[], m.ret.clone(), &[]);
            self.body(|g| {
                let module = g.lower_module(nested);
                let t = g.lir.ty(module).clone();
                let value = g.lower_adapt(module, &t, &m.ret);
                g.lir.ret(value)
            });
            self.end_method();
        }
    }

    /// What a class takes from its traits where the JVM's resolution of default methods does not
    /// give what Scala's linearisation gives: a forwarder to the winning trait when two
    /// unrelated traits define the member, and a bridge when a trait declares it under another
    /// erased signature than the one that implements it.
    fn mixin_members(&mut self, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let info = syms.class(c);
        let bases: Vec<ClassId> = info.base_types.iter().skip(1).map(|&(b, _)| b).filter(|&b| syms.class(b).kind == ClassKind::Trait).collect();
        if bases.len() < 2 {
            return;
        }
        let has_body = |s: SymId| implemented(cx, s);
        let mut by_name: FxMap<(crate::intern::Name, bool), Vec<(ClassId, SymId)>> = FxMap::default();
        let mut order: Vec<(crate::intern::Name, bool)> = Vec::new();
        for &b in &bases {
            let binfo = syms.class(b);
            for &s in binfo.member_order.iter().chain(binfo.extensions.iter()) {
                let sinfo = syms.sym(s);
                if sinfo.kind != SymKind::Def || sinfo.intrinsic.is_some() || sinfo.sig.is_none() {
                    continue;
                }
                let key = (sinfo.name, sinfo.is_extension);
                let entry = by_name.entry(key).or_default();
                if entry.is_empty() {
                    order.push(key);
                }
                entry.push((b, s));
            }
        }
        for key in order {
            let decls = &by_name[&key];
            if decls.len() < 2 {
                continue;
            }
            let own = if key.1 {
                info.extensions.iter().any(|&e| syms.sym(e).name == key.0)
            } else {
                defines(cx, &info, key.0)
            };
            if own {
                continue;
            }
            let Some(&(ib, isym)) = decls.iter().find(|&&(_, s)| has_body(s)) else { continue };
            if !key.1 && redeclared_away(cx, &info, isym, ib) {
                continue;
            }
            let im = self.mref(isym);
            let winner_bases: Vec<ClassId> = syms.class(ib).base_types.iter().map(|&(b, _)| b).collect();
            for &(b, s) in decls {
                if s == isym || winner_bases.contains(&b) {
                    continue;
                }
                let m = self.mref(s);
                if m.name != im.name || m.params.len() != im.params.len() {
                    continue;
                }
                // The losing trait's bridge defines the winner's erased signature as a default
                // too, which the class has to decide.
                if m.desc != im.desc && has_body(s) {
                    self.mixin_forwarder(&im, &im);
                }
                if m.desc == im.desc && !has_body(s) {
                    continue;
                }
                self.mixin_forwarder(&m, &im);
            }
        }
    }

    /// `m` in the class, calling the trait's default `im`.
    fn mixin_forwarder(&mut self, m: &MethodRef, im: &MethodRef) {
        // Of another erased signature than the default's, the method is erasure's bridge.
        self.default_forwarder(m, im, MethodFlags { synthetic: true, bridge: m.desc != im.desc, ..MethodFlags::PUBLIC });
    }

    /// The class's method `m` of the access `access`, calling the trait's default `im` by
    /// `invokespecial`, every forwarder of a trait's member the class writes: each parameter
    /// loaded from its slot, the slots following the types' widths (dotty's `locals.makeLocal`),
    /// and adapted to `im`'s, the result to `m`'s. Nothing where the class has `m` already.
    fn default_forwarder(&mut self, m: &MethodRef, im: &MethodRef, flags: MethodFlags) {
        if self.plan.has_method(&m.name, &m.desc) {
            return;
        }
        let params: Vec<(Option<SymId>, ErasedType)> = m.params.iter().map(|t| (None, t.clone())).collect();
        self.begin_method(flags, &m.name, &m.desc, false, &params, m.ret.clone(), &[]);
        let default = Rc::new(MethodRef { kind: Invoke::Special, owner_is_interface: true, ..im.clone() });
        self.body(|g| g.forwarder_body(&default, &m.ret));
        self.end_method();
    }

    /// A forwarder's body: `target` called on the instance over the method's parameters, each
    /// adapted to the type the target takes at its place, the result adapted to `ret`.
    fn forwarder_body(&mut self, target: &Rc<MethodRef>, ret: &ErasedType) -> LirId {
        let instance = self.lower_this().expect("a forwarder runs on an instance");
        let declared: Vec<ErasedType> = self.m.params.iter().map(|p| p.1.clone()).collect();
        let mut args = Vec::with_capacity(target.params.len());
        for (i, (from, to)) in declared.iter().zip(&target.params).enumerate() {
            let given = self.param(i);
            args.push(self.lower_adapt(given, from, to));
        }
        let value = self.lir.call(target, instance, &args);
        let value = self.lower_adapt(value, &target.ret, ret);
        self.lir.ret(value)
    }

    /// The class's bridge `name` `desc` to its own method of the descriptor `target`, called
    /// virtually (dotty's `Bridges`): each parameter loaded from its slot by the types' widths and
    /// adapted to the target's, the result adapted back.
    fn self_bridge(&mut self, name: &str, desc: &str, target: &str) {
        let (params, ret) = parse_method_desc(desc);
        let (target_params, target_ret) = parse_method_desc(target);
        let locals: Vec<(Option<SymId>, ErasedType)> = params.iter().map(|t| (None, t.clone())).collect();
        self.begin_method(MethodFlags { synthetic: true, bridge: true, ..MethodFlags::PUBLIC }, name, desc, false, &locals, ret.clone(), &[]);
        let own = method_ref(Invoke::Virtual, &self.this_name, false, name, &target_params, &target_ret);
        self.body(|g| g.forwarder_body(&own, &ret));
        self.end_method();
    }

    /// A method of a superclass wins over the default method of an interface, where Scala takes
    /// the one of the trait that the class adds: the class gets a forwarder to that default.
    fn superclass_forwarders(&mut self, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let info = syms.class(c);
        let Some(parent) = info.superclass else { return };
        let inherited: Vec<ClassId> = syms.class(parent).base_types.iter().map(|&(b, _)| b).collect();
        let mut done: Vec<crate::intern::Name> = Vec::new();
        for &(b, _) in info.base_types.iter().skip(1) {
            if inherited.contains(&b) || syms.class(b).kind != ClassKind::Trait {
                continue;
            }
            for &s in &syms.class(b).member_order {
                let sinfo = syms.sym(s);
                // A product's getter is its method's, as the whole program's class has it
                // (`getter_forwarders`).
                if syms.product_synthetics.contains_key(&s) {
                    continue;
                }
                if sinfo.kind != SymKind::Def || !implemented(cx, s) || done.contains(&sinfo.name) || defines(cx, &info, sinfo.name) {
                    continue;
                }
                if !inherited.iter().any(|&k| syms.class(k).members.contains_key(&sinfo.name)) || redeclared_away(cx, &info, s, b) {
                    continue;
                }
                done.push(sinfo.name);
                let m = self.mref(s);
                if !self.plan.has_method(&m.name, &m.desc) {
                    let params: Vec<(Option<SymId>, ErasedType)> = m.params.iter().map(|t| (None, t.clone())).collect();
                    self.begin_method(MethodFlags { synthetic: true, ..MethodFlags::PUBLIC }, &m.name, &m.desc, false, &params, m.ret.clone(), &[]);
                    let default = Rc::new(MethodRef { kind: Invoke::Special, owner_is_interface: true, ..(*m).clone() });
                    self.body(|g| g.forwarder_body(&default, &m.ret));
                    self.end_method();
                }
                self.getter_forwarders(s, &inherited);
            }
        }
    }

    /// The getters of the defaults the trait member `s` declares where a superclass's member of
    /// its name has a getter of the same parameter: a getter is a member
    /// (`Desugar.addDefaultGetters`), the trait's winning in Scala's linearisation where the
    /// JVM's resolution takes the superclass's, so the class forwards to it.
    fn getter_forwarders(&mut self, s: SymId, inherited: &[ClassId]) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let Some(sig) = syms.sym(s).sig.clone() else { return };
        let name = syms.sym(s).name;
        let declares = |m: SymId, i: usize| syms.sym(m).sig.as_ref().and_then(|sig| sig.clauses.iter().flat_map(|c| c.params.iter()).nth(i).map(|p| p.has_default)).unwrap_or(false);
        let n: usize = sig.clauses.iter().map(|c| c.params.len()).sum();
        for i in 0..n {
            if !declares(s, i) {
                continue;
            }
            let theirs = inherited.iter().any(|&k| {
                syms.class(k).members.get(&name).is_some_and(|&m| match syms.alternatives(m) {
                    Some(alts) => alts.iter().any(|&a| declares(a, i)),
                    None => declares(m, i),
                })
            });
            if !theirs {
                continue;
            }
            let m = self.mref(s);
            let shape = self.default_getter_shape(s, i);
            let (getter, desc) = self.default_getter_ref(s, i, &m.params[..shape.before]);
            if self.plan.has_method(&getter, &desc) {
                continue;
            }
            let (params, ret) = parse_method_desc(&desc);
            let taken: Vec<(Option<SymId>, ErasedType)> = params.iter().map(|t| (None, t.clone())).collect();
            self.begin_method(MethodFlags { synthetic: true, ..MethodFlags::PUBLIC }, &getter, &desc, false, &taken, ret.clone(), &[]);
            let default = method_ref(Invoke::Special, &m.owner, true, &getter, &params, &ret);
            self.body(|g| g.forwarder_body(&default, &ret));
            self.end_method();
        }
    }

    /// The bridges scalac puts in a class for an abstract member of a trait it newly mixes in
    /// that its superclass, which knows nothing of the trait, implements under another erased
    /// descriptor: with `trait T[A] { var x: A }` and `class Base { var x = 0 }`, `class C extends
    /// Base with T[Int]` forwards `x()Object` and `x_$eq(Object)V` to `x()I` and `x_$eq(I)V`.
    fn superclass_bridges(&mut self, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let info = syms.class(c);
        let Some(parent) = info.superclass else { return };
        let inherited: Vec<ClassId> = syms.class(parent).base_types.iter().map(|&(b, _)| b).collect();
        let added: Vec<ClassId> = info.base_types.iter().map(|&(b, _)| b).filter(|b| !inherited.contains(b)).collect();
        if !added.iter().any(|&b| syms.class(b).kind == ClassKind::Trait) {
            return;
        }
        // A given is a val or a def as scalac reads it (`Parsers.givenDef`), which bridges as
        // they do (`Bridges`).
        let concrete = |s: SymId| {
            let sinfo = syms.sym(s);
            matches!(sinfo.kind, SymKind::Def | SymKind::Val | SymKind::Var | SymKind::Given) && sinfo.sig.is_some() && !cx.input.reach.is_abstract(syms, s)
        };
        let mut implemented: Vec<Rc<MethodRef>> = Vec::new();
        for &k in &inherited {
            // A class read from a class file implements what the file defines, whichever of its
            // members' signatures the build has read (`jar_bridges` reads them so too).
            if let Some(cf) = cx.class_files.get(&k) {
                for m in cf.methods.iter().filter(|m| m.access & (ACC_ABSTRACT | ACC_STATIC | ACC_PRIVATE | ACC_BRIDGE) == 0 && !m.name.starts_with('<')) {
                    let (params, ret) = parse_method_desc(&m.descriptor);
                    let owner: Rc<str> = Rc::from(cf.name.as_str());
                    implemented.push(Rc::new(MethodRef { owner, owner_is_interface: cf.is_interface(), name: m.name.clone(), params, ret, desc: m.descriptor.clone(), kind: Invoke::Virtual }));
                }
                continue;
            }
            for &s in &syms.class(k).member_order {
                if syms.sym(s).owner != Owner::Class(k) || !concrete(s) {
                    continue;
                }
                let m = self.mref(s);
                if syms.sym(s).kind == SymKind::Var {
                    let desc = method_desc(&[m.ret.clone()], &ErasedType::V);
                    implemented.push(Rc::new(MethodRef { name: format!("{}_$eq", m.name), params: vec![m.ret.clone()], ret: ErasedType::V, desc, ..(*m).clone() }));
                }
                implemented.push(m);
            }
        }
        for &t in added.iter().filter(|&&b| syms.class(b).kind == ClassKind::Trait) {
            for &s in &syms.class(t).member_order {
                let sinfo = syms.sym(s);
                let declared_abstract = sinfo.owner == Owner::Class(t)
                    && matches!(sinfo.kind, SymKind::Def | SymKind::Var | SymKind::Given)
                    && sinfo.sig.is_some()
                    && cx.input.reach.declared.get(s.idx()).copied().unwrap_or(false)
                    && cx.input.reach.is_abstract(syms, s);
                // Implemented by the class or a trait it adds: no member of the superclass's.
                let added_defines = || added.iter().any(|&b| syms.class(b).members.get(&sinfo.name).map_or(false, |&d| syms.sym(d).owner == Owner::Class(b) && concrete(d)));
                if !declared_abstract || added_defines() {
                    continue;
                }
                let declared = self.mref(s);
                if self.plan.has_method(&declared.name, &declared.desc) || implemented.iter().any(|m| m.name == declared.name && m.desc == declared.desc) {
                    continue;
                }
                let Some(target) = implemented.iter().find(|m| m.name == declared.name && m.params.len() == declared.params.len()).cloned() else { continue };
                self.self_bridge(&declared.name, &declared.desc, &target.desc);
            }
        }
    }

    /// Which members of `Object`'s names, `toString`, `hashCode`, `equals`, `clone` and
    /// `finalize`, need a mixin forwarder in the class, as dotty's `Mixin.mixinForwarders` asks
    /// `MixinOps.needsMixinForwarder` of each declaration (Mixin.scala 314 to 318): each
    /// declaration of each trait the class mixes in that its superclass does not, every
    /// alternative of a name on its own (`clone(): Object` beside `clone[A](): List[A]`), where it
    /// competes with `Object`'s member of the name, which it overrides (`needsForwarder`: a
    /// method of a class, which the JVM would select over the trait's default, the protected
    /// `clone` and `finalize` failing the call), not private, an accessor or deferred, and is the
    /// class's member (`isInImplementingClass`). The other members of these names, which compete
    /// with no class's (`clone(x: Long, y: Int)`, a generic `clone(x: A)`), are the class's
    /// other paths' (`mixin_members`), as every other trait method's. Each forwarder is
    /// written as every forwarder of a trait's member (`mixin_forwarder_to`).
    fn object_method_forwarders(&mut self, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let lin: Vec<ClassId> = syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        let inherited = |t: ClassId| syms.class(c).superclass.is_some_and(|s| syms.class(s).base_types.iter().any(|&(b, _)| b == t));
        let mut names = vec![(n::TO_STRING, "()Ljava/lang/String;"), (n::HASH_CODE, "()I"), (n::EQUALS, "(Ljava/lang/Object;)Z"), (n::CLONE, "()Ljava/lang/Object;")];
        // No class of the program declares a member of a name never interned.
        if let Some(finalize) = cx.input.interner.lookup("finalize") {
            names.push((finalize, "()V"));
        }
        let mixins: Vec<ClassId> = lin.iter().skip(1).copied().filter(|&t| syms.class(t).kind == ClassKind::Trait && !inherited(t)).collect();
        for (name, object_desc) in names {
            // `Object`'s member of the name, the competing method of a class.
            let object = Decl { desc: object_desc.to_string(), poly: Some(0), sym: None, private: false, deferred: false };
            // Each base's declarations of the name, in the linearisation's order.
            let declared: Vec<(ClassId, Vec<Decl>)> = lin.iter().map(|&b| (b, self.declarations(b, name))).collect();
            for &mixin in &mixins {
                let own = declared.iter().find(|d| d.0 == mixin).map_or_else(Vec::new, |d| d.1.clone());
                for meth in own {
                    if object.matches(&meth) && self.needs_mixin_forwarder(mixin, &meth, &declared) {
                        self.mixin_forwarder_to(mixin, name, &meth, &declared, &object);
                    }
                }
            }
        }
    }

    /// Whether `c` needs a forwarder of `meth`, a declaration of its mixin `mixin` that competes
    /// with a class's method (`MixinOps.needsMixinForwarder`): a method, not private, an accessor
    /// or deferred, a sealed Java interface's only in a class it permits, and `c`'s member, no
    /// later declaration overriding it (`isInImplementingClass`, `Symbols::member_declaration`
    /// over the declarations it matches).
    fn needs_mixin_forwarder(&mut self, mixin: ClassId, meth: &Decl, declared: &[(ClassId, Vec<Decl>)]) -> bool {
        let cx = self.cx;
        let syms = cx.input.syms;
        if meth.private || meth.deferred || meth.sym.is_some_and(|s| syms.sym(s).kind != SymKind::Def) {
            return false;
        }
        if let Some(cf) = cx.class_files.get(&mixin) {
            if !cf.scala && !cf.permitted.is_empty() && !cf.permitted.iter().any(|p| **p == *self.this_name) {
                return false;
            }
        }
        let matching: Vec<(ClassId, bool)> =
            declared.iter().flat_map(|(b, ds)| ds.iter().filter(|d| !d.private && d.matches(meth)).map(move |d| (*b, !d.deferred))).collect();
        syms.member_declaration(&matching) == Some(mixin)
    }

    /// The forwarder of `meth`, a declaration of the trait `mixin`, written as every forwarder of
    /// a trait's member (`default_forwarder`, as `MixinOps.mkForwarderSym` makes it): the member's
    /// reference (`mref`, a loaded declaration's from its class file's descriptor where the
    /// program has no symbol of it), final where `meth` is (dotty's `javaFlags`), with the
    /// metadata of `meth` as its class file holds it; and a bridge of each member it overrides
    /// whose erased signature is another, in the linearisation's order, `Object`'s last
    /// (`Bridges.addBridgeIfNeeded`, Bridges.scala 88 to 93: the pairs that match before
    /// erasure), each as every bridge to the class's own method (`self_bridge`).
    fn mixin_forwarder_to(&mut self, mixin: ClassId, name: crate::intern::Name, meth: &Decl, declared: &[(ClassId, Vec<Decl>)], object: &Decl) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let text = cx.input.interner.get(name);
        let im = match meth.sym.map(|s| self.mref(s)).filter(|m| m.desc == meth.desc) {
            Some(m) => m,
            None => {
                let (params, ret) = parse_method_desc(&meth.desc);
                Rc::new(MethodRef { owner: self.class_name(mixin), owner_is_interface: true, name: text.to_string(), params, ret, desc: meth.desc.clone(), kind: Invoke::Interface })
            }
        };
        if self.plan.has_method(text, &im.desc) {
            return;
        }
        let fin = meth.sym.is_some_and(|s| syms.sym(s).mods & mods::FINAL != 0);
        self.default_forwarder(&im, &im, MethodFlags { is_final: fin, ..MethodFlags::PUBLIC });
        // And the selected method's metadata as its class file holds it, the exceptions it
        // declares and its parameters' annotations among them (`MixinOps.mkForwarderSym` copies
        // the member's and its parameters' annotations); a member teq compiles declares none
        // (`@throws` changes nothing, docs/COMPATIBILITY.md), alike whole and split.
        if let Some(m) = cx.class_files.get(&mixin).and_then(|cf| cf.methods.iter().find(|m| m.name == text && m.descriptor == im.desc && m.access & (ACC_STATIC | ACC_BRIDGE) == 0)) {
            self.cw.method_metadata(text, &im.desc, m);
        }
        let overridden = declared.iter().filter(|d| d.0 != mixin).flat_map(|d| d.1.iter()).filter(|d| !d.private && d.matches(meth)).chain(std::iter::once(object).filter(|o| o.matches(meth)));
        let overridden: Vec<String> = overridden.map(|d| d.desc.clone()).collect();
        let mut bridged: Vec<&str> = Vec::new();
        for overridden in &overridden {
            let overridden = overridden.as_str();
            if overridden == im.desc || bridged.contains(&overridden) || self.plan.has_method(text, overridden) {
                continue;
            }
            bridged.push(overridden);
            self.self_bridge(text, overridden, &im.desc);
        }
    }

    /// `b`'s declarations of `name`, each alternative on its own, with one record whichever reads
    /// it, so that the whole and the split build decide alike: a jar's class's methods of the
    /// name in its class file, not static, a bridge or synthetic, with the program's symbol of
    /// each by its descriptor; a class of the program's members of the name, by their erasure, a
    /// concrete one the program does not reach left out (it writes no body of it). A private
    /// method of a trait the products hold is written under its expanded name, which none of
    /// `Object`'s names is, as the source's carries its `private`.
    fn declarations(&mut self, b: ClassId, name: crate::intern::Name) -> Vec<Decl> {
        let cx = self.cx;
        let syms = cx.input.syms;
        let alternatives: Vec<SymId> = match syms.class(b).members.get(&name) {
            Some(&m) => match syms.alternatives(m) {
                Some(alts) => alts.to_vec(),
                None => vec![m],
            },
            None => Vec::new(),
        };
        let own: Vec<SymId> = alternatives.into_iter().filter(|&a| syms.sym(a).owner == Owner::Class(b)).collect();
        let poly = |s: SymId| syms.sym(s).sig.as_ref().map(|sig| sig.tparams.len());
        if let Some(cf) = cx.class_files.get(&b) {
            let text = cx.input.interner.get(name);
            let methods: Vec<&crate::classfile::Method> = cf.methods.iter().filter(|m| m.name == text && m.access & (ACC_STATIC | ACC_BRIDGE | ACC_SYNTHETIC) == 0).collect();
            return methods
                .into_iter()
                .map(|m| {
                    let sym = own.iter().copied().find(|&a| self.mref(a).desc == m.descriptor);
                    // Without the program's symbol, a generic signature that declares type
                    // parameters is a polymorphic method's.
                    let poly = sym.and_then(poly).or_else(|| Some(if m.signature.as_deref().is_some_and(|g| g.starts_with('<')) { 1 } else { 0 }));
                    Decl { desc: m.descriptor.clone(), poly, sym, private: m.access & ACC_PRIVATE != 0, deferred: m.access & ACC_ABSTRACT != 0 }
                })
                .collect();
        }
        let mut out = Vec::new();
        for a in own {
            let concrete = concrete_member(cx, a);
            let f = cx.fun_of_sym[a.idx()];
            if concrete && !(f != u32::MAX && cx.input.prog.funs[f as usize].body.is_some() && cx.input.reach.funs[f as usize]) {
                continue;
            }
            let desc = self.mref(a).desc.clone();
            out.push(Decl { desc, poly: poly(a), sym: Some(a), private: syms.sym(a).mods & mods::PRIVATE != 0, deferred: !concrete });
        }
        out
    }

    // ---- case classes, case objects and enum values ----

    /// The box's `hashCode` and `equals`, and a case class's product members, which scalac
    /// synthesizes for a value class (`SyntheticMembers.scala` 68, 97-125) and dotty's
    /// `ExtensionMethods` moves to the companion as `m$extension(u, ..)` (19-89): the box's
    /// method calls the companion's, or, without a companion this build writes, holds the body.
    fn value_class_members(&mut self, c: ClassId) {
        let syms = self.cx.input.syms;
        let Some(field) = syms.class(c).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym) else { return };
        let on_companion = self.cx.vc_on_companion(c);
        let owner = self.this_name.clone();
        let this = ErasedType::L(owner.clone());
        let (name, ty) = (self.field_name(field), self.sym_type(field));
        let u = self.vc_underlying(c);
        for (member, params, ret) in self.vc_synthesized(c) {
            let typed: Vec<(Option<SymId>, ErasedType)> = params.iter().map(|t| (None, t.clone())).collect();
            if on_companion {
                self.box_forwarder(c, member, &format!("{}$extension", member), &typed, &ret);
                continue;
            }
            self.begin_method(MethodFlags::PUBLIC, member, &method_desc(&params, &ret), false, &typed, ret.clone(), &[]);
            if vc_member_lowers(member, &u) {
                self.body(|g| {
                    let instance = g.lir.this(this.clone());
                    let read = g.lir.field(instance, &owner, &name, ty.clone());
                    let underlying = g.lower_adapt(read, &ty, &u);
                    let held = g.lir.temp();
                    let kept = g.lir.keep(held, underlying);
                    let answer = g.vc_member_tree(c, member, &u, Underlying::Kept(held), 0);
                    g.lir.seq(&[kept], answer)
                });
            } else {
                self.load(0, &this);
                self.getfield(&owner, &name, &ty);
                self.adapt(&ty, &u);
                let us = self.store_new(&u);
                self.vc_synthesized_body(c, member, &ret, &u, us, 1);
            }
            self.end_method();
        }
    }

    /// The members scalac synthesizes for value class `c` that the class does not define or
    /// inherit (`TClass::synthesized`), with their parameters and results, in the order of
    /// scalac's companion: `hashCode` and `equals`, and a case class's `toString` and product
    /// members.
    pub(super) fn vc_synthesized(&mut self, c: ClassId) -> Vec<(&'static str, Vec<ErasedType>, ErasedType)> {
        let cx = self.cx;
        let string = ErasedType::L(Rc::from(STRING));
        let object = ErasedType::object();
        let kind = if cx.is_case_class(c) { synthetic::CASE } else { synthetic::VALUE };
        let synthesized = cx.synthesized(c) & kind;
        let all = [
            (synthetic::HASH_CODE, "hashCode", vec![], ErasedType::I),
            (synthetic::EQUALS, "equals", vec![object.clone()], ErasedType::Z),
            (synthetic::TO_STRING, "toString", vec![], string.clone()),
            (synthetic::CAN_EQUAL, "canEqual", vec![object.clone()], ErasedType::Z),
            (synthetic::PRODUCT_ARITY, "productArity", vec![], ErasedType::I),
            (synthetic::PRODUCT_PREFIX, "productPrefix", vec![], string.clone()),
            (synthetic::PRODUCT_ELEMENT, "productElement", vec![ErasedType::I], object),
            (synthetic::PRODUCT_ELEMENT_NAME, "productElementName", vec![ErasedType::I], string),
        ];
        all.into_iter().filter(|&(bit, ..)| synthesized & bit != 0).map(|(_, name, params, ret)| (name, params, ret)).collect()
    }

    /// The tree of synthesized member `member` of value class `c` (`Gen::vc_synthesized_body`),
    /// over its underlying value `us` of type `u` and the member's parameter at the position
    /// `ps`: a primitive underlying value hashes as its own `hashCode`, a reference one through
    /// the null-safe `Objects.hashCode` (`SyntheticMembers.scala` 338-346); `equals` is a test of
    /// the class and `==` of the underlying values, primitive comparison for a primitive (`NaN`
    /// unequal, `-0.0` equal to `0.0`), then a `canEqual` the class defines, without the `eq` test
    /// of a case class (297-332).
    pub(super) fn vc_member_tree(&mut self, c: ClassId, member: &str, u: &ErasedType, us: Underlying, ps: usize) -> LirId {
        let cx = self.cx;
        let syms = cx.input.syms;
        let string = ErasedType::L(Rc::from(STRING));
        let object = ErasedType::object();
        let class = self.class_name(c);
        let boxed = ErasedType::L(class.clone());
        let answer = match member {
            "hashCode" => {
                let primitive = match u {
                    ErasedType::I => Some("java/lang/Integer"),
                    ErasedType::J => Some("java/lang/Long"),
                    ErasedType::D => Some("java/lang/Double"),
                    ErasedType::F => Some("java/lang/Float"),
                    ErasedType::Z => Some("java/lang/Boolean"),
                    ErasedType::C => Some("java/lang/Character"),
                    ErasedType::S => Some("java/lang/Short"),
                    ErasedType::B => Some("java/lang/Byte"),
                    _ => None,
                };
                let value = us.node(self, u);
                let (hash, value) = match primitive {
                    Some(b) => (method_ref(Invoke::Static, &Rc::from(b), false, "hashCode", std::slice::from_ref(u), &ErasedType::I), value),
                    None => (method_ref(Invoke::Static, &Rc::from("java/util/Objects"), false, "hashCode", &[object.clone()], &ErasedType::I), self.lower_adapt(value, u, &object)),
                };
                self.lir.static_call(&hash, &[value])
            }
            "equals" => {
                let no = self.lir.label();
                let mut tests = Vec::new();
                let given = self.param(ps);
                let same_class = self.lir.is(given, &class);
                let other_class = self.lir.not(same_class);
                let unequal = self.lir.leave(no);
                tests.push(self.lir.when(other_class, unequal));
                let mine = us.node(self, u);
                let given = self.param(ps);
                let cast = self.lir.checked(given, boxed.clone());
                let theirs = self.lower_vc_unbox(c, cast, u);
                let differs = self.lir.compare(Relation::Ne, Comparing::Numbers, mine, theirs);
                let unequal = self.lir.leave(no);
                tests.push(self.lir.when(differs, unequal));
                // A `canEqual` of the class's own is asked last, `that.canEqual(this)` (297-332).
                if cx.synthesized(c) & synthetic::EQUALS_CALLS_CAN_EQUAL != 0 {
                    let given = self.param(ps);
                    let other = self.lir.checked(given, boxed.clone());
                    let value = us.node(self, u);
                    let mine = self.lower_vc_box(c, value, u);
                    let mine = self.lower_adapt(mine, &boxed, &object);
                    let can_equal = method_ref(Invoke::Virtual, &class, false, "canEqual", &[object.clone()], &ErasedType::Z);
                    let can = self.lir.call(&can_equal, other, &[mine]);
                    let cannot = self.lir.not(can);
                    let unequal = self.lir.leave(no);
                    tests.push(self.lir.when(cannot, unequal));
                }
                let truth = self.lir.bool(true);
                let answer = self.lir.ret(truth);
                let refused = self.lir.seq(&tests, answer);
                let refused = self.lir.labeled(no, refused);
                let falsity = self.lir.bool(false);
                let answer = self.lir.ret(falsity);
                return self.lir.seq(&[refused], answer);
            }
            // `ScalaRunTime._toString` of the box, which reads its `productPrefix` and elements
            // through it.
            "toString" => {
                let to_string = method_ref(Invoke::Static, &Rc::from(SCALA_RUNTIME), false, "_toString", &[ErasedType::L(Rc::from(PRODUCT))], &string);
                let value = us.node(self, u);
                let made = self.lower_vc_box(c, value, u);
                self.lir.static_call(&to_string, &[made])
            }
            "canEqual" => {
                let given = self.param(ps);
                self.lir.is(given, &class)
            }
            "productArity" => self.lir.int(1),
            "productPrefix" => {
                let info = syms.class(c);
                let display: Rc<str> = Rc::from(info.product_name(cx.input.interner.get(info.name)));
                self.lir.text(&display, string)
            }
            _ => unreachable!("a member with a tree (`vc_member_lowers`)"),
        };
        self.lir.ret(answer)
    }

    /// The code of synthesized member `member` of value class `c` whose body is no tree
    /// (`vc_member_lowers`), returning `ret`, over the underlying value in local `us` of type `u`
    /// and the member's parameter in local `ps`: `equals` over a reference, and the elements.
    pub(super) fn vc_synthesized_body(&mut self, c: ClassId, member: &str, ret: &ErasedType, u: &ErasedType, us: u16, ps: u16) {
        self.outside(Unlowered::ValueClassSynthesized);
        let cx = self.cx;
        let syms = cx.input.syms;
        let string = ErasedType::L(Rc::from(STRING));
        let object = ErasedType::object();
        let class = self.class_name(c);
        match member {
            "equals" => {
                let no = self.code.new_label();
                self.load(ps, &object);
                self.instance_of(&class);
                self.jump_if(op::IFEQ, 1, no);
                self.load(us, u);
                self.load(ps, &object);
                self.checkcast(&class);
                self.vc_unbox(c, u);
                // The other's underlying value held while this one is adapted under it.
                self.adapt(u, &object);
                let that = self.store_new(&object);
                self.adapt(u, &object);
                self.load(that, &object);
                self.helper_call("equal", 2);
                self.jump_if(op::IFEQ, 1, no);
                // A `canEqual` of the class's own is asked last, `that.canEqual(this)` (297-332).
                if self.cx.synthesized(c) & synthetic::EQUALS_CALLS_CAN_EQUAL != 0 {
                    self.load(ps, &object);
                    self.checkcast(&class);
                    let underlying = self.slot_value(us, u);
                    let made = self.lowering(|g| g.lower_vc_box(c, underlying, u));
                    self.write(made);
                    let boxed = ErasedType::L(class.clone());
                    self.adapt(&boxed, &object);
                    self.invoke(Invoke::Virtual, &class, false, "canEqual", &[object.clone()], &ErasedType::Z);
                    self.jump_if(op::IFEQ, 1, no);
                }
                self.iconst(1);
                self.return_value(ret);
                self.code.bind(no);
                self.iconst(0);
                self.return_value(ret);
            }
            "productElement" | "productElementName" => {
                // `productElement` and `productElementName`: the one element, else
                // `IndexOutOfBoundsException`.
                let fail = self.code.new_label();
                self.load(ps, &ErasedType::I);
                self.jump_if(op::IFNE, 1, fail);
                if member == "productElement" {
                    self.load(us, u);
                    self.adapt(u, &object);
                } else {
                    let field = syms.class(c).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym);
                    let name = field.map_or(String::new(), |f| cx.input.interner.get(syms.sym(f).name).to_string());
                    self.sconst(&name);
                }
                self.return_value(ret);
                self.code.bind(fail);
                self.new_object("java/lang/IndexOutOfBoundsException");
                self.load(ps, &ErasedType::I);
                self.invoke(Invoke::Static, STRING, false, "valueOf", &[ErasedType::I], &string);
                self.invoke(Invoke::Special, "java/lang/IndexOutOfBoundsException", false, "<init>", &[string], &ErasedType::V);
                self.code.op(op::ATHROW);
                self.code.pop();
                self.code.end_path();
            }
            _ => unreachable!("lowered (`Gen::vc_member_tree`)"),
        }
    }

    /// A method `(I)ret` answering the `n`th of `count` elements, as `productElement` does:
    /// `IndexOutOfBoundsException` for any other index.
    fn element_switch(&mut self, name: &str, ret: &ErasedType, count: usize, mut element: impl FnMut(&mut Self, usize) -> LirId) {
        let string = ErasedType::L(Rc::from(STRING));
        let value_of = method_ref(Invoke::Static, &Rc::from(STRING), false, "valueOf", &[ErasedType::I], &string);
        let out_of_bounds = method_ref(Invoke::Special, &Rc::from("java/lang/IndexOutOfBoundsException"), false, "<init>", &[string.clone()], &ErasedType::V);
        self.begin_method(MethodFlags::PUBLIC, name, &method_desc(&[ErasedType::I], ret), false, &[(None, ErasedType::I)], ret.clone(), &[]);
        self.body(|g| {
            let mut answers = Vec::with_capacity(count);
            for i in 0..count {
                let index = g.param(0);
                let at = g.lir.int(i as i32);
                let matched = g.lir.compare(Relation::Eq, Comparing::Numbers, index, at);
                let value = element(g, i);
                let answer = g.lir.ret(value);
                answers.push(g.lir.when(matched, answer));
            }
            let index = g.param(0);
            let shown = g.lir.static_call(&value_of, &[index]);
            let error = g.lir.make(&out_of_bounds, &[shown]);
            let thrown = g.lir.throw(error, ErasedType::V);
            g.lir.seq(&answers, thrown)
        });
    }

    fn product_members(&mut self, c: ClassId, is_case: bool, is_value_case: bool, is_object: bool) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let info = syms.class(c);
        let is_enum_case = info.kind == ClassKind::EnumCase;
        if !is_case && !is_enum_case {
            return;
        }
        // The members the rule gives the class (`TClass::synthesized`); an enum value's
        // `productArity`, `productElement`, `productElementName` and `canEqual` are scala's
        // `EnumValue`'s, which its class does not extend.
        let synthesized = cx.synthesized(c);
        let takes = |bit: u16| synthesized & bit != 0;
        let enum_value = |bit: u16| is_value_case && bit & (synthetic::PRODUCT_ARITY | synthetic::PRODUCT_ELEMENT | synthetic::PRODUCT_ELEMENT_NAME | synthetic::CAN_EQUAL) != 0;
        let owner = self.this_name.clone();
        let this = ErasedType::L(owner.clone());
        let string = ErasedType::L(Rc::from(STRING));
        let object = ErasedType::object();
        let display: Rc<str> = Rc::from(info.product_name(cx.input.interner.get(info.name)));
        let fields: Vec<SymId> = if is_value_case || is_object {
            Vec::new()
        } else {
            info.ctor.first().map(|cl| cl.params.iter().map(|p| p.sym).collect()).unwrap_or_default()
        };
        let field_refs: Vec<(String, ErasedType)> = fields.iter().map(|&f| (self.field_name(f), self.sym_type(f))).collect();

        // productPrefix, productArity, productElement
        if takes(synthetic::PRODUCT_PREFIX) {
            self.begin_method(MethodFlags::PUBLIC, "productPrefix", "()Ljava/lang/String;", false, &[], string.clone(), &[]);
            self.body(|g| {
                let prefix = if is_value_case {
                    let instance = g.lir.this(this.clone());
                    g.lir.field(instance, &owner, "$name", string.clone())
                } else {
                    g.lir.text(&display, string.clone())
                };
                g.lir.ret(prefix)
            });
            self.end_method();
        }

        if takes(synthetic::PRODUCT_ARITY) || enum_value(synthetic::PRODUCT_ARITY) {
            self.begin_method(MethodFlags::PUBLIC, "productArity", "()I", false, &[], ErasedType::I, &[]);
            self.body(|g| {
                let arity = g.lir.int(field_refs.len() as i32);
                g.lir.ret(arity)
            });
            self.end_method();
        }

        // A field of a value class is its box, as scalac's `productElement` gives it, and hashed
        // as the box (`Statics.anyHash(new V(..))`); `equals` compares the underlying values.
        if takes(synthetic::PRODUCT_ELEMENT) || enum_value(synthetic::PRODUCT_ELEMENT) {
            self.element_switch("productElement", &object, field_refs.len(), |g, i| {
                let (name, ty) = &field_refs[i];
                let instance = g.lir.this(this.clone());
                let read = g.lir.field(instance, &owner, name, ty.clone());
                g.lower_adapt_from_sym(read, fields[i], ty, &object)
            });
            self.end_method();
        }

        // `java.lang.Enum` answers `ordinal`, `hashCode` and `equals` with final methods.
        if takes(synthetic::ORDINAL) {
            self.begin_method(MethodFlags::PUBLIC, "ordinal", "()I", false, &[], ErasedType::I, &[]);
            self.body(|g| {
                let ordinal = if is_value_case {
                    let instance = g.lir.this(this.clone());
                    g.lir.field(instance, &owner, "$ordinal", ErasedType::I)
                } else {
                    g.lir.int(info.ordinal as i32)
                };
                g.lir.ret(ordinal)
            });
            self.end_method();
        }

        if takes(synthetic::CAN_EQUAL) || enum_value(synthetic::CAN_EQUAL) {
            self.begin_method(MethodFlags::PUBLIC, "canEqual", "(Ljava/lang/Object;)Z", false, &[(None, object.clone())], ErasedType::Z, &[]);
            self.body(|g| {
                let that = g.param(0);
                let same_class = g.lir.is(that, &owner);
                g.lir.ret(same_class)
            });
            self.end_method();
        }
        if takes(synthetic::PRODUCT_ELEMENT_NAME) || enum_value(synthetic::PRODUCT_ELEMENT_NAME) {
            let names: Vec<Rc<str>> = fields.iter().map(|&f| Rc::from(cx.input.interner.get(syms.sym(f).name))).collect();
            self.element_switch("productElementName", &string, names.len(), |g, i| g.lir.text(&names[i], string.clone()));
            self.end_method();
        }

        // `toString`: an enum value's name, a case object's own name, a case class's
        // `ScalaRunTime._toString(this)`, which reads the members it calls through the instance.
        if takes(synthetic::TO_STRING) {
            self.begin_method(MethodFlags::PUBLIC, "toString", "()Ljava/lang/String;", false, &[], string.clone(), &[]);
            self.body(|g| {
                let shown = if is_value_case {
                    let instance = g.lir.this(this.clone());
                    g.lir.field(instance, &owner, "$name", string.clone())
                } else if is_object {
                    g.lir.text(&display, string.clone())
                } else {
                    let to_string = method_ref(Invoke::Static, &Rc::from(SCALA_RUNTIME), false, "_toString", &[ErasedType::L(Rc::from(PRODUCT))], &string);
                    let instance = g.lir.this(this.clone());
                    g.lir.static_call(&to_string, &[instance])
                };
                g.lir.ret(shown)
            });
            self.end_method();
        }

        if takes(synthetic::HASH_CODE) {
            self.begin_method(MethodFlags::PUBLIC, "hashCode", "()I", false, &[], ErasedType::I, &[]);
            let hash_code = method_ref(Invoke::Virtual, &Rc::from(STRING), false, "hashCode", &[], &ErasedType::I);
            if is_value_case || field_refs.is_empty() {
                self.body(|g| {
                    let name = if is_value_case {
                        let instance = g.lir.this(this.clone());
                        g.lir.field(instance, &owner, "$name", string.clone())
                    } else {
                        g.lir.text(&display, string.clone())
                    };
                    let hash = g.lir.call(&hash_code, name, &[]);
                    g.lir.ret(hash)
                });
            } else if hashes_through_elements(syms, cx.input.types, c) {
                // dotty's `SyntheticMembers.chooseHashcode` for reference fields: `MurmurHash3.productHash`
                // from the seed of the name, through `productArity` and `productElement`.
                let murmur: Rc<str> = Rc::from("scala/util/hashing/MurmurHash3$");
                let product = ErasedType::L(Rc::from(PRODUCT));
                let product_hash = method_ref(Invoke::Virtual, &murmur, false, "productHash", &[product, ErasedType::I, ErasedType::Z], &ErasedType::I);
                let seed = crate::interp::value::mix(0xcafebabeu32 as i32, crate::interp::value::str_hash(&display));
                self.body(|g| {
                    let hashing = g.lir.module(&murmur);
                    let instance = g.lir.this(this.clone());
                    let seed = g.lir.int(seed);
                    let ignore_prefix = g.lir.bool(true);
                    let hash = g.lir.call(&product_hash, hashing, &[instance, seed, ignore_prefix]);
                    g.lir.ret(hash)
                });
            } else {
                // MurmurHash3.productHash, as scalac's case classes hash: the name's hash mixed into
                // the seed, then each field's `anyHash`, then the arity.
                self.body(|g| {
                    let seed = g.lir.int(0xcafebabeu32 as i32);
                    let name = g.lir.text(&display, string.clone());
                    let name_hash = g.lir.call(&hash_code, name, &[]);
                    let mut hash = g.lower_helper("mix", &[seed, name_hash], ErasedType::I);
                    for (i, (name, ty)) in field_refs.iter().enumerate() {
                        let instance = g.lir.this(this.clone());
                        let read = g.lir.field(instance, &owner, name, ty.clone());
                        let value = g.lower_adapt_from_sym(read, fields[i], ty, &object);
                        let element = g.lower_helper("anyHash", &[value], ErasedType::I);
                        hash = g.lower_helper("mix", &[hash, element], ErasedType::I);
                    }
                    let arity = g.lir.int(field_refs.len() as i32);
                    let finished = g.lower_helper("finalizeHash", &[hash, arity], ErasedType::I);
                    g.lir.ret(finished)
                });
            }
            self.end_method();
        }

        // `SyntheticMembers.equalsBody`: the same class, the primitive fields compared first, then
        // the others, then `that.canEqual(this)` unless the class is final with the rule's.
        if takes(synthetic::EQUALS) {
            self.begin_method(MethodFlags::PUBLIC, "equals", "(Ljava/lang/Object;)Z", false, &[(None, object.clone())], ErasedType::Z, &[]);
            // The enclosing instances, by identity (`PatternMatcher.addOuterTest` of the rule's
            // `x$0 @ (_: C)`, `TClass::eq_outer`), read through the accessor on this class.
            let outer = cx.eq_outer(c).map(|accessor| {
                let m = self.mref(accessor);
                Rc::new(MethodRef { owner: owner.clone(), owner_is_interface: false, kind: Invoke::Virtual, ..(*m).clone() })
            });
            let order = cx.equals_order(c, field_refs.len());
            let can_equal = method_ref(Invoke::Virtual, &owner, false, "canEqual", &[object.clone()], &ErasedType::Z);
            self.body(|g| {
                let (no, yes) = (g.lir.label(), g.lir.label());
                let mut tests = Vec::new();
                let instance = g.lir.this(this.clone());
                let given = g.param(0);
                let same = g.lir.compare(Relation::Eq, Comparing::Identity, instance, given);
                let equal = g.lir.leave(yes);
                tests.push(g.lir.when(same, equal));
                let given = g.param(0);
                let same_class = g.lir.is(given, &owner);
                let other_class = g.lir.not(same_class);
                let unequal = g.lir.leave(no);
                tests.push(g.lir.when(other_class, unequal));
                let given = g.param(0);
                let cast = g.lir.checked(given, this.clone());
                let that = g.lir.temp();
                tests.push(g.lir.keep(that, cast));
                if let Some(outer) = &outer {
                    let instance = g.lir.this(this.clone());
                    let mine = g.lir.call(outer, instance, &[]);
                    let other = g.lir.kept(that, this.clone());
                    let theirs = g.lir.call(outer, other, &[]);
                    let differs = g.lir.compare(Relation::Ne, Comparing::Identity, mine, theirs);
                    let unequal = g.lir.leave(no);
                    tests.push(g.lir.when(differs, unequal));
                }
                for (name, ty) in order.iter().filter_map(|&i| field_refs.get(i)) {
                    let instance = g.lir.this(this.clone());
                    let mine = g.lir.field(instance, &owner, name, ty.clone());
                    let other = g.lir.kept(that, this.clone());
                    let theirs = g.lir.field(other, &owner, name, ty.clone());
                    let differs = match ty {
                        ErasedType::L(_) => {
                            let equal = g.lower_helper("equal", &[mine, theirs], ErasedType::Z);
                            g.lir.not(equal)
                        }
                        _ => g.lir.compare(Relation::Ne, Comparing::Numbers, mine, theirs),
                    };
                    let unequal = g.lir.leave(no);
                    tests.push(g.lir.when(differs, unequal));
                }
                if takes(synthetic::EQUALS_CALLS_CAN_EQUAL) {
                    let other = g.lir.kept(that, this.clone());
                    let instance = g.lir.this(this.clone());
                    let can = g.lir.call(&can_equal, other, &[instance]);
                    let cannot = g.lir.not(can);
                    let unequal = g.lir.leave(no);
                    tests.push(g.lir.when(cannot, unequal));
                }
                let unit = g.lir.unit();
                let checks = g.lir.seq(&tests, unit);
                let checked = g.lir.labeled(yes, checks);
                let truth = g.lir.bool(true);
                let answer = g.lir.ret(truth);
                let refused = g.lir.seq(&[checked], answer);
                let refused = g.lir.labeled(no, refused);
                let falsity = g.lir.bool(false);
                let answer = g.lir.ret(falsity);
                g.lir.seq(&[refused], answer)
            });
            self.end_method();
        }
    }

    // ---- the top-level definitions of a file ----

    /// The file module's `<clinit>`: its lazy members' handles looked up first, the instance, then
    /// the eager vals of its constructor's tree (`Gen::file_module_tree`), where the class file
    /// runs them (`Gen::write_module_part`).
    fn file_module_clinit(&mut self, name: &Rc<str>, fields: &[FieldDef]) {
        let roots = file_module_roots(fields);
        self.begin_module_clinit(self.clinit_flags(), name, &roots);
        self.write_module(false, |g| g.file_module_tree(fields));
        self.end_method();
    }

    /// The file module's constructor as one tree, an object's (`Gen::constructor_parts`):
    /// `Object`'s constructor, then each eager val stored in its field, in order, a constant's
    /// initialiser that is no literal run for its effects alone (`Constructors`).
    fn file_module_tree(&mut self, fields: &[FieldDef]) -> (LirId, Vec<Declared>) {
        let owner = self.this_name.clone();
        let this = ErasedType::L(owner.clone());
        let object_init = method_ref(Invoke::Special, &Rc::from(OBJECT), false, "<init>", &[], &ErasedType::V);
        let mut parts = vec![self.lir.init(&object_init, &[])];
        for f in fields.iter().filter(|f| !f.lazy) {
            let init = f.init.expect("an eager val's initialiser");
            match f.constant {
                Some(ConstVal::Declared(_)) => parts.push(self.lower_expr(init, &ErasedType::V)),
                Some(_) => {}
                None => {
                    let instance = self.lir.this(this.clone());
                    let value = self.lower_expr(init, &f.ty);
                    parts.push(self.lir.store(instance, &owner, &f.name, value));
                }
            }
        }
        self.returning(Parts::Tree(parts, Vec::new()))
    }

    pub fn emit_file_module(&mut self, file: FileId) -> (String, Vec<u8>) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let syms = cx.input.syms;
        let reach = cx.input.reach;
        let name: Rc<str> = Rc::from(cx.file_modules[file.0 as usize].as_str());
        self.context = name.to_string();
        let listed = super::callable::module_interfaces(&[], false);
        let path = &cx.input.sources[file.0 as usize].path;
        self.begin_class(&name, ClassFlags::FINAL, OBJECT, &[], &listed, Some(path.rsplit(std::path::is_separator).next().unwrap_or(path)));
        self.this_name = name.clone();
        self.this_class = None;
        self.is_interface = false;
        self.abstract_class = false;
        self.line_file = file;
        // The companions of the file's top-level opaque types are its nested objects, static
        // final fields nothing assigns, as an object's are (`emit_class`).
        for &k in cx.opaque_companions.get(&file.0).map_or(&[][..], |v| &v[..]) {
            let field = encode(cx.input.interner.get(syms.class(k).name));
            let ty = ErasedType::L(self.class_name(k));
            let flags = syms.class(k).module_sym.map_or(MODULE_FIELD, |m| module_field_flags(syms, m));
            self.field(flags, &field, &ty);
        }
        self.field(MODULE_FIELD, "MODULE$", &ErasedType::L(name.clone()));

        let group = cx.file_group[file.0 as usize];
        let vals: &[(SymId, TExprId)] = if group == u32::MAX { &[] } else { &cx.layout.file_groups[group as usize].1 };
        let file_reached = reach.files.get(file.0 as usize).copied().unwrap_or(false);
        self.lazy_holders.clear();
        let mut fields: Vec<FieldDef> = Vec::new();
        for &(s, init) in vals {
            // An inline val is erased (`isEffectivelyErased`, `Erasure.typedValDef`): no field and no getter, every
            // read of it the constant the typer folded.
            if syms.sym(s).mods & mods::INLINE != 0 {
                continue;
            }
            let eager = crate::emit::layout::is_eager_top_val(syms, s, &cx.layout.const_vals);
            if (eager && !file_reached) || (!eager && !reach.vals[s.idx()]) || cx.given_objects.contains_key(&s) || cx.given_classes.contains_key(&s) {
                continue;
            }
            let ty = self.sym_type(s);
            let fname = self.field_name(s);
            let info = syms.sym(s);
            // What dotty's `LazyVals` sees as lazy: a lazy val and a given alias (`Parsers.givenDef`).
            // A constant (`file_constant`) is `Memoize`'s getter of the constant and no field, an
            // initialiser that is no literal run in `<clinit>` for its effects; a val the compiler
            // made is eager, as dotty's are.
            let lazy = !eager && (info.mods & mods::LAZY != 0 || info.kind == SymKind::Given);
            let constant = if lazy { None } else { self.constant(s, init) };
            fields.push(FieldDef { sym: s, name: fname, ty, lazy, lazy_owner: None, init: Some(init), mutable: info.kind == SymKind::Var, constant });
        }
        for f in &fields {
            if f.lazy && self.is_thread_unsafe(f.sym) {
                // `@threadUnsafe`: its container and flag (`emit_class`).
                let (container, flag) = self.thread_unsafe_names(&f.name);
                self.field(FieldFlags::PRIVATE, &container, &f.ty);
                self.field(FieldFlags::PRIVATE, &flag, &ErasedType::Z);
                continue;
            }
            if f.lazy {
                // `private @volatile var x$lzyN: AnyRef` (`emit_class`).
                let h = self.lazy_holder(&f.name, &f.ty);
                self.field(FieldFlags { volatile: true, ..FieldFlags::PRIVATE }, &h.container, &ErasedType::object());
                continue;
            }
            if f.constant.is_some() {
                continue;
            }
            self.field(FieldFlags { volatile: self.is_volatile(f.sym), ..FieldFlags::PRIVATE }, &f.name, &f.ty);
        }
        let given_inits: Vec<TInit> = vals.iter().map(|&(s, e)| TInit::Field(s, e)).collect();
        self.given_object_fields(&given_inits);
        // The `<clinit>` first where the file has no lazy member, as it always was; else after the accessors,
        // whose lazy members' handles it looks up first.
        let clinit_first = !self.holds_lazy_members(None, file);
        if clinit_first {
            self.file_module_clinit(&name, &fields);
        }
        self.begin_method(MethodFlags { visibility: Visibility::Private, ..MethodFlags::PUBLIC }, "<init>", "()V", true, &[], ErasedType::V, &file_module_roots(&fields));
        self.write_module(true, |g| g.file_module_tree(&fields));
        self.end_method();
        for f in &fields {
            self.accessors(f);
        }
        if clinit_first {
            if !self.lazy_holders.is_empty() {
                self.unsupported("a lazy member the names walk did not number (its handle has no lookup)");
            }
        } else {
            self.file_module_clinit(&name, &fields);
        }
        let mut funs: Vec<FunId> = cx.file_funs[file.0 as usize].iter().copied().filter(|f| reach.funs[f.idx()]).collect();
        funs.sort_by(|&a, &b| crate::emit::layout::compare_syms(syms, cx.input.interner, prog.funs[a.idx()].sym, prog.funs[b.idx()].sym));
        for f in funs {
            self.emit_fun(f);
        }
        let pkg = cx.input.file_pkgs[file.0 as usize];
        let mut aliases: Vec<crate::types::AliasId> = syms.pkg(pkg).entries.values().filter_map(|e| e.alias).filter(|&a| syms.aliases[a.idx()].file == file && syms.aliases[a.idx()].owner == Owner::Package(pkg)).collect();
        aliases.sort();
        self.companion_forwarders(&aliases);
        self.given_class_defs(Owner::Package(pkg), Some(file));
        self.owned_lazy_methods(None, file);
        self.drain_pending();
        self.finish_class(&name)
    }

    /// `TeqMain.main(String[])` calls the entry point: the command line reaches a
    /// `main(args: Array[String])` as an array and a `@main def f(args: String*)` as a sequence.
    pub fn emit_launcher(&mut self, main: SymId, object: Option<ClassId>) -> (String, Vec<u8>) {
        self.begin_class(LAUNCHER, ClassFlags::FINAL, OBJECT, &[], &[], None);
        self.this_name = Rc::from(LAUNCHER);
        self.this_class = None;
        self.is_interface = false;
        self.context = LAUNCHER.to_string();
        self.entry_main(main, object);
        (LAUNCHER.to_string(), self.finish_writer())
    }

    /// `static main(String[])` calling the entry point `main` of the program.
    pub fn entry_main(&mut self, main: SymId, object: Option<ClassId>) {
        let cx = self.cx;
        // A typed `@main` method runs through its proxy, which parses the command line.
        let main = cx.input.syms.main_adapter(cx.input.interner, main).unwrap_or(main);
        let info = cx.input.syms.sym(main);
        let args_type = ErasedType::L(Rc::from("[Ljava/lang/String;"));
        self.begin_method(MethodFlags { is_static: true, ..MethodFlags::PUBLIC }, "main", "([Ljava/lang/String;)V", false, &[(None, args_type.clone())], ErasedType::V, &[]);
        let m = self.mref(main);
        let module = match (object, info.owner) {
            (Some(c), _) | (None, Owner::Class(c)) => self.module_name(c),
            _ => Some(self.file_module_name(info.file)),
        };
        let repeated = info.sig.as_ref().and_then(|s| s.clauses.iter().flat_map(|c| c.params.iter()).next()).map_or(false, |p| p.repeated);
        // The arguments' array itself in an `ArraySeq`: scala-library's `ofRef` takes it as an
        // array of references, the std's as an array of any kind.
        let wrap = cx.input.array_seq.filter(|_| repeated).map(|seq| {
            let taken = if cx.linked_class(seq) { ErasedType::L(Rc::from(OBJECT_ARRAY)) } else { ErasedType::object() };
            method_ref(Invoke::Special, &self.class_name(seq), false, "<init>", &[taken], &ErasedType::V)
        });
        let mut param = m.params.first().cloned();
        if param.is_some() && repeated && wrap.is_none() {
            self.unsupported("a repeated parameter of the entry point without ArraySeq");
            param = None;
        }
        self.body(|g| {
            let receiver = match &module {
                Some(module) => g.lir.module(module),
                None => g.lir.unit(),
            };
            let mut args = Vec::new();
            if let Some(param) = &param {
                let given = g.param(0);
                args.push(match &wrap {
                    Some(seq) => {
                        let taken = g.lower_adapt(given, &args_type, &seq.params[0]);
                        let made = g.lir.make(seq, &[taken]);
                        g.lower_adapt(made, &ErasedType::L(seq.owner.clone()), param)
                    }
                    None => g.lower_adapt(given, &args_type, param),
                });
            }
            let called = g.lir.call(&m, receiver, &args);
            let dropped = g.lower_adapt(called, &m.ret, &ErasedType::V);
            g.lir.ret(dropped)
        });
        self.end_method();
    }
}

/// What the file module's constructor's locals are found shared in (`free::cells`): its eager
/// vals' initialisers.
fn file_module_roots(fields: &[FieldDef]) -> Vec<TExprId> {
    fields.iter().filter(|f| !f.lazy).filter_map(|f| f.init).collect()
}

/// The expressions an initialiser of a class's body evaluates: a field's, a statement, and what
/// it passes to a trait's parameters with the statements before.
fn init_roots(prog: &Program, init: &TInit, roots: &mut Vec<TExprId>) {
    match *init {
        TInit::Field(_, e) | TInit::Stmt(e) => roots.push(e),
        TInit::Parent(_, call) => {
            for s in &prog.stmts[call.prelude.range()] {
                if let TStmt::Val(_, e) | TStmt::Expr(e) = *s {
                    roots.push(e);
                }
            }
            roots.extend_from_slice(prog.expr_list(call.args));
        }
    }
}

/// Whether a member has an implementation a class can forward to: the program's own reached
/// body, or another module's member that is not abstract or is an `abstract override`.
fn implemented(cx: &super::Cx, s: SymId) -> bool {
    let f = cx.fun_of_sym[s.idx()];
    if f != u32::MAX && cx.input.prog.funs[f as usize].body.is_some() && cx.input.reach.funs[f as usize] {
        return true;
    }
    let info = cx.input.syms.sym(s);
    let Owner::Class(o) = info.owner else { return false };
    cx.input.reach.product_classes.get(o.idx()).copied().unwrap_or(false)
        && (info.mods & mods::ABSTRACT == 0 || cx.input.stackable.map_or(false, |st| st.contains_key(&s)))
}

/// Whether a member has a body, as the member choice reads it: an `abstract override` of
/// another module's trait has one (`Input::stackable`), which dotty takes for no `Deferred`.
/// A declaration of a member of one of `Object`'s names (`Gen::declarations`): its erased
/// descriptor, how many type parameters it takes where that is known, the program's symbol of it,
/// and whether it is private or deferred.
#[derive(Clone)]
struct Decl {
    desc: String,
    poly: Option<usize>,
    sym: Option<SymId>,
    private: bool,
    deferred: bool,
}

impl Decl {
    /// Whether the two match as dotty's `matches` before erasure tells overriding members: of as
    /// many type parameters (where both are known) and of the same erased parameters, whatever
    /// their results (an override's may be narrower).
    fn matches(&self, other: &Decl) -> bool {
        let taking = |d: &str| d[..d.find(')').map_or(d.len(), |at| at + 1)].to_string();
        taking(&self.desc) == taking(&other.desc) && !matches!((self.poly, other.poly), (Some(a), Some(b)) if a != b)
    }
}

fn concrete_member(cx: &super::Cx, s: SymId) -> bool {
    !cx.input.reach.is_abstract(cx.input.syms, s) || cx.input.stackable.map_or(false, |st| st.contains_key(&s))
}

/// Whether an abstract redeclaration in a base of the class overrides `b`'s definition `s`, so
/// that it is no member of the class (`Symbols::member_declaration`) and dotty's
/// `MixinOps.isInImplementingClass` writes no forwarder to it: per alternative, each base's of
/// the output name `s` has (an overriding alternative takes the name of the one it overrides).
fn redeclared_away(cx: &super::Cx, info: &ClassInfo, s: SymId, b: ClassId) -> bool {
    let syms = cx.input.syms;
    let (name, d) = (syms.sym(s).name, syms.dispatch_name(s));
    let declared: Vec<(ClassId, bool)> = info
        .base_types
        .iter()
        .skip(1)
        .filter_map(|&(k, _)| {
            let &m = syms.class(k).members.get(&name)?;
            let single = [m];
            let alts = syms.alternatives(m).unwrap_or(&single);
            let a = alts.iter().copied().find(|&a| {
                let i = syms.sym(a);
                matches!(i.kind, SymKind::Def | SymKind::Val | SymKind::Var | SymKind::Given) && i.owner == Owner::Class(k) && syms.dispatch_name(a) == d
            })?;
            Some((k, concrete_member(cx, a)))
        })
        .collect();
    declared.iter().any(|&(_, concrete)| !concrete) && syms.member_declaration(&declared) != Some(b)
}

/// Whether a class has an entry of its own for the name. For a class that mixes in another
/// module's trait, or in the product mode, the placeholder that joins two ancestors' methods of
/// the name before a lookup settles it is none: the name may never be looked up in the class.
fn defines(cx: &super::Cx, info: &ClassInfo, name: crate::intern::Name) -> bool {
    let syms = cx.input.syms;
    let products = &cx.input.reach.product_classes;
    let open = cx.input.open_world || info.base_types.iter().any(|&(b, _)| products.get(b.idx()).copied().unwrap_or(false));
    info.members.get(&name).map_or(false, |&e| !open || syms.alternatives(e).map_or(true, |alts| !alts.is_empty()))
}

/// The member name `x_=` of the setter of the var `x`, where some member has it.
fn setter_member_name(interner: &crate::intern::Interner, var: Name) -> Option<Name> {
    interner.lookup(&format!("{}_=", interner.get(var)))
}

/// The flags of the static final field that holds a nested object in its owner's module class,
/// as dotty's `javaFieldFlags` derives them from the module term (`BCodeUtils.javaFlags`): private
/// for a private object, public for a qualified private one (`private[p]`) as for any other.
fn module_field_flags(syms: &Symbols, m: SymId) -> FieldFlags {
    let info = syms.sym(m);
    if info.mods & mods::PRIVATE != 0 && !info.scoped_private {
        FieldFlags { visibility: Visibility::Private, ..MODULE_FIELD }
    } else {
        MODULE_FIELD
    }
}

/// Whether `c` is a parent `Desugar.classDef` gives: `Product`, its `Equals`, `java.io.Serializable`
/// (`Symbols::is_synthetic_parent`) or `scala.reflect.Enum`; or one `SyntheticMembers
/// .addMirrorSupport` gives: `scala.deriving.Mirror` and its `Product`, `Sum` and `Singleton`.
fn is_desugared_parent(syms: &crate::symbols::Symbols, interner: &crate::intern::Interner, c: ClassId) -> bool {
    if syms.is_synthetic_parent(interner, c) {
        return true;
    }
    let info = syms.class(c);
    let in_package = |p: crate::types::PkgId, name: &str, parent: &str| {
        interner.get(syms.pkg(p).name) == name && syms.pkg(p).parent.is_some_and(|q| interner.get(syms.pkg(q).name) == parent)
    };
    match info.owner {
        Owner::Package(p) => match interner.get(info.name) {
            "Enum" => in_package(p, "reflect", "scala"),
            "Mirror" => in_package(p, "deriving", "scala"),
            _ => false,
        },
        Owner::Class(m) => {
            matches!(interner.get(info.name), "Product" | "Sum" | "Singleton")
                && matches!(interner.get(syms.class(m).name), "Mirror" | "Mirror$")
                && matches!(syms.class(m).owner, Owner::Package(p) if in_package(p, "deriving", "scala"))
        }
        _ => false,
    }
}
