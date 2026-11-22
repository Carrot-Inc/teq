//! What the code generator knows of every class and trait before it lowers a method, after
//! `codegen.Metadata` of Scala Native 0.5.12 and what it builds: the class ids and their ranges,
//! the trait ids, the fields' layout, the virtual tables, the interface tables, the tables of
//! structural methods, the type records and the array of modules. The C runtime reads the type
//! records and the object and array headers as laid out here.

use super::nir::names::{self, Rt};
use super::nir::*;
use super::reach::{Info, Linked, ScopeId};
use crate::intern::FxMap;
use std::sync::Arc;

pub const PTR_SIZE: i64 = 8;

fn is_ref(cx: &Cx, ty: Ty) -> bool {
    matches!(cx.type_of(ty), Type::Null | Type::Unit | Type::Array { .. } | Type::Ref { .. })
}

/// `MemoryLayout.sizeOf`.
pub fn size_of(cx: &Cx, ty: Ty) -> i64 {
    match cx.type_of(ty) {
        Type::Null | Type::Unit | Type::Array { .. } | Type::Ref { .. } | Type::Nothing | Type::Ptr | Type::Size => PTR_SIZE,
        Type::Bool | Type::Byte => 1,
        Type::Char | Type::Short => 2,
        Type::Int | Type::Float => 4,
        Type::Long | Type::Double => 8,
        Type::Int128 => 16,
        Type::ArrayValue(of, n) => size_of(cx, *of) * *n as i64,
        Type::StructValue(tys) => layout_of(cx, tys).size,
        Type::Vararg | Type::Function(..) | Type::Virtual | Type::Var(_) => 0,
    }
}

/// `MemoryLayout.alignmentOf`.
pub fn alignment_of(cx: &Cx, ty: Ty) -> i64 {
    match cx.type_of(ty) {
        Type::ArrayValue(of, _) => alignment_of(cx, *of),
        Type::StructValue(tys) => tys.iter().map(|&t| alignment_of(cx, t)).max().unwrap_or(1),
        _ => size_of(cx, ty).max(1),
    }
}

fn align(offset: i64, alignment: i64) -> i64 {
    let mask = alignment - 1;
    if offset & mask == 0 {
        offset
    } else {
        offset + alignment - (offset & mask)
    }
}

pub struct Layout {
    pub size: i64,
    pub tys: Vec<(Ty, i64)>,
}

/// `MemoryLayout.apply`: each type at the next offset its alignment allows.
pub fn layout_of(cx: &Cx, tys: &[Ty]) -> Layout {
    let (mut offset, mut max_align) = (0i64, 1i64);
    let mut out = Vec::with_capacity(tys.len());
    for &ty in tys {
        let a = alignment_of(cx, ty);
        offset = align(offset, a);
        out.push((ty, offset));
        offset += size_of(cx, ty);
        max_align = max_align.max(a);
    }
    Layout { size: align(offset, max_align), tys: out }
}

pub struct FieldLayout {
    /// The class's fields, its parents' first.
    pub entries: Vec<Global>,
    pub layout: Layout,
    pub struct_ty: Ty,
    pub size: i64,
    /// The offsets of the fields that hold a reference, ended by -1: what the collector scans.
    pub ref_offsets: Val,
}

impl FieldLayout {
    /// The place of a field in the object's struct, after its header.
    pub fn index(&self, field: Global) -> Option<i32> {
        self.entries.iter().position(|&f| f == field).map(|i| i as i32 + 1)
    }
}

pub struct VTable {
    pub slots: Vec<Str>,
    pub impls: Vec<Val>,
    pub ty: Ty,
}

impl VTable {
    pub fn index(&self, sig: Str) -> Option<i32> {
        self.slots.iter().position(|&s| s == sig).map(|i| i as i32)
    }
    pub fn value(&self, rt: &Rt) -> Val {
        Val::ArrayValue(rt.ptr, self.impls.clone().into())
    }
}

pub struct ITable {
    pub use_fast: bool,
    pub size: i32,
    pub value: Val,
}

