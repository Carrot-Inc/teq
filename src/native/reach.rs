//! The link: the definitions a program reaches from its entry points under a closed world, after
//! `linker.Reach`, `LinktimeValueResolver` and `LinktimeIntrinsicCallsResolver` of Scala Native
//! 0.5.12. Scala Native's result stands in the order its hash tables give, which is another at
//! each run of its build; this one is sorted by name (`Global.globalOrdering`), the order in
//! which Scala Native dumps a link's definitions. Where its link iterates a hash table (a class's
//! files and the signatures it answers, the calls met on a class, a call's targets, the dynamic
//! candidates, the delayed calls), this one keeps the same table (`super::scala`) and iterates it
//! in its order, so that the definitions are reached in the same order and a class's calls stand
//! in the order Scala Native met them; tables keyed by an object's identity stand in the order of
//! their additions, as under `-XX:hashCode=2`, which the oracle runs with.

use super::classpath::ClassPath;
use super::nir::names::{self, Rt, Scope as SigScope};
use super::scala;
use super::nir::read::NirFile;
use super::nir::*;
use crate::intern::{FxBuild, FxMap};
use std::collections::{HashSet, VecDeque};

type FxSet<T> = HashSet<T, FxBuild>;
type Res<T> = Result<T, String>;

pub struct Triple {
    pub arch: String,
    pub vendor: String,
    pub os: String,
    pub env: String,
}

impl Triple {
    pub fn parse(triple: &str) -> Triple {
        let mut parts = triple.splitn(4, '-').map(str::to_string);
        let mut next = || parts.next().unwrap_or_else(|| "unknown".to_string());
        Triple { arch: next(), vendor: next(), os: next(), env: next() }
    }
}

pub struct Config {
    pub main: String,
    /// None: threads are linked in where the program can start one.
    pub multithreading: Option<bool>,
    pub release: bool,
    pub gc: &'static str,
    pub triple: Triple,
    pub link_stubs: bool,
    /// The providers a build names for a service; `java.util.ServiceLoader` gives no other.
    pub service_providers: Vec<(String, Vec<String>)>,
}

pub type ScopeId = u32;

/// A class, a module or a trait of the linked program (`linker.ScopeInfo`).
pub struct Scope {
    pub name: Global,
    pub attrs: Attrs,
    pub pos: Pos,
    pub is_trait: bool,
    pub is_module: bool,
    pub parent: Option<ScopeId>,
    pub traits: Vec<ScopeId>,
    /// The fields and methods reached, sorted by name in the result.
    pub members: Vec<Global>,
    /// The signatures called on a value of this type, in the order of Scala Native's set.
    pub calls: scala::HashSet<Str>,
    pub responds: scala::HashMap<Str, Global>,
    /// A class's methods of its traits, taken where the class itself has none.
    pub default_responds: scala::HashMap<Str, Global>,
    /// The classes that are this class or trait, itself among a class's, in the order they were
    /// linked (Scala Native's is sorted by name: `implementors_by_name`).
    pub implementors: Vec<ScopeId>,
    pub subclasses: Vec<ScopeId>,
    pub allocations: u32,
    /// The class or trait and what it inherits, each where it stands last
    /// (`ScopeInfo.linearized`).
    pub linearized: Vec<ScopeId>,
    linearized_done: bool,
}

impl Scope {
    pub fn allocated(&self) -> bool {
        self.allocations > 0
    }

