//! Classes, traits, objects, enums and the module class of a file's top-level definitions.

use super::classfile::*;
use super::free;
use super::gen::*;
use super::names::*;
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
    vars: Vec<(String, JType)>,
    /// A member held in a lazy holder: its getter's name and type, and how it is computed.
    holders: Vec<(String, JType, HolderInit)>,
}

/// scalac's marker of a lazy val computed to null (`LazyVals.NullValue`).
const NULL_VALUE: &str = "scala/runtime/LazyVals$NullValue$";

/// How a lazy holder's getter computes the value the first time (`lazy_holder_body`).
#[derive(Clone)]
enum HolderInit {
    /// The trait's static `x$(T)`, which runs the lazy val's or object's initialiser.
    TraitStatic(Rc<str>),
    Expr(TExprId),
    Unit,
    /// A Scala 2 trait's object: its module class made with the instance as its outer
    /// reference, `new T$cell$(this)` (dotty's `Mixin`, 286-295).
    Module { class: Rc<str>, outer: Rc<str> },
}

/// A field of a class with the symbol it stores.
#[derive(Clone)]
struct FieldDef {
    sym: SymId,
    name: String,
    ty: JType,
    /// A lazy val or a given: the field holds the boxed value once it is computed.
    lazy: bool,
    /// For a lazy member of a trait: the trait whose static initialiser computes the value.
    lazy_owner: Option<ClassId>,
    init: Option<TExprId>,
    mutable: bool,
}

impl<'a> Gen<'a> {
    fn is_lazy_member(&self, s: SymId) -> bool {
        self.cx.is_lazy_member(s)
    }

    // ---- methods ----

    /// Starts a method: `this` in slot 0 unless static, then the parameters.
    pub fn begin_method(&mut self, is_static: bool, this_uninit: bool, params: &[(Option<SymId>, JType)], ret: JType, roots: &[TExprId]) {
        let mut locals: Vec<VT> = Vec::new();
        let mut m = MethodCx::default();
        if !is_static {
            let name = self.this_name.clone();
            locals.push(if this_uninit { VT::UninitThis } else { VT::Object(self.cw.cp.class(&name)) });
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
        }
        m.ret = ret;
        let syms: Vec<SymId> = params.iter().filter_map(|p| p.0).collect();
        m.cells = free::cells(self.cx, &mut self.an, &syms, roots);
        self.code = Code::new(locals);
        self.code.wide = self.wide;
        self.m = m;
    }

    pub fn end_method(&mut self, access: u16, name: &str, desc: &str) {
        let code = std::mem::replace(&mut self.code, Code::new(Vec::new()));
        if code.underflow {
            let line = code.last_line().map_or(String::new(), |l| format!(" at line {}", l));
            self.unsupported(&format!("the operand stack model underflowed while writing {}{}{}", name, desc, line));
            return;
        }
        if self.cw.has_method(name, desc) {
            return;
        }
        self.cw.method(access, name, desc, Some(code));
        self.overflow |= self.cw.overflow;
    }

    /// The lifted methods that the methods emitted so far asked for, and the ones those ask for.
    pub fn drain_pending(&mut self) {
        while let Some(p) = self.pending.pop() {
            self.emit_lifted(p);
        }
    }

