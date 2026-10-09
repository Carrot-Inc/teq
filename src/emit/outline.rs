//! Inline expansions of one shape written once, as a function each site calls.
//!
//! An inline method expanded at many sites repeats its body at each: Magnolia's field step
//! once per field of every derived codec, with the label `constValue` gave, the type class
//! `summonInline` found and the test of the field's type as the only differences. The typer
//! records each expansion and what in it came from the call site (`Program::expansions`,
//! `leaves`, `leaf_tests`). Here every reached expansion is encoded with those leaves, the
//! locals of the site it reads and the expansions nested in it as holes; expansions of one
//! method whose encodings are equal share one function, named from the method and a hash of
//! the shape, and each site calls it with its own leaves: a literal or a plain local as the
//! value, anything else as a function evaluated where the expression stood, a type test as a
//! predicate. An expansion that returns from the enclosing method, assigns a local of the site,
//! names `this` or calls a local def of the site stays where it is. The function stands in the
//! module of the inline method's file, which no edit of a site moves.

use super::layout::Module;
use super::names::sanitize;
use super::share::{encode_roots, Hole, HoleKind, HoleNode, OlCx, Record, RecordShape, Shared};
use crate::intern::{FxMap, Interner};
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::fmt::Write;
use std::sync::Arc;

/// The fewest nodes a shape has for its expansions to be outlined: below that a call and its
/// arguments save little and cost a call at run time.
const MIN_NODES: u32 = 8;

#[derive(Default)]
pub struct Outline {
    /// Per expansion written as a call: its index into `site_fun` and `site_holes`.
    pub sites: FxMap<TExprId, u32>,
    /// The keys of `sites` as bits by expression, which the emitter tests at every node.
    site_bits: Vec<u64>,
    pub site_fun: Vec<u32>,
    pub site_holes: Vec<Arc<[Hole]>>,
    pub funs: Vec<OutFun>,
}

pub struct OutFun {
    pub name: String,
    /// The expansion whose tree the function's body is written from.
    pub root: TExprId,
    pub callee: SymId,
    /// The parameter each argument of the root's tree stands for, and how it is passed.
    pub params: FxMap<HoleNode, (u32, HoleKind)>,
    pub arity: u32,
    pub calls: u32,
}

/// The expansions of one shape: the exact encoding, the stable hash, the size, and each
/// expansion with its arguments.
struct Group {
    key: Arc<[u64]>,
    hash: u64,
    stable: u64,
    nodes: u32,
    members: Vec<(TExprId, Arc<[Hole]>, Arc<[(HoleNode, u32)]>)>,
}

/// The shapes of the expansions filed so far; one routine files a record in every build,
/// whichever order the records come in.
#[derive(Default)]
struct Shapes {
    groups: Vec<Group>,
    by_hash: FxMap<u64, Vec<u32>>,
    /// Shapes left without members, whose slots the next new shapes take.
    free: Vec<u32>,
}

impl Shapes {
    /// Files `root` with its shape and returns the shape and its place among the members.
    fn file(&mut self, root: TExprId, shape: &RecordShape) -> (u32, u32) {
        let bucket = self.by_hash.entry(shape.hash).or_default();
        let groups = &mut self.groups;
        let g = match bucket.iter().copied().find(|&g| Arc::ptr_eq(&groups[g as usize].key, &shape.key) || groups[g as usize].key == shape.key) {
            Some(g) => g,
            None => {
                let group = Group { key: shape.key.clone(), hash: shape.hash, stable: shape.stable, nodes: shape.nodes, members: Vec::new() };
                let g = match self.free.pop() {
                    Some(g) => {
                        groups[g as usize] = group;
                        g
                    }
                    None => {
                        groups.push(group);
                        groups.len() as u32 - 1
                    }
                };
                bucket.push(g);
                g
            }
        };
        let members = &mut groups[g as usize].members;
        members.push((root, shape.holes.clone(), shape.aliases.clone()));
        (g, members.len() as u32 - 1)
    }

    /// Takes the member at `i` of shape `g` out and returns the one moved into its place.
    fn take_out(&mut self, g: u32, i: u32) -> Option<TExprId> {
        let group = &mut self.groups[g as usize];
        group.members.swap_remove(i as usize);
        let moved = group.members.get(i as usize).map(|m| m.0);
        if group.members.is_empty() {
            let hash = group.hash;
            group.key = Arc::from([]);
            if let Some(bucket) = self.by_hash.get_mut(&hash) {
                bucket.retain(|&x| x != g);
                if bucket.is_empty() {
                    self.by_hash.remove(&hash);
                }
            }
            self.free.push(g);
        }
        moved
    }
}