pub struct Meta {
    pub multithreading: bool,
    /// By scope: a class's id, a trait's id with its marker bit.
    pub ids: Vec<i32>,
    /// By scope: the ids of a class and the classes below it, both ends included.
    pub ranges: Vec<(i32, i32)>,
    /// The classes in the order of their ids.
    pub classes: Vec<ScopeId>,
    /// The traits by name.
    pub traits: Vec<ScopeId>,
    pub vtables: FxMap<ScopeId, VTable>,
    pub layouts: FxMap<ScopeId, FieldLayout>,
    pub itables: FxMap<ScopeId, ITable>,
    /// By class: the structural methods it answers, its parents' first.
    pub dynmethods: FxMap<ScopeId, Vec<Global>>,
    pub dynmaps: FxMap<ScopeId, Val>,
    pub uses_dynmap: bool,
    /// By scope: the name of its type record.
    pub rtti_names: Vec<Global>,
    pub rtti_structs: Vec<Ty>,
    pub can_use_fast_itables: Vec<bool>,
    pub can_always_use_fast_itables: bool,
    /// By scope: its called signatures in the order Scala Native's set gives them, and of those
    /// the ones a trait's table of methods holds.
    pub calls: Vec<Vec<Str>>,
    pub methods: Vec<Vec<Str>>,
    pub modules: Vec<ScopeId>,
    pub module_index: FxMap<ScopeId, i32>,
    pub constant_modules: Vec<bool>,
    pub defns: FxMap<Global, usize>,
    // `CommonMemoryLayouts`.
    pub rtti_layout: Ty,
    pub class_rtti_layout: Ty,
    pub object_header: Ty,
    pub array_header: Ty,
    pub itable_entry: Ty,
    pub rtti_class_id_idx: i32,
    pub class_rtti_itable_size_idx: i32,
    pub class_rtti_itables_idx: i32,
    pub class_rtti_dynmap_idx: i32,
    pub class_rtti_vtable_idx: i32,
    pub array_header_length_idx: i32,
    class_rtti_base: Vec<Ty>,
}

const TRAIT_MARKER: i32 = 1 << 31;
/// `ITable.MaxColorBits`: a class's interface table has 1, 2, 4, ... up to 2^6 entries, the fewest
/// in which its traits' ids fall into places of their own, else one entry for each trait, searched.
const ITABLE_COLOR_BITS: u32 = 6;

/// `Allowlist.constantModules`.
const CONSTANT_MODULES: &[&str] = &[
    "scala.scalanative.runtime.BoxedUnit$",
    "scala.scalanative.runtime.LazyVals$",
    "scala.scalanative.runtime.MemoryLayout$",
    "scala.scalanative.runtime.MemoryLayout$Array$",
    "scala.scalanative.runtime.MemoryLayout$Object$",
    "scala.scalanative.runtime.MemoryLayout$Rtti$",
    "scala.scalanative.runtime.monitor.BasicMonitor$",
    "scala.scalanative.runtime.monitor.package$LockWord",
    "scala.scalanative.runtime.monitor.package$LockWord$",
    "scala.scalanative.runtime.monitor.package$LockWord32$",
    "scala.scalanative.runtime.monitor.package$LockType$",
    "scala.scalanative.unsafe.Tag$",
    "scala.scalanative.unsafe.Tag$Unit$",
    "scala.scalanative.unsafe.Tag$Boolean$",
    "scala.scalanative.unsafe.Tag$Char$",
    "scala.scalanative.unsafe.Tag$Byte$",
    "scala.scalanative.unsafe.Tag$UByte$",
    "scala.scalanative.unsafe.Tag$Short$",
    "scala.scalanative.unsafe.Tag$UShort$",
    "scala.scalanative.unsafe.Tag$Int$",
    "scala.scalanative.unsafe.Tag$UInt$",
    "scala.scalanative.unsafe.Tag$Long$",
    "scala.scalanative.unsafe.Tag$ULong$",
    "scala.scalanative.unsafe.Tag$Float$",
    "scala.scalanative.unsafe.Tag$Double$",
    "scala.scalanative.unsafe.Tag$Size$",
    "scala.scalanative.unsafe.Tag$USize$",
    "scala.scalanative.unsafe.Tag$Nat0$",
    "scala.scalanative.unsafe.Tag$Nat1$",
    "scala.scalanative.unsafe.Tag$Nat2$",
    "scala.scalanative.unsafe.Tag$Nat3$",
    "scala.scalanative.unsafe.Tag$Nat4$",
    "scala.scalanative.unsafe.Tag$Nat5$",
    "scala.scalanative.unsafe.Tag$Nat6$",
    "scala.scalanative.unsafe.Tag$Nat7$",
    "scala.scalanative.unsafe.Tag$Nat8$",
    "scala.scalanative.unsafe.Tag$Nat9$",
    "java.lang.Math$",
];

