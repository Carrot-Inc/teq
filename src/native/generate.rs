//! The definitions the toolchain adds to a linked program, after `codegen.Generate` and
//! `GenerateReflectiveProxies` of Scala Native 0.5.12: C's `main`, the type records, the modules'
//! accessors and their array, the constants the C runtime reads, and a proxy for each method a
//! structural call can reach.

use super::meta::Meta;
use super::nir::names::{self, Rt, Scope as SigScope};
use super::nir::*;
use super::reach::{Info, Linked};
use crate::intern::FxMap;
use std::sync::Arc;

/// A member of `__`: a symbol with C's linkage that belongs to no class.
pub fn external(cx: &mut Cx, id: &str) -> Global {
    let owner = cx.top("__");
    let sig = cx.extern_sig(id);
    cx.member(owner, sig)
}

fn function(cx: &mut Cx, args: &[Ty], ret: Ty) -> Ty {
    cx.ty(Type::Function(args.into(), ret))
}

fn define(name: Global, ty: Ty, pos: Pos, attrs: Attrs, insts: Vec<Inst>) -> Defn {
    Defn { attrs, name, pos, kind: DefnKind::Define { ty, insts, debug: DebugInfo::default() } }
}

fn let_(pos: Pos, id: Local, op: Op, unwind: Next) -> Inst {
    Inst { pos, kind: InstKind::Let { id, op, unwind, scope: 0 } }
}

fn label(pos: Pos, id: Local, params: &[(Local, Ty)]) -> Inst {
    Inst { pos, kind: InstKind::Label { id, params: params.into() } }
}

fn call(ty: Ty, ptr: Val, args: Vec<Val>) -> Op {
    Op::Call { ty, ptr, args: args.into_boxed_slice() }
}

/// `GenerateReflectiveProxies`: for each method a structural call can reach, a method under the
/// signature such a call names it by, which takes and gives boxes.
pub fn reflective_proxies(cx: &mut Cx, rt: &Rt, linked: &Linked) -> Vec<Defn> {
    let mut first: FxMap<(Global, Str), Global> = FxMap::default();
    for &g in &linked.dynimpls {
        let Some(sig) = cx.sig(g) else { continue };
        if names::method_sig(cx.bytes(sig)).is_none_or(|m| m.is_static()) {
            continue;
        }
        let Some(proxy) = cx.proxy_sig(sig) else { continue };
        first.entry((cx.owner(g), proxy)).or_insert(g);
    }
    let chosen: std::collections::HashSet<Global, crate::intern::FxBuild> = first.into_values().collect();
    let mut out = Vec::new();
    for defn in &linked.defns {
        let DefnKind::Define { ty, .. } = &defn.kind else { continue };
        if !chosen.contains(&defn.name) {
            continue;
        }
        let pos = defn.pos;
        let (Some(sig), Type::Function(args, ret)) = (cx.sig(defn.name), cx.type_of(*ty).clone()) else { continue };
        let Some(proxy_sig) = cx.proxy_sig(sig) else { continue };
        let proxy_args: Vec<Ty> = args.iter().map(|&a| rt.boxed(a).map_or(a, |b| cx.class_ty(b))).collect();
        let boxed_unit = cx.class_ty(rt.boxed_unit);
        let proxy_ret = if ret == rt.unit { boxed_unit } else { rt.object_ty };
        let n = proxy_args.len() as Local;
        let params: Vec<(Local, Ty)> = proxy_args.iter().enumerate().map(|(i, &ty)| (i as Local + 1, ty)).collect();
        let mut fresh = n + 1;
        let mut next = || {
            fresh += 1;
            fresh
        };
        let mut insts = vec![label(pos, n + 1, &params)];
        // Each argument but the receiver: unboxed where the method takes a primitive.
        let mut call_args = vec![Val::Local(params[0].0, params[0].1)];
        for (&(param, param_ty), &arg) in params.iter().zip(args.iter()).skip(1) {
            let primitive = matches!(cx.type_of(arg), Type::Bool | Type::Char | Type::Byte | Type::Short | Type::Int | Type::Long | Type::Int128 | Type::Float | Type::Double);
            let unboxed = rt.class_name(cx, param_ty).and_then(|c| rt.unboxed(c));
            let id = next();
            match unboxed {
                Some(held) if primitive => {
                    insts.push(let_(pos, id, Op::Unbox { ty: param_ty, obj: Val::Local(param, param_ty) }, Next::None));
                    call_args.push(Val::Local(id, held));
                }
                _ => {
                    insts.push(let_(pos, id, Op::Copy(Val::Local(param, param_ty)), Next::None));
                    call_args.push(Val::Local(id, param_ty));
                }
            }
        }
        let method = next();
        insts.push(let_(pos, method, Op::Method { obj: Val::Local(params[0].0, params[0].1), sig }, Next::None));
        let result = next();
        insts.push(let_(pos, result, call(*ty, Val::Local(method, rt.ptr), call_args), Next::None));
        if ret == rt.unit {
            insts.push(Inst { pos, kind: InstKind::Ret(Val::Unit) });
        } else {
            let boxed = next();
            let op = match rt.boxed(ret) {
                Some(b) => Op::Box { ty: cx.class_ty(b), obj: Val::Local(result, ret) },
                None => Op::Copy(Val::Local(result, ret)),
            };
            insts.push(let_(pos, boxed, op, Next::None));
            insts.push(Inst { pos, kind: InstKind::Ret(Val::Local(boxed, proxy_ret)) });
        }
        let name = cx.member(cx.owner(defn.name), proxy_sig);
        let proxy_ty = function(cx, &proxy_args, proxy_ret);
        out.push(define(name, proxy_ty, pos, Attrs { is_dyn: true, ..Attrs::default() }, insts));
    }
    out
}