/// A watch session's records of the expansions the last build encoded and its shapes. A record
/// is taken again while its expansion's tree is the same one (a re-typed file's expressions
/// are new) and the representatives and closures its encoding read are; a rebuild encodes the
/// expansions of the re-typed files and those whose reads changed, and moves them between the
/// shapes. The output does not depend on the order of a shape's members, and the shapes are
/// ordered by their stable hashes; two shapes of one hash are ordered as a fresh build meets
/// them, which a rebuild then does too.
#[derive(Default)]
pub struct Cache {
    records: FxMap<TExprId, Arc<Record>>,
    /// An expansion that encoded each nested one. Those that encode one are all expansions
    /// around it (an enclosing one encodes the arguments of a nested shape again), so they share
    /// the outermost, which a rebuild encodes again whole: one of them is enough.
    parent: FxMap<TExprId, TExprId>,
    /// Per class made per expansion or created as a closure, the records that read it and what
    /// they read.
    readers: FxMap<ClassId, Vec<TExprId>>,
    reps: FxMap<ClassId, ClassId>,
    closures: FxMap<ClassId, bool>,
    /// The roots of the last build, as bits by expression.
    found: Vec<u64>,
    shapes: Shapes,
    /// Each grouped expansion's shape and place among its members.
    place: FxMap<TExprId, (u32, u32)>,
}

impl Cache {
    /// The bytes the cache holds: its tables, the roots' bits and the shapes' members; the
    /// records by their count, whose encodings the shapes share.
    pub fn held(&self) -> usize {
        use crate::held::{array, owners, table};
        table(&self.records)
            + self.records.len() * std::mem::size_of::<Record>()
            + table(&self.parent)
            + table(&self.readers)
            + self.readers.values().map(array).sum::<usize>()
            + table(&self.reps)
            + table(&self.closures)
            + array(&self.found)
            + owners(&self.shapes.groups, |g| g.key.len() * 8 + array(&g.members))
            + table(&self.shapes.by_hash)
            + array(&self.shapes.free)
            + table(&self.place)
    }

    fn note(&mut self, root: TExprId, r: &Record) {
        for &child in &r.children {
            self.parent.insert(child, root);
        }
        for &(c, rep) in &r.reps {
            self.reps.insert(c, rep);
            self.readers.entry(c).or_default().push(root);
        }
        for &(c, closure) in &r.closures {
            self.closures.insert(c, closure);
            self.readers.entry(c).or_default().push(root);
        }
    }

    fn join(&mut self, root: TExprId) {
        let Some(shape) = &self.records[&root].shape else { return };
        if self.place.contains_key(&root) {
            return;
        }
        let at = self.shapes.file(root, shape);
        self.place.insert(root, at);
    }

    fn leave(&mut self, root: TExprId) {
        let Some((g, i)) = self.place.remove(&root) else { return };
        if let Some(moved) = self.shapes.take_out(g, i) {
            self.place.insert(moved, (g, i));
        }
    }

    /// Takes the record of `e` out, with what only it read.
    fn forget(&mut self, e: TExprId) {
        self.leave(e);
        self.parent.remove(&e);
        let Some(r) = self.records.remove(&e) else { return };
        for c in r.reps.iter().map(|x| x.0).chain(r.closures.iter().map(|x| x.0)) {
            let Some(readers) = self.readers.get_mut(&c) else { continue };
            readers.retain(|&x| x != e);
            if readers.is_empty() {
                self.readers.remove(&c);
                self.reps.remove(&c);
                self.closures.remove(&c);
            }
        }
    }

    /// The expansion and those nested in it, as the records have them.
    fn tree(&self, root: TExprId, out: &mut Vec<TExprId>) {
        let at = out.len();
        out.push(root);
        let mut i = at;
        while i < out.len() {
            if let Some(r) = self.records.get(&out[i]) {
                out.extend_from_slice(&r.children);
            }
            i += 1;
        }
    }