    pub fn resolve(&self, sig: Str) -> Option<Global> {
        self.responds.get(&sig).or_else(|| self.default_responds.get(&sig)).copied()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Info {
    Unavailable,
    Scope(ScopeId),
    Method { owner: Global, concrete: bool },
    Field { owner: Global, is_const: bool },
}

struct Member {
    offset: u32,
    /// A method with a body.
    is_define: bool,
    is_extern: bool,
}

/// A class's file with the names its table holds, in the order of the table Scala Native's link
/// keeps them in (`loaded(owner)`, each name added in the file's order).
struct Loaded<'a> {
    file: NirFile<'a>,
    members: FxMap<Global, Member>,
    order: Vec<Global>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Scalar {
    Bool(bool),
    Char(u16),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Str(Str),
    Null,
}

impl Scalar {
    fn of(v: &Val) -> Option<Scalar> {
        Some(match v {
            Val::True => Scalar::Bool(true),
            Val::False => Scalar::Bool(false),
            Val::Char(c) => Scalar::Char(*c),
            Val::Byte(b) => Scalar::Byte(*b),
            Val::Short(s) => Scalar::Short(*s),
            Val::Int(i) => Scalar::Int(*i),
            Val::Long(l) => Scalar::Long(*l),
            Val::Float(bits) => Scalar::Float(f32::from_bits(*bits)),
            Val::Double(bits) => Scalar::Double(f64::from_bits(*bits)),
            Val::String(s) => Scalar::Str(*s),
            Val::Null => Scalar::Null,
            _ => return None,
        })
    }

    fn val(self) -> Val {
        match self {
            Scalar::Bool(true) => Val::True,
            Scalar::Bool(false) => Val::False,
            Scalar::Char(c) => Val::Char(c),
            Scalar::Byte(b) => Val::Byte(b),
            Scalar::Short(s) => Val::Short(s),
            Scalar::Int(i) => Val::Int(i),
            Scalar::Long(l) => Val::Long(l),
            Scalar::Float(f) => Val::Float(f.to_bits()),
            Scalar::Double(d) => Val::Double(d.to_bits()),
            Scalar::Str(s) => Val::String(s),
            Scalar::Null => Val::Null,
        }
    }

    /// A `java.lang.Number`'s value: a character and a boolean are none.
    fn number(self) -> Option<f64> {
        Some(match self {
            Scalar::Byte(b) => b as f64,
            Scalar::Short(s) => s as f64,
            Scalar::Int(i) => i as f64,
            Scalar::Long(l) => l as f64,
            Scalar::Float(f) => f as f64,
            Scalar::Double(d) => d,
            _ => return None,
        })
    }
}

/// What a link leaves for the code generator (`ReachabilityAnalysis.Result`).
pub struct Linked {
    /// The definitions reached, sorted by name.
    pub defns: Vec<Defn>,
    pub infos: FxMap<Global, Info>,
    pub scopes: Vec<Scope>,
    pub dynsigs: Vec<Str>,
    pub dynimpls: Vec<Global>,
    pub resolved: FxMap<Str, Val>,
    pub multithreading: bool,
}

struct Reach<'a> {
    cx: &'a mut Cx,
    rt: &'a Rt,
    cp: &'a ClassPath,
    config: &'a Config,
    multithreading: bool,
    loaded: FxMap<Global, Loaded<'a>>,
    missing: FxMap<Global, Vec<Global>>,
    unsupported: FxMap<Global, Vec<Global>>,
    enqueued: FxSet<Global>,
    todo: VecDeque<Global>,
    done: FxMap<Global, Option<Defn>>,
    stack: Vec<Global>,
    infos: FxMap<Global, Info>,
    scopes: Vec<Scope>,
    from: FxMap<Global, Global>,
    dyncandidates: scala::HashMap<Str, scala::HashSet<Global>>,
    dynsigs: FxSet<Str>,
    dynimpls: FxSet<Global>,
    /// The calls met before a class answering them was allocated (`delayedMethods`: a call's
    /// owner, signature and position, hashed whole).
    delayed: scala::HashSet<(Global, Str)>,
    /// The instruction being reached, with the class whose file holds it: a delayed call's
    /// position.
    at: Option<(Pos, Global)>,
    nir_sources: FxMap<Global, Option<(i32, i32)>>,
    proxies: FxMap<Str, Option<Str>>,
    properties: FxMap<Str, Scalar>,
    resolved: FxMap<Str, Scalar>,
    provider_loaders: FxMap<Global, Global>,
    resolve_property: Global,
    extern_forwarder: Str,
    unsupported_feature: Global,
    service_load: Vec<Global>,
}

const LINKTIME_INFO: &str = "scala.scalanative.meta.linktimeinfo";

/// The forms in which the code generator's output refers to each array class: `Lower.depends`.
#[derive(Clone, Copy)]
enum ArrayEntry {
    Alloc,
    AllocInZone,
    Snapshot,
    ApplyGeneric,
    Apply,
    UpdateGeneric,
    Update,
}

/// `ScalaNative.entries`: the program's `main` and what the code generator's own output refers to.
fn entries(cx: &mut Cx, rt: &Rt, main: &str, multithreading: bool, yield_point_traps: bool) -> Vec<Global> {
    let mut out = Vec::new();
    let method = |cx: &mut Cx, owner: Global, id: &str, types: &[Ty], scope: SigScope| {
        let sig = cx.method_sig(id, types, scope);
        cx.member(owner, sig)
    };
    let (int, ptr, unit, nothing) = (rt.int, rt.ptr, rt.unit, rt.nothing);
    let owner = cx.top(main);
    let args = cx.ty(Type::Array { of: rt.string_ty, nullable: true });
    out.push(method(cx, owner, "main", &[args, unit], SigScope::PublicStatic));
    // `Lower.depends`.
    out.extend(&rt.primitives);
    out.push(rt.class);
    out.extend(&rt.class_fields);
    out.push(rt.string);
    out.extend(&rt.string_fields);
    out.push(rt.arrays[1].1);
    let boxes_runtime = cx.top("scala.runtime.BoxesRunTime$");
    let runtime_boxes = cx.top("scala.scalanative.runtime.Boxes$");
    out.push(boxes_runtime);
    out.push(runtime_boxes);
    out.push(rt.boxed_unit_module);
    // `BoxTo.values` and `UnboxTo.values`: maps keyed by the box classes' types.
    let box_keys: Vec<i32> = rt.boxes.iter().map(|&(cls, _)| scala::ref_type_hash(cx.id(cls))).collect();
    let boxes: Vec<(Global, Ty)> = scala::immutable_order(&box_keys).into_iter().map(|i| rt.boxes[i]).collect();
    for unbox in [false, true] {
        for &(cls, held) in &boxes {
            let id = String::from_utf8_lossy(cx.id(cls)).into_owned();
            let module = if id.starts_with("java.") { boxes_runtime } else { runtime_boxes };
            let last = id.rsplit('.').next().unwrap_or(&id);
            let boxed = cx.class_ty(cls);
            if unbox {
                let short = match last {
                    "Integer" => "Int",
                    "Character" => "Char",
                    other => other,
                };
                out.push(method(cx, module, &format!("unboxTo{}", short), &[rt.object_ty, held], SigScope::Public));
            } else {
                out.push(method(cx, module, &format!("boxTo{}", last), &[held, boxed], SigScope::Public));
            }
        }
    }
    out.push(method(cx, rt.generic_array, "length", &[int], SigScope::Public));
    let safe_zone = cx.top("scala.scalanative.memory.SafeZone");
    let safe_zone = cx.class_ty(safe_zone);
    // Each form's map keyed by the element types (`typeToArray`), then the blob array's.
    let element_keys: Vec<i32> = rt
        .arrays
        .iter()
        .map(|&(of, _)| match cx.type_of(of) {
            Type::Ref { name, .. } => scala::ref_type_hash(cx.id(*name)),
            Type::Bool => scala::case_object_hash("Bool"),
            Type::Char => scala::case_object_hash("Char"),
            Type::Byte => scala::case_object_hash("Byte"),
            Type::Short => scala::case_object_hash("Short"),
            Type::Int => scala::case_object_hash("Int"),
            Type::Long => scala::case_object_hash("Long"),
            Type::Float => scala::case_object_hash("Float"),
            Type::Double => scala::case_object_hash("Double"),
            other => unreachable!("an array of {:?}", other),
        })
        .collect();
    let arrays: Vec<(Ty, Global)> = scala::immutable_order(&element_keys).into_iter().map(|i| rt.arrays[i]).chain(std::iter::once((rt.byte, rt.blob_array))).collect();
    use ArrayEntry::*;
    for form in [Alloc, AllocInZone, Snapshot, ApplyGeneric, Apply, UpdateGeneric, Update] {
        for &(of, cls) in &arrays {
            let module = cx.top(&format!("{}$", String::from_utf8_lossy(cx.id(cls))));
            let array = cx.class_ty(cls);
            out.push(match form {
                Alloc => method(cx, module, "alloc", &[int, array], SigScope::Public),
                AllocInZone => method(cx, module, "alloc", &[int, safe_zone, array], SigScope::Public),
                Snapshot => method(cx, module, "snapshot", &[int, ptr, array], SigScope::Public),
                ApplyGeneric => method(cx, cls, "apply", &[int, rt.object_ty], SigScope::Public),
                Apply => method(cx, cls, "apply", &[int, of], SigScope::Public),
                UpdateGeneric => method(cx, cls, "update", &[int, rt.object_ty, unit], SigScope::Public),
                Update => method(cx, cls, "update", &[int, of, unit], SigScope::Public),
            });
        }
    }
    out.push(method(cx, rt.runtime, "throwDivisionByZero", &[nothing], SigScope::Public));
    out.push(method(cx, rt.runtime, "throwClassCast", &[ptr, ptr, nothing], SigScope::Public));
    out.push(method(cx, rt.runtime, "throwNullPointer", &[nothing], SigScope::Public));
    out.push(method(cx, rt.runtime, "throwUndefined", &[nothing], SigScope::Public));
    out.push(method(cx, rt.runtime, "throwOutOfBounds", &[int, int, nothing], SigScope::Public));
    out.push(method(cx, rt.runtime, "throwNoSuchMethod", &[rt.string_ty, nothing], SigScope::Public));
    out.push(rt.runtime_null);
    out.push(rt.runtime_nothing);
    let external = |cx: &mut Cx, owner: Global, id: &str| {
        let sig = cx.extern_sig(id);
        cx.member(owner, sig)
    };
    if multithreading {
        let gc = cx.top("scala.scalanative.runtime.GC$");
        out.push(external(cx, gc, "scalanative_GC_yield"));
        if yield_point_traps {
            out.push(external(cx, gc, "scalanative_GC_yieldpoint_trap"));
        }
        out.push(external(cx, gc, "scalanative_GC_set_mutator_thread_state"));
        let blocker = cx.top("scala.scalanative.runtime.Blocker$");
        out.push(method(cx, blocker, "begin", &[rt.bool], SigScope::Public));
        out.push(method(cx, blocker, "end", &[rt.bool, unit], SigScope::Public));
    }
    // `Generate.depends`.
    out.push(rt.arrays[8].1);
    out.push(rt.runtime);
    let strings = cx.ty(Type::Array { of: rt.string_ty, nullable: true });
    out.push(method(cx, rt.runtime, "init", &[int, ptr, strings], SigScope::Public));
    out.push(method(cx, rt.runtime, "onShutdown", &[unit], SigScope::Public));
    let thread = cx.top("java.lang.Thread");
    let handler = cx.top("java.lang.Thread$UncaughtExceptionHandler");
    let (thread_ty, handler_ty) = (cx.class_ty(thread), cx.class_ty(handler));
    out.push(method(cx, rt.runtime, "executeUncaughtExceptionHandler", &[handler_ty, thread_ty, rt.throwable_ty, unit], SigScope::Public));
    out.push(thread);
    out.push(method(cx, thread, "currentThread", &[thread_ty], SigScope::PublicStatic));
    out.push(method(cx, thread, "getUncaughtExceptionHandler", &[handler_ty], SigScope::Public));
    out.push(handler);
    // `Interflow.depends`.
    let intrinsics = cx.top("scala.scalanative.runtime.LLVMIntrinsics$");
    out.push(external(cx, intrinsics, "llvm.stacksave"));
    out.push(external(cx, intrinsics, "llvm.stackrestore"));
    out
}

/// Links a program: with threads where the build says so or where the program can start one,
/// as Scala Native's build decides it.
pub fn link(cx: &mut Cx, rt: &Rt, cp: &ClassPath, config: &Config) -> Res<Linked> {
    let first = Reach::run(cx, rt, cp, config, config.multithreading.unwrap_or(true))?;
    if config.multithreading.is_some() {
        return Ok(first);
    }
    let thread = cx.top("java.lang.Thread");
    let runnable = cx.top("java.lang.Runnable");
    let (thread_ty, runnable_ty) = (cx.class_ty(thread), cx.class_ty(runnable));
    let start = cx.method_sig("start", &[rt.unit], SigScope::Public);
    let build_start = cx.method_sig("start", &[runnable_ty, thread_ty], SigScope::Public);
    let builder = cx.top("java.lang.Thread$Builder");
    let platform_builder = cx.top("java.lang.ThreadBuilders$PlatformThreadBuilder");
    let starts = [cx.member(thread, start), cx.member(builder, build_start), cx.member(platform_builder, build_start)];
    if starts.iter().any(|g| first.infos.contains_key(g)) {
        Ok(first)
    } else {
        Reach::run(cx, rt, cp, config, false)
    }
}

impl<'a> Reach<'a> {
    fn run(cx: &'a mut Cx, rt: &'a Rt, cp: &'a ClassPath, config: &'a Config, multithreading: bool) -> Res<Linked> {
        let linktime = cx.top("scala.scalanative.linktime");
        let resolve_sig = cx.method_sig("resolveProperty", &[rt.string_ty, rt.nothing], SigScope::Public);
        let resolve_property = cx.member(linktime, resolve_sig);
        let extern_forwarder = cx.generated_sig(b"$extern$forwarder");
        let unsupported_feature = cx.top("scala.scalanative.runtime.UnsupportedFeature");
        let service_load = service_load_methods(cx, rt);
        let mut properties = FxMap::default();
        {
            let mut set = |name: &str, value: Scalar| {
                let key = cx.str(format!("{}.{}", LINKTIME_INFO, name).as_bytes());
                properties.insert(key, value);
            };
            set("debugMetadata.enabled", Scalar::Bool(false));
            set("debugMetadata.generateFunctionSourcePositions", Scalar::Bool(false));
            set("debugMode", Scalar::Bool(!config.release));
            set("releaseMode", Scalar::Bool(config.release));
            set("isMultithreadingEnabled", Scalar::Bool(multithreading));
            set("isWeakReferenceSupported", Scalar::Bool(matches!(config.gc, "immix" | "commix" | "boehm")));
            set("is32BitPlatform", Scalar::Bool(false));
            set("isMsys", Scalar::Bool(false));
            set("isCygwin", Scalar::Bool(false));
            set("contendedPaddingWidth", Scalar::Int(64));
        }
        for (name, text) in [
            ("enabledSanitizer", ""),
            ("runtimeVersion", "0.5.12"),
            ("garbageCollector", config.gc),
            ("target.arch", config.triple.arch.as_str()),
            ("target.vendor", config.triple.vendor.as_str()),
            ("target.os", config.triple.os.as_str()),
            ("target.env", config.triple.env.as_str()),
        ] {
            let key = cx.str(format!("{}.{}", LINKTIME_INFO, name).as_bytes());
            let value = cx.str(text.as_bytes());
            properties.insert(key, Scalar::Str(value));
        }
        let yield_point_traps = multithreading && config.release && matches!(config.gc, "immix" | "commix");
        let entries = entries(cx, rt, &config.main, multithreading, yield_point_traps);
        let mut reach = Reach {
            cx,
            rt,
            cp,
            config,
            multithreading,
            loaded: FxMap::default(),
            missing: FxMap::default(),
            unsupported: FxMap::default(),
            enqueued: FxSet::default(),
            todo: VecDeque::new(),
            done: FxMap::default(),
            stack: Vec::new(),
            infos: FxMap::default(),
            scopes: Vec::new(),
            from: FxMap::default(),
            dyncandidates: scala::HashMap::default(),
            dynsigs: FxSet::default(),
            dynimpls: FxSet::default(),
            delayed: scala::HashSet::default(),
            at: None,
            nir_sources: FxMap::default(),
            proxies: FxMap::default(),
            properties,
            resolved: FxMap::default(),
            provider_loaders: FxMap::default(),
            resolve_property,
            extern_forwarder,
            unsupported_feature,
            service_load,
        };
        // `@align` reads it, whether or not a definition does.
        let padding = reach.cx.str(format!("{}.contendedPaddingWidth", LINKTIME_INFO).as_bytes());
        reach.resolve_property(padding)?;
        for &entry in &entries {
            reach.reach_entry(entry)?;
        }
        for class in cp.classes_with_entry_points(reach.cx)? {
            reach.reach_clinit(class)?;
        }
        reach.process()?;
        reach.process_delayed()?;
        reach.result()
    }