/// `Lower.injects` and the collector's `init`: what the lowered code calls in the C runtime.
fn injects(cx: &mut Cx, rt: &Rt) -> Vec<Defn> {
    let (ptr, int, size, unit) = (rt.ptr, rt.int, rt.size, rt.unit);
    let mut out = Vec::new();
    let mut declare = |cx: &mut Cx, id: &str, args: &[Ty], ret: Ty, is_extern: bool| {
        let name = external(cx, id);
        let ty = function(cx, args, ret);
        out.push(Defn { attrs: Attrs { is_extern, ..Attrs::default() }, name, pos: Pos::NONE, kind: DefnKind::Declare { ty } });
    };
    declare(cx, "scalanative_GC_init", &[], unit, false);
    declare(cx, "scalanative_GC_alloc_small", &[ptr, size], ptr, true);
    declare(cx, "scalanative_GC_alloc_large", &[ptr, size], ptr, true);
    declare(cx, "scalanative_dyndispatch", &[ptr, int], ptr, true);
    declare(cx, "scalanative_throw", &[ptr], rt.nothing, true);
    declare(cx, "memset", &[ptr, int, size], ptr, true);
    declare(cx, "scalanative_Exception_onCatch", &[rt.throwable_ty], unit, true);
    declare(cx, "__scalanative_trait_dispatch_slowpath", &[ptr, int, int], ptr, true);
    declare(cx, "scalanative_StackOverflowGuards_check", &[], unit, true);
    declare(cx, "__scalanative_class_has_trait_slowpath", &[ptr, int], rt.bool, true);
    out
}