    /// Files every record again in the order a fresh build meets them.
    fn regroup(&mut self, found: &[TExprId]) {
        self.shapes = group_records(found, &self.records);
        self.place.clear();
        for (g, group) in self.shapes.groups.iter().enumerate() {
            for (i, m) in group.members.iter().enumerate() {
                self.place.insert(m.0, (g as u32, i as u32));
            }
        }
    }
}

fn bits_of(found: &[TExprId], len: usize) -> Vec<u64> {
    let mut bits = vec![0u64; len / 64 + 1];
    for f in found {
        bits[f.idx() / 64] |= 1 << (f.idx() % 64);
    }
    bits
}

impl Outline {
    #[allow(clippy::too_many_arguments)]
    pub fn compute(
        prog: &Program,
        syms: &Symbols,
        interner: &Interner,
        closure_anons: &FxMap<ClassId, usize>,
        shared: &Shared,
        reach: &super::reach::Reach,
        modules: &mut [Module],
        module_of_callee: &dyn Fn(SymId) -> usize,
        cache: Option<&mut Cache>,
    ) -> Outline {
        let mut out = Outline::default();
        if prog.expansions.is_empty() {
            return out;
        }
        let found = find_roots(prog, syms, shared, reach);
        let cx = OlCx { prog, syms, interner, closure_anons, shared, track: cache.is_some() };
        let mut own = Cache::default();
        let fresh = cache.as_ref().map_or(true, |c| c.found.is_empty());
        let cache = match cache {
            Some(c) if !fresh => {
                update(c, cx, &found, prog.exprs.len());
                c
            }
            other => {
                let session = other.is_some();
                let c = other.unwrap_or(&mut own);
                *c = Cache::default();
                let (records, repeats) = encode_in_parallel(cx, &found);
                if session {
                    for (root, r) in records {
                        if !c.records.contains_key(&root) {
                            c.note(root, &r);
                            c.records.insert(root, Arc::new(r));
                        }
                    }
                    c.found = bits_of(&found, prog.exprs.len());
                    c.regroup(&found);
                } else {
                    c.shapes = group_in_order(&records, repeats.then_some(prog.exprs.len()));
                }
                c
            }
        };
        let mut chosen: Vec<&Group> = cache.shapes.groups.iter().filter(|g| g.members.len() > 1 && g.nodes >= MIN_NODES).collect();
        chosen.sort_by_key(|g| g.stable);
        if !fresh && chosen.windows(2).any(|w| w[0].stable == w[1].stable) {
            cache.regroup(&found);
            chosen = cache.shapes.groups.iter().filter(|g| g.members.len() > 1 && g.nodes >= MIN_NODES).collect();
            chosen.sort_by_key(|g| g.stable);
        }
        let mut taken: FxMap<String, ()> = FxMap::default();
        for &Group { stable, ref members, .. } in chosen {
            let (root, rep_holes, rep_aliases) = &members[0];
            // The body is written from the first member's tree: members of one encoding give the
            // same text, whichever it is.
            let mut params: FxMap<HoleNode, (u32, HoleKind)> = FxMap::default();
            for (n, h) in rep_holes.iter().enumerate() {
                params.insert(h.node, (n as u32, h.kind));
            }
            for &(node, n) in rep_aliases.iter() {
                params.insert(node, (n, rep_holes[n as usize].kind));
            }
            let root = *root;
            let callee = prog.expansions[&root].callee;
            let module = module_of_callee(callee);
            let head = sanitize(interner.get(syms.sym(callee).name));
            let mut name = format!("{}$o{:08x}", head, stable as u32);
            let mut n = 2;
            while taken.contains_key(&name) {
                name = format!("{}$o{:08x}${}", head, stable as u32, n);
                n += 1;
            }
            taken.insert(name.clone(), ());
            let fun = out.funs.len() as u32;
            for (site, holes, _) in members {
                let word = site.idx() / 64;
                if out.site_bits.len() <= word {
                    out.site_bits.resize(word + 1, 0);
                }
                out.site_bits[word] |= 1 << (site.idx() % 64);
                out.sites.insert(*site, out.site_fun.len() as u32);
                out.site_fun.push(fun);
                out.site_holes.push(holes.clone());
            }
            modules[module].outlined.push(fun as usize);
            let arity = rep_holes.len() as u32;
            out.funs.push(OutFun { name, root, callee, params, arity, calls: members.len() as u32 });
        }
        out
    }
}