pub fn is_allowlisted_constant_module(id: &[u8]) -> bool {
    CONSTANT_MODULES.iter().any(|m| m.as_bytes() == id)
}

fn struct_ty(cx: &mut Cx, tys: Vec<Ty>) -> Ty {
    cx.ty(Type::StructValue(tys.into_boxed_slice()))
}

impl Meta {
    pub fn new(cx: &mut Cx, rt: &Rt, linked: &Linked, proxies: &[Defn]) -> Result<Meta, String> {
        let scopes = &linked.scopes;
        let n = scopes.len();
        let mt = linked.multithreading;
        let lock: Vec<Ty> = if mt { vec![rt.ptr] } else { Vec::new() };
        let (ptr, int) = (rt.ptr, rt.int);
        let rtti_layout = struct_ty(cx, [vec![ptr], lock.clone(), vec![int, int, ptr, ptr]].concat());
        let uses_dynmap = !linked.dynsigs.is_empty();
        let mut class_rtti_base = vec![rtti_layout, int, int, ptr, int, ptr, ptr];
        if uses_dynmap {
            class_rtti_base.push(ptr);
        }
        let empty_vtable = cx.ty(Type::ArrayValue(ptr, 0));
        let class_rtti_layout = struct_ty(cx, [class_rtti_base.clone(), vec![empty_vtable]].concat());
        let object_header = struct_ty(cx, [vec![ptr], lock.clone()].concat());
        let array_header = struct_ty(cx, [vec![ptr], lock.clone(), vec![int, int]].concat());
        let itable_entry = struct_ty(cx, vec![int, ptr]);
        let lock_idx = if mt { 1 } else { 0 };
        let defns: FxMap<Global, usize> = linked.defns.iter().enumerate().map(|(i, d)| (d.name, i)).collect();
        let scope_of = |name: Global| match linked.infos.get(&name) {
            Some(&Info::Scope(id)) => Some(id),
            _ => None,
        };

        // The class ids: a class's own, then its subclasses' by name, the arrays first under Object.
        let mut ids = vec![0i32; n];
        let mut ranges = vec![(0i32, -1i32); n];
        let mut classes: Vec<ScopeId> = Vec::new();
        let mut next_id = 0i32;
        let mut roots: Vec<ScopeId> = rt.primitives.iter().filter_map(|&p| scope_of(p)).collect();
        let object = scope_of(rt.object).ok_or("java.lang.Object is not linked")?;
        roots.push(object);
        for root in roots {
            // (class, where it stands among its parent's children to visit, its first id)
            let mut stack: Vec<(ScopeId, Vec<ScopeId>, usize, i32)> = Vec::new();
            let children = |c: ScopeId, top: bool| {
                let mut direct: Vec<ScopeId> = scopes[c as usize].subclasses.iter().copied().filter(|&s| scopes[s as usize].parent == Some(c)).collect();
                if top {
                    let (arrays, other): (Vec<ScopeId>, Vec<ScopeId>) = direct.iter().partition(|&&s| scopes[s as usize].name == rt.generic_array);
                    direct = [arrays, other].concat();
                }
                direct
            };
            classes.push(root);
            stack.push((root, children(root, root == object), 0, next_id));
            next_id += 1;
            while let Some((cls, kids, at, start)) = stack.last_mut() {
                if *at < kids.len() {
                    let kid = kids[*at];
                    *at += 1;
                    classes.push(kid);
                    let entry = (kid, children(kid, false), 0, next_id);
                    next_id += 1;
                    stack.push(entry);
                } else {
                    ids[*cls as usize] = *start;
                    ranges[*cls as usize] = (*start, next_id - 1);
                    stack.pop();
                }
            }
        }

        // The trait ids (`TraitsUniverse`): two traits of one class get two colours, and an id
        // is a trait's place among those of its colour, above the colour's bits.
        let mut traits: Vec<ScopeId> = (0..n as ScopeId).filter(|&s| scopes[s as usize].is_trait).collect();
        traits.sort_by(|&a, &b| cx.cmp_globals(scopes[a as usize].name, scopes[b as usize].name));
        let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); traits.len()];
        {
            let mut of_class: FxMap<ScopeId, Vec<u32>> = FxMap::default();
            for (i, &t) in traits.iter().enumerate() {
                for &cls in &scopes[t as usize].implementors {
                    let related = of_class.entry(cls).or_default();
                    for &r in related.iter() {
                        neighbours[i].push(r);
                        neighbours[r as usize].push(i as u32);
                    }
                    related.push(i as u32);
                }
            }
        }
        let mut colors = vec![u32::MAX; traits.len()];
        let mut max_color = 0u32;
        let mut taken: Vec<bool> = Vec::new();
        for i in 0..traits.len() {
            taken.clear();
            taken.resize(max_color as usize + 1, false);
            for &nb in &neighbours[i] {
                let c = colors[nb as usize];
                if c != u32::MAX {
                    taken[c as usize] = true;
                }
            }
            colors[i] = match taken.iter().position(|&t| !t) {
                Some(c) => c as u32,
                None => {
                    max_color += 1;
                    max_color
                }
            };
        }
        let color_bits = ((max_color as f64 + 1.0).ln() / 2f64.ln()).ceil() as i32;
        let mut next_of_color: Vec<i32> = vec![0; max_color as usize + 1];
        for (i, &t) in traits.iter().enumerate() {
            let index = next_of_color[colors[i] as usize];
            next_of_color[colors[i] as usize] += 1;
            ids[t as usize] = (index << color_bits) | colors[i] as i32 | TRAIT_MARKER;
        }

        // A class's called signatures as Scala Native's set gives them.
        let mut calls: Vec<Vec<Str>> = Vec::with_capacity(n);
        let mut methods: Vec<Vec<Str>> = Vec::with_capacity(n);
        for scope in scopes {
            let sigs: Vec<Str> = scope.calls.iter();
            methods.push(sigs.iter().copied().filter(|&s| names::is_virtual(cx.bytes(s))).collect());
            calls.push(sigs);
        }

        let rtti_sig = cx.generated_sig(b"type");
        let rtti_names: Vec<Global> = scopes.iter().map(|s| cx.member(s.name, rtti_sig)).collect();

        // The modules that are allocated, in the order of the class ids.
        let mut modules = Vec::new();
        let mut module_index = FxMap::default();
        for &cls in &classes {
            let scope = &scopes[cls as usize];
            if scope.is_module && scope.allocated() {
                module_index.insert(cls, modules.len() as i32);
                modules.push(cls);
            }
        }

        let mut meta = Meta {
            multithreading: mt,
            ids,
            ranges,
            classes,
            traits,
            vtables: FxMap::default(),
            layouts: FxMap::default(),
            itables: FxMap::default(),
            dynmethods: FxMap::default(),
            dynmaps: FxMap::default(),
            uses_dynmap,
            rtti_names,
            rtti_structs: vec![rtti_layout; n],
            can_use_fast_itables: vec![true; n],
            can_always_use_fast_itables: true,
            calls,
            methods,
            modules,
            module_index,
            constant_modules: vec![false; n],
            defns,
            rtti_layout,
            class_rtti_layout,
            object_header,
            array_header,
            itable_entry,
            rtti_class_id_idx: lock_idx + 1,
            class_rtti_itable_size_idx: 4,
            class_rtti_itables_idx: 5,
            class_rtti_dynmap_idx: if uses_dynmap { 7 } else { -1 },
            class_rtti_vtable_idx: if uses_dynmap { 8 } else { 7 },
            array_header_length_idx: lock_idx + 1,
            class_rtti_base,
        };

        for at in 0..meta.classes.len() {
            let cls = meta.classes[at];
            let scope = &scopes[cls as usize];
            let ptr_global = |g: Option<Global>| g.map_or(Val::Null, |g| Val::Global(g, ptr));

            // The virtual table: the parent's slots, then the signatures called on the class
            // that more than one method answers.
            let mut slots: Vec<Str> = scope.parent.map_or(Vec::new(), |p| meta.vtables[&p].slots.clone());
            for &sig in &meta.calls[cls as usize] {
                if !slots.contains(&sig) && super::reach::targets(cx, scopes, cls, sig).len() > 1 {
                    slots.push(sig);
                }
            }
            let impls: Vec<Val> = slots.iter().map(|&sig| ptr_global(scope.resolve(sig))).collect();
            let ty = cx.ty(Type::ArrayValue(ptr, slots.len() as i32));
            meta.vtables.insert(cls, VTable { slots, impls, ty });

            let layout = meta.field_layout(cx, rt, linked, cls)?;
            meta.layouts.insert(cls, layout);

            if uses_dynmap {
                let own: Vec<Global> = proxies.iter().map(|p| p.name).filter(|&p| cx.owner(p) == scope.name).collect();
                let mut all: Vec<Global> = scope.parent.map_or(Vec::new(), |p| meta.dynmethods[&p].clone());
                all.retain(|&m| !own.iter().any(|&o| cx.sig(o) == cx.sig(m)));
                all.extend(own);
                let value = dynmethod_map(cx, rt, &all, &linked.dynsigs);
                meta.dynmethods.insert(cls, all);
                meta.dynmaps.insert(cls, value);
            }

            // The interface table: by a trait's id where the ids' low bits tell the class's
            // traits apart in a table of at most 64, else sorted by id for a search.
            let mut implemented: Vec<(ScopeId, i32)> = Vec::new();
            for &l in &scope.linearized {
                if scopes[l as usize].is_trait && !implemented.iter().any(|&(t, _)| t == l) {
                    implemented.push((l, meta.ids[l as usize]));
                }
            }
            let fast_size: Option<usize> = if implemented.is_empty() {
                Some(0)
            } else if implemented.len() > 1 << ITABLE_COLOR_BITS {
                None
            } else {
                (0..=ITABLE_COLOR_BITS).map(|bits| 1usize << bits).find(|&size| {
                    let mut occupied = vec![false; size];
                    implemented.iter().all(|&(_, id)| !std::mem::replace(&mut occupied[id as usize & (size - 1)], true))
                })
            };
            let size = fast_size.unwrap_or(implemented.len());
            let mut entries = vec![Val::StructValue(Arc::new([Val::Int(0), Val::Null])); size];
            implemented.sort_by_key(|&(_, id)| id);
            for (idx, &(trt, id)) in implemented.iter().enumerate() {
                let position = fast_size.map_or(idx, |size| id as usize & (size - 1));
                let trait_methods = &meta.methods[trt as usize];
                let table = if trait_methods.is_empty() {
                    Val::Null
                } else {
                    Val::Const(Arc::new(Val::ArrayValue(ptr, trait_methods.iter().map(|&sig| ptr_global(scope.resolve(sig))).collect())))
                };
                entries[position] = Val::StructValue(Arc::new([Val::Int(id), table]));
            }
            meta.can_use_fast_itables[cls as usize] = fast_size.is_some();
            meta.itables.insert(cls, ITable { use_fast: fast_size.is_some(), size: size as i32, value: Val::ArrayValue(itable_entry, entries.into()) });

            let vtable_ty = meta.vtables[&cls].ty;
            meta.rtti_structs[cls as usize] = struct_ty(cx, [meta.class_rtti_base.clone(), vec![vtable_ty]].concat());
        }
        for &t in &meta.traits.clone() {
            meta.can_use_fast_itables[t as usize] = scopes[t as usize].implementors.iter().all(|&c| meta.can_use_fast_itables[c as usize]);
        }
        meta.can_always_use_fast_itables = meta.can_use_fast_itables.iter().all(|&b| b);

        // `Class.isConstantModule`: a module with no state and nothing to run when it is made.
        let ctor = cx.ctor_sig(&[]);
        for id in 0..n {
            let scope = &scopes[id];
            if scope.is_trait || !scope.is_module {
                continue;
            }
            let no_fields = meta.layouts.get(&(id as ScopeId)).is_none_or(|l| l.entries.is_empty());
            let init = cx.member(scope.name, ctor);
            let empty_ctor = match linked.infos.get(&init) {
                None => true,
                Some(Info::Method { .. }) => match meta.defns.get(&init).map(|&i| &linked.defns[i].kind) {
                    Some(DefnKind::Define { insts, .. }) => matches!(&insts[..], [Inst { kind: InstKind::Label { .. }, .. }, Inst { kind: InstKind::Ret(_), .. }]),
                    _ => false,
                },
                Some(_) => false,
            };
            meta.constant_modules[id] = is_allowlisted_constant_module(cx.id(scope.name)) || scope.attrs.is_extern || (empty_ctor && no_fields);
        }
        Ok(meta)
    }

    fn field_layout(&self, cx: &mut Cx, rt: &Rt, linked: &Linked, cls: ScopeId) -> Result<FieldLayout, String> {
        let scope = &linked.scopes[cls as usize];
        let mut entries: Vec<Global> = scope.parent.map_or(Vec::new(), |p| self.layouts[&p].entries.clone());
        entries.extend(scope.members.iter().copied().filter(|m| matches!(linked.infos.get(m), Some(Info::Field { .. }))));
        let field = |g: Global| -> Result<(Ty, &Attrs), String> {
            match self.defns.get(&g).map(|&i| &linked.defns[i]) {
                Some(Defn { attrs, kind: DefnKind::Var { ty, .. } | DefnKind::Const { ty, .. }, .. }) => Ok((*ty, attrs)),
                _ => Err("a field with no definition".to_string()),
            }
        };
        let group = |g: Global| field(g).ok().and_then(|(_, a)| a.align).and_then(|(_, group)| group);
        let custom = entries.iter().any(|&f| field(f).is_ok_and(|(_, a)| a.align.is_some()));
        let layout = if custom {
            // The fields of one `@align` group stand together, the fields of no group first.
            entries.sort_by(|&a, &b| match (group(a), group(b)) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (Some(x), Some(y)) => names::cmp_strings(cx.bytes(x), cx.bytes(y)),
            });
            let padding = cx.str(b"scala.scalanative.meta.linktimeinfo.contendedPaddingWidth");
            let dynamic_width = match linked.resolved.get(&padding) {
                Some(Val::Int(w)) => *w as i64,
                _ => return Err("the link-time property contendedPaddingWidth is not resolved".to_string()),
            };
            let width = |size: i32| if size == -1 { dynamic_width } else { size as i64 };
            let (mut offset, mut max_align) = (0i64, 1i64);
            let mut pos: Vec<(Ty, i64)> = Vec::new();
            macro_rules! add_field {
                ($ty:expr, $alignment:expr) => {{
                    let ty: Ty = $ty;
                    let alignment: i64 = $alignment.unwrap_or_else(|| alignment_of(cx, ty));
                    max_align = max_align.max(alignment);
                    offset = align(offset, alignment);
                    pos.push((ty, offset));
                    offset += size_of(cx, ty);
                }};
            }
            macro_rules! add_padding {
                ($alignment:expr) => {{
                    let remaining = align(offset, $alignment) - offset;
                    if remaining > 0 {
                        let last = pos.len() - 1;
                        let pad = cx.ty(Type::ArrayValue(rt.byte, remaining as i32));
                        pos[last].0 = cx.ty(Type::StructValue(Box::new([pos[last].0, pad])));
                        offset += remaining;
                    }
                }};
            }
            add_field!(self.object_header, None::<i64>);
            let mut at = 0;
            while at < entries.len() {
                let (ty, attrs) = field(entries[at])?;
                let head_group = attrs.align.and_then(|(_, g)| g);
                let mut tail = 0;
                if head_group.is_some() {
                    while at + 1 + tail < entries.len() && group(entries[at + 1 + tail]) == head_group {
                        tail += 1;
                    }
                }
                let mut align_size = attrs.align.map(|(size, _)| width(size));
                if let Some(size) = align_size.as_mut() {
                    for &f in &entries[at + 1..at + 1 + tail] {
                        if let Some((s, _)) = field(f)?.1.align {
                            *size = (*size).max(width(s));
                        }
                    }
                }
                if let Some(size) = align_size {
                    add_padding!(size);
                }
                add_field!(ty, align_size);
                for i in 0..tail {
                    add_field!(field(entries[at + 1 + i])?.0, None::<i64>);
                }
                if let Some(size) = align_size {
                    add_padding!(size);
                }
                at += 1 + tail;
            }
            Layout { size: align(offset, max_align), tys: pos }
        } else {
            let header = if rt.array_element(scope.name).is_some() { self.array_header } else { self.object_header };
            let mut tys = vec![header];
            for &f in &entries {
                tys.push(field(f)?.0);
            }
            layout_of(cx, &tys)
        };
        let mut offsets: Vec<Val> = Vec::new();
        for &(ty, offset) in &layout.tys {
            let scanned = is_ref(cx, ty)
                || match cx.type_of(ty) {
                    Type::StructValue(tys) => matches!(&tys[..], [first, pad] if is_ref(cx, *first) && matches!(cx.type_of(*pad), Type::ArrayValue(of, _) if *of == rt.byte)),
                    _ => false,
                };
            if scanned {
                offsets.push(Val::Int(offset as i32));
            }
        }
        offsets.push(Val::Int(-1));
        let struct_ty = struct_ty(cx, layout.tys.iter().map(|&(ty, _)| ty).collect());
        Ok(FieldLayout { entries, size: layout.size, layout, struct_ty, ref_offsets: Val::Const(Arc::new(Val::ArrayValue(rt.int, offsets.into()))) })
    }

    pub fn rtti_const(&self, rt: &Rt, scope: ScopeId) -> Val {
        Val::Global(self.rtti_names[scope as usize], rt.ptr)
    }

    pub fn lock_word_vals(&self) -> Vec<Val> {
        if self.multithreading {
            vec![Val::Null]
        } else {
            Vec::new()
        }
    }

    /// `RuntimeTypeInformation.value`: a class's or a trait's type record.
    pub fn rtti_value(&self, cx: &mut Cx, rt: &Rt, linked: &Linked, id: ScopeId) -> Val {
        let scope = &linked.scopes[id as usize];
        let full = String::from_utf8_lossy(cx.id(scope.name)).into_owned();
        let runtime = full.strip_prefix("scala.scalanative.runtime.");
        let primitive = runtime.is_some_and(|r| r.starts_with("Primitive") || r == "RawSize" || r == "RawPtr");
        let type_name = match runtime {
            Some("PrimitiveByte") => "byte",
            Some("PrimitiveShort") => "short",
            Some("PrimitiveInt") => "int",
            Some("PrimitiveLong") => "long",
            Some("PrimitiveChar") => "char",
            Some("PrimitiveFloat") => "float",
            Some("PrimitiveDouble") => "double",
            Some("PrimitiveBoolean") => "boolean",
            Some("PrimitiveUnit") => "void",
            Some("RawSize") => "size",
            Some("RawPtr") => "pointer",
            Some("BoxedUnit$") => "scala.runtime.BoxedUnit",
            _ => full.as_str(),
        };
        let type_name = cx.str(type_name.as_bytes());
        let mut interfaces: Vec<ScopeId> = scope.linearized.iter().copied().filter(|&l| l != id && linked.scopes[l as usize].is_trait).collect();
        interfaces.sort_by_key(|&t| self.ids[t as usize]);
        let class_rtti = Val::Global(self.rtti_names[match linked.infos.get(&rt.class) {
            Some(&Info::Scope(c)) => c as usize,
            _ => id as usize,
        }], rt.ptr);
        let mut base = vec![class_rtti];
        base.extend(self.lock_word_vals());
        base.push(Val::Int(self.ids[id as usize]));
        base.push(Val::Int(interfaces.len() as i32));
        base.push(Val::Const(Arc::new(Val::ArrayValue(rt.ptr, interfaces.iter().map(|&t| self.rtti_const(rt, t)).collect()))));
        base.push(Val::String(type_name));
        let base = Val::StructValue(base.into());
        if scope.is_trait {
            return base;
        }
        let type_size = if primitive {
            match runtime {
                Some("PrimitiveByte" | "PrimitiveBoolean") => 1,
                Some("PrimitiveShort") => 2,
                Some("PrimitiveInt" | "PrimitiveChar" | "PrimitiveFloat") => 4,
                _ => 8,
            }
        } else {
            self.layouts[&id].size as i32
        };
        let itable = &self.itables[&id];
        let itables_size = if itable.use_fast { (itable.size - 1).max(0) } else { -itable.size };
        let mut fields = vec![
            base,
            Val::Int(type_size),
            Val::Int(self.ranges[id as usize].1),
            self.layouts[&id].ref_offsets.clone(),
            Val::Int(itables_size),
            Val::Const(Arc::new(itable.value.clone())),
            scope.parent.map_or(Val::Null, |p| self.rtti_const(rt, p)),
        ];
        if self.uses_dynmap {
            fields.push(self.dynmaps[&id].clone());
        }
        fields.push(self.vtables[&id].value(rt));
        Val::StructValue(fields.into())
    }
}