/// C's `main`: the collector's and the classes' initializers, the runtime's `init`, the
/// program's `main` and the runtime's shutdown; an exception that escapes goes to the thread's
/// handler and the status is 1.
fn main_defn(cx: &mut Cx, rt: &Rt, linked: &Linked, proxies: &[Defn], entry: Global) -> Defn {
    let pos = Pos::NONE;
    let (ptr, int, unit) = (rt.ptr, rt.int, rt.unit);
    let object_array = cx.class_ty(rt.arrays[8].1);
    let thread = cx.top("java.lang.Thread");
    let handler_class = cx.top("java.lang.Thread$UncaughtExceptionHandler");
    let (thread_ty, handler_ty) = (cx.class_ty(thread), cx.class_ty(handler_class));
    let member = |cx: &mut Cx, owner: Global, id: &str, types: &[Ty], scope: SigScope| {
        let sig = cx.method_sig(id, types, scope);
        Val::Global(cx.member(owner, sig), ptr)
    };
    let strings = cx.ty(Type::Array { of: rt.string_ty, nullable: true });
    let entry_main = member(cx, entry, "main", &[strings, unit], SigScope::PublicStatic);
    let runtime_init = member(cx, rt.runtime, "init", &[int, ptr, strings], SigScope::Public);
    let on_shutdown = member(cx, rt.runtime, "onShutdown", &[unit], SigScope::Public);
    let current_thread = member(cx, thread, "currentThread", &[thread_ty], SigScope::PublicStatic);
    let get_handler = member(cx, thread, "getUncaughtExceptionHandler", &[handler_ty], SigScope::Public);
    let execute_handler = member(cx, rt.runtime, "executeUncaughtExceptionHandler", &[handler_ty, thread_ty, rt.throwable_ty, unit], SigScope::Public);
    let gc_init = Val::Global(external(cx, "scalanative_GC_init"), ptr);

    let (exc, handler, thread_local, ueh) = (1, 2, 3, 4);
    let (argc, argv, runtime, arr) = (6, 7, 8, 9);
    let mut fresh: Local = 9;
    let mut next = || {
        fresh += 1;
        fresh
    };
    let mut insts = vec![label(pos, next(), &[(argc, int), (argv, ptr)])];
    // An instruction's handler takes the exception in a local of its own, made before the
    // instruction's.
    let mut unwinding = |insts: &mut Vec<Inst>, pos: Pos, id: Option<Local>, op: Op| {
        let caught = next();
        let unwind = Next::Unwind { exc: caught, ty: rt.throwable_ty, next: Box::new(Next::Label(handler, Box::new([Val::Local(caught, rt.throwable_ty)]))) };
        let id = id.unwrap_or_else(&mut next);
        insts.push(let_(pos, id, op, unwind));
    };
    let nullary = function(cx, &[], unit);
    unwinding(&mut insts, pos, None, call(nullary, gc_init, Vec::new()));
    let clinit = cx.clinit_sig();
    for defn in linked.defns.iter().chain(proxies) {
        if matches!(defn.kind, DefnKind::Define { .. }) && cx.sig(defn.name) == Some(clinit) {
            let owner = cx.class_ty(cx.owner(defn.name));
            unwinding(&mut insts, defn.pos, None, call(nullary, Val::Global(defn.name, owner), Vec::new()));
        }
    }
    let init_ty = function(cx, &[rt.runtime_ty, int, ptr], object_array);
    let entry_ty = function(cx, &[object_array], unit);
    let shutdown_ty = function(cx, &[rt.runtime_ty], unit);
    unwinding(&mut insts, pos, Some(runtime), Op::Module(rt.runtime));
    unwinding(&mut insts, pos, Some(arr), call(init_ty, runtime_init, vec![Val::Local(runtime, rt.runtime_ty), Val::Local(argc, int), Val::Local(argv, ptr)]));
    unwinding(&mut insts, pos, None, call(entry_ty, entry_main, vec![Val::Local(arr, object_array)]));
    unwinding(&mut insts, pos, None, call(shutdown_ty, on_shutdown, vec![Val::Local(runtime, rt.runtime_ty)]));
    insts.push(Inst { pos, kind: InstKind::Ret(Val::Int(0)) });
    insts.push(label(pos, handler, &[(exc, rt.throwable_ty)]));
    let current_ty = function(cx, &[], thread_ty);
    let get_ty = function(cx, &[thread_ty], handler_ty);
    let execute_ty = function(cx, &[rt.runtime_ty, handler_ty, thread_ty, rt.throwable_ty], unit);
    insts.push(let_(pos, thread_local, call(current_ty, current_thread, Vec::new()), Next::None));
    insts.push(let_(pos, ueh, call(get_ty, get_handler, vec![Val::Local(thread_local, thread_ty)]), Next::None));
    let last = next();
    insts.push(let_(pos, last, call(execute_ty, execute_handler, vec![Val::Null, Val::Local(ueh, handler_ty), Val::Local(thread_local, thread_ty), Val::Local(exc, rt.throwable_ty)]), Next::None));
    insts.push(Inst { pos, kind: InstKind::Ret(Val::Int(1)) });
    let name = external(cx, "main");
    let ty = function(cx, &[int, ptr], int);
    define(name, ty, pos, Attrs::default(), insts)
}