/// The expansions are encoded on threads, a slice of them each in the order the reach pass
/// met them. An expansion nested in one of another slice is encoded by both, alike.
/// Also whether an expansion may have more than one record: when two slices encoded it.
fn encode_in_parallel(cx: OlCx, roots: &[TExprId]) -> (Vec<(TExprId, Record)>, bool) {
    let workers = if roots.len() < PARALLEL_ROOTS { 1 } else { crate::workers().min(8) };
    if workers == 1 {
        return (encode_roots(cx, roots), false);
    }
    let slices: Vec<&[TExprId]> = roots.chunks(roots.len().div_ceil(workers).max(1)).collect();
    let shared_cx = SharedCx(cx);
    let shared_cx = &shared_cx;
    let records = std::thread::scope(|scope| {
        let handles: Vec<_> = slices.iter().map(|&slice| crate::alloc::spawn_in(scope, move || encode_roots(shared_cx.0, slice))).collect();
        handles.into_iter().flat_map(|h| h.join().expect("outline thread panicked")).collect()
    });
    (records, true)
}

/// Brings the cache to this build's roots: the expansions no longer met leave their shapes,
/// those met anew and those whose reads changed are encoded and join theirs.
fn update(cache: &mut Cache, cx: OlCx, found: &[TExprId], exprs: usize) {
    let bits = bits_of(found, exprs);
    let mut gone: Vec<TExprId> = Vec::new();
    for (w, (&old, &new)) in cache.found.iter().zip(bits.iter().chain(std::iter::repeat(&0))).enumerate() {
        let mut x = old & !new;
        while x != 0 {
            gone.push(TExprId((w * 64 + x.trailing_zeros() as usize) as u32));
            x &= x - 1;
        }
    }
    let mut changed: Vec<ClassId> = cache.reps.iter().filter(|&(&c, &rep)| cx.shared.rep_of(c) != rep).map(|(&c, _)| c).collect();
    changed.extend(cache.closures.iter().filter(|&(c, &closure)| cx.closure_anons.contains_key(c) != closure).map(|(&c, _)| c));
    let mut redo: Vec<TExprId> = Vec::new();
    for c in changed {
        cache.reps.remove(&c);
        cache.closures.remove(&c);
        for mut e in cache.readers.remove(&c).unwrap_or_default() {
            if !cache.records.contains_key(&e) {
                continue;
            }
            while let Some(&p) = cache.parent.get(&e) {
                e = p;
            }
            redo.push(e);
        }
    }
    // An expansion nested in one still met stays with it, as a fresh build encodes it there.
    gone.retain(|e| !cache.parent.contains_key(e));
    let mut tree = Vec::new();
    for &root in gone.iter().chain(&redo) {
        tree.clear();
        cache.tree(root, &mut tree);
        for &e in &tree {
            cache.forget(e);
        }
    }
    let stale: Vec<TExprId> = found.iter().copied().filter(|f| !cache.records.contains_key(f)).collect();
    let (fresh, _) = encode_in_parallel(cx, &stale);
    for (root, r) in fresh {
        if !cache.records.contains_key(&root) {
            cache.note(root, &r);
            cache.records.insert(root, Arc::new(r));
        }
    }
    for &root in &stale {
        tree.clear();
        cache.tree(root, &mut tree);
        for &e in tree.iter().rev() {
            cache.join(e);
        }
    }
    cache.found = bits;
}

/// The records filed by shape in the order they were made, each expansion once: the order of
/// `group_records`, which the threads' slices keep.
/// `repeats` holds the number of expressions when the records may name one expansion twice.
fn group_in_order(records: &[(TExprId, Record)], repeats: Option<usize>) -> Shapes {
    let mut shapes = Shapes::default();
    let mut seen = vec![0u64; repeats.map_or(0, |exprs| exprs / 64 + 1)];
    for (root, r) in records {
        if repeats.is_some() {
            let (w, bit) = (root.idx() / 64, 1u64 << (root.idx() % 64));
            if seen[w] & bit != 0 {
                continue;
            }
            seen[w] |= bit;
        }
        if let Some(shape) = &r.shape {
            shapes.file(*root, shape);
        }
    }
    shapes
}

