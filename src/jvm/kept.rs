//! A JVM session's class files kept between its builds (docs/TARGETS.md, "Watch mode"). A unit
//! (a class, a file's top-level definitions) emitted by an earlier build is taken from the store
//! when its file has not been typed again since and the facts its emission reads of the rest of
//! the program are those of the build that emitted it; every other unit is emitted again.
//!
//! The facts are compared as digests, per table: of every class, symbol and file that existed at
//! the last emit and that no typing made (a local or anonymous class, a local symbol, a member of
//! such a class: those are read by their own unit alone, whose file is typed again when they
//! change, and a retype leaves the old ones dead). A table that differs re-emits every unit. What
//! the digests leave out is what a retype of a body does not change: the symbols' other fields,
//! the type store, the inference variables' instances, the jars' class files, the sources of the
//! files not typed again.

use super::{Cx, Emitted, Unit};
use crate::intern::{FxHasher, FxMap};
use crate::source::FileId;
use crate::symbols::*;
use crate::types::*;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum UnitKey {
    Class(ClassId),
    File(FileId),
}

/// What emitting a unit gave besides its class files: the function arities it used, and the
/// digest of what its own record holds that a retype elsewhere can change (`own`).
struct KeptUnit {
    classes: Vec<Emitted>,
    arities: u64,
    own: u64,
}

/// The facts' digests over the classes, symbols and files that existed at a build.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Digests {
    classes: u64,
    syms: u64,
    files: u64,
    /// The program's tables, per `WHOLE`.
    whole: [u64; 6],
}

/// Why nothing was kept when a part of the program's tables differs.
const WHOLE: [&str; 6] = [
    "not kept: the linked and external classes differ",
    "not kept: the entry points differ",
    "not kept: the overrides differ",
    "not kept: the traits' super calls differ",
    "not kept: the jar members' target names differ",
    "not kept: the export forwarders differ",
];

impl Digests {
    /// The first table that differs from `other`'s.
    fn differs(&self, other: &Digests) -> Option<&'static str> {
        [
            (self.classes != other.classes, "not kept: the classes' facts differ"),
            (self.syms != other.syms, "not kept: the symbols' facts differ"),
            (self.files != other.files, "not kept: the files' facts differ"),
        ]
            .into_iter()
            .chain(self.whole.iter().zip(&other.whole).zip(WHOLE).map(|((a, b), why)| (a != b, why)))
            .find(|(d, _)| *d)
            .map(|(_, t)| t)
    }
}

/// The digests of a build over what existed then, with the counts they cover.
#[derive(Clone, Copy, Default)]
pub struct Snapshot {
    classes: usize,
    syms: usize,
    files: usize,
    digests: Digests,
}

#[derive(Default)]
pub struct Kept {
    units: FxMap<UnitKey, KeptUnit>,
    snapshot: Option<Snapshot>,
    /// Per source (a file or a `package` block of one), whether it was typed again since the
    /// units were emitted, the retypes whose emit an error stopped included.
    dirty: Vec<bool>,
}

/// Whether a session keeps its class files: `TEQ_KEPT_EMIT=off` emits every unit on every build,
/// in the release build too.
pub fn wanted() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    !*OFF.get_or_init(|| std::env::var_os("TEQ_KEPT_EMIT").is_some_and(|v| v == "off"))
}

/// Whether the assertion-enabled build emits the kept units again and compares
/// (`TEQ_KEPT_EMIT_CHECK=off` skips it).
pub fn checked() -> bool {
    cfg!(debug_assertions) && std::env::var_os("TEQ_KEPT_EMIT_CHECK").map_or(true, |v| v != "off")
}

/// How a build used the store, for the session's answer.
#[derive(Default)]
pub struct Use {
    pub kept: usize,
    pub emitted: usize,
    /// Why nothing was kept: the table whose facts differ, or that there is nothing to keep.
    pub why: Option<&'static str>,
}

impl Kept {
    /// A full build: nothing typed before it is kept.
    pub fn clear(&mut self) {
        *self = Kept::default();
    }

    /// The sources typed again by a retype: their units are emitted with the next build that
    /// emits.
    pub fn retyped(&mut self, sources: impl Iterator<Item = FileId>) {
        for f in sources {
            if self.dirty.len() <= f.0 as usize {
                self.dirty.resize(f.0 as usize + 1, false);
            }
            self.dirty[f.0 as usize] = true;
        }
    }

    /// How many units the store holds; their class files' bytes are the session's last build's.
    pub fn units(&self) -> usize {
        self.units.len()
    }