    fn result(mut self) -> Res<Linked> {
        if !self.missing.is_empty() || !self.unsupported.is_empty() {
            return Err(self.failure());
        }
        // A method the link did not reach answers no signature.
        for scope in &mut self.scopes {
            scope.responds.retain(|_, name| self.done.contains_key(name));
            scope.default_responds.retain(|_, name| self.done.contains_key(name));
        }
        let cx = &*self.cx;
        let mut defns: Vec<Defn> = self.done.into_values().flatten().collect();
        defns.sort_by(|a, b| cx.cmp_globals(a.name, b.name));
        for scope in &mut self.scopes {
            if scope.name != self.rt.class && scope.name != self.rt.string {
                scope.members.sort_by(|&a, &b| cx.cmp_globals(a, b));
            }
        }
        let names: Vec<Global> = self.scopes.iter().map(|s| s.name).collect();
        for scope in &mut self.scopes {
            scope.subclasses.sort_by(|&a, &b| cx.cmp_globals(names[a as usize], names[b as usize]));
            scope.implementors.sort_by(|&a, &b| cx.cmp_globals(names[a as usize], names[b as usize]));
        }
        let mut dynsigs: Vec<Str> = self.dynsigs.into_iter().collect();
        dynsigs.sort_by(|&a, &b| names::cmp_strings(cx.bytes(a), cx.bytes(b)));
        let mut dynimpls: Vec<Global> = self.dynimpls.into_iter().collect();
        dynimpls.sort_by(|&a, &b| cx.cmp_globals(a, b));
        Ok(Linked {
            defns,
            infos: self.infos,
            scopes: self.scopes,
            dynsigs,
            dynimpls,
            resolved: self.resolved.into_iter().map(|(k, v)| (k, v.val())).collect(),
            multithreading: self.multithreading,
        })
    }

    fn show(&self, g: Global) -> String {
        let cx = &*self.cx;
        let owner = String::from_utf8_lossy(cx.id(cx.owner(g))).into_owned();
        match cx.sig(g) {
            None => owner,
            Some(sig) => {
                let sig = cx.bytes(sig);
                match names::sig_id(sig) {
                    Some(id) => format!("{}.{} ({})", owner, String::from_utf8_lossy(id), String::from_utf8_lossy(sig)),
                    None if names::is_ctor(sig) => format!("{}.<init> ({})", owner, String::from_utf8_lossy(sig)),
                    None => format!("{}.{}", owner, String::from_utf8_lossy(sig)),
                }
            }
        }
    }

    fn failure(&self) -> String {
        let mut lines = Vec::new();
        let mut listed = |title: &str, set: &FxMap<Global, Vec<Global>>| {
            let mut names: Vec<(&Global, &Vec<Global>)> = set.iter().collect();
            names.sort_by(|a, b| self.cx.cmp_globals(*a.0, *b.0));
            for (name, trace) in names {
                lines.push(format!("{}: {}", title, self.show(*name)));
                for from in trace {
                    lines.push(format!("    referenced by {}", self.show(*from)));
                }
            }
        };
        listed("not found on the native class path", &self.missing);
        listed("not supported by this build", &self.unsupported);
        lines.join("\n")
    }

    fn fail<T>(&self, message: String) -> Res<T> {
        Err(message)
    }

    // ---- loading ----

    fn load(&mut self, owner: Global) -> Res<bool> {
        if self.loaded.contains_key(&owner) {
            return Ok(true);
        }
        let Some(bytes) = self.cp.read(owner) else { return Ok(false) };
        let origin = |cp: &ClassPath, cx: &Cx, e: String| format!("{}: {}: {}", cp.origin(owner).unwrap_or(""), String::from_utf8_lossy(cx.id(owner)), e);
        let mut file = NirFile::open(bytes?).map_err(|e| origin(self.cp, self.cx, e))?;
        let offsets = file.offsets(self.cx).map_err(|e| origin(self.cp, self.cx, e))?;
        let mut members = FxMap::default();
        let mut table = scala::HashMap::default();
        for (name, offset) in offsets {
            let (is_define, attrs) = file.defn_head(self.cx, offset).map_err(|e| origin(self.cp, self.cx, e))?;
            members.insert(name, Member { offset, is_define, is_extern: attrs.is_extern });
            table.update(name, self.global_hash(name), ());
        }
        self.loaded.insert(owner, Loaded { file, members, order: table.keys() });
        Ok(true)
    }

    /// Whether a class's file holds the definition; the file is the loaded one.
    fn has(&self, name: Global) -> bool {
        self.loaded.get(&self.cx.owner(name)).is_some_and(|l| l.members.contains_key(&name))
    }

    fn lookup(&mut self, name: Global, ignore_unavailable: bool) -> Res<Option<Defn>> {
        let owner = self.cx.owner(name);
        if !self.loaded.contains_key(&owner) && !self.missing.contains_key(&owner) && !self.load(owner)? && !ignore_unavailable {
            self.add_missing(name);
        }
        let found = match self.loaded.get_mut(&owner) {
            Some(loaded) => match loaded.members.get(&name) {
                Some(member) => Some(loaded.file.defn(self.cx, member.offset).map_err(|e| format!("{}: {}", String::from_utf8_lossy(self.cx.id(owner)), e))?),
                None => None,
            },
            None => None,
        };
        if found.is_none() && !ignore_unavailable {
            self.add_missing(name);
        }
        Ok(found)
    }

    fn add_missing(&mut self, name: Global) {
        let mut trace = Vec::new();
        let mut at = name;
        while let Some(&from) = self.from.get(&at) {
            if from == Global::NONE || trace.contains(&from) {
                break;
            }
            trace.push(from);
            at = from;
        }
        let set = if self.cx.owner(name) == self.unsupported_feature && !self.cx.is_top(name) { &mut self.unsupported } else { &mut self.missing };
        set.entry(name).or_insert(trace);
    }

    fn track(&mut self, name: Global) {
        let from = self.stack.last().copied().unwrap_or(Global::NONE);
        self.from.entry(name).or_insert(from);
    }

    // ---- the work list ----

    fn process(&mut self) -> Res<()> {
        while let Some(name) = self.todo.pop_front() {
            if !self.done.contains_key(&name) {
                self.reach_defn_named(name)?;
            }
        }
        Ok(())
    }

    /// The calls that had no target when they were met: by now a class that answers them is
    /// allocated, or the method is missing.
    fn process_delayed(&mut self) -> Res<()> {
        while !self.delayed.is_empty() {
            for (top, sig) in self.delayed.iter() {
                let Some(id) = self.scope_info(top)? else {
                    let member = self.cx.member(top, sig);
                    self.add_missing(member);
                    continue;
                };
                let scope = &self.scopes[id as usize];
                let was_allocated = if scope.is_trait { scope.implementors.iter().any(|&c| self.scopes[c as usize].allocated()) } else { scope.allocated() };
                let targets = self.targets(id, sig);
                if targets.is_empty() && was_allocated {
                    let member = self.cx.member(top, sig);
                    self.add_missing(member);
                } else {
                    self.todo.extend(targets);
                }
            }
            // The table keeps its length for the calls delayed next.
            self.delayed.clear();
            self.process()?;
        }
        Ok(())
    }

    fn reach_defn_named(&mut self, name: Global) -> Res<()> {
        self.stack.push(name);
        match self.lookup(name, false)? {
            Some(defn) if !defn.attrs.is_stub || self.config.link_stubs => self.reach_defn(defn)?,
            _ => self.reach_unavailable(name),
        }
        self.stack.pop();
        Ok(())
    }

    fn reach_defn(&mut self, defn: Defn) -> Res<()> {
        let defn = self.preprocess(defn)?;
        let name = defn.name;
        match &defn.kind {
            DefnKind::Var { ty, rhs } | DefnKind::Const { ty, rhs } => {
                let is_const = matches!(defn.kind, DefnKind::Const { .. });
                let owner = self.scope_or_unavailable(self.cx.owner(name))?;
                self.new_member(name, Info::Field { owner, is_const });
                self.reach_type(*ty);
                self.reach_val(rhs)?;
            }
            DefnKind::Declare { ty } => {
                let owner = self.scope_or_unavailable(self.cx.owner(name))?;
                self.new_member(name, Info::Method { owner, concrete: false });
                self.reach_type(*ty);
            }
            DefnKind::Define { ty, insts, .. } => {
                if let Some(sig) = self.cx.sig(name) {
                    if let Some(&(_, array)) = self.rt.array_alloc.iter().find(|&&(s, _)| s == sig) {
                        if let Some(cls) = self.class_info(array)? {
                            self.reach_allocation(cls);
                        }
                    }
                }
                let owner = self.scope_or_unavailable(self.cx.owner(name))?;
                self.new_member(name, Info::Method { owner, concrete: !insts.is_empty() });
                self.reach_type(*ty);
                let class = self.cx.owner(name);
                for inst in insts {
                    self.at = Some((inst.pos, class));
                    self.reach_inst(inst)?;
                }
            }
            DefnKind::Trait { traits } => {
                let traits = self.trait_infos(traits)?;
                self.new_scope(name, &defn.attrs, defn.pos, true, false, None, traits);
                self.reach_exported(name);
            }
            DefnKind::Class { parent, traits } | DefnKind::Module { parent, traits } => {
                let is_module = matches!(defn.kind, DefnKind::Module { .. });
                let parent = match parent {
                    Some(p) => Some(self.class_info_or_object(*p)?),
                    None => None,
                };
                let traits = self.trait_infos(traits)?;
                self.new_scope(name, &defn.attrs, defn.pos, false, is_module, parent, traits);
                self.reach_exported(name);
            }
        }
        self.done.insert(name, Some(defn));
        Ok(())
    }

