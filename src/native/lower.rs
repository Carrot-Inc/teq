//! The lowering of a linked program to what LLVM IR can say, after `codegen.Lower` of Scala
//! Native 0.5.12: objects become memory behind a type record, a field an offset, a virtual call
//! a load from a table, a cast a comparison of ids; null, bounds, division and cast checks
//! become branches to a slow path; and what no instruction uses is dropped
//! (`interflow.UseDef.eliminateDeadCode`). The numbering of the locals it makes follows Scala
//! Native's, so that the result can be held against its own.

use super::generate::external;
use super::meta::{self, Meta};
use super::nir::names::{self, Rt, Scope as SigScope};
use super::nir::*;
use super::reach::{self, Info, Linked, ScopeId};
use crate::intern::{FxBuild, FxMap};
use std::collections::HashSet;
use std::sync::Arc;

type FxSet<T> = HashSet<T, FxBuild>;
type Res<T> = Result<T, String>;

const LARGE_OBJECT_MIN_SIZE: i64 = 8192;

#[derive(Clone, Copy, PartialEq)]
enum Intrinsic {
    LoadAllClasses,
    MultiplyHigh,
    UnsignedMultiplyHigh,
}

/// The names and types of `Lower`'s companion: what lowered code calls.
struct Names {
    unit: Val,
    throw: Val,
    throw_ty: Ty,
    alloc_small: Global,
    alloc_large: Global,
    alloc_ty: Ty,
    safe_zone_alloc_sig: Str,
    safe_zone_alloc_ty: Ty,
    dyndispatch: Val,
    dyndispatch_ty: Ty,
    memset: Val,
    memset_ty: Ty,
    stack_check: Val,
    nullary_unit: Ty,
    gc_yield: Val,
    set_thread_state: Val,
    set_thread_state_ty: Ty,
    blocker_begin: Val,
    blocker_begin_ty: Ty,
    blocker_end: Val,
    blocker_end_ty: Ty,
    trait_dispatch: Val,
    trait_dispatch_ty: Ty,
    class_has_trait: Val,
    class_has_trait_ty: Ty,
    /// The six throwers of the runtime's package: null pointer, division by zero, class cast,
    /// undefined, out of bounds, no such method.
    throwers: [(Val, Ty); 6],
    box_to: FxMap<Global, Global>,
    unbox_to: FxMap<Global, Global>,
    /// By the element's type: the array module's `alloc`, `alloc` in a zone and `snapshot`,
    /// each with its type; the object array's last.
    array_fns: Vec<(Ty, [(Global, Ty); 3])>,
    virtual_thread: Global,
    instance_sig: Str,
    load_sig: Str,
    /// `scala.scalanative.runtime.package$.intrinsic` and the classes whose methods the
    /// lowering itself implements.
    linked_classes: Global,
    intrinsics: Global,
    /// The runtime's `enterMonitor`, which Scala Native's plugin calls for `synchronized`.
    enter_monitor: Global,
}

/// What a check branches to when it fails: a call of one of the runtime's throwers, the place of
/// its thrower in `Names::throwers`. A method's slow paths stand at its end in this order.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SlowPath {
    NullPointer,
    DivisionByZero,
    ClassCast,
    Undefined,
    OutOfBounds,
    NoSuchMethod,
}

/// `Lower`'s measure of a method that gets yield points where threads are linked: one of more
/// instructions than this, or one with a jump to a block, which may loop.
const YIELDING_METHOD_INSTS: usize = 4;

const SLOW_PATHS: [SlowPath; 6] = [SlowPath::NullPointer, SlowPath::DivisionByZero, SlowPath::ClassCast, SlowPath::Undefined, SlowPath::OutOfBounds, SlowPath::NoSuchMethod];

impl Names {
    fn new(cx: &mut Cx, rt: &Rt) -> Names {
        let (ptr, int, size, unit, nothing) = (rt.ptr, rt.int, rt.size, rt.unit, rt.nothing);
        let func = |cx: &mut Cx, args: &[Ty], ret: Ty| cx.ty(Type::Function(args.into(), ret));
        let ext = |cx: &mut Cx, id: &str| Val::Global(external(cx, id), ptr);
        let method = |cx: &mut Cx, owner: Global, id: &str, types: &[Ty]| {
            let sig = cx.method_sig(id, types, SigScope::Public);
            cx.member(owner, sig)
        };
        let instance_sig = cx.generated_sig(b"instance");
        let unit_instance = cx.member(rt.boxed_unit_module, instance_sig);
        let safe_zone = cx.top("scala.scalanative.memory.SafeZone");
        let safe_zone_ty = cx.class_ty(safe_zone);
        let gc = cx.top("scala.scalanative.runtime.GC$");
        let gc_ext = |cx: &mut Cx, id: &str| {
            let sig = cx.extern_sig(id);
            cx.member(gc, sig)
        };
        let gc_yield_name = gc_ext(cx, "scalanative_GC_yield");
        let blocker = cx.top("scala.scalanative.runtime.Blocker$");
        let thrower = |cx: &mut Cx, id: &str, sig: &[Ty], ty: &[Ty]| (Val::Global(method(cx, rt.runtime, id, sig), ptr), func(cx, ty, nothing));
        let throwers = [
            thrower(cx, "throwNullPointer", &[nothing], &[rt.runtime_ty]),
            thrower(cx, "throwDivisionByZero", &[nothing], &[rt.runtime_ty]),
            thrower(cx, "throwClassCast", &[ptr, ptr, nothing], &[rt.runtime_ty, ptr, ptr]),
            thrower(cx, "throwUndefined", &[nothing], &[ptr]),
            thrower(cx, "throwOutOfBounds", &[int, int, nothing], &[ptr, int, int]),
            thrower(cx, "throwNoSuchMethod", &[rt.string_ty, nothing], &[ptr, ptr]),
        ];
        let boxes_runtime = cx.top("scala.runtime.BoxesRunTime$");
        let runtime_boxes = cx.top("scala.scalanative.runtime.Boxes$");
        let (mut box_to, mut unbox_to) = (FxMap::default(), FxMap::default());
        for &(cls, held) in &rt.boxes {
            let id = String::from_utf8_lossy(cx.id(cls)).into_owned();
            let module = if id.starts_with("java.") { boxes_runtime } else { runtime_boxes };
            let last = id.rsplit('.').next().unwrap_or(&id).to_string();
            let short = match last.as_str() {
                "Integer" => "Int",
                "Character" => "Char",
                other => other,
            };
            let boxed = cx.class_ty(cls);
            box_to.insert(cls, method(cx, module, &format!("boxTo{}", last), &[held, boxed]));
            unbox_to.insert(cls, method(cx, module, &format!("unboxTo{}", short), &[rt.object_ty, held]));
        }
        let mut array_fns = Vec::new();
        for &(of, cls) in &rt.arrays {
            let module = cx.top(&format!("{}$", String::from_utf8_lossy(cx.id(cls))));
            let (module_ty, array_ty) = (cx.class_ty(module), cx.class_ty(cls));
            array_fns.push((
                of,
                [
                    (method(cx, module, "alloc", &[int, array_ty]), func(cx, &[module_ty, int], array_ty)),
                    (method(cx, module, "alloc", &[int, safe_zone_ty, array_ty]), func(cx, &[module_ty, int, safe_zone_ty], array_ty)),
                    (method(cx, module, "snapshot", &[int, ptr, array_ty]), func(cx, &[module_ty, int, ptr], array_ty)),
                ],
            ));
        }
        Names {
            unit: Val::Global(unit_instance, ptr),
            throw: ext(cx, "scalanative_throw"),
            throw_ty: func(cx, &[ptr], nothing),
            alloc_small: external(cx, "scalanative_GC_alloc_small"),
            alloc_large: external(cx, "scalanative_GC_alloc_large"),
            alloc_ty: func(cx, &[ptr, size], ptr),
            safe_zone_alloc_sig: cx.method_sig("allocImpl", &[ptr, size, ptr], SigScope::Public),
            safe_zone_alloc_ty: func(cx, &[safe_zone_ty, ptr, size], ptr),
            dyndispatch: ext(cx, "scalanative_dyndispatch"),
            dyndispatch_ty: func(cx, &[ptr, int], ptr),
            memset: ext(cx, "memset"),
            memset_ty: func(cx, &[ptr, int, size], ptr),
            stack_check: ext(cx, "scalanative_StackOverflowGuards_check"),
            nullary_unit: func(cx, &[], unit),
            gc_yield: Val::Global(gc_yield_name, ptr),
            set_thread_state: Val::Global(gc_ext(cx, "scalanative_GC_set_mutator_thread_state"), ptr),
            set_thread_state_ty: func(cx, &[int], unit),
            blocker_begin: Val::Global(method(cx, blocker, "begin", &[rt.bool]), ptr),
            blocker_begin_ty: func(cx, &[], rt.bool),
            blocker_end: Val::Global(method(cx, blocker, "end", &[rt.bool, unit]), ptr),
            blocker_end_ty: func(cx, &[rt.bool], unit),
            trait_dispatch: ext(cx, "__scalanative_trait_dispatch_slowpath"),
            trait_dispatch_ty: func(cx, &[ptr, int, int], ptr),
            class_has_trait: ext(cx, "__scalanative_class_has_trait_slowpath"),
            class_has_trait_ty: func(cx, &[ptr, int], rt.bool),
            throwers,
            box_to,
            unbox_to,
            array_fns,
            virtual_thread: cx.top("scala.scalanative.runtime.VirtualThread"),
            instance_sig,
            load_sig: cx.generated_sig(b"load"),
            linked_classes: cx.top("scala.scalanative.runtime.LinkedClassesRepository$"),
            intrinsics: cx.top("scala.scalanative.runtime.Intrinsics$"),
            enter_monitor: {
                let sig = cx.method_sig("enterMonitor", &[rt.object_ty, unit], SigScope::PublicStatic);
                let owner = cx.top("scala.scalanative.runtime.package");
                cx.member(owner, sig)
            },
        }
    }
}

/// A method's blocks: by label, where the block's instructions lie and the blocks that come to
/// it (`ControlFlow.Graph`, of the blocks a path from the first one comes to).
struct Graph {
    /// In the instructions' order: the label, the first instruction after it, one past its
    /// last (the first that transfers control).
    blocks: Vec<(Local, usize, usize)>,
    reachable: Vec<bool>,
    /// The ways into each block: the block each comes from and how (`Lowering::checked_into`).
    preds: Vec<Vec<(usize, Edge)>>,
}

/// How a block comes to another.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Edge {
    /// A jump, a branch or a switch's case.
    Jump,
    /// An instruction's unwinding to its handler.
    Unwind,
}

/// The rule that says a value is checked for null on entering a block (`Lowering::checked_into`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NullGuards {
    /// teq's: sound across joins and handlers, and a monitor's receiver checked too.
    #[default]
    Sound,
    /// Scala Native 0.5.12's own rule, which `teq nir --lower` alone takes: the reference the
    /// lowered listings are held to. It writes no check before a monitor is entered.
    ScalaNative,
}

fn is_cf(inst: &Inst) -> bool {
    !matches!(inst.kind, InstKind::Label { .. } | InstKind::Let { .. })
}

fn next_label(next: &Next) -> Option<Local> {
    match next {
        Next::None => None,
        Next::Unwind { next, .. } | Next::Case(_, next) => next_label(next),
        Next::Label(id, _) => Some(*id),
    }
}