    /// The facts of this build, which units of `units` the store gives (those whose file was not
    /// typed again and whose own record is as it was, when the facts are the last emit's), and
    /// the digest of each unit's own record.
    pub fn keepable(&self, cx: &Cx, units: &[Unit], usage: &mut Use) -> (Snapshot, Vec<bool>, Vec<u64>) {
        let prev = self.snapshot.unwrap_or_default();
        let (before, now) = facts(cx, &prev);
        let owns: Vec<u64> = units.iter().map(|&unit| own(cx, unit)).collect();
        let mut keep = vec![false; units.len()];
        usage.why = match &self.snapshot {
            None => Some("not kept: no emit before"),
            Some(s) => s.digests.differs(&before),
        };
        if usage.why.is_none() {
            for (i, &unit) in units.iter().enumerate() {
                let file = unit_file(cx, unit);
                keep[i] = !self.dirty.get(file.0 as usize).copied().unwrap_or(false) && self.units.get(&key(cx, unit)).is_some_and(|u| u.own == owns[i]);
            }
        }
        usage.kept = keep.iter().filter(|&&k| k).count();
        usage.emitted = units.len() - usage.kept;
        (now, keep, owns)
    }

    /// A kept unit's class files and arities.
    pub fn get(&self, cx: &Cx, unit: Unit) -> (&[Emitted], u64) {
        let u = &self.units[&key(cx, unit)];
        (&u.classes, u.arities)
    }

    /// After an emit without errors: the units of this build, those emitted with what they gave
    /// and the others as the store holds them, with their own records' digests and the build's
    /// facts; nothing is dirty.
    pub fn store(&mut self, cx: &Cx, units: &[Unit], emitted: Vec<Option<(Vec<Emitted>, u64)>>, owns: &[u64], snapshot: Snapshot) {
        let mut before = std::mem::take(&mut self.units);
        self.units = units
            .iter()
            .zip(emitted)
            .zip(owns)
            .filter_map(|((&unit, done), &own)| {
                let k = key(cx, unit);
                let kept = match done {
                    Some((classes, arities)) => KeptUnit { classes, arities, own },
                    None => before.remove(&k)?,
                };
                Some((k, kept))
            })
            .collect();
        self.snapshot = Some(snapshot);
        self.dirty.clear();
    }
}

fn key(cx: &Cx, unit: Unit) -> UnitKey {
    match unit {
        Unit::Class(i) => UnitKey::Class(cx.input.prog.classes[i].id),
        Unit::File(f) => UnitKey::File(f),
    }
}

/// The source a unit's definitions stand in: an inline expansion's anonymous class is its
/// caller's.
fn unit_file(cx: &Cx, unit: Unit) -> FileId {
    match unit {
        Unit::Class(i) => cx.input.syms.class(cx.input.prog.classes[i].id).module_file(),
        Unit::File(f) => f,
    }
}

/// What a unit's own record holds that a retype of another file can change, which the facts
/// leave out for a class a typing made: whether it and its methods are reached, its captures and
/// constructor parameters, the super accessors, forwarders and bridges the reach's binding of
/// mixins writes into it, whether it has subclasses, and which of its private members' names
/// clash with a member of a class above or below it.
fn own(cx: &Cx, unit: Unit) -> u64 {
    let Unit::Class(i) = unit else { return 0 };
    let (syms, reach) = (cx.input.syms, cx.input.reach);
    let tc = &cx.input.prog.classes[i];
    let flag = |v: &Vec<bool>, i: usize| v.get(i).copied().unwrap_or(false);
    let mut h = FxHasher::default();
    h.write_u8(flag(&reach.classes, tc.id.idx()) as u8);
    for &f in tc.methods.iter().chain(&tc.ctors) {
        h.write_u8(flag(&reach.funs, f.idx()) as u8);
    }
    for &s in &syms.class(tc.id).member_order {
        h.write_u8(flag(&reach.vals, s.idx()) as u8 | (flag(&reach.declared, s.idx()) as u8) << 1);
    }
    h.write_usize(tc.captures);
    h.write_usize(tc.ctor_params.len());
    for a in &tc.super_accessors {
        h.write_u32(a.of_trait.0);
        sym_key(syms, a.member, &mut h);
        if let Some(t) = a.target {
            sym_key(syms, t, &mut h);
        }
    }
    h.write_u8(0xff);
    for &(a, b) in tc.forwarders.iter().chain(&tc.bridges) {
        sym_key(syms, a, &mut h);
        sym_key(syms, b, &mut h);
    }
    h.write_u8(syms.class(tc.id).subclasses.is_empty() as u8);
    class_record(&syms.class(tc.id), &mut h);
    private_clashes(syms, tc.id, &mut h);
    h.finish()
}