    fn reach_unavailable(&mut self, name: Global) {
        self.infos.insert(name, Info::Unavailable);
        self.add_missing(name);
        self.done.insert(name, None);
    }

    fn reach_entry(&mut self, name: Global) -> Res<()> {
        if !self.cx.is_top(name) {
            self.reach_entry(self.cx.owner(name))?;
        }
        self.from.entry(name).or_insert(Global::NONE);
        self.reach_global_now(name)?;
        if let Some(&Info::Scope(id)) = self.infos.get(&name) {
            let scope = &self.scopes[id as usize];
            if !scope.is_trait && !scope.attrs.is_abstract {
                let is_module = scope.is_module;
                self.reach_allocation(id);
                if is_module {
                    let ctor = self.cx.ctor_sig(&[]);
                    let init = self.cx.member(name, ctor);
                    if self.has(init) {
                        self.reach_global(init);
                    }
                }
            }
        }
        Ok(())
    }

    fn reach_clinit(&mut self, class: Global) -> Res<()> {
        self.reach_global_now(class)?;
        if self.infos.contains_key(&class) {
            let sig = self.cx.clinit_sig();
            let clinit = self.cx.member(class, sig);
            if self.has(clinit) {
                self.reach_global(clinit);
            }
        }
        Ok(())
    }

    /// A function with C's linkage that a class defines is reached with the class: other code
    /// names it by its symbol alone.
    fn reach_exported(&mut self, name: Global) {
        let Some(loaded) = self.loaded.get(&name) else { return };
        let exported: Vec<Global> = loaded
            .order
            .iter()
            .copied()
            .filter(|member| {
                let m = &loaded.members[member];
                let sig = self.cx.sig(*member);
                m.is_define && (m.is_extern || sig.is_some_and(|s| names::is_extern(self.cx.bytes(s)))) && sig != Some(self.extern_forwarder)
            })
            .collect();
        for member in exported {
            self.reach_global(member);
        }
    }

    fn sig_hash(&self, sig: Str) -> i32 {
        scala::string_hash(self.cx.bytes(sig))
    }

    /// `hashCode` of a `nir.Global`: a `Top` or a `Member`.
    fn global_hash(&self, g: Global) -> i32 {
        global_hash(self.cx, g)
    }

    /// The hash of a delayed call (`DelayedMethod(owner, sig, pos)`), its position the
    /// instruction's, in the file of the class that holds it.
    fn delayed_hash(&mut self, owner: Global, sig: Str) -> i32 {
        let position = match self.at {
            Some((pos, class)) => {
                let cp = self.cp;
                let nir_source = *self.nir_sources.entry(class).or_insert_with(|| cp.nir_source(class));
                scala::position_hash(self.cx.bytes(pos.file), pos.line, pos.column, nir_source)
            }
            None => scala::position_hash(&[], 0, 0, None),
        };
        scala::delayed_hash(scala::top_hash(self.cx.id(owner)), self.sig_hash(sig), position)
    }

    fn reach_global(&mut self, name: Global) {
        if name != Global::NONE && self.enqueued.insert(name) {
            self.track(name);
            self.todo.push_front(name);
        }
    }

    fn reach_global_now(&mut self, name: Global) -> Res<()> {
        if self.done.contains_key(&name) {
            Ok(())
        } else if !self.stack.contains(&name) {
            self.enqueued.insert(name);
            self.track(name);
            self.reach_defn_named(name)
        } else {
            let mut lines = vec![format!("cyclic reference to {}:", self.show(name))];
            lines.extend(self.stack.iter().rev().map(|&el| format!("* {}", self.show(el))));
            self.fail(lines.join("\n"))
        }
    }

    // ---- infos ----

    fn new_member(&mut self, name: Global, info: Info) {
        self.infos.insert(name, info);
        let owner = match info {
            Info::Method { owner, .. } | Info::Field { owner, .. } => owner,
            _ => return,
        };
        if let Some(&Info::Scope(id)) = self.infos.get(&owner) {
            self.scopes[id as usize].members.push(name);
        }
    }