/// A module's accessor: a constant module is one static object; another is made at its first
/// use, by the C runtime's `loadModule` where threads can race for it.
fn module_accessors(cx: &mut Cx, rt: &Rt, linked: &Linked, meta: &Meta, out: &mut Vec<Defn>) {
    let (ptr, unit) = (rt.ptr, rt.unit);
    let load_module = external(cx, "__scalanative_loadModule");
    let load_module_ty = function(cx, &[ptr, ptr, rt.size, ptr], ptr);
    if meta.multithreading {
        out.push(Defn { attrs: Attrs { is_extern: true, ..Attrs::default() }, name: load_module, pos: Pos::NONE, kind: DefnKind::Declare { ty: load_module_ty } });
    }
    let modules = external(cx, "__modules");
    let (instance_sig, load_sig, ctor_sig) = (cx.generated_sig(b"instance"), cx.generated_sig(b"load"), cx.ctor_sig(&[]));
    for &cls in &meta.modules {
        let scope = &linked.scopes[cls as usize];
        let (name, pos) = (scope.name, scope.pos);
        let cls_ty = cx.class_ty(name);
        if meta.constant_modules[cls as usize] {
            let mut fields = vec![meta.rtti_const(rt, cls)];
            fields.extend(meta.lock_word_vals());
            out.push(Defn { attrs: Attrs::default(), name: cx.member(name, instance_sig), pos, kind: DefnKind::Var { ty: meta.object_header, rhs: Val::StructValue(fields.into()) } });
            continue;
        }
        let (entry, existing, initialize, slot, this, cond, alloc) = (1, 2, 3, 4, 5, 6, 7);
        let init = Val::Global(cx.member(name, ctor_sig), ptr);
        let select = Op::Elem { ty: ptr, ptr: Val::Global(modules, ptr), indexes: Box::new([Val::Int(meta.module_index[&cls])]) };
        let jump = |to: Local| Next::Label(to, Box::new([]));
        let insts = if meta.multithreading {
            vec![
                label(pos, entry, &[]),
                let_(pos, slot, select, Next::None),
                let_(pos, this, call(load_module_ty, Val::Global(load_module, ptr), vec![Val::Local(slot, ptr), meta.rtti_const(rt, cls), Val::Size(meta.layouts[&cls].size), init]), Next::None),
                Inst { pos, kind: InstKind::Ret(Val::Local(this, cls_ty)) },
            ]
        } else {
            let init_ty = function(cx, &[cls_ty], unit);
            vec![
                label(pos, entry, &[]),
                let_(pos, slot, select, Next::None),
                let_(pos, this, Op::Load { ty: cls_ty, ptr: Val::Local(slot, ptr), order: None }, Next::None),
                let_(pos, cond, Op::Comp { comp: Comp::Ine, ty: rt.object_ty, l: Val::Local(this, cls_ty), r: Val::Null }, Next::None),
                Inst { pos, kind: InstKind::If { value: Val::Local(cond, rt.bool), then: jump(existing), otherwise: jump(initialize) } },
                label(pos, existing, &[]),
                Inst { pos, kind: InstKind::Ret(Val::Local(this, cls_ty)) },
                label(pos, initialize, &[]),
                let_(pos, alloc, Op::Classalloc { name, zone: None }, Next::None),
                let_(pos, 8, Op::Store { ty: cls_ty, ptr: Val::Local(slot, ptr), value: Val::Local(alloc, cls_ty), order: None }, Next::None),
                let_(pos, 9, call(init_ty, init, vec![Val::Local(alloc, cls_ty)]), Next::None),
                Inst { pos, kind: InstKind::Ret(Val::Local(alloc, cls_ty)) },
            ]
        };
        let attrs = Attrs { inline: if meta.multithreading { Inline::May } else { Inline::No }, ..Attrs::default() };
        let ty = function(cx, &[], cls_ty);
        out.push(define(cx.member(name, load_sig), ty, pos, attrs, insts));
    }
}