/// The fields of a class's record a typing of another file may set: its flags, and how many
/// members, extensions, givens, constructors, children and nested classes it lists.
fn class_record(info: &ClassInfo, h: &mut FxHasher) {
    let flags = [info.stateful, info.has_exports, info.has_imports, info.has_statements, info.replaced, info.has_overloads, info.named_for_output, info.value_class, info.inherits_types];
    h.write_u16(flags.iter().fold(0u16, |a, &f| (a << 1) | f as u16));
    for n in [info.member_order.len(), info.members.len(), info.extensions.len(), info.givens.len(), info.ctors.len(), info.children.len(), info.nested.len(), info.base_types.len()] {
        h.write_usize(n);
    }
    h.write_u32(info.ordinal);
    info.superclass.map(|c| c.0).hash(h);
    info.companion.map(|c| c.0).hash(h);
}

/// Per private member or extension of `c`, whether its name clashes
/// (`Symbols::private_name_clashes`), which names it in the class files with the class's prefix.
fn private_clashes(syms: &Symbols, c: ClassId, h: &mut FxHasher) {
    let info = syms.class(c);
    if info.base_types.len() <= 1 && info.subclasses.is_empty() {
        return;
    }
    for &s in info.member_order.iter().chain(&info.extensions) {
        let m = syms.sym(s);
        if m.mods & crate::ast::mods::PRIVATE != 0 {
            h.write_u8(syms.private_name_clashes(c, m.name) as u8);
        }
    }
}

/// Whether a typing made `c`: a local or anonymous class, or one inside such a class.
fn made_class(syms: &Symbols, c: ClassId) -> bool {
    let mut c = c;
    loop {
        match syms.class(c).owner {
            Owner::Local => return true,
            Owner::Class(o) => c = o,
            Owner::Package(_) => return false,
        }
    }
}

/// Whether a typing made `s`: a local, or a member of a class a typing made.
fn made_sym(syms: &Symbols, s: SymId) -> bool {
    match syms.sym(s).owner {
        Owner::Local => true,
        Owner::Class(c) => made_class(syms, c),
        Owner::Package(_) => false,
    }
}

/// A symbol in a digest: by its id, or for one a typing made, which a retype makes anew, by its
/// name.
fn sym_key(syms: &Symbols, s: SymId, h: &mut FxHasher) {
    if made_sym(syms, s) {
        h.write_u8(1);
        syms.sym(s).name.hash(h);
    } else {
        h.write_u8(0);
        h.write_u32(s.0);
    }
}

/// The digests of a table over its first `upto` ids and over all `len`, each the hash of the
/// digests of its slices: the ids in slices of `SLICE` from 0, the last one cut at the end, so
/// that a table's digest over its first n ids depends on n alone and the slices are hashed on the
/// workers.
fn table(cx: &Cx, len: usize, upto: usize, fact: &(dyn Fn(usize, &mut FxHasher) + Sync)) -> (u64, u64) {
    const SLICE: usize = 1 << 15;
    let slice = |from: usize, to: usize| {
        let mut h = FxHasher::default();
        for i in from..to {
            fact(i, &mut h);
        }
        h.finish()
    };
    let upto = upto.min(len);
    let mut ranges: Vec<(usize, usize)> = (0..len / SLICE).map(|k| (k * SLICE, (k + 1) * SLICE)).collect();
    let tail = |n: usize| (n % SLICE != 0).then(|| (n / SLICE * SLICE, n));
    ranges.extend(tail(len));
    ranges.extend(tail(upto).filter(|&r| Some(r) != tail(len)));
    let shared = super::SyncCx(cx);
    let digests: Vec<u64> = if ranges.len() <= 1 {
        ranges.iter().map(|&(a, b)| slice(a, b)).collect()
    } else {
        let next = std::sync::atomic::AtomicUsize::new(0);
        let work = || {
            let _ = &shared;
            let mut done = Vec::new();
            while let Some(&(a, b)) = ranges.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed)) {
                done.push(((a, b), slice(a, b)));
            }
            done
        };
        let mut done: Vec<((usize, usize), u64)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..crate::workers().min(ranges.len())).map(|_| crate::alloc::spawn_in(scope, &work)).collect();
            handles.into_iter().flat_map(|h| h.join().expect("facts thread panicked")).collect()
        });
        done.sort_unstable();
        ranges.iter().map(|r| done[done.binary_search_by(|(d, _)| d.cmp(r)).expect("a slice hashed")].1).collect()
    };
    let over = |n: usize| {
        let mut h = FxHasher::default();
        for (&(a, b), &d) in ranges.iter().zip(&digests) {
            if b <= n && (b % SLICE == 0 || b == n) && a < n {
                h.write_u64(d);
            }
        }
        h.write_usize(n);
        h.finish()
    };
    (over(upto), over(len))
}