impl Graph {
    fn new(insts: &[Inst]) -> Res<Graph> {
        let mut blocks = Vec::new();
        for (i, inst) in insts.iter().enumerate() {
            if let InstKind::Label { id, .. } = &inst.kind {
                let end = (i + 1..insts.len()).find(|&j| is_cf(&insts[j])).map_or(insts.len(), |j| j + 1);
                blocks.push((*id, i + 1, end));
            }
        }
        if blocks.first().is_none_or(|b| b.1 != 1) {
            return Err("a method that starts with no label".to_string());
        }
        let index: FxMap<Local, usize> = blocks.iter().enumerate().map(|(n, b)| (b.0, n)).collect();
        let mut reachable = vec![false; blocks.len()];
        let mut preds = vec![Vec::new(); blocks.len()];
        let mut todo = vec![0usize];
        while let Some(b) = todo.pop() {
            if std::mem::replace(&mut reachable[b], true) {
                continue;
            }
            let (_, start, end) = blocks[b];
            let mut successors: Vec<Local> = Vec::new();
            let mut handlers: Vec<Local> = Vec::new();
            for inst in &insts[start..end] {
                match &inst.kind {
                    InstKind::Let { unwind, .. } | InstKind::Throw { unwind, .. } | InstKind::Unreachable { unwind } => handlers.extend(next_label(unwind)),
                    InstKind::Jump(next) => successors.extend(next_label(next)),
                    InstKind::If { then, otherwise, .. } | InstKind::LinktimeIf { then, otherwise, .. } => {
                        successors.extend(next_label(then));
                        successors.extend(next_label(otherwise));
                    }
                    InstKind::Switch { default, cases, .. } => {
                        successors.extend(next_label(default));
                        successors.extend(cases.iter().filter_map(next_label));
                    }
                    _ => {}
                }
            }
            for s in successors {
                let &to = index.get(&s).ok_or("a jump to a label the method has not")?;
                preds[to].push((b, Edge::Jump));
                todo.push(to);
            }
            for h in handlers {
                let &to = index.get(&h).ok_or("a handler the method has not")?;
                preds[to].push((b, Edge::Unwind));
                todo.push(to);
            }
        }
        Ok(Graph { blocks, reachable, preds })
    }
}

fn for_each_val(v: &Val, f: &mut impl FnMut(&Val)) {
    f(v);
    match v {
        Val::StructValue(vs) | Val::ArrayValue(_, vs) => vs.iter().for_each(|v| for_each_val(v, f)),
        Val::Const(v) => for_each_val(v, f),
        _ => {}
    }
}

fn op_vals<'o>(op: &'o Op) -> Vec<&'o Val> {
    match op {
        Op::Call { ptr, args, .. } => std::iter::once(ptr).chain(args.iter()).collect(),
        Op::Load { ptr, .. } => vec![ptr],
        Op::Store { ptr, value, .. } => vec![ptr, value],
        Op::Elem { ptr, indexes, .. } => std::iter::once(ptr).chain(indexes.iter()).collect(),
        Op::Extract { aggr, .. } => vec![aggr],
        Op::Insert { aggr, value, .. } => vec![aggr, value],
        Op::Stackalloc { n, .. } => vec![n],
        Op::Bin { l, r, .. } | Op::Comp { l, r, .. } => vec![l, r],
        Op::Conv { value, .. } => vec![value],
        Op::Fence(_) | Op::Module(_) | Op::SizeOf(_) | Op::AlignmentOf(_) | Op::Var(_) => Vec::new(),
        Op::Classalloc { zone, .. } => zone.iter().collect(),
        Op::Fieldload { obj, .. } | Op::Field { obj, .. } | Op::Method { obj, .. } | Op::Dynmethod { obj, .. } | Op::As { obj, .. } | Op::Is { obj, .. } | Op::Box { obj, .. } | Op::Unbox { obj, .. } => vec![obj],
        Op::Fieldstore { obj, value, .. } => vec![obj, value],
        Op::Copy(v) | Op::Varload(v) | Op::Arraylength(v) => vec![v],
        Op::Varstore { slot, value } => vec![slot, value],
        Op::Arrayalloc { init, zone, .. } => std::iter::once(init).chain(zone.iter()).collect(),
        Op::Arrayload { arr, idx, .. } => vec![arr, idx],
        Op::Arraystore { arr, idx, value, .. } => vec![arr, idx, value],
    }
}

fn op_vals_mut(op: &mut Op) -> Vec<&mut Val> {
    match op {
        Op::Call { ptr, args, .. } => std::iter::once(ptr).chain(args.iter_mut()).collect(),
        Op::Load { ptr, .. } => vec![ptr],
        Op::Store { ptr, value, .. } => vec![ptr, value],
        Op::Elem { ptr, indexes, .. } => std::iter::once(ptr).chain(indexes.iter_mut()).collect(),
        Op::Extract { aggr, .. } => vec![aggr],
        Op::Insert { aggr, value, .. } => vec![aggr, value],
        Op::Stackalloc { n, .. } => vec![n],
        Op::Bin { l, r, .. } | Op::Comp { l, r, .. } => vec![l, r],
        Op::Conv { value, .. } => vec![value],
        Op::Fence(_) | Op::Module(_) | Op::SizeOf(_) | Op::AlignmentOf(_) | Op::Var(_) => Vec::new(),
        Op::Classalloc { zone, .. } => zone.iter_mut().collect(),
        Op::Fieldload { obj, .. } | Op::Field { obj, .. } | Op::Method { obj, .. } | Op::Dynmethod { obj, .. } | Op::As { obj, .. } | Op::Is { obj, .. } | Op::Box { obj, .. } | Op::Unbox { obj, .. } => vec![obj],
        Op::Fieldstore { obj, value, .. } => vec![obj, value],
        Op::Copy(v) | Op::Varload(v) | Op::Arraylength(v) => vec![v],
        Op::Varstore { slot, value } => vec![slot, value],
        Op::Arrayalloc { init, zone, .. } => std::iter::once(init).chain(zone.iter_mut()).collect(),
        Op::Arrayload { arr, idx, .. } => vec![arr, idx],
        Op::Arraystore { arr, idx, value, .. } => vec![arr, idx, value],
    }
}

fn next_vals_mut<'n>(next: &'n mut Next, out: &mut Vec<&'n mut Val>) {
    match next {
        Next::None => {}
        Next::Unwind { next, .. } => next_vals_mut(next, out),
        Next::Case(v, next) => {
            out.push(v);
            next_vals_mut(next, out);
        }
        Next::Label(_, args) => out.extend(args.iter_mut()),
    }
}

/// The values an instruction holds at its top, in the order `nir.Transform` visits them.
fn inst_vals_mut(inst: &mut Inst) -> Vec<&mut Val> {
    let mut out = Vec::new();
    match &mut inst.kind {
        InstKind::Label { .. } => {}
        InstKind::Let { op, unwind, .. } => {
            out = op_vals_mut(op);
            next_vals_mut(unwind, &mut out);
        }
        InstKind::Ret(v) => out.push(v),
        InstKind::Jump(next) => next_vals_mut(next, &mut out),
        InstKind::If { value, then, otherwise } => {
            out.push(value);
            next_vals_mut(then, &mut out);
            next_vals_mut(otherwise, &mut out);
        }
        InstKind::Switch { value, default, cases } => {
            out.push(value);
            next_vals_mut(default, &mut out);
            cases.iter_mut().for_each(|c| next_vals_mut(c, &mut out));
        }
        InstKind::Throw { value, unwind } => {
            out.push(value);
            next_vals_mut(unwind, &mut out);
        }
        InstKind::Unreachable { unwind } => next_vals_mut(unwind, &mut out),
        InstKind::LinktimeIf { then, otherwise, .. } => {
            next_vals_mut(then, &mut out);
            next_vals_mut(otherwise, &mut out);
        }
    }
    out
}

fn next_deps(next: &Next, out: &mut Vec<Local>) {
    match next {
        Next::None => {}
        Next::Unwind { next: inner, .. } => {
            out.extend(next_label(next));
            next_deps(inner, out);
        }
        Next::Case(v, inner) => {
            out.extend(next_label(next));
            for_each_val(v, &mut |v| {
                if let Val::Local(id, _) = v {
                    out.push(*id)
                }
            });
            next_deps(inner, out);
        }
        Next::Label(id, args) => {
            out.push(*id);
            args.iter().for_each(|a| {
                for_each_val(a, &mut |v| {
                    if let Val::Local(id, _) = v {
                        out.push(*id)
                    }
                })
            });
        }
    }
}

/// The locals and labels an instruction depends on (`UseDef.collect`).
fn inst_deps(inst: &Inst) -> Vec<Local> {
    let mut out = Vec::new();
    let local = |v: &Val, out: &mut Vec<Local>| {
        for_each_val(v, &mut |v| {
            if let Val::Local(id, _) = v {
                out.push(*id)
            }
        })
    };
    match &inst.kind {
        InstKind::Label { .. } => {}
        InstKind::Let { op, unwind, .. } => {
            op_vals(op).into_iter().for_each(|v| local(v, &mut out));
            next_deps(unwind, &mut out);
        }
        InstKind::Ret(v) => local(v, &mut out),
        InstKind::Jump(next) => next_deps(next, &mut out),
        InstKind::If { value, then, otherwise } => {
            local(value, &mut out);
            next_deps(then, &mut out);
            next_deps(otherwise, &mut out);
        }
        InstKind::Switch { value, default, cases } => {
            local(value, &mut out);
            next_deps(default, &mut out);
            cases.iter().for_each(|c| next_deps(c, &mut out));
        }
        InstKind::Throw { value, unwind } => {
            local(value, &mut out);
            next_deps(unwind, &mut out);
        }
        InstKind::Unreachable { unwind } => next_deps(unwind, &mut out),
        InstKind::LinktimeIf { .. } => {}
    }
    out
}

fn is_canonical(v: &Val) -> bool {
    matches!(v, Val::True | Val::False | Val::Char(_) | Val::Size(_) | Val::Byte(_) | Val::Short(_) | Val::Int(_) | Val::Long(_) | Val::Float(..) | Val::Double(..) | Val::Global(..) | Val::Null)
}

fn is_zero(v: &Val) -> bool {
    match v {
        Val::Zero(_) | Val::False | Val::Char(0) | Val::Size(0) | Val::Byte(0) | Val::Short(0) | Val::Int(0) | Val::Long(0) | Val::Null => true,
        Val::Float(bits, _) => f32::from_bits(*bits) == 0.0,
        Val::Double(bits, _) => f64::from_bits(*bits) == 0.0,
        _ => false,
    }
}

pub struct Lowering<'a> {
    cx: &'a mut Cx,
    rt: &'a Rt,
    linked: &'a Linked,
    meta: &'a Meta,
    names: Names,
    object: ScopeId,
    /// The methods that can call themselves, by any path: they check the stack's guard.
    recursive: FxSet<Global>,
    /// `Lower.BoxTo`'s methods: a call of one has no effect but its result.
    pure_calls: FxSet<Global>,
    uses_virtual_threads: bool,
    all_classes: Vec<Val>,
    string_fields: Vec<Global>,
    strings: FxMap<Str, Val>,

    // The method being lowered.
    name: Global,
    ret: Ty,
    yields: bool,
    fresh: Local,
    buf: Vec<Inst>,
    pos: Pos,
    scope: u32,
    unwind_handler: Option<Local>,
    current_block: Local,
    guarded: FxMap<Local, Vec<Val>>,
    preds: FxMap<Local, Vec<(Local, Edge)>>,
    /// Each block's place in the order the blocks are lowered in.
    positions: FxMap<Local, usize>,
    pub null_guards: NullGuards,
    slow_paths: [Vec<(Option<Local>, Local)>; 6],
    intrinsic_methods: FxMap<Local, Intrinsic>,
}