/// `Generate.generate`: what is added to the linked definitions and the proxies.
pub fn generate(cx: &mut Cx, rt: &Rt, linked: &Linked, meta: &Meta, proxies: &[Defn], main: Global) -> Result<Vec<Defn>, String> {
    let (ptr, int) = (rt.ptr, rt.int);
    let mut out = injects(cx, rt);
    let has_main = match linked.infos.get(&main) {
        Some(&Info::Scope(id)) if !linked.scopes[id as usize].is_trait => {
            let strings = cx.ty(Type::Array { of: rt.string_ty, nullable: true });
            let sig = cx.method_sig("main", &[strings, rt.unit], SigScope::PublicStatic);
            linked.scopes[id as usize].resolve(sig).is_some()
        }
        _ => false,
    };
    if !has_main {
        return Err(format!("{} has no method main(args: Array[String]): Unit", String::from_utf8_lossy(cx.id(main))));
    }
    out.push(main_defn(cx, rt, linked, proxies, main));
    for &id in meta.classes.iter().chain(&meta.traits) {
        let rhs = meta.rtti_value(cx, rt, linked, id);
        out.push(Defn { attrs: Attrs::default(), name: meta.rtti_names[id as usize], pos: linked.scopes[id as usize].pos, kind: DefnKind::Var { ty: meta.rtti_structs[id as usize], rhs } });
    }
    module_accessors(cx, rt, linked, meta, &mut out);
    let constant = |cx: &mut Cx, id: &str, value: i32| Defn { attrs: Attrs::default(), name: external(cx, id), pos: Pos::NONE, kind: DefnKind::Const { ty: int, rhs: Val::Int(value) } };
    let instance_sig = cx.generated_sig(b"instance");
    let module_values: Vec<Val> = meta.modules.iter().map(|&m| if meta.constant_modules[m as usize] { Val::Global(cx.member(linked.scopes[m as usize].name, instance_sig), ptr) } else { Val::Null }).collect();
    let modules_ty = cx.ty(Type::ArrayValue(ptr, module_values.len() as i32));
    out.push(Defn { attrs: Attrs::default(), name: external(cx, "__modules"), pos: Pos::NONE, kind: DefnKind::Var { ty: modules_ty, rhs: Val::ArrayValue(ptr, module_values.into()) } });
    out.push(constant(cx, "__modules_size", meta.modules.len() as i32));
    let class_id = |name: Global| match linked.infos.get(&name) {
        Some(&Info::Scope(id)) => Ok(meta.ids[id as usize]),
        _ => Err("a class of the runtime is not linked".to_string()),
    };
    out.push(constant(cx, "__object_array_id", class_id(rt.arrays[8].1)?));
    out.push(constant(cx, "__blob_array_id", class_id(rt.blob_array)?));
    out.push(constant(cx, "__boxed_ptr_id", class_id(rt.boxed_ptr)?));
    // What the collector needs of `java.lang.ref.WeakReference`: its classes' ids and where the
    // referent lies.
    let weak = cx.top("java.lang.ref.WeakReference");
    let referent_sig = cx.field_sig("_gc_modified_referent", SigScope::Public);
    let referent = cx.member(weak, referent_sig);
    let mut weak_ids = (-1, -1, -1);
    if let (Some(&Info::Scope(id)), Some(Info::Field { .. })) = (linked.infos.get(&weak), linked.infos.get(&referent)) {
        if linked.scopes[id as usize].allocated() {
            let layout = &meta.layouts[&id];
            let index = layout.index(referent).ok_or("java.lang.ref.WeakReference has no field _gc_modified_referent")?;
            weak_ids = (meta.ranges[id as usize].0, meta.ranges[id as usize].1, layout.layout.tys[index as usize].1 as i32);
        }
    }
    out.push(constant(cx, "__weak_ref_ids_max", weak_ids.1));
    out.push(constant(cx, "__weak_ref_ids_min", weak_ids.0));
    out.push(constant(cx, "__weak_ref_field_offset", weak_ids.2));
    let mut array_ids = Vec::new();
    for &(_, cls) in rt.arrays.iter().chain(std::iter::once(&(rt.byte, rt.blob_array))) {
        array_ids.push(class_id(cls)?);
    }
    array_ids.sort();
    if array_ids.windows(2).any(|w| w[1] != w[0] + 1) {
        return Err("the ids of the array classes are not consecutive".to_string());
    }
    out.push(constant(cx, "__array_ids_min", array_ids[0]));
    out.push(constant(cx, "__array_ids_max", array_ids[array_ids.len() - 1]));
    // `ResourceEmbedder` with nothing to embed.
    let empty = |of: Ty| Val::Const(Arc::new(Val::ArrayValue(of, Arc::new([]))));
    for (id, rhs) in [
        ("__scala_native_resources_all_path", empty(ptr)),
        ("__scala_native_resources_all_path_lengths", empty(int)),
        ("__scala_native_resources_amount", Val::Int(0)),
        ("__scala_native_resources_all_content", empty(ptr)),
        ("__scala_native_resources_all_content_lengths", empty(int)),
    ] {
        out.push(Defn { attrs: Attrs::default(), name: external(cx, id), pos: Pos::NONE, kind: DefnKind::Var { ty: ptr, rhs } });
    }
    Ok(out)
}