/// The records filed by shape in the order one encoding of the roots in turn meets them: an
/// expansion's nested ones before it, each once. The order `group_in_order` files a fresh
/// build's records in, which a session's first build and a rebuild with two shapes of one hash
/// take from the records the session keeps.
fn group_records(found: &[TExprId], records: &FxMap<TExprId, Arc<Record>>) -> Shapes {
    let mut shapes = Shapes::default();
    let mut seen: FxMap<TExprId, ()> = FxMap::default();
    let mut stack: Vec<(TExprId, usize)> = Vec::new();
    for f in found {
        if seen.insert(*f, ()).is_some() {
            continue;
        }
        stack.push((*f, 0));
        while let Some(&mut (root, ref mut next)) = stack.last_mut() {
            let r = &records[&root];
            if let Some(&child) = r.children.get(*next) {
                *next += 1;
                if seen.insert(child, ()).is_none() {
                    stack.push((child, 0));
                }
                continue;
            }
            stack.pop();
            if let Some(shape) = &r.shape {
                shapes.file(root, shape);
            }
        }
    }
    shapes
}

/// The encoding context shared between the emitter's threads.
struct SharedCx<'a>(OlCx<'a>);

/// How many expansions make the encoding worth spreading over threads.
const PARALLEL_ROOTS: usize = 4096;

/// The expansions the reach pass met in the definitions the output holds; what a class that
/// another stands for holds is not written.
fn find_roots(prog: &Program, syms: &Symbols, shared: &Shared, reach: &super::reach::Reach) -> Vec<TExprId> {
    use super::reach::Place;
    let mut seen = vec![0u64; prog.exprs.len() / 64 + 1];
    let mut found = Vec::with_capacity(reach.roots.len());
    for &(root, place) in &reach.roots {
        let (word, bit) = (root.idx() / 64, 1u64 << (root.idx() % 64));
        if seen[word] & bit != 0 {
            continue;
        }
        seen[word] |= bit;
        let written = match place {
            Place::Val => true,
            Place::Class(c) => !shared.stands_for_another(c),
            Place::Fun(f) => match syms.sym(prog.funs[f.idx()].sym).owner {
                Owner::Class(c) => !shared.stands_for_another(c),
                _ => true,
            },
        };
        if written {
            found.push(root);
        }
    }
    found
}

impl<'a> super::Emitter<'a> {
    /// The node `e` as the emitter may inspect it: an argument of the function being written is
    /// a parameter there, whatever expression the root's own site passes for it.
    #[inline]
    pub(super) fn peek(&self, e: TExprId) -> TExpr {
        let e = self.through_same(e);
        if self.hole_of(e).is_some() {
            return TExpr::Null;
        }
        self.prog.expr(e)
    }

    /// `e` under the widenings that leave a JavaScript number as it is (`UnOp::same_number`):
    /// the emitter's structure is the operand's, which is written as it stands. A widening that
    /// is a parameter or a site of an outlined function is its own node, and the operand reached
    /// may be one: the caller tests the node returned, as `peek` does.
    pub(super) fn through_same(&self, mut e: TExprId) -> TExprId {
        loop {
            match self.prog.expr(e) {
                TExpr::Unary(op, a) if op.same_number() && !self.is_opaque(e) => e = a,
                _ => return e,
            }
        }
    }

    /// The parameter `e` is while the body of an outlined function is written.
    #[inline]
    pub(super) fn hole_of(&self, e: TExprId) -> Option<(u32, HoleKind)> {
        let f = self.body?;
        self.outline.funs[f as usize].params.get(&HoleNode::Expr(e)).copied()
    }

    #[inline]
    pub(super) fn test_hole(&self, node: HoleNode) -> Option<u32> {
        let f = self.body?;
        self.outline.funs[f as usize].params.get(&node).map(|&(n, _)| n)
    }

    /// The outlined expansion `e` is, unless it is the root whose function is being written.
    #[inline]
    pub(super) fn site_of(&self, e: TExprId) -> Option<u32> {
        let bits = &self.outline.site_bits;
        if bits.get(e.idx() / 64).map_or(true, |w| w & (1 << (e.idx() % 64)) == 0) {
            return None;
        }
        let site = *self.outline.sites.get(&e)?;
        match self.body {
            Some(f) if self.outline.funs[f as usize].root == e => None,
            _ => Some(site),
        }
    }

    /// Written as a call or a parameter rather than from its own tree.
    #[inline]
    pub(super) fn is_opaque(&self, e: TExprId) -> bool {
        self.hole_of(e).is_some() || self.site_of(e).is_some()
    }