fn modulo(a: i32, b: i32) -> i32 {
    let m = a % b;
    if m < 0 {
        m + b
    } else {
        m
    }
}

/// `DynmethodPerfectHashMap.hash`.
fn dyn_hash(key: i32, salt: i32) -> i32 {
    key.wrapping_add(salt.wrapping_mul(31)) ^ salt
}

/// `PerfectHashMap`: the keys (a signature's index among the structural signatures) placed with
/// no collision, by a first hash that names a bucket and a second one, salted per bucket, that
/// names the slot; a bucket of one key takes a free slot, named by a negative salt.
fn perfect_hash(entries: &[i32]) -> (Vec<i32>, Vec<Option<i32>>) {
    let mut size = entries.len();
    loop {
        if size == 0 {
            return (Vec::new(), Vec::new());
        }
        let mut buckets: Vec<Vec<i32>> = vec![Vec::new(); size];
        for &key in entries {
            buckets[modulo(dyn_hash(key, 0), size as i32) as usize].push(key);
        }
        buckets.sort_by_key(|b| std::cmp::Reverse(b.len()));
        let mut keys: FxMap<i32, i32> = FxMap::default();
        let mut values: Vec<Option<i32>> = vec![None; size];
        let mut failed = false;
        for bucket in buckets.iter().take_while(|b| b.len() > 1) {
            let mut d = 1;
            let found = loop {
                if d > 10000 {
                    break None;
                }
                let mut slots: Vec<i32> = Vec::new();
                let fits = bucket.iter().all(|&key| {
                    let slot = modulo(dyn_hash(key, d), size as i32);
                    let free = values[slot as usize].is_none() && !slots.contains(&slot);
                    slots.push(slot);
                    free
                });
                if fits {
                    break Some(d);
                }
                d += 1;
            };
            let Some(d) = found else {
                failed = true;
                break;
            };
            for &key in bucket {
                values[modulo(dyn_hash(key, d), size as i32) as usize] = Some(key);
            }
            keys.insert(modulo(dyn_hash(bucket[0], 0), size as i32), d);
        }
        if failed {
            size += 1;
            continue;
        }
        let free: Vec<usize> = (0..size).filter(|&i| values[i].is_none()).collect();
        for (bucket, &slot) in buckets.iter().filter(|b| b.len() == 1).zip(&free) {
            keys.insert(modulo(dyn_hash(bucket[0], 0), size as i32), -(slot as i32) - 1);
            values[slot] = Some(bucket[0]);
        }
        return ((0..size as i32).map(|i| keys.get(&i).copied().unwrap_or(0)).collect(), values);
    }
}