    fn linearized(&mut self, id: ScopeId) -> Vec<ScopeId> {
        if self.scopes[id as usize].linearized_done {
            return self.scopes[id as usize].linearized.clone();
        }
        let mut raw = vec![id];
        let traits = self.scopes[id as usize].traits.clone();
        for &t in traits.iter().rev() {
            raw.extend(self.linearized(t));
        }
        if let Some(parent) = self.scopes[id as usize].parent {
            raw.extend(self.linearized(parent));
        }
        // Each one stands where it stands last.
        let mut last: FxMap<ScopeId, usize> = FxMap::default();
        for (i, &s) in raw.iter().enumerate() {
            last.insert(s, i);
        }
        let out: Vec<ScopeId> = raw.iter().enumerate().filter(|&(i, s)| last[s] == i).map(|(_, &s)| s).collect();
        self.scopes[id as usize].linearized = out.clone();
        self.scopes[id as usize].linearized_done = true;
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn new_scope(&mut self, name: Global, attrs: &Attrs, pos: Pos, is_trait: bool, is_module: bool, parent: Option<ScopeId>, traits: Vec<ScopeId>) {
        let id = self.scopes.len() as ScopeId;
        self.scopes.push(Scope {
            name,
            attrs: attrs.clone(),
            pos,
            is_trait,
            is_module,
            parent,
            traits,
            members: Vec::new(),
            calls: scala::HashSet::default(),
            responds: scala::HashMap::default(),
            default_responds: scala::HashMap::default(),
            implementors: if is_trait { Vec::new() } else { vec![id] },
            subclasses: Vec::new(),
            allocations: 0,
            linearized: Vec::new(),
            linearized_done: false,
        });
        self.infos.insert(name, Info::Scope(id));
        let mut ancestors: Vec<ScopeId> = Vec::new();
        let mut stack: Vec<ScopeId> = self.scopes[id as usize].traits.clone();
        let mut classes: Vec<ScopeId> = Vec::new();
        let mut at = self.scopes[id as usize].parent;
        while let Some(p) = at {
            classes.push(p);
            stack.extend(&self.scopes[p as usize].traits);
            at = self.scopes[p as usize].parent;
        }
        while let Some(t) = stack.pop() {
            if !ancestors.contains(&t) {
                ancestors.push(t);
                stack.extend(&self.scopes[t as usize].traits);
            }
        }
        if !is_trait {
            for &c in &classes {
                self.scopes[c as usize].implementors.push(id);
                self.scopes[c as usize].subclasses.push(id);
            }
            for &t in &ancestors {
                self.scopes[t as usize].implementors.push(id);
            }
        }
        let linearized = self.linearized(id);
        let own: Vec<(Str, Global)> = self.loaded.get(&name).map_or(Vec::new(), |loaded| {
            loaded.order.iter().filter(|m| loaded.members[m].is_define).filter_map(|&m| self.cx.sig(m).map(|sig| (sig, m))).collect()
        });
        // `newInfo`: the tables built as Scala Native builds them, `++=` and `update` alike.
        if is_trait {
            let mut responds = scala::HashMap::default();
            for &l in &linearized {
                responds.add_all(&self.scopes[l as usize].responds);
            }
            for (sig, member) in own {
                responds.update(sig, self.sig_hash(sig), member);
            }
            self.scopes[id as usize].responds = responds;
        } else {
            let mut responds = scala::HashMap::default();
            if let Some(p) = parent {
                responds.add_all(&self.scopes[p as usize].responds);
            }
            for (sig, member) in own {
                let s = self.cx.bytes(sig);
                if names::is_method(s) || names::is_ctor(s) || names::is_clinit(s) || names::is_generated(s) {
                    responds.update(sig, self.sig_hash(sig), member);
                }
            }
            let mut defaults = scala::HashMap::default();
            for &l in &linearized {
                if self.scopes[l as usize].is_trait {
                    defaults.add_all(&self.scopes[l as usize].responds);
                }
            }
            self.scopes[id as usize].responds = responds;
            self.scopes[id as usize].default_responds = defaults;
        }
    }

    fn proxy(&mut self, sig: Str) -> Option<Str> {
        if let Some(&p) = self.proxies.get(&sig) {
            return p;
        }
        let p = self.cx.proxy_sig(sig);
        self.proxies.insert(sig, p);
        p
    }

    fn reach_allocation(&mut self, id: ScopeId) {
        self.scopes[id as usize].allocations += 1;
        if self.scopes[id as usize].allocations > 1 {
            return;
        }
        for sig in gathered_calls(&self.scopes, id).iter() {
            if let Some(target) = self.scopes[id as usize].resolve(sig) {
                self.reach_global(target);
            }
        }
        for (sig, target) in self.scopes[id as usize].responds.iter() {
            if names::is_method(self.cx.bytes(sig)) {
                let Some(dynsig) = self.proxy(sig) else { continue };
                if self.dynsigs.contains(&dynsig) {
                    self.dynimpls.insert(target);
                    self.reach_global(target);
                } else {
                    let (dynsig_hash, target_hash) = (self.sig_hash(dynsig), self.global_hash(target));
                    self.dyncandidates.get_or_else_update(dynsig, dynsig_hash, scala::HashSet::default).add(target, target_hash);
                }
            } else if sig == self.extern_forwarder {
                self.reach_global(target);
            }
        }
    }

    fn scope_info(&mut self, name: Global) -> Res<Option<ScopeId>> {
        self.reach_global_now(name)?;
        Ok(match self.infos.get(&name) {
            Some(&Info::Scope(id)) => Some(id),
            _ => None,
        })
    }

    /// The class of a member: its name, an unavailable one's too.
    fn scope_or_unavailable(&mut self, name: Global) -> Res<Global> {
        self.reach_global_now(name)?;
        Ok(name)
    }

    fn class_info(&mut self, name: Global) -> Res<Option<ScopeId>> {
        Ok(self.scope_info(name)?.filter(|&id| !self.scopes[id as usize].is_trait))
    }

    fn class_info_or_object(&mut self, name: Global) -> Res<ScopeId> {
        if let Some(id) = self.class_info(name)? {
            return Ok(id);
        }
        match self.class_info(self.rt.object)? {
            Some(id) => Ok(id),
            None => self.fail(format!("no class {} on the native class path", self.show(name))),
        }
    }

    fn trait_infos(&mut self, names: &[Global]) -> Res<Vec<ScopeId>> {
        let mut out = Vec::with_capacity(names.len());
        for &name in names {
            if let Some(id) = self.scope_info(name)?.filter(|&id| self.scopes[id as usize].is_trait) {
                out.push(id);
            }
        }
        Ok(out)
    }

    fn targets(&self, id: ScopeId, sig: Str) -> Vec<Global> {
        targets(self.cx, &self.scopes, id, sig)
    }

    // ---- what a definition refers to ----

    fn reach_type(&mut self, ty: Ty) {
        match self.cx.type_of(ty) {
            Type::ArrayValue(of, _) | Type::Var(of) => self.reach_type(*of),
            Type::Array { of, .. } => self.reach_type(*of),
            Type::StructValue(tys) => {
                let tys = tys.clone();
                tys.iter().for_each(|&t| self.reach_type(t));
            }
            Type::Function(args, ret) => {
                let (args, ret) = (args.clone(), *ret);
                args.iter().for_each(|&t| self.reach_type(t));
                self.reach_type(ret);
            }
            Type::Ref { name, .. } => self.reach_global(*name),
            _ => {}
        }
    }

    fn reach_val(&mut self, value: &Val) -> Res<()> {
        match value {
            Val::Zero(ty) | Val::Local(_, ty) => self.reach_type(*ty),
            Val::StructValue(values) => {
                for v in values.iter() {
                    self.reach_val(v)?;
                }
            }
            Val::ArrayValue(ty, values) => {
                self.reach_type(*ty);
                for v in values.iter() {
                    self.reach_val(v)?;
                }
            }
            Val::Global(name, ty) => {
                self.reach_global(*name);
                self.reach_type(*ty);
            }
            Val::Const(v) => self.reach_val(v)?,
            Val::ClassOf(cls) => self.reach_global(*cls),
            _ => {}
        }
        Ok(())
    }

    fn reach_next(&mut self, next: &Next) -> Res<()> {
        if let Next::Label(_, args) = next {
            for v in args.iter() {
                self.reach_val(v)?;
            }
        }
        Ok(())
    }

    fn reach_inst(&mut self, inst: &Inst) -> Res<()> {
        match &inst.kind {
            InstKind::Label { params, .. } => params.iter().for_each(|&(_, ty)| self.reach_type(ty)),
            InstKind::Let { op, unwind, .. } => {
                self.reach_op(op)?;
                self.reach_next(unwind)?;
            }
            InstKind::Ret(v) => self.reach_val(v)?,
            InstKind::Jump(next) => self.reach_next(next)?,
            InstKind::If { value, then, otherwise } => {
                self.reach_val(value)?;
                self.reach_next(then)?;
                self.reach_next(otherwise)?;
            }
            InstKind::Switch { value, default, cases } => {
                self.reach_val(value)?;
                self.reach_next(default)?;
                for case in cases.iter() {
                    self.reach_next(case)?;
                }
            }
            InstKind::Throw { value, unwind } => {
                self.reach_val(value)?;
                self.reach_next(unwind)?;
            }
            InstKind::Unreachable { unwind } => self.reach_next(unwind)?,
            InstKind::LinktimeIf { .. } => return self.fail("a link-time condition outside a link-time method".to_string()),
        }
        Ok(())
    }

    fn reach_class_allocation(&mut self, name: Global) -> Res<()> {
        if let Some(cls) = self.class_info(name)? {
            self.reach_allocation(cls);
        }
        Ok(())
    }

    fn reach_op(&mut self, op: &Op) -> Res<()> {
        match op {
            Op::Call { ty, ptr, args } => {
                self.reach_type(*ty);
                self.reach_val(ptr)?;
                for v in args.iter() {
                    self.reach_val(v)?;
                }
            }
            Op::Load { ty, ptr, .. } => {
                self.reach_type(*ty);
                self.reach_val(ptr)?;
            }
            Op::Store { ty, ptr, value, .. } => {
                self.reach_type(*ty);
                self.reach_val(ptr)?;
                self.reach_val(value)?;
            }
            Op::Elem { ty, ptr, indexes } => {
                self.reach_type(*ty);
                self.reach_val(ptr)?;
                for v in indexes.iter() {
                    self.reach_val(v)?;
                }
            }
            Op::Extract { aggr, .. } => self.reach_val(aggr)?,
            Op::Insert { aggr, value, .. } => {
                self.reach_val(aggr)?;
                self.reach_val(value)?;
            }
            Op::Stackalloc { ty, n } => {
                self.reach_type(*ty);
                self.reach_val(n)?;
                if let Some(cls) = self.rt.class_name(self.cx, *ty) {
                    self.reach_class_allocation(cls)?;
                }
            }
            Op::Bin { ty, l, r, .. } | Op::Comp { ty, l, r, .. } => {
                self.reach_type(*ty);
                self.reach_val(l)?;
                self.reach_val(r)?;
            }
            Op::Conv { ty, value, .. } => {
                self.reach_type(*ty);
                self.reach_val(value)?;
            }
            Op::Fence(_) => {}
            Op::Classalloc { name, zone } => {
                self.reach_class_allocation(*name)?;
                if let Some(zone) = zone {
                    self.reach_val(zone)?;
                }
            }
            Op::Fieldload { ty, obj, name } => {
                self.reach_type(*ty);
                self.reach_val(obj)?;
                self.reach_global(*name);
            }
            Op::Fieldstore { ty, obj, name, value } => {
                self.reach_type(*ty);
                self.reach_val(obj)?;
                self.reach_global(*name);
                self.reach_val(value)?;
            }
            Op::Field { obj, name } => {
                self.reach_val(obj)?;
                self.reach_global(*name);
            }
            Op::Method { obj, sig } => {
                self.reach_val(obj)?;
                let ty = self.val_type(obj);
                self.reach_method_targets(ty, *sig)?;
            }
            Op::Dynmethod { obj, sig } => {
                self.reach_val(obj)?;
                if self.dynsigs.insert(*sig) {
                    if let Some(candidates) = self.dyncandidates.remove(sig) {
                        for target in candidates.iter() {
                            self.dynimpls.insert(target);
                            self.reach_global(target);
                        }
                    }
                }
            }
            Op::Module(name) => {
                self.reach_class_allocation(*name)?;
                if self.loaded.contains_key(name) {
                    let ctor = self.cx.ctor_sig(&[]);
                    let init = self.cx.member(*name, ctor);
                    if self.has(init) {
                        self.reach_global(init);
                    }
                } else {
                    self.add_missing(*name);
                }
            }
            Op::As { ty, obj } | Op::Is { ty, obj } => {
                self.reach_type(*ty);
                self.reach_val(obj)?;
            }
            Op::Copy(v) | Op::Varload(v) | Op::Arraylength(v) => self.reach_val(v)?,
            Op::SizeOf(ty) | Op::AlignmentOf(ty) | Op::Var(ty) => self.reach_type(*ty),
            Op::Box { obj, .. } | Op::Unbox { obj, .. } => self.reach_val(obj)?,
            Op::Varstore { slot, value } => {
                self.reach_val(slot)?;
                self.reach_val(value)?;
            }
            Op::Arrayalloc { ty, init, zone } => {
                self.reach_class_allocation(self.rt.array_class(*ty))?;
                self.reach_type(*ty);
                self.reach_val(init)?;
                if let Some(zone) = zone {
                    self.reach_val(zone)?;
                }
            }
            Op::Arrayload { ty, arr, idx } => {
                self.reach_type(*ty);
                self.reach_val(arr)?;
                self.reach_val(idx)?;
            }
            Op::Arraystore { ty, arr, idx, value } => {
                self.reach_type(*ty);
                self.reach_val(arr)?;
                self.reach_val(idx)?;
                self.reach_val(value)?;
            }
        }
        Ok(())
    }

    /// The type of a value as far as a call's receiver needs it: a local's or a global's own.
    fn val_type(&mut self, value: &Val) -> Ty {
        match value {
            Val::Local(_, ty) | Val::Global(_, ty) | Val::Zero(ty) => *ty,
            Val::String(_) => self.cx.ty(Type::Ref { name: self.rt.string, exact: true, nullable: false }),
            Val::ClassOf(_) => self.cx.ty(Type::Ref { name: self.rt.class, exact: true, nullable: false }),
            Val::Unit => self.rt.unit,
            Val::Null => self.rt.null,
            _ => self.rt.ptr,
        }
    }

    fn reach_method_targets(&mut self, ty: Ty, sig: Str) -> Res<()> {
        let name = match self.cx.type_of(ty) {
            Type::Array { of, .. } => self.rt.array_class(*of),
            Type::Ref { name, .. } => *name,
            _ => return Ok(()),
        };
        // The call's position, before reaching its class may reach other code.
        let at = self.at;
        let Some(id) = self.scope_info(name)? else { return Ok(()) };
        if !self.scopes[id as usize].calls.contains(&sig) {
            let sig_hash = self.sig_hash(sig);
            self.scopes[id as usize].calls.add(sig, sig_hash);
            let targets = self.targets(id, sig);
            if targets.is_empty() {
                // No class that answers it is allocated yet: one may be before the link ends.
                let member = self.cx.member(name, sig);
                self.track(member);
                self.at = at;
                let hash = self.delayed_hash(name, sig);
                self.delayed.add((name, sig), hash);
            } else {
                targets.into_iter().for_each(|t| self.reach_global(t));
            }
        }
        Ok(())
    }

    // ---- link-time values ----

    fn preprocess(&mut self, mut defn: Defn) -> Res<Defn> {
        if !matches!(defn.kind, DefnKind::Define { .. }) {
            return Ok(defn);
        }
        if defn.attrs.is_linktime_resolved {
            defn = if self.can_be_evaluated(&defn)? { self.evaluated(defn)? } else { self.partially_evaluated(defn)? };
        }
        if defn.attrs.uses_intrinsic {
            defn = self.resolve_intrinsics(defn)?;
        }
        Ok(defn)
    }

    fn referenced_property(&self, op: &Op) -> Option<Str> {
        match op {
            Op::Call { ptr: Val::Global(name, _), args, .. } if *name == self.resolve_property => match &args[..] {
                [Val::String(property)] => Some(*property),
                _ => None,
            },
            _ => None,
        }
    }

    fn can_be_evaluated(&mut self, defn: &Defn) -> Res<bool> {
        let DefnKind::Define { ty, insts, .. } = &defn.kind else { return Ok(false) };
        for inst in insts {
            let runtime_only = match &inst.kind {
                InstKind::Label { .. } | InstKind::LinktimeIf { .. } | InstKind::Ret(_) | InstKind::Jump(Next::Label(..)) => false,
                InstKind::Let { op, unwind: Next::None, .. } => match op {
                    Op::Call { ptr: Val::Global(name, _), .. } => {
                        self.track(*name);
                        *name != self.resolve_property && !self.lookup(*name, false)?.is_some_and(|d| d.attrs.is_linktime_resolved)
                    }
                    Op::Comp { .. } => false,
                    _ => true,
                },
                _ => true,
            };
            if runtime_only {
                return Ok(false);
            }
        }
        let Type::Function(_, ret) = self.cx.type_of(*ty) else { return Ok(false) };
        Ok(match self.cx.type_of(*ret) {
            Type::Ptr | Type::Bool | Type::Size | Type::Char | Type::Byte | Type::Short | Type::Int | Type::Long | Type::Int128 | Type::Float | Type::Double | Type::ArrayValue(..) | Type::StructValue(_) | Type::Null => true,
            Type::Ref { name, .. } => *name == self.rt.string,
            _ => false,
        })
    }

    fn evaluated(&mut self, mut defn: Defn) -> Res<Defn> {
        let pos = defn.pos;
        let name = defn.name;
        let DefnKind::Define { insts, .. } = &mut defn.kind else { return Ok(defn) };
        let property = match &insts[..] {
            [_, Inst { kind: InstKind::Ret(_), .. }] => return Ok(defn),
            [_, Inst { kind: InstKind::Let { op, .. }, .. }, Inst { kind: InstKind::Ret(_), .. }] => self.referenced_property(op),
            _ => None,
        };
        let property = match property {
            Some(p) => p,
            None => {
                let mut mangled = Vec::new();
                self.cx.mangle(name, &mut mangled);
                self.cx.str(&mangled)
            }
        };
        let value = self.resolve_property(property)?;
        let DefnKind::Define { insts, .. } = &mut defn.kind else { return Ok(defn) };
        insts.truncate(1);
        insts.push(Inst { pos, kind: InstKind::Ret(value.val()) });
        Ok(defn)
    }

    fn partially_evaluated(&mut self, mut defn: Defn) -> Res<Defn> {
        let DefnKind::Define { insts, .. } = &mut defn.kind else { return Ok(defn) };
        let mut resolved = Vec::with_capacity(insts.len());
        for inst in std::mem::take(insts) {
            let pos = inst.pos;
            resolved.push(match inst.kind {
                InstKind::LinktimeIf { cond, then, otherwise } => Inst { pos, kind: InstKind::Jump(if self.condition(&cond)? { then } else { otherwise }) },
                InstKind::Let { id, op, unwind, scope } => match self.referenced_property(&op) {
                    Some(property) => {
                        let value = self.resolve_property(property)?.val();
                        Inst { pos, kind: InstKind::Let { id, op: Op::Copy(value), unwind, scope } }
                    }
                    None => Inst { pos, kind: InstKind::Let { id, op, unwind, scope } },
                },
                kind => Inst { pos, kind },
            });
        }
        let DefnKind::Define { insts, .. } = &mut defn.kind else { return Ok(defn) };
        *insts = remove_dead_blocks(resolved)?;
        Ok(defn)
    }

    fn resolve_property(&mut self, name: Str) -> Res<Scalar> {
        if let Some(&v) = self.resolved.get(&name) {
            return Ok(v);
        }
        let value = match self.properties.get(&name) {
            Some(&v) => v,
            None => {
                let calculated = match self.cx.unmangle_global(name) {
                    Some(global) => match self.lookup(global, false)? {
                        Some(defn) if defn.attrs.is_linktime_resolved && matches!(defn.kind, DefnKind::Define { .. }) => Some(self.interpret(&defn)?),
                        _ => None,
                    },
                    None => None,
                };
                match calculated {
                    Some(v) => v,
                    None => return self.fail(format!("the link-time property `{}` is not defined", String::from_utf8_lossy(self.cx.bytes(name)))),
                }
            }
        };
        self.resolved.insert(name, value);
        Ok(value)
    }

    fn condition(&mut self, cond: &LinktimeCondition) -> Res<bool> {
        match cond {
            LinktimeCondition::Complex { op: Bin::And, left, right, .. } => Ok(self.condition(left)? && self.condition(right)?),
            LinktimeCondition::Complex { op: Bin::Or, left, right, .. } => Ok(self.condition(left)? || self.condition(right)?),
            LinktimeCondition::Complex { .. } => self.fail("a link-time condition that is no conjunction and no disjunction".to_string()),
            LinktimeCondition::Simple { property, comparison, value, .. } => {
                let resolved = self.resolve_property(*property)?;
                let Some(wanted) = Scalar::of(value) else { return self.fail("a link-time condition on a value that is no constant".to_string()) };
                use std::cmp::Ordering::*;
                let order = match (resolved, wanted) {
                    (Scalar::Bool(a), Scalar::Bool(b)) => Some(a.cmp(&b)),
                    (Scalar::Char(a), Scalar::Char(b)) => Some(a.cmp(&b)),
                    (Scalar::Str(a), Scalar::Str(b)) => Some(names::cmp_strings(self.cx.bytes(a), self.cx.bytes(b))),
                    (Scalar::Null, Scalar::Null) => Some(Equal),
                    (a, b) => match (a.number(), b.number()) {
                        (Some(a), Some(b)) => a.partial_cmp(&b),
                        _ => None,
                    },
                };
                Ok(match (comparison, order) {
                    (Comp::Ieq | Comp::Feq, o) => o == Some(Equal),
                    (Comp::Ine | Comp::Fne, o) => o != Some(Equal),
                    (Comp::Sgt | Comp::Ugt | Comp::Fgt, Some(o)) => o == Greater,
                    (Comp::Sge | Comp::Uge | Comp::Fge, Some(o)) => o != Less,
                    (Comp::Slt | Comp::Ult | Comp::Flt, Some(o)) => o == Less,
                    (Comp::Sle | Comp::Ule | Comp::Fle, Some(o)) => o != Greater,
                    _ => return self.fail("a link-time comparison of values of two kinds".to_string()),
                })
            }
        }
    }

    /// The value of a link-time method whose blocks each hold one instruction: a return, a jump
    /// or a link-time condition.
    fn interpret(&mut self, defn: &Defn) -> Res<Scalar> {
        let DefnKind::Define { insts, .. } = &defn.kind else { return self.fail("no method".to_string()) };
        let mut labels: FxMap<Local, usize> = FxMap::default();
        for (i, inst) in insts.iter().enumerate() {
            if let InstKind::Label { id, .. } = &inst.kind {
                labels.insert(*id, i);
            }
        }
        let message = format!("the link-time method {} cannot be interpreted", self.show(defn.name));
        let cannot = || message.clone();
        let mut locals: FxMap<Local, Val> = FxMap::default();
        let mut at = 0usize;
        for _ in 0..insts.len() + 1 {
            let Some(inst) = insts.get(at + 1) else { return Err(cannot()) };
            let jump = match &inst.kind {
                InstKind::Ret(value) => {
                    let mut value = value.clone();
                    for _ in 0..insts.len() + 1 {
                        match value {
                            Val::Local(id, _) => value = locals.get(&id).cloned().ok_or_else(cannot)?,
                            _ => break,
                        }
                    }
                    return Scalar::of(&value).ok_or_else(cannot);
                }
                InstKind::Jump(next) => next.clone(),
                InstKind::LinktimeIf { cond, then, otherwise } => {
                    if self.condition(cond)? {
                        then.clone()
                    } else {
                        otherwise.clone()
                    }
                }
                _ => return Err(cannot()),
            };
            let Next::Label(target, args) = jump else { return Err(cannot()) };
            let &index = labels.get(&target).ok_or_else(cannot)?;
            if let InstKind::Label { params, .. } = &insts[index].kind {
                for (&(param, _), arg) in params.iter().zip(args.iter()) {
                    locals.insert(param, arg.clone());
                }
            }
            at = index;
        }
        Err(cannot())
    }

    // ---- java.util.ServiceLoader ----

    /// A call of `ServiceLoader.load` on a class literal becomes the loader of the providers the
    /// build names for that service: the class path's `META-INF/services` is read here, at the
    /// link, since no file of it is there when the program runs.
    fn resolve_intrinsics(&mut self, mut defn: Defn) -> Res<Defn> {
        let DefnKind::Define { insts, .. } = &mut defn.kind else { return Ok(defn) };
        let mut fresh = insts.iter().map(max_local).max().unwrap_or(0) + 1;
        let mut out = Vec::with_capacity(insts.len());
        for inst in std::mem::take(insts) {
            let service = match &inst.kind {
                InstKind::Let { op: Op::Call { ptr: Val::Global(name, _), args, .. }, .. } if self.service_load.contains(name) => match &args[..] {
                    [Val::ClassOf(cls), ..] | [_, Val::ClassOf(cls), ..] => Some(*cls),
                    _ => None,
                },
                _ => None,
            };
            match service {
                Some(service) => self.service_loader_load(inst, service, &mut fresh, &mut out)?,
                None => out.push(inst),
            }
        }
        let DefnKind::Define { insts, .. } = &mut defn.kind else { return Ok(defn) };
        *insts = out;
        Ok(defn)
    }

    fn service_loader_load(&mut self, inst: Inst, service: Global, fresh: &mut Local, out: &mut Vec<Inst>) -> Res<()> {
        let pos = inst.pos;
        let InstKind::Let { id, unwind, scope, .. } = inst.kind else { return Ok(()) };
        let rt = self.rt;
        let loader = self.cx.top("java.util.ServiceLoader");
        let loader_module = self.cx.top("java.util.ServiceLoader$");
        let provider = self.cx.top("java.util.ServiceLoader$Provider");
        let (loader_ty, module_ty, provider_ty) = (self.cx.class_ty(loader), self.cx.class_ty(loader_module), self.cx.class_ty(provider));
        let mut next = || {
            *fresh += 1;
            *fresh - 1
        };
        let emit = |out: &mut Vec<Inst>, id: Local, op: Op| out.push(Inst { pos, kind: InstKind::Let { id, op, unwind: unwind.clone(), scope } });
        let service_name = String::from_utf8_lossy(self.cx.id(service)).into_owned();
        let wanted: Vec<String> = self.config.service_providers.iter().filter(|(s, _)| *s == service_name).flat_map(|(_, p)| p.iter().cloned()).collect();
        let module_local = next();
        emit(out, module_local, Op::Module(loader_module));
        let module_val = Val::Local(module_local, self.cx.ty(Type::Ref { name: loader_module, exact: true, nullable: false }));
        let create_sig = self.cx.method_sig("createIntrinsicProvider", &[rt.class_ty, rt.ptr, provider_ty], SigScope::Public);
        let create = self.cx.member(loader_module, create_sig);
        let create_ty = self.cx.ty(Type::Function(Box::new([module_ty, rt.class_ty, rt.ptr]), provider_ty));
        let mut providers = Vec::new();
        for &candidate in self.cp.service_providers(service) {
            let name = String::from_utf8_lossy(self.cx.id(candidate)).into_owned();
            if self.lookup(candidate, true)?.is_none() || !wanted.contains(&name) {
                continue;
            }
            let load = self.provider_loader(service, candidate, &unwind, pos, scope)?;
            let local = next();
            emit(out, local, Op::Call { ty: create_ty, ptr: Val::Global(create, rt.ptr), args: Box::new([module_val.clone(), Val::ClassOf(candidate), Val::Global(load, rt.ptr)]) });
            providers.push(Val::Local(local, provider_ty));
        }
        let array_local = next();
        emit(out, array_local, Op::Arrayalloc { ty: provider_ty, init: Val::ArrayValue(provider_ty, providers.into()), zone: None });
        let array_ty = self.cx.ty(Type::Array { of: provider_ty, nullable: false });
        emit(out, id, Op::Classalloc { name: loader, zone: None });
        let service_ty = self.cx.class_ty(service);
        let services = self.cx.ty(Type::Array { of: service_ty, nullable: true });
        let providers_ty = self.cx.ty(Type::Array { of: provider_ty, nullable: true });
        let ctor_sig = self.cx.ctor_sig(&[rt.class_ty, providers_ty]);
        let ctor = self.cx.member(loader, ctor_sig);
        let ctor_ty = self.cx.ty(Type::Function(Box::new([loader_ty, rt.class_ty, services]), rt.unit));
        let call = next();
        emit(out, call, Op::Call { ty: ctor_ty, ptr: Val::Global(ctor, rt.ptr), args: Box::new([Val::Local(id, loader_ty), Val::ClassOf(service), Val::Local(array_local, array_ty)]) });
        Ok(())
    }

    /// The method that makes a provider's instance, which the loader calls where its iterator
    /// comes to it.
    fn provider_loader(&mut self, service: Global, provider: Global, unwind: &Next, pos: Pos, scope: u32) -> Res<Global> {
        if let Some(&name) = self.provider_loaders.get(&provider) {
            return Ok(name);
        }
        let rt = self.rt;
        let mut id = b"loadProvider_".to_vec();
        id.extend_from_slice(self.cx.id(provider));
        let sig = self.cx.generated_sig(&id);
        let name = self.cx.member(service, sig);
        let provider_ty = self.cx.class_ty(provider);
        let ctor_sig = self.cx.ctor_sig(&[]);
        let ctor = self.cx.member(provider, ctor_sig);
        let ctor_ty = self.cx.ty(Type::Function(Box::new([provider_ty]), rt.unit));
        let exact = self.cx.ty(Type::Ref { name: provider, exact: true, nullable: false });
        let mut insts = vec![Inst { pos, kind: InstKind::Label { id: 1, params: Box::new([]) } }];
        let mut emit = |id: Local, op: Op| insts.push(Inst { pos, kind: InstKind::Let { id, op, unwind: unwind.clone(), scope } });
        emit(2, Op::Classalloc { name: provider, zone: None });
        emit(3, Op::Call { ty: ctor_ty, ptr: Val::Global(ctor, rt.ptr), args: Box::new([Val::Local(2, provider_ty)]) });
        // The provider's module may register it: it is loaded with the first instance.
        let module = self.cx.top(&format!("{}$", String::from_utf8_lossy(self.cx.id(provider))));
        if self.lookup(module, true)?.is_some() {
            emit(4, Op::Module(module));
        }
        insts.push(Inst { pos, kind: InstKind::Ret(Val::Local(2, exact)) });
        let ty = self.cx.ty(Type::Function(Box::new([]), provider_ty));
        self.provider_loaders.insert(provider, name);
        self.reach_defn(Defn { attrs: Attrs::default(), name, pos, kind: DefnKind::Define { ty, insts, debug: DebugInfo::default() } })?;
        Ok(name)
    }
}

/// The methods a call of `sig` on a value of this type can run: those of the allocated classes
/// that are it (`ScopeInfo.targets`).
/// `hashCode` of a `nir.Global`: a `Top` or a `Member`.
/// Every call met so far on a class, on a class above it or on a trait of theirs, gathered into one
/// set as Scala Native's `reachAllocation` gathers them: the class's, then each class above with its
/// traits after those above it, then the class's traits, each trait with those it extends
/// (docs/NATIVE.md, "The order of a link").
fn gathered_calls(scopes: &[Scope], id: ScopeId) -> scala::HashSet<Str> {
    // A scope met again has finished its walk (its parent and traits are made before it): what is skipped is a
    // repeat, a `++=` of sets added already, their size hints applied and elements there, the set left as it was.
    fn parents(scopes: &[Scope], p: ScopeId, calls: &mut scala::HashSet<Str>, met: &mut FxSet<ScopeId>) {
        if !met.insert(p) {
            return;
        }
        calls.add_all(&scopes[p as usize].calls);
        if let Some(above) = scopes[p as usize].parent {
            parents(scopes, above, calls, met);
        }
        for &t in &scopes[p as usize].traits {
            traits(scopes, t, calls, met);
        }
    }
    fn traits(scopes: &[Scope], t: ScopeId, calls: &mut scala::HashSet<Str>, met: &mut FxSet<ScopeId>) {
        if !met.insert(t) {
            return;
        }
        calls.add_all(&scopes[t as usize].calls);
        for &above in &scopes[t as usize].traits {
            traits(scopes, above, calls, met);
        }
    }
    let mut calls = scala::HashSet::default();
    let mut met = FxSet::default();
    calls.add_all(&scopes[id as usize].calls);
    if let Some(p) = scopes[id as usize].parent {
        parents(scopes, p, &mut calls, &mut met);
    }
    for &t in &scopes[id as usize].traits {
        traits(scopes, t, &mut calls, &mut met);
    }
    calls
}

pub fn global_hash(cx: &Cx, g: Global) -> i32 {
    let owner = scala::top_hash(cx.id(cx.owner(g)));
    match cx.sig(g) {
        Some(sig) => scala::member_hash(owner, scala::string_hash(cx.bytes(sig))),
        None => owner,
    }
}

/// A scope's implementors in the order of Scala Native's `SortedSet` of them, by name.
pub fn implementors_by_name(cx: &Cx, scopes: &[Scope], id: ScopeId) -> Vec<ScopeId> {
    let mut sorted = scopes[id as usize].implementors.clone();
    sorted.sort_by(|&a, &b| cx.cmp_globals(scopes[a as usize].name, scopes[b as usize].name));
    sorted
}

/// The methods a call of `sig` on a value of the scope may reach (`ScopeInfo.targets`): those of
/// the allocated classes that are it, in the order of the set Scala Native gathers them into, a
/// class's own before its subclasses' in the order they were linked, a trait's implementors by name.
pub fn targets(cx: &Cx, scopes: &[Scope], id: ScopeId, sig: Str) -> Vec<Global> {
    let scope = &scopes[id as usize];
    let mut out = scala::HashSet::default();
    let mut add = |c: ScopeId| {
        let cls = &scopes[c as usize];
        if cls.allocated() {
            if let Some(target) = cls.resolve(sig) {
                out.add(target, global_hash(cx, target));
            }
        }
    };
    if scope.is_trait {
        implementors_by_name(cx, scopes, id).into_iter().for_each(&mut add);
    } else {
        add(id);
        scope.subclasses.iter().for_each(|&c| add(c));
    }
    out.iter()
}

/// `ServiceLoader.load`, `load` with a class loader and `loadInstalled`, as the class's static
/// methods and as its module's.
fn service_load_methods(cx: &mut Cx, rt: &Rt) -> Vec<Global> {
    let loader = cx.top("java.util.ServiceLoader");
    let module = cx.top("java.util.ServiceLoader$");
    let class_loader = cx.top("java.lang.ClassLoader");
    let (loader_ty, class_loader_ty) = (cx.class_ty(loader), cx.class_ty(class_loader));
    let sigs = [
        cx.method_sig("load", &[rt.class_ty, loader_ty], SigScope::PublicStatic),
        cx.method_sig("load", &[rt.class_ty, class_loader_ty, loader_ty], SigScope::PublicStatic),
        cx.method_sig("loadInstalled", &[rt.class_ty, loader_ty], SigScope::PublicStatic),
    ];
    let mut out = Vec::new();
    for sig in sigs {
        out.push(cx.member(loader, sig));
        let public = cx.public_sig(sig);
        out.push(cx.member(module, public));
    }
    out
}

fn max_local(inst: &Inst) -> Local {
    fn next(n: &Next) -> Local {
        match n {
            Next::None => 0,
            Next::Unwind { exc, next: inner, .. } => (*exc).max(next(inner)),
            Next::Case(_, inner) => next(inner),
            Next::Label(id, _) => *id,
        }
    }
    match &inst.kind {
        InstKind::Label { id, params } => params.iter().map(|&(p, _)| p).fold(*id, Local::max),
        InstKind::Let { id, unwind, .. } => (*id).max(next(unwind)),
        InstKind::Throw { unwind, .. } | InstKind::Unreachable { unwind } => next(unwind),
        _ => 0,
    }
}

fn next_label(next: &Next) -> Option<Local> {
    match next {
        Next::None => None,
        Next::Unwind { next, .. } | Next::Case(_, next) => next_label(next),
        Next::Label(id, _) => Some(*id),
    }
}

/// `ControlFlow.removeDeadBlocks`: the blocks a path from the first one comes to, in their order.
fn remove_dead_blocks(insts: Vec<Inst>) -> Res<Vec<Inst>> {
    let is_cf = |inst: &Inst| !matches!(inst.kind, InstKind::Label { .. } | InstKind::Let { .. });
    let mut blocks: Vec<(Local, usize, usize)> = Vec::new();
    for (i, inst) in insts.iter().enumerate() {
        if let InstKind::Label { id, .. } = &inst.kind {
            let end = (i + 1..insts.len()).find(|&j| is_cf(&insts[j])).map_or(insts.len(), |j| j + 1);
            blocks.push((*id, i, end));
        }
    }
    let index: FxMap<Local, usize> = blocks.iter().enumerate().map(|(n, b)| (b.0, n)).collect();
    let mut visited = vec![false; blocks.len()];
    let mut todo = vec![0usize];
    if blocks.is_empty() || blocks[0].1 != 0 {
        return Err("a method that starts with no label".to_string());
    }
    while let Some(b) = todo.pop() {
        if std::mem::replace(&mut visited[b], true) {
            continue;
        }
        let (_, start, end) = blocks[b];
        let mut successors: Vec<Local> = Vec::new();
        for inst in &insts[start + 1..end] {
            match &inst.kind {
                InstKind::Let { unwind, .. } | InstKind::Throw { unwind, .. } | InstKind::Unreachable { unwind } => successors.extend(next_label(unwind)),
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
            todo.push(*index.get(&s).ok_or("a jump to a label the method has not")?);
        }
    }
    let mut keep = vec![false; insts.len()];
    for (b, &(_, start, end)) in blocks.iter().enumerate() {
        if visited[b] {
            keep[start..end].iter_mut().for_each(|k| *k = true);
        }
    }
    Ok(insts.into_iter().zip(keep).filter(|&(_, k)| k).map(|(inst, _)| inst).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The entry list is Scala Native's in its order: tests/native-link/entries.txt holds what its
    /// own `ScalaNative.entries` gave.
    #[test]
    fn entries_stand_in_scala_natives_order() {
        let expected = include_str!("../../tests/native-link/entries.txt");
        let mut cx = Cx::new();
        let rt = Rt::new(&mut cx);
        let mut got = String::new();
        for multithreading in [false, true] {
            got.push_str(&format!("== multithreading {}\n", multithreading));
            for g in entries(&mut cx, &rt, "fix.Shapes", multithreading, false) {
                let mut show = super::super::nir::show::Show::new(&cx);
                show.global(g);
                got.push_str(&String::from_utf8_lossy(&show.out));
                got.push('\n');
            }
        }
        let expected: String = expected.lines().filter(|l| !l.starts_with('#')).map(|l| format!("{}\n", l)).collect();
        assert_eq!(got, expected);
    }

    /// The gathering as Scala Native's `reachAllocation` walks, a scope once for each path to it, with
    /// the number of scopes met.
    fn gathered_on_every_path(scopes: &[Scope], id: ScopeId) -> (scala::HashSet<Str>, usize) {
        fn parents(scopes: &[Scope], p: ScopeId, calls: &mut scala::HashSet<Str>, met: &mut usize) {
            *met += 1;
            calls.add_all(&scopes[p as usize].calls);
            if let Some(above) = scopes[p as usize].parent {
                parents(scopes, above, calls, met);
            }
            for &t in &scopes[p as usize].traits {
                traits(scopes, t, calls, met);
            }
        }
        fn traits(scopes: &[Scope], t: ScopeId, calls: &mut scala::HashSet<Str>, met: &mut usize) {
            *met += 1;
            calls.add_all(&scopes[t as usize].calls);
            for &above in &scopes[t as usize].traits {
                traits(scopes, above, calls, met);
            }
        }
        let (mut calls, mut met) = (scala::HashSet::default(), 0);
        calls.add_all(&scopes[id as usize].calls);
        if let Some(p) = scopes[id as usize].parent {
            parents(scopes, p, &mut calls, &mut met);
        }
        for &t in &scopes[id as usize].traits {
            traits(scopes, t, &mut calls, &mut met);
        }
        (calls, met)
    }

    /// Diamonds of traits and generated hierarchies whose scopes are called with signatures of few
    /// hashes, some sets large enough to grow the gathered table: the guarded gathering leaves the
    /// set, its order and its table's length as the walk of every path leaves them.
    #[test]
    fn a_scope_met_again_leaves_the_gathered_calls_as_every_path_does() {
        let mut state = 0x2545f4914f6cdd1du64;
        let mut draw = move |n: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % n
        };
        // Signatures of a few hashes, a scope's calls up to more than a fresh table holds, the diamonds'
        // depths, and the generated hierarchies of traits and classes.
        const SIGNATURES: u64 = 48;
        const MOST_CALLS: u64 = 24;
        const DEEPEST: u32 = 12;
        const GENERATED: usize = 40;
        const TRAITS: usize = 24;
        const CLASSES: usize = 16;
        const MOST_PARENTS: u64 = 3;
        let hashes: [i32; 12] = std::array::from_fn(|_| draw(1 << 32) as u32 as i32);
        let calls_of = |draw: &mut dyn FnMut(u64) -> u64| {
            let mut calls = scala::HashSet::default();
            for _ in 0..draw(MOST_CALLS + 1) {
                let sig = draw(SIGNATURES) as u32;
                calls.add(Str(sig), hashes[sig as usize % hashes.len()]);
            }
            calls
        };
        let scope = |is_trait: bool, parent: Option<ScopeId>, traits: Vec<ScopeId>, calls: scala::HashSet<Str>, at: usize| Scope {
            name: Global(at as u32),
            attrs: Attrs::default(),
            pos: Pos::NONE,
            is_trait,
            is_module: false,
            parent,
            traits,
            members: Vec::new(),
            calls,
            responds: scala::HashMap::default(),
            default_responds: scala::HashMap::default(),
            implementors: Vec::new(),
            subclasses: Vec::new(),
            allocations: 0,
            linearized: Vec::new(),
            linearized_done: false,
        };
        let mut hierarchies: Vec<Vec<Scope>> = Vec::new();
        for depth in 1..=DEEPEST {
            // A0, B0 extends A0, then A(n) and B(n) each extending A(n-1) with B(n-1); a class above Leaf.
            let mut scopes = Vec::new();
            scopes.push(scope(true, None, vec![], calls_of(&mut draw), 0));
            scopes.push(scope(true, None, vec![0], calls_of(&mut draw), 1));
            for n in 1..=depth {
                let below = vec![2 * n - 2, 2 * n - 1];
                scopes.push(scope(true, None, below.clone(), calls_of(&mut draw), scopes.len()));
                scopes.push(scope(true, None, below, calls_of(&mut draw), scopes.len()));
            }
            let base = scopes.len() as ScopeId;
            scopes.push(scope(false, None, vec![], calls_of(&mut draw), scopes.len()));
            scopes.push(scope(false, Some(base), vec![2 * depth, 2 * depth + 1], calls_of(&mut draw), scopes.len()));
            hierarchies.push(scopes);
        }
        for _ in 0..GENERATED {
            // Traits extending traits made before them, then classes with a parent and traits made before them.
            let mut scopes = Vec::new();
            for t in 0..TRAITS {
                let traits: Vec<ScopeId> = (0..if t == 0 { 0 } else { draw(MOST_PARENTS + 1) }).map(|_| draw(t as u64) as ScopeId).collect();
                scopes.push(scope(true, None, traits, calls_of(&mut draw), t));
            }
            for c in TRAITS..TRAITS + CLASSES {
                let parent = if c == TRAITS { None } else { Some((TRAITS as u64 + draw((c - TRAITS) as u64)) as ScopeId) };
                let traits: Vec<ScopeId> = (0..draw(MOST_PARENTS + 1)).map(|_| draw(TRAITS as u64) as ScopeId).collect();
                scopes.push(scope(false, parent, traits, calls_of(&mut draw), c));
            }
            hierarchies.push(scopes);
        }
        let above = |scopes: &[Scope], id: ScopeId| {
            let mut seen: FxSet<ScopeId> = FxSet::default();
            let mut stack: Vec<ScopeId> = scopes[id as usize].parent.into_iter().chain(scopes[id as usize].traits.iter().copied()).collect();
            while let Some(s) = stack.pop() {
                if seen.insert(s) {
                    stack.extend(scopes[s as usize].parent);
                    stack.extend(&scopes[s as usize].traits);
                }
            }
            seen.len()
        };
        let (mut repeats, mut grown) = (0, 0);
        for scopes in &hierarchies {
            for id in (0..scopes.len() as ScopeId).filter(|&id| !scopes[id as usize].is_trait) {
                let guarded = gathered_calls(scopes, id);
                let (every_path, met) = gathered_on_every_path(scopes, id);
                assert_eq!(guarded.iter(), every_path.iter());
                assert_eq!(guarded.table_len(), every_path.table_len());
                repeats += usize::from(met > above(scopes, id));
                grown += usize::from(guarded.table_len() > 16);
            }
        }
        // Not vacuous: walks that meet a scope more than once, and gathered tables that grew.
        assert!(repeats > 0 && grown > 0, "{} walks with repeats, {} grown tables", repeats, grown);
    }
}