    pub(super) fn emit_site_call(&mut self, site: u32) {
        let outline = self.outline;
        let f = outline.site_fun[site as usize];
        self.bindings.outlined.push(f);
        self.note_name(&outline.funs[f as usize].name);
        self.out.push_str(&outline.funs[f as usize].name);
        self.out.push('(');
        for (i, hole) in outline.site_holes[site as usize].iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
            }
            match (hole.kind, hole.node) {
                (HoleKind::Value, HoleNode::Expr(x)) => self.emit_value(x),
                (HoleKind::Thunk, HoleNode::Expr(x)) => self.emit_thunk(x),
                (_, node) => self.emit_predicate(node),
            }
        }
        self.out.push(')');
    }

    /// `() => e`, evaluated where the shared function reads it.
    fn emit_thunk(&mut self, x: TExprId) {
        if let Some((n, HoleKind::Thunk)) = self.hole_of(x) {
            let _ = write!(self.out, "$k{}", n);
            return;
        }
        self.out.push_str("() => ");
        self.fn_depth += 1;
        self.enter_scope(super::scope::Root::Expr(x));
        let node = self.peek(x);
        if self.is_simple(x) && !matches!(node, TExpr::Unit) {
            let needs_parens = matches!(node, TExpr::Js(..) | TExpr::Block(..) | TExpr::ObjLit(..));
            if needs_parens {
                self.out.push('(');
            }
            self.enter_cond();
            self.emit_value(x);
            self.leave_cond();
            if needs_parens {
                self.out.push(')');
            }
        } else {
            self.emit_body(x);
        }
        self.leave_scope();
        self.fn_depth -= 1;
    }

    /// The type test at `node` as a function of the tested value.
    fn emit_predicate(&mut self, node: HoleNode) {
        if let Some(n) = self.test_hole(node) {
            let _ = write!(self.out, "$k{}", n);
            return;
        }
        let prog = self.prog;
        let test = match node {
            HoleNode::TestExpr(e) => match prog.expr(e) {
                TExpr::TypeTest(_, t) => t,
                _ => unreachable!("a test argument stands at a type test"),
            },
            HoleNode::TestPat(p) => match prog.pats[p.idx()] {
                TPat::Test(t, _, _) => t,
                _ => unreachable!("a test argument stands at a type test"),
            },
            HoleNode::Expr(_) => unreachable!("an expression is passed as itself"),
        };
        self.out.push_str("($v) => ");
        self.enter_cond();
        self.emit_test_as(test, "$v", true);
        self.leave_cond();
    }

    /// `function name($k0, ...) { ... }`: the root's tree with its arguments read from the
    /// parameters, in a context of its own.
    pub(super) fn emit_outlined(&mut self, f: u32) {
        let outline = self.outline;
        let fun = &outline.funs[f as usize];
        self.bindings.defs.push(super::Def::Outlined(f));
        self.line();
        let _ = write!(self.out, "function {}(", fun.name);
        for i in 0..fun.arity {
            if i > 0 {
                self.out.push_str(", ");
            }
            let _ = write!(self.out, "$k{}", i);
        }
        self.out.push_str(") ");
        let fn_depth = std::mem::replace(&mut self.fn_depth, 0);
        let nonlocal = self.nonlocal_return.take();
        let tail = self.tail.take();
        let captures = std::mem::take(&mut self.captures);
        let closure_captures = std::mem::take(&mut self.closure_captures);
        let this_name = std::mem::replace(&mut self.this_name, "this");
        let before_super = std::mem::replace(&mut self.before_super, false);
        let body_params = std::mem::take(&mut self.body_params);
        let ctor_locals = std::mem::take(&mut self.ctor_locals);
        let body = self.body.replace(f);
        self.enter_scope(super::scope::Root::Expr(fun.root));
        for i in 0..fun.arity {
            self.declare_fixed(&format!("$k{}", i));
        }
        let def_scope = self.def_scope.replace(self.scopes.len() - 1);
        self.emit_body(fun.root);
        self.def_scope = def_scope;
        self.leave_scope();
        self.body = body;
        self.fn_depth = fn_depth;
        self.nonlocal_return = nonlocal;
        self.tail = tail;
        self.captures = captures;
        self.closure_captures = closure_captures;
        self.this_name = this_name;
        self.before_super = before_super;
        self.body_params = body_params;
        self.ctor_locals = ctor_locals;
    }
}