/// `DynmethodPerfectHashMap`: a class's structural methods by their signature's index.
fn dynmethod_map(cx: &Cx, rt: &Rt, methods: &[Global], dynsigs: &[Str]) -> Val {
    let mut by_index: Vec<(i32, Global)> = Vec::new();
    for &m in methods {
        let Some(index) = cx.sig(m).and_then(|sig| dynsigs.iter().position(|&s| s == sig)) else { continue };
        match by_index.iter_mut().find(|(i, _)| *i == index as i32) {
            Some(entry) => entry.1 = m,
            None => by_index.push((index as i32, m)),
        }
    }
    let entries: Vec<i32> = by_index.iter().map(|&(i, _)| i).collect();
    let (keys, values) = perfect_hash(&entries);
    if keys.is_empty() {
        return Val::Null;
    }
    let (found_keys, found_values): (Vec<Val>, Vec<Val>) = values
        .iter()
        .map(|v| match v {
            Some(k) => (Val::Int(*k), Val::Global(by_index.iter().find(|(i, _)| i == k).map_or(Global::NONE, |&(_, m)| m), rt.ptr)),
            None => (Val::Int(-1), Val::Null),
        })
        .unzip();
    let constant = |v: Val| Val::Const(Arc::new(v));
    constant(Val::StructValue(Arc::new([
        Val::Int(keys.len() as i32),
        constant(Val::ArrayValue(rt.int, keys.into_iter().map(Val::Int).collect())),
        constant(Val::ArrayValue(rt.int, found_keys.into())),
        constant(Val::ArrayValue(rt.ptr, found_values.into())),
    ])))
}