/// The facts' digests over what the last snapshot covered, and over everything now.
fn facts(cx: &Cx, prev: &Snapshot) -> (Digests, Snapshot) {
    let input = &cx.input;
    let (prog, syms, reach) = (input.prog, input.syms, input.reach);
    let flag = |v: &Vec<bool>, i: usize| v.get(i).copied().unwrap_or(false);

    let class_fact = |i: usize, h: &mut FxHasher| {
        let c = ClassId(i as u32);
        if made_class(syms, c) {
            return;
        }
        h.write_u32(c.0);
        let info = syms.class(c);
        let flags = [
            flag(&reach.classes, i),
            flag(&reach.library_classes, i),
            flag(&reach.product_classes, i),
            flag(&cx.held, i),
            cx.class_files.contains_key(&c),
            cx.vc_companions.contains_key(&c),
            input.link.jar_classes.contains_key(&c),
            cx.layout.has_body(c),
            info.subclasses.is_empty(),
        ];
        h.write_u16(flags.iter().fold(0u16, |a, &f| (a << 1) | f as u16));
        h.write(cx.class_names[i].as_bytes());
        private_clashes(syms, c, h);
        class_record(&info, h);
        let t = cx.tclass_of[i];
        if t != u32::MAX {
            let tc = &prog.classes[t as usize];
            h.write_usize(tc.captures);
            for &s in &tc.ctor_params {
                sym_key(syms, s, h);
            }
            h.write_u8(0xff);
            for &f in tc.methods.iter().chain(&tc.ctors) {
                sym_key(syms, prog.funs[f.idx()].sym, h);
            }
            h.write_u8(0xff);
            for &(a, b) in tc.forwarders.iter().chain(&tc.bridges) {
                sym_key(syms, a, h);
                sym_key(syms, b, h);
            }
            h.write_u8(0xff);
            for a in &tc.super_accessors {
                h.write_u32(a.of_trait.0);
                sym_key(syms, a.member, h);
                if let Some(t) = a.target {
                    sym_key(syms, t, h);
                }
            }
        }
    };
    let n_classes = syms.classes.len();
    let classes = table(cx, n_classes, prev.classes, &class_fact);

    let sym_fact = |i: usize, h: &mut FxHasher| {
        let s = SymId(i as u32);
        if made_sym(syms, s) {
            return;
        }
        let info = syms.sym(s);
        let f = cx.fun_of_sym[i];
        let (body, reached) = if f == u32::MAX { (false, false) } else { (prog.funs[f as usize].body.is_some(), flag(&reach.funs, f as usize)) };
        // Every field of the symbol's record a typing may set on a symbol it does not own, besides
        // its signature, which changes only with the full path: an overload made elsewhere marks
        // the member it meets (`alternative`, `meets_inherited`), which then goes by its
        // `@targetName`.
        let flags = [
            f != u32::MAX,
            body,
            reached,
            flag(&reach.vals, i),
            flag(&reach.declared, i),
            flag(&cx.layout.const_vals, i),
            info.needs_accessor,
            info.overridden_by_param,
            info.alternative,
            info.meets_inherited,
            info.superseded,
            info.scoped_private,
            info.is_main,
            info.by_name,
            info.java_defined,
            info.jvm_evidence,
            info.is_extension,
            info.js_bracket,
            info.js_symbol,
            info.intrinsic.is_some(),
        ];
        h.write_u32(s.0);
        h.write_u32(flags.iter().fold(0u32, |a, &f| (a << 1) | f as u32));
        info.mods.hash(h);
        h.write_u32(info.erased_tags);
        h.write_u8(info.ext_tparams);
        h.write_u8(info.ext_clauses);
        h.write_u8(info.dispatch as u8);
        info.impl_class.map(|c| c.0).hash(h);
        info.js_global.hash(h);
        info.js_name.hash(h);
        info.js_import.hash(h);
    };
    let n_syms = syms.syms.len();
    let syms_d = table(cx, n_syms, prev.syms, &sym_fact);

    let file_fact = |i: usize, h: &mut FxHasher| {
        h.write_u8(flag(&reach.files, i) as u8);
        let g = cx.file_group[i];
        if g != u32::MAX {
            for &(s, _) in &cx.layout.file_groups[g as usize].1 {
                sym_key(syms, s, h);
            }
        }
        h.write_u8(0xff);
        for &f in &cx.file_funs[i] {
            sym_key(syms, prog.funs[f.idx()].sym, h);
        }
    };
    let n_files = input.sources.len();
    let files = table(cx, n_files, prev.files, &file_fact);

    // The lists as sets: a walk meets the linked classes in its own order, and a retype puts its
    // file's entry points last.
    let sorted = |mut v: Vec<u64>| {
        v.sort_unstable();
        v
    };
    let mut whole = [0u64; 6];
    let mut h = FxHasher::default();
    sorted(reach.linked.iter().map(|c| c.0 as u64).collect()).hash(&mut h);
    sorted(reach.external_bodies.iter().map(|c| c.0 as u64).collect()).hash(&mut h);
    sorted(reach.interpolators.iter().map(|(a, b)| (a.0 as u64) << 32 | b.0 as u64).collect()).hash(&mut h);
    whole[0] = h.finish();
    let mut h = FxHasher::default();
    prog.main.hash(&mut h);
    prog.main_object.hash(&mut h);
    sorted(input.all_mains.unwrap_or(&[]).iter().map(|(s, c)| (s.0 as u64) << 32 | c.map_or(u32::MAX, |c| c.0) as u64).collect()).hash(&mut h);
    whole[1] = h.finish();
    // Order-free sums: the maps' iteration order is their own.
    let mut overrides = 0u64;
    for (&s, over) in prog.overrides.iter() {
        if !made_sym(syms, s) {
            let mut e = FxHasher::default();
            s.hash(&mut e);
            over.hash(&mut e);
            overrides = overrides.wrapping_add(e.finish());
        }
    }
    whole[2] = overrides;
    // The members each trait declares an accessor of its `super` call for, which the records of
    // the classes mixing it in make, those a typing made among them.
    let mut supers = 0u64;
    for (&t, members) in &cx.super_called {
        if !made_class(syms, t) {
            let mut e = FxHasher::default();
            t.hash(&mut e);
            let mut keys: Vec<u64> = members.iter().map(|&m| {
                let mut k = FxHasher::default();
                sym_key(syms, m, &mut k);
                k.finish()
            }).collect();
            keys.sort_unstable();
            keys.hash(&mut e);
            supers = supers.wrapping_add(e.finish());
        }
    }
    whole[3] = supers;
    // The jar members' `@targetName`s, which the loader enters as it completes their classes.
    let mut targets = 0u64;
    for (&s, &n) in input.target_names.into_iter().chain(input.product_target_names).flat_map(|m| m.iter()) {
        let mut e = FxHasher::default();
        s.hash(&mut e);
        n.hash(&mut e);
        targets = targets.wrapping_add(e.finish());
    }
    whole[4] = targets;
    // The export forwarders the plan gives each site, which name members of other files.
    let mut exports = 0u64;
    let sites = reach.exports.classes.iter().map(|(&c, l)| (c.0 as u64, l)).chain(reach.exports.files.iter().map(|(&f, l)| (1u64 << 40 | f.0 as u64, l)));
    for (site, list) in sites {
        let mut e = FxHasher::default();
        site.hash(&mut e);
        for f in list {
            f.name.hash(&mut e);
            f.selected.hash(&mut e);
            f.relayed.hash(&mut e);
            match f.q {
                crate::typer::exports::ExportQualifier::Object(o) => (0u8, o.0).hash(&mut e),
                crate::typer::exports::ExportQualifier::Package(p) => (1u8, p.0).hash(&mut e),
            }
            match f.target {
                crate::typer::export_plan::JvmTarget::Object(k) => k.hash(&mut e),
                crate::typer::export_plan::JvmTarget::Member(m) => sym_key(syms, m, &mut e),
            }
        }
        exports = exports.wrapping_add(e.finish());
    }
    whole[5] = exports;
    // `prog.template_syms` stays out: a unit reads the entries of its own template strings alone,
    // and a retype of another file enters its new templates under new strings.

    let before = Digests { classes: classes.0, syms: syms_d.0, files: files.0, whole };
    let now = Snapshot { classes: n_classes, syms: n_syms, files: n_files, digests: Digests { classes: classes.1, syms: syms_d.1, files: files.1, whole } };
    (before, now)
}