impl<'a> Lowering<'a> {
    pub fn new(cx: &'a mut Cx, rt: &'a Rt, linked: &'a Linked, meta: &'a Meta) -> Res<Lowering<'a>> {
        let names = Names::new(cx, rt);
        let object = match linked.infos.get(&rt.object) {
            Some(&Info::Scope(id)) => id,
            _ => return Err("java.lang.Object is not linked".to_string()),
        };
        let string = match linked.infos.get(&rt.string) {
            Some(&Info::Scope(id)) => id,
            _ => return Err("java.lang.String is not linked".to_string()),
        };
        let string_fields = meta.layouts[&string].entries.clone();
        if string_fields.len() != 4 {
            return Err("java.lang.String is expected to have 4 fields".to_string());
        }
        let uses_virtual_threads = meta.multithreading
            && match linked.infos.get(&names.virtual_thread) {
                Some(&Info::Scope(id)) => linked.scopes[id as usize].implementors.iter().any(|&c| linked.scopes[c as usize].allocated()),
                _ => false,
            };
        let all_classes = meta.traits.iter().chain(&meta.classes).map(|&s| meta.rtti_const(rt, s)).collect();
        let pure_calls = names.box_to.values().copied().collect();
        let mut lowering = Lowering {
            cx,
            rt,
            linked,
            meta,
            names,
            object,
            recursive: FxSet::default(),
            pure_calls,
            uses_virtual_threads,
            all_classes,
            string_fields,
            strings: FxMap::default(),
            name: Global::NONE,
            ret: rt.unit,
            yields: false,
            fresh: 0,
            buf: Vec::new(),
            pos: Pos::NONE,
            scope: 0,
            unwind_handler: None,
            current_block: 0,
            guarded: FxMap::default(),
            preds: FxMap::default(),
            positions: FxMap::default(),
            null_guards: NullGuards::default(),
            slow_paths: Default::default(),
            intrinsic_methods: FxMap::default(),
        };
        lowering.recursive = lowering.self_recursive();
        Ok(lowering)
    }

    /// The methods on a cycle of the graph of direct references
    /// (`ReachabilityAnalysis.references.isSelfRecursive`): a method refers to the symbols its
    /// values name and, for a method looked up on a value, to that signature in the value's
    /// class.
    fn self_recursive(&mut self) -> FxSet<Global> {
        let mut ids: FxMap<Global, u32> = FxMap::default();
        let mut edges: Vec<Vec<u32>> = Vec::new();
        let mut node = |g: Global, edges: &mut Vec<Vec<u32>>| {
            *ids.entry(g).or_insert_with(|| {
                edges.push(Vec::new());
                edges.len() as u32 - 1
            })
        };
        for defn in &self.linked.defns {
            let DefnKind::Define { insts, .. } = &defn.kind else { continue };
            let from = node(defn.name, &mut edges);
            let mut targets: Vec<Global> = Vec::new();
            let global = |v: &Val, targets: &mut Vec<Global>| {
                for_each_val(v, &mut |v| {
                    if let Val::Global(g, _) = v {
                        targets.push(*g)
                    }
                })
            };
            for inst in insts {
                match &inst.kind {
                    InstKind::Let { op: Op::Method { obj, sig }, unwind, .. } => {
                        let ty = self.rt.val_ty(self.cx, obj);
                        if let Some(owner) = self.rt.class_name(self.cx, ty) {
                            targets.push(self.cx.member(owner, *sig));
                        }
                        let mut next = unwind.clone();
                        let mut vals = Vec::new();
                        next_vals_mut(&mut next, &mut vals);
                        vals.into_iter().for_each(|v| global(v, &mut targets));
                    }
                    _ => {
                        let mut copy = inst.clone();
                        inst_vals_mut(&mut copy).into_iter().for_each(|v| global(v, &mut targets));
                    }
                }
            }
            for target in targets {
                if !self.cx.is_top(target) {
                    let to = node(target, &mut edges);
                    edges[from as usize].push(to);
                }
            }
        }
        // Tarjan's strongly connected components, without recursion.
        let n = edges.len();
        let (mut index, mut low, mut on_stack) = (vec![u32::MAX; n], vec![0u32; n], vec![false; n]);
        let mut cyclic = vec![false; n];
        let (mut stack, mut counter) = (Vec::new(), 0u32);
        for root in 0..n {
            if index[root] != u32::MAX {
                continue;
            }
            let mut work: Vec<(usize, usize)> = vec![(root, 0)];
            while let Some(&mut (v, ref mut at)) = work.last_mut() {
                if *at == 0 {
                    index[v] = counter;
                    low[v] = counter;
                    counter += 1;
                    stack.push(v);
                    on_stack[v] = true;
                }
                if let Some(&w) = edges[v].get(*at) {
                    *at += 1;
                    let w = w as usize;
                    if index[w] == u32::MAX {
                        work.push((w, 0));
                    } else if on_stack[w] {
                        low[v] = low[v].min(index[w]);
                    }
                } else {
                    work.pop();
                    if let Some(&(parent, _)) = work.last() {
                        low[parent] = low[parent].min(low[v]);
                    }
                    if low[v] == index[v] {
                        let mut component = Vec::new();
                        while let Some(w) = stack.pop() {
                            on_stack[w] = false;
                            component.push(w);
                            if w == v {
                                break;
                            }
                        }
                        if component.len() > 1 || edges[v].contains(&(v as u32)) {
                            component.into_iter().for_each(|w| cyclic[w] = true);
                        }
                    }
                }
            }
        }
        ids.into_iter().filter(|&(_, id)| cyclic[id as usize]).map(|(g, _)| g).collect()
    }

    // ---- which definitions, and their values ----

    /// `Lower.onDefns`: every definition but the classes, the methods with no body that are not
    /// C's, and the fields of classes, which the type records and the layouts stand for.
    pub fn defns(&mut self, defns: &[Defn], this_is_used: bool, out: &mut Vec<Defn>) -> Res<()> {
        for defn in defns {
            let owner_is_scope = |info: Option<&Info>| match info {
                Some(Info::Method { owner, .. } | Info::Field { owner, .. }) => matches!(self.linked.infos.get(owner), Some(Info::Scope(_))),
                _ => false,
            };
            let info = self.linked.infos.get(&defn.name);
            match &defn.kind {
                DefnKind::Class { .. } | DefnKind::Module { .. } | DefnKind::Trait { .. } => continue,
                DefnKind::Declare { .. } if !defn.attrs.is_extern && matches!(info, Some(Info::Method { .. })) && owner_is_scope(info) => continue,
                DefnKind::Var { .. } if !defn.attrs.is_extern && matches!(info, Some(Info::Field { .. })) && self.field_of_class(defn.name) => continue,
                _ => {}
            }
            out.push(self.defn(defn, this_is_used).map_err(|e| format!("{}: {}", self.show(defn.name), e))?);
        }
        Ok(())
    }

    fn field_of_class(&self, name: Global) -> bool {
        match self.linked.infos.get(&name) {
            Some(Info::Field { owner, .. }) => matches!(self.linked.infos.get(owner), Some(&Info::Scope(id)) if !self.linked.scopes[id as usize].is_trait),
            _ => false,
        }
    }

    fn show(&self, g: Global) -> String {
        let mut mangled = Vec::new();
        self.cx.mangle(g, &mut mangled);
        String::from_utf8_lossy(&mangled).into_owned()
    }

    fn defn(&mut self, defn: &Defn, this_is_used: bool) -> Res<Defn> {
        let mut out = defn.clone();
        match &mut out.kind {
            DefnKind::Var { rhs, .. } | DefnKind::Const { rhs, .. } => self.on_val(rhs)?,
            DefnKind::Define { ty, insts, .. } => {
                let Type::Function(_, ret) = self.cx.type_of(*ty) else { return Err("a method whose type is no function's".to_string()) };
                self.ret = *ret;
                self.name = defn.name;
                let lowered = self.insts(defn, insts, this_is_used)?;
                *insts = lowered;
            }
            _ => {}
        }
        Ok(out)
    }

    fn scope_of(&self, name: Global) -> Option<ScopeId> {
        match self.linked.infos.get(&name) {
            Some(&Info::Scope(id)) => Some(id),
            _ => None,
        }
    }

    /// `ScopeRef` on a type: the class or trait a reference type names.
    fn scope_ref(&self, ty: Ty) -> Option<ScopeId> {
        self.rt.class_name(self.cx, ty).and_then(|name| self.scope_of(name))
    }

    fn class_ref(&self, ty: Ty) -> Option<ScopeId> {
        self.scope_ref(ty).filter(|&id| !self.linked.scopes[id as usize].is_trait)
    }

    fn trait_ref(&self, ty: Ty) -> Option<ScopeId> {
        self.scope_ref(ty).filter(|&id| self.linked.scopes[id as usize].is_trait)
    }

    fn val_ty(&mut self, v: &Val) -> Ty {
        self.rt.val_ty(self.cx, v)
    }

    /// `Lower.genStringVal`: a string literal as the constant object it is in the program's
    /// data, a `java.lang.String` over a constant array of characters.
    fn string_val(&mut self, value: Str) -> Val {
        if let Some(v) = self.strings.get(&value) {
            return v.clone();
        }
        let chars: Vec<u16> = utf16_units(self.cx.bytes(value)).collect();
        let hash = chars.iter().fold(0i32, |h, &c| (c as i32).wrapping_add(h.wrapping_shl(5).wrapping_sub(h)));
        let length = Val::Int(chars.len() as i32);
        let char_array = self.scope_of(self.rt.arrays[1].1).map_or(Val::Null, |id| self.meta.rtti_const(self.rt, id));
        let mut array = vec![char_array];
        array.extend(self.meta.lock_word_vals());
        array.extend([length.clone(), Val::Int(2), Val::ArrayValue(self.rt.char, chars.into_iter().map(Val::Char).collect())]);
        let chars_const = Val::Const(Arc::new(Val::StructValue(array.into())));
        let string = self.scope_of(self.rt.string).map_or(Val::Null, |id| self.meta.rtti_const(self.rt, id));
        let mut fields = vec![string];
        fields.extend(self.meta.lock_word_vals());
        for field in self.string_fields.clone() {
            let at = self.rt.string_fields.iter().position(|&f| f == field).unwrap_or(0);
            fields.push(match at {
                0 => chars_const.clone(),
                1 => Val::Int(0),
                2 => length.clone(),
                _ => Val::Int(hash),
            });
        }
        let v = Val::Const(Arc::new(Val::StructValue(fields.into())));
        self.strings.insert(value, v.clone());
        v
    }

    /// `Lower.onVal`: a class named as a value is its type record, a string its constant
    /// object, the unit value the one instance of `BoxedUnit`.
    fn on_val(&mut self, v: &mut Val) -> Res<()> {
        match v {
            Val::ClassOf(_) => return Err("a class literal where no instruction can be added".to_string()),
            Val::Global(name, _) => {
                if let Some(id) = self.scope_of(*name) {
                    *v = self.meta.rtti_const(self.rt, id);
                }
            }
            Val::String(s) => *v = self.string_val(*s),
            Val::Unit => *v = self.names.unit.clone(),
            Val::StructValue(vs) | Val::ArrayValue(_, vs) => {
                for v in Arc::make_mut(vs).iter_mut() {
                    self.on_val(v)?;
                }
            }
            Val::Const(inner) => self.on_val(Arc::make_mut(inner))?,
            _ => {}
        }
        Ok(())
    }

    /// `Lower.genVal`: as `on_val`, and a class literal is its type record.
    fn gen_val(&mut self, v: &Val) -> Res<Val> {
        Ok(match v {
            Val::ClassOf(name) => match self.scope_of(*name) {
                Some(id) => self.meta.rtti_const(self.rt, id),
                None => return Err("a class literal of a class that is not linked".to_string()),
            },
            Val::Const(inner) => Val::Const(Arc::new(self.gen_val(inner)?)),
            Val::StructValue(vs) => Val::StructValue(vs.iter().map(|v| self.gen_val(v)).collect::<Res<_>>()?),
            Val::ArrayValue(ty, vs) => Val::ArrayValue(*ty, vs.iter().map(|v| self.gen_val(v)).collect::<Res<_>>()?),
            _ => {
                let mut v = v.clone();
                self.on_val(&mut v)?;
                v
            }
        })
    }

    fn gen_next(&mut self, next: &Next) -> Res<Next> {
        Ok(match next {
            Next::Unwind { exc, ty, next } => Next::Unwind { exc: *exc, ty: *ty, next: Box::new(self.gen_next(next)?) },
            Next::Case(v, next) => Next::Case(self.gen_val(v)?, Box::new(self.gen_next(next)?)),
            Next::Label(id, args) => Next::Label(*id, args.iter().map(|a| self.gen_val(a)).collect::<Res<_>>()?),
            Next::None => Next::None,
        })
    }

    // ---- the instruction builder ----

    fn fresh(&mut self) -> Local {
        self.fresh += 1;
        self.fresh
    }

    /// The handler of the instruction being lowered, taking the exception in a new local.
    fn unwind(&mut self) -> Next {
        match self.unwind_handler {
            None => Next::None,
            Some(handler) => {
                let exc = self.fresh();
                Next::Unwind { exc, ty: self.rt.throwable_ty, next: Box::new(Next::Label(handler, Box::new([Val::Local(exc, self.rt.throwable_ty)]))) }
            }
        }
    }

    fn let_as(&mut self, id: Local, op: Op, unwind: Next) -> Val {
        let ty = self.rt.op_resty(self.cx, &op);
        self.buf.push(Inst { pos: self.pos, kind: InstKind::Let { id, op, unwind, scope: self.scope } });
        Val::Local(id, ty)
    }

    /// An instruction under a new local: its handler's local is made first.
    fn emit(&mut self, op: Op) -> Val {
        let unwind = self.unwind();
        let id = self.fresh();
        self.let_as(id, op, unwind)
    }

    fn emit_as(&mut self, id: Local, op: Op) -> Val {
        let unwind = self.unwind();
        self.let_as(id, op, unwind)
    }

    fn emit_plain(&mut self, op: Op) -> Val {
        let id = self.fresh();
        self.let_as(id, op, Next::None)
    }

    fn label(&mut self, id: Local, params: &[Val]) {
        let params = params.iter().filter_map(|p| if let Val::Local(id, ty) = p { Some((*id, *ty)) } else { None }).collect();
        self.buf.push(Inst { pos: self.pos, kind: InstKind::Label { id, params } });
    }

    fn jump(&mut self, to: Local, args: Vec<Val>) {
        self.buf.push(Inst { pos: self.pos, kind: InstKind::Jump(Next::Label(to, args.into_boxed_slice())) });
    }

    fn branch(&mut self, value: Val, then: Next, otherwise: Next) {
        self.buf.push(Inst { pos: self.pos, kind: InstKind::If { value, then, otherwise } });
    }

    fn unreachable(&mut self) {
        self.buf.push(Inst { pos: self.pos, kind: InstKind::Unreachable { unwind: Next::None } });
    }

    fn to(label: Local) -> Next {
        Next::Label(label, Box::new([]))
    }

    fn call(ty: Ty, ptr: Val, args: Vec<Val>) -> Op {
        Op::Call { ty, ptr, args: args.into_boxed_slice() }
    }

    fn elem(ty: Ty, ptr: Val, indexes: Vec<Val>) -> Op {
        Op::Elem { ty, ptr, indexes: indexes.into_boxed_slice() }
    }

    fn slow_path(&mut self, kind: SlowPath) -> Local {
        if let Some(&(_, label)) = self.slow_paths[kind as usize].iter().find(|(handler, _)| *handler == self.unwind_handler) {
            return label;
        }
        let label = self.fresh();
        self.slow_paths[kind as usize].push((self.unwind_handler, label));
        label
    }

    // ---- a method ----

    fn stack_checked(&self, defn: &Defn) -> bool {
        let sig = self.cx.sig_bytes(defn.name);
        if !names::is_method(sig) {
            return false;
        }
        let owner = self.cx.owner(defn.name);
        let id = self.cx.id(owner);
        let skipped = owner == self.rt.object
            || owner == self.rt.string
            || id.starts_with(b"scala.scalanative.runtime.package")
            || id.starts_with(b"scala.scalanative.runtime.monitor.")
            || id.starts_with(b"scala.scalanative.unsafe.")
            || matches!(id, b"java.lang.StringBuilder" | b"java.lang.AbstractStringBuilder" | b"java.lang.System$");
        !skipped && self.recursive.contains(&defn.name)
    }

    fn gc_yield_point(&mut self) {
        if self.yields {
            let op = Self::call(self.names.nullary_unit, self.names.gc_yield.clone(), Vec::new());
            self.emit_plain(op);
        }
    }

    fn insts(&mut self, defn: &Defn, insts: &[Inst], this_is_used: bool) -> Res<Vec<Inst>> {
        let rt = self.rt;
        self.fresh = insts.iter().map(max_local).max().unwrap_or(0);
        let graph = Graph::new(insts)?;
        self.preds = graph.blocks.iter().enumerate().filter(|&(b, _)| graph.reachable[b]).map(|(b, &(id, _, _))| (id, graph.preds[b].iter().map(|&(p, edge)| (graph.blocks[p].0, edge)).collect())).collect();
        self.positions = graph.blocks.iter().enumerate().map(|(n, &(id, _, _))| (id, n)).collect();
        self.guarded.clear();
        self.intrinsic_methods.clear();
        self.buf = Vec::with_capacity(insts.len() * 2);
        let sig = self.cx.sig_bytes(defn.name);
        self.yields = self.meta.multithreading && !names::is_generated(sig) && (insts.len() > YIELDING_METHOD_INSTS || insts.iter().any(|i| matches!(i.kind, InstKind::Jump(Next::Label(..)))));
        let can_have_this = !(names::method_sig(sig).is_some_and(|m| m.is_static()) || names::is_clinit(sig) || names::is_extern(sig));
        let mut handlers: Vec<Inst> = Vec::new();
        let mut handler_of: FxMap<Next, Option<Local>> = FxMap::default();

        self.buf.push(insts[0].clone());
        (self.pos, self.scope) = (defn.pos, 0);
        if self.stack_checked(defn) {
            let op = Self::call(self.names.nullary_unit, self.names.stack_check.clone(), Vec::new());
            self.emit_plain(op);
        }
        for inst in insts {
            if let InstKind::Let { id, op: Op::Var(ty), unwind, scope } = &inst.kind {
                (self.pos, self.scope) = (inst.pos, *scope);
                self.let_as(*id, Op::Stackalloc { ty: *ty, n: Val::Int(1) }, unwind.clone());
            }
        }
        let InstKind::Label { id: first_label, params } = &insts[0].kind else { return Err("a method that starts with no label".to_string()) };
        self.current_block = *first_label;
        let label_positions: FxMap<Local, usize> = insts.iter().filter_map(|i| if let InstKind::Label { id, .. } = &i.kind { Some(*id) } else { None }).enumerate().map(|(n, id)| (id, n)).collect();
        let mut block_position = 0usize;
        // A reflective proxy checks its receiver: it is the one kind of method whose first
        // parameter is the very value its instructions use.
        if this_is_used && can_have_this {
            if let Some(&(this, ty)) = params.first() {
                if matches!(self.cx.type_of(ty), Type::Ref { nullable: true, .. }) {
                    (self.pos, self.scope) = (defn.pos, 0);
                    self.unwind_handler = None;
                    self.guard_not_null(&Val::Local(this, ty))?;
                }
            }
        }
        let mut last_scope = 0u32;
        for inst in &insts[1..] {
            let mut handler = |this: &mut Self, unwind: &Next, pos: Pos| -> Option<Local> {
                if let Some(&h) = handler_of.get(unwind) {
                    return h;
                }
                let h = match unwind {
                    Next::Unwind { exc, ty, next } => {
                        let h = this.fresh();
                        handlers.push(Inst { pos, kind: InstKind::Label { id: h, params: Box::new([(*exc, *ty)]) } });
                        handlers.push(Inst { pos, kind: InstKind::Jump((**next).clone()) });
                        Some(h)
                    }
                    _ => None,
                };
                handler_of.insert(unwind.clone(), h);
                h
            };
            match &inst.kind {
                InstKind::Let { id, op, unwind, scope } => {
                    self.unwind_handler = handler(self, unwind, inst.pos);
                    last_scope = *scope;
                    (self.pos, self.scope) = (inst.pos, last_scope);
                    self.gen_let(*id, op)?;
                }
                InstKind::Throw { value, unwind } => {
                    self.unwind_handler = handler(self, unwind, inst.pos);
                    (self.pos, self.scope) = (inst.pos, last_scope);
                    self.guard_not_null(value)?;
                    match unwind {
                        Next::Unwind { next, .. } => {
                            let to = next_label(next).ok_or("a handler with no label")?;
                            self.jump(to, vec![value.clone()]);
                        }
                        _ => {
                            let id = self.fresh();
                            let op = Self::call(self.names.throw_ty, self.names.throw.clone(), vec![value.clone()]);
                            self.gen_op(id, &op)?;
                            self.unreachable();
                        }
                    }
                }
                InstKind::Unreachable { unwind } => {
                    self.unwind_handler = handler(self, unwind, inst.pos);
                    self.pos = inst.pos;
                    self.gen_unreachable();
                }
                InstKind::Ret(value) => {
                    (self.pos, self.scope) = (inst.pos, last_scope);
                    if names::is_ctor(self.cx.sig_bytes(defn.name)) && self.has_safe_publish_fields(self.cx.owner(defn.name)) {
                        self.emit_plain(Op::Fence(MemoryOrder::Release));
                    }
                    self.gc_yield_point();
                    let value = if self.val_ty(value) == rt.unit { self.optionally_boxed_unit() } else { self.gen_val(value)? };
                    self.buf.push(Inst { pos: inst.pos, kind: InstKind::Ret(value) });
                }
                InstKind::Jump(next) => {
                    (self.pos, self.scope) = (inst.pos, last_scope);
                    if let Next::Label(target, _) = next {
                        if label_positions.get(target).is_some_and(|&p| p <= block_position) {
                            self.gc_yield_point();
                        }
                    }
                    let next = self.gen_next(next)?;
                    self.buf.push(Inst { pos: inst.pos, kind: InstKind::Jump(next) });
                }
                InstKind::Label { id, .. } => {
                    block_position = label_positions[id];
                    self.current_block = *id;
                    self.buf.push(inst.clone());
                }
                _ => self.buf.push(inst.clone()),
            }
        }
        (self.pos, self.scope) = (Pos::NONE, last_scope);
        for kind in SLOW_PATHS {
            let mut paths = std::mem::take(&mut self.slow_paths[kind as usize]);
            paths.sort_by_key(|&(_, label)| label);
            for (handler, label) in paths {
                self.unwind_handler = handler;
                let (ptr, int) = (rt.ptr, rt.int);
                let params: Vec<Val> = match kind {
                    SlowPath::ClassCast => vec![Val::Local(self.fresh(), ptr), Val::Local(self.fresh(), ptr)],
                    SlowPath::OutOfBounds => vec![Val::Local(self.fresh(), int), Val::Local(self.fresh(), int)],
                    SlowPath::NoSuchMethod => vec![Val::Local(self.fresh(), ptr)],
                    _ => Vec::new(),
                };
                self.label(label, &params);
                let mut args = vec![Val::Null];
                if kind == SlowPath::ClassCast {
                    let from = self.emit(Op::Load { ty: ptr, ptr: params[0].clone(), order: None });
                    args.extend([from, params[1].clone()]);
                } else {
                    args.extend(params);
                }
                let (thrower, ty) = self.names.throwers[kind as usize].clone();
                self.emit(Self::call(ty, thrower, args));
                self.unreachable();
            }
        }
        self.buf.extend(handlers);
        let mut lowered = std::mem::take(&mut self.buf);
        for inst in &mut lowered {
            if let InstKind::Ret(v) = &inst.kind {
                if self.val_ty(v) == rt.unit {
                    inst.kind = InstKind::Ret(self.optionally_boxed_unit());
                    continue;
                }
            }
            for v in inst_vals_mut(inst) {
                self.on_val(v)?;
            }
        }
        self.eliminate_dead_code(lowered)
    }

    fn optionally_boxed_unit(&self) -> Val {
        if self.ret == self.rt.unit {
            Val::Unit
        } else {
            self.names.unit.clone()
        }
    }

    fn field_attrs(&self, field: Global) -> Option<&Attrs> {
        self.meta.defns.get(&field).map(|&i| &self.linked.defns[i].attrs)
    }

    /// `Class.hasFinalSafePublishFields`: a constructor of such a class ends with a fence.
    fn has_safe_publish_fields(&self, owner: Global) -> bool {
        match self.scope_of(owner).and_then(|id| self.meta.layouts.get(&id)) {
            Some(layout) => layout.entries.iter().any(|&f| self.field_attrs(f).is_some_and(|a| a.is_final && a.is_safe_publish)),
            None => false,
        }
    }

    /// `UseDef.eliminateDeadCode`: the blocks a path comes to, without the instructions that
    /// have no effect and whose result nothing live uses.
    fn eliminate_dead_code(&mut self, insts: Vec<Inst>) -> Res<Vec<Inst>> {
        let graph = Graph::new(&insts)?;
        // A definition is a block (by its label) or a local; a block's parameters are locals.
        let mut ids: FxMap<Local, u32> = FxMap::default();
        let mut deps: Vec<Vec<u32>> = Vec::new();
        let mut define = |local: Local, deps: &mut Vec<Vec<u32>>| -> Res<u32> {
            let id = deps.len() as u32;
            if ids.insert(local, id).is_some() {
                return Err(format!("the local %{} is defined twice", local));
            }
            deps.push(Vec::new());
            Ok(id)
        };
        let exc = |next: &Next| if let Next::Unwind { exc, .. } = next { Some(*exc) } else { None };
        for (b, &(label, start, end)) in graph.blocks.iter().enumerate() {
            if !graph.reachable[b] {
                continue;
            }
            if let InstKind::Label { params, .. } = &insts[start - 1].kind {
                for &(param, _) in params.iter() {
                    define(param, &mut deps)?;
                }
            }
            define(label, &mut deps)?;
            for inst in &insts[start..end] {
                match &inst.kind {
                    InstKind::Let { id, unwind, .. } => {
                        define(*id, &mut deps)?;
                        if let Some(e) = exc(unwind) {
                            define(e, &mut deps)?;
                        }
                    }
                    InstKind::Throw { unwind, .. } | InstKind::Unreachable { unwind } => {
                        if let Some(e) = exc(unwind) {
                            define(e, &mut deps)?;
                        }
                    }
                    _ => {}
                }
            }
        }
        let lookup = |local: Local| ids.get(&local).copied().ok_or_else(|| format!("the local %{} is used and never defined", local));
        for (b, &(label, start, end)) in graph.blocks.iter().enumerate() {
            if !graph.reachable[b] {
                continue;
            }
            let block = lookup(label)?;
            for inst in &insts[start..end] {
                let mut used = Vec::new();
                for dep in inst_deps(inst) {
                    used.push(lookup(dep)?);
                }
                match &inst.kind {
                    InstKind::Let { id, op, .. } => {
                        let this = lookup(*id)?;
                        deps[this as usize].extend(used);
                        let pure = match op {
                            Op::Call { ptr: Val::Global(name, _), .. } => self.pure_calls.contains(name),
                            Op::Elem { .. } | Op::Extract { .. } | Op::Insert { .. } | Op::Comp { .. } | Op::Conv { .. } | Op::Is { .. } | Op::Copy(_) | Op::SizeOf(_) => true,
                            Op::Bin { bin: Bin::Sdiv | Bin::Udiv | Bin::Srem | Bin::Urem, ty, r, .. } if matches!(self.cx.type_of(*ty), Type::Size | Type::Char | Type::Byte | Type::Short | Type::Int | Type::Long | Type::Int128) => is_canonical(r) && !is_zero(r),
                            Op::Bin { .. } => true,
                            _ => false,
                        };
                        if !pure {
                            deps[block as usize].push(this);
                        }
                    }
                    _ => deps[block as usize].extend(used),
                }
            }
        }
        let mut alive = vec![false; deps.len()];
        let mut todo = vec![lookup(graph.blocks[0].0)?];
        while let Some(d) = todo.pop() {
            if !std::mem::replace(&mut alive[d as usize], true) {
                todo.extend(&deps[d as usize]);
            }
        }
        let mut keep = vec![false; insts.len()];
        for (b, &(label, start, end)) in graph.blocks.iter().enumerate() {
            if !graph.reachable[b] || !alive[lookup(label)? as usize] {
                continue;
            }
            keep[start - 1] = true;
            for (i, inst) in insts[start..end].iter().enumerate() {
                keep[start + i] = match &inst.kind {
                    InstKind::Let { id, .. } => alive[lookup(*id)? as usize],
                    _ => true,
                };
            }
        }
        Ok(insts.into_iter().zip(keep).filter(|&(_, k)| k).map(|(inst, _)| inst).collect())
    }

    // ---- an instruction ----

    fn gen_let(&mut self, id: Local, op: &Op) -> Res<()> {
        let ty = self.rt.op_resty(self.cx, op);
        if ty == self.rt.unit {
            let fresh = self.fresh();
            self.gen_op(fresh, op)?;
            self.emit_as(id, Op::Copy(self.names.unit.clone()));
        } else if ty == self.rt.nothing {
            let fresh = self.fresh();
            self.gen_op(fresh, op)?;
            self.gen_unreachable();
            let after = self.fresh();
            self.label(after, &[Val::Local(id, ty)]);
        } else {
            self.gen_op(id, op)?;
        }
        Ok(())
    }

    fn gen_unreachable(&mut self) {
        let fail = self.slow_path(SlowPath::Undefined);
        self.jump(fail, Vec::new());
    }

    fn gen_op(&mut self, n: Local, op: &Op) -> Res<()> {
        let rt = self.rt;
        match op {
            Op::Field { obj, name } => {
                let elem = self.field_elem(obj, *name)?;
                self.emit_as(n, Op::Copy(elem));
            }
            Op::Fieldload { ty, obj, name } => {
                let attrs = self.field_attrs(*name).ok_or("a field that is not linked")?;
                let order = if attrs.is_volatile { MemoryOrder::SeqCst } else { MemoryOrder::Unordered };
                if attrs.is_final && attrs.is_safe_publish {
                    self.emit_plain(Op::Fence(MemoryOrder::Acquire));
                }
                let obj = self.gen_val(obj)?;
                let elem = self.field_elem(&obj, *name)?;
                self.load(n, *ty, elem, Some(order))?;
            }
            Op::Fieldstore { ty, obj, name, value } => {
                let attrs = self.field_attrs(*name).ok_or("a field that is not linked")?;
                let order = if attrs.is_volatile { MemoryOrder::SeqCst } else { MemoryOrder::Unordered };
                let obj = self.gen_val(obj)?;
                let elem = self.field_elem(&obj, *name)?;
                self.store(n, *ty, elem, value.clone(), Some(order))?;
            }
            Op::Load { ty, ptr, order } => self.load(n, *ty, ptr.clone(), *order)?,
            Op::Store { ty, ptr, value, order } => self.store(n, *ty, ptr.clone(), value.clone(), *order)?,
            Op::Method { obj, sig } => self.method(n, obj, *sig)?,
            Op::Dynmethod { obj, sig } => self.dynmethod(n, obj, *sig)?,
            Op::Is { ty, obj } => {
                if matches!(obj, Val::Null | Val::Zero(_)) {
                    self.emit_as(n, Op::Copy(Val::False));
                } else {
                    let obj = self.gen_val(obj)?;
                    let (is_null, check, result) = (self.fresh(), self.fresh(), self.fresh());
                    let null = self.emit(Op::Comp { comp: Comp::Ieq, ty: rt.ptr, l: obj.clone(), r: Val::Null });
                    self.branch(null, Self::to(is_null), Self::to(check));
                    self.label(is_null, &[]);
                    self.jump(result, vec![Val::False]);
                    self.label(check, &[]);
                    let is = self.is_instance(*ty, &obj)?;
                    self.jump(result, vec![is]);
                    self.label(result, &[Val::Local(n, rt.bool)]);
                }
            }
            Op::As { ty, obj } => {
                let from = self.val_ty(obj);
                if from == rt.null {
                    self.emit_as(n, Op::Copy(Val::Null));
                } else if self.rt.class_name(self.cx, *ty).is_some() && self.rt.class_name(self.cx, from).is_some() {
                    let v = self.gen_val(obj)?;
                    let (check, cast) = (self.fresh(), self.fresh());
                    let fail = self.slow_path(SlowPath::ClassCast);
                    let v_ty = self.val_ty(&v);
                    let null = self.emit(Op::Comp { comp: Comp::Ieq, ty: v_ty, l: v.clone(), r: Val::Null });
                    self.branch(null, Self::to(cast), Self::to(check));
                    self.label(check, &[]);
                    let is = self.is_instance(*ty, &v)?;
                    let to = self.scope_ref(*ty).ok_or("a cast to a class that is not linked")?;
                    self.branch(is, Self::to(cast), Next::Label(fail, Box::new([v.clone(), self.meta.rtti_const(rt, to)])));
                    self.label(cast, &[]);
                    self.emit_as(n, Op::Copy(v));
                } else {
                    return Err("a cast between types that are no references".to_string());
                }
            }
            Op::SizeOf(ty) => {
                let size = match self.class_ref(*ty) {
                    Some(cls) if *ty != rt.unit => self.meta.layouts[&cls].size,
                    _ => meta::size_of(self.cx, *ty),
                };
                self.emit_as(n, Op::Copy(Val::Size(size)));
            }
            Op::AlignmentOf(ty) => {
                let alignment = meta::alignment_of(self.cx, *ty);
                self.emit_as(n, Op::Copy(Val::Size(alignment)));
            }
            Op::Classalloc { name, zone } => {
                let cls = self.scope_of(*name).filter(|&id| !self.linked.scopes[id as usize].is_trait).ok_or("an allocation of a class that is not linked")?;
                let zone = match zone {
                    Some(z) => Some(self.gen_val(z)?),
                    None => None,
                };
                let size = self.meta.layouts[&cls].size;
                let type_record = self.meta.rtti_const(rt, cls);
                match zone {
                    Some(zone) => {
                        let method = self.fresh();
                        self.method(method, &zone, self.names.safe_zone_alloc_sig)?;
                        self.emit_as(n, Self::call(self.names.safe_zone_alloc_ty, Val::Local(method, rt.ptr), vec![zone, type_record, Val::Size(size)]));
                    }
                    None => {
                        let alloc = if size < LARGE_OBJECT_MIN_SIZE { self.names.alloc_small } else { self.names.alloc_large };
                        let cls_ty = self.cx.class_ty(*name);
                        let ty = self.cx.ty(Type::Function(Box::new([rt.ptr, rt.size]), cls_ty));
                        self.emit_as(n, Self::call(ty, Val::Global(alloc, self.names.alloc_ty), vec![type_record, Val::Size(size)]));
                    }
                }
            }
            Op::Conv { conv, ty, value } => self.conv(n, *conv, *ty, value)?,
            Op::Call { ty, ptr, args } => {
                let intrinsic = match ptr {
                    Val::Global(name, _) if !self.cx.is_top(*name) => self.intrinsic(self.cx.owner(*name), self.cx.sig(*name).unwrap_or(Str::EMPTY)),
                    Val::Local(id, _) => self.intrinsic_methods.get(id).copied(),
                    _ => None,
                };
                match intrinsic {
                    Some(Intrinsic::LoadAllClasses) => {
                        let all = Val::ArrayValue(rt.class_ty, self.all_classes.clone().into());
                        self.arrayalloc(n, rt.class_ty, &all, &None)?;
                    }
                    Some(kind) => {
                        let signed = kind == Intrinsic::MultiplyHigh;
                        let (l, r) = match &args[..] {
                            [l, r] | [_, l, r] => (l.clone(), r.clone()),
                            _ => return Err("multiplyHigh with other than two arguments".to_string()),
                        };
                        let wide = self.cx.ty(Type::Int128);
                        let conv = if signed { Conv::Sext } else { Conv::Zext };
                        let l = self.emit_plain(Op::Conv { conv, ty: wide, value: l });
                        let r = self.emit_plain(Op::Conv { conv, ty: wide, value: r });
                        let product = self.emit_plain(Op::Bin { bin: Bin::Imul, ty: wide, l, r });
                        let high = self.emit_plain(Op::Bin { bin: if signed { Bin::Ashr } else { Bin::Lshr }, ty: wide, l: product, r: Val::Int(64) });
                        self.let_as(n, Op::Conv { conv: Conv::Trunc, ty: rt.long, value: high }, Next::None);
                    }
                    None => {
                        // The runtime's enterMonitor checks nothing (nativelib 0.5.12,
                        // scala/scalanative/runtime/package.scala 25-30) and Scala Native's
                        // lowering writes no check before it, so `synchronized` on null would run
                        // its body: teq's rule checks the receiver as the JVM's monitorenter does.
                        // The check spares no later one, so that every check of Scala Native's
                        // listing stays in teq's (docs/NATIVE.md, "The lowered listings' oracle").
                        if self.null_guards == NullGuards::Sound && matches!(ptr, Val::Global(name, _) if *name == self.names.enter_monitor) {
                            if let Some(obj) = args.first() {
                                self.check_not_null(obj, false)?;
                            }
                        }
                        self.call_op(n, *ty, ptr, args)?
                    }
                }
            }
            Op::Comp { comp, ty, l, r } => {
                let (l, r) = (self.gen_val(l)?, self.gen_val(r)?);
                self.emit_as(n, Op::Comp { comp: *comp, ty: *ty, l, r });
            }
            Op::Bin { bin, ty, l, r } => self.bin(n, *bin, *ty, l, r),
            Op::Box { ty, obj } => {
                let from = self.gen_val(obj)?;
                let cls = self.rt.class_name(self.cx, *ty).ok_or("a box of no class")?;
                let (method, held) = (*self.names.box_to.get(&cls).ok_or("a box of a class that boxes nothing")?, self.rt.unboxed(cls).unwrap_or(rt.ptr));
                let module = self.cx.class_ty(self.cx.owner(method));
                let fn_ty = self.cx.ty(Type::Function(Box::new([module, held]), *ty));
                self.emit_as(n, Self::call(fn_ty, Val::Global(method, rt.ptr), vec![Val::Null, from]));
            }
            Op::Unbox { ty, obj } => {
                let from = self.gen_val(obj)?;
                let cls = self.rt.class_name(self.cx, *ty).ok_or("an unboxing of no class")?;
                let (method, held) = (*self.names.unbox_to.get(&cls).ok_or("an unboxing of a class that boxes nothing")?, self.rt.unboxed(cls).unwrap_or(rt.ptr));
                let module = self.cx.class_ty(self.cx.owner(method));
                let fn_ty = self.cx.ty(Type::Function(Box::new([module, *ty]), held));
                self.emit_as(n, Self::call(fn_ty, Val::Global(method, rt.ptr), vec![Val::Null, from]));
            }
            Op::Module(name) => {
                self.module(n, *name);
            }
            Op::Var(_) => {}
            Op::Varload(Val::Local(slot, var)) => {
                let Type::Var(ty) = self.cx.type_of(*var) else { return Err("a load of what is no variable".to_string()) };
                self.load(n, *ty, Val::Local(*slot, rt.ptr), None)?;
            }
            Op::Varstore { slot: Val::Local(slot, var), value } => {
                let Type::Var(ty) = self.cx.type_of(*var) else { return Err("a store to what is no variable".to_string()) };
                let (ty, value) = (*ty, self.gen_val(value)?);
                self.store(n, ty, Val::Local(*slot, rt.ptr), value, None)?;
            }
            Op::Arrayalloc { ty, init, zone } => self.arrayalloc(n, *ty, init, zone)?,
            Op::Arrayload { ty, arr, idx } => {
                let arr = self.gen_val(arr)?;
                let len = self.fresh();
                self.arraylength(len, &arr)?;
                self.guard_in_bounds(idx, Val::Local(len, rt.int));
                let elem = self.array_elem(*ty, arr, idx.clone());
                self.load(n, *ty, elem, Some(MemoryOrder::Unordered))?;
            }
            Op::Arraystore { ty, arr, idx, value } => {
                let len = self.fresh();
                let value = self.gen_val(value)?;
                self.arraylength(len, arr)?;
                self.guard_in_bounds(idx, Val::Local(len, rt.int));
                let elem = self.array_elem(*ty, arr.clone(), idx.clone());
                self.store(n, *ty, elem, value, Some(MemoryOrder::Unordered))?;
            }
            Op::Arraylength(arr) => self.arraylength(n, arr)?,
            Op::Stackalloc { ty, n: size } => self.stackalloc(n, *ty, size),
            Op::Copy(v) => {
                let v = self.gen_val(v)?;
                self.emit_as(n, Op::Copy(v));
            }
            _ => {
                self.emit_as(n, op.clone());
            }
        }
        Ok(())
    }

    fn array_elem(&mut self, of: Ty, arr: Val, idx: Val) -> Val {
        let values = self.cx.ty(Type::ArrayValue(of, 0));
        let layout = self.cx.ty(Type::StructValue(Box::new([self.meta.array_header, values])));
        self.emit(Self::elem(layout, arr, vec![Val::Int(0), Val::Int(1), idx]))
    }

    fn is_null_guarded(&self, v: &Val) -> bool {
        self.guarded_in(self.current_block, v) || self.checked_into(self.current_block, v, &mut FxMap::default())
    }

    fn guarded_in(&self, block: Local, v: &Val) -> bool {
        self.guarded.get(&block).is_some_and(|vals| vals.contains(v))
    }

    /// Whether every way into `block` passes `way`, told the block it comes from and how; a block
    /// no way comes to has none that passes.
    fn every_way_in(&self, block: Local, way: &mut dyn FnMut(Local, Edge) -> bool) -> bool {
        let ways = self.preds.get(&block).map_or(&[][..], Vec::as_slice);
        !ways.is_empty() && ways.iter().all(|&(from, edge)| way(from, edge))
    }

    /// Whether `v` counts as checked for null on entering `block`, by the rule `null_guards`
    /// names; `known` holds the blocks already answered for `v`. The sound rule: every way in is a
    /// jump from a block lowered before this one (blocks are lowered in their order, so a loop's
    /// back edge or a later block brings nothing) that had checked it by its end; an unwinding
    /// brings nothing, since a handler is entered from anywhere in the block that throws, before a
    /// check that block makes or at the check itself. Scala Native's rule (`Lower.isNullGuarded`,
    /// tools 0.5.12, Lower.scala 62-82, over `ControlFlow.Graph`, ControlFlow.scala 97-101, which
    /// counts an unwinding as a way in): every way in comes from a block with SOME block behind it,
    /// on any path back, that checked it. That rule is unsound: a value checked on one branch counts
    /// as checked after the join, and one checked in a `try` in its handler and after it, though
    /// the check itself may be what threw, so a program reading a null there reads through it. It
    /// is kept as the reference the lowered listings are held to (docs/NATIVE.md, "The lowered
    /// listings' oracle"), which `teq nir --lower` alone takes.
    fn checked_into(&self, block: Local, v: &Val, known: &mut FxMap<Local, bool>) -> bool {
        match self.null_guards {
            NullGuards::Sound => {
                if let Some(&checked) = known.get(&block) {
                    return checked;
                }
                let at = self.positions[&block];
                let checked = self.every_way_in(block, &mut |from, edge| {
                    edge == Edge::Jump && self.positions[&from] < at && (self.guarded_in(from, v) || self.checked_into(from, v, known))
                });
                known.insert(block, checked);
                checked
            }
            NullGuards::ScalaNative => self.every_way_in(block, &mut |from, _| self.guarded_behind(from, v)),
        }
    }

    /// Whether a block on some path back from `from`, `from` itself first, checked `v`
    /// (`Lower.findNonRecursive`, Lower.scala 62-74).
    fn guarded_behind(&self, from: Local, v: &Val) -> bool {
        let mut visited: FxSet<Local> = FxSet::default();
        let mut todo = vec![from];
        while let Some(block) = todo.pop() {
            if self.guarded_in(block, v) {
                return true;
            }
            if visited.insert(block) {
                todo.extend(self.preds.get(&block).into_iter().flatten().map(|&(b, _)| b));
            }
        }
        false
    }

    fn guard_not_null(&mut self, obj: &Val) -> Res<()> {
        self.check_not_null(obj, true)
    }

    /// A check of `obj` for null where the rule does not already know it checked; `recorded`: whether the
    /// rule may then take it as checked, so that a later check of it is spared.
    fn check_not_null(&mut self, obj: &Val, recorded: bool) -> Res<()> {
        let ty = self.val_ty(obj);
        let never_null = match self.cx.type_of(ty) {
            Type::Unit => true,
            Type::Array { nullable, .. } | Type::Ref { nullable, .. } => !nullable,
            _ => false,
        };
        if never_null || self.is_null_guarded(obj) {
            return Ok(());
        }
        if recorded {
            self.guarded.entry(self.current_block).or_default().push(obj.clone());
        }
        let v = self.gen_val(obj)?;
        let not_null = self.fresh();
        let is_null = self.slow_path(SlowPath::NullPointer);
        let v_ty = self.val_ty(&v);
        let cond = self.emit(Op::Comp { comp: Comp::Ine, ty: v_ty, l: v, r: Val::Null });
        self.branch(cond, Self::to(not_null), Self::to(is_null));
        self.label(not_null, &[]);
        Ok(())
    }

    fn guard_in_bounds(&mut self, idx: &Val, len: Val) {
        let (int, bool) = (self.rt.int, self.rt.bool);
        let in_bounds = self.fresh();
        let out_of_bounds = self.slow_path(SlowPath::OutOfBounds);
        let ge = self.emit(Op::Comp { comp: Comp::Sge, ty: int, l: idx.clone(), r: Val::Int(0) });
        let lt = self.emit(Op::Comp { comp: Comp::Slt, ty: int, l: idx.clone(), r: len.clone() });
        let both = self.emit(Op::Bin { bin: Bin::And, ty: bool, l: ge, r: lt });
        self.branch(both, Self::to(in_bounds), Next::Label(out_of_bounds, Box::new([idx.clone(), len])));
        self.label(in_bounds, &[]);
    }

    fn field_elem(&mut self, obj: &Val, name: Global) -> Res<Val> {
        let v = self.gen_val(obj)?;
        let cls = match self.linked.infos.get(&name) {
            Some(Info::Field { owner, .. }) => self.scope_of(*owner).filter(|&id| !self.linked.scopes[id as usize].is_trait),
            _ => None,
        }
        .ok_or("a field that is not linked")?;
        let layout = &self.meta.layouts[&cls];
        let (ty, index) = (layout.struct_ty, layout.index(name).ok_or("a field its class has not")?);
        self.guard_not_null(&v)?;
        Ok(self.emit(Self::elem(ty, v, vec![Val::Int(0), Val::Int(index)])))
    }

    /// A load; an ordered load of a boolean goes through a byte.
    fn load(&mut self, n: Local, ty: Ty, ptr: Val, order: Option<MemoryOrder>) -> Res<()> {
        let rt = self.rt;
        if ty == rt.bool && order.is_some() {
            let byte = self.fresh();
            self.load(byte, rt.byte, ptr, order)?;
            self.conv(n, Conv::Trunc, rt.bool, &Val::Local(byte, rt.byte))
        } else {
            let ptr = self.gen_val(&ptr)?;
            self.emit_as(n, Op::Load { ty, ptr, order });
            Ok(())
        }
    }

    fn store(&mut self, n: Local, ty: Ty, ptr: Val, value: Val, order: Option<MemoryOrder>) -> Res<()> {
        let rt = self.rt;
        if ty == rt.bool && order.is_some() {
            let byte = self.fresh();
            self.conv(byte, Conv::Zext, rt.byte, &value)?;
            self.store(n, rt.byte, ptr, Val::Local(byte, rt.byte), order)
        } else {
            let (ptr, value) = (self.gen_val(&ptr)?, self.gen_val(&value)?);
            self.emit_as(n, Op::Store { ty, ptr, value, order });
            Ok(())
        }
    }

    /// The intrinsic a method of the runtime stands for (`IntrinsicCall`).
    fn intrinsic(&self, owner: Global, sig: Str) -> Option<Intrinsic> {
        let id = names::method_sig(self.cx.bytes(sig))?.id;
        if owner == self.names.linked_classes && id == b"loadAll" {
            Some(Intrinsic::LoadAllClasses)
        } else if owner == self.names.intrinsics && id == b"multiplyHigh" {
            Some(Intrinsic::MultiplyHigh)
        } else if owner == self.names.intrinsics && id == b"unsignedMultiplyHigh" {
            Some(Intrinsic::UnsignedMultiplyHigh)
        } else {
            None
        }
    }

    fn call_op(&mut self, n: Local, ty: Ty, ptr: &Val, args: &[Val]) -> Res<()> {
        let attrs = match ptr {
            Val::Global(name, _) if !self.cx.is_top(*name) && self.meta.multithreading => self.meta.defns.get(name).map(|&i| &self.linked.defns[i].attrs).filter(|a| a.is_extern),
            _ => None,
        };
        let switches = attrs.is_some_and(|a| a.is_blocking);
        let compensates = self.uses_virtual_threads && attrs.is_some_and(|a| a.is_blocking);
        let attempted = if compensates { Some(self.emit(Self::call(self.names.blocker_begin_ty, self.names.blocker_begin.clone(), Vec::new()))) } else { None };
        if switches {
            self.emit(Self::call(self.names.set_thread_state_ty, self.names.set_thread_state.clone(), vec![Val::Int(1)]));
        }
        let ptr = self.gen_val(ptr)?;
        let args = args.iter().map(|a| self.gen_val(a)).collect::<Res<Vec<Val>>>()?;
        self.emit_as(n, Self::call(ty, ptr, args));
        if switches {
            self.gc_yield_point();
            self.emit(Self::call(self.names.set_thread_state_ty, self.names.set_thread_state.clone(), vec![Val::Int(0)]));
        }
        if let Some(attempted) = attempted {
            self.emit_plain(Self::call(self.names.blocker_end_ty, self.names.blocker_end.clone(), vec![attempted]));
        }
        Ok(())
    }

    fn method(&mut self, n: Local, v: &Val, sig: Str) -> Res<()> {
        let rt = self.rt;
        let ty = self.val_ty(v);
        if let Some(owner) = self.rt.class_name(self.cx, ty) {
            if let Some(intrinsic) = self.intrinsic(owner, sig) {
                self.intrinsic_methods.insert(n, intrinsic);
                return Ok(());
            }
        }
        let obj = self.gen_val(v)?;
        if ty == rt.null {
            let (thrower, thrower_ty) = self.names.throwers[SlowPath::NullPointer as usize].clone();
            self.emit_as(n, Self::call(thrower_ty, thrower, vec![Val::Null]));
            self.unreachable();
            return Ok(());
        }
        let is_virtual = names::is_virtual(self.cx.bytes(sig));
        if let Some(cls) = self.class_ref(ty) {
            let scope = &self.linked.scopes[cls as usize];
            if !is_virtual || !scope.calls.contains(&sig) {
                let method = scope.resolve(sig).ok_or_else(|| format!("no method {} in {}", String::from_utf8_lossy(self.cx.bytes(sig)), self.show(scope.name)))?;
                self.emit_as(n, Op::Copy(Val::Global(method, rt.ptr)));
                return Ok(());
            }
        }
        let scope = self.scope_ref(ty).ok_or("a method of a type that is no class")?;
        self.guard_not_null(&obj)?;
        let targets = reach::targets(self.cx, &self.linked.scopes, scope, sig);
        match targets[..] {
            [] => {
                self.emit_as(n, Op::Copy(Val::Null));
            }
            [target] => {
                self.emit_as(n, Op::Copy(Val::Global(target, rt.ptr)));
            }
            _ => {
                let obj_ty = self.val_ty(&obj);
                let lookup_class = match self.class_ref(obj_ty) {
                    Some(cls) => Some(cls),
                    None if self.trait_ref(obj_ty).is_some() && self.linked.scopes[self.object as usize].calls.contains(&sig) => Some(self.object),
                    None => None,
                };
                match (lookup_class, self.trait_ref(obj_ty)) {
                    (Some(cls), _) => {
                        let index = self.meta.vtables[&cls].index(sig).ok_or_else(|| format!("the virtual table of {} has no {}", self.show(self.linked.scopes[cls as usize].name), String::from_utf8_lossy(self.cx.bytes(sig))))?;
                        let type_ptr = self.emit(Op::Load { ty: rt.ptr, ptr: obj, order: None });
                        let slot = self.emit(Self::elem(self.meta.rtti_structs[cls as usize], type_ptr, vec![Val::Int(0), Val::Int(self.meta.class_rtti_vtable_idx), Val::Int(index)]));
                        self.emit_as(n, Op::Load { ty: rt.ptr, ptr: slot, order: None });
                    }
                    (None, Some(trt)) => {
                        let index = self.meta.methods[trt as usize].iter().position(|&s| s == sig).ok_or("a trait's table has not the method")?;
                        let method_idx = Val::Int(index as i32);
                        let rtti = self.emit(Op::Load { ty: rt.ptr, ptr: obj, order: None });
                        self.itable_lookup(trt, false, Some(n), rtti, rt.ptr, Some(method_idx));
                    }
                    (None, None) => return Err("a virtual call on a value that is no object".to_string()),
                }
            }
        }
        Ok(())
    }

    /// A lookup in the object's interface table: by the trait id's low bits where the table
    /// is one of those, else by the C runtime's search. `method` asks for the trait's method at
    /// that index, none for whether the object's class has the trait.
    fn itable_lookup(&mut self, trt: ScopeId, may_be_not_found: bool, result: Option<Local>, rtti: Val, result_ty: Ty, method: Option<Val>) -> Val {
        let rt = self.rt;
        let (int, ptr) = (rt.int, rt.ptr);
        let trait_id = Val::Int(self.meta.ids[trt as usize]);
        let path = |idx: i32| vec![Val::Int(0), Val::Int(idx)];
        let size_ptr = self.emit(Self::elem(self.meta.class_rtti_layout, rtti.clone(), path(self.meta.class_rtti_itable_size_idx)));
        let size = self.emit(Op::Load { ty: int, ptr: size_ptr, order: None });
        let fast = |this: &mut Self, result: Local| -> Val {
            let itables_ptr = this.emit(Self::elem(this.meta.class_rtti_layout, rtti.clone(), path(this.meta.class_rtti_itables_idx)));
            let itables = this.emit(Op::Load { ty: ptr, ptr: itables_ptr, order: None });
            let index = this.emit(Op::Bin { bin: Bin::And, ty: int, l: trait_id.clone(), r: size.clone() });
            match &method {
                Some(method_idx) => {
                    let entry = this.emit(Self::elem(this.meta.itable_entry, itables, vec![index, Val::Int(1)]));
                    let table = this.emit(Op::Load { ty: ptr, ptr: entry, order: None });
                    let slot = this.emit(Self::elem(ptr, table, vec![method_idx.clone()]));
                    this.emit_as(result, Op::Load { ty: ptr, ptr: slot, order: None })
                }
                None => {
                    let entry = this.emit(Self::elem(this.meta.itable_entry, itables, vec![index, Val::Int(0)]));
                    let id = this.emit(Op::Load { ty: int, ptr: entry, order: None });
                    this.emit(Op::Comp { comp: Comp::Ieq, ty: int, l: trait_id.clone(), r: id })
                }
            }
        };
        let slow = |this: &mut Self| -> Val {
            match &method {
                Some(method_idx) => this.emit(Self::call(this.names.trait_dispatch_ty, this.names.trait_dispatch.clone(), vec![rtti.clone(), trait_id.clone(), method_idx.clone()])),
                None => this.emit(Self::call(this.names.class_has_trait_ty, this.names.class_has_trait.clone(), vec![rtti.clone(), trait_id.clone()])),
            }
        };
        if self.meta.can_always_use_fast_itables || (!may_be_not_found && self.meta.can_use_fast_itables[trt as usize]) {
            let result = result.unwrap_or_else(|| self.fresh());
            return fast(self, result);
        }
        let (on_fast, on_slow, merge) = (self.fresh(), self.fresh(), self.fresh());
        let result = Val::Local(result.unwrap_or_else(|| self.fresh()), result_ty);
        let use_fast = self.emit(Op::Comp { comp: Comp::Sge, ty: int, l: size.clone(), r: Val::Int(0) });
        self.branch(use_fast, Self::to(on_fast), Self::to(on_slow));
        self.label(on_fast, &[]);
        let label = self.fresh();
        let found = fast(self, label);
        self.jump(merge, vec![found]);
        self.label(on_slow, &[]);
        self.fresh();
        let found = slow(self);
        self.jump(merge, vec![found]);
        self.label(merge, std::slice::from_ref(&result));
        result
    }

    fn dynmethod(&mut self, n: Local, v: &Val, sig: Str) -> Res<()> {
        let rt = self.rt;
        let ptr = rt.ptr;
        let obj = self.gen_val(v)?;
        self.guard_not_null(&obj)?;
        let index = self.linked.dynsigs.iter().position(|&s| s == sig).ok_or("a structural call of a signature the link has not")?;
        let type_ptr = self.emit(Op::Load { ty: ptr, ptr: obj, order: None });
        let map_elem = self.emit(Self::elem(self.meta.rtti_structs[self.object as usize], type_ptr, vec![Val::Int(0), Val::Int(self.meta.class_rtti_dynmap_idx)]));
        let map = self.emit(Op::Load { ty: ptr, ptr: map_elem, order: None });
        let throw_if_null = |this: &mut Self, value: Val| {
            let not_null = this.fresh();
            let no_such_method = this.slow_path(SlowPath::NoSuchMethod);
            let cond = this.emit(Op::Comp { comp: Comp::Ine, ty: ptr, l: value, r: Val::Null });
            this.branch(cond, Self::to(not_null), Next::Label(no_such_method, Box::new([Val::String(sig)])));
            this.label(not_null, &[]);
        };
        throw_if_null(self, map.clone());
        let method_ptr = self.emit(Self::call(self.names.dyndispatch_ty, self.names.dyndispatch.clone(), vec![map, Val::Int(index as i32)]));
        throw_if_null(self, method_ptr.clone());
        self.emit_as(n, Op::Load { ty: ptr, ptr: method_ptr, order: None });
        Ok(())
    }

    /// Whether an object, known not to be null, is of a type: its class's id within the
    /// class's range, or the trait in its class's interface table.
    fn is_instance(&mut self, ty: Ty, v: &Val) -> Res<Val> {
        let rt = self.rt;
        let (int, ptr) = (rt.int, rt.ptr);
        let obj = self.gen_val(v)?;
        if let Some(cls) = self.class_ref(ty) {
            let (start, end) = self.meta.ranges[cls as usize];
            let type_ptr = self.emit(Op::Load { ty: ptr, ptr: obj, order: None });
            if start == end {
                return Ok(self.emit(Op::Comp { comp: Comp::Ieq, ty: ptr, l: type_ptr, r: self.meta.rtti_const(rt, cls) }));
            }
            let id_ptr = self.emit(Self::elem(self.meta.rtti_layout, type_ptr, vec![Val::Int(0), Val::Int(self.meta.rtti_class_id_idx)]));
            let id = self.emit(Op::Load { ty: int, ptr: id_ptr, order: None });
            let ge = self.emit(Op::Comp { comp: Comp::Sle, ty: int, l: Val::Int(start), r: id.clone() });
            let le = self.emit(Op::Comp { comp: Comp::Sle, ty: int, l: id, r: Val::Int(end) });
            return Ok(self.emit(Op::Bin { bin: Bin::And, ty: rt.bool, l: ge, r: le }));
        }
        let trt = self.trait_ref(ty).ok_or("a type test against a type that is not linked")?;
        let rtti = self.emit(Op::Load { ty: ptr, ptr: obj, order: None });
        Ok(self.itable_lookup(trt, true, None, rtti, rt.bool, None))
    }

    fn conv(&mut self, n: Local, conv: Conv, ty: Ty, value: &Val) -> Res<()> {
        let rt = self.rt;
        if conv == Conv::Fptosi {
            // Java's conversion: zero for a NaN, the nearest bound for what is out of range.
            let v = self.gen_val(value)?;
            let from = self.val_ty(&v);
            let double = from == rt.double;
            let float = |f: f32| Val::Float(f.to_bits(), None);
            let (imin, imax, fmin, fmax) = if ty == rt.int {
                let (min, max) = (i32::MIN, i32::MAX);
                if double { (Val::Int(min), Val::Int(max), Val::Double((min as f64).to_bits(), None), Val::Double((max as f64).to_bits(), None)) } else { (Val::Int(min), Val::Int(max), float(min as f32), float(max as f32)) }
            } else if ty == rt.long {
                let (min, max) = (i64::MIN, i64::MAX);
                if double { (Val::Long(min), Val::Long(max), Val::Double((min as f64).to_bits(), None), Val::Double((max as f64).to_bits(), None)) } else { (Val::Long(min), Val::Long(max), float(min as f32), float(max as f32)) }
            } else {
                return Err("a conversion of a floating-point number to other than int and long".to_string());
            };
            let (is_nan, check_min, less, check_max, larger, in_bounds, result) = (self.fresh(), self.fresh(), self.fresh(), self.fresh(), self.fresh(), self.fresh(), self.fresh());
            let nan = self.emit(Op::Comp { comp: Comp::Fne, ty: from, l: v.clone(), r: v.clone() });
            self.branch(nan, Self::to(is_nan), Self::to(check_min));
            self.label(is_nan, &[]);
            self.jump(result, vec![Val::Zero(ty)]);
            self.label(check_min, &[]);
            let below = self.emit(Op::Comp { comp: Comp::Fle, ty: from, l: v.clone(), r: fmin });
            self.branch(below, Self::to(less), Self::to(check_max));
            self.label(less, &[]);
            self.jump(result, vec![imin]);
            self.label(check_max, &[]);
            let above = self.emit(Op::Comp { comp: Comp::Fge, ty: from, l: v, r: fmax });
            self.branch(above, Self::to(larger), Self::to(in_bounds));
            self.label(larger, &[]);
            self.jump(result, vec![imax]);
            self.label(in_bounds, &[]);
            let converted = self.emit(Op::Conv { conv, ty, value: value.clone() });
            self.jump(result, vec![converted]);
            self.label(result, &[Val::Local(n, ty)]);
            return Ok(());
        }
        let is_ptr = |cx: &Cx, t: Ty| matches!(cx.type_of(t), Type::Null | Type::Array { .. } | Type::Ref { .. } | Type::Ptr | Type::Nothing);
        let from = self.val_ty(value);
        let value = self.gen_val(value)?;
        if conv == Conv::Bitcast && is_ptr(self.cx, ty) && is_ptr(self.cx, from) {
            self.emit_as(n, Op::Copy(value));
        } else {
            self.emit_as(n, Op::Conv { conv, ty, value });
        }
        Ok(())
    }

    fn bin(&mut self, n: Local, bin: Bin, ty: Ty, l: &Val, r: &Val) {
        let rt = self.rt;
        let integer = matches!(self.cx.type_of(ty), Type::Size | Type::Char | Type::Byte | Type::Short | Type::Int | Type::Long | Type::Int128);
        let op = Op::Bin { bin, ty, l: l.clone(), r: r.clone() };
        match bin {
            Bin::Srem | Bin::Urem | Bin::Sdiv | Bin::Udiv if integer => {
                // Two labels Scala Native makes and leaves unused.
                self.fresh();
                self.fresh();
                let ok = self.fresh();
                let fail = self.slow_path(SlowPath::DivisionByZero);
                let not_zero = self.emit(Op::Comp { comp: Comp::Ine, ty, l: r.clone(), r: Val::Zero(ty) });
                self.branch(not_zero, Self::to(ok), Self::to(fail));
                self.label(ok, &[]);
                if !matches!(bin, Bin::Srem | Bin::Sdiv) {
                    self.emit_as(n, op);
                    return;
                }
                // The least value divided by -1 overflows: Java gives the least value, and zero
                // for the remainder.
                let (may, no, did, result) = (self.fresh(), self.fresh(), self.fresh(), self.fresh());
                let (minus_one, min) = if ty == rt.int { (Val::Int(-1), Val::Int(i32::MIN)) } else if ty == rt.long { (Val::Long(-1), Val::Long(i64::MIN)) } else { (Val::Size(-1), Val::Size(i64::MIN)) };
                let by_minus_one = self.emit(Op::Comp { comp: Comp::Ieq, ty, l: r.clone(), r: minus_one });
                self.branch(by_minus_one, Self::to(may), Self::to(no));
                self.label(may, &[]);
                let of_min = self.emit(Op::Comp { comp: Comp::Ieq, ty, l: l.clone(), r: min.clone() });
                self.branch(of_min, Self::to(did), Self::to(no));
                self.label(did, &[]);
                self.jump(result, vec![if bin == Bin::Srem { Val::Zero(ty) } else { min }]);
                self.label(no, &[]);
                let divided = self.emit(op);
                self.jump(result, vec![divided]);
                self.label(result, &[Val::Local(n, ty)]);
            }
            Bin::Shl | Bin::Lshr | Bin::Ashr if integer => {
                let mask = Val::Int(if ty == rt.long { 63 } else { 31 });
                let masked = self.emit(Op::Bin { bin: Bin::And, ty, l: r.clone(), r: mask });
                self.emit_as(n, Op::Bin { bin, ty, l: l.clone(), r: masked });
            }
            _ => {
                self.emit_as(n, op);
            }
        }
    }

    /// A module's instance: the static object of a constant one, else its accessor's result.
    fn module(&mut self, n: Local, name: Global) -> Val {
        let constant = self.scope_of(name).is_some_and(|id| !self.linked.scopes[id as usize].is_trait && self.meta.constant_modules[id as usize]);
        if constant {
            let instance = self.cx.member(name, self.names.instance_sig);
            self.emit_as(n, Op::Copy(Val::Global(instance, self.rt.ptr)))
        } else {
            let cls_ty = self.cx.class_ty(name);
            let ty = self.cx.ty(Type::Function(Box::new([]), cls_ty));
            let load = self.cx.member(name, self.names.load_sig);
            self.emit_as(n, Self::call(ty, Val::Global(load, self.rt.ptr), Vec::new()))
        }
    }

    fn arrayalloc(&mut self, n: Local, ty: Ty, init: &Val, zone: &Option<Val>) -> Res<()> {
        let rt = self.rt;
        let init = self.gen_val(init)?;
        let zone = match zone {
            Some(z) => Some(self.gen_val(z)?),
            None => None,
        };
        let fns = self.names.array_fns.iter().find(|&&(of, _)| of == ty).unwrap_or(&self.names.array_fns[8]).1;
        if self.val_ty(&init) == rt.int {
            let (func, sig) = fns[if zone.is_some() { 1 } else { 0 }];
            let fresh = self.fresh();
            let module = self.module(fresh, self.cx.owner(func));
            let mut args = vec![module, init];
            args.extend(zone);
            self.emit_as(n, Self::call(sig, Val::Global(func, rt.ptr), args));
        } else if let Val::ArrayValue(of, values) = &init {
            let (func, sig) = fns[2];
            let fresh = self.fresh();
            let module = self.module(fresh, self.cx.owner(func));
            let len = Val::Int(values.len() as i32);
            let data = if values.iter().any(|v| !is_canonical(v)) {
                let array_ty = self.cx.ty(Type::ArrayValue(*of, values.len() as i32));
                let alloc = self.emit(Op::Stackalloc { ty: array_ty, n: Val::Int(1) });
                for (i, value) in values.iter().enumerate() {
                    let slot = self.emit(Self::elem(array_ty, alloc.clone(), vec![Val::Int(0), Val::Int(i as i32)]));
                    let value = self.gen_val(value)?;
                    self.emit(Op::Store { ty: *of, ptr: slot, value, order: None });
                }
                alloc
            } else {
                Val::Const(Arc::new(init.clone()))
            };
            self.emit_as(n, Self::call(sig, Val::Global(func, rt.ptr), vec![module, len, data]));
        } else {
            return Err("an array made of neither a length nor its values".to_string());
        }
        Ok(())
    }

    fn arraylength(&mut self, n: Local, arr: &Val) -> Res<()> {
        let arr = self.gen_val(arr)?;
        self.guard_not_null(&arr)?;
        let len_ptr = self.emit(Self::elem(self.meta.array_header, arr, vec![Val::Int(0), Val::Int(self.meta.array_header_length_idx)]));
        self.load(n, self.rt.int, len_ptr, Some(MemoryOrder::Unordered))
    }

    /// A stack allocation, zeroed: by a store of the zero where it is one value of a simple
    /// type, else by `memset`.
    fn stackalloc(&mut self, n: Local, ty: Ty, size: &Val) {
        let rt = self.rt;
        let zero = match self.cx.type_of(ty) {
            Type::Bool => Some(Val::False),
            Type::Size => Some(Val::Size(0)),
            Type::Char => Some(Val::Char(0)),
            Type::Byte => Some(Val::Byte(0)),
            Type::Short => Some(Val::Short(0)),
            Type::Int => Some(Val::Int(0)),
            Type::Long => Some(Val::Long(0)),
            Type::Float => Some(Val::Float(0f32.to_bits(), None)),
            Type::Double => Some(Val::Double(0f64.to_bits(), None)),
            Type::Ptr | Type::Null | Type::Unit | Type::Array { .. } | Type::Ref { .. } => Some(Val::Null),
            _ => None,
        };
        let pointee = self.emit_as(n, Op::Stackalloc { ty, n: size.clone() });
        if let (Val::Size(1), Some(zero)) = (size, zero) {
            self.emit(Op::Store { ty, ptr: pointee, value: zero, order: None });
            return;
        }
        let elem_size = meta::size_of(self.cx, ty);
        let bytes = match size {
            Val::Size(count) => Val::Size(count * elem_size),
            _ => {
                let size_ty = self.val_ty(size);
                let width = match self.cx.type_of(size_ty) {
                    Type::Byte => Some(8),
                    Type::Char | Type::Short => Some(16),
                    Type::Int => Some(32),
                    Type::Long => Some(64),
                    Type::Int128 => Some(128),
                    _ => None,
                };
                let as_size = match width {
                    Some(64) | None => size.clone(),
                    Some(w) => self.emit(Op::Conv { conv: if w > 64 { Conv::Trunc } else { Conv::Zext }, ty: rt.size, value: size.clone() }),
                };
                if elem_size == 1 { as_size } else { self.emit(Op::Bin { bin: Bin::Imul, ty: rt.size, l: as_size, r: Val::Size(elem_size) }) }
            }
        };
        self.emit(Self::call(self.names.memset_ty, self.names.memset.clone(), vec![pointee, Val::Int(0), bytes]));
    }
}

/// `Fresh.apply(insts)`: the greatest local a method defines.
fn max_local(inst: &Inst) -> Local {
    let exc = |n: &Next| if let Next::Unwind { exc, .. } = n { *exc } else { 0 };
    match &inst.kind {
        InstKind::Label { id, params } => params.iter().map(|&(p, _)| p).fold(*id, Local::max),
        InstKind::Let { id, unwind, .. } => (*id).max(exc(unwind)),
        InstKind::Throw { unwind, .. } | InstKind::Unreachable { unwind } => exc(unwind),
        _ => 0,
    }
}