    fn emit_lifted(&mut self, p: Pending) {
        let cx = self.cx;
        let l = p.lifted.clone();
        if let PendingBody::JarGetter(g) = &p.body {
            self.jar_getter_thunk(&l, g);
            return;
        }
        let mut params: Vec<(Option<SymId>, JType)> = Vec::new();
        if l.this {
            let this = self.lifted_this_type();
            params.push((None, this));
        }
        for c in &l.captures {
            params.push((Some(c.sym), if c.cell { JType::L(Rc::from(OBJECT_REF)) } else { c.ty.clone() }));
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
                (fun.params[..*i].to_vec(), fun.defaults[*i], fun.defaults[*i].into_iter().collect())
            }
            PendingBody::Lazy(_, init) => (Vec::new(), Some(*init), vec![*init]),
            PendingBody::JarGetter(_) => unreachable!(),
        };
        let is_lambda = matches!(p.body, PendingBody::Lambda(..));
        for (i, t) in l.params.iter().enumerate() {
            // The parameters of a lambda arrive boxed and get locals of their own below.
            let sym = if is_lambda { None } else { param_syms.get(i).copied() };
            params.push((sym, t.clone()));
        }
        self.begin_method(true, false, &params, l.ret.clone(), &roots);
        self.m.lambda = is_lambda;
        if l.this {
            self.m.this_slot = Some(0);
        }
        for c in &l.captures {
            if let Some(local) = self.m.locals.get_mut(&c.sym) {
                local.ty = c.ty.clone();
                local.cell = c.cell;
            }
        }
        if is_lambda {
            let object = JType::object();
            let mut slot = params[..first_param].iter().map(|(_, t)| if t.wide() { 2 } else { 1 }).sum::<u16>();
            for &s in &param_syms {
                let declared = self.sym_type(s);
                if declared.is_object() && !self.is_vc_object(&declared) {
                    self.m.locals.insert(s, Local { slot, ty: declared, cell: false });
                } else {
                    self.load(slot, &object);
                    let declared = self.adapt_to_sym(&object, s);
                    let cell = self.m.cells.contains(&s);
                    self.declare_local(s, declared, cell);
                }
                slot += 1;
            }
        }
        match (&p.body, body) {
            (PendingBody::Lazy(sym, init), _) => {
                // The cell is the last parameter; it holds null until the value is there, a
                // computed null as the cell itself, which no value of the program is
                // (`LazyVals.transformLocalDef`'s `LazyRef` keeps an `initialized` flag apart).
                let cell = JType::L(Rc::from(OBJECT_REF));
                let cell_slot = params[..params.len() - 1].iter().map(|(_, t)| if t.wide() { 2 } else { 1 }).sum::<u16>();
                let object = JType::object();
                let declared = self.sym_type(*sym);
                let null_once = matches!(declared, JType::L(_));
                self.m.locals.insert(*sym, Local { slot: cell_slot, ty: declared.clone(), cell: true });
                let ready = self.code.new_label();
                let done = self.code.new_label();
                self.load(cell_slot, &cell);
                self.getfield(OBJECT_REF, "elem", &object);
                self.dup();
                self.jump_if(op::IFNONNULL, 1, ready);
                self.pop_value(&object);
                self.expr(*init, &declared);
                self.adapt(&declared, &object);
                let vt = self.vt(&object);
                self.code.retype_top(vt);
                let value = self.store_new(&object);
                self.load(cell_slot, &cell);
                self.load(value, &object);
                if null_once {
                    let computed = self.code.new_label();
                    self.dup();
                    self.jump_if(op::IFNONNULL, 1, computed);
                    self.pop_value(&object);
                    self.load(cell_slot, &cell);
                    let vt = self.vt(&object);
                    self.code.retype_top(vt);
                    self.code.bind(computed);
                }
                self.putfield(OBJECT_REF, "elem", &object);
                self.load(value, &object);
                self.goto(done);
                self.code.bind(ready);
                if null_once {
                    let held = self.code.new_label();
                    self.dup();
                    self.load(cell_slot, &cell);
                    self.jump_if(op::IF_ACMPNE, 2, held);
                    self.pop_value(&object);
                    self.code.op(op::ACONST_NULL);
                    self.code.push(VT::Null);
                    self.code.bind(held);
                }
                self.code.bind(done);
                self.adapt(&object, &declared);
                self.return_value(&declared);
            }
            (PendingBody::Fun(f), Some(body)) => {
                let fun = &cx.input.prog.funs[f.idx()];
                self.method_body(fun.sym, &fun.params, body);
            }
            (_, Some(body)) => self.tail(body),
            (_, None) => {
                self.unsupported("a local def without a body");
                let ret = l.ret.clone();
                self.adapt(&JType::V, &ret);
                self.return_value(&ret);
            }
        }
        self.end_method(ACC_PUBLIC | ACC_STATIC | ACC_SYNTHETIC, &l.name, &l.desc);
    }

    fn jar_getter_thunk(&mut self, l: &Lifted, g: &JarGetter) {
        let (captured, _) = parse_method_desc(&l.desc);
        let params: Vec<(Option<SymId>, JType)> = captured.iter().map(|t| (None, t.clone())).collect();
        self.begin_method(true, false, &params, l.ret.clone(), &[]);
        let mut slot = 0;
        for t in &captured {
            self.load(slot, t);
            slot += if t.wide() { 2 } else { 1 };
        }
        let n_params = captured.len() - (g.kind != Invoke::Static) as usize;
        self.invoke_desc(g.kind, &g.owner, g.owner_is_interface, &g.name, &g.desc, n_params, &g.ret);
        self.adapt(&g.ret, &l.ret);
        self.return_value(&l.ret);
        self.end_method(ACC_PUBLIC | ACC_STATIC | ACC_SYNTHETIC, &l.name, &l.desc);
    }

    /// The body in return position, as a loop when the def calls itself in tail position and
    /// nothing can override it.
    pub fn method_body(&mut self, sym: SymId, params: &[SymId], body: TExprId) {
        let cx = self.cx;
        let (calls, _) = cx.input.prog.tail_self_calls(sym, params.len(), body);
        if calls > 0 && cx.input.syms.is_effectively_final(sym) && params.iter().all(|p| self.m.locals.get(p).map_or(false, |l| !l.cell)) {
            let head = self.code.new_label();
            self.code.bind(head);
            self.m.tail = Some(TailLoop { sym, params: params.to_vec(), head });
        }
        self.tail(body);
        self.m.tail = None;
    }

    fn emit_fun(&mut self, f: FunId) {
        let cx = self.cx;
        let fun = &cx.input.prog.funs[f.idx()];
        let info = cx.input.syms.sym(fun.sym);
        let m = self.mref(fun.sym);
        let name = cx.input.interner.get(info.name);
        self.context = format!("{}.{}", self.this_name, name);
        let ordered: Vec<SymId> = match self.jvm_param_order(fun.sym) {
            Some(order) if order.len() == fun.params.len() => order.iter().map(|&k| fun.params[k]).collect(),
            _ => fun.params.clone(),
        };
        let params: Vec<(Option<SymId>, JType)> = ordered.iter().zip(&m.params).map(|(&s, t)| (Some(s), t.clone())).collect();
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
                self.begin_method(false, false, &params, m.ret.clone(), &[body]);
                self.code.line(cx.line_of(info.file, info.span.start));
                self.method_body(fun.sym, &fun.params, body);
                // A given class's outer accessor is scalac's, final and synthetic.
                let given = self.this_class.map_or(false, |c| cx.given_class_classes.contains_key(&c) || cx.inner_given_object_classes.contains_key(&c));
                let outer_accessor = given && info.def.is_none() && name.ends_with("$$$outer");
                let access = if outer_accessor { ACC_PUBLIC | ACC_FINAL | ACC_SYNTHETIC } else { ACC_PUBLIC };
                self.end_method(access, &m.name, &m.desc);
                self.emit_bridges(fun.sym, &m);
                if self.is_interface && (info.mods & mods::PRIVATE == 0 || info.scoped_private) {
                    self.trait_static_forwarder(&m.name, &m.params, &m.ret);
                }
            }
            None => {
                if self.is_interface || self.abstract_class {
                    if !self.cw.has_method(&m.name, &m.desc) {
                        self.cw.method(ACC_PUBLIC | ACC_ABSTRACT, &m.name, &m.desc, None);
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
            self.begin_method(false, false, &params[..before], ret.clone(), &[d]);
            self.default_value(d, shape.by_name, &ret);
            self.return_value(&ret);
            let desc = method_desc(&m.params[..before], &ret);
            self.end_method(ACC_PUBLIC, &getter, &desc);
            if self.is_interface {
                self.trait_static_forwarder(&getter, &m.params[..before], &ret);
            }
        }
    }

    /// scalac's static forwarder `m$(T, args)` of a trait's concrete method `m`, which calls the
    /// default method on its first argument: a class scalac compiles mixing the trait in
    /// implements `m` by calling it. Every concrete method of the trait has one, a protected
    /// or final one, an extension, a default getter, a lazy or constant val's getter; a private
    /// one has none.
    pub fn trait_static_forwarder(&mut self, name: &str, params: &[JType], ret: &JType) {
        let owner = self.this_name.clone();
        let this = JType::L(owner.clone());
        let mut types = vec![this.clone()];
        types.extend(params.iter().cloned());
        let desc = method_desc(params, ret);
        let typed: Vec<(Option<SymId>, JType)> = types.iter().map(|t| (None, t.clone())).collect();
        self.begin_method(true, false, &typed, ret.clone(), &[]);
        let mut slot = 0u16;
        for t in &types {
            self.load(slot, t);
            slot += if t.wide() { 2 } else { 1 };
        }
        self.invoke_desc(Invoke::Special, &owner, true, name, &desc, params.len(), ret);
        self.return_value(ret);
        self.end_method(ACC_PUBLIC | ACC_STATIC, &format!("{}$", name), &method_desc(&types, ret));
    }

    /// A trait's `final val` of a constant type (`final val limit = 5`), whose getter scalac
    /// writes in the trait itself, a default method answering the constant: no class mixing the
    /// trait in has a field, a getter or a setter for it.
    /// It is one as the typer's `constant_step` has it: written without a type or with a literal
    /// one, its initialiser a literal.
    fn is_trait_constant(&self, s: SymId, init: TExprId) -> bool {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        let untyped = info.def.map_or(false, |d| matches!(cx.input.asts[info.file.0 as usize].def(d).kind, crate::ast::DefKind::Val { ty: None, .. }))
            || info.sig.as_ref().map_or(false, |sig| matches!(cx.input.types.get(sig.ret), Type::Lit(_)));
        info.kind == SymKind::Val
            && info.mods & mods::FINAL != 0
            && info.mods & (mods::LAZY | mods::GIVEN) == 0
            && matches!(info.owner, Owner::Class(t) if cx.is_interface(t))
            && untyped
            && matches!(cx.input.prog.expr(init), TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_))
    }

    /// The value of default `d` as its getter returns it: a by-name parameter's default is typed
    /// as the argument passed for it, a lambda without parameters, whose body is the value.
    pub fn default_value(&mut self, d: TExprId, by_name: bool, ret: &JType) {
        let prog = self.cx.input.prog;
        match prog.expr(d) {
            TExpr::Lambda(ps, body) if by_name && ps.len == 0 => self.expr(body, ret),
            _ if by_name => {
                let thunk = self.function_type(0);
                self.expr(d, &thunk);
                let JType::L(owner) = &thunk else { unreachable!() };
                let object = JType::object();
                self.invoke(Invoke::Interface, owner, true, "apply", &[], &object);
                self.adapt(&object, ret);
            }
            _ => self.expr(d, ret),
        }
    }

    fn is_value_class_method(&self) -> bool {
        let cx = self.cx;
        self.this_class.map_or(false, |c| cx.input.syms.class(c).value_class)
    }

    /// In link mode, a method of a value class is scalac's pair: the static `m$extension`,
    /// which takes the underlying value first and holds the body, and the method of the box,
    /// which forwards to it.
    fn value_class_method(&mut self, f: FunId, m: &MRef, params: &[(Option<SymId>, JType)], body: TExprId) {
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
        let ext_types: Vec<JType> = ext_params.iter().map(|p| p.1.clone()).collect();
        let ext_desc = method_desc(&ext_types, &m.ret);
        let ext_name = format!("{}$extension", m.name);
        self.begin_method(true, false, &ext_params, m.ret.clone(), &[body]);
        self.m.this_slot = Some(0);
        self.m.this_underlying = Some(u.clone());
        self.code.line(cx.line_of(info.file, info.span.start));
        self.method_body(fun.sym, &fun.params, body);
        self.end_method(ACC_PUBLIC | ACC_STATIC, &ext_name, &ext_desc);
        self.begin_method(false, false, params, m.ret.clone(), &[]);
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
        self.end_method(ACC_PUBLIC, &m.name, &m.desc);
    }

    /// A method of the box of value class `c`, `name(params)`, calling the companion's
    /// `target(u, params)` on the underlying value.
    fn box_forwarder(&mut self, c: ClassId, name: &str, target: &str, params: &[(Option<SymId>, JType)], ret: &JType) {
        let u = self.vc_underlying(c);
        let module: Rc<str> = Rc::from(self.cx.companion_name(c));
        let typed: Vec<(Option<SymId>, JType)> = params.iter().map(|(_, t)| (None, t.clone())).collect();
        self.begin_method(false, false, &typed, ret.clone(), &[]);
        self.getstatic(&module, "MODULE$", &JType::L(module.clone()));
        let this = self.class_type(c);
        self.load(0, &this);
        self.vc_unbox(c, &u);
        let mut types = vec![u.clone()];
        let mut slot = 1;
        for (_, t) in params {
            self.load(slot, t);
            types.push(t.clone());
            slot += if t.wide() { 2 } else { 1 };
        }
        self.invoke_desc(Invoke::Virtual, &module, false, target, &method_desc(&types, ret), types.len(), ret);
        self.return_value(ret);
        let own: Vec<JType> = params.iter().map(|(_, t)| t.clone()).collect();
        self.end_method(ACC_PUBLIC, name, &method_desc(&own, ret));
    }

    /// A member overrides the members of the ancestors that share its name; where the erased
    /// signature differs (`compare(Int, Int)` for `compare(T, T)`) a bridge forwards to it.
    fn emit_bridges(&mut self, sym: SymId, m: &MRef) {
        self.emit_bridges_as(sym, self.cx.input.syms.sym(sym).name, m);
    }

    pub fn emit_bridges_as_pub(&mut self, sym: SymId, name: Name, m: &MRef) {
        self.emit_bridges_as(sym, name, m);
    }

    /// The bridges of `m`, which `sym` defines under the member name `name`: a var's setter is
    /// the member `x_=`.
    fn emit_bridges_as(&mut self, sym: SymId, name: Name, m: &MRef) {
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
            // An alternative of an overloaded name is bridged when it takes the parameters
            // of the inherited one as the class sees them: the types agree after erasure of
            // the class's type arguments, which the bridge's own parameters show.
            if syms.sym(inherited).alternative || syms.sym(sym).alternative {
                let overrides = cx.input.prog.overrides.get(&sym).map_or(false, |ps| ps.contains(&inherited));
                if !overrides {
                    continue;
                }
            }
            let bm = self.mref(inherited);
            if bm.name != m.name || bm.desc == m.desc || bm.params.len() != m.params.len() || self.cw.has_method(&bm.name, &bm.desc) {
                continue;
            }
            if !self.params_may_override(inherited, &bm.params, &m.params) {
                continue;
            }
            self.bridge(sym, &bm, m);
            }
        }
    }

    /// The bridge `bm` forwarding to `m`, the member `sym`'s method: each argument and the
    /// result adapted between the two erasures, through the value class where `sym` declares
    /// one (`A <: V`), which the bridge's reference holds as its box.
    fn bridge(&mut self, sym: SymId, bm: &MRef, m: &MRef) {
        let (declared, declared_ret): (Vec<TypeId>, TypeId) = match self.cx.input.syms.sym(sym).sig.as_ref() {
            Some(sig) => (sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).collect(), sig.ret),
            None => (Vec::new(), NO_TYPE),
        };
        let params: Vec<(Option<SymId>, JType)> = bm.params.iter().map(|t| (None, t.clone())).collect();
        self.begin_method(false, false, &params, bm.ret.clone(), &[]);
        self.load_this();
        let mut slot = 1u16;
        for (i, (from, to)) in bm.params.iter().zip(&m.params).enumerate() {
            self.load(slot, from);
            match declared.get(i).and_then(|&t| self.value_class_type(t)) {
                Some((c, args)) => self.adapt_vc(c, args, from.clone(), to),
                None => self.adapt(from, to),
            }
            slot += if from.wide() { 2 } else { 1 };
        }
        let owner = self.this_name.clone();
        let kind = if self.is_interface { Invoke::Interface } else { Invoke::Virtual };
        self.invoke_desc(kind, &owner, self.is_interface, &m.name, &m.desc, m.params.len(), &m.ret);
        match self.value_class_type(declared_ret) {
            Some((c, args)) if m.ret != JType::V => self.adapt_vc(c, args, m.ret.clone(), &bm.ret),
            _ => self.adapt(&m.ret, &bm.ret),
        }
        self.return_value(&bm.ret);
        self.end_method(ACC_PUBLIC | ACC_BRIDGE | ACC_SYNTHETIC, &bm.name, &bm.desc);
    }

    /// Whether a member with the parameters `own` can override `inherited`, whose erased
    /// parameters are `erased`: a parameter may erase differently only where the inherited one
    /// is abstract (a type parameter, an abstract type member), which the override fixes; a
    /// class type there makes the member an overload (`max(FiniteDuration)` beside
    /// `max(Duration)`), which scalac does not bridge.
    fn params_may_override(&self, inherited: SymId, erased: &[JType], own: &[JType]) -> bool {
        let cx = self.cx;
        let Some(sig) = cx.input.syms.sym(inherited).info.sig.as_ref() else { return true };
        let declared: Vec<TypeId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).collect();
        if declared.len() != erased.len() {
            return true;
        }
        erased.iter().zip(own).zip(&declared).all(|((e, o), &t)| {
            e == o || !matches!(cx.input.types.get(t), Type::Class(..) | Type::Union(..) | Type::Inter(..) | Type::Lit(_) | Type::Refined(..))
        })
    }

    // ---- classes ----

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
        // A library class the std's stands for (`link_library_class`) goes by the same name.
        for &(b, _) in info.base_types.iter().skip(1) {
            let bname = &cx.class_names[b.idx()];
            if syms.class(b).kind == ClassKind::Trait && syms.class(b).js == JsKind::Scala && **bname != *name && !interfaces.contains(bname) {
                interfaces.push(bname.clone());
            }
        }
        // The parents a product by rule has (`ClassInfo::is_product_by_rule`, `Desugar.classDef`).
        if info.is_product_by_rule() {
            for parent in [PRODUCT, SERIALIZABLE] {
                if !interfaces.iter().any(|i| i == parent) {
                    interfaces.push(parent.to_string());
                }
            }
        }
        // A case object is its own mirror, as scalac's: `Mirror.Singleton`.
        if is_case_object && !interfaces.iter().any(|i| i == super::callable::MIRROR_SINGLETON) {
            interfaces.push(super::callable::MIRROR_SINGLETON.to_string());
        }
        if is_enum {
            interfaces.push(ENUM.to_string());
        }
        let case_companion = info.companion.filter(|&k| is_object && cx.has_case_companion(k));
        if case_companion.map_or(false, |k| cx.has_product_mirror(k)) && !interfaces.iter().any(|i| i == super::callable::MIRROR_PRODUCT) {
            interfaces.push(super::callable::MIRROR_PRODUCT.to_string());
        }
        if is_object && info.companion.map_or(false, |e| cx.has_enum_fields(e)) && !interfaces.iter().any(|i| i == super::callable::MIRROR_SUM) {
            interfaces.push(super::callable::MIRROR_SUM.to_string());
        }
        let access = if is_trait {
            ACC_PUBLIC | ACC_INTERFACE | ACC_ABSTRACT
        } else if is_enum || is_abstract {
            ACC_PUBLIC | ACC_SUPER | ACC_ABSTRACT
        } else {
            ACC_PUBLIC | ACC_SUPER
        };
        self.cw = ClassWriter::new(access, &name, &super_name, &interfaces);

        self.this_name = name.clone();
        self.this_class = Some(c);
        self.is_interface = is_trait;
        self.abstract_class = is_enum || is_abstract;
        let path = &cx.input.sources[info.file.0 as usize].path;
        self.cw.source_file(path.rsplit(std::path::is_separator).next().unwrap_or(path));
        self.line_file = info.file;

        let n_captures = if is_anon { tc.ctor_params.len() } else { tc.captures };
        if n_captures > 0 {
            // A given class's or object's enclosing instance is scalac's field `$outer`.
            let given_class = cx.given_class_classes.contains_key(&c) || cx.inner_given_object_classes.contains_key(&c);
            for (i, s) in self.class_captured(c).into_iter().enumerate() {
                let base = encode(cx.input.interner.get(syms.sym(s).name));
                let sinfo = syms.sym(s);
                let ty = if sinfo.by_name { self.function_type(0) } else { self.sym_type(s) };
                let held = self.held(s);
                let field = if given_class && i == 0 { "$outer".to_string() } else { format!("{}$c{}", base, i) };
                self.class_captures.push((s, field, ty, held));
            }
        }

        let fields = if is_trait { Vec::new() } else { self.class_fields(tc, n_captures) };
        for f in &fields {
            let mut d = String::new();
            let stored = if f.lazy { JType::object() } else { f.ty.clone() };
            stored.desc(&mut d);
            let access = if !f.lazy && self.is_volatile(f.sym) {
                ACC_PRIVATE | ACC_VOLATILE
            } else if cx.private_params.contains_key(&f.sym) {
                ACC_PRIVATE | ACC_FINAL
            } else {
                ACC_PRIVATE
            };
            self.cw.field(access, &if f.lazy { format!("{}$lzy", f.name) } else { f.name.clone() }, &d);
        }
        for (s, field, _, _) in self.class_captures.clone() {
            let mut d = String::new();
            self.anon_capture_type(s).desc(&mut d);
            let access = if field == "$outer" { ACC_PRIVATE | ACC_FINAL | ACC_SYNTHETIC } else { ACC_PRIVATE };
            self.cw.field(access, &field, &d);
        }
        if is_object {
            self.cw.field(MODULE_FIELD, "MODULE$", &format!("L{};", name));
            self.module_clinit(tc, &fields);
        }
        // A given object of a package or of objects is scalac's module class: `<clinit>` makes
        // the instance, whose constructor runs the body, and stores it in `MODULE$`.
        if cx.given_object_classes.contains_key(&c) {
            let this = JType::L(name.clone());
            self.cw.field(MODULE_FIELD, "MODULE$", &format!("L{};", name));
            self.begin_method(true, false, &[], JType::V, &[]);
            self.new_object(&name);
            self.invoke(Invoke::Special, &name, false, "<init>", &[], &JType::V);
            self.putstatic(&name, "MODULE$", &this);
            self.return_value(&JType::V);
            self.end_method(ACC_STATIC, "<clinit>", "()V");
        }
        self.given_object_fields(&tc.init);
        if is_value_case {
            self.cw.field(ACC_PRIVATE, "$name", "Ljava/lang/String;");
            self.cw.field(ACC_PRIVATE, "$ordinal", "I");
            self.cw.field(ACC_PUBLIC | ACC_STATIC, "$instance", &format!("L{};", name));
        }
        if is_enum {
            // The values are made together, in declaration order, when the enum is first
            // touched: a read of one initialises its class, and with it this superclass. Those of
            // an enum read through its companion's fields the companion makes (`store_enum_fields`).
            let companion_makes = cx.has_enum_fields(c);
            self.begin_method(true, false, &[], JType::V, &[]);
            for &case in &info.children {
                let cinfo = syms.class(case);
                if companion_makes || cinfo.singleton.is_none() || !cx.input.reach.classes[case.idx()] {
                    continue;
                }
                let case_name = self.class_name(case);
                self.new_object(&case_name);
                self.sconst(cx.input.interner.get(cinfo.name));
                self.iconst(cinfo.ordinal as i32);
                let string = JType::L(Rc::from(STRING));
                self.invoke(Invoke::Special, &case_name, false, "<init>", &[string, JType::I], &JType::V);
                self.putstatic(&case_name, "$instance", &JType::L(case_name.clone()));
            }
            self.return_value(&JType::V);
            self.end_method(ACC_STATIC, "<clinit>", "()V");
            // What the companion calls so that the values exist before its own body runs.
            self.begin_method(true, false, &[], JType::V, &[]);
            self.return_value(&JType::V);
            self.end_method(ACC_PUBLIC | ACC_STATIC, "$touch", "()V");
        }

        if is_trait {
            self.trait_members(tc);
            self.super_accessor_declarations(c);
            if !cx.ctor_defaults_on_companion(c) {
                self.ctor_default_getters(&tc.ctor_params, &tc.ctor_defaults);
            }
        } else {
            self.constructor(tc, &fields, &super_name, enum_class.or(superclass), superclass.is_some(), is_value_case, is_object, is_anon);
            for &f in &tc.ctors {
                if cx.input.reach.funs[f.idx()] {
                    self.secondary_ctor(f);
                }
            }
            for f in fields.iter().filter(|f| !cx.private_params.contains_key(&f.sym)) {
                self.accessors(f);
            }
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
        if let Some(list) = cx.input.reach.exports.classes.get(&c) {
            self.export_forwarders(list);
        }
        self.given_class_defs(Owner::Class(c), None);
        if is_trait || is_enum || is_abstract {
            self.abstract_members(c);
        }
        if !is_trait {
            self.super_accessors(tc);
            self.inherited_export_bridges(c);
            self.emit_forwarders(tc);
            self.emit_inherited_bridges(tc);
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
            if is_case || (info.kind == ClassKind::EnumCase && info.singleton.is_none()) {
                self.selector_members(c);
            }
            if let Some(k) = case_companion {
                self.companion_members(k, Some(c));
            }
            if let Some(k) = info.companion.filter(|&k| is_object && !cx.has_case_companion(k) && cx.ctor_defaults_on_companion(k)) {
                self.ctor_default_getters_of(k);
            }
            if let Some(v) = info.companion.filter(|&v| is_object && cx.has_value_companion(v)) {
                self.extension_copies(v);
            }
            if let Some(e) = info.companion.filter(|&e| is_object && syms.class(e).kind == ClassKind::Enum) {
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
            self.jar_bridges(c);
            self.jar_trait_mixins(c);
        }
        self.drain_pending();
        self.finish_class(&name)
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
            self.cw.field(ACC_PUBLIC | ACC_STATIC | ACC_FINAL, &name, &format!("L{};", class));
        }
    }

    /// The start of a module class's `<clinit>`, as scalac's: the instance is made and stored in
    /// `MODULE$` before anything of the body runs, so the body may name the object, and the
    /// constructor has run the superclass's alone. The instance stays in a local that stands
    /// for `this` in the body, which the caller writes next.
    pub fn begin_module_clinit(&mut self, name: &Rc<str>, roots: &[TExprId]) {
        self.begin_method(true, false, &[], JType::V, roots);
        let this = JType::L(name.clone());
        self.new_object(name);
        self.invoke(Invoke::Special, name, false, "<init>", &[], &JType::V);
        self.dup();
        self.putstatic(name, "MODULE$", &this);
        let slot = self.store_new(&this);
        self.m.this_slot = Some(slot);
    }

    /// `<clinit>` of an object: the instance, then the body the object's constructor would
    /// run were it a class (the traits it adds, its fields and its statements, in order).
    fn module_clinit(&mut self, tc: &TClass, fields: &[FieldDef]) {
        let cx = self.cx;
        let name = self.this_name.clone();
        let this = JType::L(name.clone());
        let companion = cx.input.syms.class(tc.id).companion;
        let enum_values = companion.map_or(Vec::new(), |e| self.enum_value_fields(e));
        let mut roots: Vec<TExprId> = Vec::new();
        for init in &tc.init {
            init_roots(cx.input.prog, init, &mut roots);
        }
        self.begin_module_clinit(&name, &roots);
        if let Some(e) = companion.filter(|&e| cx.input.syms.class(e).kind == ClassKind::Enum && cx.input.reach.classes[e.idx()] && !cx.has_enum_fields(e)) {
            let enum_name = self.class_name(e);
            self.invoke(Invoke::Static, &enum_name, false, "$touch", &[], &JType::V);
        }
        self.store_enum_fields(&name, &enum_values);
        self.class_body(tc, fields, &this);
        self.return_value(&JType::V);
        self.end_method(ACC_STATIC, "<clinit>", "()V");
    }

    /// The body of a class as its constructor runs it after the superclass's: the initialisers
    /// of the traits it adds, its fields and its statements, in the order they are written.
    fn class_body(&mut self, tc: &TClass, fields: &[FieldDef], this: &JType) {
        let owner = self.this_name.clone();
        let slot = self.m.this_slot.unwrap_or(0);
        // The `$init$` of the Scala 2 traits the typer counts no statement of run in their place
        // among the traits', before the class's own body.
        let order: Vec<ClassId> = self.cx.input.syms.class(tc.id).base_types.iter().skip(1).rev().map(|&(b, _)| b).collect();
        let place = |b: ClassId| order.iter().position(|&x| x == b).unwrap_or(usize::MAX);
        let mut scala2 = self.scala2_trait_inits(tc).into_iter().peekable();
        for init in &tc.init {
            let before = match *init {
                TInit::Parent(b, _) => place(b),
                TInit::Field(s, _) if matches!(self.cx.input.syms.sym(s).owner, Owner::Class(t) if t != tc.id) => 0,
                _ => usize::MAX,
            };
            while let Some(b) = scala2.next_if(|&b| place(b) < before) {
                self.call_trait_init(b, slot, this);
            }
            match *init {
                TInit::Field(s, e) => {
                    let Some(f) = fields.iter().find(|f| f.sym == s) else { continue };
                    if f.lazy {
                        continue;
                    }
                    self.load(slot, this);
                    self.expr(e, &f.ty);
                    self.putfield(&owner, &f.name, &f.ty);
                }
                TInit::Stmt(e) => self.statement(e),
                TInit::Parent(b, call) => {
                    if !call.args.is_empty() && !self.trait_arguments(b, call, fields, slot, this) {
                        return;
                    }
                    if self.trait_has_init(b) {
                        self.call_trait_init(b, slot, this);
                    }
                }
            }
        }
        for b in scala2 {
            self.call_trait_init(b, slot, this);
        }
    }

    fn call_trait_init(&mut self, b: ClassId, slot: u16, this: &JType) {
        let base = self.class_name(b);
        self.load(slot, this);
        self.invoke(Invoke::Static, &base, true, "$init$", &[JType::L(base.clone())], &JType::V);
    }

    /// The Scala 2 traits a class mixes in anew whose class file declares an `$init$` the typer
    /// leaves out, least derived first: the transcoded pickle keeps no private member and no
    /// statement, so a trait whose initialisers are all private, or whose body is statements
    /// alone, counts none, where dotty's `Mixin` calls the `$init$` of every trait but a
    /// `NoInits` one (`superCallOpt`).
    fn scala2_trait_inits(&self, tc: &TClass) -> Vec<ClassId> {
        let cx = self.cx;
        let syms = cx.input.syms;
        let inherited = self.traits_implemented_above(tc.id);
        let mut out = Vec::new();
        for &(b, _) in syms.class(tc.id).base_types.iter().skip(1).rev() {
            if syms.class(b).kind != ClassKind::Trait || inherited.contains(&b) || cx.tclass_of[b.idx()] != u32::MAX {
                continue;
            }
            if tc.init.iter().any(|i| matches!(*i, TInit::Parent(t, _) if t == b)) {
                continue;
            }
            let Some(file) = cx.class_files.get(&b) else { continue };
            if cx.scala2_pickles.contains_key(&b) && file.methods.iter().any(|m| m.name == "$init$" && m.access & ACC_STATIC != 0) {
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

    /// What a class passes to the parameters of a trait it mixes in anew, before the trait's
    /// `$init$` runs, as dotty's `Mixin` writes it (`transformConstructor`, `traitInits`): the
    /// statements of named or default arguments first, then each argument stored into the
    /// field of its parameter, or evaluated and dropped where the class or a trait further down
    /// overrides the parameter's accessor (`Mixin` 301-303, so `class C extends T(side()) {
    /// override val x = 9 }` runs `side()` and T's body reads `x` as 0).
    fn trait_arguments(&mut self, b: ClassId, call: ParentCall, fields: &[FieldDef], slot: u16, this: &JType) -> bool {
        let cx = self.cx;
        let prog = cx.input.prog;
        let owner = self.this_name.clone();
        let params = self.trait_params(b);
        let args = prog.expr_list(call.args).to_vec();
        if params.len() != args.len() {
            self.unsupported("the arguments of a trait that its parameters do not match");
            return false;
        }
        for s in prog.stmts[call.prelude.range()].to_vec() {
            match s {
                TStmt::Expr(x) => self.statement(x),
                TStmt::Val(sym, init) => self.local_val(sym, init),
                _ => {}
            }
        }
        let flags: Vec<bool> = cx.input.syms.class(b).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.has_default)).collect();
        let defaulted = args.iter().enumerate().any(|(i, &a)| flags.get(i) == Some(&true) && matches!(prog.expr(a), TExpr::Unit));
        if defaulted {
            // A left-out argument is its default getter's, of the trait's companion as scalac
            // lays it out, called after the written arguments as a class's constructor's are.
            let types = self.ctor_param_types(b);
            self.ctor_arguments(b, call.args, &types);
            let mut slots = vec![0u16; types.len()];
            for i in (0..types.len()).rev() {
                slots[i] = self.store_new(&types[i]);
            }
            for (i, p) in params.into_iter().enumerate() {
                let Some(f) = fields.iter().find(|f| f.sym == p) else { continue };
                self.load(slot, this);
                self.load(slots[i], &types[i]);
                self.adapt(&types[i], &f.ty);
                self.putfield(&owner, &f.name, &f.ty);
            }
            return true;
        }
        for (p, arg) in params.into_iter().zip(args) {
            match fields.iter().find(|f| f.sym == p) {
                Some(f) => {
                    self.load(slot, this);
                    self.expr(arg, &f.ty);
                    self.putfield(&owner, &f.name, &f.ty);
                }
                None => self.statement(arg),
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
            out.push(FieldDef { sym: p, name, ty, lazy: false, lazy_owner: None, init: None, mutable: syms.sym(p).kind == SymKind::Var });
        }
        for init in &tc.init {
            let TInit::Field(s, e) = *init else { continue };
            // A given object of an object is its own module class, of no field here, and a given
            // class without parameter clauses (`given f[A]: F[A] with`) is scalac's def making one.
            if out.iter().any(|f| f.sym == s) || cx.given_objects.contains_key(&s) || cx.given_classes.contains_key(&s) {
                continue;
            }
            let ty = self.sym_type(s);
            let name = self.field_name(s);
            let lazy = self.is_lazy_member(s);
            out.push(FieldDef { sym: s, name, ty, lazy, lazy_owner: None, init: Some(e), mutable: syms.sym(s).kind == SymKind::Var });
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
                out.push(FieldDef { sym: p, name, ty, lazy: false, lazy_owner: None, init: None, mutable: syms.sym(p).kind == SymKind::Var });
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
                if self.is_trait_constant(s, e) {
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
                    out.push(FieldDef { sym: s, name, ty, lazy: true, lazy_owner: Some(b), init: Some(e), mutable: false });
                } else {
                    out.push(FieldDef { sym: s, name, ty, lazy: false, lazy_owner: None, init: None, mutable: syms.sym(s).kind == SymKind::Var });
                }
            }
        }
        out
    }

    fn constructor(
        &mut self,
        tc: &TClass,
        fields: &[FieldDef],
        super_name: &str,
        enum_class: Option<ClassId>,
        fields_first: bool,
        is_value_case: bool,
        is_object: bool,
        is_anon: bool,
    ) {
        let cx = self.cx;
        let mut params: Vec<(Option<SymId>, JType)> = Vec::new();
        // An enum over `java.lang.Enum` passes the name and ordinal of its values up to it.
        let java_enum = cx.is_java_enum(tc.id) || (is_value_case && cx.enum_of_case(tc.id).map_or(false, |e| cx.is_java_enum(e)));
        if is_value_case || java_enum {
            params.push((None, JType::L(Rc::from(STRING))));
            params.push((None, JType::I));
        }
        let n_captures = if is_anon { tc.ctor_params.len() } else { tc.captures };
        for t in self.anon_ctor_params(tc.id) {
            params.push((None, t));
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
        let mut roots: Vec<TExprId> = Vec::new();
        for init in tc.init.iter().filter(|_| !is_object) {
            init_roots(cx.input.prog, init, &mut roots);
        }
        if let Some(args) = tc.parent_args.filter(|_| !is_anon) {
            roots.extend_from_slice(cx.input.prog.expr_list(args));
        }
        self.begin_method(false, true, &params, JType::V, &roots);
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
                Held::Thunk => {}
            }
            slot += if stored.wide() { 2 } else { 1 };
        }
        let this = JType::L(self.this_name.clone());
        let owner = self.this_name.clone();
        if is_value_case {
            // Before the enum's constructor runs, whose body may print the value.
            let string = JType::L(Rc::from(STRING));
            self.load(0, &this);
            self.load(1, &string);
            self.putfield(&owner, "$name", &string);
            self.load(0, &this);
            self.load(2, &JType::I);
            self.putfield(&owner, "$ordinal", &JType::I);
        }
        // Scala sets the fields a class takes as parameters before the constructor of its
        // superclass runs, whose body may read them through an overridden member.
        if fields_first && !is_anon {
            for &p in &tc.ctor_params {
                let Some(f) = fields.iter().find(|f| f.sym == p) else { continue };
                let local = self.m.locals[&p].clone();
                self.load(0, &this);
                self.load(local.slot, &local.ty);
                self.putfield(&owner, &f.name, &f.ty);
            }
        }
        for s in cx.input.prog.stmts[tc.parent_prelude.range()].to_vec() {
            match s {
                TStmt::Expr(x) => self.statement(x),
                TStmt::Val(sym, init) => self.local_val(sym, init),
                _ => {}
            }
        }
        self.load(0, &this);
        let name_ordinal = if java_enum { vec![JType::L(Rc::from(STRING)), JType::I] } else { Vec::new() };
        if java_enum {
            self.load(1, &name_ordinal[0]);
            self.load(2, &JType::I);
        }
        let super_params = match enum_class {
            Some(_) if anon_super.is_some() => {
                let (_, _, types) = anon_super.unwrap();
                let mut slot: u16 = params[..first_super_arg].iter().map(|(_, t)| if t.wide() { 2 } else { 1 }).sum::<u16>() + 1;
                for t in &types {
                    self.load(slot, t);
                    slot += if t.wide() { 2 } else { 1 };
                }
                types
            }
            // A parent call that goes to a secondary constructor of the superclass, the
            // enclosing instance of an inner superclass first as a secondary `new` passes it
            // (`ExplicitOuter.OuterOps.args` for `New`, `Super` and `This` alike).
            Some(e) if tc.parent_via.is_some() && !is_anon => {
                let via = tc.parent_via.unwrap();
                let args = tc.parent_args.unwrap_or(crate::ast::ListRef::EMPTY);
                self.m.before_super = true;
                let types = self.via_arguments(e, via, args);
                self.m.before_super = false;
                types
            }
            // A superclass nested in a class or local takes what it captures first.
            Some(e) if tc.parent_args.is_some() && self.capture_count(e) > 0 && !is_anon => {
                self.m.before_super = true;
                let types = self.new_arguments(e, tc.parent_args.unwrap());
                self.m.before_super = false;
                types
            }
            Some(e) => {
                let types = self.ctor_param_types(e);
                match tc.parent_args {
                    Some(args) => {
                        self.m.before_super = true;
                        self.super_arguments(e, args, &types);
                        self.m.before_super = false;
                    }
                    None if types.is_empty() => {}
                    // Every parameter of the superclass has a default.
                    None if cx.input.syms.class(e).ctor.iter().all(|cl| cl.params.iter().all(|p| p.has_default)) => {
                        let none = crate::ast::ListRef::EMPTY;
                        self.m.before_super = true;
                        self.super_arguments(e, none, &types);
                        self.m.before_super = false;
                    }
                    None => {
                        self.unsupported("an enum case without the arguments of its enum");
                        return;
                    }
                }
                types
            }
            None => Vec::new(),
        };
        let super_params = [name_ordinal, super_params].concat();
        self.invoke(Invoke::Special, super_name, false, "<init>", &super_params, &JType::V);
        if n_captures > 0 {
            let mut slot = 1u16;
            for (s, field, _, _) in self.class_captures.clone() {
                let t = self.anon_capture_type(s);
                self.load(0, &this);
                self.load(slot, &t);
                self.putfield(&owner, &field, &t);
                slot += if t.wide() { 2 } else { 1 };
            }
        }
        if !is_anon && !fields_first {
            for &p in &tc.ctor_params {
                let Some(f) = fields.iter().find(|f| f.sym == p) else { continue };
                let local = self.m.locals[&p].clone();
                self.load(0, &this);
                self.load(local.slot, &local.ty);
                self.putfield(&owner, &f.name, &f.ty);
            }
        }
        // An object's body runs in its `<clinit>`, once `MODULE$` holds the instance.
        if !is_object {
            self.class_body(tc, fields, &this);
        }
        self.return_value(&JType::V);
        let types: Vec<JType> = params.iter().map(|p| p.1.clone()).collect();
        // A static module is made by its own `<clinit>` only, as scalac's.
        let ctor_access = if is_object && cx.statically_placed(tc.id) && cx.input.syms.class(tc.id).local_module.is_none() { ACC_PRIVATE } else { ACC_PUBLIC };
        self.end_method(ctor_access, "<init>", &method_desc(&types, &JType::V));

        if !cx.ctor_defaults_on_companion(tc.id) {
            self.ctor_default_getters(&tc.ctor_params, &tc.ctor_defaults);
        }
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
        let params: Vec<(Option<SymId>, JType)> = fun.params.iter().map(|&s| (Some(s), self.sym_type(s))).collect();
        self.context = format!("{}.<init>", self.this_name);
        self.begin_method(false, true, &params, JType::V, &[body]);
        let owner = self.this_name.clone();
        let this = JType::L(owner.clone());
        for s in prelude {
            match *s {
                TStmt::Expr(x) => self.statement(x),
                TStmt::Val(sym, init) => self.local_val(sym, init),
                _ => {}
            }
        }
        self.load(0, &this);
        self.m.before_super = true;
        let types = match prog.expr(call) {
            // The primary constructor, its captures (the enclosing instance of a class nested
            // in a class, the secondary's own outer parameter) first.
            TExpr::New(c, args) => self.new_arguments(c, args),
            TExpr::NewVia(s, args) => {
                let Owner::Class(c) = cx.input.syms.sym(s).owner else { return };
                self.via_arguments(c, s, args)
            }
            _ => return,
        };
        self.m.before_super = false;
        self.invoke(Invoke::Special, &owner, false, "<init>", &types, &JType::V);
        self.statement(after);
        self.return_value(&JType::V);
        let types: Vec<JType> = params.iter().map(|p| p.1.clone()).collect();
        self.end_method(ACC_PUBLIC, "<init>", &method_desc(&types, &JType::V));
        if !self.this_class.map_or(false, |c| cx.ctor_defaults_on_companion(c)) {
            self.ctor_default_getters(&fun.params, &fun.defaults);
        }
    }

    /// The defaults of a constructor of a class whose companion is not static (an inner or a
    /// local class), as static getters of the class that take the parameters before.
    fn ctor_default_getters(&mut self, params: &[SymId], defaults: &[Option<TExprId>]) {
        for (i, d) in defaults.iter().enumerate() {
            let Some(d) = *d else { continue };
            let before: Vec<(Option<SymId>, JType)> = params[..i].iter().map(|&p| (Some(p), self.sym_type(p))).collect();
            let ret = self.sym_type(params[i]);
            self.begin_method(true, false, &before, ret.clone(), &[d]);
            self.expr(d, &ret);
            self.return_value(&ret);
            let types: Vec<JType> = before.iter().map(|p| p.1.clone()).collect();
            self.end_method(ACC_PUBLIC | ACC_STATIC, &format!("$lessinit$greater$default${}", i + 1), &method_desc(&types, &ret));
        }
    }

    fn accessors(&mut self, f: &FieldDef) {
        let owner = self.this_name.clone();
        let this = JType::L(owner.clone());
        let getter_desc = method_desc(&[], &f.ty);
        self.begin_method(false, false, &[], f.ty.clone(), &f.init.filter(|_| f.lazy && f.lazy_owner.is_none()).into_iter().collect::<Vec<_>>());
        if f.lazy {
            let init = match (f.lazy_owner, f.init) {
                (Some(t), _) => HolderInit::TraitStatic(self.class_name(t)),
                (None, Some(init)) => HolderInit::Expr(init),
                (None, None) => HolderInit::Unit,
            };
            self.lazy_holder_body(&f.name, &f.ty, init);
        } else {
            self.load(0, &this);
            self.getfield(&owner, &f.name, &f.ty);
        }
        self.return_value(&f.ty);
        // A trait's final val, object and given are final in the class that holds them, as
        // scalac's `Mixin` copies the member's flags (`mkForwarderSym`); a using parameter's is
        // not, as the parameter is no given definition.
        let info = self.cx.input.syms.sym(f.sym);
        let from_trait = matches!(info.owner, Owner::Class(t) if self.cx.input.syms.class(t).kind == ClassKind::Trait);
        let param = matches!(info.owner, Owner::Class(t) if self.trait_params(t).contains(&f.sym));
        let fin = info.mods & mods::FINAL != 0 || (!param && (info.mods & mods::GIVEN != 0 || info.kind == SymKind::Given));
        // So is the implementation of a deferred given, as dotty's `implementDeferredGivens` makes it,
        // and the getter of a class's given object, scalac's module.
        let module = self.cx.inner_given_objects.contains_key(&f.sym);
        let access = if (from_trait && fin) || module || self.implements_deferred(f.sym) { ACC_PUBLIC | ACC_FINAL } else { ACC_PUBLIC };
        self.end_method(access, &f.name, &getter_desc);
        let getter = MRef {
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
        if f.mutable {
            self.begin_method(false, false, &[(None, f.ty.clone())], JType::V, &[]);
            self.load(0, &this);
            self.load(1, &f.ty);
            self.putfield(&owner, &f.name, &f.ty);
            self.return_value(&JType::V);
            let setter = MRef {
                owner: owner.clone(),
                owner_is_interface: false,
                name: format!("{}_$eq", f.name),
                params: vec![f.ty.clone()],
                ret: JType::V,
                desc: method_desc(&[f.ty.clone()], &JType::V),
                kind: Invoke::Virtual,
            };
            self.end_method(ACC_PUBLIC, &setter.name, &setter.desc);
            let setter_name = setter_member_name(self.cx.input.interner, self.cx.input.syms.sym(f.sym).name);
            if let Some(name) = setter_name.filter(|_| typed) {
                self.emit_bridges_as(f.sym, name, &setter);
            }
        }
    }

    /// The body of a lazy member's getter over teq's holder, the field `x$lzy` of the class
    /// being written (an `Object`, null until the value is computed): computed once by `init`,
    /// then read. Not safe across threads. A computed null is held as scala-library's
    /// `LazyVals$NullValue$`, as scalac's holders hold it, so that it is computed once too (the
    /// read that computes it answers the value itself, a later one the holder's, as scalac's
    /// `mkThreadSafeDef`); an initialiser that throws leaves the holder empty.
    fn lazy_holder_body(&mut self, name: &str, ty: &JType, init: HolderInit) {
        let owner = self.this_name.clone();
        let this = JType::L(owner.clone());
        let object = JType::object();
        let field = format!("{}$lzy", name);
        let null_once = matches!(ty, JType::L(_));
        let ready = self.code.new_label();
        let done = self.code.new_label();
        self.load(0, &this);
        self.getfield(&owner, &field, &object);
        self.dup();
        self.jump_if(op::IFNONNULL, 1, ready);
        self.pop_value(&object);
        match init {
            HolderInit::TraitStatic(base) => {
                self.load(0, &this);
                self.invoke(Invoke::Static, &base, true, &format!("{}$", name), &[JType::L(base.clone())], ty);
            }
            HolderInit::Expr(init) => self.expr(init, ty),
            HolderInit::Unit => self.adapt(&JType::V, ty),
            HolderInit::Module { class, outer } => {
                self.new_object(&class);
                self.load(0, &this);
                self.invoke(Invoke::Special, &class, false, "<init>", &[JType::L(outer)], &JType::V);
            }
        }
        self.adapt(ty, &object);
        let vt = self.vt(&object);
        self.code.retype_top(vt);
        let value = self.store_new(&object);
        self.load(0, &this);
        self.load(value, &object);
        if null_once {
            self.null_to_sentinel();
        }
        self.putfield(&owner, &field, &object);
        self.load(value, &object);
        self.goto(done);
        self.code.bind(ready);
        if null_once {
            self.sentinel_to_null();
        }
        self.code.bind(done);
        self.adapt(&object, ty);
    }

    /// The value on the stack, about to be stored in a lazy holder, with null replaced by
    /// `LazyVals$NullValue$`: the holder's null stands for a value not computed yet.
    fn null_to_sentinel(&mut self) {
        let object = JType::object();
        let null_value = JType::L(Rc::from(NULL_VALUE));
        let vt = self.vt(&object);
        self.code.retype_top(vt);
        let computed = self.code.new_label();
        self.dup();
        self.jump_if(op::IFNONNULL, 1, computed);
        self.pop_value(&object);
        self.getstatic(NULL_VALUE, "MODULE$", &null_value);
        self.code.bind(computed);
    }

    /// The value on the stack, read from a computed lazy holder, with `LazyVals$NullValue$`
    /// replaced by the null it stands for.
    fn sentinel_to_null(&mut self) {
        let object = JType::object();
        let null_value = JType::L(Rc::from(NULL_VALUE));
        let held = self.code.new_label();
        self.dup();
        self.getstatic(NULL_VALUE, "MODULE$", &null_value);
        self.jump_if(op::IF_ACMPNE, 2, held);
        self.pop_value(&object);
        self.code.op(op::ACONST_NULL);
        self.code.push(VT::Null);
        self.code.bind(held);
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
        let this = JType::L(self.this_name.clone());
        let owner = self.this_name.clone();
        for &p in &tc.ctor_params {
            let ty = self.sym_type(p);
            let name = self.field_name(p);
            self.cw.method(ACC_PUBLIC | ACC_ABSTRACT, &name, &method_desc(&[], &ty), None);
            if cx.input.syms.sym(p).kind == SymKind::Var {
                self.cw.method(ACC_PUBLIC | ACC_ABSTRACT, &format!("{}_$eq", name), &method_desc(&[ty], &JType::V), None);
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
                    self.begin_method(false, false, &[], ty.clone(), &[e]);
                    self.expr(e, &ty);
                    self.return_value(&ty);
                    self.end_method(ACC_PUBLIC, &name, &desc);
                    self.begin_method(true, false, &[(None, this.clone())], ty.clone(), &[]);
                    self.load(0, &this);
                    self.invoke(Invoke::Special, &owner, true, &name, &[], &ty);
                    self.return_value(&ty);
                    self.end_method(ACC_PUBLIC | ACC_STATIC, &format!("{}$", name), &method_desc(&[this.clone()], &ty));
                }
                TInit::Field(s, e) if self.is_trait_constant(s, e) => {
                    let ty = self.sym_type(s);
                    let name = self.field_name(s);
                    self.begin_method(false, false, &[], ty.clone(), &[e]);
                    self.expr(e, &ty);
                    self.return_value(&ty);
                    self.end_method(ACC_PUBLIC, &name, &method_desc(&[], &ty));
                    self.trait_static_forwarder(&name, &[], &ty);
                }
                TInit::Field(s, e) => {
                    let ty = self.sym_type(s);
                    let name = self.field_name(s);
                    let desc = method_desc(&[], &ty);
                    if !self.cw.has_method(&name, &desc) {
                        self.cw.method(ACC_PUBLIC | ACC_ABSTRACT, &name, &desc, None);
                    }
                    let setter = self.trait_setter_name(s);
                    let setter_desc = method_desc(&[ty], &JType::V);
                    if !self.cw.has_method(&setter, &setter_desc) {
                        self.cw.method(ACC_PUBLIC | ACC_ABSTRACT, &setter, &setter_desc, None);
                    }
                    runs.push((Some(s), e));
                }
                TInit::Stmt(e) => runs.push((None, e)),
                TInit::Parent(..) => {}
            }
        }
        if self.source_trait_has_init(tc) {
            let roots: Vec<TExprId> = runs.iter().map(|r| r.1).collect();
            self.begin_method(true, false, &[(None, this.clone())], JType::V, &roots);
            self.m.this_slot = Some(0);
            for (field, e) in runs {
                match field {
                    Some(s) => {
                        let ty = self.sym_type(s);
                        let setter = self.trait_setter_name(s);
                        self.load(0, &this);
                        self.expr(e, &ty);
                        self.invoke(Invoke::Interface, &owner, true, &setter, &[ty], &JType::V);
                    }
                    None => self.statement(e),
                }
            }
            self.return_value(&JType::V);
            self.end_method(ACC_PUBLIC | ACC_STATIC, "$init$", &method_desc(&[this], &JType::V));
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
        let owner = self.this_name.clone();
        let this = JType::L(owner.clone());
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
            let traits_fields: Vec<(SymId, Option<JType>)> = if idx == u32::MAX {
                self.class_file_trait_fields(b).fields.iter().map(|f| (f.sym, Some(f.ty.clone()))).collect()
            } else {
                cx.input.prog.classes[idx as usize].init.iter().filter_map(|init| match *init {
                    TInit::Field(s, e) if !self.is_trait_constant(s, e) => Some((s, None)),
                    _ => None,
                }).collect()
            };
            for (s, declared) in traits_fields {
                if self.is_lazy_member(s) || syms.sym(s).kind == SymKind::Var {
                    continue;
                }
                let ty = declared.unwrap_or_else(|| self.sym_type(s));
                let setter = self.trait_setter_name(s);
                let desc = method_desc(&[ty.clone()], &JType::V);
                if self.cw.has_method(&setter, &desc) {
                    continue;
                }
                // scalac's `Memoize` (156-181) stores into the class's field of the member's name
                // where one exists when `Mixin` writes the setter: a val of the class's body, or
                // the field of this trait's val or of a trait before it in the order the class
                // mixes them in (`transformTemplate`, 345-354), so that the trait's `$init$` reads
                // its value; nothing where the class's member is a lazy val or a constructor
                // parameter (163-168), or a trait after this one holds the field.
                let name = self.field_name(s);
                let held = by_name.get(name.as_str()).map(|&i| &fields[i]).filter(|f| !f.lazy && !params.contains(&f.sym)).filter(|f| match syms.sym(f.sym).owner {
                    Owner::Class(k) if k == c => true,
                    Owner::Class(t) => position(t) >= position(b),
                    _ => false,
                });
                self.begin_method(false, false, &[(None, ty.clone())], JType::V, &[]);
                if let Some(f) = held {
                    self.load(0, &this);
                    self.load(1, &ty);
                    self.adapt(&ty, &f.ty);
                    self.putfield(&owner, &f.name, &f.ty);
                }
                self.return_value(&JType::V);
                self.end_method(ACC_PUBLIC, &setter, &desc);
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
            out.push(FieldDef { sym: s, name: name.to_string(), ty, lazy, lazy_owner: lazy.then_some(b), init: None, mutable });
        }
        // A val whose getter the trait defines, of no setter, is a constant (`is_trait_constant`).
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
    fn class_file_only_members(&mut self, b: ClassId, file: &crate::classfile::ClassFile, members: &FxMap<String, SymId>) -> (Vec<(String, JType)>, Vec<(String, JType, HolderInit)>) {
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
                if !x.starts_with(&expanded) || members.contains_key(x) || !m.descriptor.starts_with(&this_desc) || ret == JType::V || !lazies.iter().any(|l| l == x) {
                    continue;
                }
                if declares(x, &method_desc(&[], &ret), false) {
                    holders.push((x.to_string(), ret, HolderInit::TraitStatic(binary.clone())));
                }
            } else if m.access & ACC_ABSTRACT != 0 {
                if let Some(x) = m.name.strip_suffix("_$eq") {
                    let [ty] = params.as_slice() else { continue };
                    if x.starts_with(&expanded) && !members.contains_key(x) && ret == JType::V && declares(x, &method_desc(&[], ty), true) {
                        vars.push((x.to_string(), ty.clone()));
                    }
                    continue;
                }
                let JType::L(class) = &ret else { continue };
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
    /// `$init$` stores their initial values (`scala2_trait_inits`).
    fn class_file_storage(&mut self, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let owner = self.this_name.clone();
        let this = JType::L(owner.clone());
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
                if params.len() != 1 || ret != JType::V || self.cw.has_method(setter, desc) {
                    continue;
                }
                let ty = params[0].clone();
                let overridden = self.cw.has_method(field, &method_desc(&[], &ty));
                if !overridden {
                    self.plain_field(field, &ty);
                }
                self.begin_method(false, false, &[(None, ty.clone())], JType::V, &[]);
                if !overridden {
                    self.load(0, &this);
                    self.load(1, &ty);
                    self.putfield(&owner, field, &ty);
                }
                self.return_value(&JType::V);
                self.end_method(ACC_PUBLIC, setter, desc);
            }
            for (field, ty) in &read.vars {
                let setter_desc = method_desc(&[ty.clone()], &JType::V);
                if self.cw.has_method(field, &method_desc(&[], ty)) || self.cw.has_method(&format!("{}_$eq", field), &setter_desc) {
                    continue;
                }
                self.plain_field(field, ty);
                self.begin_method(false, false, &[(None, ty.clone())], JType::V, &[]);
                self.load(0, &this);
                self.load(1, ty);
                self.putfield(&owner, field, ty);
                self.return_value(&JType::V);
                self.end_method(ACC_PUBLIC, &format!("{}_$eq", field), &setter_desc);
            }
            for (name, ty, init) in &read.holders {
                let getter_desc = method_desc(&[], ty);
                if self.cw.has_method(name, &getter_desc) {
                    continue;
                }
                self.cw.field(ACC_PRIVATE, &format!("{}$lzy", name), "Ljava/lang/Object;");
                self.begin_method(false, false, &[], ty.clone(), &[]);
                self.lazy_holder_body(name, ty, init.clone());
                self.return_value(ty);
                self.end_method(ACC_PUBLIC, name, &getter_desc);
            }
        }
    }

    /// A private field of the class being written and its public getter.
    fn plain_field(&mut self, field: &str, ty: &JType) {
        let owner = self.this_name.clone();
        let mut d = String::new();
        ty.desc(&mut d);
        self.cw.field(ACC_PRIVATE, field, &d);
        self.begin_method(false, false, &[], ty.clone(), &[]);
        self.load(0, &JType::L(owner.clone()));
        self.getfield(&owner, field, ty);
        self.return_value(ty);
        self.end_method(ACC_PUBLIC, field, &method_desc(&[], ty));
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
            if !self.cw.has_method(&m.name, &m.desc) {
                self.cw.method(ACC_PUBLIC | ACC_ABSTRACT, &m.name, &m.desc, None);
            }
        }
    }

    /// Inherited alternatives of an overloaded name that implement an inherited declaration
    /// with another erased signature (`TClass::bridges`).
    fn emit_inherited_bridges(&mut self, tc: &TClass) {
        for &(declared, implementation) in &tc.bridges {
            let bm = self.mref(declared);
            let m = self.mref(implementation);
            if bm.desc == m.desc || bm.params.len() != m.params.len() || self.cw.has_method(&bm.name, &bm.desc) {
                continue;
            }
            self.bridge(implementation, &bm, &m);
        }
    }

    /// In link mode, the bridges scalac's mixin phase puts in a class for the jar traits it
    /// extends: a jar method left abstract under a descriptor nothing the class inherits
    /// implements, where a method of its name and arity does, forwards to that method
    /// (`PartialOrdering.tryCompare` returning `Option` beside `Ordering`'s default returning
    /// `Some`, `IterableOnceOps.filter` returning `Object` beside `Iterator`'s).
    fn jar_bridges(&mut self, c: ClassId) {
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
            if self.cw.has_method(&name, &desc) {
                continue;
            }
            let (params, ret) = parse_method_desc(&desc);
            let target = implemented
                .iter()
                .filter(|(n, d)| *n == name && parse_method_desc(d).0.len() == params.len())
                .max_by_key(|(_, d)| parse_method_desc(d).0.iter().zip(&params).filter(|(a, b)| a == b).count())
                .cloned();
            let Some((_, target_desc)) = target else { continue };
            let (target_params, target_ret) = parse_method_desc(&target_desc);
            let locals: Vec<(Option<SymId>, JType)> = params.iter().map(|t| (None, t.clone())).collect();
            self.begin_method(false, false, &locals, ret.clone(), &[]);
            self.load_this();
            let mut slot = 1u16;
            for (from, to) in params.iter().zip(&target_params) {
                self.load(slot, from);
                self.adapt(from, to);
                slot += if from.wide() { 2 } else { 1 };
            }
            let owner = self.this_name.clone();
            self.invoke_desc(Invoke::Virtual, &owner, false, &name, &target_desc, target_params.len(), &target_ret);
            self.adapt(&target_ret, &ret);
            self.return_value(&ret);
            self.end_method(ACC_PUBLIC | ACC_BRIDGE | ACC_SYNTHETIC, &name, &desc);
        }
    }

    /// Members that an export clause or a nested object implements for a parent trait.
    fn emit_forwarders(&mut self, tc: &TClass) {
        let cx = self.cx;
        let syms = cx.input.syms;
        for &(member, target) in &tc.forwarders {
            let m = self.mref(member);
            let target_is_def = syms.sym(target).kind == SymKind::Def;
            let params: Vec<(Option<SymId>, JType)> = m.params.iter().map(|t| (None, t.clone())).collect();
            self.begin_method(false, false, &params, m.ret.clone(), &[]);
            let t = if target_is_def {
                let tm = self.mref(target);
                match syms.sym(target).owner {
                    Owner::Class(o) => {
                        self.load_module(o);
                    }
                    _ => {
                        let module = cx.file_modules[syms.sym(target).file.0 as usize].clone();
                        self.getstatic(&module, "MODULE$", &JType::L(Rc::from(module.as_str())));
                    }
                }
                let mut slot = 1u16;
                for (from, to) in m.params.iter().zip(&tm.params) {
                    self.load(slot, from);
                    self.adapt(from, to);
                    slot += if from.wide() { 2 } else { 1 };
                }
                self.invoke_mref(&tm);
                tm.ret.clone()
            } else {
                self.static_value(target)
            };
            self.adapt(&t, &m.ret);
            self.return_value(&m.ret);
            self.end_method(ACC_PUBLIC, &m.name, &m.desc);
        }
        for (declared, nested) in crate::emit::implemented_nested_objects(syms, tc.id) {
            let m = self.mref(declared);
            self.begin_method(false, false, &[], m.ret.clone(), &[]);
            let t = self.load_module(nested);
            self.adapt(&t, &m.ret);
            self.return_value(&m.ret);
            self.end_method(ACC_PUBLIC, &m.name, &m.desc);
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
    fn mixin_forwarder(&mut self, m: &MRef, im: &MRef) {
        if self.cw.has_method(&m.name, &m.desc) {
            return;
        }
        let params: Vec<(Option<SymId>, JType)> = m.params.iter().map(|t| (None, t.clone())).collect();
        self.begin_method(false, false, &params, m.ret.clone(), &[]);
        self.load_this();
        let mut slot = 1u16;
        for (from, to) in m.params.iter().zip(&im.params) {
            self.load(slot, from);
            self.adapt(from, to);
            slot += if from.wide() { 2 } else { 1 };
        }
        self.invoke_desc(Invoke::Special, &im.owner, true, &im.name, &im.desc, im.params.len(), &im.ret);
        self.adapt(&im.ret, &m.ret);
        self.return_value(&m.ret);
        self.end_method(ACC_PUBLIC | ACC_SYNTHETIC, &m.name, &m.desc);
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
                if sinfo.kind != SymKind::Def || !implemented(cx, s) || done.contains(&sinfo.name) || defines(cx, &info, sinfo.name) {
                    continue;
                }
                if !inherited.iter().any(|&k| syms.class(k).members.contains_key(&sinfo.name)) {
                    continue;
                }
                done.push(sinfo.name);
                let m = self.mref(s);
                if self.cw.has_method(&m.name, &m.desc) {
                    continue;
                }
                let params: Vec<(Option<SymId>, JType)> = m.params.iter().map(|t| (None, t.clone())).collect();
                self.begin_method(false, false, &params, m.ret.clone(), &[]);
                self.load_this();
                let mut slot = 1u16;
                for t in &m.params {
                    self.load(slot, t);
                    slot += if t.wide() { 2 } else { 1 };
                }
                self.invoke_desc(Invoke::Special, &m.owner, true, &m.name, &m.desc, m.params.len(), &m.ret);
                self.return_value(&m.ret);
                self.end_method(ACC_PUBLIC | ACC_SYNTHETIC, &m.name, &m.desc);
            }
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
        let mut implemented: Vec<Rc<MRef>> = Vec::new();
        for &k in &inherited {
            // A class read from a class file implements what the file defines, whichever of its
            // members' signatures the build has read (`jar_bridges` reads them so too).
            if let Some(cf) = cx.class_files.get(&k) {
                for m in cf.methods.iter().filter(|m| m.access & (ACC_ABSTRACT | ACC_STATIC | ACC_PRIVATE | ACC_BRIDGE) == 0 && !m.name.starts_with('<')) {
                    let (params, ret) = parse_method_desc(&m.descriptor);
                    let owner: Rc<str> = Rc::from(cf.name.as_str());
                    implemented.push(Rc::new(MRef { owner, owner_is_interface: cf.is_interface(), name: m.name.clone(), params, ret, desc: m.descriptor.clone(), kind: Invoke::Virtual }));
                }
                continue;
            }
            for &s in &syms.class(k).member_order {
                if syms.sym(s).owner != Owner::Class(k) || !concrete(s) {
                    continue;
                }
                let m = self.mref(s);
                if syms.sym(s).kind == SymKind::Var {
                    let desc = method_desc(&[m.ret.clone()], &JType::V);
                    implemented.push(Rc::new(MRef { name: format!("{}_$eq", m.name), params: vec![m.ret.clone()], ret: JType::V, desc, ..(*m).clone() }));
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
                if self.cw.has_method(&declared.name, &declared.desc) || implemented.iter().any(|m| m.name == declared.name && m.desc == declared.desc) {
                    continue;
                }
                let Some(target) = implemented.iter().find(|m| m.name == declared.name && m.params.len() == declared.params.len()).cloned() else { continue };
                let params: Vec<(Option<SymId>, JType)> = declared.params.iter().map(|t| (None, t.clone())).collect();
                self.begin_method(false, false, &params, declared.ret.clone(), &[]);
                self.load_this();
                let mut slot = 1u16;
                for (from, to) in declared.params.iter().zip(&target.params) {
                    self.load(slot, from);
                    self.adapt(from, to);
                    slot += if from.wide() { 2 } else { 1 };
                }
                let owner = self.this_name.clone();
                self.invoke_desc(Invoke::Virtual, &owner, false, &target.name, &target.desc, target.params.len(), &target.ret);
                self.adapt(&target.ret, &declared.ret);
                self.return_value(&declared.ret);
                self.end_method(ACC_PUBLIC | ACC_BRIDGE | ACC_SYNTHETIC, &declared.name, &declared.desc);
            }
        }
    }

    /// `toString`, `hashCode` and `equals` of `Object` win over the default methods of an
    /// interface, so a class takes the ones its traits define through a forwarder (scalac's
    /// `Mixin.mixinForwarders` by `MixinOps.needsMixinForwarder`): where the first of its
    /// linearisation to define one is a trait, of the program or of a jar, that its superclass
    /// does not mix in already (the superclass's forwarder is inherited, and so is a class's
    /// own method).
    fn object_method_forwarders(&mut self, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let bases: Vec<ClassId> = syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        let inherited = |t: ClassId| syms.class(c).superclass.is_some_and(|s| syms.class(s).base_types.iter().any(|&(b, _)| b == t));
        for (name, desc, params, ret) in [
            (n::TO_STRING, "()Ljava/lang/String;", vec![], JType::L(Rc::from(STRING))),
            (n::HASH_CODE, "()I", vec![], JType::I),
            (n::EQUALS, "(Ljava/lang/Object;)Z", vec![JType::object()], JType::Z),
        ] {
            let text = cx.input.interner.get(name);
            if self.cw.has_method(text, desc) {
                continue;
            }
            let Some(b) = bases.iter().copied().find(|&b| self.defines_object_method(b, name, desc)) else { continue };
            if syms.class(b).kind != ClassKind::Trait || inherited(b) {
                continue;
            }
            let base = self.class_name(b);
            let ps: Vec<(Option<SymId>, JType)> = params.iter().map(|t| (None, t.clone())).collect();
            self.begin_method(false, false, &ps, ret.clone(), &[]);
            self.load_this();
            for (i, t) in params.iter().enumerate() {
                self.load(i as u16 + 1, t);
            }
            self.invoke_desc(Invoke::Special, &base, true, text, desc, params.len(), &ret);
            self.return_value(&ret);
            self.end_method(ACC_PUBLIC, text, desc);
        }
    }

    /// Whether `b` declares `toString`, `hashCode` or `equals` with a body: a jar's class in its
    /// class file, a class of the program as a member the program reaches.
    fn defines_object_method(&self, b: ClassId, name: crate::intern::Name, desc: &str) -> bool {
        let cx = self.cx;
        if let Some(cf) = cx.class_files.get(&b) {
            let text = cx.input.interner.get(name);
            return cf.methods.iter().any(|m| m.name == text && m.descriptor == desc && m.access & (ACC_ABSTRACT | ACC_STATIC) == 0);
        }
        let syms = cx.input.syms;
        syms.class(b).members.get(&name).is_some_and(|&m| {
            let f = cx.fun_of_sym[m.idx()];
            syms.sym(m).owner == Owner::Class(b) && f != u32::MAX && cx.input.prog.funs[f as usize].body.is_some() && cx.input.reach.funs[f as usize]
        })
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
        let this = JType::L(owner.clone());
        let (name, ty) = (self.field_name(field), self.sym_type(field));
        let u = self.vc_underlying(c);
        for (member, params, ret) in self.vc_synthesized(c) {
            let typed: Vec<(Option<SymId>, JType)> = params.iter().map(|t| (None, t.clone())).collect();
            if on_companion {
                self.box_forwarder(c, member, &format!("{}$extension", member), &typed, &ret);
                continue;
            }
            self.begin_method(false, false, &typed, ret.clone(), &[]);
            self.load(0, &this);
            self.getfield(&owner, &name, &ty);
            self.adapt(&ty, &u);
            let us = self.store_new(&u);
            self.vc_synthesized_body(c, member, &ret, &u, us, 1);
            self.end_method(ACC_PUBLIC, member, &method_desc(&params, &ret));
        }
    }

    /// The members scalac synthesizes for value class `c` that the class does not define or
    /// inherit, with their parameters and results, in the order of scalac's companion:
    /// `hashCode` and `equals`, and a case class's `toString` and product members.
    pub(super) fn vc_synthesized(&mut self, c: ClassId) -> Vec<(&'static str, Vec<JType>, JType)> {
        let cx = self.cx;
        let string = JType::L(Rc::from(STRING));
        let object = JType::object();
        let mut all = vec![("hashCode", vec![], JType::I), ("equals", vec![object.clone()], JType::Z)];
        if cx.is_case_class(c) {
            all.extend([
                ("toString", vec![], string.clone()),
                ("canEqual", vec![object.clone()], JType::Z),
                ("productArity", vec![], JType::I),
                ("productPrefix", vec![], string.clone()),
                ("productElement", vec![JType::I], object),
                ("productElementName", vec![JType::I], string),
            ]);
        }
        all.retain(|(name, params, _)| !self.vc_defines(c, name, params));
        all
    }

    /// Whether member `name` of value class `c` taking `params` is concrete where the class or a
    /// universal trait it extends declares it, which is then no synthesized one (`existingDef`,
    /// `SyntheticMembers.scala` 82-89: the member `c`'s type sees, unless it is deferred or the
    /// root's own): the first declaration in the linearization whose erased parameters are
    /// `params`, a program's with a body, or a jar class file's method that is not abstract. The
    /// box inherits it.
    pub(super) fn vc_defines(&mut self, c: ClassId, name: &str, params: &[JType]) -> bool {
        let cx = self.cx;
        let syms = cx.input.syms;
        let Some(n) = cx.input.interner.lookup(name) else { return false };
        let desc = method_desc(params, &JType::V);
        let want = &desc[..desc.len() - 1];
        let bases: Vec<ClassId> = syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        for b in bases {
            // `Any`, `Object`, `Product` and `Equals` declare what scalac synthesizes.
            if syms.class(b).kind == ClassKind::Builtin || matches!(&*self.class_name(b), OBJECT | PRODUCT | EQUALS) {
                return false;
            }
            // A jar's class, by its class file (its members' signatures may be unread).
            if let Some(cf) = cx.class_files.get(&b) {
                if let Some(m) = cf.methods.iter().find(|m| m.name == name && m.descriptor.starts_with(want) && m.access & ACC_STATIC == 0) {
                    return m.access & ACC_ABSTRACT == 0;
                }
                continue;
            }
            let declared: Vec<SymId> = match syms.class(b).members.get(&n) {
                Some(&m) => syms.alternatives(m).map_or_else(|| vec![m], |alts| alts.to_vec()),
                None => Vec::new(),
            };
            for s in declared {
                if self.param_types(s) == params {
                    let f = cx.fun_of_sym[s.idx()];
                    return f != u32::MAX && cx.input.prog.funs[f as usize].body.is_some();
                }
            }
        }
        false
    }

    /// The code of synthesized member `member` of value class `c`, returning `ret`, over the
    /// underlying value in local `us` of type `u` and the member's parameter in local `ps`: a
    /// primitive underlying value hashes as its own `hashCode`, a reference one through the
    /// null-safe `Objects.hashCode` (`SyntheticMembers.scala` 338-346); `equals` is a test of
    /// the class and `==` of the underlying values, primitive comparison for a primitive (`NaN`
    /// unequal, `-0.0` equal to `0.0`), then a `canEqual` the class defines, without the `eq` test
    /// of a case class (297-332).
    pub(super) fn vc_synthesized_body(&mut self, c: ClassId, member: &str, ret: &JType, u: &JType, us: u16, ps: u16) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let string = JType::L(Rc::from(STRING));
        let object = JType::object();
        let class = self.class_name(c);
        match member {
            "hashCode" => {
                self.load(us, u);
                let primitive = match u {
                    JType::I => Some("java/lang/Integer"),
                    JType::J => Some("java/lang/Long"),
                    JType::D => Some("java/lang/Double"),
                    JType::F => Some("java/lang/Float"),
                    JType::Z => Some("java/lang/Boolean"),
                    JType::C => Some("java/lang/Character"),
                    JType::S => Some("java/lang/Short"),
                    JType::B => Some("java/lang/Byte"),
                    _ => None,
                };
                match primitive {
                    Some(b) => {
                        self.invoke(Invoke::Static, b, false, "hashCode", std::slice::from_ref(u), &JType::I);
                    }
                    None => {
                        self.adapt(u, &object);
                        self.invoke(Invoke::Static, "java/util/Objects", false, "hashCode", &[object], &JType::I);
                    }
                }
                self.return_value(ret);
            }
            "equals" => {
                let no = self.code.new_label();
                self.load(ps, &object);
                self.instance_of(&class);
                self.jump_if(op::IFEQ, 1, no);
                self.load(us, u);
                self.load(ps, &object);
                self.checkcast(&class);
                self.vc_unbox(c, u);
                match u {
                    JType::J | JType::F | JType::D => {
                        self.code.op(match u {
                            JType::J => op::LCMP,
                            JType::F => op::FCMPL,
                            _ => op::DCMPL,
                        });
                        self.code.popn(2);
                        self.code.push(VT::Int);
                        self.jump_if(op::IFNE, 1, no);
                    }
                    JType::L(_) => {
                        self.adapt(u, &object);
                        let that = self.store_new(&object);
                        self.adapt(u, &object);
                        self.load(that, &object);
                        self.helper_call("equal", 2);
                        self.jump_if(op::IFEQ, 1, no);
                    }
                    _ => self.jump_if(op::IF_ICMPNE, 2, no),
                }
                // A `canEqual` of the class's own is asked last, `that.canEqual(this)` (297-332).
                if self.vc_defines(c, "canEqual", std::slice::from_ref(&object)) {
                    self.load(ps, &object);
                    self.checkcast(&class);
                    self.load(us, u);
                    self.vc_box(c, u);
                    let boxed = JType::L(class.clone());
                    self.adapt(&boxed, &object);
                    self.invoke(Invoke::Virtual, &class, false, "canEqual", &[object.clone()], &JType::Z);
                    self.jump_if(op::IFEQ, 1, no);
                }
                self.iconst(1);
                self.return_value(ret);
                self.code.bind(no);
                self.iconst(0);
                self.return_value(ret);
            }
            "toString" => {
                let info = syms.class(c);
                let display = info.product_name(cx.input.interner.get(info.name)).to_string();
                let sb = "java/lang/StringBuilder";
                let sb_type = JType::L(Rc::from(sb));
                self.new_object(sb);
                self.invoke(Invoke::Special, sb, false, "<init>", &[], &JType::V);
                self.sconst(&format!("{}(", display));
                self.invoke(Invoke::Virtual, sb, false, "append", &[string.clone()], &sb_type);
                self.load(us, u);
                let param = match u {
                    JType::L(_) if u.is_string() => u.clone(),
                    JType::L(_) => object.clone(),
                    JType::B | JType::S => JType::I,
                    _ => u.clone(),
                };
                self.adapt(u, &param);
                self.invoke(Invoke::Virtual, sb, false, "append", &[param], &sb_type);
                self.sconst(")");
                self.invoke(Invoke::Virtual, sb, false, "append", &[string.clone()], &sb_type);
                self.invoke(Invoke::Virtual, sb, false, "toString", &[], &string);
                self.return_value(ret);
            }
            "canEqual" => {
                self.load(ps, &object);
                self.instance_of(&class);
                self.return_value(ret);
            }
            "productArity" => {
                self.iconst(1);
                self.return_value(ret);
            }
            "productPrefix" => {
                let info = syms.class(c);
                let display = info.product_name(cx.input.interner.get(info.name)).to_string();
                self.sconst(&display);
                self.return_value(ret);
            }
            _ => {
                // `productElement` and `productElementName`: the one element, else
                // `IndexOutOfBoundsException`.
                let fail = self.code.new_label();
                self.load(ps, &JType::I);
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
                self.load(ps, &JType::I);
                self.invoke(Invoke::Static, STRING, false, "valueOf", &[JType::I], &string);
                self.invoke(Invoke::Special, "java/lang/IndexOutOfBoundsException", false, "<init>", &[string], &JType::V);
                self.code.op(op::ATHROW);
                self.code.pop();
                self.code.end_path();
            }
        }
    }

    /// A method `(I)ret` answering the `n`th of `count` elements, as `productElement` does:
    /// `IndexOutOfBoundsException` for any other index.
    fn element_switch(&mut self, ret: &JType, count: usize, mut element: impl FnMut(&mut Self, usize)) {
        let string = JType::L(Rc::from(STRING));
        self.begin_method(false, false, &[(None, JType::I)], ret.clone(), &[]);
        for i in 0..count {
            let next = self.code.new_label();
            self.load(1, &JType::I);
            self.iconst(i as i32);
            self.jump_if(op::IF_ICMPNE, 2, next);
            element(self, i);
            self.return_value(ret);
            self.code.bind(next);
        }
        self.new_object("java/lang/IndexOutOfBoundsException");
        self.load(1, &JType::I);
        self.invoke(Invoke::Static, STRING, false, "valueOf", &[JType::I], &string);
        self.invoke(Invoke::Special, "java/lang/IndexOutOfBoundsException", false, "<init>", &[string], &JType::V);
        self.code.op(op::ATHROW);
        self.code.pop();
        self.code.end_path();
    }

    fn product_members(&mut self, c: ClassId, is_case: bool, is_value_case: bool, is_object: bool) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let info = syms.class(c);
        let is_enum_case = info.kind == ClassKind::EnumCase;
        if !is_case && !is_enum_case {
            return;
        }
        let owner = self.this_name.clone();
        let this = JType::L(owner.clone());
        let string = JType::L(Rc::from(STRING));
        let object = JType::object();
        let display = info.product_name(cx.input.interner.get(info.name)).to_string();
        let is_tuple = display.strip_prefix("Tuple").map_or(false, |r| !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit()))
            && info.def.is_none();
        let fields: Vec<SymId> = if is_value_case || is_object {
            Vec::new()
        } else {
            info.ctor.first().map(|cl| cl.params.iter().map(|p| p.sym).collect()).unwrap_or_default()
        };
        let field_refs: Vec<(String, JType)> = fields.iter().map(|&f| (self.field_name(f), self.sym_type(f))).collect();

        // productPrefix, productArity, productElement
        self.begin_method(false, false, &[], string.clone(), &[]);
        if is_value_case {
            self.load(0, &this);
            self.getfield(&owner, "$name", &string);
        } else {
            self.sconst(&display);
        }
        self.return_value(&string);
        self.end_method(ACC_PUBLIC, "productPrefix", "()Ljava/lang/String;");

        self.begin_method(false, false, &[], JType::I, &[]);
        self.iconst(field_refs.len() as i32);
        self.return_value(&JType::I);
        self.end_method(ACC_PUBLIC, "productArity", "()I");

        // A field of a value class is its box, as scalac's `productElement` and `_toString` give
        // it, and hashed as the box (`Statics.anyHash(new V(..))`); `equals` compares the
        // underlying values.
        let boxed: Vec<bool> = fields.iter().map(|&f| syms.sym(f).sig.as_ref().map_or(false, |sig| self.value_class_type(sig.ret).is_some())).collect();
        self.element_switch(&object, field_refs.len(), |g, i| {
            let (name, ty) = &field_refs[i];
            g.load(0, &this);
            g.getfield(&owner, name, ty);
            g.adapt_from_sym(fields[i], ty, &object);
        });
        self.end_method(ACC_PUBLIC, "productElement", "(I)Ljava/lang/Object;");

        // `java.lang.Enum` answers `ordinal`, `hashCode` and `equals` with final methods.
        let java_enum = cx.enum_of_case(c).map_or(false, |e| cx.is_java_enum(e));
        if is_enum_case && !java_enum {
            self.begin_method(false, false, &[], JType::I, &[]);
            if is_value_case {
                self.load(0, &this);
                self.getfield(&owner, "$ordinal", &JType::I);
            } else {
                self.iconst(info.ordinal as i32);
            }
            self.return_value(&JType::I);
            self.end_method(ACC_PUBLIC, "ordinal", "()I");
        }

        let inherits = |g: &Self, name: crate::intern::Name| {
            syms.class(c).base_types.iter().any(|&(b, _)| {
                syms.class(b).members.get(&name).map_or(false, |&m| {
                    let f = g.cx.fun_of_sym[m.idx()];
                    syms.sym(m).intrinsic.is_some() || (f != u32::MAX && g.cx.input.prog.funs[f as usize].body.is_some())
                })
            })
        };

        // Product's own members, which scala-library leaves abstract or answers with "".
        let defined_below_product = |g: &Self, name: &str| {
            g.cx.input.interner.lookup(name).map_or(false, |name| {
                syms.class(c).base_types.iter().any(|&(b, _)| {
                    let jar = g.cx.class_files.get(&b).map_or(false, |cf| {
                        cf.methods.iter().any(|m| m.name == g.cx.input.interner.get(name) && m.access & (ACC_ABSTRACT | ACC_STATIC) == 0)
                    });
                    !matches!(g.cx.class_names[b.idx()].as_str(), PRODUCT | EQUALS)
                        && (jar || syms.class(b).members.get(&name).map_or(false, |&m| {
                            let f = g.cx.fun_of_sym[m.idx()];
                            f != u32::MAX && g.cx.input.prog.funs[f as usize].body.is_some()
                        }))
                })
            })
        };
        if !defined_below_product(self, "canEqual") {
            self.begin_method(false, false, &[(None, object.clone())], JType::Z, &[]);
            self.load(1, &object);
            self.instance_of(&owner);
            self.return_value(&JType::Z);
            self.end_method(ACC_PUBLIC, "canEqual", "(Ljava/lang/Object;)Z");
        }
        if !defined_below_product(self, "productElementName") {
            let names: Vec<String> = fields.iter().map(|&f| cx.input.interner.get(syms.sym(f).name).to_string()).collect();
            self.element_switch(&string, names.len(), |g, i| g.sconst(&names[i]));
            self.end_method(ACC_PUBLIC, "productElementName", "(I)Ljava/lang/String;");
        }

        if !inherits(self, n::TO_STRING) {
            self.begin_method(false, false, &[], string.clone(), &[]);
            if is_value_case {
                self.load(0, &this);
                self.getfield(&owner, "$name", &string);
            } else if field_refs.is_empty() && (is_object || !is_case) {
                self.sconst(&display);
            } else {
                let sb = "java/lang/StringBuilder";
                let sb_type = JType::L(Rc::from(sb));
                self.new_object(sb);
                self.invoke(Invoke::Special, sb, false, "<init>", &[], &JType::V);
                self.sconst(&if is_tuple { "(".to_string() } else { format!("{}(", display) });
                self.invoke(Invoke::Virtual, sb, false, "append", &[string.clone()], &sb_type);
                for (i, (name, ty)) in field_refs.iter().enumerate() {
                    if i > 0 {
                        self.sconst(",");
                        self.invoke(Invoke::Virtual, sb, false, "append", &[string.clone()], &sb_type);
                    }
                    self.load(0, &this);
                    self.getfield(&owner, name, ty);
                    if boxed[i] {
                        self.adapt_from_sym(fields[i], ty, &object);
                    }
                    let param = match ty {
                        _ if boxed[i] => object.clone(),
                        JType::L(_) if ty.is_string() => ty.clone(),
                        JType::L(_) => object.clone(),
                        JType::B | JType::S => JType::I,
                        _ => ty.clone(),
                    };
                    self.invoke(Invoke::Virtual, sb, false, "append", &[param], &sb_type);
                }
                self.sconst(")");
                self.invoke(Invoke::Virtual, sb, false, "append", &[string.clone()], &sb_type);
                self.invoke(Invoke::Virtual, sb, false, "toString", &[], &string);
            }
            self.return_value(&string);
            self.end_method(ACC_PUBLIC, "toString", "()Ljava/lang/String;");
        }

        if !inherits(self, n::HASH_CODE) && !java_enum {
            self.begin_method(false, false, &[], JType::I, &[]);
            if is_value_case {
                self.load(0, &this);
                self.getfield(&owner, "$name", &string);
                self.invoke(Invoke::Virtual, STRING, false, "hashCode", &[], &JType::I);
            } else if field_refs.is_empty() {
                self.sconst(&display);
                self.invoke(Invoke::Virtual, STRING, false, "hashCode", &[], &JType::I);
            } else {
                // MurmurHash3.productHash, as scalac's case classes hash.
                self.iconst(0xcafebabeu32 as i32);
                self.sconst(&display);
                self.invoke(Invoke::Virtual, STRING, false, "hashCode", &[], &JType::I);
                self.helper_call("mix", 2);
                for (i, (name, ty)) in field_refs.iter().enumerate() {
                    self.load(0, &this);
                    self.getfield(&owner, name, ty);
                    self.adapt_from_sym(fields[i], ty, &object);
                    self.helper_call("anyHash", 1);
                    self.helper_call("mix", 2);
                }
                self.iconst(field_refs.len() as i32);
                self.helper_call("finalizeHash", 2);
            }
            self.return_value(&JType::I);
            self.end_method(ACC_PUBLIC, "hashCode", "()I");
        }

        if is_case && !is_object && !inherits(self, n::EQUALS) {
            self.begin_method(false, false, &[(None, object.clone())], JType::Z, &[]);
            let no = self.code.new_label();
            let yes = self.code.new_label();
            self.load(0, &this);
            self.load(1, &object);
            self.jump_if(op::IF_ACMPEQ, 2, yes);
            self.load(1, &object);
            self.instance_of(&owner);
            self.jump_if(op::IFEQ, 1, no);
            self.load(1, &object);
            self.checkcast(&owner);
            let that = self.store_new(&this);
            for (name, ty) in &field_refs {
                self.load(0, &this);
                self.getfield(&owner, name, ty);
                self.load(that, &this);
                self.getfield(&owner, name, ty);
                match ty {
                    JType::J => {
                        self.code.op(op::LCMP);
                        self.code.popn(2);
                        self.code.push(VT::Int);
                        self.jump_if(op::IFNE, 1, no);
                    }
                    JType::D => {
                        self.code.op(op::DCMPL);
                        self.code.popn(2);
                        self.code.push(VT::Int);
                        self.jump_if(op::IFNE, 1, no);
                    }
                    JType::F => {
                        self.code.op(op::FCMPL);
                        self.code.popn(2);
                        self.code.push(VT::Int);
                        self.jump_if(op::IFNE, 1, no);
                    }
                    JType::L(_) => {
                        self.helper_call("equal", 2);
                        self.jump_if(op::IFEQ, 1, no);
                    }
                    _ => self.jump_if(op::IF_ICMPNE, 2, no),
                }
            }
            self.code.bind(yes);
            self.iconst(1);
            self.return_value(&JType::Z);
            self.code.bind(no);
            self.iconst(0);
            self.return_value(&JType::Z);
            self.end_method(ACC_PUBLIC, "equals", "(Ljava/lang/Object;)Z");
        }
    }

    // ---- the top-level definitions of a file ----

    pub fn emit_file_module(&mut self, file: FileId) -> (String, Vec<u8>) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let syms = cx.input.syms;
        let reach = cx.input.reach;
        let name: Rc<str> = Rc::from(cx.file_modules[file.0 as usize].as_str());
        self.context = name.to_string();
        self.cw = ClassWriter::new(ACC_PUBLIC | ACC_FINAL | ACC_SUPER, &name, OBJECT, &[]);
        self.this_name = name.clone();
        self.this_class = None;
        self.is_interface = false;
        self.abstract_class = false;
        let path = &cx.input.sources[file.0 as usize].path;
        self.cw.source_file(path.rsplit(std::path::is_separator).next().unwrap_or(path));
        self.line_file = file;
        self.cw.field(MODULE_FIELD, "MODULE$", &format!("L{};", name));

        let group = cx.file_group[file.0 as usize];
        let vals: &[(SymId, TExprId)] = if group == u32::MAX { &[] } else { &cx.layout.file_groups[group as usize].1 };
        let file_reached = reach.files.get(file.0 as usize).copied().unwrap_or(false);
        let mut fields: Vec<FieldDef> = Vec::new();
        for &(s, init) in vals {
            let eager = crate::emit::layout::is_eager_top_val(syms, s, &cx.layout.const_vals);
            if (eager && !file_reached) || (!eager && !reach.vals[s.idx()]) || cx.given_objects.contains_key(&s) || cx.given_classes.contains_key(&s) {
                continue;
            }
            let ty = self.sym_type(s);
            let fname = self.field_name(s);
            fields.push(FieldDef { sym: s, name: fname, ty, lazy: !eager, lazy_owner: None, init: Some(init), mutable: syms.sym(s).kind == SymKind::Var });
        }
        for f in &fields {
            let mut d = String::new();
            (if f.lazy { JType::object() } else { f.ty.clone() }).desc(&mut d);
            let access = if !f.lazy && self.is_volatile(f.sym) { ACC_PRIVATE | ACC_VOLATILE } else { ACC_PRIVATE };
            self.cw.field(access, &if f.lazy { format!("{}$lzy", f.name) } else { f.name.clone() }, &d);
        }
        let given_inits: Vec<TInit> = vals.iter().map(|&(s, e)| TInit::Field(s, e)).collect();
        self.given_object_fields(&given_inits);
        let roots: Vec<TExprId> = fields.iter().filter(|f| !f.lazy).filter_map(|f| f.init).collect();
        let this = JType::L(name.clone());
        self.begin_module_clinit(&name, &roots);
        let slot = self.m.this_slot.unwrap_or(0);
        for f in fields.iter().filter(|f| !f.lazy) {
            self.load(slot, &this);
            self.expr(f.init.unwrap(), &f.ty);
            self.putfield(&name, &f.name, &f.ty);
        }
        self.return_value(&JType::V);
        self.end_method(ACC_STATIC, "<clinit>", "()V");
        self.begin_method(false, true, &[], JType::V, &[]);
        self.load(0, &this);
        self.invoke(Invoke::Special, OBJECT, false, "<init>", &[], &JType::V);
        self.return_value(&JType::V);
        self.end_method(ACC_PRIVATE, "<init>", "()V");
        for f in &fields {
            self.accessors(f);
        }
        let mut funs: Vec<FunId> = cx.file_funs[file.0 as usize].iter().copied().filter(|f| reach.funs[f.idx()]).collect();
        funs.sort_by(|&a, &b| crate::emit::layout::compare_syms(syms, cx.input.interner, prog.funs[a.idx()].sym, prog.funs[b.idx()].sym));
        for f in funs {
            self.emit_fun(f);
        }
        if let Some(list) = reach.exports.files.get(&file) {
            self.export_forwarders(list);
        }
        self.given_class_defs(Owner::Package(cx.input.file_pkgs[file.0 as usize]), Some(file));
        self.drain_pending();
        self.finish_class(&name)
    }

    /// `TeqMain.main(String[])` calls the entry point: the command line reaches a
    /// `main(args: Array[String])` as an array and a `@main def f(args: String*)` as a sequence.
    pub fn emit_launcher(&mut self, main: SymId, object: Option<ClassId>) -> (String, Vec<u8>) {
        self.cw = ClassWriter::new(ACC_PUBLIC | ACC_FINAL | ACC_SUPER, LAUNCHER, OBJECT, &[]);
        self.this_name = Rc::from(LAUNCHER);
        self.this_class = None;
        self.is_interface = false;
        self.context = LAUNCHER.to_string();
        self.entry_main(main, object);
        let cw = std::mem::replace(&mut self.cw, ClassWriter::new(0, "x", OBJECT, &[]));
        (LAUNCHER.to_string(), cw.finish(self.cx.input.output_version))
    }

    /// `static main(String[])` calling the entry point `main` of the program.
    pub fn entry_main(&mut self, main: SymId, object: Option<ClassId>) {
        let cx = self.cx;
        let info = cx.input.syms.sym(main);
        let args_type = JType::L(Rc::from("[Ljava/lang/String;"));
        self.begin_method(true, false, &[(None, args_type.clone())], JType::V, &[]);
        let m = self.mref(main);
        match (object, info.owner) {
            (Some(c), _) | (None, Owner::Class(c)) => {
                self.load_module(c);
            }
            _ => {
                let module = cx.file_modules[info.file.0 as usize].clone();
                self.getstatic(&module, "MODULE$", &JType::L(Rc::from(module.as_str())));
            }
        }
        if let Some(param) = m.params.first().cloned() {
            let repeated = info.sig.as_ref().and_then(|s| s.clauses.iter().flat_map(|c| c.params.iter()).next()).map_or(false, |p| p.repeated);
            match cx.input.array_seq.filter(|_| repeated) {
                // The arguments' array itself in an `ArraySeq`: scala-library's `ofRef` takes
                // it as an array of references, the std's as an array of any kind.
                Some(seq) => {
                    let seq_name = self.class_name(seq);
                    self.new_object(&seq_name);
                    self.load(0, &args_type);
                    let taken = if cx.linked_class(seq) { JType::L(Rc::from(OBJECT_ARRAY)) } else { JType::object() };
                    self.adapt(&args_type, &taken);
                    self.invoke(Invoke::Special, &seq_name, false, "<init>", &[taken], &JType::V);
                    let t = JType::L(seq_name);
                    self.adapt(&t, &param);
                }
                None if repeated => self.unsupported("a repeated parameter of the entry point without ArraySeq"),
                None => {
                    self.load(0, &args_type);
                    self.adapt(&args_type, &param);
                }
            }
        }
        self.invoke_mref(&m);
        self.pop_value(&m.ret.clone());
        self.return_value(&JType::V);
        self.end_method(ACC_PUBLIC | ACC_STATIC, "main", "([Ljava/lang/String;)V");
    }
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
